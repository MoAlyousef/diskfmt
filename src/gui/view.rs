use super::gui_utils::*;
use crate::backends::BlockDevice;
use crate::common::{Msg, UiSender};
use crate::utils::{default_fs, detect_supported_fs};
use fltk::{
    app,
    button::Button,
    enums::{Align, Event},
    frame::Frame,
    group::Flex,
    image::PngImage,
    input::Input,
    menu::Choice,
    misc::Progress,
    prelude::{ButtonExt, GroupExt, InputExt, MenuExt, WidgetBase, WidgetExt, WindowExt},
    window::Window,
};
#[cfg(feature = "a11y")]
use fltk_accesskit::{builder, update_focused};
use std::{cell::RefCell, rc::Rc};

const ICON: &[u8] = include_bytes!("../../assets/icon16x16.png");

const TOOLTIP_DEVICE_CHOICE: &str = concat!(
    "Select the target block device or partition to format. ",
    "Be cautious when selecting a device, as formatting will erase all data on it. ",
    "Ensure you choose the correct device to avoid data loss."
);
const TOOLTIP_FS_CHOICE: &str = concat!(
    "Select the filesystem type to format the selected device with. ",
    "Different filesystems have different features, performance characteristics, and ",
    "compatibility with various operating systems. Common options include FAT32 ",
    "(widely compatible), NTFS (Windows), ext4 (Linux), and exFAT (large files and ",
    "cross-platform). Choose a filesystem that best suits your needs based on the ",
    "intended use of the device."
);
const TOOLTIP_SIZE_CHOICE: &str = concat!(
    "Specifies the cluster size (for FAT filesystems) or block size (for ext filesystems) ",
    "to be used when formatting. A smaller size can lead to more efficient space usage ",
    "for many small files, while a larger size can improve performance for larger files. ",
    "'Auto' lets the formatter choose an optimal size based on the filesystem and device ",
    "characteristics."
);
const TOOLTIP_LABEL_INPUT: &str = concat!(
    "The volume label is a human-readable name for the filesystem. ",
    "Different filesystems have different restrictions on the length and allowed characters ",
    "for labels. For example, FAT32 labels can be up to 11 characters long and may include ",
    "letters, numbers, and certain special characters, while ext4 labels can be up to 16 ",
    "characters long and support a wider range of characters including spaces."
);
const TOOLTIP_PT_CHOICE: &str = concat!(
    "GPT (GUID Partition Table) is the modern standard for disk partitioning, while MBR ",
    "(Master Boot Record) is the older, legacy standard. GPT supports larger drives (>2 TB), ",
    "a higher number of partitions (128 vs. 4), and has better data protection with a backup, ",
    "whereas MBR is limited to 2 TB disks and fewer partitions and lacks this redundancy. ",
    "Modern systems use GPT with UEFI firmware, while MBR is limited to legacy BIOS systems. ",
    "Choose GPT for new installations unless compatibility with very old systems is required."
);
const TOOLTIP_START_BTN: &str = "Begin the formatting process with the selected options.";
const TOOLTIP_CANCEL_BTN: &str = "Cancel the ongoing formatting process.";
const TOOLTIP_QUICK_FORMAT: &str =
    "Faster: skips data wipe and error scan. Uncheck for full format.";

pub(crate) struct View {
    pub(crate) device_choice: Choice,
    pub(crate) pt_choice: Choice,
    pub(crate) start_btn: Button,
    pub(crate) cancel_btn: Button,
    pub(crate) progress: Progress,
    pub(crate) status: Frame,
}

impl View {
    pub(crate) fn new(
        tx: crossbeam_channel::Sender<Msg>,
        devices: Rc<RefCell<Vec<BlockDevice>>>,
    ) -> Self {
        let mut win = Window::default().with_size(400, 500).with_label("diskfmt");
        win.set_xclass("diskfmt");
        win.set_icon(Some(PngImage::from_data(ICON).unwrap()));
        let mut col = Flex::default_fill().column();
        col.set_margins(10, 5, 10, 5);
        col.set_pad(5);

        Frame::default().with_label("Target Device");
        let mut device_choice = Choice::default();
        device_choice.set_tooltip(TOOLTIP_DEVICE_CHOICE);

        Frame::default().with_label("Filesystem");
        let mut fs_choice = Choice::default();
        fs_choice.set_tooltip(TOOLTIP_FS_CHOICE);
        let supported = detect_supported_fs();
        if supported.is_empty() {
            fs_choice.add_choice("(No filesystem tools found)");
            fs_choice.set_value(0);
            fs_choice.deactivate();
        } else {
            for fs in &supported {
                fs_choice.add_choice(fs);
            }
            if let Some(def) = default_fs(&supported)
                && let Some(idx) = supported.iter().position(|&x| x == def)
            {
                fs_choice.set_value(idx as i32);
            }
        }

        let mut size_label = Frame::default().with_label("Allocation Unit Size");
        let mut size_choice = Choice::default();
        size_choice.set_tooltip(TOOLTIP_SIZE_CHOICE);

        Frame::default().with_label("Volume Label");
        let mut label_input = Input::default();
        label_input.set_tooltip(TOOLTIP_LABEL_INPUT);

        Frame::default().with_label("Partition Table");
        let mut pt_choice = Choice::default();
        pt_choice.set_tooltip(TOOLTIP_PT_CHOICE);
        pt_choice.add_choice("GPT (default)");
        pt_choice.add_choice("MBR (DOS)");
        pt_choice.set_value(0);

        let mut row_quick = Flex::default().row();
        let mut quick_chk = fltk::button::CheckButton::default().with_label("Quick format");
        quick_chk.set_tooltip(TOOLTIP_QUICK_FORMAT);
        quick_chk.set_value(true);
        Frame::default();
        row_quick.fixed(&quick_chk, 80);
        row_quick.end();

        let mut row_btn = Flex::default().row();
        let mut start_btn = Button::default().with_label("Start");
        start_btn.set_tooltip(TOOLTIP_START_BTN);
        let mut cancel_btn = Button::default().with_label("Cancel");
        cancel_btn.set_tooltip(TOOLTIP_CANCEL_BTN);
        cancel_btn.deactivate();
        row_btn.set_pad(10);
        row_btn.end();

        let mut progress = Progress::default();
        progress.set_minimum(0.0);
        progress.set_maximum(100.0);

        let mut status = Frame::default().with_label("");
        status.set_align(Align::Left | Align::Inside);

        col.end();
        win.end();
        win.show();
        win.resizable(&col);
        win.size_range(400, 300, 800, 1000);
        let tx_close = tx.clone();
        win.set_callback(move |_w| {
            if app::event() == Event::Close {
                tx_close.emit(Msg::RequestClose);
            }
        });
        #[cfg(feature = "a11y")]
        {
            let ac = builder(win.clone()).attach();
            win.handle(move |_, ev| match ev {
                Event::KeyUp => {
                    update_focused(&ac);
                    false
                }
                _ => false,
            });
        }

        let current_fs = fs_choice.choice();
        size_label.set_label(size_label_text(current_fs.as_deref()));
        fill_size_choices(&mut size_choice, current_fs.as_deref());

        fs_choice.set_callback({
            let mut size_choice = size_choice.clone();
            let mut size_label = size_label.clone();
            move |c| {
                let fs = c.choice().unwrap_or_else(|| "vfat".into());
                size_label.set_label(size_label_text(Some(&fs)));
                fill_size_choices(&mut size_choice, Some(&fs));
            }
        });

        device_choice.set_callback({
            let devices_ref = devices.clone();
            let mut pt_choice = pt_choice.clone();
            move |c| {
                let idx = c.value();
                let devs = devices_ref.borrow();
                let is_partition = if idx >= 0 && (idx as usize) < devs.len() {
                    devs[idx as usize].is_partition
                } else {
                    false
                };
                if is_partition {
                    pt_choice.deactivate();
                } else {
                    pt_choice.activate();
                }
            }
        });

        cancel_btn.set_callback({
            let tx = tx.clone();
            move |_| {
                tx.emit(Msg::Cancel);
            }
        });

        start_btn.set_callback({
            let tx = tx.clone();
            let supported_fs = supported.clone();
            let devices_ref = devices.clone();
            let fs_choice = fs_choice.clone();
            let device_choice = device_choice.clone();
            let label_input = label_input.clone();
            let size_choice = size_choice.clone();
            let quick_chk = quick_chk.clone();
            let pt_choice = pt_choice.clone();
            move |_| {
                if supported_fs.is_empty() {
                    fltk::dialog::message_default(
                        "No filesystem formatting tools found. Please install mkfs utilities (e.g., mkfs.vfat, mkfs.ext4)."
                    );
                    return;
                }
                let idx = device_choice.value();
                let devs = devices_ref.borrow();
                if idx < 0 || (idx as usize) >= devs.len() {
                    return;
                }
                let device = &devs[idx as usize];
                let obj_path = device.object_path.clone();
                let ans = fltk::dialog::choice2_default(
                    &format!(
                        "WARNING: Formatting will erase all data on {}. Continue?",
                        obj_path
                    ),
                    "No",
                    "Yes",
                    "Cancel",
                );
                if ans != Some(1) {
                    return;
                }
                let fs = fs_choice.choice().unwrap_or_else(|| "vfat".into());
                let label = {
                    let s = label_input.value();
                    if s.is_empty() { None } else { Some(s) }
                };
                let size = crate::utils::parse_size_choice_label(size_choice.choice().as_deref());
                let partition_table = parse_partition_table_choice(pt_choice.choice().as_deref());
                let opts = match crate::utils::build_format_options(
                    fs,
                    label,
                    quick_chk.value(),
                    size,
                    partition_table,
                ) {
                    Ok(o) => o,
                    Err(err) => {
                        fltk::dialog::message_default(&format!("Invalid label: {}", err));
                        return;
                    }
                };
                tx.emit(Msg::Start { obj_path, opts });
            }
        });

        Self {
            device_choice,
            pt_choice,
            start_btn,
            cancel_btn,
            progress,
            status,
        }
    }
}
