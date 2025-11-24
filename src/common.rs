use crate::backends::mock::MockBackend;
use crate::backends::udisks::UdisksBackend;
use crate::backends::{Backend, ProgressEvent};
#[cfg(feature = "gui")]
use crate::backends::{BlockDevice, FormatOptions};
use std::sync::Arc;

#[derive(Clone)]
pub(crate) enum Msg {
    #[cfg(feature = "gui")]
    Devices(Vec<BlockDevice>),
    #[cfg(feature = "gui")]
    Start {
        obj_path: String,
        opts: FormatOptions,
    },
    #[cfg(feature = "gui")]
    Cancel,
    #[cfg(feature = "gui")]
    RequestClose,
    Progress(ProgressEvent),
    Status(String),
}

pub(crate) async fn make_backend(
    tx: crossbeam_channel::Sender<Msg>,
    use_mock: bool,
) -> Arc<dyn Backend> {
    if use_mock {
        Arc::new(MockBackend::new(tx))
    } else {
        match UdisksBackend::new(tx.clone()).await {
            Ok(u) => Arc::new(u),
            Err(e) => {
                eprintln!("Warning: Failed to connect to UDisks2: {}", e);
                eprintln!(
                    "Falling back to mock backend (no actual disk operations will be performed)"
                );
                Arc::new(MockBackend::new(tx))
            }
        }
    }
}

pub(crate) trait UiSender<T> {
    fn emit(&self, msg: T);
}

impl<T> UiSender<T> for crossbeam_channel::Sender<T> {
    fn emit(&self, msg: T) {
        self.try_send(msg).ok();
        #[cfg(feature = "gui")]
        {
            fltk::app::awake();
        }
    }
}

pub(crate) trait ProgressReporter {
    fn status(&mut self, msg: &str);
    fn progress(&mut self, ev: &ProgressEvent);
}

pub(crate) struct ConsoleReporter;

impl ProgressReporter for ConsoleReporter {
    fn status(&mut self, msg: &str) {
        eprintln!("{msg}");
    }
    fn progress(&mut self, ev: &ProgressEvent) {
        match ev {
            ProgressEvent::JobStarted(id) => eprintln!("Job {id} started"),
            ProgressEvent::Percent(p) => eprintln!("Progress: {:.0}%", p),
            ProgressEvent::RateBytesPerSec(r) => eprintln!("Rate: {} B/s", r),
            ProgressEvent::Message(m) => eprintln!("{m}"),
            ProgressEvent::Completed(Ok(())) => eprintln!("Completed"),
            ProgressEvent::Completed(Err(e)) => eprintln!("Error: {e}"),
        }
    }
}
