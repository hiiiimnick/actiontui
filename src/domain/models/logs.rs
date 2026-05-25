use std::{collections::HashMap, fs::File, io::Write};

use color_eyre::eyre::{Result, eyre};
use tempfile::{NamedTempFile, tempfile};

use crate::domain::Step;

#[derive(Debug)]
pub struct Logs {
    pub text: Vec<u8>,
    pub length: usize,
}

impl Logs {
    pub fn save_to_file(&self) -> Result<File> {
        println!("{}", std::env::temp_dir().display());
        let mut file = tempfile()?;
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

        let search_terms: Vec<String> = steps
            .iter()
            .map(|s| s.started_at.replace("Z", ""))
            .collect();

        while i < self.length {
            if self.text[i..].starts_with(search_terms[step_count].as_bytes()) {
                if step_count < steps.len() - 1 {
                    step_index_map.insert(
                        steps[step_count - 1].name.clone(),
                        (temp_start, i as u64 - temp_start),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Step;

    #[test]
    fn test_create_step_index_exact_match() {
        let text = b"2023-11-20T14:48:00Z step 1\n2023-11-20T14:48:01Z step 2\n";
        let logs = Logs {
            text: text.to_vec(),
            length: text.len(),
        };
        let steps = vec![
            Step {
                name: "Step 1".to_string(),
                status: "completed".to_string(),
                conclusion: Some("success".to_string()),
                number: 1,
                started_at: "2023-11-20T14:48:00Z".to_string(),
                completed_at: "2023-11-20T14:48:01Z".to_string(),
            },
            Step {
                name: "Step 2".to_string(),
                status: "completed".to_string(),
                conclusion: Some("success".to_string()),
                number: 2,
                started_at: "2023-11-20T14:48:01Z".to_string(),
                completed_at: "2023-11-20T14:48:02Z".to_string(),
            },
        ];

        let index = logs.create_step_index(steps).unwrap();
        assert_eq!(index.len(), 2);
        assert!(index.contains_key("Step 1"));
        assert!(index.contains_key("Step 2"));
    }

    #[test]
    fn test_create_step_index_mismatch() {
        let text = b"2023-11-20T14:48:00.123Z step 1\n2023-11-20T14:48:01.456Z step 2\n";
        let logs = Logs {
            text: text.to_vec(),
            length: text.len(),
        };
        let steps = vec![
            Step {
                name: "Step 1".to_string(),
                status: "completed".to_string(),
                conclusion: Some("success".to_string()),
                number: 1,
                started_at: "2023-11-20T14:48:00Z".to_string(),
                completed_at: "2023-11-20T14:48:01Z".to_string(),
            },
            Step {
                name: "Step 2".to_string(),
                status: "completed".to_string(),
                conclusion: Some("success".to_string()),
                number: 2,
                started_at: "2023-11-20T14:48:01Z".to_string(),
                completed_at: "2023-11-20T14:48:02Z".to_string(),
            },
        ];

        let index = logs.create_step_index(steps).unwrap();
        assert_eq!(
            index.len(),
            2,
            "Index should have 2 entries even with fractional seconds in logs"
        );
    }
}
