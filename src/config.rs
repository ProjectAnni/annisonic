use serde::{Serialize, Deserialize};
use std::path::Path;
use std::fs;

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub repo: RepoConfig,
    pub backend: BackendConfig,
}

impl Config {
    pub fn from_file<P: AsRef<Path>>(config_path: P) -> anyhow::Result<Self> {
        let string = fs::read_to_string(config_path)?;
        let result = toml::from_str(&string)?;
        Ok(result)
    }
}

#[derive(Serialize, Deserialize)]
pub struct ServerConfig {
    listen: Option<String>,
    #[serde(default = "anni")]
    pub username: String,
    #[serde(default = "anni")]
    pub password: String,
}

fn anni() -> String {
    "anni".to_string()
}

impl ServerConfig {
    pub fn listen(&self, default: &'static str) -> &str {
        if let Some(listen) = &self.listen {
            listen.as_str()
        } else {
            default
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct RepoConfig {
    #[serde(rename = "type")]
    repo_type: String,
    pub root: String,
}

#[derive(Serialize, Deserialize)]
pub struct BackendConfig {
    #[serde(rename = "type")]
    pub backend_type: String,

    root: Option<String>,
    #[serde(default)]
    pub strict: bool,
}

impl BackendConfig {
    pub fn root(&self) -> &str {
        if let Some(root) = &self.root {
            root.as_str()
        } else {
            panic!("no root provided!")
        }
    }
}