use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub url: String,
    pub pat: String,
}

impl ::std::default::Default for Config {
    fn default() -> Self {
        Self {
            url: "github.com".into(),
            pat: "".into(),
        }
    }
}
