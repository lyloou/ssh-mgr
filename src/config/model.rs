use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
pub struct FileConfig {
    #[serde(default)]
    pub server: Vec<Server>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Server {
    pub name: String,
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    pub user: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub password: Option<PasswordConfig>,
    #[serde(skip)]
    pub group: String,
    #[serde(skip)]
    pub source: PathBuf,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct PasswordConfig {
    pub key: Option<String>,
    pub env: Option<String>,
    pub value: Option<String>,
    #[serde(default)]
    pub prompt: bool,
}

fn default_port() -> u16 {
    22
}

impl Server {
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("name cannot be empty".into());
        }
        if self.host.trim().is_empty() {
            return Err("host cannot be empty".into());
        }
        if self.user.trim().is_empty() {
            return Err("user cannot be empty".into());
        }
        if self.port == 0 {
            return Err("port must be between 1 and 65535".into());
        }
        if let Some(password) = &self.password {
            let configured = [
                password.key.is_some(),
                password.env.is_some(),
                password.value.is_some(),
                password.prompt,
            ]
            .into_iter()
            .filter(|configured| *configured)
            .count();
            if configured > 1 {
                return Err("password must use exactly one of key, env, value, or prompt".into());
            }
            if let Some(env) = &password.env {
                if env.trim().is_empty() {
                    return Err("password.env cannot be empty".into());
                }
            }
        }
        Ok(())
    }

    pub fn searchable_text(&self) -> String {
        format!(
            "{} {} {} {} {} {}",
            self.group,
            self.name,
            self.description,
            self.host,
            self.user,
            self.tags.join(" ")
        )
    }
}
