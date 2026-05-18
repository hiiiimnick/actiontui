use std::{collections::HashMap, fs::File, io::Write};

use color_eyre::eyre::{Result, eyre};
use tempfile::tempfile;

use crate::domain::Step;

pub struct Logs {
    text: Vec<u8>,
    length: u64,
}

impl Logs {
    pub fn save_to_file(&self) -> Result<File> {
        let mut file = tempfile().map_err(|error| eyre!("error creating tempfile: {}", error))?;
        let _ = file.write_all(&self.text);
        Ok(file)
    }

    pub fn create_step_index(&self, steps: Vec<Step>) -> Result<HashMap<String, (u64, u64)>> {
        let mut i = 0;
        let mut step_count = 0;
        let mut step_index_map: HashMap<String, (u64, u64)> = HashMap::new();

        while i < self.text.len() {
            if self.text[i..].starts_with(steps[step_count].started_at.as_bytes()) {}
            i += 1;
        }

        Ok(HashMap::default())
    }
}
