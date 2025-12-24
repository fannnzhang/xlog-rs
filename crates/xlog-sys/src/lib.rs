#![doc = include_str!("../README.md")]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

use libc::{c_char, c_int, c_long, c_uint, c_void, intmax_t, size_t, timeval};

/// Log severity used by Mars Xlog.
///
/// Values match the C `TLogLevel` enum from `mars/comm/xlogger/xloggerbase.h`.
#[repr(i32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TLogLevel {
    /// Most verbose level; also used as "all".
    kLevelVerbose = 0,
    /// Detailed debug information.
    kLevelDebug = 1,
    /// Informational runtime events.
    kLevelInfo = 2,
    /// Unexpected but recoverable situations.
    kLevelWarn = 3,
    /// Errors indicating failed operations.
    kLevelError = 4,
    /// Severe errors that usually precede termination.
    kLevelFatal = 5,
    /// Disable all log output.
    kLevelNone = 6,
}

/// Alias for the "all"/verbose log level (`kLevelVerbose`).
pub const TLOGLEVEL_ALL: c_int = 0;

/// Appender mode controlling how logs are written.
#[repr(i32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TAppenderMode {
    /// Buffer logs and flush them from a background thread.
    kAppenderAsync = 0,
    /// Write logs synchronously on the caller thread.
    kAppenderSync = 1,
}

/// Compression algorithm for log buffers/files.
#[repr(i32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TCompressMode {
    /// Zlib compression.
    kZlib = 0,
    /// Zstd compression.
    kZstd = 1,
}

/// Result code used by `mars_xlog_oneshot_flush`.
#[repr(i32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TFileIOAction {
    /// No action performed.
    kActionNone = 0,
    /// Action completed successfully.
    kActionSuccess = 1,
    /// No action required (for example, nothing to flush).
    kActionUnnecessary = 2,
    /// Failed to open a file.
    kActionOpenFailed = 3,
    /// Failed to read a file.
    kActionReadFailed = 4,
    /// Failed to write a file.
    kActionWriteFailed = 5,
    /// Failed to close a file.
    kActionCloseFailed = 6,
    /// Failed to remove a file.
    kActionRemoveFailed = 7,
}

/// Metadata describing a single log entry.
///
/// Pointer fields may be null. When non-null they must be valid NUL-terminated C strings
/// for the duration of the call into the C++ library. Although this struct is often passed
/// as `*const`, the C++ implementation may still mutate it (for example to fill pid/tid).
#[repr(C)]
#[derive(Copy, Clone)]
pub struct XLoggerInfo {
    /// Log level for this entry.
    pub level: TLogLevel,
    /// Optional tag/category string.
    pub tag: *const c_char,
    /// Optional source file path.
    pub filename: *const c_char,
    /// Optional function name.
    pub func_name: *const c_char,
    /// Source line number.
    pub line: c_int,
    /// Timestamp for the log entry.
    pub timeval: timeval,
    /// Process id (set all of pid/tid/maintid to -1 to let Mars fill them).
    pub pid: intmax_t,
    /// Thread id.
    pub tid: intmax_t,
    /// Main thread id.
    pub maintid: intmax_t,
    /// When set to 1 on Android, forces console output even if console logging is closed.
    pub traceLog: c_int,
}

/// Configuration used to create instances or open the global appender.
///
/// String pointers are copied into the C++ configuration during the call. After the call
/// returns, the Rust strings may be freed.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct MarsXlogConfig {
    /// Appender mode (`TAppenderMode` as int).
    pub mode: c_int,
    /// Directory for log files (required).
    pub logdir: *const c_char,
    /// Prefix for log file names and the instance name (required).
    pub nameprefix: *const c_char,
    /// Optional public key (hex string, 128 chars) enabling log encryption.
    pub pub_key: *const c_char,
    /// Compression mode (`TCompressMode` as int).
    pub compress_mode: c_int,
    /// Compression level forwarded to the compressor.
    pub compress_level: c_int,
    /// Optional cache directory for mmap buffers and temporary logs.
    pub cache_dir: *const c_char,
    /// Days to keep cached logs before moving them to `logdir` (0 disables).
    pub cache_days: c_int,
}

extern "C" {
    /// Create a new Xlog instance and return an opaque handle.
    ///
    /// Returns 0 on failure.
    ///
    /// # Safety
    /// - `cfg` must be a valid pointer to `MarsXlogConfig` for the duration of the call.
    /// - Any string pointers inside `cfg` must be valid NUL-terminated C strings.
    /// - `level` must be a valid `TLogLevel` value.
    pub fn mars_xlog_new_instance(cfg: *const MarsXlogConfig, level: c_int) -> usize;

    /// Look up an existing instance by `nameprefix`.
    ///
    /// Returns 0 if the instance does not exist.
    ///
    /// # Safety
    /// - `nameprefix` must be a valid NUL-terminated C string.
    pub fn mars_xlog_get_instance(nameprefix: *const c_char) -> usize;

    /// Release the instance associated with `nameprefix`.
    ///
    /// After this call, any handles obtained for the same `nameprefix` become invalid.
    ///
    /// # Safety
    /// - `nameprefix` must be a valid NUL-terminated C string.
    pub fn mars_xlog_release_instance(nameprefix: *const c_char);

    /// Open the global appender (default instance).
    ///
    /// # Safety
    /// - `cfg` must be a valid pointer to `MarsXlogConfig` for the duration of the call.
    /// - Any string pointers inside `cfg` must be valid NUL-terminated C strings.
    /// - `level` must be a valid `TLogLevel` value.
    pub fn mars_xlog_appender_open(cfg: *const MarsXlogConfig, level: c_int);

    /// Close the global appender.
    pub fn mars_xlog_appender_close();

    /// Write a log entry.
    ///
    /// `instance` is an opaque handle returned by `mars_xlog_new_instance` or
    /// `mars_xlog_get_instance`. Passing 0 writes to the global/default logger.
    ///
    /// # Safety
    /// - `instance` must be 0 or a valid handle returned by this library.
    /// - `info` must point to writable memory for the duration of the call if non-null.
    /// - `log` must be a valid NUL-terminated C string if non-null.
    /// - The C++ library may mutate `info` to fill pid/tid/maintid if they are all -1.
    pub fn mars_xlog_write(instance: usize, info: *const XLoggerInfo, log: *const c_char);

    /// Returns non-zero if logging at `level` is enabled for `instance`.
    ///
    /// # Safety
    /// - `instance` must be 0 or a valid handle returned by this library.
    /// - `level` must be a valid `TLogLevel` value.
    pub fn mars_xlog_is_enabled(instance: usize, level: c_int) -> c_int;

    /// Get the current log level for `instance`.
    ///
    /// # Safety
    /// - `instance` must be 0 or a valid handle returned by this library.
    pub fn mars_xlog_get_level(instance: usize) -> c_int;

    /// Set the log level for `instance`.
    ///
    /// # Safety
    /// - `instance` must be 0 or a valid handle returned by this library.
    /// - `level` must be a valid `TLogLevel` value.
    pub fn mars_xlog_set_level(instance: usize, level: c_int);

    /// Set the appender mode for `instance` (`TAppenderMode` as int).
    ///
    /// # Safety
    /// - `instance` must be 0 or a valid handle returned by this library.
    pub fn mars_xlog_set_appender_mode(instance: usize, mode: c_int);

    /// Flush pending logs for `instance`.
    ///
    /// # Safety
    /// - `instance` must be 0 or a valid handle returned by this library.
    /// - `is_sync` is treated as a boolean (0 or non-zero).
    pub fn mars_xlog_flush(instance: usize, is_sync: c_int);

    /// Flush pending logs for all instances.
    ///
    /// # Safety
    /// - `is_sync` is treated as a boolean (0 or non-zero).
    pub fn mars_xlog_flush_all(is_sync: c_int);

    /// Enable or disable console logging for `instance`.
    ///
    /// # Safety
    /// - `instance` must be 0 or a valid handle returned by this library.
    /// - `is_open` is treated as a boolean (0 or non-zero).
    pub fn mars_xlog_set_console_log_open(instance: usize, is_open: c_int);

    /// Set the maximum size (in bytes) of a single log file for `instance`.
    ///
    /// A value of 0 disables splitting.
    ///
    /// # Safety
    /// - `instance` must be 0 or a valid handle returned by this library.
    pub fn mars_xlog_set_max_file_size(instance: usize, max_file_size: c_long);

    /// Set the maximum lifetime (in seconds) of a log file for `instance`.
    ///
    /// Values below 1 day are ignored by Mars.
    ///
    /// # Safety
    /// - `instance` must be 0 or a valid handle returned by this library.
    pub fn mars_xlog_set_max_alive_time(instance: usize, alive_seconds: c_long);

    /// Get the current log path for the global appender.
    ///
    /// On success, writes a NUL-terminated string into `buf` and returns non-zero.
    ///
    /// # Safety
    /// - `buf` must point to writable memory of at least `len` bytes.
    pub fn mars_xlog_get_current_log_path(buf: *mut c_char, len: c_uint) -> c_int;

    /// Get the current cache log path for the global appender.
    ///
    /// On success, writes a NUL-terminated string into `buf` and returns non-zero.
    ///
    /// # Safety
    /// - `buf` must point to writable memory of at least `len` bytes.
    pub fn mars_xlog_get_current_log_cache_path(buf: *mut c_char, len: c_uint) -> c_int;

    /// Get log file paths from a timespan.
    ///
    /// `timespan` is in days (0 = today, 1 = yesterday, etc). Paths are joined with '\n'.
    /// Returns the required buffer length (including the trailing NUL), even if `buf` is null.
    ///
    /// # Safety
    /// - `prefix` may be null or a valid NUL-terminated C string.
    /// - `buf` must point to writable memory of at least `len` bytes if non-null.
    pub fn mars_xlog_get_filepath_from_timespan(
        timespan: c_int,
        prefix: *const c_char,
        buf: *mut c_char,
        len: size_t,
    ) -> size_t;

    /// Generate log file names for a timespan.
    ///
    /// `timespan` is in days (0 = today, 1 = yesterday, etc). Paths are joined with '\n'.
    /// Returns the required buffer length (including the trailing NUL), even if `buf` is null.
    ///
    /// # Safety
    /// - `prefix` may be null or a valid NUL-terminated C string.
    /// - `buf` must point to writable memory of at least `len` bytes if non-null.
    pub fn mars_xlog_make_logfile_name(
        timespan: c_int,
        prefix: *const c_char,
        buf: *mut c_char,
        len: size_t,
    ) -> size_t;

    /// Flush mmap buffers to log files without opening the global appender.
    ///
    /// Returns non-zero on success. `result_action` receives a `TFileIOAction` code.
    ///
    /// # Safety
    /// - `cfg` must be a valid pointer to `MarsXlogConfig` for the duration of the call.
    /// - Any string pointers inside `cfg` must be valid NUL-terminated C strings.
    /// - `result_action` must be a valid pointer if non-null.
    pub fn mars_xlog_oneshot_flush(cfg: *const MarsXlogConfig, result_action: *mut c_int) -> c_int;

    /// Dump a buffer to a file under the configured log directory and return a summary string.
    ///
    /// The returned pointer refers to a thread-local buffer owned by the C++ library. Copy it
    /// immediately; do not free it.
    ///
    /// # Safety
    /// - `buffer` must point to at least `len` bytes of readable memory.
    pub fn mars_xlog_dump(buffer: *const c_void, len: size_t) -> *const c_char;

    /// Return a formatted hex dump of `buffer` without writing a file.
    ///
    /// The returned pointer refers to a thread-local buffer owned by the C++ library. Copy it
    /// immediately; do not free it.
    ///
    /// # Safety
    /// - `buffer` must point to at least `len` bytes of readable memory.
    pub fn mars_xlog_memory_dump(buffer: *const c_void, len: size_t) -> *const c_char;

    /// Select the console logging backend on Apple platforms.
    ///
    /// `fun` matches `mars::xlog::TConsoleFun`:
    /// 0 = printf, 1 = NSLog, 2 = OSLog. This is a no-op on non-Apple platforms.
    pub fn mars_xlog_set_console_fun(fun: c_int);
}
