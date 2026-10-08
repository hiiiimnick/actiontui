use std::fmt;

use color_eyre::eyre::{Result, eyre};
use serde::{Deserialize, Serialize};

/// The settings file: any number of GitHub accounts, called profiles.
///
/// The old single-account format (`url` and `pat` at the top level) is still
/// read and treated as one profile named `default`.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    /// Profile to use when several match the repository.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_profile: Option<String>,

    // the old single-account format
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pat: Option<String>,

    #[serde(default)]
    pub profiles: Vec<Profile>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Profile {
    pub name: String,
    /// The GitHub host, e.g. `github.com` or `github.example.com`.
    #[serde(default = "default_url")]
    pub url: String,
    /// Only use this profile for repositories of this owner.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    /// The token itself ...
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pat: Option<String>,
    /// ... or the name of an environment variable holding it. Wins over `pat`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pat_env: Option<String>,
    /// Base URL of the REST API, if it cannot be derived from `url`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_url: Option<String>,
}

fn default_url() -> String {
    "github.com".into()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_profile: None,
            url: None,
            pat: None,
            profiles: vec![Profile {
                name: "github".into(),
                url: default_url(),
                owner: None,
                pat: Some(String::new()),
                pat_env: None,
                api_url: None,
            }],
        }
    }
}

/// A profile ready to make API requests with.
#[derive(Clone)]
pub struct Account {
    pub profile: String,
    /// Host of the account, e.g. `github.com`.
    pub host: String,
    pub api_base: String,
    pub token: String,
}

// the token must never end up in a log or debug output
impl fmt::Debug for Account {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Account")
            .field("profile", &self.profile)
            .field("host", &self.host)
            .field("api_base", &self.api_base)
            .field("token", &"<redacted>")
            .finish()
    }
}

impl Config {
    /// All profiles, including the one of the old single-account format.
    pub fn all_profiles(&self) -> Vec<Profile> {
        let mut profiles = self.profiles.clone();
        if profiles.is_empty() && (self.url.is_some() || self.pat.is_some()) {
            profiles.push(Profile {
                name: "default".into(),
                url: self.url.clone().unwrap_or_else(default_url),
                owner: None,
                pat: self.pat.clone(),
                pat_env: None,
                api_url: None,
            });
        }
        profiles
    }

    /// Picks the profile for a repository.
    ///
    /// `requested` (the `--profile` flag) wins. Otherwise profiles are matched
    /// on the host of the repository's remote; a profile with an `owner` only
    /// matches that owner and beats one without. If several are equally
    /// specific, `default_profile` decides. A single configured profile is
    /// used even when nothing matches.
    pub fn select_profile(
        &self,
        requested: Option<&str>,
        host: &str,
        owner: &str,
    ) -> Result<Profile> {
        let profiles = self.all_profiles();
        if profiles.is_empty() {
            return Err(eyre!("No profile configured"));
        }

        if let Some(name) = requested {
            return profiles
                .iter()
                .find(|p| p.name == name)
                .cloned()
                .ok_or_else(|| eyre!("Unknown profile '{name}'. Available: {}", names(&profiles)));
        }

        let matching: Vec<&Profile> = profiles.iter().filter(|p| p.matches_host(host)).collect();
        let specific: Vec<&Profile> = matching
            .iter()
            .copied()
            .filter(|p| {
                p.owner
                    .as_deref()
                    .is_some_and(|o| o.eq_ignore_ascii_case(owner))
            })
            .collect();
        // a profile for another owner never applies
        let general = matching.iter().copied().filter(|p| p.owner.is_none());
        let candidates: Vec<&Profile> = if specific.is_empty() {
            general.collect()
        } else {
            specific
        };

        match candidates.as_slice() {
            [only] => Ok((*only).clone()),
            [] if profiles.len() == 1 => Ok(profiles[0].clone()),
            [] => Err(eyre!(
                "No profile for {host}/{owner}. Add one to the config or pass --profile. \
                 Available: {}",
                names(&profiles)
            )),
            several => several
                .iter()
                .find(|p| self.default_profile.as_deref() == Some(p.name.as_str()))
                .map(|p| (*p).clone())
                .ok_or_else(|| {
                    eyre!(
                        "Several profiles match {host}/{owner}: {}. Pass --profile or set \
                         default_profile",
                        several
                            .iter()
                            .map(|p| p.name.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                }),
        }
    }
}

fn names(profiles: &[Profile]) -> String {
    profiles
        .iter()
        .map(|p| p.name.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

impl Profile {
    /// `host[:port]` of `url`, lower case. A scheme and anything after the
    /// host, such as a path, are ignored.
    fn authority(&self) -> String {
        let url = self.url.trim();
        let url = url.split_once("://").map_or(url, |(_, rest)| rest);
        let authority = url.split('/').next().unwrap_or(url);
        authority.to_ascii_lowercase()
    }

    /// The host without scheme, port and path, lower case.
    pub fn host(&self) -> String {
        let authority = self.authority();
        authority
            .split_once(':')
            .map_or(authority.clone(), |(host, _)| host.to_string())
    }

    fn matches_host(&self, host: &str) -> bool {
        self.host() == host.to_ascii_lowercase()
    }

    /// Base URL of the REST API.
    /// - github.com: `https://api.github.com`
    /// - GitHub Enterprise Cloud with data residency (`*.ghe.com`):
    ///   `https://api.<host>`
    /// - GitHub Enterprise Server: `https://<host>/api/v3`
    ///
    /// `api_url` overrides all of these.
    pub fn api_base(&self) -> String {
        if let Some(api_url) = &self.api_url {
            return api_url.trim_end_matches('/').to_string();
        }
        let host = self.host();
        if host == "github.com" || host.ends_with(".ghe.com") {
            format!("https://api.{host}")
        } else {
            format!("https://{}/api/v3", self.authority())
        }
    }

    pub fn token(&self) -> Result<String> {
        self.token_with(|name| std::env::var(name).ok())
    }

    fn token_with(&self, env: impl Fn(&str) -> Option<String>) -> Result<String> {
        let token = match (&self.pat_env, &self.pat) {
            (Some(var), _) => env(var).ok_or_else(|| {
                eyre!(
                    "Profile '{}': environment variable {var} is not set",
                    self.name
                )
            })?,
            (None, Some(pat)) => pat.clone(),
            (None, None) => String::new(),
        };
        let token = token.trim().to_string();
        if token.is_empty() {
            return Err(eyre!(
                "Profile '{}' has no token. Set `pat` or `pat_env`",
                self.name
            ));
        }
        Ok(token)
    }

    pub fn account(&self) -> Result<Account> {
        Ok(Account {
            profile: self.name.clone(),
            host: self.host(),
            api_base: self.api_base(),
            token: self.token()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(name: &str, url: &str, owner: Option<&str>) -> Profile {
        Profile {
            name: name.into(),
            url: url.into(),
            owner: owner.map(Into::into),
            pat: Some("secret".into()),
            pat_env: None,
            api_url: None,
        }
    }

    fn config(profiles: Vec<Profile>) -> Config {
        Config {
            default_profile: None,
            url: None,
            pat: None,
            profiles,
        }
    }

    fn selected(config: &Config, requested: Option<&str>, host: &str, owner: &str) -> String {
        config.select_profile(requested, host, owner).unwrap().name
    }

    #[test]
    fn legacy_single_account_format_still_loads() {
        let config: Config = toml::from_str("url = \"github.com\"\npat = \"ghp_abc\"\n").unwrap();
        let profiles = config.all_profiles();
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].name, "default");
        assert_eq!(profiles[0].url, "github.com");
        assert_eq!(profiles[0].token().unwrap(), "ghp_abc");
    }

    #[test]
    fn profiles_format_loads() {
        let config: Config = toml::from_str(
            r#"
default_profile = "work"

[[profiles]]
name = "personal"
pat = "ghp_1"

[[profiles]]
name = "work"
url = "github.example.com"
owner = "acme"
pat_env = "WORK_TOKEN"
"#,
        )
        .unwrap();
        assert_eq!(config.default_profile.as_deref(), Some("work"));
        assert_eq!(config.profiles.len(), 2);
        assert_eq!(config.profiles[0].url, "github.com");
        assert_eq!(config.profiles[1].owner.as_deref(), Some("acme"));
    }

    #[test]
    fn default_config_roundtrips_through_toml() {
        let text = toml::to_string(&Config::default()).unwrap();
        let config: Config = toml::from_str(&text).unwrap();
        assert_eq!(config.profiles.len(), 1);
        assert_eq!(config.profiles[0].url, "github.com");
    }

    #[test]
    fn host_decides_the_profile() {
        let config = config(vec![
            profile("personal", "github.com", None),
            profile("work", "https://GitHub.Example.com/", None),
        ]);
        assert_eq!(selected(&config, None, "github.com", "me"), "personal");
        assert_eq!(selected(&config, None, "github.example.com", "me"), "work");
    }

    #[test]
    fn owner_makes_a_profile_more_specific() {
        let config = config(vec![
            profile("personal", "github.com", None),
            profile("acme", "github.com", Some("Acme")),
        ]);
        assert_eq!(selected(&config, None, "github.com", "acme"), "acme");
        assert_eq!(selected(&config, None, "github.com", "me"), "personal");
    }

    #[test]
    fn profile_for_another_owner_never_applies() {
        let config = config(vec![
            profile("acme", "github.com", Some("acme")),
            profile("work", "github.example.com", None),
        ]);
        assert!(config.select_profile(None, "github.com", "me").is_err());
    }

    #[test]
    fn flag_overrides_detection() {
        let config = config(vec![
            profile("personal", "github.com", None),
            profile("work", "github.example.com", None),
        ]);
        assert_eq!(selected(&config, Some("work"), "github.com", "me"), "work");
    }

    #[test]
    fn unknown_profile_lists_the_available_ones() {
        let config = config(vec![profile("personal", "github.com", None)]);
        let error = config
            .select_profile(Some("nope"), "github.com", "me")
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("nope") && error.contains("personal"),
            "{error}"
        );
    }

    #[test]
    fn ambiguous_matches_use_the_default_profile() {
        let mut config = config(vec![
            profile("a", "github.com", None),
            profile("b", "github.com", None),
        ]);
        assert!(config.select_profile(None, "github.com", "me").is_err());
        config.default_profile = Some("b".into());
        assert_eq!(selected(&config, None, "github.com", "me"), "b");
    }

    #[test]
    fn a_single_profile_is_used_even_without_a_match() {
        let config = config(vec![profile("only", "github.com", None)]);
        assert_eq!(selected(&config, None, "ghe.corp", "me"), "only");
    }

    #[test]
    fn no_profile_at_all_is_an_error() {
        assert!(
            config(vec![])
                .select_profile(None, "github.com", "me")
                .is_err()
        );
    }

    #[test]
    fn api_base_per_kind_of_host() {
        let base = |url: &str| profile("p", url, None).api_base();
        assert_eq!(base("github.com"), "https://api.github.com");
        assert_eq!(base("https://github.com/"), "https://api.github.com");
        assert_eq!(base("octocorp.ghe.com"), "https://api.octocorp.ghe.com");
        assert_eq!(
            base("github.example.com"),
            "https://github.example.com/api/v3"
        );

        let mut custom = profile("p", "github.example.com", None);
        custom.api_url = Some("https://api.example.com/v3/".into());
        assert_eq!(custom.api_base(), "https://api.example.com/v3");
    }

    #[test]
    fn url_is_reduced_to_the_host() {
        let host = |url: &str| profile("p", url, None).host();
        assert_eq!(host("github.com"), "github.com");
        assert_eq!(host("https://GitHub.com/"), "github.com");
        // a path (e.g. the organization) is not part of the host
        assert_eq!(
            host("mercedes-benz.ghe.com/DATIX-FRITZ"),
            "mercedes-benz.ghe.com"
        );
        assert_eq!(
            host("https://mercedes-benz.ghe.com/DATIX-FRITZ/"),
            "mercedes-benz.ghe.com"
        );
        assert_eq!(host("github.example.com:8443/x"), "github.example.com");
    }

    #[test]
    fn a_url_with_a_path_still_matches_and_gets_the_right_api() {
        let config = config(vec![
            profile("personal", "github.com", None),
            profile("work", "mercedes-benz.ghe.com/DATIX-FRITZ", None),
        ]);
        assert_eq!(
            selected(&config, None, "mercedes-benz.ghe.com", "DATIX-FRITZ"),
            "work"
        );
        assert_eq!(
            profile("work", "mercedes-benz.ghe.com/DATIX-FRITZ", None).api_base(),
            "https://api.mercedes-benz.ghe.com"
        );
    }

    #[test]
    fn a_port_is_kept_for_the_api_but_not_for_matching() {
        let p = profile("p", "github.example.com:8443", None);
        assert_eq!(p.api_base(), "https://github.example.com:8443/api/v3");
        assert!(p.matches_host("github.example.com"));
    }

    #[test]
    fn token_from_pat_or_environment() {
        let mut p = profile("p", "github.com", None);
        assert_eq!(p.token_with(|_| None).unwrap(), "secret");

        p.pat_env = Some("MY_TOKEN".into());
        let env = |name: &str| (name == "MY_TOKEN").then(|| " from-env \n".to_string());
        assert_eq!(p.token_with(env).unwrap(), "from-env");

        let error = p.token_with(|_| None).unwrap_err().to_string();
        assert!(error.contains("MY_TOKEN"), "{error}");
    }

    #[test]
    fn missing_or_empty_token_is_an_error() {
        let mut p = profile("p", "github.com", None);
        p.pat = Some("  ".into());
        assert!(p.token_with(|_| None).is_err());
        p.pat = None;
        assert!(p.token_with(|_| None).is_err());
    }

    #[test]
    fn account_debug_hides_the_token() {
        let account = profile("p", "github.com", None).account().unwrap();
        let text = format!("{account:?}");
        assert!(!text.contains("secret"), "{text}");
        assert!(text.contains("redacted"));
    }
}
