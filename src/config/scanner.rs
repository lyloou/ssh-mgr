use super::model::{FileConfig, Server};
use anyhow::{Context, Result};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn scan(root: &Path) -> Result<Vec<Server>> {
    if !root.exists() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    collect_toml(root, &mut files)?;
    let mut result = Vec::new();
    for file in files {
        let text = fs::read_to_string(&file).with_context(|| format!("read {}", file.display()))?;
        let parsed: FileConfig =
            toml::from_str(&text).with_context(|| format!("parse {}", file.display()))?;
        let group = file
            .parent()
            .and_then(|p| p.strip_prefix(root).ok())
            .unwrap_or(Path::new(""));
        let group = if group.as_os_str().is_empty() {
            "default".to_string()
        } else {
            group.display().to_string()
        };
        for mut server in parsed.server {
            server.validate().map_err(|error| {
                anyhow::anyhow!("{}: server '{}': {error}", file.display(), server.name)
            })?;
            server.group = group.clone();
            server.source = file.clone();
            result.push(server);
        }
    }
    result.sort_by_key(|s| (s.group.to_lowercase(), s.name.to_lowercase()));
    Ok(result)
}

fn collect_toml(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir).with_context(|| format!("read directory {}", dir.display()))? {
        let path = entry?.path();
        if path.is_dir() {
            collect_toml(&path, files)?;
        } else if path.extension().is_some_and(|ext| ext == "toml") {
            files.push(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn scans_directories_as_groups() {
        let root = std::env::temp_dir().join(format!(
            "ssh-mgr-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(root.join("prod")).unwrap();
        fs::write(
            root.join("prod/api.toml"),
            "[[server]]\nname='api'\nhost='localhost'\nuser='root'\n",
        )
        .unwrap();
        let servers = scan(&root).unwrap();
        assert_eq!(servers[0].group, "prod");
        assert_eq!(servers[0].port, 22);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn parses_unified_auth_block() {
        let root = std::env::temp_dir().join(format!(
            "ssh-mgr-auth-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("auth.toml"),
            "[[server]]\nname='key'\nhost='localhost'\nuser='root'\n[server.password]\nkey='~/.ssh/id_ed25519'\n",
        )
        .unwrap();
        let servers = scan(&root).unwrap();
        assert_eq!(
            servers[0].password.as_ref().unwrap().key.as_deref(),
            Some("~/.ssh/id_ed25519")
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_conflicting_password_sources() {
        let root = std::env::temp_dir().join(format!(
            "ssh-mgr-invalid-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("invalid.toml"),
            "[[server]]\nname='bad'\nhost='localhost'\nuser='root'\n[server.password]\nenv='PASSWORD'\nprompt=true\n",
        )
        .unwrap();
        let error = scan(&root).unwrap_err().to_string();
        assert!(error.contains("invalid.toml"));
        assert!(error.contains("exactly one"));
        fs::remove_dir_all(root).unwrap();
    }
}
