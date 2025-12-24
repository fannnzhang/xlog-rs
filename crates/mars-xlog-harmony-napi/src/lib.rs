//! N-API bindings for Harmony/ohos.
//!
//! This crate exposes a minimal JS-friendly surface that wraps the `mars-xlog`
//! Rust API. It is intended for use by Harmony apps via N-API.
use mars_xlog::Xlog;
use napi_derive_ohos::napi;

/// Simple smoke-test function to verify the binding works.
#[napi]
pub fn add(left: u32, right: u32) -> u32 {
    left + right
}

#[napi(constructor)]
#[derive(Debug, Clone)]
pub struct XlogConfig {
    /// Directory for log files.
    pub log_dir: String,
    /// Prefix for log file names and instance id.
    pub name_prefix: String,
    /// Public key for encrypted logs (empty string disables encryption).
    pub pub_key: String,
    /// Cache directory for mmap buffers and temporary logs.
    pub cache_dir: String,
    /// Use async appender when true.
    pub r#async: bool,
    /// Enable console logging.
    pub console: bool,
    /// Minimum log level.
    pub level: Level,
}

#[napi]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Level {
    Verbose,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
    None,
}

fn to_core_level(level: Level) -> mars_xlog::LogLevel {
    match level {
        Level::Verbose => mars_xlog::LogLevel::Verbose,
        Level::Debug => mars_xlog::LogLevel::Debug,
        Level::Info => mars_xlog::LogLevel::Info,
        Level::Warn => mars_xlog::LogLevel::Warn,
        Level::Error => mars_xlog::LogLevel::Error,
        Level::Fatal => mars_xlog::LogLevel::Fatal,
        Level::None => mars_xlog::LogLevel::None,
    }
}

#[napi]
impl XlogConfig {
    /// Build a logger from the provided config.
    #[napi]
    pub fn build(&self) -> Logger {
        let user_config = self.clone();
        let xlog_config = mars_xlog::XlogConfig::new(user_config.log_dir, user_config.name_prefix)
            .cache_dir(user_config.cache_dir)
            .pub_key(user_config.pub_key)
            .mode(if user_config.r#async {
                mars_xlog::AppenderMode::Async
            } else {
                mars_xlog::AppenderMode::Sync
            })
            .compress_mode(mars_xlog::CompressMode::Zlib);
        let xlog = mars_xlog::Xlog::init(xlog_config, to_core_level(user_config.level)).unwrap();
        xlog.set_console_log_open(user_config.console);
        Logger { backend: xlog }
    }
}

#[napi]
pub struct Logger {
    backend: Xlog,
}

#[napi]
impl Logger {
    /// Log a message with a tag.
    #[napi]
    pub fn log(&self, level: Level, tag: String, message: String) {
        let level = to_core_level(level);
        self.backend.write(level, Some(&tag), &message);
    }
}
