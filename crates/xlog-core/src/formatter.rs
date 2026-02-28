use chrono::{DateTime, Datelike, Local, Timelike};

use crate::record::LogRecord;

pub fn extract_file_name(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

fn format_time(ts: std::time::SystemTime) -> String {
    let dt: DateTime<Local> = ts.into();
    let offset_hours = (dt.offset().local_minus_utc() as f64) / 3600.0;
    format!(
        "{:04}-{:02}-{:02} {:+.1} {:02}:{:02}:{:02}.{:03}",
        dt.year(),
        dt.month(),
        dt.day(),
        offset_hours,
        dt.hour(),
        dt.minute(),
        dt.second(),
        dt.timestamp_subsec_millis()
    )
}

/// Reproduce C++ `formater.cc` output layout as one text line.
pub fn format_record(record: &LogRecord, body: &str) -> String {
    let filename = extract_file_name(&record.filename);
    let tid_suffix = if record.tid == record.maintid {
        "*"
    } else {
        ""
    };
    let func_name = if record.func_name.is_empty() {
        ""
    } else {
        &record.func_name
    };
    let mut out = String::with_capacity(256 + body.len());
    out.push_str(&format!(
        "[{}][{}][{}, {}{}][{}][{}:{}, {}][",
        record.level.short(),
        format_time(record.timestamp),
        record.pid,
        record.tid,
        tid_suffix,
        record.tag,
        filename,
        record.line,
        func_name
    ));
    out.push_str(body);
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, UNIX_EPOCH};

    use super::format_record;
    use crate::record::{LogLevel, LogRecord};

    #[test]
    fn format_includes_expected_fields() {
        let record = LogRecord {
            level: LogLevel::Error,
            tag: "core".to_string(),
            filename: "/a/b/c.rs".to_string(),
            func_name: "module::f".to_string(),
            line: 42,
            timestamp: UNIX_EPOCH + Duration::from_secs(1_700_000_000) + Duration::from_millis(123),
            pid: 12,
            tid: 34,
            maintid: 34,
        };

        let line = format_record(&record, "msg");
        assert!(line.starts_with("[E]["));
        assert!(line.contains("[12, 34*]"));
        assert!(line.contains("[core]"));
        assert!(line.contains("[c.rs:42, module::f]"));
        assert!(line.ends_with("msg\n"));
    }
}
