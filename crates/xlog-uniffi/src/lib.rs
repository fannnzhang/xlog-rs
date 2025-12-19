use mars_xlog as core;

uniffi::setup_scaffolding!("xlog");

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

#[derive(uniffi::Record, Debug, Clone)]
pub struct XlogConfig {
    pub log_dir: String,
    pub name_prefix: String,
    pub pub_key: String,
    pub cache_dir: String,
    pub cache_days: i32,
    pub mode: i32,
    pub compress_mode: i32,
    pub compress_level: i32,
}

#[derive(uniffi::Error, thiserror::Error, Debug)]
pub enum XlogError {
    #[error("{message}")]
    Message { message: String },
}

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

#[uniffi::export]
impl Logger {
    #[uniffi::constructor]
    pub fn new(config: XlogConfig, level: LogLevel) -> Result<Self, XlogError> {
        let cfg = to_core_config(config);
        let level = to_core_level(level);
        let logger = core::Xlog::new(cfg, level)
            .map_err(|e| XlogError::Message { message: e.to_string() })?;
        Ok(Self { inner: logger })
    }

    pub fn log(&self, level: LogLevel, tag: String, message: String) {
        self.inner
            .write(to_core_level(level), Some(&tag), &message);
    }

    pub fn flush(&self, sync: bool) {
        self.inner.flush(sync);
    }
}
