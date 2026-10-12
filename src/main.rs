mod config;
mod search;
mod tui;

use anyhow::Result;
use config::scanner::scan;
use std::{
    fs,
    path::{Path, PathBuf},
};

fn main() -> Result<()> {
    let config_dir = parse_config_dir()?;
    if is_default_config_dir() {
        initialize_default_config(&config_dir)?;
    }
    let servers = scan(&config_dir)?;
    tui::run(servers)
}

fn is_default_config_dir() -> bool {
    !std::env::args_os()
        .skip(1)
        .any(|arg| arg == "--config" || arg == "-c")
}

fn initialize_default_config(config_dir: &Path) -> Result<()> {
    if config_dir.exists() {
        return Ok(());
    }
    fs::create_dir_all(config_dir)?;
    fs::create_dir_all(config_dir.join("production"))?;
    fs::create_dir_all(config_dir.join("staging"))?;
    fs::write(
        config_dir.join("production/api.toml"),
        include_str!("../examples/config/production/api.toml"),
    )?;
    fs::write(
        config_dir.join("staging/api.toml"),
        include_str!("../examples/config/staging/api.toml"),
    )?;
    Ok(())
}

fn parse_config_dir() -> Result<PathBuf> {
    let mut args = std::env::args_os().skip(1);
    if let Some(flag) = args.next() {
        if flag == "--config" || flag == "-c" {
            return args
                .next()
                .map(PathBuf::from)
                .ok_or_else(|| anyhow::anyhow!("--config requires a directory"));
        }
        anyhow::bail!("unknown argument: {:?}", flag);
    }
    Ok(dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("ssh-mgr"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn initializes_examples_only_for_missing_directory() {
        let root = std::env::temp_dir().join(format!(
            "ssh-mgr-init-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        initialize_default_config(&root).unwrap();
        assert!(root.join("production/api.toml").is_file());
        assert!(root.join("staging/api.toml").is_file());
        let existing = root.join("production/api.toml");
        fs::write(&existing, "custom").unwrap();
        initialize_default_config(&root).unwrap();
        assert_eq!(fs::read_to_string(existing).unwrap(), "custom");
        fs::remove_dir_all(root).unwrap();
    }
}
