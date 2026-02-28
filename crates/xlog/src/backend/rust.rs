use std::sync::atomic::{AtomicBool, AtomicI32, AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};

use chrono::Local;
use mars_xlog_core::appender_engine::{AppenderEngine, EngineMode};
use mars_xlog_core::buffer::{PersistentBuffer, DEFAULT_BUFFER_BLOCK_LEN};
use mars_xlog_core::compress::{StreamCompressor, ZlibStreamCompressor, ZstdChunkCompressor};
use mars_xlog_core::crypto::EcdhTeaCipher;
use mars_xlog_core::dump::{dump_to_file, memory_dump};
use mars_xlog_core::file_manager::FileManager;
use mars_xlog_core::oneshot::{
    oneshot_flush as core_oneshot_flush, FileIoAction as CoreFileIoAction,
};
use mars_xlog_core::platform_console::{write_console_line, ConsoleLevel};
use mars_xlog_core::platform_tid::current_tid;
use mars_xlog_core::protocol::{
    select_magic, AppendMode, CompressionKind, LogHeader, SeqGenerator, MAGIC_END,
};
use mars_xlog_core::record::{LogLevel as CoreLogLevel, LogRecord};
use mars_xlog_core::registry::InstanceRegistry;

use super::{XlogBackend, XlogBackendProvider};
use crate::{AppenderMode, CompressMode, FileIoAction, LogLevel, XlogConfig, XlogError};

#[cfg(any(
    target_os = "ios",
    target_os = "macos",
    target_os = "tvos",
    target_os = "watchos"
))]
use crate::ConsoleFun;

pub(super) fn provider() -> &'static dyn XlogBackendProvider {
    static PROVIDER: RustBackendProvider = RustBackendProvider;
    &PROVIDER
}

struct RustBackendProvider;

struct RustBackend {
    id: usize,
    config: XlogConfig,
    level: AtomicI32,
    console_open: AtomicBool,
    seq: SeqGenerator,
    cipher: EcdhTeaCipher,
    engine: AppenderEngine,
}

static NEXT_ID: AtomicUsize = AtomicUsize::new(1);

fn registry() -> &'static InstanceRegistry<RustBackend> {
    static REGISTRY: OnceLock<InstanceRegistry<RustBackend>> = OnceLock::new();
    REGISTRY.get_or_init(InstanceRegistry::new)
}

impl RustBackend {
    fn new(config: XlogConfig, level: LogLevel) -> Result<Self, XlogError> {
        if config.log_dir.is_empty() || config.name_prefix.is_empty() {
            return Err(XlogError::InvalidConfig);
        }

        let cipher = match config.pub_key.as_deref() {
            Some(key) if !key.is_empty() => {
                EcdhTeaCipher::new(key).map_err(|_| XlogError::InitFailed)?
            }
            _ => EcdhTeaCipher::disabled(),
        };

        let file_manager = FileManager::new(
            config.log_dir.clone().into(),
            config.cache_dir.clone().map(Into::into),
            config.name_prefix.clone(),
            config.cache_days,
        )
        .map_err(|_| XlogError::InitFailed)?;
        let buffer = PersistentBuffer::open_with_capacity(
            file_manager.mmap_path(),
            DEFAULT_BUFFER_BLOCK_LEN,
        )
        .map_err(|_| XlogError::InitFailed)?;

        let engine = AppenderEngine::new(
            file_manager,
            buffer,
            appender_to_engine_mode(config.mode),
            0,
            10 * 24 * 60 * 60,
        );

        Ok(Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            console_open: AtomicBool::new(false),
            level: AtomicI32::new(level_to_i32(level)),
            config,
            seq: SeqGenerator::default(),
            cipher,
            engine,
        })
    }

    fn build_block(
        &self,
        level: LogLevel,
        tag: &str,
        file: &str,
        func: &str,
        line: u32,
        msg: &str,
    ) -> Vec<u8> {
        let mode = engine_to_appender_mode(self.engine.mode());
        let compress = self.config.compress_mode;

        let now = Local::now();
        let pid = std::process::id() as i64;
        let tid = current_tid();

        let record = LogRecord {
            level: to_core_level(level),
            tag: tag.to_string(),
            filename: file.to_string(),
            func_name: func.to_string(),
            line: line as i32,
            timestamp: std::time::SystemTime::now(),
            pid,
            tid,
            maintid: pid,
        };
        let line = mars_xlog_core::formatter::format_record(&record, msg);

        let mut payload = match mode {
            AppenderMode::Sync => line.into_bytes(),
            AppenderMode::Async => match compress {
                CompressMode::Zlib => {
                    let mut c = ZlibStreamCompressor::default();
                    let mut out = Vec::new();
                    let _ = c.compress_chunk(line.as_bytes(), &mut out);
                    let _ = c.flush(&mut out);
                    out
                }
                CompressMode::Zstd => {
                    let mut c = ZstdChunkCompressor::new(self.config.compress_level);
                    let mut out = Vec::new();
                    let _ = c.compress_chunk(line.as_bytes(), &mut out);
                    let _ = c.flush(&mut out);
                    out
                }
            },
        };

        if self.cipher.enabled() {
            payload = match mode {
                AppenderMode::Sync => self.cipher.encrypt_sync(&payload),
                AppenderMode::Async => self.cipher.encrypt_async(&payload),
            };
        }

        let compression_kind = match compress {
            CompressMode::Zlib => CompressionKind::Zlib,
            CompressMode::Zstd => CompressionKind::Zstd,
        };
        let append_mode = match mode {
            AppenderMode::Sync => AppendMode::Sync,
            AppenderMode::Async => AppendMode::Async,
        };

        let header = LogHeader {
            magic: select_magic(compression_kind, append_mode, self.cipher.enabled()),
            seq: match mode {
                AppenderMode::Sync => SeqGenerator::sync_seq(),
                AppenderMode::Async => self.seq.next_async(),
            },
            begin_hour: chrono::Timelike::hour(&now) as u8,
            end_hour: chrono::Timelike::hour(&now) as u8,
            len: payload.len() as u32,
            client_pubkey: self.cipher.client_pubkey(),
        };

        let mut block = Vec::with_capacity(73 + payload.len() + 1);
        block.extend_from_slice(&header.encode());
        block.extend_from_slice(&payload);
        block.push(MAGIC_END);
        block
    }

    fn make_logfile_name_impl(&self, timespan: i32, prefix: &str) -> Vec<String> {
        self.engine.make_logfile_name(timespan, prefix)
    }

    fn filepaths_from_timespan_impl(&self, timespan: i32, prefix: &str) -> Vec<String> {
        self.engine.filepaths_from_timespan(timespan, prefix)
    }
}

impl XlogBackendProvider for RustBackendProvider {
    fn new_instance(
        &self,
        config: &XlogConfig,
        level: LogLevel,
    ) -> Result<Arc<dyn XlogBackend>, XlogError> {
        let backend = registry().get_or_try_insert_with(&config.name_prefix, || {
            Ok::<_, XlogError>(Arc::new(RustBackend::new(config.clone(), level)?))
        })?;
        Ok(backend)
    }

    fn get_instance(&self, name_prefix: &str) -> Option<Arc<dyn XlogBackend>> {
        registry()
            .get(name_prefix)
            .map(|v| v as Arc<dyn XlogBackend>)
    }

    fn appender_open(&self, config: &XlogConfig, level: LogLevel) -> Result<(), XlogError> {
        let backend = Arc::new(RustBackend::new(config.clone(), level)?);
        registry().set_default(backend);
        Ok(())
    }

    fn appender_close(&self) {
        registry().clear_default();
    }

    fn flush_all(&self, sync: bool) {
        if let Some(default) = registry().default_instance() {
            default.flush(sync);
        }
        registry().for_each_live(|backend| {
            backend.flush(sync);
        });
    }

    #[cfg(any(
        target_os = "ios",
        target_os = "macos",
        target_os = "tvos",
        target_os = "watchos"
    ))]
    fn set_console_fun(&self, _fun: ConsoleFun) {
        // no-op in rust backend for now
    }

    fn current_log_path(&self) -> Option<String> {
        registry()
            .default_instance()
            .and_then(|b| b.engine.log_dir())
    }

    fn current_log_cache_path(&self) -> Option<String> {
        registry()
            .default_instance()
            .and_then(|b| b.engine.cache_dir())
    }

    fn filepaths_from_timespan(&self, timespan: i32, prefix: &str) -> Vec<String> {
        registry()
            .default_instance()
            .map(|b| b.filepaths_from_timespan_impl(timespan, prefix))
            .unwrap_or_default()
    }

    fn make_logfile_name(&self, timespan: i32, prefix: &str) -> Vec<String> {
        registry()
            .default_instance()
            .map(|b| b.make_logfile_name_impl(timespan, prefix))
            .unwrap_or_default()
    }

    fn oneshot_flush(&self, config: &XlogConfig) -> Result<FileIoAction, XlogError> {
        if config.log_dir.is_empty() || config.name_prefix.is_empty() {
            return Err(XlogError::InvalidConfig);
        }

        let file_manager = FileManager::new(
            config.log_dir.clone().into(),
            config.cache_dir.clone().map(Into::into),
            config.name_prefix.clone(),
            config.cache_days,
        )
        .map_err(|_| XlogError::InitFailed)?;

        let max_file_size = registry()
            .get(&config.name_prefix)
            .or_else(|| registry().default_instance())
            .map(|b| b.engine.max_file_size())
            .unwrap_or(0);

        let action = core_oneshot_flush(&file_manager, DEFAULT_BUFFER_BLOCK_LEN, max_file_size);
        Ok(match action {
            CoreFileIoAction::None => FileIoAction::None,
            CoreFileIoAction::Success => FileIoAction::Success,
            CoreFileIoAction::Unnecessary => FileIoAction::Unnecessary,
            CoreFileIoAction::OpenFailed => FileIoAction::OpenFailed,
            CoreFileIoAction::ReadFailed => FileIoAction::ReadFailed,
            CoreFileIoAction::WriteFailed => FileIoAction::WriteFailed,
            CoreFileIoAction::CloseFailed => FileIoAction::CloseFailed,
            CoreFileIoAction::RemoveFailed => FileIoAction::RemoveFailed,
        })
    }

    fn dump(&self, buffer: &[u8]) -> String {
        if let Some(default) = registry().default_instance() {
            if let Some(log_dir) = default.engine.log_dir() {
                let dumped = dump_to_file(&log_dir, buffer);
                if !dumped.is_empty() {
                    return dumped;
                }
            }
        }
        memory_dump(buffer)
    }

    fn memory_dump(&self, buffer: &[u8]) -> String {
        memory_dump(buffer)
    }
}

impl XlogBackend for RustBackend {
    fn instance(&self) -> usize {
        self.id
    }

    fn is_enabled(&self, level: LogLevel) -> bool {
        level_to_i32(level) >= self.level.load(Ordering::Relaxed)
    }

    fn level(&self) -> LogLevel {
        i32_to_level(self.level.load(Ordering::Relaxed))
    }

    fn set_level(&self, level: LogLevel) {
        self.level.store(level_to_i32(level), Ordering::Relaxed);
    }

    fn set_appender_mode(&self, mode: AppenderMode) {
        let _ = self.engine.set_mode(appender_to_engine_mode(mode));
    }

    fn flush(&self, sync: bool) {
        let _ = self.engine.flush(sync);
    }

    fn set_console_log_open(&self, open: bool) {
        self.console_open.store(open, Ordering::Relaxed);
    }

    fn set_max_file_size(&self, max_bytes: i64) {
        self.engine.set_max_file_size(max_bytes.max(0) as u64);
    }

    fn set_max_alive_time(&self, alive_seconds: i64) {
        self.engine.set_max_alive_time(alive_seconds);
    }

    fn write_with_meta(
        &self,
        level: LogLevel,
        tag: &str,
        file: &str,
        func: &str,
        line: u32,
        msg: &str,
    ) {
        if !self.is_enabled(level) {
            return;
        }

        if self.console_open.load(Ordering::Relaxed) {
            write_console_line(to_console_level(level), msg);
        }

        let block = self.build_block(level, tag, file, func, line, msg);
        let _ = self.engine.write_block(&block, level == LogLevel::Fatal);
    }
}

fn level_to_i32(level: LogLevel) -> i32 {
    match level {
        LogLevel::Verbose => 0,
        LogLevel::Debug => 1,
        LogLevel::Info => 2,
        LogLevel::Warn => 3,
        LogLevel::Error => 4,
        LogLevel::Fatal => 5,
        LogLevel::None => 6,
    }
}

fn i32_to_level(v: i32) -> LogLevel {
    match v {
        0 => LogLevel::Verbose,
        1 => LogLevel::Debug,
        2 => LogLevel::Info,
        3 => LogLevel::Warn,
        4 => LogLevel::Error,
        5 => LogLevel::Fatal,
        _ => LogLevel::None,
    }
}

fn to_core_level(level: LogLevel) -> CoreLogLevel {
    match level {
        LogLevel::Verbose => CoreLogLevel::Verbose,
        LogLevel::Debug => CoreLogLevel::Debug,
        LogLevel::Info => CoreLogLevel::Info,
        LogLevel::Warn => CoreLogLevel::Warn,
        LogLevel::Error => CoreLogLevel::Error,
        LogLevel::Fatal => CoreLogLevel::Fatal,
        LogLevel::None => CoreLogLevel::None,
    }
}

fn appender_to_engine_mode(mode: AppenderMode) -> EngineMode {
    match mode {
        AppenderMode::Async => EngineMode::Async,
        AppenderMode::Sync => EngineMode::Sync,
    }
}

fn engine_to_appender_mode(mode: EngineMode) -> AppenderMode {
    match mode {
        EngineMode::Async => AppenderMode::Async,
        EngineMode::Sync => AppenderMode::Sync,
    }
}

fn to_console_level(level: LogLevel) -> ConsoleLevel {
    match level {
        LogLevel::Verbose => ConsoleLevel::Verbose,
        LogLevel::Debug => ConsoleLevel::Debug,
        LogLevel::Info => ConsoleLevel::Info,
        LogLevel::Warn => ConsoleLevel::Warn,
        LogLevel::Error => ConsoleLevel::Error,
        LogLevel::Fatal => ConsoleLevel::Fatal,
        LogLevel::None => ConsoleLevel::None,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::RustBackend;
    use crate::backend::XlogBackend;
    use crate::LogLevel;

    #[test]
    fn rust_backend_writes_xlog_block() {
        let root = std::env::temp_dir().join(format!("xlog-rust-backend-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();

        let cfg = crate::XlogConfig::new(root.to_string_lossy().to_string(), "demo");
        let backend = RustBackend::new(cfg, LogLevel::Info).unwrap();

        backend.write_with_meta(LogLevel::Info, "demo", "main.rs", "f", 1, "hello");
        backend.flush(true);

        let mut found = false;
        for entry in fs::read_dir(&root).unwrap().flatten() {
            let p = entry.path();
            if p.extension().and_then(|x| x.to_str()) == Some("xlog") {
                let bytes = fs::read(&p).unwrap();
                assert!(!bytes.is_empty());
                found = true;
            }
        }

        assert!(found, "expected at least one xlog output file");
        let _ = fs::remove_dir_all(&root);
    }
}
