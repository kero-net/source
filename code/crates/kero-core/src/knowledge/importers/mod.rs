//! Importers keep acquisition, syntax, and semantic emission as distinct steps.

pub mod markdown;
pub use markdown::MarkdownImporter;

use super::{Diagnostic, SemanticIr, Source, SourceRevision};

pub trait Importer {
    type Acquired;
    type Syntax;

    fn acquire(&self, source: &Source, bytes: Vec<u8>) -> Result<Self::Acquired, Diagnostic>;
    fn parse(
        &self,
        revision: &SourceRevision,
        input: Self::Acquired,
    ) -> Result<Self::Syntax, Vec<Diagnostic>>;
    fn emit(&self, syntax: Self::Syntax, ir: &mut SemanticIr) -> Result<(), Vec<Diagnostic>>;
}

pub const MARKDOWN_ANNOTATION_VERSION: &str = "kero/markdown-annotations/v1alpha1";
