use super::{Backend, BlockDevice, FormatOptions, ProgressEvent};
use crate::common::{Msg, UiSender};
use anyhow::{Result, bail};
use async_trait::async_trait;
use fudisks as ud;
use futures_util::StreamExt;

pub(crate) struct UdisksBackend {
    ud: ud::Udisks,
    ui_tx: crossbeam_channel::Sender<Msg>,
}

impl UdisksBackend {
    pub(crate) async fn new(ui_tx: crossbeam_channel::Sender<Msg>) -> Result<Self> {
        let ud = ud::Udisks::connect_system().await?;
        //  Quietly check we actually have a udisks2 service!
        ud.list_devices().await.map_err(anyhow::Error::from)?;
        Ok(Self { ud, ui_tx })
    }

    async fn forward_progress_until_complete(
        handle: ud::JobHandle,
        tx: crossbeam_channel::Sender<Msg>,
    ) -> Result<()> {
        let mut stream = handle.watch();
        while let Some(evt) = stream.next().await {
            match evt {
                ud::JobEvent::Percent(p) => {
                    tx.emit(Msg::Progress(ProgressEvent::Percent(p)));
                }
                ud::JobEvent::RateBytesPerSec(r) => {
                    tx.emit(Msg::Progress(ProgressEvent::RateBytesPerSec(r)));
                }
                ud::JobEvent::Completed(res) => match res {
                    Ok(()) => {
                        tx.emit(Msg::Progress(ProgressEvent::Completed(Ok(()))));
                        return Ok(());
                    }
                    Err(e) => {
                        tx.emit(Msg::Progress(ProgressEvent::Completed(Err(e.to_string()))));
                        bail!(e);
                    }
                },
            }
        }
        bail!("UDisks job ended unexpectedly without completion");
    }

    fn to_ud_opts(opts: &FormatOptions) -> Result<ud::FormatOptions> {
        let label = opts.label.clone();
        let quick = opts.quick;
        let sz = opts.cluster_or_block_size;
        match opts.fs.as_str() {
            "ext4" => Ok(ud::FormatOptions::Ext4 {
                label,
                block_size: sz,
                quick,
            }),
            "xfs" => Ok(ud::FormatOptions::Xfs {
                label,
                block_size: sz,
                quick,
            }),
            "btrfs" => Ok(ud::FormatOptions::Btrfs {
                label,
                nodesize: sz,
                quick,
            }),
            "exfat" => Ok(ud::FormatOptions::Exfat {
                label,
                cluster_size: sz,
                quick,
            }),
            "ntfs" => Ok(ud::FormatOptions::Ntfs {
                label,
                cluster_size: sz,
                quick,
            }),
            "vfat" => Ok(ud::FormatOptions::Vfat {
                label,
                sectors_per_cluster: sz,
                quick,
            }),
            other => bail!("Unsupported filesystem: {other}"),
        }
    }
}

#[async_trait]
impl Backend for UdisksBackend {
    async fn list_block_devices(&self) -> Result<Vec<BlockDevice>> {
        let devs = self.ud.list_devices().await.map_err(anyhow::Error::from)?;
        let out = devs
            .into_iter()
            .filter(|d| {
                if d.is_optical || d.dev_path.starts_with("/dev/sr") {
                    return false;
                }
                if !d.is_removable {
                    return false;
                }
                true
            })
            .map(|d| BlockDevice {
                dev_path: d.dev_path,
                object_path: d.object_path,
                fs_type: d.fs_type,
                label: d.label,
                size_bytes: d.size_bytes,
                vendor_model: d.vendor_model,
                is_partition: d.is_partition,
            })
            .collect();
        Ok(out)
    }

    async fn format(&self, obj_path: &str, opts: super::FormatOptions) -> Result<String> {
        let ud_opts = Self::to_ud_opts(&opts)?;
        if self
            .ud
            .is_partition(obj_path)
            .await
            .map_err(anyhow::Error::from)?
        {
            let handle = self
                .ud
                .format_partition(obj_path, &ud_opts)
                .await
                .map_err(anyhow::Error::from)?;
            let job_id = handle.path().to_string();
            let _ = self
                .ui_tx
                .emit(Msg::Progress(ProgressEvent::JobStarted(job_id.clone())));
            Self::forward_progress_until_complete(handle, self.ui_tx.clone()).await?;
            Ok(obj_path.to_string())
        } else {
            let table = match opts.partition_table {
                Some(crate::backends::PartitionTable::Dos) => ud::PartitionTable::Dos,
                _ => ud::PartitionTable::Gpt,
            };
            let _ = self.ui_tx.emit(Msg::Progress(ProgressEvent::Message(
                "Creating partition table...".into(),
            )));
            let (new_part_path, handle) = self
                .ud
                .format_block_device_with_table(obj_path, table, &ud_opts, true)
                .await
                .map_err(anyhow::Error::from)?;
            let _ = self.ui_tx.emit(Msg::Progress(ProgressEvent::Message(
                "Formatting partition...".into(),
            )));
            let job_id = handle.path().to_string();
            let _ = self
                .ui_tx
                .emit(Msg::Progress(ProgressEvent::JobStarted(job_id.clone())));
            Self::forward_progress_until_complete(handle, self.ui_tx.clone()).await?;
            Ok(new_part_path)
        }
    }

    async fn cancel(&self, job_id: &str) -> Result<()> {
        self.ud
            .cancel_job(job_id)
            .await
            .map_err(anyhow::Error::from)
    }
}
