//! Retained source ownership prepared for the coordinated product cutover.

use std::sync::Arc;

use mech_syntax::document::{
    AstNode, DocumentSyntax, ParseConfig, SyntaxSnapshot, TextSnapshot,
    parser::parse_canonical_document,
};

use super::{CanonicalDocumentIndex, CanonicalSourceIndexError};

/// One complete source revision and its canonical syntax, including diagnostics.
///
/// Parsing this record declares finite input. Streaming callers must keep their
/// `DocumentStream` until application submission; a provisional preview is not
/// an executable source record. No constructor accepts an arbitrary snapshot.
///
/// The coordinated cutover replaces product AST caches with this owner. It is
/// not a second tree field or a parser selector on the shipping source record.
#[derive(Clone, Debug)]
pub struct SourceDocument {
    snapshot: Arc<SyntaxSnapshot>,
}

impl SourceDocument {
    /// Retain exactly the supplied text and revision, without trimming. Malformed
    /// source is retained so infallible builders can report it at later admission.
    pub fn parse(source: TextSnapshot, config: ParseConfig) -> Self {
        Self {
            snapshot: Arc::new(parse_canonical_document(source, config)),
        }
    }

    pub fn snapshot(&self) -> &SyntaxSnapshot {
        &self.snapshot
    }

    pub fn source(&self) -> &TextSnapshot {
        &self.snapshot.source
    }

    pub fn document(&self) -> DocumentSyntax {
        DocumentSyntax::cast(self.snapshot.syntax())
            .expect("canonical document parser owns the document root")
    }

    pub fn is_strictly_clean(&self) -> bool {
        self.snapshot.is_strictly_clean()
    }

    /// Project all local owners only after strict admission. A failed document
    /// cannot publish an index of the clean declarations preceding its error.
    /// Actual resolved binding compilation remains the semantic owner's job.
    pub fn index(&self) -> Result<CanonicalDocumentIndex, CanonicalSourceIndexError> {
        if !self.is_strictly_clean() {
            return Err(CanonicalSourceIndexError {
                document: self.snapshot.document,
                revision: self.snapshot.revision,
                range: self.source().full_range(),
                message: "cannot index an invalid retained source document",
            });
        }
        CanonicalDocumentIndex::from_document(&self.document())
    }
}
