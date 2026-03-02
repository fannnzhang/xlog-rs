use chrono::{DateTime, Datelike, Local, Timelike};

use crate::record::LogRecord;

/// Keep parity with Mars formatter's body cap behavior.
const MAX_LOG_BODY_BYTES: usize = 0xFFFF;

pub fn extract_file_name(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

fn truncate_utf8_to_max_bytes(input: &str, max_bytes: usize) -> &str {
    if input.len() <= max_bytes {
        return input;
    }

    let mut end = 0usize;
    for (idx, ch) in input.char_indices() {
        let next = idx + ch.len_utf8();
        if next > max_bytes {
            break;
        }
        end = next;
    }

    &input[..end]
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
    let body = truncate_utf8_to_max_bytes(body, MAX_LOG_BODY_BYTES);
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

    #[test]
    fn format_truncates_oversized_body_on_utf8_boundary() {
        let record = LogRecord::default();
        let body = "好".repeat(40_000); // 120_000 bytes, exceeds cap

        let line = format_record(&record, &body);
        assert!(line.ends_with('\n'));

        let payload = line.strip_suffix('\n').unwrap();
        let open = payload.rfind('[').unwrap();
        let body_out = &payload[open + 1..];
        assert!(body_out.len() <= super::MAX_LOG_BODY_BYTES);
        assert!(std::str::from_utf8(body_out.as_bytes()).is_ok());
    }
}
