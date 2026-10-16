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
    nominal_origin: Option<mech_core::CanonicalNominalPath>,
    nominal_package_id: Option<String>,
}

impl PartialEq for SourceDocument {
    fn eq(&self, other: &Self) -> bool {
        self.nominal_origin == other.nominal_origin
            && self.nominal_package_id == other.nominal_package_id
            && (Arc::ptr_eq(&self.snapshot, &other.snapshot)
                || (self.snapshot.document == other.snapshot.document
                    && self.snapshot.revision == other.snapshot.revision
                    && self.snapshot.diagnostics == other.snapshot.diagnostics
                    && same_syntax_identity(&self.snapshot.root, &other.snapshot.root)
                    && self.source().to_contiguous_string()
                        == other.source().to_contiguous_string()))
    }
}

// Compare the complete ordered graph: scope/result ownership uses IDs, and a
// fresh parse may preserve every structural hash while changing those owners.
// An explicit stack also handles deeply nested retained input without recursion.
fn same_syntax_identity(
    left: &Arc<mech_syntax::document::GreenNode>,
    right: &Arc<mech_syntax::document::GreenNode>,
) -> bool {
    use mech_syntax::document::GreenElement;
    let mut pending = vec![(left, right)];
    while let Some((left, right)) = pending.pop() {
        if Arc::ptr_eq(left, right) {
            continue;
        }
        if left.id != right.id
            || left.kind != right.kind
            || left.flags != right.flags
            || left.text_len != right.text_len
            || left.structural_hash != right.structural_hash
            || left.children.len() != right.children.len()
        {
            return false;
        }
        for (left, right) in left.children.iter().zip(right.children.iter()) {
            match (left, right) {
                (GreenElement::Node(left), GreenElement::Node(right)) => {
                    pending.push((left, right))
                }
                (GreenElement::Token(left), GreenElement::Token(right)) => {
                    if left.id != right.id
                        || left.kind != right.kind
                        || left.flags != right.flags
                        || left.text_len != right.text_len
                        || left.text_hash != right.text_hash
                    {
                        return false;
                    }
                }
                _ => return false,
            }
        }
    }
    true
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
    /// Validate reserved standalone identities wherever source provenance enters
    /// a host. Package namespaces are authoritative resolver inputs.
    pub fn validate_nominal_provenance(
        origin: Option<&mech_core::CanonicalNominalPath>,
        package_id: Option<&str>,
    ) -> mech_core::MResult<()> {
        if origin.is_some_and(|origin| {
            origin
                .segments()
                .first()
                .is_some_and(|namespace| namespace == "mech:standalone")
                && (!matches!(origin.segments(), [_, owner] if uuid::Uuid::parse_str(owner)
                .is_ok_and(|uuid| uuid.get_variant() == uuid::Variant::RFC4122
                    && uuid.get_version_num() == 7 && uuid.to_string() == *owner))
                    || package_id.is_some())
        }) {
            return Err(mech_core::MechError::new(
                mech_core::GenericError {
                    msg: "standalone nominal provenance is invalid; regenerate the bundle"
                        .to_owned(),
                },
                None,
            )
            .with_compiler_loc());
        }
        Ok(())
    }

    /// Allocate a durable owner for one standalone document or interactive session.
    /// The reserved virtual package cannot collide with a Cargo package name.
    /// Hosts that save editable documents must retain this origin alongside the
    /// source; neither source text nor its transport URI defines nominal identity.
    pub fn new_standalone_origin() -> mech_core::CanonicalNominalPath {
        mech_core::CanonicalNominalPath::new(vec![
            "mech:standalone".to_owned(),
            uuid::Uuid::now_v7().to_string(),
        ])
        .expect("the reserved standalone namespace and UUID are valid path segments")
    }

    /// Admit an explicitly standalone document without replacing package provenance.
    /// Call once when creating its owner, then retain the origin across revisions.
    pub fn with_standalone_nominal_origin(mut self) -> Self {
        if self.nominal_origin.is_none() {
            self.nominal_origin = Some(Self::new_standalone_origin());
        }
        self
    }

    /// Configure direct semantic compilation with this document's retained identity.
    /// Unowned resolver documents still require their defining package provenance.
    #[cfg(feature = "semantic-compiler")]
    pub fn canonical_frontend(&self) -> mech_engine::CanonicalSourceFrontend {
        self.nominal_origin
            .as_ref()
            .map_or(mech_engine::CanonicalSourceFrontend, |origin| {
                mech_engine::CanonicalSourceFrontend.with_nominal_origin(origin.clone())
            })
    }

    /// Attach the defining package and module namespace supplied by the
    /// resolver. Nominal declarations require this before compilation.
    pub fn with_nominal_origin(mut self, origin: mech_core::CanonicalNominalPath) -> Self {
        self.nominal_origin = Some(origin);
        self
    }

    /// Replace the defining origin and its optional package discriminator together.
    /// A new origin must not inherit a discriminator belonging to the old owner.
    pub fn with_nominal_provenance(
        mut self,
        origin: mech_core::CanonicalNominalPath,
        package_id: Option<String>,
    ) -> Self {
        self.nominal_origin = Some(origin);
        self.nominal_package_id = package_id;
        self
    }

    pub fn nominal_origin(&self) -> Option<&mech_core::CanonicalNominalPath> {
        self.nominal_origin.as_ref()
    }

    pub fn with_nominal_package_id(mut self, package_id: impl Into<String>) -> Self {
        self.nominal_package_id = Some(package_id.into());
        self
    }

    pub fn nominal_package_id(&self) -> Option<&str> {
        self.nominal_package_id.as_deref()
    }

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
            nominal_origin: None,
            nominal_package_id: None,
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
                nominal_origin: None,
                nominal_package_id: None,
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
            nominal_origin: None,
            nominal_package_id: None,
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
                    range: self
                        .snapshot
                        .diagnostics
                        .iter()
                        .find_map(|diagnostic| {
                            diagnostic
                                .primary
                                .resolve(self.source().revision(), &self.snapshot.nodes)
                        })
                        .unwrap_or_else(|| self.source().full_range()),
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

    /// Keep the retained diagnostic owner when projecting an index refusal into
    /// the runtime's existing error representation. File and inline admission
    /// use this boundary; neither reparses source nor publishes a partial index.
    pub(crate) fn index_with_diagnostics(&self) -> mech_core::MResult<CanonicalDocumentIndex> {
        self.index().map_err(|error| {
            let range = match &error {
                SourceDocumentIndexError::Syntax(error) => Some(error.range),
                SourceDocumentIndexError::AddressTargets { .. } => None,
            };
            let details = (!self.is_strictly_clean())
                .then(|| {
                    let mut details = String::new();
                    for diagnostic in self.snapshot.diagnostics.iter() {
                        details.push_str(&mech_syntax::document::render_plain(
                            diagnostic,
                            self.source(),
                            &self.snapshot.nodes,
                        ));
                        if let Some(range) = diagnostic
                            .primary
                            .resolve(self.source().revision(), &self.snapshot.nodes)
                        {
                            details.push_str(&format!(
                                "  source bytes {}..{}\n",
                                range.start.0, range.end.0
                            ));
                        }
                    }
                    details
                })
                .filter(|details| !details.is_empty());
            let mut error = mech_core::MechError::new(error, details);
            if let Some(range) = range {
                let location = |offset| {
                    let (line, column) = self.source().line_index().line_and_byte_column(offset);
                    mech_core::SourceLocation {
                        row: line + 1,
                        col: column.0 as usize + 1,
                    }
                };
                error.program_range = Some(mech_core::SourceRange {
                    start: location(range.start),
                    end: location(range.end),
                });
            }
            error
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mech_syntax::document::{GreenElement, GreenNode};

    fn change_descendant_identity(node: &mut GreenNode, token: bool) -> bool {
        let mut children = node.children.to_vec();
        let mut changed = false;
        for child in &mut children {
            match child {
                GreenElement::Node(child) if !token => {
                    Arc::make_mut(child).id.0 += 10_000;
                    changed = true;
                }
                GreenElement::Node(child) => {
                    changed = change_descendant_identity(Arc::make_mut(child), token);
                }
                GreenElement::Token(child) if token => {
                    child.id.0 += 10_000;
                    changed = true;
                }
                _ => {}
            }
            if changed {
                break;
            }
        }
        node.children = children.into();
        changed
    }

    #[test]
    fn equality_compares_descendant_node_and_token_ids_even_when_root_matches() {
        let original = SourceDocument::parse_resolved(
            "memory:ids",
            Revision(0),
            "answer := 1\n",
            ParseConfig::default(),
        )
        .unwrap();
        for token in [false, true] {
            let mut changed = original.clone();
            let snapshot = Arc::make_mut(&mut changed.snapshot);
            assert!(change_descendant_identity(
                Arc::make_mut(&mut snapshot.root),
                token
            ));
            assert_eq!(original.snapshot.root.id, changed.snapshot.root.id);
            assert_eq!(
                original.snapshot.root.structural_hash,
                changed.snapshot.root.structural_hash
            );
            assert_ne!(original, changed);
        }
    }
}
