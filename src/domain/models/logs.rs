use std::{collections::HashMap, fs::File, io::Write};

use color_eyre::eyre::{Result, eyre};
use tempfile::tempfile;

use crate::domain::Step;

#[derive(Debug)]
pub struct Logs {
    pub text: Vec<u8>,
    pub length: usize,
}

impl Logs {
    pub fn save_to_file(&self) -> Result<File> {
        let mut file = tempfile()
            .map_err(|error| eyre!("error creating tempfile: {}", error))
            .unwrap();
        let _ = file.write_all(&self.text);
        Ok(file)
    }

    pub fn create_step_index(&self, steps: Vec<Step>) -> Result<HashMap<String, (u64, u64)>> {
        let mut i = 0;
        let mut step_count = 1;
        let mut step_index_map: HashMap<String, (u64, u64)> = HashMap::new();
        if steps.len() == 1 {
            step_index_map.insert(steps[0].name.clone(), (0, self.length as u64 - 1));
            return Ok(step_index_map);
        }
        let mut temp_start = 0;

        while i < self.length {
            if self.text[i..].starts_with(steps[step_count].started_at.as_bytes()) {
                if step_count < steps.len() - 1 {
                    step_index_map.insert(
                        steps[step_count - 1].name.clone(),
                        (temp_start, i as u64 - 1 - temp_start),
                    );
                    temp_start = i as u64;
                    step_count += 1;
                } else {
                    step_index_map.insert(
                        steps[step_count - 1].name.clone(),
                        (temp_start, i as u64 - 1 - temp_start),
                    );
                    step_index_map.insert(
                        steps[step_count].name.clone(),
                        (i as u64, (self.length - 1 - i) as u64),
                    );
                    break;
                }
            }
            i += 1;
        }

        Ok(step_index_map)
    }
}
