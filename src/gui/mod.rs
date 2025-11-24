use crate::backends::{Backend, BlockDevice, ProgressEvent};
use crate::common::{Msg, ProgressReporter, UiSender, make_backend};
use crate::style::{SchemeOpt, ThemeOpt, apply_theme};
use fltk::{
    app, dialog,
    prelude::{MenuExt, WidgetExt},
};
mod gui_utils;
mod view;
use gui_utils::report_error;
use std::{cell::RefCell, panic, process, rc::Rc, sync::Arc};
use view::View;

#[derive(Clone, Debug)]
pub(crate) enum AppState {
    Idle,
    Starting,
    Formatting { job_id: String },
}

pub struct Ui {
    view: View,
    devices: Rc<RefCell<Vec<BlockDevice>>>,
    tx: crossbeam_channel::Sender<Msg>,
    pub(crate) state: AppState,
}

impl ProgressReporter for Ui {
    fn status(&mut self, msg: &str) {
        self.update_progress(ProgressEvent::Message(msg.to_string()));
    }
    fn progress(&mut self, ev: &ProgressEvent) {
        self.update_progress(ev.clone());
    }
}

impl Ui {
    pub(crate) fn build(tx: crossbeam_channel::Sender<Msg>) -> Ui {
        let devices = Rc::new(RefCell::new(Vec::<BlockDevice>::new()));
        let view = View::new(tx.clone(), devices.clone());
        Ui {
            view,
            devices,
            tx,
            state: AppState::Idle,
        }
    }

    pub(crate) fn update_devices(&mut self, devs: Vec<BlockDevice>) {
        self.view.device_choice.clear();
        for d in &devs {
            let display = crate::utils::device_display(d);
            self.view.device_choice.add_choice(&display);
        }
        *self.devices.borrow_mut() = devs;
        if self.view.device_choice.size() > 0 {
            self.view.device_choice.set_value(0);
            let devs = self.devices.borrow();
            let is_partition = devs.first().map(|d| d.is_partition).unwrap_or(false);
            if is_partition {
                self.view.pt_choice.deactivate();
            } else {
                self.view.pt_choice.activate();
            }
        }
    }

    fn sync_ui_to_state(&mut self) {
        match &self.state {
            AppState::Idle => {
                self.view.start_btn.activate();
                self.view.cancel_btn.deactivate();
            }
            AppState::Starting => {
                self.view.start_btn.deactivate();
                self.view.cancel_btn.deactivate();
            }
            AppState::Formatting { .. } => {
                self.view.start_btn.deactivate();
                self.view.cancel_btn.activate();
            }
        }
    }

    pub(crate) fn set_state(&mut self, new_state: AppState) {
        self.state = new_state;
        self.sync_ui_to_state();
    }

    pub(crate) fn update_progress(&mut self, ev: ProgressEvent) {
        match ev {
            ProgressEvent::JobStarted(job_id) => {
                self.set_state(AppState::Formatting { job_id });
            }
            ProgressEvent::Percent(p) => {
                let clamped = p
                    .max(self.view.progress.minimum())
                    .min(self.view.progress.maximum());
                self.view.progress.set_value(clamped);
                self.view.progress.redraw();
            }
            ProgressEvent::Message(m) => self.view.status.set_label(&m),
            ProgressEvent::RateBytesPerSec(r) => self.view.status.set_label(&format!("{r} B/s")),
            ProgressEvent::Completed(res) => {
                match res {
                    Ok(()) => {
                        self.view.progress.set_value(self.view.progress.maximum());
                        self.view.progress.redraw();
                        self.view.status.set_label("Completed");
                    }
                    Err(e) => {
                        self.view.progress.set_value(self.view.progress.minimum());
                        self.view.progress.redraw();
                        self.view.status.set_label(&format!("Error: {e}"));
                    }
                }
                self.set_state(AppState::Idle);
            }
        }
    }

    pub(crate) fn is_busy(&self) -> bool {
        matches!(self.state, AppState::Formatting { .. } | AppState::Starting)
    }

    pub(crate) fn active_job_id(&self) -> Option<&str> {
        match &self.state {
            AppState::Formatting { job_id } => Some(job_id),
            AppState::Starting => None,
            AppState::Idle => None,
        }
    }

    pub(crate) fn handle_msg(&mut self, backend: Arc<dyn Backend>, msg: Msg) {
        let tx = self.tx.clone();
        match msg {
            Msg::Devices(devs) => self.update_devices(devs),
            Msg::Status(s) => {
                let reporter: &mut dyn ProgressReporter = self;
                reporter.status(&s);
                dialog::message_default(&s);
            }
            Msg::Progress(ev) => {
                let reporter: &mut dyn ProgressReporter = self;
                reporter.progress(&ev);
            }
            Msg::Start { obj_path, opts } => {
                self.set_state(AppState::Starting);
                self.update_progress(ProgressEvent::Percent(0.0));
                self.update_progress(ProgressEvent::Message("Starting...".into()));

                tokio::spawn({
                    let tx = tx.clone();
                    let be = backend.clone();
                    async move {
                        let formatted_path = match be.format(&obj_path, opts).await {
                            Ok(path) => path,
                            Err(e) => {
                                report_error(tx.clone(), "Format", e);
                                return;
                            }
                        };
                        match be.list_block_devices().await {
                            Ok(devs) => {
                                tx.emit(Msg::Devices(devs));
                                tx.emit(Msg::Status(format!("Ready: {formatted_path}")));
                            }
                            Err(e) => {
                                tx.emit(Msg::Status(format!("Refresh failed: {e}")));
                            }
                        }
                    }
                });
            }
            Msg::Cancel => {
                if let Some(job_id) = self.active_job_id() {
                    tokio::spawn({
                        let job_id = job_id.to_string();
                        let tx = tx.clone();
                        let be = backend.clone();
                        async move {
                            match be.cancel(&job_id).await {
                                Ok(()) => {
                                    tx.emit(Msg::Progress(ProgressEvent::Message(
                                        "Cancellation requested...".into(),
                                    )));
                                }
                                Err(e) => {
                                    tx.emit(Msg::Status(format!("Cancel failed: {e}")));
                                    tx.emit(Msg::Progress(ProgressEvent::JobStarted(
                                        job_id.clone(),
                                    )));
                                }
                            }
                        }
                    });
                }
            }
            Msg::RequestClose => {
                if self.is_busy() {
                    let msg = "A format is still running. Cancel it before closing.";
                    self.update_progress(ProgressEvent::Message(msg.into()));
                    dialog::message_default(msg);
                } else {
                    app::quit();
                }
            }
        }
    }

    pub async fn start(
        theme: Option<ThemeOpt>,
        scheme: Option<SchemeOpt>,
        use_mock: bool,
    ) -> anyhow::Result<()> {
        let app = app::App::default();
        apply_theme(theme, scheme);
        let (tx, rx) = crossbeam_channel::unbounded::<Msg>();

        panic::set_hook(Box::new({
            let tx = tx.clone();
            move |e| {
                report_error(tx.clone(), "Panic", anyhow::anyhow!(e.to_string()));
                process::exit(2);
            }
        }));

        let mut ui = Ui::build(tx.clone());
        let backend = make_backend(tx.clone(), use_mock).await;

        tokio::spawn({
            let tx = tx.clone();
            let be = backend.clone();
            async move {
                match be.list_block_devices().await {
                    Ok(devs) => {
                        tx.emit(Msg::Devices(devs));
                    }
                    Err(e) => {
                        tx.emit(Msg::Status(format!("List error: {e}")));
                    }
                }
            }
        });

        while app.wait() {
            while let Ok(msg) = rx.try_recv() {
                let be = backend.clone();
                ui.handle_msg(be, msg);
            }
        }
        Ok(())
    }
}
