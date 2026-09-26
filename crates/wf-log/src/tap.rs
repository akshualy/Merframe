use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use wf_mem::MemoryReader;
use wf_scan::{LogBuffer, find_log_buffer, is_line_head, read_pending};

use crate::error::Result;
use crate::line::{LogLine, drain_lines, parse_lines};

const FLUSH_SPAN: u64 = 0x1_0000;

fn unterminated_tail(path: &Path) -> Result<Vec<u8>> {
    let mut file = std::fs::File::open(path)?;
    let len = file.metadata()?.len();
    file.seek(SeekFrom::Start(len.saturating_sub(FLUSH_SPAN)))?;
    let mut tail = Vec::new();
    file.read_to_end(&mut tail)?;
    let head = memchr::memrchr(b'\n', &tail).map_or(0, |end| end + 1);
    Ok(tail.split_off(head))
}

#[derive(Debug, Default)]
pub struct BufferCursor {
    previous: Vec<u8>,
    partial: Vec<u8>,
}

impl BufferCursor {
    pub fn advance(
        &mut self,
        pending: &[u8],
        flushed_head: impl FnOnce() -> Result<Vec<u8>>,
    ) -> Result<Vec<String>> {
        let fresh = if pending.starts_with(&self.previous) {
            &pending[self.previous.len()..]
        } else {
            self.partial = if is_line_head(pending) {
                Vec::new()
            } else {
                flushed_head()?
            };
            pending
        };
        self.partial.extend_from_slice(fresh);
        self.previous.clear();
        self.previous.extend_from_slice(pending);
        Ok(drain_lines(&mut self.partial))
    }

    pub fn restart(&mut self) {
        self.previous.clear();
        self.partial.clear();
    }
}

pub struct LogTap {
    reader: Box<dyn MemoryReader>,
    log_file: PathBuf,
    buffer: Option<LogBuffer>,
    cursor: BufferCursor,
}

impl LogTap {
    pub fn attach(log_file: &Path) -> Result<Self> {
        Ok(Self::new(wf_mem::open_game()?, log_file))
    }

    pub fn new(reader: Box<dyn MemoryReader>, log_file: &Path) -> Self {
        Self {
            reader,
            log_file: log_file.to_path_buf(),
            buffer: None,
            cursor: BufferCursor::default(),
        }
    }

    pub fn buffer(&self) -> Option<LogBuffer> {
        self.buffer
    }

    pub fn poll(&mut self) -> Result<Vec<LogLine>> {
        let Some(buffer) = self.located()? else {
            return Ok(Vec::new());
        };
        let Some(pending) = read_pending(self.reader.as_ref(), buffer)? else {
            self.buffer = None;
            self.cursor.restart();
            return Ok(Vec::new());
        };
        let lines = self
            .cursor
            .advance(&pending, || unterminated_tail(&self.log_file))?;
        Ok(parse_lines(lines.iter().map(String::as_str)))
    }

    fn located(&mut self) -> Result<Option<LogBuffer>> {
        if let Some(buffer) = self.buffer {
            return Ok(Some(buffer));
        }
        let found = find_log_buffer(self.reader.as_ref())?;
        if found.is_some() {
            self.cursor.restart();
            self.buffer = found;
        }
        Ok(found)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn advance(cursor: &mut BufferCursor, pending: &[u8]) -> Vec<String> {
        cursor.advance(pending, || Ok(Vec::new())).unwrap()
    }

    #[test]
    fn unterminated_tail_after_last_newline() {
        let dir = std::env::temp_dir().join(format!("wf-log-tap-tail-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("EE.log");
        std::fs::write(&path, b"1.000 Sys [Info]: whole\r\n2.000 Sys [Info]: spl").unwrap();
        assert_eq!(unterminated_tail(&path).unwrap(), b"2.000 Sys [Info]: spl");
        std::fs::write(&path, b"1.000 Sys [Info]: whole\r\n").unwrap();
        assert!(unterminated_tail(&path).unwrap().is_empty());
    }

    #[test]
    fn first_poll() {
        let mut cursor = BufferCursor::default();
        assert_eq!(
            advance(
                &mut cursor,
                b"1.000 Sys [Info]: one\r\n2.000 Sys [Info]: two\r\n"
            ),
            vec![
                "1.000 Sys [Info]: one".to_owned(),
                "2.000 Sys [Info]: two".to_owned(),
            ]
        );
    }

    #[test]
    fn growing_buffer() {
        let mut cursor = BufferCursor::default();
        assert_eq!(advance(&mut cursor, b"1.000 Sys [Info]: one\r\n").len(), 1);
        assert_eq!(
            advance(
                &mut cursor,
                b"1.000 Sys [Info]: one\r\n2.000 Sys [Info]: two\r\n"
            ),
            vec![String::from("2.000 Sys [Info]: two")]
        );
        assert!(
            advance(
                &mut cursor,
                b"1.000 Sys [Info]: one\r\n2.000 Sys [Info]: two\r\n"
            )
            .is_empty()
        );
    }

    #[test]
    fn line_in_pieces() {
        let mut cursor = BufferCursor::default();
        assert!(advance(&mut cursor, b"3.000 Sys [Info]: par").is_empty());
        assert!(advance(&mut cursor, b"3.000 Sys [Info]: partial").is_empty());
        assert_eq!(
            advance(&mut cursor, b"3.000 Sys [Info]: partial line\r\n"),
            vec!["3.000 Sys [Info]: partial line".to_owned()]
        );
    }

    #[test]
    fn flush_then_new_line() {
        let mut cursor = BufferCursor::default();
        assert_eq!(advance(&mut cursor, b"1.000 Sys [Info]: one\r\n").len(), 1);
        assert!(advance(&mut cursor, b"").is_empty());
        assert_eq!(
            advance(&mut cursor, b"2.000 Sys [Info]: two\r\n"),
            vec![String::from("2.000 Sys [Info]: two")]
        );
    }

    #[test]
    fn fragment_cut_by_flush() {
        let mut cursor = BufferCursor::default();
        assert!(
            advance(
                &mut cursor,
                b"51046.900 Sys [Info]: ResourceLoader 0x1 (/Lotus/Levels/x) Fou"
            )
            .is_empty()
        );
        assert_eq!(
            advance(
                &mut cursor,
                b"51047.426 Script [Info]: ThemedDetailedPurchaseDialog.lua: DBG: HudVis 0\r\n"
            ),
            vec![
                "51047.426 Script [Info]: ThemedDetailedPurchaseDialog.lua: DBG: HudVis 0"
                    .to_owned()
            ]
        );
    }

    #[test]
    fn line_split_across_flush() {
        let mut cursor = BufferCursor::default();
        assert_eq!(
            advance(&mut cursor, b"3.000 Sys [Info]: before\r\n").len(),
            1
        );
        let lines = cursor
            .advance(b"er the flush\r\n5.000 Sys [Info]: next\r\n", || {
                Ok(b"4.000 Sys [Info]: split ov".to_vec())
            })
            .unwrap();
        assert_eq!(
            lines,
            vec![
                "4.000 Sys [Info]: split over the flush".to_owned(),
                "5.000 Sys [Info]: next".to_owned(),
            ]
        );
    }

    #[test]
    fn shrinking_buffer_restarts() {
        let mut cursor = BufferCursor::default();
        assert_eq!(
            advance(&mut cursor, b"1.000 Sys [Info]: a long first line\r\n").len(),
            1
        );
        assert_eq!(
            advance(&mut cursor, b"2.000 Sys [Info]: short\r\n"),
            vec![String::from("2.000 Sys [Info]: short")]
        );
    }

    #[test]
    fn restart_drops_partial() {
        let mut cursor = BufferCursor::default();
        assert!(advance(&mut cursor, b"6.000 Sys [Info]: incomplete").is_empty());
        cursor.restart();
        assert_eq!(
            advance(&mut cursor, b"7.000 Sys [Info]: fresh\r\n"),
            vec![String::from("7.000 Sys [Info]: fresh")]
        );
    }

    #[test]
    fn bare_newline() {
        let mut cursor = BufferCursor::default();
        assert_eq!(
            advance(&mut cursor, b"8.000 Sys [Info]: unix\n"),
            vec!["8.000 Sys [Info]: unix".to_owned()]
        );
    }
}
