use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub url: String,
    pub pat: String,
}

impl ::std::default::Default for Config {
    fn default() -> Self {
        Self {
            url: "https://www.github.com".into(),
            pat: "".into(),
        }
    }
}
