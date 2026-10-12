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
