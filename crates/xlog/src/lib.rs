use libc::{c_char, c_int, gettimeofday, timeval};
use mars_xlog_sys as sys;
use std::ffi::{CStr, CString};
use std::ptr;
use std::sync::Arc;

#[cfg(feature = "tracing")]
mod tracing_layer;

#[cfg(feature = "tracing")]
pub use tracing_layer::{XlogLayer, XlogLayerConfig, XlogLayerHandle};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum LogLevel {
    Verbose,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
    None,
}

impl LogLevel {
    fn as_sys(self) -> sys::TLogLevel {
        match self {
            LogLevel::Verbose => sys::TLogLevel::kLevelVerbose,
            LogLevel::Debug => sys::TLogLevel::kLevelDebug,
            LogLevel::Info => sys::TLogLevel::kLevelInfo,
            LogLevel::Warn => sys::TLogLevel::kLevelWarn,
            LogLevel::Error => sys::TLogLevel::kLevelError,
            LogLevel::Fatal => sys::TLogLevel::kLevelFatal,
            LogLevel::None => sys::TLogLevel::kLevelNone,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum AppenderMode {
    Async,
    Sync,
}

impl AppenderMode {
    fn as_sys(self) -> sys::TAppenderMode {
        match self {
            AppenderMode::Async => sys::TAppenderMode::kAppenderAsync,
            AppenderMode::Sync => sys::TAppenderMode::kAppenderSync,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CompressMode {
    Zlib,
    Zstd,
}

impl CompressMode {
    fn as_sys(self) -> sys::TCompressMode {
        match self {
            CompressMode::Zlib => sys::TCompressMode::kZlib,
            CompressMode::Zstd => sys::TCompressMode::kZstd,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum FileIoAction {
    None,
    Success,
    Unnecessary,
    OpenFailed,
    ReadFailed,
    WriteFailed,
    CloseFailed,
    RemoveFailed,
}

impl From<c_int> for FileIoAction {
    fn from(value: c_int) -> Self {
        match value {
            1 => FileIoAction::Success,
            2 => FileIoAction::Unnecessary,
            3 => FileIoAction::OpenFailed,
            4 => FileIoAction::ReadFailed,
            5 => FileIoAction::WriteFailed,
            6 => FileIoAction::CloseFailed,
            7 => FileIoAction::RemoveFailed,
            _ => FileIoAction::None,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum XlogError {
    #[error("log_dir and name_prefix must be non-empty")]
    InvalidConfig,
    #[error("xlog initialization failed")]
    InitFailed,
}

#[derive(Debug, Clone)]
pub struct XlogConfig {
    pub log_dir: String,
    pub name_prefix: String,
    pub pub_key: Option<String>,
    pub cache_dir: Option<String>,
    pub cache_days: i32,
    pub mode: AppenderMode,
    pub compress_mode: CompressMode,
    pub compress_level: i32,
}

impl XlogConfig {
    pub fn new(log_dir: impl Into<String>, name_prefix: impl Into<String>) -> Self {
        Self {
            log_dir: log_dir.into(),
            name_prefix: name_prefix.into(),
            pub_key: None,
            cache_dir: None,
            cache_days: 0,
            mode: AppenderMode::Async,
            compress_mode: CompressMode::Zlib,
            compress_level: 6,
        }
    }

    pub fn pub_key(mut self, key: impl Into<String>) -> Self {
        self.pub_key = Some(key.into());
        self
    }

    pub fn cache_dir(mut self, dir: impl Into<String>) -> Self {
        self.cache_dir = Some(dir.into());
        self
    }

    pub fn cache_days(mut self, days: i32) -> Self {
        self.cache_days = days;
        self
    }

    pub fn mode(mut self, mode: AppenderMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn compress_mode(mut self, mode: CompressMode) -> Self {
        self.compress_mode = mode;
        self
    }

    pub fn compress_level(mut self, level: i32) -> Self {
        self.compress_level = level;
        self
    }

    fn to_sys(&self) -> (sys::MarsXlogConfig, Vec<CString>) {
        let mut cstrings = Vec::new();
        let log_dir = to_cstring(&self.log_dir, &mut cstrings);
        let name_prefix = to_cstring(&self.name_prefix, &mut cstrings);
        let pub_key = self
            .pub_key
            .as_deref()
            .map(|s| to_cstring(s, &mut cstrings))
            .unwrap_or(ptr::null());
        let cache_dir = self
            .cache_dir
            .as_deref()
            .map(|s| to_cstring(s, &mut cstrings))
            .unwrap_or(ptr::null());

        let cfg = sys::MarsXlogConfig {
            mode: self.mode.as_sys() as c_int,
            logdir: log_dir,
            nameprefix: name_prefix,
            pub_key: pub_key,
            compress_mode: self.compress_mode.as_sys() as c_int,
            compress_level: self.compress_level as c_int,
            cache_dir: cache_dir,
            cache_days: self.cache_days as c_int,
        };
        (cfg, cstrings)
    }
}

#[derive(Clone)]
pub struct Xlog {
    inner: Arc<Inner>,
}

struct Inner {
    instance: usize,
    name_prefix: String,
}

impl Drop for Inner {
    fn drop(&mut self) {
        let name = CString::new(self.name_prefix.clone()).unwrap_or_else(|_| CString::new("xlog").unwrap());
        unsafe {
            sys::mars_xlog_release_instance(name.as_ptr());
        }
    }
}

impl Xlog {
    pub fn new(config: XlogConfig, level: LogLevel) -> Result<Self, XlogError> {
        if config.log_dir.is_empty() || config.name_prefix.is_empty() {
            return Err(XlogError::InvalidConfig);
        }
        let (cfg, _cstr) = config.to_sys();
        let instance = unsafe { sys::mars_xlog_new_instance(&cfg, level.as_sys() as c_int) };
        if instance == 0 {
            return Err(XlogError::InitFailed);
        }
        Ok(Self {
            inner: Arc::new(Inner {
                instance,
                name_prefix: config.name_prefix,
            }),
        })
    }

    pub fn get(name_prefix: &str) -> Option<Self> {
        let name = CString::new(name_prefix).ok()?;
        let instance = unsafe { sys::mars_xlog_get_instance(name.as_ptr()) };
        if instance == 0 {
            return None;
        }
        Some(Self {
            inner: Arc::new(Inner {
                instance,
                name_prefix: name_prefix.to_string(),
            }),
        })
    }

    pub fn appender_open(config: XlogConfig, level: LogLevel) -> Result<(), XlogError> {
        if config.log_dir.is_empty() || config.name_prefix.is_empty() {
            return Err(XlogError::InvalidConfig);
        }
        let (cfg, _cstr) = config.to_sys();
        unsafe {
            sys::mars_xlog_appender_open(&cfg, level.as_sys() as c_int);
        }
        Ok(())
    }

    pub fn appender_close() {
        unsafe {
            sys::mars_xlog_appender_close();
        }
    }

    pub fn flush_all(sync: bool) {
        unsafe {
            sys::mars_xlog_flush_all(if sync { 1 } else { 0 });
        }
    }

    pub fn set_console_fun(fun: ConsoleFun) {
        unsafe {
            sys::mars_xlog_set_console_fun(fun as c_int);
        }
    }

    pub fn instance(&self) -> usize {
        self.inner.instance
    }

    pub fn is_enabled(&self, level: LogLevel) -> bool {
        unsafe { sys::mars_xlog_is_enabled(self.inner.instance, level.as_sys() as c_int) != 0 }
    }

    pub fn level(&self) -> LogLevel {
        match unsafe { sys::mars_xlog_get_level(self.inner.instance) } {
            0 => LogLevel::Verbose,
            1 => LogLevel::Debug,
            2 => LogLevel::Info,
            3 => LogLevel::Warn,
            4 => LogLevel::Error,
            5 => LogLevel::Fatal,
            _ => LogLevel::None,
        }
    }

    pub fn set_level(&self, level: LogLevel) {
        unsafe {
            sys::mars_xlog_set_level(self.inner.instance, level.as_sys() as c_int);
        }
    }

    pub fn set_appender_mode(&self, mode: AppenderMode) {
        unsafe {
            sys::mars_xlog_set_appender_mode(self.inner.instance, mode.as_sys() as c_int);
        }
    }

    pub fn flush(&self, sync: bool) {
        unsafe {
            sys::mars_xlog_flush(self.inner.instance, if sync { 1 } else { 0 });
        }
    }

    pub fn set_console_log_open(&self, open: bool) {
        unsafe {
            sys::mars_xlog_set_console_log_open(self.inner.instance, if open { 1 } else { 0 });
        }
    }

    pub fn set_max_file_size(&self, max_bytes: i64) {
        unsafe {
            sys::mars_xlog_set_max_file_size(self.inner.instance, max_bytes as _);
        }
    }

    pub fn set_max_alive_time(&self, alive_seconds: i64) {
        unsafe {
            sys::mars_xlog_set_max_alive_time(self.inner.instance, alive_seconds as _);
        }
    }

    pub fn write(&self, level: LogLevel, tag: Option<&str>, msg: &str) {
        if !self.is_enabled(level) {
            return;
        }
        self.write_with_meta(level, tag, file!(), module_path!(), line!(), msg);
    }

    pub fn write_with_meta(
        &self,
        level: LogLevel,
        tag: Option<&str>,
        file: &str,
        func: &str,
        line: u32,
        msg: &str,
    ) {
        if !self.is_enabled(level) {
            return;
        }

        let mut cstrings = Vec::new();
        let tag_ptr = tag
            .unwrap_or(&self.inner.name_prefix)
            .to_string();
        let tag_c = to_cstring(&tag_ptr, &mut cstrings);
        let file_c = to_cstring(file, &mut cstrings);
        let func_c = to_cstring(func, &mut cstrings);
        let msg_c = to_cstring(msg, &mut cstrings);

        let mut tv: timeval = unsafe { std::mem::zeroed() };
        unsafe {
            gettimeofday(&mut tv, ptr::null_mut());
        }

        let info = sys::XLoggerInfo {
            level: level.as_sys(),
            tag: tag_c,
            filename: file_c,
            func_name: func_c,
            line: line as c_int,
            timeval: tv,
            pid: -1,
            tid: -1,
            maintid: -1,
            traceLog: 0,
        };

        unsafe {
            sys::mars_xlog_write(self.inner.instance, &info, msg_c);
        }
    }

    pub fn current_log_path() -> Option<String> {
        read_path(|buf, len| unsafe { sys::mars_xlog_get_current_log_path(buf, len) })
    }

    pub fn current_log_cache_path() -> Option<String> {
        read_path(|buf, len| unsafe { sys::mars_xlog_get_current_log_cache_path(buf, len) })
    }

    pub fn filepaths_from_timespan(timespan: i32, prefix: &str) -> Vec<String> {
        read_joined(|buf, len| unsafe {
            sys::mars_xlog_get_filepath_from_timespan(timespan, cstr_or_null(prefix).as_ptr(), buf, len)
        })
    }

    pub fn make_logfile_name(timespan: i32, prefix: &str) -> Vec<String> {
        read_joined(|buf, len| unsafe {
            sys::mars_xlog_make_logfile_name(timespan, cstr_or_null(prefix).as_ptr(), buf, len)
        })
    }

    pub fn oneshot_flush(config: XlogConfig) -> Result<FileIoAction, XlogError> {
        if config.log_dir.is_empty() || config.name_prefix.is_empty() {
            return Err(XlogError::InvalidConfig);
        }
        let (cfg, _cstr) = config.to_sys();
        let mut action: c_int = 0;
        let ok = unsafe { sys::mars_xlog_oneshot_flush(&cfg, &mut action as *mut c_int) };
        if ok == 0 {
            return Err(XlogError::InitFailed);
        }
        Ok(FileIoAction::from(action))
    }

    pub fn dump(buffer: &[u8]) -> String {
        if buffer.is_empty() {
            return String::new();
        }
        unsafe {
            let ptr = sys::mars_xlog_dump(buffer.as_ptr().cast(), buffer.len());
            cstr_to_string(ptr)
        }
    }

    pub fn memory_dump(buffer: &[u8]) -> String {
        if buffer.is_empty() {
            return String::new();
        }
        unsafe {
            let ptr = sys::mars_xlog_memory_dump(buffer.as_ptr().cast(), buffer.len());
            cstr_to_string(ptr)
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ConsoleFun {
    Printf = 0,
    NSLog = 1,
    OSLog = 2,
}

fn read_path<F>(f: F) -> Option<String>
where
    F: Fn(*mut c_char, u32) -> i32,
{
    let mut buf = vec![0 as c_char; 4096];
    let ok = f(buf.as_mut_ptr(), buf.len() as u32);
    if ok == 0 {
        return None;
    }
    let cstr = unsafe { CStr::from_ptr(buf.as_ptr()) };
    cstr.to_str().ok().map(|s| s.to_string())
}

fn read_joined<F>(f: F) -> Vec<String>
where
    F: Fn(*mut c_char, usize) -> usize,
{
    let mut buf = vec![0 as c_char; 4096];
    let required = f(buf.as_mut_ptr(), buf.len());
    if required > buf.len() {
        buf.resize(required, 0);
        let _ = f(buf.as_mut_ptr(), buf.len());
    }
    let cstr = unsafe { CStr::from_ptr(buf.as_ptr()) };
    let s = cstr.to_string_lossy();
    if s.is_empty() {
        return Vec::new();
    }
    s.split('\n').map(|v| v.to_string()).collect()
}

fn to_cstring(s: &str, storage: &mut Vec<CString>) -> *const c_char {
    let clean = if s.as_bytes().contains(&0) {
        s.replace('\0', "")
    } else {
        s.to_string()
    };
    let c = CString::new(clean).unwrap_or_else(|_| CString::new("<invalid>").unwrap());
    let ptr = c.as_ptr();
    storage.push(c);
    ptr
}

fn cstr_or_null(s: &str) -> CStringHolder {
    CStringHolder::new(s)
}

fn cstr_to_string(ptr: *const c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    unsafe { CStr::from_ptr(ptr).to_string_lossy().to_string() }
}

struct CStringHolder {
    cstr: CString,
}

impl CStringHolder {
    fn new(s: &str) -> Self {
        let clean = if s.as_bytes().contains(&0) {
            s.replace('\0', "")
        } else {
            s.to_string()
        };
        let cstr = CString::new(clean).unwrap_or_else(|_| CString::new("").unwrap());
        Self { cstr }
    }

    fn as_ptr(&self) -> *const c_char {
        self.cstr.as_ptr()
    }
}

#[cfg(feature = "macros")]
#[macro_export]
macro_rules! xlog {
    ($logger:expr, $level:expr, $tag:expr, $($arg:tt)+) => {{
        let logger_ref = $logger;
        let level = $level;
        if logger_ref.is_enabled(level) {
            let msg = format!($($arg)+);
            logger_ref.write_with_meta(level, Some($tag), file!(), module_path!(), line!(), &msg);
        }
    }};
}

#[cfg(feature = "macros")]
#[macro_export]
macro_rules! xlog_debug {
    ($logger:expr, $tag:expr, $($arg:tt)+) => {{
        $crate::xlog!($logger, $crate::LogLevel::Debug, $tag, $($arg)+)
    }};
}

#[cfg(feature = "macros")]
#[macro_export]
macro_rules! xlog_info {
    ($logger:expr, $tag:expr, $($arg:tt)+) => {{
        $crate::xlog!($logger, $crate::LogLevel::Info, $tag, $($arg)+)
    }};
}

#[cfg(feature = "macros")]
#[macro_export]
macro_rules! xlog_warn {
    ($logger:expr, $tag:expr, $($arg:tt)+) => {{
        $crate::xlog!($logger, $crate::LogLevel::Warn, $tag, $($arg)+)
    }};
}

#[cfg(feature = "macros")]
#[macro_export]
macro_rules! xlog_error {
    ($logger:expr, $tag:expr, $($arg:tt)+) => {{
        $crate::xlog!($logger, $crate::LogLevel::Error, $tag, $($arg)+)
    }};
}
