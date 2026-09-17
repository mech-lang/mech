//! Retained source ownership prepared for the coordinated product cutover.

use std::sync::Arc;

use mech_syntax::document::{
    AstNode, DocumentScopeId, DocumentSession, DocumentStream, DocumentSyntax, ParseConfig,
    Revision, SourceError, StreamError, StreamState, SyntaxSnapshot, TextSnapshot,
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

impl PartialEq for SourceDocument {
    fn eq(&self, other: &Self) -> bool {
        self.snapshot.document == other.snapshot.document
            && self.snapshot.revision == other.snapshot.revision
            && self.snapshot.root.kind == other.snapshot.root.kind
            && self.snapshot.root.flags == other.snapshot.root.flags
            && self.snapshot.root.text_len == other.snapshot.root.text_len
            && self.snapshot.root.structural_hash == other.snapshot.root.structural_hash
            && self.snapshot.diagnostics == other.snapshot.diagnostics
            && self.source().to_contiguous_string() == other.source().to_contiguous_string()
    }
}

impl Eq for SourceDocument {}

/// Admission distinguishes invalid syntax from conflicts in a local resolver owner.
#[derive(Clone, Debug)]
pub enum SourceDocumentIndexError {
    Syntax(CanonicalSourceIndexError),
    AddressTargets {
        owner: DocumentScopeId,
        error: Box<mech_core::MechError>,
    },
}

impl std::fmt::Display for SourceDocumentIndexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Syntax(error) => error.fmt(f),
            Self::AddressTargets { owner, error } => {
                write!(f, "{} in {owner:?}", error.kind_message())
            }
        }
    }
}
impl std::error::Error for SourceDocumentIndexError {}

impl mech_core::MechErrorKind for SourceDocumentIndexError {
    fn name(&self) -> &str {
        "SourceDocumentIndexError"
    }

    fn message(&self) -> String {
        self.to_string()
    }
}

impl SourceDocument {
    /// Parse the exact resolver-owned text under a stable document identity.
    /// The canonical URI selects the document owner; the caller supplies the
    /// revision so replacements can retain an explicit revision sequence.
    pub fn parse_resolved(
        canonical_uri: &str,
        revision: Revision,
        source: impl Into<Arc<str>>,
        config: ParseConfig,
    ) -> Result<Self, SourceError> {
        Ok(Self::parse(
            TextSnapshot::new(
                mech_syntax::document::DocumentId(mech_core::hash_str(canonical_uri)),
                revision,
                source,
            )?,
            config,
        ))
    }

    /// Retain exactly the supplied text and revision, without trimming. Malformed
    /// source is retained so infallible builders can report it at later admission.
    pub fn parse(source: TextSnapshot, config: ParseConfig) -> Self {
        Self {
            snapshot: Arc::new(parse_canonical_document(source, config)),
        }
    }

    /// Adopt the stream's finalized canonical result without parsing again.
    ///
    /// Submission and scheduling remain the caller's responsibility. This method
    /// neither finishes an open stream nor admits its finite preview. Limited and
    /// cancelled streams stay with their diagnostic owner. A finished malformed
    /// document can be retained, but still fails strict index/execution admission.
    pub fn from_finished_stream(stream: &mut DocumentStream) -> Result<Self, StreamError> {
        match stream.state() {
            StreamState::Finished => Ok(Self {
                snapshot: stream.materialize()?,
            }),
            StreamState::Open | StreamState::Finishing => Err(StreamError::NotFinal),
            state => Err(StreamError::Closed(state)),
        }
    }

    /// Retain an explicitly finite editor revision, sharing its canonical tree
    /// and preserving node identities. Later session edits cannot mutate it.
    /// This does not accept a streaming preview or an arbitrary SyntaxSnapshot.
    pub fn from_session(session: &DocumentSession) -> Self {
        Self {
            snapshot: Arc::new(session.snapshot().clone()),
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
    pub fn index(&self) -> Result<CanonicalDocumentIndex, SourceDocumentIndexError> {
        if !self.is_strictly_clean() {
            return Err(SourceDocumentIndexError::Syntax(
                CanonicalSourceIndexError {
                    document: self.snapshot.document,
                    revision: self.snapshot.revision,
                    range: self.source().full_range(),
                    message: "cannot index an invalid retained source document",
                },
            ));
        }
        let index = CanonicalDocumentIndex::from_document(&self.document())
            .map_err(SourceDocumentIndexError::Syntax)?;
        index.root.validate_address_targets().map_err(|error| {
            SourceDocumentIndexError::AddressTargets {
                owner: index.owner,
                error: Box::new(error),
            }
        })?;
        for local in &index.mika {
            local.index.validate_address_targets().map_err(|error| {
                SourceDocumentIndexError::AddressTargets {
                    owner: local.owner.section.scope_id(),
                    error: Box::new(error),
                }
            })?;
        }
        Ok(index)
    }
}
