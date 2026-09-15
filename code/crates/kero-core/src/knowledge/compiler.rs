//! Deterministic batch compiler service. Progress and cancellation are operational
//! concerns and are deliberately absent from canonical bytes.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use thiserror::Error;

use super::*;

pub const COMPILER_VERSION: &str = "kero/compiler/v1alpha1";
pub const COMPILED_STATE_VERSION: &str = "kero/compiled-state/v1alpha1";
pub const COMPILATION_REPORT_VERSION: &str = "kero/compilation-report/v1alpha1";
pub const ARTIFACT_SCHEMA: &str = "kero/compiled-artifact/v1alpha1";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CompilerStage {
    ProjectLoaded,
    SourcesAcquired,
    Parsed,
    Resolved,
    Validated,
    Canonicalized,
    Written,
    Verified,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ProgressEvent {
    pub sequence: u32,
    pub stage: CompilerStage,
}

pub trait ProgressSink {
    fn event(&mut self, event: ProgressEvent);
}
impl<F: FnMut(ProgressEvent)> ProgressSink for F {
    fn event(&mut self, event: ProgressEvent) {
        self(event)
    }
}

#[derive(Default)]
pub struct CancellationToken(AtomicBool);
impl CancellationToken {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CompiledMountState {
    pub mount_id: MountId,
    pub source_id: SourceId,
    pub enabled: bool,
    pub availability: Availability,
    pub revision_id: Option<SourceRevisionId>,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CompiledState {
    pub version: String,
    pub project_digest: String,
    pub mounts: Vec<CompiledMountState>,
    pub compiler: String,
    pub semantic_ir: String,
    pub canonicalization: String,
    pub encoding: String,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CompilationReport {
    pub version: String,
    pub source_count: u64,
    pub semantic_record_count: u64,
    pub diagnostic_count: u64,
    pub compiler: String,
    pub semantic_ir: String,
    pub canonicalization: String,
    pub encoding: String,
    pub output_digest: String,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CompiledArtifact {
    pub schema: String,
    pub state: CompiledState,
    pub report: CompilationReport,
    pub knowledge: CanonicalKnowledge,
}

#[derive(Debug, Error)]
pub enum CompilerError {
    #[error("compiler.project: {0}")]
    Project(#[from] ProjectError),
    #[error("compiler.source: {0}")]
    Source(#[from] SourceError),
    #[error("compiler.io: {0}")]
    Io(#[from] std::io::Error),
    #[error("compiler.encoding: {0}")]
    Encoding(#[from] EncodingError),
    #[error("compiler.semantic: {0}")]
    Semantic(#[from] SemanticError),
    #[error("compiler.diagnostics: compilation failed with {0:?}")]
    Diagnostics(Vec<Diagnostic>),
    #[error("compiler.cancelled")]
    Cancelled,
    #[error("compiler.output-collision: {0}")]
    OutputCollision(PathBuf),
    #[error("compiler.artifact-invalid: {0}")]
    Artifact(String),
}
impl CompilerError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Project(_) => "compiler.project",
            Self::Source(_) => "compiler.source",
            Self::Io(_) => "compiler.io",
            Self::Encoding(_) => "compiler.encoding",
            Self::Semantic(_) => "compiler.semantic",
            Self::Diagnostics(_) => "compiler.diagnostics",
            Self::Cancelled => "compiler.cancelled",
            Self::OutputCollision(_) => "compiler.output-collision",
            Self::Artifact(_) => "compiler.artifact-invalid",
        }
    }
}

pub struct CompilerService;
impl CompilerService {
    pub fn validate(boundary: &ProjectBoundary) -> Result<CompilationReport, CompilerError> {
        Self::compile_inner(boundary, None, &CancellationToken::default(), &mut |_| {})
            .map(|v| v.report)
    }
    pub fn compile(
        boundary: &ProjectBoundary,
        output: &Path,
        cancellation: &CancellationToken,
        progress: &mut dyn ProgressSink,
    ) -> Result<CompiledArtifact, CompilerError> {
        Self::compile_inner(boundary, Some(output), cancellation, progress)
    }
    fn compile_inner(
        boundary: &ProjectBoundary,
        output: Option<&Path>,
        cancellation: &CancellationToken,
        progress: &mut dyn ProgressSink,
    ) -> Result<CompiledArtifact, CompilerError> {
        let mut seq = 0;
        let mut emit = |stage| {
            progress.event(ProgressEvent {
                sequence: seq,
                stage,
            });
            seq += 1;
        };
        let check = || {
            if cancellation.is_cancelled() {
                Err(CompilerError::Cancelled)
            } else {
                Ok(())
            }
        };
        let project = load(boundary)?;
        emit(CompilerStage::ProjectLoaded);
        check()?;
        if let Some(output) = output {
            let absolute = if output.is_absolute() {
                output.to_path_buf()
            } else {
                std::env::current_dir()?.join(output)
            };
            for mount in &project.mounts {
                let source = boundary.root.join(&mount.source);
                if source.canonicalize().ok()
                    == absolute.canonicalize().ok().or(Some(absolute.clone()))
                {
                    return Err(CompilerError::OutputCollision(output.into()));
                }
            }
        }
        let mut observations = Vec::new();
        for mount in &project.mounts {
            observations.push(observe_mount(&boundary.root, mount)?);
        }
        emit(CompilerStage::SourcesAcquired);
        check()?;
        let mut ir = SemanticIr::new(project.id.clone());
        let importer = MarkdownImporter;
        for observation in &observations {
            match observation {
                SourceObservation::Current { source } => {
                    if !source.media_type.contains("markdown") {
                        return Err(CompilerError::Diagnostics(vec![
                            Diagnostic::error(
                                "importer.unsupported",
                                format!("no importer for {}", source.media_type),
                            )
                            .with_source(source.id.clone()),
                        ]));
                    }
                    let bytes = fs::read(boundary.root.join(&source.logical_locator))?;
                    let acquired = importer
                        .acquire(source, bytes)
                        .map_err(|d| CompilerError::Diagnostics(vec![d]))?;
                    let syntax = importer
                        .parse(source.revision.as_ref().unwrap(), acquired)
                        .map_err(CompilerError::Diagnostics)?;
                    importer
                        .emit(syntax, &mut ir)
                        .map_err(CompilerError::Diagnostics)?;
                }
                SourceObservation::Unavailable { source, diagnostic }
                    if project
                        .mounts
                        .iter()
                        .find(|m| m.id == source.mount_id)
                        .is_some_and(|m| m.required) =>
                {
                    return Err(CompilerError::Diagnostics(vec![diagnostic.clone()]));
                }
                SourceObservation::Unsupported { opaque } => {
                    return Err(CompilerError::Diagnostics(opaque.diagnostics.clone()));
                }
                _ => {}
            }
        }
        emit(CompilerStage::Parsed);
        check()?;
        emit(CompilerStage::Resolved);
        check()?;
        ir.validate()?;
        emit(CompilerStage::Validated);
        check()?;
        let knowledge = canonicalize(&ir)?;
        let canonical_bytes = encode(&knowledge, true)?;
        emit(CompilerStage::Canonicalized);
        check()?;
        let digest = sha256(&canonical_bytes);
        let mounts = observations.iter().map(observation_state).collect();
        let state = CompiledState {
            version: COMPILED_STATE_VERSION.into(),
            project_digest: declaration_digest(&project)?,
            mounts,
            compiler: COMPILER_VERSION.into(),
            semantic_ir: SEMANTIC_IR_VERSION.into(),
            canonicalization: CANONICAL_VERSION.into(),
            encoding: ENCODING_SCHEMA.into(),
        };
        let report = CompilationReport {
            version: COMPILATION_REPORT_VERSION.into(),
            source_count: observations
                .iter()
                .filter(|o| matches!(o, SourceObservation::Current { .. }))
                .count() as u64,
            semantic_record_count: (ir.entities.len()
                + ir.references.len()
                + ir.claims.len()
                + ir.relations.len()) as u64,
            diagnostic_count: 0,
            compiler: COMPILER_VERSION.into(),
            semantic_ir: SEMANTIC_IR_VERSION.into(),
            canonicalization: CANONICAL_VERSION.into(),
            encoding: ENCODING_SCHEMA.into(),
            output_digest: digest,
        };
        let artifact = CompiledArtifact {
            schema: ARTIFACT_SCHEMA.into(),
            state,
            report,
            knowledge,
        };
        if let Some(output) = output {
            write_atomic(output, &artifact)?;
            emit(CompilerStage::Written);
            emit(CompilerStage::Verified);
        }
        Ok(artifact)
    }
}
fn observation_state(value: &SourceObservation) -> CompiledMountState {
    let (source, availability) = match value {
        SourceObservation::Current { source } => (source, Availability::Available),
        SourceObservation::Disabled { source } => (source, Availability::Disabled),
        SourceObservation::Unavailable { source, .. } => (source, Availability::Unavailable),
        SourceObservation::Unsupported { opaque } => (&opaque.source, Availability::Unsupported),
    };
    CompiledMountState {
        mount_id: source.mount_id.clone(),
        source_id: source.id.clone(),
        enabled: !matches!(availability, Availability::Disabled),
        availability,
        revision_id: source.revision.as_ref().map(|v| v.id.clone()),
    }
}
fn sha256(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    format!("sha256:{}", hex::encode(h.finalize()))
}
pub fn artifact_bytes(value: &CompiledArtifact) -> Result<Vec<u8>, CompilerError> {
    let mut v = serde_json::to_vec(value).map_err(|e| CompilerError::Artifact(e.to_string()))?;
    v.push(b'\n');
    Ok(v)
}
pub fn read_artifact(path: &Path) -> Result<CompiledArtifact, CompilerError> {
    let bytes = fs::read(path)?;
    let value: CompiledArtifact =
        serde_json::from_slice(&bytes).map_err(|e| CompilerError::Artifact(e.to_string()))?;
    if value.schema != ARTIFACT_SCHEMA || artifact_bytes(&value)? != bytes {
        return Err(CompilerError::Artifact(
            "noncanonical or unsupported artifact".into(),
        ));
    }
    let canonical = canonicalize(&value.knowledge.ir)?;
    if canonical != value.knowledge
        || value.report.output_digest != sha256(&encode(&canonical, true)?)
    {
        return Err(CompilerError::Artifact(
            "digest or canonical knowledge mismatch".into(),
        ));
    }
    Ok(value)
}
fn write_atomic(path: &Path, artifact: &CompiledArtifact) -> Result<(), CompilerError> {
    let parent = path.parent().unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;
    let name = path
        .file_name()
        .and_then(|v| v.to_str())
        .unwrap_or("artifact");
    let tmp = parent.join(format!(".{name}.tmp"));
    let _ = fs::remove_file(&tmp);
    let mut f = OpenOptions::new().write(true).create_new(true).open(&tmp)?;
    let result = (|| {
        f.write_all(&artifact_bytes(artifact)?)?;
        f.sync_all()?;
        drop(f);
        let actual = read_artifact(&tmp)?;
        if &actual != artifact {
            return Err(CompilerError::Artifact("verification mismatch".into()));
        }
        fs::rename(&tmp, path)?;
        Ok::<_, CompilerError>(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(tmp);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn knowledge_compiler_cancellation_is_observed_before_source_work() {
        let temp = tempfile::tempdir().unwrap();
        let boundary =
            initialize(temp.path(), KnowledgeSetId::new("compiler.test").unwrap()).unwrap();
        let token = CancellationToken::default();
        token.cancel();
        assert!(matches!(
            CompilerService::compile(
                &boundary,
                &temp.path().join("out.json"),
                &token,
                &mut |_| {}
            ),
            Err(CompilerError::Cancelled)
        ));
    }
}
