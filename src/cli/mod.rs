use crate::backends::PartitionTable;
use crate::backends::ProgressEvent;
use crate::common::{ConsoleReporter, Msg, ProgressReporter, make_backend};
#[cfg(feature = "gui")]
use crate::style::{SchemeOpt, ThemeOpt};
use crate::utils;
use clap::ValueEnum;
#[allow(unused_imports)]
use clap::{CommandFactory, Parser, Subcommand};
use std::{process, time::Duration};

#[derive(Copy, Clone, Debug, ValueEnum)]
#[value(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PartitionTableOpt {
    Gpt,
    Dos,
}

pub(crate) fn map_partition_table_opt(table: Option<PartitionTableOpt>) -> Option<PartitionTable> {
    match table {
        Some(PartitionTableOpt::Dos) => Some(PartitionTable::Dos),
        Some(PartitionTableOpt::Gpt) => Some(PartitionTable::Gpt),
        None => None,
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct Cli {
    #[cfg(feature = "gui")]
    /// Start the GUI
    #[arg(long)]
    pub start_ui: bool,

    /// Use the mock backend instead of UDisks2
    #[arg(long, global = true)]
    pub mock_backend: bool,

    #[cfg(feature = "gui")]
    /// UI color theme
    #[arg(long, value_enum, global = true)]
    pub theme: Option<ThemeOpt>,

    #[cfg(feature = "gui")]
    /// UI widget scheme
    #[arg(long, value_enum, global = true)]
    pub scheme: Option<SchemeOpt>,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// List available block devices
    List,
    /// Show or manage configuration
    Config {
        /// Print the effective theme and scheme after merging config and CLI
        #[arg(long)]
        print: bool,
        /// Show the resolved config file path
        #[arg(long)]
        path: bool,
        /// Open the config in $VISUAL/$EDITOR (creates file if missing)
        #[arg(long)]
        edit: bool,
        /// Initialize a config file if missing (use with --force to overwrite)
        #[arg(long)]
        init: bool,
        /// Overwrite existing config when used with --init
        #[arg(long)]
        force: bool,
    },
    /// Format a device/partition with options similar to the GUI
    Format {
        /// Object path or device identifier
        #[arg(long)]
        path: String,
        /// Filesystem type (e.g., vfat, exfat, ntfs, ext4, xfs, btrfs)
        #[arg(long)]
        fs: Option<String>,
        /// Volume label
        #[arg(long)]
        label: Option<String>,
        /// Use quick format
        #[arg(long, default_value_t = false)]
        quick: bool,
        /// Allocation unit size choice (e.g., "Auto", "4096 bytes", "8 sectors")
        #[arg(long, value_name = "SIZE")]
        size: Option<String>,
        /// Partition table type for whole-disk format
        #[arg(long, value_enum)]
        table: Option<PartitionTableOpt>,
    },
    /// Cancel a running format by job id
    Cancel {
        /// Job id to cancel
        job_id: String,
    },
}

impl Cli {
    pub async fn start(mut cli: Cli) -> anyhow::Result<()> {
        let command = match cli.command.take() {
            Some(cmd) => cmd,
            None => {
                #[cfg(not(feature = "gui"))]
                {
                    let mut cmd = Cli::command();
                    let _ = cmd.print_help();
                    println!();
                    return Ok(());
                }
                #[cfg(feature = "gui")]
                {
                    // With GUI builds, `main` should have handled `None` by launching the UI.
                    // If we get here, report a clear error instead of panicking.
                    return Err(anyhow::anyhow!("no subcommand provided"));
                }
            }
        };
        let (tx, rx) = crossbeam_channel::unbounded::<Msg>();
        let backend = make_backend(tx, cli.mock_backend).await;

        match command {
            Command::Config { .. } => unreachable!("handled above"),
            Command::List => match backend.list_block_devices().await {
                Ok(devs) => {
                    for d in devs {
                        println!("{}", utils::device_display(&d));
                    }
                }
                Err(e) => {
                    eprintln!("List error: {e}");
                    process::exit(1);
                }
            },
            Command::Format {
                path,
                fs,
                label,
                quick,
                size,
                table,
            } => {
                let fs = match fs {
                    Some(f) => f,
                    None => {
                        let supported = utils::detect_supported_fs();
                        utils::default_fs(&supported).unwrap_or("vfat").to_string()
                    }
                };

                let size = utils::parse_size_choice_label(size.as_deref());
                let partition_table = map_partition_table_opt(table);

                let opts =
                    match utils::build_format_options(fs, label, quick, size, partition_table) {
                        Ok(o) => o,
                        Err(err) => {
                            eprintln!("Invalid label: {err}");
                            process::exit(2);
                        }
                    };

                let be = backend.clone();
                let path_clone = path.clone();
                let fmt = tokio::spawn(async move { be.format(&path_clone, opts).await });

                let mut done = false;
                let mut reporter = ConsoleReporter;

                while !done {
                    match rx.recv_timeout(Duration::from_millis(50)) {
                        Ok(msg) => match msg {
                            Msg::Status(s) => reporter.status(&s),
                            Msg::Progress(ev) => {
                                reporter.progress(&ev);
                                if let ProgressEvent::Completed(_) = ev {
                                    done = true;
                                }
                            }
                            #[cfg(feature = "gui")]
                            _ => {}
                        },
                        Err(crossbeam_channel::RecvTimeoutError::Timeout) => {}
                        Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
                    }

                    if done || fmt.is_finished() {
                        break;
                    }
                }

                match fmt.await {
                    Ok(Ok(new_path)) => println!("Ready: {}", new_path),
                    Ok(Err(e)) => {
                        eprintln!("Format failed: {e}");
                        process::exit(1);
                    }
                    Err(join_err) => {
                        eprintln!("Format task failed to join: {join_err}");
                        process::exit(1);
                    }
                }
            }
            Command::Cancel { job_id } => match backend.cancel(&job_id).await {
                Ok(()) => println!("Cancellation requested for job {job_id}"),
                Err(e) => {
                    eprintln!("Cancel failed: {e}");
                    process::exit(1);
                }
            },
        }

        Ok(())
    }
}
