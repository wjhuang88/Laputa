//! Core persisted domain state owned by Laputa.
//!
//! The domain deliberately stores references to imported originals. It never takes
//! ownership of, rewrites, or embeds original media bytes in the project manifest.

use std::{fmt, path::PathBuf};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const PROJECT_SCHEMA_VERSION: u32 = 2;
pub const MIN_SUPPORTED_PROJECT_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProjectId(Uuid);

impl ProjectId {
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ProjectId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AssetId(Uuid);

impl AssetId {
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for AssetId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for AssetId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaKind {
    Image,
    Video,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AssetTechnicalMetadata {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub duration_ms: Option<u64>,
    pub video_codec: Option<String>,
    pub has_audio: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AnalysisTimeRange {
    pub start_ms: u64,
    pub end_ms: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AnalysisSegment {
    pub range: AnalysisTimeRange,
    pub summary: String,
    pub tags: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AssetAnalysis {
    pub summary: String,
    pub tags: Vec<String>,
    #[serde(default)]
    pub segments: Vec<AnalysisSegment>,
}

impl AssetAnalysis {
    pub fn validate_for(&self, asset: &Asset) -> Result<(), AnalysisValidationError> {
        ensure_text("analysis summary", &self.summary)?;
        ensure_tags("analysis tags", &self.tags)?;

        match asset.media_kind {
            MediaKind::Image => {
                if !self.segments.is_empty() {
                    return Err(AnalysisValidationError::new(
                        "image analysis must not contain temporal segments",
                    ));
                }
            }
            MediaKind::Video => {
                if self.segments.is_empty() {
                    return Err(AnalysisValidationError::new(
                        "video analysis must contain at least one temporal segment",
                    ));
                }

                let mut previous_end_ms = 0_u64;
                for (index, segment) in self.segments.iter().enumerate() {
                    ensure_text("segment summary", &segment.summary)?;
                    ensure_tags("segment tags", &segment.tags)?;

                    if segment.range.start_ms >= segment.range.end_ms {
                        return Err(AnalysisValidationError::new(format!(
                            "segment {index} has an empty or reversed time range"
                        )));
                    }
                    if index > 0 && segment.range.start_ms < previous_end_ms {
                        return Err(AnalysisValidationError::new(format!(
                            "segment {index} overlaps or is out of order"
                        )));
                    }
                    if let Some(duration_ms) = asset.technical.duration_ms
                        && segment.range.end_ms > duration_ms
                    {
                        return Err(AnalysisValidationError::new(format!(
                            "segment {index} ends after the asset duration"
                        )));
                    }
                    previous_end_ms = segment.range.end_ms;
                }
            }
        }

        Ok(())
    }
}

fn ensure_text(field: &str, value: &str) -> Result<(), AnalysisValidationError> {
    if value.trim().is_empty() {
        return Err(AnalysisValidationError::new(format!(
            "{field} must not be empty"
        )));
    }
    Ok(())
}

fn ensure_tags(field: &str, tags: &[String]) -> Result<(), AnalysisValidationError> {
    if tags.is_empty() {
        return Err(AnalysisValidationError::new(format!(
            "{field} must contain at least one tag"
        )));
    }
    if tags.iter().any(|tag| tag.trim().is_empty()) {
        return Err(AnalysisValidationError::new(format!(
            "{field} must not contain empty tags"
        )));
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnalysisValidationError {
    message: String,
}

impl AnalysisValidationError {
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for AnalysisValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for AnalysisValidationError {}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AssetAnalysisRecord {
    pub source_sha256: String,
    pub analysis: AssetAnalysis,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AssetAnalysisFailure {
    pub source_sha256: String,
    pub message: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum AssetAnalysisState {
    Ready(AssetAnalysisRecord),
    Failed(AssetAnalysisFailure),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Asset {
    pub id: AssetId,
    pub original_path: PathBuf,
    pub display_name: String,
    pub media_kind: MediaKind,
    pub source_sha256: String,
    pub byte_len: u64,
    pub technical: AssetTechnicalMetadata,
    #[serde(default)]
    pub analysis: Option<AssetAnalysisState>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProjectManifest {
    pub schema_version: u32,
    pub id: ProjectId,
    pub name: String,
    pub assets: Vec<Asset>,
}

impl ProjectManifest {
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            schema_version: PROJECT_SCHEMA_VERSION,
            id: ProjectId::new(),
            name: name.into(),
            assets: Vec::new(),
        }
    }

    #[must_use]
    pub fn asset(&self, id: AssetId) -> Option<&Asset> {
        self.assets.iter().find(|asset| asset.id == id)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{
        AnalysisSegment, AnalysisTimeRange, Asset, AssetAnalysis, AssetId,
        AssetTechnicalMetadata, MediaKind, ProjectManifest,
    };

    fn fixture_asset(kind: MediaKind, duration_ms: Option<u64>) -> Asset {
        Asset {
            id: AssetId::new(),
            original_path: PathBuf::from("fixture"),
            display_name: "fixture".to_owned(),
            media_kind: kind,
            source_sha256: "revision".to_owned(),
            byte_len: 1,
            technical: AssetTechnicalMetadata {
                duration_ms,
                ..AssetTechnicalMetadata::default()
            },
            analysis: None,
        }
    }

    #[test]
    fn new_project_uses_current_schema_and_starts_empty() {
        let project = ProjectManifest::new("Kyoto");

        assert_eq!(project.schema_version, super::PROJECT_SCHEMA_VERSION);
        assert_eq!(project.name, "Kyoto");
        assert!(project.assets.is_empty());
    }

    #[test]
    fn image_analysis_rejects_temporal_segments() {
        let asset = fixture_asset(MediaKind::Image, None);
        let analysis = AssetAnalysis {
            summary: "Temple portrait".to_owned(),
            tags: vec!["portrait".to_owned()],
            segments: vec![AnalysisSegment {
                range: AnalysisTimeRange {
                    start_ms: 0,
                    end_ms: 100,
                },
                summary: "not valid for an image".to_owned(),
                tags: vec!["invalid".to_owned()],
            }],
        };

        assert!(analysis.validate_for(&asset).is_err());
    }

    #[test]
    fn video_analysis_rejects_out_of_bounds_segment() {
        let asset = fixture_asset(MediaKind::Video, Some(1_000));
        let analysis = AssetAnalysis {
            summary: "Street walk".to_owned(),
            tags: vec!["street".to_owned()],
            segments: vec![AnalysisSegment {
                range: AnalysisTimeRange {
                    start_ms: 0,
                    end_ms: 1_001,
                },
                summary: "Walking through the street".to_owned(),
                tags: vec!["walking".to_owned()],
            }],
        };

        assert!(analysis.validate_for(&asset).is_err());
    }
}
