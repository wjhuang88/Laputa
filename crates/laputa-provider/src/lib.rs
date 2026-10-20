//! Functional provider contracts for Laputa.
//!
//! Stable provider traits are capability-oriented. Vendor SDK/request/response types
//! stay inside adapters and must not leak into Laputa domain or tool contracts.

use std::fmt;

use async_trait::async_trait;
use laputa_domain::{Asset, AssetAnalysis};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MediaUnderstandingErrorKind {
    Unavailable,
    Rejected,
    MalformedResponse,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MediaUnderstandingError {
    kind: MediaUnderstandingErrorKind,
    message: String,
}

impl MediaUnderstandingError {
    #[must_use]
    pub fn new(kind: MediaUnderstandingErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    #[must_use]
    pub fn kind(&self) -> MediaUnderstandingErrorKind {
        self.kind
    }

    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for MediaUnderstandingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for MediaUnderstandingError {}

#[async_trait]
pub trait MediaUnderstandingProvider: Send + Sync {
    /// Produce a provider-neutral candidate analysis for an imported source Asset.
    ///
    /// Implementations may upload/read the referenced original, but they must not mutate it.
    /// Callers remain responsible for validating the returned canonical analysis against the
    /// current Asset before persistence.
    async fn analyze(&self, asset: &Asset) -> Result<AssetAnalysis, MediaUnderstandingError>;
}

#[cfg(test)]
mod tests {
    use super::{MediaUnderstandingError, MediaUnderstandingErrorKind};

    #[test]
    fn provider_error_exposes_stable_kind_without_vendor_schema() {
        let error = MediaUnderstandingError::new(
            MediaUnderstandingErrorKind::MalformedResponse,
            "fixture payload omitted summary",
        );

        assert_eq!(error.kind(), MediaUnderstandingErrorKind::MalformedResponse);
        assert_eq!(error.message(), "fixture payload omitted summary");
        assert_eq!(error.to_string(), "fixture payload omitted summary");
    }
}
