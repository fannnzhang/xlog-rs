#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

use libc::{c_char, c_int, c_long, c_uint, c_void, intmax_t, size_t, timeval};

#[repr(i32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TLogLevel {
    kLevelVerbose = 0,
    kLevelDebug = 1,
    kLevelInfo = 2,
    kLevelWarn = 3,
    kLevelError = 4,
    kLevelFatal = 5,
    kLevelNone = 6,
}

pub const TLOGLEVEL_ALL: c_int = 0;

#[repr(i32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TAppenderMode {
    kAppenderAsync = 0,
    kAppenderSync = 1,
}

#[repr(i32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TCompressMode {
    kZlib = 0,
    kZstd = 1,
}

#[repr(i32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TFileIOAction {
    kActionNone = 0,
    kActionSuccess = 1,
    kActionUnnecessary = 2,
    kActionOpenFailed = 3,
    kActionReadFailed = 4,
    kActionWriteFailed = 5,
    kActionCloseFailed = 6,
    kActionRemoveFailed = 7,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct XLoggerInfo {
    pub level: TLogLevel,
    pub tag: *const c_char,
    pub filename: *const c_char,
    pub func_name: *const c_char,
    pub line: c_int,
    pub timeval: timeval,
    pub pid: intmax_t,
    pub tid: intmax_t,
    pub maintid: intmax_t,
    pub traceLog: c_int,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct MarsXlogConfig {
    pub mode: c_int,
    pub logdir: *const c_char,
    pub nameprefix: *const c_char,
    pub pub_key: *const c_char,
    pub compress_mode: c_int,
    pub compress_level: c_int,
    pub cache_dir: *const c_char,
    pub cache_days: c_int,
}

extern "C" {
    pub fn mars_xlog_new_instance(cfg: *const MarsXlogConfig, level: c_int) -> usize;
    pub fn mars_xlog_get_instance(nameprefix: *const c_char) -> usize;
    pub fn mars_xlog_release_instance(nameprefix: *const c_char);

    pub fn mars_xlog_appender_open(cfg: *const MarsXlogConfig, level: c_int);
    pub fn mars_xlog_appender_close();

    pub fn mars_xlog_write(instance: usize, info: *const XLoggerInfo, log: *const c_char);
    pub fn mars_xlog_is_enabled(instance: usize, level: c_int) -> c_int;
    pub fn mars_xlog_get_level(instance: usize) -> c_int;
    pub fn mars_xlog_set_level(instance: usize, level: c_int);

    pub fn mars_xlog_set_appender_mode(instance: usize, mode: c_int);
    pub fn mars_xlog_flush(instance: usize, is_sync: c_int);
    pub fn mars_xlog_flush_all(is_sync: c_int);
    pub fn mars_xlog_set_console_log_open(instance: usize, is_open: c_int);
    pub fn mars_xlog_set_max_file_size(instance: usize, max_file_size: c_long);
    pub fn mars_xlog_set_max_alive_time(instance: usize, alive_seconds: c_long);

    pub fn mars_xlog_get_current_log_path(buf: *mut c_char, len: c_uint) -> c_int;
    pub fn mars_xlog_get_current_log_cache_path(buf: *mut c_char, len: c_uint) -> c_int;

    pub fn mars_xlog_get_filepath_from_timespan(
        timespan: c_int,
        prefix: *const c_char,
        buf: *mut c_char,
        len: size_t,
    ) -> size_t;
    pub fn mars_xlog_make_logfile_name(
        timespan: c_int,
        prefix: *const c_char,
        buf: *mut c_char,
        len: size_t,
    ) -> size_t;

    pub fn mars_xlog_oneshot_flush(cfg: *const MarsXlogConfig, result_action: *mut c_int) -> c_int;

    pub fn mars_xlog_dump(buffer: *const c_void, len: size_t) -> *const c_char;
    pub fn mars_xlog_memory_dump(buffer: *const c_void, len: size_t) -> *const c_char;

    pub fn mars_xlog_set_console_fun(fun: c_int);
}
