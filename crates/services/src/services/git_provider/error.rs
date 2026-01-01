//! Provider error types

use thiserror::Error;

/// Errors from git provider operations
#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("Provider CLI not installed: {cli_name}")]
    NotInstalled { cli_name: String },

    #[error("Provider authentication failed: {0}")]
    NotAuthenticated(String),

    #[error("Feature not supported: {feature}")]
    NotSupported { feature: String },

    #[error("API error ({status}): {message}")]
    ApiError { status: u16, message: String },

    #[error("Failed to parse: {0}")]
    ParseError(String),

    #[error("Command failed: {0}")]
    CommandFailed(String),

    #[error("Git error: {0}")]
    Git(String),

    #[error("Unknown provider for URL: {0}")]
    UnknownProvider(String),
}

impl ProviderError {
    /// Check if error is retryable
    pub fn should_retry(&self) -> bool {
        !matches!(
            self,
            ProviderError::NotInstalled { .. }
                | ProviderError::NotAuthenticated(_)
                | ProviderError::NotSupported { .. }
                | ProviderError::UnknownProvider(_)
        )
    }

    /// Check if error is auth-related
    pub fn is_auth_error(&self) -> bool {
        matches!(self, ProviderError::NotAuthenticated(_))
    }

    /// Check if error is install-related
    pub fn is_not_installed(&self) -> bool {
        matches!(self, ProviderError::NotInstalled { .. })
    }
}
