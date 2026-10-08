use std::{cmp::Ordering, collections::HashMap, io};

use chrono::{DateTime, NaiveDateTime, Timelike, Utc};

use crate::domain::{LogRange, Logs, Step, models::logs::RawLine};

const POST_MARKER: &[u8] = b"Post job cleanup.";
const COMPLETE_MARKER: &[u8] = b"Cleaning up orphan processes";
const GROUP_MARKER: &[u8] = b"##[group]";
const TOP_LEVEL_GROUP_MARKER: &[u8] = b"##[group]Run ";

/// Which part of a job's [`Logs`] belongs to which step.
#[derive(Debug, Clone, Default)]
pub struct StepLogIndex {
    ranges: HashMap<u64, LogRange>,
}

impl StepLogIndex {
    /// The log range of `step`; empty if it has no output or is unknown.
    pub fn range_of(&self, step: &Step) -> LogRange {
        self.ranges
            .get(&step.number)
            .copied()
            .unwrap_or(LogRange::EMPTY)
    }
}

/// Domain service that works out which log lines belong to which step.
///
/// GitHub only reports second-resolution start times, and several steps
/// regularly start within the same second, so steps are located in layers:
/// - skipped and never started steps produce no output and get an empty range,
/// - `Post ...` and `Complete job` steps are found by the line they always
///   begin with,
/// - all other steps are found by comparing the timestamp of each log line
///   with the step's start. If that is ambiguous (several steps start in the
///   same second, or the previous step ends in it), the `##[group]` header
///   lines of that second decide.
///
/// Every step is always present in the index; steps that could not be located
/// get an empty range.
pub struct StepLogLocator;

impl StepLogLocator {
    /// Reads the log line by line; only small per-line metadata is kept.
    pub fn locate(logs: &Logs, steps: &[Step]) -> io::Result<StepLogIndex> {
        let total = logs.byte_len();
        let lines = scan_lines(logs)?;
        let mut starts: Vec<Option<u64>> = vec![None; steps.len()];

        // Post and Complete steps come after every other step
        let kinds: Vec<Kind> = steps.iter().map(Kind::of).collect();
        let mut post_lines = lines.iter().filter(|l| l.is_post);
        let complete_line = lines.iter().find(|l| l.is_complete);
        for (i, kind) in kinds.iter().enumerate() {
            match kind {
                Kind::Post => starts[i] = post_lines.next().map(|l| l.offset),
                Kind::Complete => starts[i] = complete_line.map(|l| l.offset),
                _ => {}
            }
        }
        let limit = lines
            .iter()
            .position(|l| l.is_post || l.is_complete)
            .unwrap_or(lines.len());

        // all other steps, by timestamp
        let regular: Vec<usize> = (0..steps.len())
            .filter(|&i| kinds[i] == Kind::Regular)
            .collect();
        let start_secs: Vec<NaiveDateTime> = regular
            .iter()
            .filter_map(|&i| steps[i].started_at.map(to_second))
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
                match ts.cmp(&start_secs[next]) {
                    Ordering::Less => break,
                    Ordering::Greater => {
                        starts[regular[next]] = Some(line.offset);
                        cur = next;
                    }
                    Ordering::Equal => {
                        // steps sharing this start second, in order
                        let tied = start_secs[next..].iter().take_while(|t| **t == ts).count();
                        let prev_ends_here =
                            steps[regular[cur]].completed_at.map(to_second) == Some(ts);
                        let prev_starts_here = start_secs[cur] == ts;
                        if tied == 1 && !prev_ends_here && !prev_starts_here {
                            starts[regular[next]] = Some(line.offset);
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
                            .filter(|&j| lines[j].is_top_level_group)
                            .collect();
                        if !top_level.is_empty() {
                            groups = top_level;
                        }
                        if groups.is_empty() {
                            starts[regular[next]] = Some(line.offset);
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
                            starts[regular[next + n]] = Some(lines[g].offset);
                            cur = next + n;
                        }
                        break;
                    }
                }
            }
            li += 1;
        }

        let mut ranges = HashMap::with_capacity(steps.len());
        for (i, step) in steps.iter().enumerate() {
            let range = match starts[i] {
                Some(start) => {
                    let end = starts[i + 1..]
                        .iter()
                        .flatten()
                        .next()
                        .map_or(total, |&e| e);
                    LogRange::new(start, end.saturating_sub(start))
                }
                None => LogRange::EMPTY,
            };
            ranges.insert(step.number, range);
        }
        Ok(StepLogIndex { ranges })
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
        if step.is_skipped() || step.started_at.is_none() {
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

/// Truncates to whole seconds, the resolution of GitHub's step times.
fn to_second(time: DateTime<Utc>) -> NaiveDateTime {
    time.naive_utc().with_nanosecond(0).unwrap_or_default()
}

struct LogLine {
    offset: u64,
    /// Timestamp prefix, truncated to seconds, if the line has one.
    ts: Option<NaiveDateTime>,
    is_group: bool,
    /// `##[group]Run ...`, the header of a top-level step.
    is_top_level_group: bool,
    is_post: bool,
    is_complete: bool,
}

fn scan_lines(logs: &Logs) -> io::Result<Vec<LogLine>> {
    logs.raw_lines()?
        .map(|raw| {
            let RawLine { offset, bytes } = raw?;
            let line = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(&bytes);
            let (ts, body) = match parse_timestamp(line) {
                Some(ts) => {
                    // skip the fractional seconds and the "Z "
                    let rest = &line[19..];
                    let end = rest
                        .iter()
                        .position(|&b| b == b' ')
                        .map_or(rest.len(), |p| p + 1);
                    (Some(ts), &rest[end..])
                }
                None => (None, line),
            };
            Ok(LogLine {
                offset,
                ts,
                is_group: body.starts_with(GROUP_MARKER),
                is_top_level_group: body.starts_with(TOP_LEVEL_GROUP_MARKER),
                is_post: body.starts_with(POST_MARKER),
                is_complete: body.starts_with(COMPLETE_MARKER),
            })
        })
        .collect()
}

/// Parses a leading `YYYY-MM-DDTHH:MM:SS` timestamp.
fn parse_timestamp(line: &[u8]) -> Option<NaiveDateTime> {
    let prefix = std::str::from_utf8(line.get(..19)?).ok()?;
    NaiveDateTime::parse_from_str(prefix, "%Y-%m-%dT%H:%M:%S").ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(s: &str) -> Option<DateTime<Utc>> {
        if s.is_empty() {
            return None;
        }
        Some(DateTime::parse_from_rfc3339(s).unwrap().into())
    }

    fn step(
        number: u64,
        name: &str,
        started_at: &str,
        completed_at: &str,
        conclusion: &str,
    ) -> Step {
        Step {
            name: name.to_string(),
            status: "completed".to_string(),
            conclusion: Some(conclusion.to_string()),
            number,
            started_at: at(started_at),
            completed_at: at(completed_at),
        }
    }

    fn ok_step(number: u64, name: &str, started_at: &str) -> Step {
        step(number, name, started_at, "", "success")
    }

    fn logs(text: &str) -> Logs {
        Logs::from_bytes(text.as_bytes()).unwrap()
    }

    fn slice<'a>(text: &'a str, index: &StepLogIndex, step: &Step) -> &'a str {
        let range = index.range_of(step);
        &text[range.start as usize..range.end() as usize]
    }

    #[test]
    fn no_steps() {
        let index = StepLogLocator::locate(&logs("whatever\n"), &[]).unwrap();
        assert!(index.ranges.is_empty());
    }

    #[test]
    fn empty_log() {
        let steps = [
            ok_step(1, "a", "2023-11-20T14:48:00Z"),
            ok_step(2, "b", "2023-11-20T14:48:01Z"),
        ];
        let index = StepLogLocator::locate(&logs(""), &steps).unwrap();
        assert!(index.range_of(&steps[0]).is_empty());
        assert!(index.range_of(&steps[1]).is_empty());
    }

    #[test]
    fn unknown_step_has_empty_range() {
        let index = StepLogLocator::locate(&logs("x\n"), &[]).unwrap();
        let other = ok_step(9, "other", "2023-11-20T14:48:00Z");
        assert_eq!(index.range_of(&other), LogRange::EMPTY);
    }

    #[test]
    fn exact_second_match() {
        let text = "2023-11-20T14:48:00Z step 1\n2023-11-20T14:48:01Z step 2\n";
        let steps = [
            ok_step(1, "Step 1", "2023-11-20T14:48:00Z"),
            ok_step(2, "Step 2", "2023-11-20T14:48:01Z"),
        ];
        let index = StepLogLocator::locate(&logs(text), &steps).unwrap();
        assert_eq!(
            slice(text, &index, &steps[0]),
            "2023-11-20T14:48:00Z step 1\n"
        );
        assert_eq!(
            slice(text, &index, &steps[1]),
            "2023-11-20T14:48:01Z step 2\n"
        );
    }

    #[test]
    fn fractional_seconds_in_log() {
        let text = "2023-11-20T14:48:00.123Z step 1\n2023-11-20T14:48:01.456Z step 2\n";
        let steps = [
            ok_step(1, "Step 1", "2023-11-20T14:48:00Z"),
            ok_step(2, "Step 2", "2023-11-20T14:48:01Z"),
        ];
        let index = StepLogLocator::locate(&logs(text), &steps).unwrap();
        assert_eq!(
            slice(text, &index, &steps[1]),
            "2023-11-20T14:48:01.456Z step 2\n"
        );
    }

    #[test]
    fn skipped_step_gets_no_output() {
        let text =
            "2023-11-20T14:48:00.1Z a\n2023-11-20T14:48:00.2Z more a\n2023-11-20T14:48:05.0Z c\n";
        let steps = [
            ok_step(1, "a", "2023-11-20T14:48:00Z"),
            step(2, "skipped", "", "", "skipped"),
            ok_step(3, "c", "2023-11-20T14:48:05Z"),
        ];
        let index = StepLogLocator::locate(&logs(text), &steps).unwrap();
        assert_eq!(
            slice(text, &index, &steps[0]),
            "2023-11-20T14:48:00.1Z a\n2023-11-20T14:48:00.2Z more a\n"
        );
        assert!(index.range_of(&steps[1]).is_empty());
        assert_eq!(slice(text, &index, &steps[2]), "2023-11-20T14:48:05.0Z c\n");
    }

    #[test]
    fn step_without_output() {
        let text = "2023-11-20T14:48:00Z a\n2023-11-20T14:48:09Z c\n";
        let steps = [
            ok_step(1, "a", "2023-11-20T14:48:00Z"),
            ok_step(2, "b", "2023-11-20T14:48:03Z"),
            ok_step(3, "c", "2023-11-20T14:48:09Z"),
        ];
        let index = StepLogLocator::locate(&logs(text), &steps).unwrap();
        assert!(index.range_of(&steps[1]).is_empty());
        assert_eq!(
            index.range_of(&steps[0]).end(),
            index.range_of(&steps[2]).start
        );
        assert_eq!(index.range_of(&steps[2]).end(), text.len() as u64);
    }

    #[test]
    fn same_second_uses_group_header() {
        let text = "2023-11-20T14:48:00Z ##[group]Run a\n2023-11-20T14:48:00Z out a\n\
                    2023-11-20T14:48:00Z ##[group]Run b\n2023-11-20T14:48:00Z out b\n";
        let steps = [
            ok_step(1, "a", "2023-11-20T14:48:00Z"),
            ok_step(2, "b", "2023-11-20T14:48:00Z"),
        ];
        let index = StepLogLocator::locate(&logs(text), &steps).unwrap();
        let split = text.find("2023-11-20T14:48:00Z ##[group]Run b").unwrap() as u64;
        assert_eq!(index.range_of(&steps[0]), LogRange::new(0, split));
        assert_eq!(
            index.range_of(&steps[1]),
            LogRange::new(split, text.len() as u64 - split)
        );
    }

    #[test]
    fn lines_without_timestamp_stay_with_the_step() {
        let text = "2023-11-20T14:48:00Z a\ncontinuation\n2023-11-20T14:48:02Z b\n";
        let steps = [
            ok_step(1, "a", "2023-11-20T14:48:00Z"),
            ok_step(2, "b", "2023-11-20T14:48:02Z"),
        ];
        let index = StepLogLocator::locate(&logs(text), &steps).unwrap();
        assert_eq!(
            slice(text, &index, &steps[0]),
            "2023-11-20T14:48:00Z a\ncontinuation\n"
        );
    }

    #[test]
    fn real_failed_fmt_job() {
        let text = include_str!("testdata/failed_fmt_job.log");
        let steps = [
            step(
                1,
                "Set up job",
                "2026-07-10T15:02:57Z",
                "2026-07-10T15:02:59Z",
                "success",
            ),
            step(
                2,
                "Checkout",
                "2026-07-10T15:02:59Z",
                "2026-07-10T15:03:00Z",
                "success",
            ),
            step(
                3,
                "Install Rust stable",
                "2026-07-10T15:03:00Z",
                "2026-07-10T15:03:09Z",
                "success",
            ),
            step(
                4,
                "check formatting",
                "2026-07-10T15:03:09Z",
                "2026-07-10T15:03:09Z",
                "failure",
            ),
            step(
                5,
                "Cache Cargo dependencies",
                "2026-07-10T15:03:09Z",
                "2026-07-10T15:03:09Z",
                "skipped",
            ),
            step(
                10,
                "Post Checkout",
                "2026-07-10T15:03:09Z",
                "2026-07-10T15:03:09Z",
                "success",
            ),
            step(
                11,
                "Complete job",
                "2026-07-10T15:03:09Z",
                "2026-07-10T15:03:09Z",
                "success",
            ),
        ];
        let index = StepLogLocator::locate(&logs(text), &steps).unwrap();
        let part = |i: usize| slice(text, &index, &steps[i]);

        assert!(part(0).contains("Runner Image Provisioner"));
        assert!(part(1).contains("Run actions/checkout@v6"));
        assert!(part(2).contains("Run dtolnay/rust-toolchain@stable"));
        assert!(part(3).starts_with("2026-07-10T15:03:09.0267867Z ##[group]Run cargo fmt"));
        assert!(part(3).contains("expected `{`, found `}`"));
        assert!(index.range_of(&steps[4]).is_empty());
        assert!(part(5).contains("Post job cleanup."));
        assert!(part(6).contains("Cleaning up orphan processes"));
        assert!(!part(6).contains("Post job cleanup."));
    }
}
