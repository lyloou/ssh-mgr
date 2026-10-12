use anyhow::{Context, Result};
use std::{fs::File, io::Read, path::Path};

pub fn load_default() -> Result<()> {
    let home = dirs::home_dir().context("cannot locate home directory for ssh-mgr/env")?;
    load(&home.join(".config/ssh-mgr/env"))
}

fn parse(reader: impl Read) -> Result<Vec<(String, String)>> {
    dotenvy::from_read_iter(reader)
        .enumerate()
        .map(|(index, entry)| {
            // dotenv parser errors can include the input, which may contain passwords.
            entry.map_err(|_| anyhow::anyhow!("invalid environment entry #{}", index + 1))
        })
        .collect()
}

fn load(path: &Path) -> Result<()> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error).with_context(|| format!("read {}", path.display())),
    };
    let entries = parse(file).with_context(|| format!("parse {}", path.display()))?;
    for (name, value) in entries {
        if std::env::var_os(&name).is_none() {
            std::env::set_var(name, value);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supports_comments_quotes_and_export() {
        let entries =
            parse(b"# local secrets\nPASSWORD='hello world'\nexport REGION=local\n".as_slice())
                .unwrap();
        assert_eq!(
            entries,
            [
                ("PASSWORD".into(), "hello world".into()),
                ("REGION".into(), "local".into())
            ]
        );
    }

    #[test]
    fn preserves_existing_environment_and_loads_missing_values() {
        let name = format!("SSH_MGR_ENV_TEST_{}", std::process::id());
        let path = std::env::temp_dir().join(format!("ssh-mgr-env-test-{}", std::process::id()));
        std::fs::write(&path, format!("{name}=from-file\n")).unwrap();
        load(&path).unwrap();
        assert_eq!(std::env::var(&name).unwrap(), "from-file");
        std::env::set_var(&name, "from-shell");
        load(&path).unwrap();
        assert_eq!(std::env::var(&name).unwrap(), "from-shell");
        std::env::remove_var(name);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn malformed_entries_do_not_expose_secrets() {
        let error = parse(b"PASSWORD='private-secret\n".as_slice())
            .unwrap_err()
            .to_string();
        assert!(!error.contains("private-secret"));
    }
}
