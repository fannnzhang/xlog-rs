# mars-xlog-sys

Low-level FFI bindings to the Tencent Mars Xlog C++ logging library (vendored in
`third_party/mars`). This crate intentionally exposes the raw C ABI and is unsafe to
use directly. Prefer the safe wrapper in `crates/xlog` unless you need the FFI.

## Overview
- `MarsXlogConfig` configures log directories, prefixes, compression, and cache behavior.
- `mars_xlog_new_instance` / `mars_xlog_release_instance` manage per-prefix instances.
- `mars_xlog_appender_open` / `mars_xlog_appender_close` manage the global appender.
- `mars_xlog_write` writes log entries to an instance or the global logger.

## Safety notes
- All pointers must be valid for the duration of the call.
- String pointers must be NUL-terminated C strings.
- Buffer-return APIs write a NUL-terminated string into the provided buffer.
- `mars_xlog_get_filepath_from_timespan` and `mars_xlog_make_logfile_name` return the
  required buffer size (including the trailing NUL) and join multiple paths with `\n`.
- `mars_xlog_dump` and `mars_xlog_memory_dump` return thread-local buffers; copy them
  immediately and do not free them.
- If you want Mars to fill `pid`/`tid`/`maintid`, set all three to `-1`.

## Units and conventions
- `timespan` is in days (0 = today, 1 = yesterday, etc).
- `max_file_size` is in bytes (0 disables splitting).
- `max_alive_time` is in seconds (default in Mars is 10 days).

## Platform notes
- `mars_xlog_set_console_fun` only has effect on Apple platforms; it is a no-op elsewhere.

## Minimal flow (unsafe)
```rust
use libc::c_int;
use mars_xlog_sys as sys;
use std::ffi::CString;

let logdir = CString::new("/tmp/xlog").unwrap();
let prefix = CString::new("app").unwrap();

let cfg = sys::MarsXlogConfig {
    mode: sys::TAppenderMode::kAppenderAsync as c_int,
    logdir: logdir.as_ptr(),
    nameprefix: prefix.as_ptr(),
    pub_key: std::ptr::null(),
    compress_mode: sys::TCompressMode::kZlib as c_int,
    compress_level: 6,
    cache_dir: std::ptr::null(),
    cache_days: 0,
};

let instance = unsafe { sys::mars_xlog_new_instance(&cfg, sys::TLogLevel::kLevelInfo as c_int) };
if instance != 0 {
    // ...
    unsafe { sys::mars_xlog_release_instance(prefix.as_ptr()) };
}
```
