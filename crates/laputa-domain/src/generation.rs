use std::{
    collections::{HashMap, HashSet},
    fmt,
    path::{Component, Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{AssetId, MediaKind, ProjectManifest};

pub const GENERATION_SCHEMA_VERSION: u32 = 1;

macro_rules! uuid_id {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            #[must_use]
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(formatter)
            }
        }
    };
}

uuid_id!(GenerationRequestId);
uuid_id!(GenerationJobId);
uuid_id!(ArtifactId);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GenerationOperation {
    AnimatePhoto,
    GenerateBroll,
    CreateTransition,
    Stylize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GenerationSource {
    Asset {
        asset_id: AssetId,
        source_sha256: String,
    },
    Artifact {
        artifact_id: ArtifactId,
        source_sha256: String,
    },
}

impl GenerationSource {
    fn identity(&self) -> String {
        match self {
            Self::Asset { asset_id, .. } => format!("asset:{asset_id}"),
            Self::Artifact { artifact_id, .. } => format!("artifact:{artifact_id}"),
        }
    }

    fn source_sha256(&self) -> &str {
        match self {
            Self::Asset { source_sha256, .. } | Self::Artifact { source_sha256, .. } => {
                source_sha256
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GenerationRequest {
    pub id: GenerationRequestId,
    pub operation: GenerationOperation,
    #[serde(default)]
    pub sources: Vec<GenerationSource>,
}

impl GenerationRequest {
    #[must_use]
    pub fn new(operation: GenerationOperation, sources: Vec<GenerationSource>) -> Self {
        Self {
            id: GenerationRequestId::new(),
            operation,
            sources,
        }
    }

    pub fn validate(&self) -> Result<(), GenerationValidationError> {
        let mut identities = HashSet::with_capacity(self.sources.len());
        for source in &self.sources {
            ensure_sha256("generation source sha256", source.source_sha256())?;
            if !identities.insert(source.identity()) {
                return Err(GenerationValidationError::new(
                    "generation request contains a duplicate source",
                ));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProviderProvenance {
    pub provider: String,
    pub request_id: Option<String>,
    pub job_id: Option<String>,
}

impl ProviderProvenance {
    pub fn validate(&self) -> Result<(), GenerationValidationError> {
        ensure_text("provider provenance provider", &self.provider)?;
        ensure_optional_text("provider provenance request id", self.request_id.as_deref())?;
        ensure_optional_text("provider provenance job id", self.job_id.as_deref())?;
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GenerationProviderOutput {
    pub locator: String,
}

impl GenerationProviderOutput {
    pub fn validate(&self) -> Result<(), GenerationValidationError> {
        ensure_text("provider output locator", &self.locator)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GenerationJobPhase {
    Queued,
    Submitted,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum GenerationJobState {
    Queued,
    Submitted {
        provenance: ProviderProvenance,
    },
    Running {
        provenance: ProviderProvenance,
    },
    Completed {
        provenance: ProviderProvenance,
        output: GenerationProviderOutput,
    },
    Failed {
        provenance: Option<ProviderProvenance>,
        message: String,
    },
    Cancelled {
        provenance: Option<ProviderProvenance>,
    },
}

impl GenerationJobState {
    #[must_use]
    pub fn phase(&self) -> GenerationJobPhase {
        match self {
            Self::Queued => GenerationJobPhase::Queued,
            Self::Submitted { .. } => GenerationJobPhase::Submitted,
            Self::Running { .. } => GenerationJobPhase::Running,
            Self::Completed { .. } => GenerationJobPhase::Completed,
            Self::Failed { .. } => GenerationJobPhase::Failed,
            Self::Cancelled { .. } => GenerationJobPhase::Cancelled,
        }
    }

    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Completed { .. } | Self::Failed { .. } | Self::Cancelled { .. }
        )
    }

    fn validate(&self) -> Result<(), GenerationValidationError> {
        match self {
            Self::Queued => Ok(()),
            Self::Submitted { provenance } | Self::Running { provenance } => {
                provenance.validate()
            }
            Self::Completed { provenance, output } => {
                provenance.validate()?;
                output.validate()
            }
            Self::Failed {
                provenance,
                message,
            } => {
                if let Some(provenance) = provenance {
                    provenance.validate()?;
                }
                ensure_text("generation failure message", message)
            }
            Self::Cancelled { provenance } => {
                if let Some(provenance) = provenance {
                    provenance.validate()?;
                }
                Ok(())
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GenerationJob {
    pub id: GenerationJobId,
    pub request: GenerationRequest,
    pub state: GenerationJobState,
}

impl GenerationJob {
    #[must_use]
    pub fn new(request: GenerationRequest) -> Self {
        Self {
            id: GenerationJobId::new(),
            request,
            state: GenerationJobState::Queued,
        }
    }

    pub fn transition_to(
        &mut self,
        next: GenerationJobState,
    ) -> Result<(), GenerationValidationError> {
        next.validate()?;
        let current_phase = self.state.phase();
        let next_phase = next.phase();
        let allowed = matches!(
            (current_phase, next_phase),
            (GenerationJobPhase::Queued, GenerationJobPhase::Submitted)
                | (GenerationJobPhase::Queued, GenerationJobPhase::Failed)
                | (GenerationJobPhase::Queued, GenerationJobPhase::Cancelled)
                | (GenerationJobPhase::Submitted, GenerationJobPhase::Running)
                | (GenerationJobPhase::Submitted, GenerationJobPhase::Completed)
                | (GenerationJobPhase::Submitted, GenerationJobPhase::Failed)
                | (GenerationJobPhase::Submitted, GenerationJobPhase::Cancelled)
                | (GenerationJobPhase::Running, GenerationJobPhase::Completed)
                | (GenerationJobPhase::Running, GenerationJobPhase::Failed)
                | (GenerationJobPhase::Running, GenerationJobPhase::Cancelled)
        );
        if !allowed {
            return Err(GenerationValidationError::new(format!(
                "invalid generation job transition from {current_phase:?} to {next_phase:?}"
            )));
        }
        self.state = next;
        Ok(())
    }

    pub fn validate(&self) -> Result<(), GenerationValidationError> {
        self.request.validate()?;
        self.state.validate()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DerivedArtifact {
    pub id: ArtifactId,
    pub job_id: GenerationJobId,
    pub request_id: GenerationRequestId,
    pub media_kind: MediaKind,
    pub materialized_path: PathBuf,
    pub sha256: String,
    pub byte_len: u64,
}

impl DerivedArtifact {
    pub fn from_verified_materialization(
        job: &GenerationJob,
        media_kind: MediaKind,
        materialized_path: PathBuf,
        sha256: String,
        byte_len: u64,
    ) -> Result<Self, GenerationValidationError> {
        if !matches!(&job.state, GenerationJobState::Completed { .. }) {
            return Err(GenerationValidationError::new(
                "only a completed generation job can publish a derived artifact",
            ));
        }
        let artifact = Self {
            id: ArtifactId::new(),
            job_id: job.id,
            request_id: job.request.id,
            media_kind,
            materialized_path,
            sha256,
            byte_len,
        };
        artifact.validate_basic()?;
        Ok(artifact)
    }

    fn validate_basic(&self) -> Result<(), GenerationValidationError> {
        ensure_project_relative_path(&self.materialized_path)?;
        ensure_sha256("derived artifact sha256", &self.sha256)?;
        if self.byte_len == 0 {
            return Err(GenerationValidationError::new(
                "derived artifact byte length must be greater than zero",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GenerationState {
    pub schema_version: u32,
    #[serde(default)]
    pub jobs: Vec<GenerationJob>,
    #[serde(default)]
    pub artifacts: Vec<DerivedArtifact>,
}

impl GenerationState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            schema_version: GENERATION_SCHEMA_VERSION,
            jobs: Vec::new(),
            artifacts: Vec::new(),
        }
    }

    #[must_use]
    pub fn job(&self, id: GenerationJobId) -> Option<&GenerationJob> {
        self.jobs.iter().find(|job| job.id == id)
    }

    #[must_use]
    pub fn artifact(&self, id: ArtifactId) -> Option<&DerivedArtifact> {
        self.artifacts.iter().find(|artifact| artifact.id == id)
    }

    pub fn validate_for(
        &self,
        project: &ProjectManifest,
    ) -> Result<(), GenerationValidationError> {
        if self.schema_version != GENERATION_SCHEMA_VERSION {
            return Err(GenerationValidationError::new(format!(
                "unsupported generation schema {}; expected {}",
                self.schema_version, GENERATION_SCHEMA_VERSION
            )));
        }

        let mut job_ids = HashSet::with_capacity(self.jobs.len());
        let mut request_ids = HashSet::with_capacity(self.jobs.len());
        let mut jobs = HashMap::with_capacity(self.jobs.len());
        for job in &self.jobs {
            job.validate()?;
            if !job_ids.insert(job.id) {
                return Err(GenerationValidationError::new(
                    "generation state contains a duplicate job id",
                ));
            }
            if !request_ids.insert(job.request.id) {
                return Err(GenerationValidationError::new(
                    "generation state contains a duplicate request id",
                ));
            }
            jobs.insert(job.id, job);
        }

        let mut artifact_ids = HashSet::with_capacity(self.artifacts.len());
        let mut artifacts = HashMap::with_capacity(self.artifacts.len());
        for artifact in &self.artifacts {
            artifact.validate_basic()?;
            if !artifact_ids.insert(artifact.id) {
                return Err(GenerationValidationError::new(
                    "generation state contains a duplicate artifact id",
                ));
            }
            let job = jobs.get(&artifact.job_id).ok_or_else(|| {
                GenerationValidationError::new(
                    "derived artifact references a generation job that does not exist",
                )
            })?;
            if artifact.request_id != job.request.id {
                return Err(GenerationValidationError::new(
                    "derived artifact request id does not match its generation job",
                ));
            }
            if !matches!(&job.state, GenerationJobState::Completed { .. }) {
                return Err(GenerationValidationError::new(
                    "derived artifact references a generation job that is not completed",
                ));
            }
            artifacts.insert(artifact.id, artifact);
        }

        for job in &self.jobs {
            for source in &job.request.sources {
                match source {
                    GenerationSource::Asset {
                        asset_id,
                        source_sha256,
                    } => {
                        let asset = project.asset(*asset_id).ok_or_else(|| {
                            GenerationValidationError::new(format!(
                                "generation request references asset {asset_id} that is not in the project"
                            ))
                        })?;
                        if &asset.source_sha256 != source_sha256 {
                            return Err(GenerationValidationError::new(format!(
                                "generation request source revision for asset {asset_id} does not match the project"
                            )));
                        }
                    }
                    GenerationSource::Artifact {
                        artifact_id,
                        source_sha256,
                    } => {
                        let source_artifact = artifacts.get(artifact_id).ok_or_else(|| {
                            GenerationValidationError::new(format!(
                                "generation request references artifact {artifact_id} that is not published"
                            ))
                        })?;
                        if &source_artifact.sha256 != source_sha256 {
                            return Err(GenerationValidationError::new(format!(
                                "generation request source revision for artifact {artifact_id} does not match the published artifact"
                            )));
                        }
                    }
                }
            }
        }

        validate_acyclic_lineage(&artifacts, &jobs)?;
        Ok(())
    }
}

impl Default for GenerationState {
    fn default() -> Self {
        Self::new()
    }
}

fn validate_acyclic_lineage(
    artifacts: &HashMap<ArtifactId, &DerivedArtifact>,
    jobs: &HashMap<GenerationJobId, &GenerationJob>,
) -> Result<(), GenerationValidationError> {
    let mut visited = HashSet::with_capacity(artifacts.len());
    let mut visiting = HashSet::with_capacity(artifacts.len());
    for artifact_id in artifacts.keys().copied() {
        visit_artifact_lineage(artifact_id, artifacts, jobs, &mut visiting, &mut visited)?;
    }
    Ok(())
}

fn visit_artifact_lineage(
    artifact_id: ArtifactId,
    artifacts: &HashMap<ArtifactId, &DerivedArtifact>,
    jobs: &HashMap<GenerationJobId, &GenerationJob>,
    visiting: &mut HashSet<ArtifactId>,
    visited: &mut HashSet<ArtifactId>,
) -> Result<(), GenerationValidationError> {
    if visited.contains(&artifact_id) {
        return Ok(());
    }
    if !visiting.insert(artifact_id) {
        return Err(GenerationValidationError::new(
            "derived artifact lineage contains a cycle",
        ));
    }

    let artifact = artifacts.get(&artifact_id).ok_or_else(|| {
        GenerationValidationError::new("derived artifact lineage references a missing artifact")
    })?;
    let job = jobs.get(&artifact.job_id).ok_or_else(|| {
        GenerationValidationError::new("derived artifact lineage references a missing job")
    })?;
    for source in &job.request.sources {
        if let GenerationSource::Artifact {
            artifact_id: source_artifact_id,
            ..
        } = source
        {
            visit_artifact_lineage(*source_artifact_id, artifacts, jobs, visiting, visited)?;
        }
    }

    visiting.remove(&artifact_id);
    visited.insert(artifact_id);
    Ok(())
}

fn ensure_project_relative_path(path: &Path) -> Result<(), GenerationValidationError> {
    if path.as_os_str().is_empty() || path.is_absolute() {
        return Err(GenerationValidationError::new(
            "derived artifact materialized path must be a non-empty project-relative path",
        ));
    }
    if path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Err(GenerationValidationError::new(
            "derived artifact materialized path must not escape the project root",
        ));
    }
    Ok(())
}

fn ensure_text(field: &str, value: &str) -> Result<(), GenerationValidationError> {
    if value.trim().is_empty() {
        return Err(GenerationValidationError::new(format!(
            "{field} must not be empty"
        )));
    }
    Ok(())
}

fn ensure_optional_text(
    field: &str,
    value: Option<&str>,
) -> Result<(), GenerationValidationError> {
    if let Some(value) = value {
        ensure_text(field, value)?;
    }
    Ok(())
}

fn ensure_sha256(field: &str, value: &str) -> Result<(), GenerationValidationError> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(GenerationValidationError::new(format!(
            "{field} must be a 64-character hexadecimal digest"
        )));
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GenerationValidationError {
    message: String,
}

impl GenerationValidationError {
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for GenerationValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for GenerationValidationError {}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::{Asset, AssetId, AssetTechnicalMetadata, MediaKind, ProjectManifest};

    use super::{
        DerivedArtifact, GenerationJob, GenerationJobState, GenerationOperation,
        GenerationProviderOutput, GenerationRequest, GenerationSource, GenerationState,
        ProviderProvenance,
    };

    fn digest(character: char) -> String {
        std::iter::repeat_n(character, 64).collect()
    }

    fn fixture_project() -> ProjectManifest {
        let mut project = ProjectManifest::new("Trip");
        project.assets.push(Asset {
            id: AssetId::new(),
            original_path: PathBuf::from("photo.jpg"),
            display_name: "photo.jpg".to_owned(),
            media_kind: MediaKind::Image,
            source_sha256: digest('a'),
            byte_len: 10,
            technical: AssetTechnicalMetadata::default(),
        });
        project
    }

    fn provenance() -> ProviderProvenance {
        ProviderProvenance {
            provider: "fixture-provider".to_owned(),
            request_id: Some("request-1".to_owned()),
            job_id: Some("job-1".to_owned()),
        }
    }

    fn completed_job(project: &ProjectManifest) -> GenerationJob {
        let asset = &project.assets[0];
        let request = GenerationRequest::new(
            GenerationOperation::AnimatePhoto,
            vec![GenerationSource::Asset {
                asset_id: asset.id,
                source_sha256: asset.source_sha256.clone(),
            }],
        );
        let mut job = GenerationJob::new(request);
        job.transition_to(GenerationJobState::Submitted {
            provenance: provenance(),
        })
        .expect("submit transition");
        job.transition_to(GenerationJobState::Completed {
            provenance: provenance(),
            output: GenerationProviderOutput {
                locator: "fixture://output".to_owned(),
            },
        })
        .expect("complete transition");
        job
    }

    #[test]
    fn non_completed_job_cannot_publish_artifact() {
        let project = fixture_project();
        let asset = &project.assets[0];
        let request = GenerationRequest::new(
            GenerationOperation::AnimatePhoto,
            vec![GenerationSource::Asset {
                asset_id: asset.id,
                source_sha256: asset.source_sha256.clone(),
            }],
        );
        let job = GenerationJob::new(request);

        let result = DerivedArtifact::from_verified_materialization(
            &job,
            MediaKind::Video,
            PathBuf::from(".laputa/derivatives/out.mp4"),
            digest('b'),
            100,
        );

        assert!(result.is_err());
    }

    #[test]
    fn completed_job_can_publish_verified_artifact() {
        let project = fixture_project();
        let job = completed_job(&project);

        let artifact = DerivedArtifact::from_verified_materialization(
            &job,
            MediaKind::Video,
            PathBuf::from(".laputa/derivatives/out.mp4"),
            digest('b'),
            100,
        )
        .expect("completed job should publish artifact");
        let mut state = GenerationState::new();
        state.jobs.push(job);
        state.artifacts.push(artifact);

        state.validate_for(&project).expect("valid state");
    }

    #[test]
    fn terminal_job_cannot_restart() {
        let project = fixture_project();
        let mut job = completed_job(&project);

        let result = job.transition_to(GenerationJobState::Running {
            provenance: provenance(),
        });

        assert!(result.is_err());
    }

    #[test]
    fn stale_asset_revision_is_rejected() {
        let project = fixture_project();
        let asset = &project.assets[0];
        let request = GenerationRequest::new(
            GenerationOperation::AnimatePhoto,
            vec![GenerationSource::Asset {
                asset_id: asset.id,
                source_sha256: digest('f'),
            }],
        );
        let mut state = GenerationState::new();
        state.jobs.push(GenerationJob::new(request));

        assert!(state.validate_for(&project).is_err());
    }

    #[test]
    fn artifact_path_cannot_escape_project_root() {
        let project = fixture_project();
        let job = completed_job(&project);

        let result = DerivedArtifact::from_verified_materialization(
            &job,
            MediaKind::Video,
            PathBuf::from("../outside.mp4"),
            digest('b'),
            100,
        );

        assert!(result.is_err());
    }
}
