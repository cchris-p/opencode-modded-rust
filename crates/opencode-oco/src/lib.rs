use anyhow::{Context, Result};
use std::collections::HashMap;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};

pub mod commit;
pub mod commitlint;
pub mod config;
pub mod engine;
pub mod git;
pub mod hook;
pub mod models;
pub mod prompt;
pub mod text;

#[cfg(test)]
mod tests;

use config::Config;

pub const VERSION: &str = "3.2.19-ort";

#[derive(Debug, Clone)]
pub struct Runtime {
    pub cwd: PathBuf,
    pub home: PathBuf,
    pub global_config_path: PathBuf,
    pub models_cache_path: PathBuf,
    pub env: HashMap<String, String>,
    pub interactive: bool,
}

impl Runtime {
    pub fn from_process() -> Result<Self> {
        let cwd = std::env::current_dir().context("failed to read the current directory")?;
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let cache_root = dirs::cache_dir().unwrap_or_else(|| home.join(".cache"));
        Ok(Self {
            cwd,
            global_config_path: home.join(config::DEFAULT_CONFIG_FILE),
            models_cache_path: cache_root.join("ort").join("oco-models.json"),
            home,
            env: std::env::vars().collect(),
            interactive: std::io::stdin().is_terminal(),
        })
    }

    pub fn load_config(&self) -> Result<Config> {
        Config::load(&self.global_config_path, &self.cwd, &self.env)
    }

    pub fn path(&self, path: &Path) -> PathBuf {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.cwd.join(path)
        }
    }

    pub async fn run(&self, args: Vec<String>) -> Result<()> {
        let first = args.first().map(String::as_str);
        match first {
            Some("-h") | Some("--help") | None => {
                print_help();
                Ok(())
            }
            Some("-V") | Some("--version") => {
                println!("opencommit {VERSION} (ORT native)");
                Ok(())
            }
            Some("config") => config_command(self, &args[1..]),
            Some("hook") => hook::hook_command(self, &args[1..]).await,
            Some("commitlint") => commitlint::commitlint_command(self, &args[1..]).await,
            Some("models") => models::models_command(self, &args[1..]).await,
            _ => commit::commit_command(self, &args).await,
        }
    }
}

pub async fn run(args: Vec<String>) -> Result<()> {
    Runtime::from_process()?.run(args).await
}

pub fn print_help() {
    println!(
        "opencommit {VERSION} (ORT native)\n\
         \n\
         Usage: oco [options] [git commit args...]\n\
         \n\
         Generate a commit message for staged changes and commit them.\n\
         \n\
         Options:\n\
           -c, --context <text>  Additional context for the commit message\n\
           -y, --yes             Skip the commit confirmation prompt\n\
               --fgm             Use the full GitMoji specification\n\
           -h, --help            Print help\n\
           -V, --version         Print version\n\
         \n\
         Commands:\n\
           config get|set|describe <keys...>\n\
           models [--provider <id>] [--refresh]\n\
           hook set|unset\n\
           commitlint get|force\n\
         \n\
         Unknown options are forwarded to `git commit`."
    );
}

fn config_command(rt: &Runtime, args: &[String]) -> Result<()> {
    use anyhow::bail;

    if args.is_empty() || matches!(args[0].as_str(), "-h" | "--help") {
        print_config_help();
        return Ok(());
    }

    let mode = args[0].as_str();
    let keys = &args[1..];
    match mode {
        "describe" => {
            let config = rt.load_config()?;
            if keys.is_empty() {
                for (key, description) in config::KEY_DESCRIPTIONS {
                    println!("{key}\n  {description}");
                }
            } else {
                for key in keys {
                    if let Some((_, description)) = config::KEY_DESCRIPTIONS
                        .iter()
                        .find(|(name, _)| name == key)
                    {
                        println!("{key}\n  {description}");
                        if let Some(value) = config.get(key) {
                            println!("  current: {}", display_value(value));
                        }
                    } else {
                        bail!("Unknown config key: {key}");
                    }
                }
            }
            Ok(())
        }
        "get" => {
            if keys.is_empty() {
                bail!("No config keys specified for get mode");
            }
            let config = rt.load_config()?;
            for key in keys {
                let value = config
                    .get(key)
                    .map(display_value)
                    .unwrap_or_else(|| "undefined".to_string());
                println!("{key}={value}");
            }
            Ok(())
        }
        "set" => {
            if keys.is_empty() {
                bail!("No config keys specified for set mode");
            }
            let mut pairs = Vec::new();
            for pair in keys {
                let Some((key, value)) = pair.split_once('=') else {
                    bail!("Expected KEY=VALUE, got: {pair}");
                };
                pairs.push((key.to_string(), value.to_string()));
            }
            let mut config = rt.load_config()?;
            config.set_pairs(&pairs)?;
            config.write(&rt.global_config_path)?;
            println!("✔ config successfully set");
            Ok(())
        }
        other => {
            bail!("Unsupported mode: {other}. Valid modes are: \"set\", \"get\", and \"describe\"")
        }
    }
}

fn print_config_help() {
    println!(
        "Configure opencommit settings\n\
         \n\
         Usage: oco config <get|set|describe> [KEY=VALUE...]\n\
         \n\
         Examples:\n\
           Describe all config parameters: oco config describe\n\
           Describe a specific parameter: oco config describe OCO_MODEL\n\
           Get a config value: oco config get OCO_MODEL\n\
           Set a config value: oco config set OCO_MODEL=gpt-4"
    );
}

fn display_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}
