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
//! MechProgram should not own source resolution. MechProgram executes source
//! once it already has source. MechRuntime owns a SourceResolver and decides how
//! resolved sources are stored, checked, activated, and executed.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use mech_core::{MResult, MechError, MechErrorKind, MechSourceCode};

use crate::capability::CapabilityRequest;

// -----------------------------------------------------------------------------
// Submodules
// -----------------------------------------------------------------------------

pub mod memory;
pub mod source;
pub mod file;
pub mod imports;
pub mod ast;

pub use memory::*;
pub use source::*;
pub use file::*;
pub use imports::*;
pub use ast::*;

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
pub struct SourceImportDeclaration {
  pub specifier: String,
  pub alias: Option<String>,
  pub kind: SourceImportKind,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceExportDeclaration {
  pub name: String,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedSource {
  pub name: String,
  pub canonical_uri: String,
  pub source: MechSourceCode,
  pub kind: SourceKind,
  pub imports: Vec<SourceImportDeclaration>,
  pub exports: Vec<SourceExportDeclaration>,
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
      source,
      kind: SourceKind::Unknown("".to_string()),
      imports: Vec::new(),
      exports: Vec::new(),
      dependencies: Vec::new(),
      capability_requirements: Vec::new(),
    }
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

    for dependency in &self.dependencies {
      dependency.validate()?;
    }

    Ok(())
  }

  pub fn is_executable_mech_source(&self) -> bool {
    matches!(
      self.source,
      MechSourceCode::String(_)
        | MechSourceCode::Tree(_)
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
pub struct InvalidSourceRequestError {
  pub field: &'static str,
  pub reason: &'static str,
}

impl MechErrorKind for InvalidSourceRequestError {
  fn name(&self) -> &str {
    "InvalidSourceRequest"
  }

  fn message(&self) -> String {
    format!("Invalid source request field `{}`: {}", self.field, self.reason)
  }
}

fn invalid_source_request<T>(
  field: &'static str,
  reason: &'static str,
) -> MResult<T> {
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
    format!("Invalid resolved source field `{}`: {}", self.field, self.reason)
  }
}

fn invalid_resolved_source<T>(
  field: &'static str,
  reason: &'static str,
) -> MResult<T> {
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
    );

    assert!(source.is_executable_mech_source());
  }
}
