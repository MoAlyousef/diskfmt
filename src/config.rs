use crate::style::{self, SchemeOpt, ThemeOpt, parse_scheme, parse_theme};
use serde::Deserialize;
use std::{env, fs, io, path::PathBuf, process::Command};

const CONFIG_TEMPLATE: &str = "\
# diskfmt configuration

[style]
# theme = \"DARK2\"
# scheme = \"Fleet1\"
";

#[derive(Debug, Deserialize)]
pub(crate) struct StyleConfig {
    theme: Option<String>,
    scheme: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct FileConfig {
    style: Option<StyleConfig>,
}

pub struct ConfigOpts {
    pub cfg_theme: Option<ThemeOpt>,
    pub cfg_scheme: Option<SchemeOpt>,
    pub print: bool,
    pub path: bool,
    pub edit: bool,
    pub init: bool,
    pub force: bool,
}

pub(crate) fn resolve_config_path() -> Option<PathBuf> {
    let mut base: Option<PathBuf> = env::var_os("XDG_CONFIG_HOME").map(PathBuf::from);
    if base
        .as_deref()
        .map(|p| p.as_os_str().is_empty())
        .unwrap_or(true)
    {
        if let Some(home) = env::var_os("HOME") {
            base = Some(PathBuf::from(home).join(".config"));
        }
    }
    base.map(|b| b.join("diskfmt").join("config.toml"))
}

pub struct ConfigManager {
    path: Option<PathBuf>,
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self {
            path: resolve_config_path(),
        }
    }
}

impl ConfigManager {
    pub fn handle_config_command(
        &self,
        cli_theme: Option<ThemeOpt>,
        cli_scheme: Option<SchemeOpt>,
        opts: ConfigOpts,
    ) -> anyhow::Result<()> {
        let ConfigOpts {
            cfg_theme,
            cfg_scheme,
            print,
            path,
            edit,
            init,
            force,
        } = opts;
        if init {
            match self.write_default(force) {
                Ok(()) => {
                    if let Some(p) = self.resolved_path() {
                        println!("Initialized: {}", p.display());
                    } else {
                        println!("Initialized config (path unknown)");
                    }
                }
                Err(e) => eprintln!("Init failed: {}", e),
            }
        }

        if path {
            if let Some(p) = self.resolved_path() {
                println!("{}", p.display());
            } else {
                println!("(no config path)");
            }
        }

        if edit {
            self.edit();
        }

        if print {
            let eff = style::resolve(cli_theme, cli_scheme, cfg_theme, cfg_scheme);
            let theme_name = style::canonical_theme_name(eff.theme);
            let scheme_name = style::canonical_scheme_name(eff.scheme);
            println!("theme = \"{}\"\nscheme = \"{}\"", theme_name, scheme_name);
        }

        if !print && !path && !edit && !init {
            println!("Use: diskfmt config --print|--path|--edit|--init [--force]");
        }

        Ok(())
    }

    pub(crate) fn resolved_path(&self) -> Option<PathBuf> {
        self.path.clone()
    }

    pub fn get_styles(&self) -> (Option<ThemeOpt>, Option<SchemeOpt>) {
        let Some(path) = self.resolved_path() else {
            return (None, None);
        };
        let Ok(contents) = fs::read_to_string(path) else {
            return (None, None);
        };

        let parsed: FileConfig = match toml::from_str(&contents) {
            Ok(c) => c,
            Err(_) => return (None, None),
        };

        let theme = parsed
            .style
            .as_ref()
            .and_then(|s| s.theme.as_deref())
            .and_then(parse_theme);
        let scheme = parsed
            .style
            .as_ref()
            .and_then(|s| s.scheme.as_deref())
            .and_then(parse_scheme);
        (theme, scheme)
    }

    pub(crate) fn write_default(&self, overwrite: bool) -> io::Result<()> {
        let Some(path) = self.resolved_path() else {
            return Err(io::Error::new(io::ErrorKind::NotFound, "No config path"));
        };
        if !overwrite && path.exists() {
            return Ok(());
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, CONFIG_TEMPLATE)
    }

    pub(crate) fn validate(&self) -> bool {
        let Some(path) = self.resolved_path() else {
            eprintln!("No config path");
            return false;
        };
        let Ok(contents) = fs::read_to_string(&path) else {
            eprintln!("Could not read {}", path.display());
            return false;
        };
        let parsed: FileConfig = match toml::from_str(&contents) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("TOML parse error: {}", e);
                return false;
            }
        };
        let mut ok = true;
        if let Some(style) = parsed.style {
            if let Some(t) = style.theme {
                if parse_theme(&t).is_none() {
                    ok = false;
                    let vals = style::valid_theme_names().join(", ");
                    eprintln!("Invalid theme '{}'. Valid: {}", t, vals);
                }
            }
            if let Some(s) = style.scheme {
                if parse_scheme(&s).is_none() {
                    ok = false;
                    let vals = style::valid_scheme_names().join(", ");
                    eprintln!("Invalid scheme '{}'. Valid: {}", s, vals);
                }
            }
        } else {
            eprintln!("Missing [style] table");
            ok = false;
        }
        if ok {
            println!("Config OK");
        }
        ok
    }

    pub(crate) fn edit(&self) {
        if let Some(p) = self.resolved_path() {
            if let Some(dir) = p.parent() {
                let _ = fs::create_dir_all(dir);
            }
            if !p.exists() {
                let _ = fs::write(&p, CONFIG_TEMPLATE);
            }
            let editor = env::var_os("VISUAL")
                .or_else(|| env::var_os("EDITOR"))
                .unwrap_or_else(|| "xdg-open".into());
            let status = Command::new(editor).arg(&p).status();
            if let Err(e) = status {
                eprintln!("Failed to launch editor: {}", e);
            }
            let _ = self.validate();
        } else {
            eprintln!("Could not resolve config path");
        }
    }
}
