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

pub fn write_console_line(_level: ConsoleLevel, line: &str) {
    if line.is_empty() {
        return;
    }

    #[cfg(target_os = "android")]
    {
        println!("{line}");
    }

    #[cfg(not(target_os = "android"))]
    {
        eprintln!("{line}");
    }
}
