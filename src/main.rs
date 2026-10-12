mod config;
mod search;
mod tui;

use anyhow::Result;
use config::scanner::scan;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

fn main() -> Result<()> {
    if handle_meta_args() {
        return Ok(());
    }
    ensure_tmux_session()?;
    let config_dir = parse_config_dir()?;
    if is_default_config_dir() {
        initialize_default_config(&config_dir)?;
    }
    let servers = scan(&config_dir)?;
    tui::run(servers, config_dir)
}

fn handle_meta_args() -> bool {
    let arg = std::env::args().nth(1);
    match arg.as_deref() {
        Some("--version") | Some("-V") => {
            println!("ssh-mgr {}", env!("CARGO_PKG_VERSION"));
            true
        }
        Some("--help") | Some("-h") => {
            println!(
                "ssh-mgr - keyboard-driven SSH manager\n\nUsage:\n  ssh-mgr [--config PATH]\n\nOptions:\n  -c, --config PATH  scan a specific configuration directory\n  -h, --help         show this help\n  -V, --version      show the version\n\nThe default configuration directory is ~/.config/ssh-mgr/."
            );
            true
        }
        _ => false,
    }
}

fn ensure_tmux_session() -> Result<()> {
    if std::env::var_os("TMUX").is_some() {
        return Ok(());
    }
    let executable = std::env::current_exe()?;
    let session_exists = Command::new("tmux")
        .args(["has-session", "-t", "ssh-mgr"])
        .stderr(Stdio::null())
        .status()
        .map_err(|error| anyhow::anyhow!("tmux is required but could not be started: {error}"))?
        .success();
    if !session_exists {
        let mut command = Command::new("tmux");
        command.args(["new-session", "-d", "-s", "ssh-mgr", "-n", "ssh-mgr", "--"]);
        command.arg(&executable);
        command.args(std::env::args_os().skip(1));
        let status = command.status()?;
        if !status.success() {
            anyhow::bail!("tmux failed to create session ssh-mgr: {status}");
        }
    } else {
        let manager_window_exists = Command::new("tmux")
            .args(["select-window", "-t", "ssh-mgr:ssh-mgr"])
            .status()?
            .success();
        if !manager_window_exists {
            let mut command = Command::new("tmux");
            command.args(["new-window", "-d", "-t", "ssh-mgr:", "-n", "ssh-mgr", "--"]);
            command.arg(&executable);
            command.args(std::env::args_os().skip(1));
            let status = command.status()?;
            if !status.success() {
                anyhow::bail!("tmux failed to create ssh-mgr window: {status}");
            }
        }
    }
    let status = Command::new("tmux")
        .args(["attach-session", "-t", "ssh-mgr:ssh-mgr"])
        .status()?;
    if !status.success() {
        anyhow::bail!("tmux failed to attach ssh-mgr session: {status}");
    }
    std::process::exit(0);
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
    Ok(dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".config")
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
