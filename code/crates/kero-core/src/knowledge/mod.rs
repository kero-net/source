//! Project, mount, source, and provenance foundations.

pub mod canonical;
pub mod compiler;
pub mod diagnostics;
pub mod encoding;
pub mod ids;
pub mod importers;
pub mod mount;
pub mod project;
pub mod provenance;
pub mod semantic;
pub mod source;
pub mod status;

pub use canonical::{CANONICAL_VERSION, CanonicalKnowledge, canonicalize};
pub use encoding::{ENCODING_SCHEMA, EncodingError, decode, encode};

pub use compiler::*;
pub use diagnostics::{Diagnostic, Severity};
pub use ids::{
    ContentDigest, DerivationId, EntityId, IdError, KnowledgeSetId, MountId, SemanticRecordId,
    SourceId, SourceRegionId, SourceRevisionId,
};
pub use importers::*;
pub use mount::{Mount, MountKind};
pub use project::{
    PROJECT_DIRECTORY, PROJECT_FILE, PROJECT_SCHEMA, Project, ProjectBoundary, ProjectError,
    declaration_digest, discover, initialize, load, save, source_candidates,
};
pub use provenance::Provenance;
pub use semantic::*;
pub use source::{
    Availability, OpaqueSource, Source, SourceError, SourceObservation, SourceRegion,
    SourceRevision, observe_mount,
};
pub use status::*;
