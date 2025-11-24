use crate::backends::{BlockDevice, human_size};
use crate::backends::{FormatOptions, PartitionTable};
use std::process::{Command, Stdio};

const FAT_INVALID_CHARS: [char; 10] = ['"', '*', '/', ':', '<', '>', '?', '\\', '|', '\0'];

fn has_fat_invalid_chars(label: &str) -> bool {
    label
        .chars()
        .any(|c| FAT_INVALID_CHARS.contains(&c) || c.is_control())
}

fn mkfs_present(fs: &str) -> bool {
    let candidates: &[&[&str]] = match fs {
        "vfat" => &[&["mkfs.vfat"]],
        "exfat" => &[&["mkfs.exfat"]],
        "ntfs" => &[&["mkfs.ntfs"], &["mkntfs"]],
        "ext4" => &[&["mkfs.ext4"], &["mke2fs"]],
        "xfs" => &[&["mkfs.xfs"]],
        "btrfs" => &[&["mkfs.btrfs"]],
        _ => &[],
    };
    for group in candidates {
        for bin in *group {
            if which(bin) {
                return true;
            }
        }
    }
    false
}

fn which(bin: &str) -> bool {
    Command::new("which")
        .arg(bin)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub(crate) fn detect_supported_fs() -> Vec<&'static str> {
    let all = ["exfat", "vfat", "ntfs", "ext4", "xfs", "btrfs"];
    all.into_iter().filter(|fs| mkfs_present(fs)).collect()
}

pub(crate) fn device_display(dev: &BlockDevice) -> String {
    let mut extras: Vec<String> = Vec::new();
    extras.push(
        if dev.is_partition {
            "Partition"
        } else {
            "Disk"
        }
        .to_string(),
    );
    if let Some(size) = dev.size_bytes {
        extras.push(human_size(size));
    }
    if let Some(vm) = &dev.vendor_model {
        extras.push(vm.clone());
    }
    if let Some(fs) = &dev.fs_type {
        if !fs.is_empty() {
            extras.push(fs.clone());
        }
    }
    if let Some(lbl) = &dev.label {
        if !lbl.is_empty() {
            extras.push(format!("\"{}\"", lbl));
        }
    }
    let base = if !dev.dev_path.is_empty() {
        &dev.dev_path
    } else {
        &dev.object_path
    };
    if extras.is_empty() {
        base.to_string()
    } else {
        format!("{} ({})", base, extras.join(", "))
    }
}

pub(crate) fn default_fs(supported: &[&str]) -> Option<&'static str> {
    ["exfat", "vfat", "ext4", "ntfs", "xfs", "btrfs"]
        .into_iter()
        .find(|&pref| supported.contains(&pref))
        .map(|v| v as _)
}

fn validate_label(label: &str, fs: &str) -> Option<String> {
    if label.is_empty() {
        return None;
    }

    match fs {
        "vfat" => {
            if label.len() > 11 {
                return Some("vfat: max 11 bytes".to_string());
            }
            if has_fat_invalid_chars(label) {
                return Some("vfat: invalid characters".to_string());
            }
        }
        "exfat" => {
            if label.chars().count() > 15 {
                return Some("exfat: max 15 characters".to_string());
            }
            if has_fat_invalid_chars(label) {
                return Some("exfat: invalid characters".to_string());
            }
        }
        "ntfs" => {
            if label.chars().count() > 32 {
                return Some("ntfs: max 32 characters".to_string());
            }
            if label.contains('\0') {
                return Some("ntfs: invalid characters".to_string());
            }
        }
        "ext4" => {
            if label.len() > 16 {
                return Some("ext4: max 16 bytes".to_string());
            }
            if label.contains('\0') || label.contains('/') {
                return Some("ext4: invalid characters".to_string());
            }
        }
        "xfs" => {
            if label.len() > 12 {
                return Some("xfs: max 12 bytes".to_string());
            }
            if label.contains('\0') {
                return Some("xfs: invalid characters".to_string());
            }
        }
        "btrfs" => {
            if label.len() > 255 {
                return Some("btrfs: max 255 bytes".to_string());
            }
            if label.contains('\0') {
                return Some("btrfs: invalid characters".to_string());
            }
        }
        _ => {}
    }

    None
}

pub(crate) fn parse_size_choice_label(label: Option<&str>) -> Option<u64> {
    match label {
        Some("Auto") => None,
        Some(s) => s
            .split_whitespace()
            .next()
            .and_then(|n| n.parse::<u64>().ok()),
        None => None,
    }
}

pub(crate) fn build_format_options(
    fs: String,
    label: Option<String>,
    quick: bool,
    cluster_or_block_size: Option<u64>,
    partition_table: Option<PartitionTable>,
) -> Result<FormatOptions, String> {
    if let Some(ref lbl) = label {
        if let Some(err) = validate_label(lbl, &fs) {
            return Err(err);
        }
    }
    Ok(FormatOptions {
        fs,
        label,
        quick,
        cluster_or_block_size,
        partition_table,
    })
}
