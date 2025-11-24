use crate::backends::{PartitionTable, ProgressEvent};
use crate::common::Msg;
use crate::common::UiSender;
use fltk::{menu::Choice, prelude::MenuExt};

pub(crate) fn report_error(
    tx: crossbeam_channel::Sender<Msg>,
    operation: &str,
    error: anyhow::Error,
) {
    let err = error.to_string();
    tx.emit(Msg::Status(format!("{operation} failed: {err}")));
    tx.emit(Msg::Progress(ProgressEvent::Completed(Err(err))));
}

pub(crate) fn fill_size_choices(choice: &mut Choice, fs: Option<&str>) {
    choice.clear();
    choice.add_choice("Auto");
    match fs.unwrap_or("vfat") {
        "vfat" => {
            for spc in [1_u64, 2, 4, 8, 16, 32, 64, 128] {
                let unit = if spc == 1 { "sector" } else { "sectors" };
                choice.add_choice(&format!("{} {}", spc, unit));
            }
        }
        "exfat" | "ntfs" => {
            for sz in [
                4096_u64, 8192, 16384, 32768, 65536, 131072, 262144, 524288, 1048576,
            ] {
                choice.add_choice(&format!("{} bytes", sz));
            }
        }
        "ext4" | "xfs" => {
            for sz in [1024_u64, 2048, 4096] {
                choice.add_choice(&format!("{} bytes", sz));
            }
        }
        "btrfs" => {
            for sz in [4096_u64, 16384, 32768, 65536] {
                choice.add_choice(&format!("{} bytes", sz));
            }
        }
        _ => {}
    }
    choice.set_value(0);
}

pub(crate) fn size_label_text(fs: Option<&str>) -> &'static str {
    match fs.unwrap_or("vfat") {
        "vfat" => "Sectors per cluster",
        "exfat" | "ntfs" => "Cluster size (bytes)",
        "ext4" | "xfs" => "Block size (bytes)",
        "btrfs" => "Nodesize (bytes)",
        _ => "Allocation Unit Size",
    }
}

pub(crate) fn parse_partition_table_choice(choice: Option<&str>) -> Option<PartitionTable> {
    match choice {
        Some(s) if s.starts_with("MBR") => Some(PartitionTable::Dos),
        Some(_) => Some(PartitionTable::Gpt),
        None => Some(PartitionTable::Gpt),
    }
}
