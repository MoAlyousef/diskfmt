use super::*;
use crate::common::{Msg, UiSender};
use tokio::time::{Duration, sleep};

const MOCK_QUICK_OPERATION_MS: u64 = 100;
const MOCK_FORMAT_OPERATION_MS: u64 = 1000;

pub(crate) struct MockBackend {
    ui_tx: crossbeam_channel::Sender<Msg>,
}

impl MockBackend {
    pub(crate) fn new(ui_tx: crossbeam_channel::Sender<Msg>) -> Self {
        ui_tx.emit(Msg::Status(
            "Warning: Using mock backend. UDisks2 unavailable.".to_string(),
        ));
        Self { ui_tx }
    }
}

#[async_trait]
impl Backend for MockBackend {
    async fn list_block_devices(&self) -> Result<Vec<BlockDevice>> {
        sleep(Duration::from_millis(MOCK_QUICK_OPERATION_MS)).await;
        Ok(vec![BlockDevice {
            dev_path: "/dev/sdc1".to_string(),
            object_path: "0".to_string(),
            fs_type: Some("vfat".into()),
            label: Some("MOCK".into()),
            size_bytes: Some(64 * 1_000_000_000),
            vendor_model: Some("Mock USB".into()),
            is_partition: true,
        }])
    }
    async fn format(&self, _obj_path: &str, _opts: FormatOptions) -> Result<String> {
        let job_id = "mock_job_123".to_string();
        let _ = self
            .ui_tx
            .emit(Msg::Progress(ProgressEvent::JobStarted(job_id)));
        sleep(Duration::from_millis(MOCK_FORMAT_OPERATION_MS)).await;
        let _ = self.ui_tx.emit(Msg::Progress(ProgressEvent::Percent(25.0)));
        sleep(Duration::from_millis(MOCK_FORMAT_OPERATION_MS)).await;
        let _ = self.ui_tx.emit(Msg::Progress(ProgressEvent::Percent(50.0)));
        sleep(Duration::from_millis(MOCK_FORMAT_OPERATION_MS)).await;
        let _ = self.ui_tx.emit(Msg::Progress(ProgressEvent::Percent(75.0)));
        sleep(Duration::from_millis(MOCK_FORMAT_OPERATION_MS)).await;
        let _ = self
            .ui_tx
            .emit(Msg::Progress(ProgressEvent::Percent(100.0)));
        let _ = self
            .ui_tx
            .emit(Msg::Progress(ProgressEvent::Completed(Ok(()))));
        Ok("Done".to_string())
    }
    async fn cancel(&self, _job_id: &str) -> Result<()> {
        sleep(Duration::from_millis(MOCK_QUICK_OPERATION_MS)).await;
        Ok(())
    }
}
