use std::io::SeekFrom;
use std::path::{Path, PathBuf};

use futures_core::Stream;
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio::sync::mpsc;

use crate::error::Result;
use crate::line::{LogLine, drain_lines, parse_lines};

struct Cursor {
    path: PathBuf,
    offset: u64,
    buffer: Vec<u8>,
    rewind_on_truncation: bool,
}

impl Cursor {
    fn new(path: PathBuf, offset: u64) -> Self {
        Self {
            path,
            offset,
            buffer: Vec::new(),
            rewind_on_truncation: false,
        }
    }

    async fn read_into_buffer(&mut self) -> Result<()> {
        let mut file = tokio::fs::File::open(&self.path).await?;
        let len = file.metadata().await?.len();
        if len < self.offset {
            self.offset = if self.rewind_on_truncation { 0 } else { len };
            self.buffer.clear();
        }
        self.rewind_on_truncation = true;
        if len > self.offset {
            file.seek(SeekFrom::Start(self.offset)).await?;
            self.offset += file.read_to_end(&mut self.buffer).await? as u64;
        }
        Ok(())
    }

    async fn poll_once(&mut self) -> Vec<Result<LogLine>> {
        if let Err(error) = self.read_into_buffer().await {
            return vec![Err(error)];
        }
        let lines = drain_lines(&mut self.buffer);
        parse_lines(lines.iter().map(String::as_str))
            .into_iter()
            .map(Ok)
            .collect()
    }
}

pub fn tail(path: &Path, from_start: bool) -> Result<impl Stream<Item = Result<LogLine>> + use<>> {
    let path = path.to_path_buf();
    let len = std::fs::metadata(&path)?.len();
    let parent = match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
        _ => PathBuf::from("."),
    };
    let (tx, mut rx) = mpsc::channel::<()>(1);
    let watch_target = path.clone();
    let mut watcher: RecommendedWatcher =
        notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
            if let Ok(event) = event
                && matches!(event.kind, EventKind::Create(_) | EventKind::Modify(_))
                && event.paths.iter().any(|changed| changed == &watch_target)
            {
                let _ = tx.try_send(());
            }
        })?;
    watcher.watch(&parent, RecursiveMode::NonRecursive)?;

    Ok(async_stream::stream! {
        let _watcher = watcher;
        let mut cursor = Cursor::new(path, if from_start { 0 } else { len });
        for item in cursor.poll_once().await {
            yield item;
        }
        loop {
            #[cfg(windows)]
            let woken = tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
                .await
                .map_or(true, |s| s.is_some());
            #[cfg(not(windows))]
            let woken = rx.recv().await.is_some();
            if !woken {
                return;
            }
            for item in cursor.poll_once().await {
                yield item;
            }
        }
    })
}

pub fn read_all(path: &Path) -> Result<Vec<LogLine>> {
    let content = std::fs::read(path)?;
    let text = String::from_utf8_lossy(&content);
    Ok(parse_lines(text.lines()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::time::Duration;
    use tokio_stream::StreamExt;

    #[tokio::test]
    async fn append_and_truncate() {
        let dir = std::env::temp_dir().join(format!("wf-log-tailer-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("EE.log");
        std::fs::write(&path, b"").unwrap();

        let stream = tail(&path, true).unwrap();
        tokio::pin!(stream);

        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        file.write_all(b"1.0 Sys [Info]: first\n").unwrap();
        file.flush().unwrap();

        let first = tokio::time::timeout(Duration::from_secs(5), stream.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(first.message, "first");

        file.write_all(b"2.0 Sys [Info]: sec").unwrap();
        file.flush().unwrap();
        tokio::time::sleep(Duration::from_millis(200)).await;
        file.write_all(b"ond\n").unwrap();
        file.flush().unwrap();

        let second = tokio::time::timeout(Duration::from_secs(5), stream.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(second.message, "second");
        drop(file);

        std::fs::write(&path, b"").unwrap();
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        file.write_all(b"3.0 Sys [Info]: third\n").unwrap();
        file.flush().unwrap();

        let third = tokio::time::timeout(Duration::from_secs(5), stream.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(third.message, "third");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn split_multi_byte_char() {
        let dir = std::env::temp_dir().join(format!("wf-log-tailer-utf8-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("EE.log");
        std::fs::write(&path, b"").unwrap();

        let stream = tail(&path, true).unwrap();
        tokio::pin!(stream);

        let message = "Köbinn West".as_bytes();
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        file.write_all(b"1.0 Sys [Info]: ").unwrap();
        file.write_all(&message[..2]).unwrap();
        file.flush().unwrap();
        tokio::time::sleep(Duration::from_millis(200)).await;
        file.write_all(&message[2..]).unwrap();
        file.write_all(b"\n").unwrap();
        file.flush().unwrap();

        let line = tokio::time::timeout(Duration::from_secs(5), stream.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(line.message, "Köbinn West");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn from_end_skips_existing() {
        let dir = std::env::temp_dir().join(format!("wf-log-tailer-end-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("EE.log");
        std::fs::write(
            &path,
            b"1.0 Sys [Info]: historical one\n2.0 Sys [Info]: historical two\n",
        )
        .unwrap();

        let stream = tail(&path, false).unwrap();
        tokio::pin!(stream);

        assert!(
            tokio::time::timeout(Duration::from_millis(500), stream.next())
                .await
                .is_err()
        );

        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        file.write_all(b"3.0 Sys [Info]: appended\n").unwrap();
        file.flush().unwrap();
        drop(file);

        let appended = tokio::time::timeout(Duration::from_secs(5), stream.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(appended.message, "appended");

        assert!(
            tokio::time::timeout(Duration::from_millis(500), stream.next())
                .await
                .is_err()
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn append_via_open_handle() {
        let dir = std::env::temp_dir().join(format!("wf-log-tailer-open-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("EE.log");
        std::fs::write(&path, b"1.0 Sys [Info]: historical\n").unwrap();

        let stream = tail(&path, false).unwrap();
        tokio::pin!(stream);

        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        file.write_all(b"2.0 Sys [Info]: while the game holds the file\n")
            .unwrap();
        file.flush().unwrap();

        let appended = tokio::time::timeout(Duration::from_secs(5), stream.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(appended.message, "while the game holds the file");
        drop(file);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn read_all_fixture() {
        let path = std::path::Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/EE.log"
        ));
        let lines = read_all(path).unwrap();
        assert!(!lines.is_empty());
    }
}
