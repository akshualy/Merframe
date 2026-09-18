use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct CommandError {
    pub message: String,
    pub code: Option<String>,
}

impl CommandError {
    pub fn starting() -> Self {
        Self {
            message: "Merframe is still starting".to_owned(),
            code: Some("starting".to_owned()),
        }
    }
}

impl From<anyhow::Error> for CommandError {
    fn from(error: anyhow::Error) -> Self {
        Self::from(format!("{error:#}"))
    }
}

impl From<String> for CommandError {
    fn from(message: String) -> Self {
        Self {
            message,
            code: None,
        }
    }
}

impl From<&str> for CommandError {
    fn from(message: &str) -> Self {
        Self::from(message.to_owned())
    }
}

pub(crate) fn cause_chain(error: &(dyn std::error::Error + 'static)) -> String {
    let mut message = error.to_string();
    let mut cause = error.source();
    while let Some(inner) = cause {
        let text = inner.to_string();
        if !message.contains(&text) {
            message.push_str(": ");
            message.push_str(&text);
        }
        cause = inner.source();
    }
    message
}

impl From<wf_core::CoreError> for CommandError {
    fn from(error: wf_core::CoreError) -> Self {
        Self::from(cause_chain(&error))
    }
}

impl From<wf_market::MarketError> for CommandError {
    fn from(error: wf_market::MarketError) -> Self {
        Self::from(cause_chain(&error))
    }
}

impl From<tauri::Error> for CommandError {
    fn from(error: tauri::Error) -> Self {
        Self::from(error.to_string())
    }
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

pub type CommandResult<T> = std::result::Result<T, CommandError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starting_code() {
        let error = CommandError::starting();
        let json = serde_json::to_value(&error).unwrap();
        assert_eq!(json["code"], "starting");
        assert_eq!(json["message"], "Merframe is still starting");
    }

    #[test]
    fn anyhow_message_no_code() {
        let error = CommandError::from(anyhow::anyhow!("boom"));
        assert_eq!(error.message, "boom");
        assert!(error.code.is_none());
        assert_eq!(error.to_string(), "boom");
    }

    #[test]
    fn market_error_message() {
        let error = CommandError::from(wf_market::MarketError::Unauthorized);
        assert_eq!(error.message, "Unauthorized");
        assert!(error.code.is_none());
    }

    #[test]
    fn core_error_message() {
        let error = CommandError::from(wf_core::CoreError::SchemaVersion(7));
        assert_eq!(
            error.message,
            "Store schema version 7 is newer than this build understands"
        );
    }

    #[derive(Debug)]
    struct Outer(std::io::Error);

    impl std::fmt::Display for Outer {
        fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("Reading the cache failed")
        }
    }

    impl std::error::Error for Outer {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            Some(&self.0)
        }
    }

    #[test]
    fn hidden_source_appended() {
        let inner = std::io::Error::other("disk full");
        assert_eq!(
            cause_chain(&Outer(inner)),
            "Reading the cache failed: disk full"
        );
    }

    #[test]
    fn displayed_source_not_repeated() {
        let error = wf_core::CoreError::Io {
            path: std::path::PathBuf::from("/data/merframe.sqlite"),
            source: std::io::Error::other("disk full"),
        };
        assert_eq!(
            cause_chain(&error),
            "Writing /data/merframe.sqlite failed: disk full"
        );
    }
}
