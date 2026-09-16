//! Source resolution for the Mech runtime.
//!
//! A SourceResolver answers:
//!
//!   "Given this source specifier, where does the source come from?"
//!
//! This layer is intentionally broader than module resolution. It can resolve:
//!
//! - Mech source
//! - Mech bytecode
//! - Mech docs
//! - package/config files
//! - HTML
//! - CSS
//! - Markdown
//! - CSV/data files
//! - JavaScript
//! - images
//! - database-backed sources
//! - package-manager sources
//! - editor/workspace buffers
//! - embedded runtime sources
//!
//! The compiler session should not own source resolution. It compiles source
//! only after resolution. MechRuntime owns a SourceResolver and decides how
//! resolved sources are stored, checked, activated, and executed.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use mech_core::{MResult, MechError, MechErrorKind, MechSourceCode};

use crate::capability::CapabilityRequest;

// -----------------------------------------------------------------------------
// Submodules
// -----------------------------------------------------------------------------

#[cfg(feature = "source")]
mod canonical_handoff;
#[cfg(feature = "source")]
mod document;
#[cfg(feature = "source")]
pub use document::{SourceDocument, SourceDocumentIndexError};
pub mod file;
pub mod imports;
pub mod index;
pub mod memory;
pub mod source;

#[cfg(feature = "source")]
pub use canonical_handoff::*;
pub use file::*;
pub use imports::*;
pub use index::*;
pub use memory::*;
pub use source::*;

// -----------------------------------------------------------------------------
// Source Request
// -----------------------------------------------------------------------------

/// Request to resolve a source-like asset.
///
/// `specifier` is the user/runtime-facing reference:
///
/// - `main.mec`
/// - `./src/foo.mec`
/// - `pkg:plot@1.2.0`
/// - `mech:std/math`
/// - `db:module/main`
/// - `workspace:current-buffer`
/// - `file:///project/main.mec`
/// - `https://example.com/main.mec`
///
/// `referrer` is the source that made the request. Filesystem and package
/// resolvers can use it for relative resolution.
///
/// `kind_hint` is optional and should not be trusted as authoritative. It is a
/// caller hint such as `mech`, `html`, `css`, `image`, `package`, or `data`.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SourceRequest {
    pub specifier: String,
    pub referrer: Option<String>,
    pub kind_hint: Option<String>,
}

impl SourceRequest {
    pub fn new(specifier: impl Into<String>) -> Self {
        Self {
            specifier: specifier.into(),
            referrer: None,
            kind_hint: None,
        }
    }

    pub fn with_referrer(mut self, referrer: impl Into<String>) -> Self {
        self.referrer = Some(referrer.into());
        self
    }

    pub fn with_kind_hint(mut self, kind_hint: impl Into<String>) -> Self {
        self.kind_hint = Some(kind_hint.into());
        self
    }

    pub fn validate(&self) -> MResult<()> {
        if self.specifier.trim().is_empty() {
            return invalid_source_request("specifier", "must not be empty");
        }

        if let Some(referrer) = &self.referrer {
            if referrer.trim().is_empty() {
                return invalid_source_request("referrer", "must not be empty when present");
            }
        }

        if let Some(kind_hint) = &self.kind_hint {
            if kind_hint.trim().is_empty() {
                return invalid_source_request("kind_hint", "must not be empty when present");
            }
        }

        Ok(())
    }
}

impl From<&str> for SourceRequest {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for SourceRequest {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

// -----------------------------------------------------------------------------
// Resolved Source
// -----------------------------------------------------------------------------

/// Source returned by a SourceResolver.
///
/// `name` is a human-readable name used in diagnostics and module records.
///
/// `canonical_uri` is the stable identity of this source from the resolver's
/// perspective:
///
/// - `file:///abs/path/main.mec`
/// - `memory:main`
/// - `pkg:plot@1.2.0/src/main.mec`
/// - `db:module/main@version`
/// - `mech:std/math`
/// - `workspace:current-buffer`
/// - `https://example.com/main.mec`
///
/// The runtime should prefer `canonical_uri` over the original request
/// specifier when computing stable source/module identity.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceImportKind {
    Namespace,
    Single { name: String },
    Wildcard,
    DependencyOnly,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceImportAlias {
    Value(String),
    Context(String),
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceImportDeclaration {
    pub specifier: String,
    pub alias: Option<SourceImportAlias>,
    pub module: Option<String>,
    pub item: Option<String>,
    pub kind: SourceImportKind,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceExportDeclaration {
    pub name: String,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceAddressReference {
    pub name: String,
    pub target: String,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceContextDeclaration {
    pub name: String,
    pub base: SourceContextBase,
    pub capabilities: Vec<SourceContextCapability>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceContextBase {
    ResourceUri(String),
    Context(String),
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceContextCapability {
    pub operation: String,
    pub scope: SourceContextCapabilityScope,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceContextCapabilityScope {
    Path(String),
    Wildcard,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedSource {
    pub name: String,
    pub canonical_uri: String,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub nominal_origin: Option<mech_core::CanonicalNominalPath>,
    /// Resolver-owned package identity for collision checks; never part of a nominal key.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub nominal_package_id: Option<String>,
    pub source: MechSourceCode,
    /// Resolver-owned canonical revision. This is the source/syntax authority
    /// prepared for product compilation, indexing, rendering, and diagnostics.
    #[cfg(feature = "source")]
    #[cfg_attr(feature = "serde", serde(skip))]
    pub source_document: Option<SourceDocument>,
    pub kind: SourceKind,
    pub imports: Vec<SourceImportDeclaration>,
    pub exports: Vec<SourceExportDeclaration>,
    pub contexts: Vec<SourceContextDeclaration>,
    pub address_references: Vec<SourceAddressReference>,
    pub scopes: Vec<ModuleScopeMetadata>,
    pub dependencies: Vec<SourceRequest>,
    pub capability_requirements: Vec<CapabilityRequest>,
}

impl ResolvedSource {
    pub fn new(
        name: impl Into<String>,
        canonical_uri: impl Into<String>,
        source: MechSourceCode,
    ) -> Self {
        Self {
            name: name.into(),
            canonical_uri: canonical_uri.into(),
            nominal_origin: None,
            nominal_package_id: None,
            source,
            #[cfg(feature = "source")]
            source_document: None,
            kind: SourceKind::Unknown("".to_string()),
            imports: Vec::new(),
            exports: Vec::new(),
            contexts: Vec::new(),
            address_references: Vec::new(),
            scopes: Vec::new(),
            dependencies: Vec::new(),
            capability_requirements: Vec::new(),
        }
    }

    pub fn with_nominal_origin(mut self, origin: mech_core::CanonicalNominalPath) -> Self {
        #[cfg(feature = "source")]
        if let Some(document) = self.source_document.take() {
            self.source_document = Some(document.with_nominal_origin(origin.clone()));
        }
        self.nominal_origin = Some(origin);
        self
    }

    pub fn with_nominal_package_id(mut self, package_id: impl Into<String>) -> Self {
        let package_id = package_id.into();
        #[cfg(feature = "source")]
        if let Some(document) = self.source_document.take() {
            self.source_document = Some(document.with_nominal_package_id(package_id.clone()));
        }
        self.nominal_package_id = Some(package_id);
        self
    }

    /// Attach the canonical revision only when it retains the exact same raw
    /// source. This prevents a resolver record from publishing two source
    /// authorities with different bytes.
    #[cfg(feature = "source")]
    pub fn with_source_document(mut self, document: SourceDocument) -> MResult<Self> {
        let document = match (self.nominal_origin.as_ref(), document.nominal_origin()) {
            (Some(resolved), Some(retained)) if resolved != retained => {
                return invalid_resolved_source(
                    "nominal_origin",
                    "must agree with the retained document origin",
                );
            }
            (Some(origin), _) => document.with_nominal_origin(origin.clone()),
            (None, Some(origin)) => {
                self.nominal_origin = Some(origin.clone());
                document
            }
            (None, None) => document,
        };
        let document = match (
            self.nominal_package_id.as_deref(),
            document.nominal_package_id(),
        ) {
            (Some(resolved), Some(retained)) if resolved != retained => {
                return invalid_resolved_source(
                    "nominal_package_id",
                    "must agree with the retained document package identity",
                );
            }
            (Some(package_id), _) => document.with_nominal_package_id(package_id),
            (None, Some(package_id)) => {
                self.nominal_package_id = Some(package_id.to_owned());
                document
            }
            (None, None) => document,
        };
        self.validate_document_owner(&document)?;
        match &self.source {
            MechSourceCode::String(source)
                if source.as_str() == document.source().to_contiguous_string() => {}
            MechSourceCode::String(_) => {
                return invalid_resolved_source(
                    "source_document",
                    "must retain the exact resolved source bytes",
                );
            }
            _ => {
                return invalid_resolved_source(
                    "source_document",
                    "is only valid for textual Mech source",
                );
            }
        }
        self.source_document = Some(document);
        Ok(self)
    }

    #[cfg(feature = "source")]
    fn validate_document_owner(&self, document: &SourceDocument) -> MResult<()> {
        if document.source().document().0 != mech_core::hash_str(&self.canonical_uri) {
            return invalid_resolved_source(
                "source_document",
                "document owner does not match the canonical URI",
            );
        }
        Ok(())
    }

    #[cfg(feature = "source")]
    pub fn source_document(&self) -> Option<&SourceDocument> {
        self.source_document.as_ref()
    }

    /// Admit the retained canonical authority without consulting a cached or
    /// reparsed legacy tree. Invalid documents remain retained for diagnostics
    /// but cannot publish resolver facts through this boundary.
    #[cfg(feature = "source")]
    pub fn canonical_document_index(&self) -> MResult<crate::CanonicalDocumentIndex> {
        self.source_document
            .as_ref()
            .ok_or_else(|| {
                MechError::new(
                    InvalidResolvedSourceError {
                        field: "source_document",
                        reason: "is required for canonical admission",
                    },
                    None,
                )
            })?
            .index()
            .map_err(|error| MechError::new(error, None))
    }

    /// Populate the resolver handoff solely from the retained canonical
    /// document. The legacy Program cache is neither read nor manufactured.
    #[cfg(feature = "source")]
    pub fn admit_canonical_document(mut self) -> MResult<Self> {
        let index = self.canonical_document_index()?;
        let root = index.root;
        let imports = root.all_imports();
        let referrer = self.canonical_uri.clone();
        self.dependencies = imports
            .iter()
            .map(|import| source_request_for_import(import, Some(&referrer)))
            .collect();
        self.exports = root.all_exports();
        self.contexts = root.all_contexts();
        self.address_references = root.all_address_references();
        self.scopes = root.module_scopes();
        self.imports = imports;
        Ok(self)
    }

    /// Admit one exact canonical revision and publish only the resolver facts
    /// owned by its root module scope. Mika-local scopes remain independently
    /// owned by the retained document and never leak into module resolution.
    #[cfg(feature = "source")]
    pub fn with_indexed_source_document(self, document: SourceDocument) -> MResult<Self> {
        self.with_source_document(document)?
            .admit_canonical_document()
    }

    /// Parse and retain this record's exact textual source under its canonical
    /// URI. This is the normal adoption point for product paths that construct
    /// `ResolvedSource` directly rather than through a resolver.
    #[cfg(feature = "source")]
    pub fn retain_source_document(
        self,
        revision: mech_syntax::document::Revision,
        config: mech_syntax::document::ParseConfig,
    ) -> MResult<Self> {
        let MechSourceCode::String(source) = &self.source else {
            return invalid_resolved_source("source_document", "requires textual Mech source");
        };
        let document =
            SourceDocument::parse_resolved(&self.canonical_uri, revision, source.as_str(), config)
                .map_err(|_| {
                    MechError::new(
                        InvalidResolvedSourceError {
                            field: "source",
                            reason: "exceeds the canonical retained-source range",
                        },
                        None,
                    )
                })?;
        self.with_source_document(document)
    }

    /// Replace the authoritative source and invalidate every projection that
    /// was derived from its previous contents.
    ///
    /// Replacing the source invalidates its retained document and every
    /// declaration projection.
    pub fn replace_source(&mut self, source: MechSourceCode) {
        #[cfg(feature = "source")]
        {
            self.source_document = None;
        }
        self.source = source;
        self.clear_source_projections();
    }

    fn clear_source_projections(&mut self) {
        self.imports.clear();
        self.exports.clear();
        self.contexts.clear();
        self.address_references.clear();
        self.scopes.clear();
    }

    pub fn with_kind(mut self, kind: SourceKind) -> Self {
        self.kind = kind;
        self
    }

    pub fn with_dependencies(mut self, dependencies: Vec<SourceRequest>) -> Self {
        self.dependencies = dependencies;
        self
    }

    pub fn with_imports(mut self, imports: Vec<SourceImportDeclaration>) -> Self {
        self.imports = imports;
        self
    }

    pub fn with_exports(mut self, exports: Vec<SourceExportDeclaration>) -> Self {
        self.exports = exports;
        self
    }

    pub fn with_contexts(mut self, contexts: Vec<SourceContextDeclaration>) -> Self {
        self.contexts = contexts;
        self
    }

    pub fn with_address_references(
        mut self,
        address_references: Vec<SourceAddressReference>,
    ) -> Self {
        self.address_references = address_references;
        self
    }

    pub fn with_scopes(mut self, scopes: Vec<ModuleScopeMetadata>) -> Self {
        self.scopes = scopes;
        self
    }

    pub fn with_capability_requirements(
        mut self,
        capability_requirements: Vec<CapabilityRequest>,
    ) -> Self {
        self.capability_requirements = capability_requirements;
        self
    }

    pub fn validate(&self) -> MResult<()> {
        if self.name.trim().is_empty() {
            return invalid_resolved_source("name", "must not be empty");
        }

        if self.canonical_uri.trim().is_empty() {
            return invalid_resolved_source("canonical_uri", "must not be empty");
        }

        #[cfg(feature = "source")]
        if let Some(document) = &self.source_document {
            self.validate_document_owner(document)?;
            match &self.source {
                MechSourceCode::String(source)
                    if source.as_str() == document.source().to_contiguous_string() => {}
                MechSourceCode::String(_) => {
                    return invalid_resolved_source(
                        "source_document",
                        "does not match the resolved source bytes",
                    );
                }
                _ => {
                    return invalid_resolved_source(
                        "source_document",
                        "requires textual Mech source",
                    );
                }
            }
        }

        for import in &self.imports {
            if import.specifier.trim().is_empty() {
                return invalid_resolved_source("imports.specifier", "must not be empty");
            }
        }

        for export in &self.exports {
            if export.name.trim().is_empty() {
                return invalid_resolved_source("exports.name", "must not be empty");
            }
        }

        for reference in &self.address_references {
            if reference.name.trim().is_empty() {
                return invalid_resolved_source("address_references.name", "must not be empty");
            }
            if reference.target.trim().is_empty() {
                return invalid_resolved_source("address_references.target", "must not be empty");
            }
        }

        for dependency in &self.dependencies {
            dependency.validate()?;
        }

        self.validate_address_targets()?;

        Ok(())
    }

    fn validate_address_targets(&self) -> MResult<()> {
        let mut targets: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();

        for metadata in &self.scopes {
            if let ModuleScopeMetadata {
                scope: SourceScope::Interpreter(interpreter),
                ..
            } = metadata
            {
                if let Some(first_kind) =
                    targets.insert(interpreter.namespace_str.clone(), "interpreter".to_string())
                {
                    return Err(MechError::new(
                        AddressTargetNameConflict {
                            name: interpreter.namespace_str.clone(),
                            first_kind,
                            second_kind: "interpreter".to_string(),
                        },
                        None,
                    ));
                }
            }

            for context in &metadata.contexts {
                if let Some(first_kind) =
                    targets.insert(context.name.clone(), "context".to_string())
                {
                    return Err(MechError::new(
                        AddressTargetNameConflict {
                            name: context.name.clone(),
                            first_kind,
                            second_kind: "context".to_string(),
                        },
                        None,
                    ));
                }
            }
        }

        Ok(())
    }

    pub fn is_executable_mech_source(&self) -> bool {
        self.kind.is_executable_mech()
            && matches!(
                self.source,
                MechSourceCode::String(_)
                    | MechSourceCode::ByteCode(_)
                    | MechSourceCode::Program(_)
            )
    }
}

// -----------------------------------------------------------------------------
// Resolver Traits
// -----------------------------------------------------------------------------

/// Resolves source-like assets from a request.
///
/// This trait is intentionally small. Filesystem, package-manager, database,
/// embedded, network, and editor/workspace resolvers should all implement this.
pub trait SourceResolver: std::fmt::Debug + Send {
    fn resolve(&self, request: &SourceRequest) -> MResult<Option<ResolvedSource>>;
}

/// Optional trait for resolvers that can accept in-memory source.
///
/// This is useful for editor buffers, tests, notebooks, and hosts that generate
/// source dynamically.
pub trait MutableSourceResolver: SourceResolver {
    fn insert_source(
        &mut self,
        specifier: impl Into<String>,
        source: ResolvedSource,
    ) -> MResult<()>;

    fn insert_string(
        &mut self,
        specifier: impl Into<String>,
        source: impl Into<String>,
    ) -> MResult<()>;
}

/// Optional trait for resolvers that can watch external sources.
///
/// Filesystem resolvers should implement this. Package, database, or editor
/// resolvers may also implement it later if they support change notifications.
pub trait WatchableSourceResolver: SourceResolver {
    fn watch_source(&mut self, specifier: &str) -> MResult<()>;
}

// -----------------------------------------------------------------------------
// Errors
// -----------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct AddressTargetNameConflict {
    pub name: String,
    pub first_kind: String,
    pub second_kind: String,
}

impl MechErrorKind for AddressTargetNameConflict {
    fn name(&self) -> &str {
        "AddressTargetNameConflict"
    }

    fn message(&self) -> String {
        format!(
            "address target `{}` is declared more than once as `{}` and `{}`",
            self.name, self.first_kind, self.second_kind,
        )
    }
}

#[derive(Debug, Clone)]
pub struct InvalidSourceRequestError {
    pub field: &'static str,
    pub reason: &'static str,
}

impl MechErrorKind for InvalidSourceRequestError {
    fn name(&self) -> &str {
        "InvalidSourceRequest"
    }

    fn message(&self) -> String {
        format!(
            "Invalid source request field `{}`: {}",
            self.field, self.reason
        )
    }
}

fn invalid_source_request<T>(field: &'static str, reason: &'static str) -> MResult<T> {
    Err(MechError::new(
        InvalidSourceRequestError { field, reason },
        None,
    ))
}

#[derive(Debug, Clone)]
pub struct InvalidResolvedSourceError {
    pub field: &'static str,
    pub reason: &'static str,
}

impl MechErrorKind for InvalidResolvedSourceError {
    fn name(&self) -> &str {
        "InvalidResolvedSource"
    }

    fn message(&self) -> String {
        format!(
            "Invalid resolved source field `{}`: {}",
            self.field, self.reason
        )
    }
}

fn invalid_resolved_source<T>(field: &'static str, reason: &'static str) -> MResult<T> {
    Err(MechError::new(
        InvalidResolvedSourceError { field, reason },
        None,
    ))
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_request_validates_nonempty_specifier() {
        let request = SourceRequest::new("");
        assert!(request.validate().is_err());
    }

    #[test]
    fn source_request_from_str() {
        let request = SourceRequest::from("main.mec");
        assert_eq!(request.specifier, "main.mec");
        assert_eq!(request.referrer, None);
        assert_eq!(request.kind_hint, None);
    }

    #[test]
    fn resolved_source_validates_identity_fields() {
        let source = ResolvedSource::new(
            "main",
            "memory:main",
            MechSourceCode::String("x := 1".to_string()),
        );

        assert!(source.validate().is_ok());
    }

    #[test]
    fn resolved_source_detects_executable_mech_source() {
        let source = ResolvedSource::new(
            "main",
            "memory:main",
            MechSourceCode::String("x := 1".to_string()),
        )
        .with_kind(SourceKind::Mech);

        assert!(source.is_executable_mech_source());
    }

    #[test]
    fn resolved_source_default_kind_is_not_executable() {
        let source = ResolvedSource::new(
            "main",
            "memory:main",
            MechSourceCode::String("x := 1".to_string()),
        );

        assert!(!source.is_executable_mech_source());
    }
}
