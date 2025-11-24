pub(crate) mod mock;
pub(crate) mod udisks;
use anyhow::Result;
use async_trait::async_trait;

#[derive(Clone, Debug)]
pub(crate) struct BlockDevice {
    pub(crate) dev_path: String,
    pub(crate) object_path: String,
    pub(crate) fs_type: Option<String>,
    pub(crate) label: Option<String>,
    pub(crate) size_bytes: Option<u64>,
    pub(crate) vendor_model: Option<String>,
    pub(crate) is_partition: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct FormatOptions {
    pub(crate) fs: String,
    pub(crate) label: Option<String>,
    pub(crate) quick: bool,
    pub(crate) cluster_or_block_size: Option<u64>,
    pub(crate) partition_table: Option<PartitionTable>,
}

#[derive(Clone, Debug)]
pub(crate) enum ProgressEvent {
    JobStarted(String),
    Percent(f64),
    RateBytesPerSec(u64),
    Message(String),
    Completed(Result<(), String>),
}

#[derive(Clone, Debug)]
pub(crate) enum PartitionTable {
    Gpt,
    Dos,
}

#[async_trait]
pub(crate) trait Backend: Sync + Send {
    async fn list_block_devices(&self) -> Result<Vec<BlockDevice>>;
    async fn format(&self, obj_path: &str, opts: FormatOptions) -> Result<String>;
    async fn cancel(&self, job_id: &str) -> Result<()>;
}

pub(crate) fn human_size(size: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    const SI_UNIT_BASE: f64 = 1000.0;

    let mut s = size as f64;
    let mut i = 0;
    while s >= SI_UNIT_BASE && i < UNITS.len() - 1 {
        s /= SI_UNIT_BASE;
        i += 1;
    }
    if i == 0 {
        format!("{size} B")
    } else {
        format!("{:.1} {}", s, UNITS[i])
    }
}
