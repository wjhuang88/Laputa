//! Core persisted domain state owned by Laputa.
//!
//! The domain deliberately stores references to imported originals. It never takes
//! ownership of, rewrites, or embeds original media bytes in the project manifest.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const PROJECT_SCHEMA_VERSION: u32 = 1;

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

impl std::fmt::Display for AssetId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Asset {
    pub id: AssetId,
    pub original_path: PathBuf,
    pub display_name: String,
    pub media_kind: MediaKind,
    pub source_sha256: String,
    pub byte_len: u64,
    pub technical: AssetTechnicalMetadata,
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
    use super::ProjectManifest;

    #[test]
    fn new_project_uses_current_schema_and_starts_empty() {
        let project = ProjectManifest::new("Kyoto");

        assert_eq!(project.schema_version, super::PROJECT_SCHEMA_VERSION);
        assert_eq!(project.name, "Kyoto");
        assert!(project.assets.is_empty());
    }
}
