use std::fs::File;

use memmap2::MmapOptions;

use crate::buffer::scan_recovery;
use crate::file_manager::FileManager;
use crate::protocol::{LogHeader, HEADER_LEN, MAGIC_END};
use crate::recovery::{build_sync_tip_block, current_mark_info};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
/// Result code returned by [`oneshot_flush`].
pub enum FileIoAction {
    /// No file action was taken.
    None = 0,
    /// The recovered bytes were flushed successfully.
    Success = 1,
    /// No mmap file was present or it contained no recoverable bytes.
    Unnecessary = 2,
    /// Opening the mmap file failed.
    OpenFailed = 3,
    /// Reading or mapping the mmap file failed.
    ReadFailed = 4,
    /// Writing recovered bytes to the logfile failed.
    WriteFailed = 5,
    /// Reserved for parity with historical file-action result codes.
    CloseFailed = 6,
    /// Removing the consumed mmap file failed.
    RemoveFailed = 7,
}

/// Drain another process's mmap buffer into the active logfile exactly once.
///
/// This is the Rust equivalent of Mars xlog's oneshot recovery path. It reads
/// the raw mmap bytes, recovers a pending block when possible, appends optional
/// begin/end marker blocks, durably syncs each appended recovery block, and
/// removes the mmap file only after the destination writes have been synced.
pub fn oneshot_flush(
    file_manager: &FileManager,
    mmap_capacity: usize,
    max_file_size: u64,
) -> FileIoAction {
    let mmap_path = file_manager.mmap_path();
    if !mmap_path.exists() {
        return FileIoAction::Unnecessary;
    }

    let f = match File::open(&mmap_path) {
        Ok(f) => f,
        Err(_) => return FileIoAction::OpenFailed,
    };

    let mmap_len = match f.metadata() {
        Ok(meta) => match usize::try_from(meta.len()) {
            Ok(len) => len,
            Err(_) => return FileIoAction::ReadFailed,
        },
        Err(_) => return FileIoAction::ReadFailed,
    };
    if mmap_len != mmap_capacity {
        return FileIoAction::ReadFailed;
    }
    let data = match unsafe { MmapOptions::new().len(mmap_capacity).map(&f) } {
        Ok(mapped) => mapped,
        Err(_) => return FileIoAction::ReadFailed,
    };

    let scan = scan_recovery(&data);
    if scan.valid_len == 0 {
        return FileIoAction::Unnecessary;
    }

    let sample_header = if scan.valid_len >= HEADER_LEN {
        LogHeader::decode(&data[..HEADER_LEN]).ok()
    } else {
        None
    };
    if let Some(begin) = build_sync_tip_block(
        sample_header,
        "~~~~~ begin of mmap from other process ~~~~~\n",
    ) {
        if append_recovered_bytes_durable(file_manager, &begin, max_file_size).is_err() {
            return FileIoAction::WriteFailed;
        }
    }

    if scan.recovered_pending_block {
        // Keep the recovered block contiguous so another process cannot
        // interleave between payload bytes and the repaired tail marker.
        let mut recovered = Vec::with_capacity(scan.valid_len.saturating_add(1));
        recovered.extend_from_slice(&data[..scan.valid_len]);
        recovered.push(MAGIC_END);
        if append_recovered_bytes_durable(file_manager, &recovered, max_file_size).is_err() {
            return FileIoAction::WriteFailed;
        }
    } else if append_recovered_bytes_durable(file_manager, &data[..scan.valid_len], max_file_size)
        .is_err()
    {
        return FileIoAction::WriteFailed;
    }
    let end_tip = format!(
        "~~~~~ end of mmap from other process ~~~~~{}\n",
        current_mark_info()
    );
    if let Some(end) = build_sync_tip_block(sample_header, &end_tip) {
        if append_recovered_bytes_durable(file_manager, &end, max_file_size).is_err() {
            return FileIoAction::WriteFailed;
        }
    }

    drop(data);
    if std::fs::remove_file(&mmap_path).is_err() {
        return FileIoAction::RemoveFailed;
    }

    FileIoAction::Success
}

fn append_recovered_bytes_durable(
    file_manager: &FileManager,
    bytes: &[u8],
    max_file_size: u64,
) -> Result<(), crate::file_manager::FileManagerError> {
    file_manager.append_log_bytes_durable(bytes, max_file_size, false)
}
