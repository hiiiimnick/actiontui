use color_eyre::eyre::{Result, eyre};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct Repository {
    pub owner: String,
    pub repo: String,
}

impl Repository {
    pub fn new(owner: String, repo: String) -> Self {
        Self { owner, repo }
    }

    pub fn parse_current() -> Result<Self> {
        let output = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .arg("/C")
                .arg("git config --get remote.origin.url")
                .output()?
        } else {
            Command::new("sh")
                .arg("-c")
                .arg("git config --get remote.origin.url")
                .output()?
        };

        if !output.status.success() {
            return Err(eyre!(
                "Failed to get remote origin URL. Are you in a git repository?"
            ));
        }

        let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Self::parse_url(&url)
    }

    pub fn parse_url(url: &str) -> Result<Self> {
        let path = if let Some(ssh_part) = url.strip_prefix("git@") {
            ssh_part
                .split_once(':')
                .map(|x| x.1)
                .ok_or_else(|| eyre!("Invalid SSH URL: {}", url))?
        } else {
            url.trim_start_matches("https://")
                .trim_start_matches("http://")
                .split_once('/')
                .map(|x| x.1)
                .ok_or_else(|| eyre!("Invalid HTTP URL: {}", url))?
        };

        let path = path.trim_end_matches(".git");
        let parts: Vec<&str> = path.split('/').collect();

        if parts.len() < 2 {
            return Err(eyre!(
                "Could not determine owner and repo from URL: {}",
                url
            ));
        }

        Ok(Repository {
            owner: parts[parts.len() - 2].to_string(),
            repo: parts[parts.len() - 1].to_string(),
        })
    }
}
