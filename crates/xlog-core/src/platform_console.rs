#[cfg(target_os = "android")]
use std::ffi::CString;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ConsoleLevel {
    Verbose,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
    None,
}

pub fn write_console_line(level: ConsoleLevel, line: &str) {
    if line.is_empty() {
        return;
    }
    #[cfg(not(target_os = "android"))]
    let _ = level;

    #[cfg(target_os = "android")]
    {
        write_android_line(level, line);
    }

    #[cfg(not(target_os = "android"))]
    {
        eprintln!("{line}");
    }
}

#[cfg(target_os = "android")]
fn write_android_line(level: ConsoleLevel, line: &str) {
    const TAG: &[u8] = b"mars-xlog\0";
    let msg = line.replace('\0', " ");
    let c_msg = CString::new(msg).expect("nul bytes replaced");
    unsafe {
        __android_log_write(android_priority(level), TAG.as_ptr().cast(), c_msg.as_ptr());
    }
}

#[cfg(target_os = "android")]
fn android_priority(level: ConsoleLevel) -> i32 {
    match level {
        ConsoleLevel::Verbose => 2, // ANDROID_LOG_VERBOSE
        ConsoleLevel::Debug => 3,   // ANDROID_LOG_DEBUG
        ConsoleLevel::Info => 4,    // ANDROID_LOG_INFO
        ConsoleLevel::Warn => 5,    // ANDROID_LOG_WARN
        ConsoleLevel::Error => 6,   // ANDROID_LOG_ERROR
        ConsoleLevel::Fatal => 7,   // ANDROID_LOG_FATAL
        ConsoleLevel::None => 4,    // ANDROID_LOG_INFO
    }
}

#[cfg(target_os = "android")]
unsafe extern "C" {
    fn __android_log_write(prio: i32, tag: *const libc::c_char, text: *const libc::c_char) -> i32;
}
