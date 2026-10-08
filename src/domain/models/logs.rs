use std::{collections::HashMap, fs::File, io::Write};

use color_eyre::eyre::Result;
use tempfile::tempfile;

use crate::domain::Step;

#[derive(Debug)]
pub struct Logs {
    pub text: Vec<u8>,
    pub length: usize,
}

impl Logs {
    pub fn save_to_file(&self) -> Result<File> {
        let mut file = tempfile()?;
        let _ = file.write_all(&self.text);
        Ok(file)
    }

    /// Maps every step name to its `(start, len)` byte range in the log.
    ///
    /// GitHub only reports second-resolution start times, and several steps
    /// regularly start within the same second, so steps are located in layers:
    /// - skipped steps produce no output and get an empty range,
    /// - `Post ...` and `Complete job` steps are found by the line they
    ///   always begin with,
    /// - all other steps are found by comparing the timestamp of each log line
    ///   with the step's `started_at`. If that is ambiguous (several steps
    ///   start in the same second, or the previous step ends in it), the
    ///   `##[group]` header lines of that second decide.
    ///
    /// Every step is always present in the map; steps that could not be
    /// located get an empty range.
    pub fn create_step_index(&self, steps: &[Step]) -> Result<HashMap<String, (u64, u64)>> {
        let total = self.text.len().min(self.length);
        let lines = scan_lines(&self.text[..total]);

        let mut starts: Vec<Option<u64>> = vec![None; steps.len()];

        // marker based steps; they always come after every regular step
        let kinds: Vec<Kind> = steps.iter().map(Kind::of).collect();
        let post_lines = lines
            .iter()
            .filter(|l| l.body.starts_with(b"Post job cleanup."));
        let complete_line = lines
            .iter()
            .find(|l| l.body.starts_with(b"Cleaning up orphan processes"));
        let mut post_lines = post_lines;
        for (i, kind) in kinds.iter().enumerate() {
            match kind {
                Kind::Post => starts[i] = post_lines.next().map(|l| l.offset as u64),
                Kind::Complete => starts[i] = complete_line.map(|l| l.offset as u64),
                _ => {}
            }
        }
        let limit = lines
            .iter()
            .position(|l| {
                l.body.starts_with(b"Post job cleanup.")
                    || l.body.starts_with(b"Cleaning up orphan processes")
            })
            .unwrap_or(lines.len());

        // timestamp based steps
        let regular: Vec<usize> = (0..steps.len())
            .filter(|&i| kinds[i] == Kind::Regular)
            .collect();
        let start_secs: Vec<&[u8]> = regular
            .iter()
            .map(|&i| steps[i].started_at.as_bytes()[..19].as_ref())
            .collect();
        if let Some(&first) = regular.first() {
            starts[first] = Some(0);
        }

        let mut cur = 0; // position within `regular`
        let mut li = 1;
        while li < limit {
            let line = &lines[li];
            let Some(ts) = line.ts else {
                li += 1;
                continue;
            };
            while cur + 1 < regular.len() {
                let next = cur + 1;
                match ts.cmp(start_secs[next]) {
                    std::cmp::Ordering::Less => break,
                    std::cmp::Ordering::Greater => {
                        starts[regular[next]] = Some(line.offset as u64);
                        cur = next;
                    }
                    std::cmp::Ordering::Equal => {
                        // steps sharing this start second, in order
                        let tied = start_secs[next..].iter().take_while(|t| **t == ts).count();
                        let prev_ends_here =
                            steps[regular[cur]].completed_at.as_bytes().get(..19) == Some(ts);
                        let prev_starts_here = start_secs[cur] == ts;
                        if tied == 1 && !prev_ends_here && !prev_starts_here {
                            starts[regular[next]] = Some(line.offset as u64);
                            cur = next;
                            continue;
                        }
                        // the group headers within this second mark the starts
                        let mut groups: Vec<usize> = (li..limit)
                            .take_while(|&j| lines[j].ts.is_none_or(|t| t <= ts))
                            .filter(|&j| lines[j].is_group && lines[j].ts == Some(ts))
                            .collect();
                        // top-level steps are headed `##[group]Run ...`, other
                        // groups are nested inside a step
                        let top_level: Vec<usize> = groups
                            .iter()
                            .copied()
                            .filter(|&j| lines[j].body.starts_with(b"##[group]Run "))
                            .collect();
                        if !top_level.is_empty() {
                            groups = top_level;
                        }
                        if groups.is_empty() {
                            starts[regular[next]] = Some(line.offset as u64);
                            cur = next;
                            continue;
                        }
                        // Nested groups (e.g. of composite actions) look like
                        // step headers, so the position decides: the last
                        // regular step is followed by the post steps, hence its
                        // header is the last group of the second; any other
                        // step's header is the first.
                        let last_regular = next + tied == regular.len();
                        let skip = if last_regular {
                            groups.len().saturating_sub(tied)
                        } else {
                            0
                        };
                        for (n, &g) in groups[skip..].iter().take(tied).enumerate() {
                            starts[regular[next + n]] = Some(lines[g].offset as u64);
                            cur = next + n;
                        }
                        break;
                    }
                }
            }
            li += 1;
        }

        let mut index = HashMap::with_capacity(steps.len());
        for (i, step) in steps.iter().enumerate() {
            let range = match starts[i] {
                Some(start) => {
                    let end = starts[i + 1..]
                        .iter()
                        .flatten()
                        .next()
                        .map_or(total as u64, |&e| e);
                    (start, end.saturating_sub(start))
                }
                None => (0, 0),
            };
            index.insert(step.name.clone(), range);
        }
        Ok(index)
    }
}

#[derive(PartialEq, Eq)]
enum Kind {
    /// Skipped or never started: no log output at all.
    NoOutput,
    Post,
    Complete,
    Regular,
}

impl Kind {
    fn of(step: &Step) -> Kind {
        if step.conclusion.as_deref() == Some("skipped")
            || !is_timestamp(step.started_at.as_bytes())
        {
            Kind::NoOutput
        } else if step.name.starts_with("Post ") {
            Kind::Post
        } else if step.name == "Complete job" {
            Kind::Complete
        } else {
            Kind::Regular
        }
    }
}

struct LogLine<'a> {
    offset: usize,
    /// `YYYY-MM-DDTHH:MM:SS` prefix, if the line has a timestamp.
    ts: Option<&'a [u8]>,
    /// The line without its timestamp.
    body: &'a [u8],
    is_group: bool,
}

fn scan_lines(text: &[u8]) -> Vec<LogLine<'_>> {
    let mut offset = 0;
    text.split_inclusive(|&b| b == b'\n')
        .map(|raw| {
            let line_offset = offset;
            offset += raw.len();
            let line = raw.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(raw);
            let (ts, body) = if is_timestamp(line) {
                // skip the fractional seconds and the "Z "
                let rest = &line[19..];
                let end = rest
                    .iter()
                    .position(|&b| b == b' ')
                    .map_or(rest.len(), |p| p + 1);
                (Some(&line[..19]), &rest[end..])
            } else {
                (None, line)
            };
            LogLine {
                offset: line_offset,
                ts,
                body,
                is_group: body.starts_with(b"##[group]"),
            }
        })
        .collect()
}

/// Whether `bytes` begins with an ISO-8601 `YYYY-MM-DDTHH:MM:SS` timestamp.
fn is_timestamp(bytes: &[u8]) -> bool {
    bytes.len() >= 19
        && bytes[..19].iter().enumerate().all(|(i, b)| match i {
            4 | 7 => *b == b'-',
            10 => *b == b'T',
            13 | 16 => *b == b':',
            _ => b.is_ascii_digit(),
        })
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

        let index = logs.create_step_index(&steps).unwrap();
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

        let index = logs.create_step_index(&steps).unwrap();
        assert_eq!(
            index.len(),
            2,
            "Index should have 2 entries even with fractional seconds in logs"
        );
    }

    fn step(name: &str, started_at: &str) -> Step {
        Step {
            name: name.to_string(),
            status: "completed".to_string(),
            conclusion: Some("success".to_string()),
            number: 0,
            started_at: started_at.to_string(),
            completed_at: String::new(),
        }
    }

    fn logs(text: &str) -> Logs {
        Logs {
            text: text.as_bytes().to_vec(),
            length: text.len(),
        }
    }

    #[test]
    fn test_create_step_index_no_steps() {
        assert!(
            logs("whatever\n")
                .create_step_index(&[])
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn test_create_step_index_empty_log() {
        let steps = [
            step("a", "2023-11-20T14:48:00Z"),
            step("b", "2023-11-20T14:48:01Z"),
        ];
        let index = logs("").create_step_index(&steps).unwrap();
        assert_eq!(index["a"], (0, 0));
        assert_eq!(index["b"], (0, 0));
    }

    #[test]
    fn test_create_step_index_skipped_step() {
        let text =
            "2023-11-20T14:48:00.1Z a\n2023-11-20T14:48:00.2Z more a\n2023-11-20T14:48:05.0Z c\n";
        let steps = [
            step("a", "2023-11-20T14:48:00Z"),
            step("skipped", ""),
            step("c", "2023-11-20T14:48:05Z"),
        ];
        let index = logs(text).create_step_index(&steps).unwrap();
        let split = text.find("2023-11-20T14:48:05").unwrap() as u64;
        assert_eq!(index["a"], (0, split));
        assert_eq!(index["skipped"], (0, 0));
        assert_eq!(index["c"], (split, text.len() as u64 - split));
    }

    #[test]
    fn test_create_step_index_step_without_output() {
        let text = "2023-11-20T14:48:00Z a\n2023-11-20T14:48:09Z c\n";
        let steps = [
            step("a", "2023-11-20T14:48:00Z"),
            step("b", "2023-11-20T14:48:03Z"),
            step("c", "2023-11-20T14:48:09Z"),
        ];
        let index = logs(text).create_step_index(&steps).unwrap();
        assert_eq!(index["b"].1, 0);
        assert_eq!(index["a"].0 + index["a"].1, index["c"].0);
        assert_eq!(index["c"].0 + index["c"].1, text.len() as u64);
    }

    #[test]
    fn test_create_step_index_same_second_uses_group_header() {
        let text = "2023-11-20T14:48:00Z ##[group]Run a\n2023-11-20T14:48:00Z out a\n\
                    2023-11-20T14:48:00Z ##[group]Run b\n2023-11-20T14:48:00Z out b\n";
        let steps = [
            step("a", "2023-11-20T14:48:00Z"),
            step("b", "2023-11-20T14:48:00Z"),
        ];
        let index = logs(text).create_step_index(&steps).unwrap();
        let split = text.find("2023-11-20T14:48:00Z ##[group]Run b").unwrap() as u64;
        assert_eq!(index["a"], (0, split));
        assert_eq!(index["b"], (split, text.len() as u64 - split));
    }

    #[test]
    fn test_create_step_index_lines_without_timestamp() {
        let text = "2023-11-20T14:48:00Z a\ncontinuation\n2023-11-20T14:48:02Z b\n";
        let steps = [
            step("a", "2023-11-20T14:48:00Z"),
            step("b", "2023-11-20T14:48:02Z"),
        ];
        let index = logs(text).create_step_index(&steps).unwrap();
        assert_eq!(
            index["a"].1,
            text.find("2023-11-20T14:48:02").unwrap() as u64
        );
    }

    #[test]
    fn test_create_step_index_real_failed_fmt_job() {
        let text = include_str!("testdata/failed_fmt_job.log");
        let steps = [
            step_full(
                "Set up job",
                "2026-07-10T15:02:57Z",
                "2026-07-10T15:02:59Z",
                "success",
            ),
            step_full(
                "Checkout",
                "2026-07-10T15:02:59Z",
                "2026-07-10T15:03:00Z",
                "success",
            ),
            step_full(
                "Install Rust stable",
                "2026-07-10T15:03:00Z",
                "2026-07-10T15:03:09Z",
                "success",
            ),
            step_full(
                "check formatting",
                "2026-07-10T15:03:09Z",
                "2026-07-10T15:03:09Z",
                "failure",
            ),
            step_full(
                "Cache Cargo dependencies",
                "2026-07-10T15:03:09Z",
                "2026-07-10T15:03:09Z",
                "skipped",
            ),
            step_full(
                "Post Checkout",
                "2026-07-10T15:03:09Z",
                "2026-07-10T15:03:09Z",
                "success",
            ),
            step_full(
                "Complete job",
                "2026-07-10T15:03:09Z",
                "2026-07-10T15:03:09Z",
                "success",
            ),
        ];
        let index = logs(text).create_step_index(&steps).unwrap();
        let slice = |name: &str| {
            let (start, len) = index[name];
            &text[start as usize..(start + len) as usize]
        };

        assert!(slice("Set up job").contains("Runner Image Provisioner"));
        assert!(slice("Checkout").contains("Run actions/checkout@v6"));
        assert!(slice("Install Rust stable").contains("Run dtolnay/rust-toolchain@stable"));
        assert!(
            slice("check formatting")
                .starts_with("2026-07-10T15:03:09.0267867Z ##[group]Run cargo fmt")
        );
        assert!(slice("check formatting").contains("expected `{`, found `}`"));
        assert_eq!(index["Cache Cargo dependencies"], (0, 0));
        assert!(slice("Post Checkout").contains("Post job cleanup."));
        assert!(slice("Complete job").contains("Cleaning up orphan processes"));
        assert!(!slice("Complete job").contains("Post job cleanup."));
    }

    fn step_full(name: &str, started_at: &str, completed_at: &str, conclusion: &str) -> Step {
        Step {
            conclusion: Some(conclusion.to_string()),
            completed_at: completed_at.to_string(),
            ..step(name, started_at)
        }
    }
}
