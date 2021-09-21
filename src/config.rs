use serde::Deserialize;
use std::path::Path;
use std::fs;

#[derive(Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub repo: RepoConfig,
    pub annil: AnnilConfig,
}

impl Config {
    pub fn from_file<P: AsRef<Path>>(config_path: P) -> anyhow::Result<Self> {
        let string = fs::read_to_string(config_path)?;
        let result = toml::from_str(&string)?;
        Ok(result)
    }
}

#[derive(Deserialize)]
pub struct ServerConfig {
    listen: Option<String>,
    pub username: String,
    pub password: String,
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

#[derive(Deserialize)]
pub struct RepoConfig {
    pub root: String,
}

#[derive(Deserialize, Clone)]
pub struct AnnilConfig {
    server: String,
    token: String,
}

impl AnnilConfig {
    // removing padding '/'
    fn server(&self) -> &str {
        if self.server.ends_with("/") {
            &self.server[0..self.server.len() - 1]
        } else {
            self.server.as_str()
        }
    }

    pub async fn albums(&self) -> anyhow::Result<Vec<String>> {
        let r = reqwest::get(format!("{}/albums?auth={}", self.server(), self.token)).await?;
        Ok(r.json().await?)
    }

    pub fn get_url(&self, middle: &str) -> String {
        format!("{}/{}?auth={}", self.server(), middle, self.token)
    }
}

