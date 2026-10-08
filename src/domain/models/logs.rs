use std::{
    fs::File,
    io::{self, BufRead, BufReader, Read, Seek, SeekFrom},
};

use tempfile::tempfile;

/// A byte range within [`Logs`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LogRange {
    pub start: u64,
    pub len: u64,
}

impl LogRange {
    pub const EMPTY: LogRange = LogRange { start: 0, len: 0 };

    pub fn new(start: u64, len: u64) -> Self {
        Self { start, len }
    }

    pub fn end(&self) -> u64 {
        self.start + self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

/// The raw log of a job, as delivered by GitHub.
///
/// Logs can be large, so they are kept in a temporary file (deleted when the
/// value is dropped) and only read on demand.
#[derive(Debug)]
pub struct Logs {
    file: File,
    byte_len: u64,
}

/// A line of the log with its byte offset, newline included.
pub struct RawLine {
    pub offset: u64,
    pub bytes: Vec<u8>,
}

impl Logs {
    /// Streams `reader` into a temporary file.
    pub fn from_reader(mut reader: impl Read) -> io::Result<Self> {
        let mut file = tempfile()?;
        let byte_len = io::copy(&mut reader, &mut file)?;
        Ok(Self { file, byte_len })
    }

    #[cfg(test)]
    pub fn from_bytes(bytes: &[u8]) -> io::Result<Self> {
        Self::from_reader(bytes)
    }

    pub fn byte_len(&self) -> u64 {
        self.byte_len
    }

    /// Iterates over all lines without loading the whole log into memory.
    pub fn raw_lines(&self) -> io::Result<impl Iterator<Item = io::Result<RawLine>>> {
        let mut file = &self.file;
        file.seek(SeekFrom::Start(0))?;
        let mut reader = BufReader::new(file);
        let mut offset = 0;
        Ok(std::iter::from_fn(move || {
            let mut bytes = Vec::new();
            match reader.read_until(b'\n', &mut bytes) {
                Ok(0) => None,
                Ok(n) => {
                    let line = RawLine { offset, bytes };
                    offset += n as u64;
                    Some(Ok(line))
                }
                Err(e) => Some(Err(e)),
            }
        }))
    }

    /// The lines within `range`. A range reaching past the end is clamped.
    pub fn lines_in(&self, range: LogRange) -> io::Result<Vec<String>> {
        if range.is_empty() {
            return Ok(Vec::new());
        }
        let start = range.start.min(self.byte_len);
        let len = range.end().min(self.byte_len) - start;
        if len == 0 {
            return Ok(Vec::new());
        }
        let mut file = &self.file;
        file.seek(SeekFrom::Start(start))?;
        let mut buf = Vec::new();
        file.take(len).read_to_end(&mut buf)?;
        Ok(String::from_utf8_lossy(&buf)
            .lines()
            .map(str::to_owned)
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn logs(text: &str) -> Logs {
        Logs::from_bytes(text.as_bytes()).unwrap()
    }

    #[test]
    fn lines_in_returns_only_the_range() {
        let logs = logs("one\ntwo\nthree\n");
        assert_eq!(logs.lines_in(LogRange::new(4, 4)).unwrap(), vec!["two"]);
    }

    #[test]
    fn lines_in_clamps_to_the_end() {
        let logs = logs("one\ntwo\n");
        assert_eq!(logs.lines_in(LogRange::new(4, 100)).unwrap(), vec!["two"]);
        assert!(logs.lines_in(LogRange::new(100, 5)).unwrap().is_empty());
    }

    #[test]
    fn lines_in_empty_range() {
        assert!(logs("one\n").lines_in(LogRange::EMPTY).unwrap().is_empty());
    }

    #[test]
    fn lines_in_can_be_called_repeatedly_in_any_order() {
        let logs = logs("one\ntwo\nthree\n");
        assert_eq!(logs.lines_in(LogRange::new(8, 6)).unwrap(), vec!["three"]);
        assert_eq!(logs.lines_in(LogRange::new(0, 4)).unwrap(), vec!["one"]);
    }

    #[test]
    fn raw_lines_have_offsets_and_keep_newlines() {
        let logs = logs("ab\ncd");
        let lines: Vec<RawLine> = logs.raw_lines().unwrap().map(Result::unwrap).collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(
            (lines[0].offset, lines[0].bytes.as_slice()),
            (0, &b"ab\n"[..])
        );
        assert_eq!(
            (lines[1].offset, lines[1].bytes.as_slice()),
            (3, &b"cd"[..])
        );
        assert_eq!(logs.byte_len(), 5);
    }
}
