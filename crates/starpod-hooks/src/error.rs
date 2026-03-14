use thiserror::Error;

/// Errors that can occur during hook operations.
#[derive(Error, Debug)]
pub enum HookError {
    /// Invalid regex pattern in a hook matcher.
    #[error("Invalid hook matcher regex: {0}")]
    InvalidRegex(#[from] regex::Error),

    /// Hook callback returned an error.
    #[error("Hook callback failed: {0}")]
    CallbackFailed(String),

    /// Hook execution timed out.
    #[error("Hook timed out after {0}s")]
    Timeout(u64),

    /// Serialization/deserialization error.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, HookError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_invalid_regex() {
        let err = HookError::InvalidRegex(regex::Regex::new("[invalid").unwrap_err());
        let msg = err.to_string();
        assert!(msg.contains("Invalid hook matcher regex"), "got: {}", msg);
    }

    #[test]
    fn display_callback_failed() {
        let err = HookError::CallbackFailed("connection reset".into());
        assert_eq!(err.to_string(), "Hook callback failed: connection reset");
    }

    #[test]
    fn display_timeout() {
        let err = HookError::Timeout(30);
        assert_eq!(err.to_string(), "Hook timed out after 30s");
    }

    #[test]
    fn from_regex_error() {
        let regex_err = regex::Regex::new("[bad").unwrap_err();
        let hook_err: HookError = regex_err.into();
        assert!(matches!(hook_err, HookError::InvalidRegex(_)));
    }

    #[test]
    fn from_serde_error() {
        let serde_err = serde_json::from_str::<String>("not json").unwrap_err();
        let hook_err: HookError = serde_err.into();
        assert!(matches!(hook_err, HookError::Serialization(_)));
    }
}
