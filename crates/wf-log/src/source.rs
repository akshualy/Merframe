use std::collections::{HashSet, VecDeque};
use std::path::Path;
use std::time::Duration;

use futures_core::Stream;
use tokio::sync::mpsc;
use tokio_stream::StreamExt;

#[cfg(windows)]
use crate::dbgwin::DbgWinListener;
use crate::error::Result;
use crate::line::LogLine;
use crate::tailer::tail;
#[cfg(not(windows))]
use crate::tap::LogTap;

pub const TAP_INTERVAL: Duration = Duration::from_millis(150);
const TAP_RETRY: Duration = Duration::from_secs(10);
const DEDUP_WINDOW: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    Tap,
    File,
}

impl Origin {
    pub fn label(self) -> &'static str {
        match self {
            Self::Tap => "tap",
            Self::File => "file",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Selection {
    #[default]
    Auto,
    File,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SourceLine {
    pub origin: Origin,
    pub line: LogLine,
}

#[derive(Debug, Default)]
pub struct Dedup {
    seen: HashSet<(u64, String)>,
    order: VecDeque<(u64, String)>,
}

impl Dedup {
    pub fn accept(&mut self, line: &LogLine) -> bool {
        let key = (line.time.to_bits(), line.message.clone());
        if !self.seen.insert(key.clone()) {
            return false;
        }
        self.order.push_back(key);
        if self.order.len() > DEDUP_WINDOW
            && let Some(oldest) = self.order.pop_front()
        {
            self.seen.remove(&oldest);
        }
        true
    }
}

#[cfg(windows)]
fn spawn_listener(mut listener: DbgWinListener) -> mpsc::UnboundedReceiver<Result<LogLine>> {
    let (sender, receiver) = mpsc::unbounded_channel();
    std::thread::spawn(move || {
        while !sender.is_closed() && !listener.game_exited() {
            if let Some(line) = listener.next_line()
                && sender.send(Ok(line)).is_err()
            {
                return;
            }
        }
    });
    receiver
}

#[cfg(windows)]
fn spawn_tap(_: &Path, _: Duration) -> Result<mpsc::UnboundedReceiver<Result<LogLine>>> {
    let game = wf_mem::find_process(wf_mem::GAME_PROCESS)?;
    Ok(spawn_listener(DbgWinListener::open(game)?))
}

#[cfg(not(windows))]
fn spawn_tap(
    log_file: &Path,
    interval: Duration,
) -> Result<mpsc::UnboundedReceiver<Result<LogLine>>> {
    Ok(spawn_memory_tap(LogTap::attach(log_file)?, interval))
}

#[cfg(not(windows))]
fn spawn_memory_tap(
    mut tap: LogTap,
    interval: Duration,
) -> mpsc::UnboundedReceiver<Result<LogLine>> {
    let (sender, receiver) = mpsc::unbounded_channel();
    std::thread::spawn(move || {
        while !sender.is_closed() {
            match tap.poll() {
                Ok(lines) => {
                    for line in lines {
                        if sender.send(Ok(line)).is_err() {
                            return;
                        }
                    }
                }
                Err(error) => {
                    let _ = sender.send(Err(error));
                    return;
                }
            }
            std::thread::sleep(interval);
        }
    });
    receiver
}

enum Received {
    Tap(Option<Result<LogLine>>),
    File(Option<Result<LogLine>>),
    RetryTap,
}

pub fn lines(
    path: &Path,
    selection: Selection,
) -> Result<impl Stream<Item = Result<SourceLine>> + use<>> {
    let file = tail(path, false)?;
    let tap = match selection {
        Selection::File => None,
        Selection::Auto => spawn_tap(path, TAP_INTERVAL).ok(),
    };

    let log_file = path.to_path_buf();

    Ok(async_stream::stream! {
        let mut tap = tap;
        let mut dedup = Dedup::default();
        let mut retry_at = tokio::time::Instant::now() + TAP_RETRY;
        tokio::pin!(file);
        loop {
            let received = match tap.as_mut() {
                Some(receiver) => tokio::select! {
                    line = receiver.recv() => Received::Tap(line),
                    line = file.next() => Received::File(line),
                },
                None if selection == Selection::Auto => tokio::select! {
                    line = file.next() => Received::File(line),
                    () = tokio::time::sleep_until(retry_at) => Received::RetryTap,
                },
                None => Received::File(file.next().await),
            };
            match received {
                Received::Tap(None) => tap = None,
                Received::RetryTap => {
                    retry_at = tokio::time::Instant::now() + TAP_RETRY;
                    tap = spawn_tap(&log_file, TAP_INTERVAL).ok();
                }
                Received::File(None) => return,
                Received::Tap(Some(Ok(line))) => {
                    if dedup.accept(&line) {
                        yield Ok(SourceLine { origin: Origin::Tap, line });
                    }
                }
                Received::File(Some(Ok(line))) => {
                    if dedup.accept(&line) {
                        yield Ok(SourceLine { origin: Origin::File, line });
                    }
                }
                Received::Tap(Some(Err(error))) | Received::File(Some(Err(error))) => {
                    yield Err(error);
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::line::{Channel, Level, LogLine};

    fn line(time: f64, message: &str) -> LogLine {
        LogLine {
            time,
            channel: Channel::Sys,
            level: Level::Info,
            message: message.to_owned(),
        }
    }

    #[test]
    fn duplicate_line() {
        let mut dedup = Dedup::default();
        assert!(dedup.accept(&line(1.0, "Logged in TestTenno")));
        assert!(!dedup.accept(&line(1.0, "Logged in TestTenno")));
    }

    #[test]
    fn timestamp_is_part_of_key() {
        let mut dedup = Dedup::default();
        assert!(dedup.accept(&line(1.0, "NAT bound for client")));
        assert!(dedup.accept(&line(2.0, "NAT bound for client")));
        assert!(!dedup.accept(&line(2.0, "NAT bound for client")));
    }

    #[test]
    fn repeated_message_new_time() {
        let mut dedup = Dedup::default();
        for step in 0..8 {
            assert!(dedup.accept(&line(f64::from(step) * 0.25, "OnAgentCreated")));
        }
    }

    #[test]
    fn window_forgets_oldest_lines() {
        let mut dedup = Dedup::default();
        let first = line(0.0, "first");
        assert!(dedup.accept(&first));
        for step in 1..=DEDUP_WINDOW {
            let time = f64::from(u32::try_from(step).unwrap());
            assert!(dedup.accept(&line(time, "filler")));
        }
        assert!(dedup.accept(&first));
    }

    #[test]
    fn default_selection() {
        assert_eq!(Selection::default(), Selection::Auto);
        assert_eq!(Origin::Tap.label(), "tap");
        assert_eq!(Origin::File.label(), "file");
    }
}
