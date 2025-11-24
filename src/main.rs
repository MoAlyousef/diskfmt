#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    use clap::Parser;
    use diskfmt::{cli, config};

    let cli = cli::Cli::parse();
    let config = config::ConfigManager::default();
    let (cfg_theme, cfg_scheme) = config.get_styles();

    if let Some(cli::Command::Config {
        print,
        path,
        edit,
        init,
        force,
    }) = &cli.command
    {
        let cli_theme = {
            #[cfg(feature = "gui")]
            {
                cli.theme
            }
            #[cfg(not(feature = "gui"))]
            {
                None
            }
        };
        let cli_scheme = {
            #[cfg(feature = "gui")]
            {
                cli.scheme
            }
            #[cfg(not(feature = "gui"))]
            {
                None
            }
        };

        let opts = config::ConfigOpts {
            cfg_theme,
            cfg_scheme,
            print: *print,
            path: *path,
            edit: *edit,
            init: *init,
            force: *force,
        };
        return config.handle_config_command(cli_theme, cli_scheme, opts);
    }

    #[cfg(feature = "gui")]
    if cli.start_ui || cli.command.is_none() {
        use diskfmt::{gui, style};

        let resolved = style::resolve(cli.theme, cli.scheme, cfg_theme, cfg_scheme);
        return gui::Ui::start(
            Some(resolved.theme),
            Some(resolved.scheme),
            cli.mock_backend,
        )
        .await;
    }

    cli::Cli::start(cli).await
}
