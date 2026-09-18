#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    Sys,
    Script,
    Net,
    Game,
    Input,
    Ai,
    Other,
}

impl Channel {
    fn parse(token: &str) -> Self {
        match token {
            "Sys" => Self::Sys,
            "Script" => Self::Script,
            "Net" => Self::Net,
            "Game" => Self::Game,
            "Input" => Self::Input,
            "AI" | "Ai" => Self::Ai,
            _ => Self::Other,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Info,
    Warning,
    Error,
}

impl Level {
    fn parse(token: &str) -> Option<Self> {
        match token {
            "Info" => Some(Self::Info),
            "Warning" => Some(Self::Warning),
            "Error" => Some(Self::Error),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LogLine {
    pub time: f64,
    pub channel: Channel,
    pub level: Level,
    pub message: String,
}

pub(crate) fn drain_lines(buffer: &mut Vec<u8>) -> Vec<String> {
    let Some(end) = memchr::memrchr(b'\n', buffer) else {
        return Vec::new();
    };
    let complete = String::from_utf8_lossy(&buffer[..=end]).into_owned();
    buffer.drain(..=end);
    complete.lines().map(str::to_owned).collect()
}

pub fn parse_line(line: &str) -> Option<LogLine> {
    let line = line.trim_end_matches(['\n', '\r']);
    let (time_token, rest) = line.split_once(' ')?;
    let time = time_token.parse::<f64>().ok()?;
    let (channel_token, after_channel) = rest.split_once(" [")?;
    let (level_token, message) = after_channel.split_once("]: ")?;
    let level = Level::parse(level_token)?;
    Some(LogLine {
        time,
        channel: Channel::parse(channel_token),
        level,
        message: message.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn well_formed_line() {
        let line = parse_line("7501.690 Sys [Info]: VoidProjections: OpenVoidProjectionRewardScreen - PostMigration: 0").unwrap();
        assert!((line.time - 7501.690).abs() < f64::EPSILON);
        assert_eq!(line.channel, Channel::Sys);
        assert_eq!(line.level, Level::Info);
        assert_eq!(
            line.message,
            "VoidProjections: OpenVoidProjectionRewardScreen - PostMigration: 0"
        );
    }

    #[test]
    fn ai_channel() {
        let line = parse_line("1.0 AI [Info]: something").unwrap();
        assert_eq!(line.channel, Channel::Ai);
    }

    #[test]
    fn unknown_channel_is_other() {
        let line = parse_line("1.0 Gfx [Info]: something").unwrap();
        assert_eq!(line.channel, Channel::Other);
    }

    #[test]
    fn unknown_level() {
        assert!(parse_line("0.043 Sys [Diag]: Process Command-line: -foo").is_none());
    }

    #[test]
    fn no_timestamp() {
        assert!(parse_line("UploadChallengeProgress: uploading...").is_none());
    }

    #[test]
    fn trailing_carriage_return() {
        let line = parse_line("1.0 Sys [Info]: hello\r").unwrap();
        assert_eq!(line.message, "hello");
    }

    #[test]
    fn drain_keeps_partial_tail() {
        let mut buffer = b"one\r\ntwo\nthr".to_vec();
        assert_eq!(drain_lines(&mut buffer), ["one", "two"]);
        assert_eq!(buffer, b"thr");
        assert!(drain_lines(&mut buffer).is_empty());
        buffer.extend_from_slice("ee \u{e071}900\n".as_bytes());
        assert_eq!(drain_lines(&mut buffer), ["three \u{e071}900"]);
        assert!(buffer.is_empty());
    }
}
