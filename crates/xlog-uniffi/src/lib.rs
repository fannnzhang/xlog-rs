//! UniFFI bindings for Mars Xlog.
//!
//! This crate exposes a small surface for Kotlin/Swift consumers. It mirrors
//! the core `mars-xlog` types while keeping the API stable for FFI.
use mars_xlog as core;
use tracing::info;
use tracing_subscriber::prelude::*;

uniffi::setup_scaffolding!("mars_xlog_uniffi");

/// Log levels exposed to UniFFI consumers.
#[derive(uniffi::Enum, Debug, Copy, Clone, PartialEq, Eq)]
pub enum LogLevel {
    Verbose,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
    None,
}

/// Configuration passed from foreign-language callers.
#[derive(uniffi::Record, Debug, Clone)]
pub struct XlogConfig {
    /// Directory for log files.
    pub log_dir: String,
    /// Prefix for log file names and instance id.
    pub name_prefix: String,
    /// Public key for encrypted logs (empty string disables encryption).
    pub pub_key: String,
    /// Cache directory for mmap buffers and temporary logs.
    pub cache_dir: String,
    /// Days to keep cached logs before moving them.
    pub cache_days: i32,
    /// Appender mode enum encoded as an int (0=Async, 1=Sync).
    pub mode: i32,
    /// Compression mode enum encoded as an int (0=Zlib, 1=Zstd).
    pub compress_mode: i32,
    /// Compression level forwarded to the compressor.
    pub compress_level: i32,
}

/// Errors surfaced through UniFFI.
#[derive(uniffi::Error, thiserror::Error, Debug)]
pub enum XlogError {
    #[error("{details}")]
    Message { details: String },
}

/// Logger handle exposed to foreign-language callers.
#[derive(uniffi::Object)]
pub struct Logger {
    inner: core::Xlog,
}

fn to_core_level(level: LogLevel) -> core::LogLevel {
    match level {
        LogLevel::Verbose => core::LogLevel::Verbose,
        LogLevel::Debug => core::LogLevel::Debug,
        LogLevel::Info => core::LogLevel::Info,
        LogLevel::Warn => core::LogLevel::Warn,
        LogLevel::Error => core::LogLevel::Error,
        LogLevel::Fatal => core::LogLevel::Fatal,
        LogLevel::None => core::LogLevel::None,
    }
}

fn to_core_config(cfg: XlogConfig) -> core::XlogConfig {
    let mut config = core::XlogConfig::new(cfg.log_dir, cfg.name_prefix)
        .cache_days(cfg.cache_days)
        .compress_level(cfg.compress_level)
        .mode(if cfg.mode == 1 {
            core::AppenderMode::Sync
        } else {
            core::AppenderMode::Async
        })
        .compress_mode(if cfg.compress_mode == 1 {
            core::CompressMode::Zstd
        } else {
            core::CompressMode::Zlib
        });

    if !cfg.pub_key.is_empty() {
        config = config.pub_key(cfg.pub_key);
    }
    if !cfg.cache_dir.is_empty() {
        config = config.cache_dir(cfg.cache_dir);
    }
    config
}

fn init_tracing(logger: core::Xlog, level: core::LogLevel) -> Result<(), XlogError> {
    let (layer, _handle) =
        core::XlogLayer::with_config(logger, core::XlogLayerConfig::new(level).enabled(true));
    let subscriber = tracing_subscriber::registry().with(layer);
    tracing::subscriber::set_global_default(subscriber).map_err(|e| XlogError::Message {
        details: format!("init tracing subscriber failed: {e}"),
    })
}
#[uniffi::export]
impl Logger {
    /// Create a new logger instance and configure tracing.
    #[uniffi::constructor]
    pub fn new(config: XlogConfig, level: LogLevel) -> Result<Self, XlogError> {
        let cfg = to_core_config(config);
        let level = to_core_level(level);
        let logger = core::Xlog::init(cfg, level).map_err(|e| XlogError::Message {
            details: e.to_string(),
        })?;
        logger.set_console_log_open(true);
        init_tracing(logger.clone(), level)?;
        info!("Initialized logger successfully!");
        Ok(Self { inner: logger })
    }

    /// Log a message without file/function metadata.
    pub fn log(&self, level: LogLevel, tag: String, message: String) {
        // Avoid Rust-side caller info; Kotlin will provide metadata if needed.
        self.inner
            .write_with_meta(to_core_level(level), Some(&tag), "", "", 0, &message);
    }

    /// Log a message with explicit metadata from the caller.
    pub fn log_with_meta(
        &self,
        level: LogLevel,
        tag: String,
        file: String,
        func: String,
        line: i32,
        message: String,
    ) {
        let line = if line < 0 { 0 } else { line as u32 };
        self.inner.write_with_meta(
            to_core_level(level),
            Some(&tag),
            &file,
            &func,
            line,
            &message,
        );
    }

    /// Flush buffered logs.
    pub fn flush(&self, sync: bool) {
        self.inner.flush(sync);
    }
}
