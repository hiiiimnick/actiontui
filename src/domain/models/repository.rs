use color_eyre::eyre::{Result, eyre};
use std::process::Command;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Repository {
    /// The GitHub host of the remote, lower case, e.g. `github.com`.
    pub host: String,
    pub owner: String,
    pub repo: String,
}

impl Repository {
    pub fn new(host: String, owner: String, repo: String) -> Self {
        Self { host, owner, repo }
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

    /// The branch currently checked out, if HEAD is on a branch.
    pub fn current_branch() -> Result<String> {
        let output = Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .output()?;
        let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !output.status.success() || branch.is_empty() || branch == "HEAD" {
            return Err(eyre!("Not on a branch"));
        }
        Ok(branch)
    }

    /// Reads host, owner and name from a remote URL, either
    /// `scheme://[user@]host[:port]/owner/repo[.git]` or the scp-like
    /// `[user@]host:owner/repo[.git]`.
    pub fn parse_url(url: &str) -> Result<Self> {
        let url = url.trim();
        let (host, path) = if let Some((_, rest)) = url.split_once("://") {
            let (authority, path) = rest
                .split_once('/')
                .ok_or_else(|| eyre!("Invalid URL: {url}"))?;
            let host_port = authority.rsplit('@').next().unwrap_or(authority);
            (host_port.split(':').next().unwrap_or(host_port), path)
        } else {
            let (left, path) = url
                .split_once(':')
                .ok_or_else(|| eyre!("Invalid SSH URL: {url}"))?;
            (left.rsplit('@').next().unwrap_or(left), path)
        };
        if host.is_empty() {
            return Err(eyre!("Could not determine the host from URL: {url}"));
        }

        let path = path.trim_matches('/').trim_end_matches(".git");
        let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
        if parts.len() < 2 {
            return Err(eyre!("Could not determine owner and repo from URL: {url}"));
        }

        Ok(Repository {
            host: host.to_ascii_lowercase(),
            owner: parts[parts.len() - 2].to_string(),
            repo: parts[parts.len() - 1].to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(url: &str) -> (String, String, String) {
        let repo = Repository::parse_url(url).unwrap();
        (repo.host, repo.owner, repo.repo)
    }

    fn expected(host: &str, owner: &str, repo: &str) -> (String, String, String) {
        (host.into(), owner.into(), repo.into())
    }

    #[test]
    fn https_urls() {
        let github = expected("github.com", "me", "tool");
        assert_eq!(parse("https://github.com/me/tool.git"), github);
        assert_eq!(parse("https://github.com/me/tool"), github);
        assert_eq!(parse("https://github.com/me/tool/"), github);
        assert_eq!(parse("https://user:token@github.com/me/tool.git"), github);
        assert_eq!(
            parse("https://GitHub.Example.com:8443/acme/api.git"),
            expected("github.example.com", "acme", "api")
        );
    }

    #[test]
    fn ssh_urls() {
        let github = expected("github.com", "me", "tool");
        assert_eq!(parse("git@github.com:me/tool.git"), github);
        assert_eq!(parse("github.com:me/tool"), github);
        assert_eq!(parse("ssh://git@github.com/me/tool.git"), github);
        assert_eq!(
            parse("ssh://git@github.example.com:2222/acme/api.git"),
            expected("github.example.com", "acme", "api")
        );
    }

    #[test]
    fn invalid_urls() {
        for url in [
            "",
            "/home/me/tool",
            "https://github.com",
            "https://github.com/me",
            "git@github.com:tool",
        ] {
            assert!(Repository::parse_url(url).is_err(), "{url}");
        }
    }
}
