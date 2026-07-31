//! In-memory source resolver.
//!
//! This resolver is useful for:
//!
//! - tests
//! - REPL sessions
//! - generated source
//! - editor buffers
//! - notebooks
//! - simple embedded hosts
//!
//! It does not read from the filesystem, package manager, database, or network.
//! It only resolves sources explicitly inserted into it.

use std::collections::HashMap;

use mech_core::{MResult, MechError, MechErrorKind, MechSourceCode};

use super::{
  MutableSourceResolver, ResolvedSource, SourceKind, SourceRequest,
  SourceResolver,
};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct InMemoryResolutionKey {
  referrer_canonical_uri: String,
  requested_specifier: String,
}

#[derive(Clone, Debug)]
pub struct InMemorySourceResolutionConflict {
  pub referrer_canonical_uri: String,
  pub requested_specifier: String,
  pub existing_target: String,
  pub requested_target: String,
}

impl MechErrorKind for InMemorySourceResolutionConflict {
  fn name(&self) -> &str { "InMemorySourceResolutionConflict" }

  fn message(&self) -> String {
    format!(
      "in-memory source resolution `{}` from `{}` already targets `{}` and cannot target `{}`",
      self.requested_specifier,
      self.referrer_canonical_uri,
      self.existing_target,
      self.requested_target,
    )
  }
}

#[derive(Clone, Debug)]
pub struct InMemorySourceResolutionTargetMissing {
  pub role: &'static str,
  pub source: String,
}

impl MechErrorKind for InMemorySourceResolutionTargetMissing {
  fn name(&self) -> &str { "InMemorySourceResolutionTargetMissing" }

  fn message(&self) -> String {
    format!(
      "in-memory source resolution {} source `{}` is not registered",
      self.role,
      self.source,
    )
  }
}

#[derive(Clone, Debug, Default)]
pub struct InMemorySourceResolver {
  sources: HashMap<String, ResolvedSource>,
  aliases: HashMap<String, String>,
  resolutions: HashMap<InMemoryResolutionKey, String>,
}

impl InMemorySourceResolver {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn insert_source(
    &mut self,
    specifier: impl Into<String>,
    source: ResolvedSource,
  ) -> MResult<()> {
    let specifier = specifier.into();

    source.validate()?;

    self.sources.insert(specifier, source);
    Ok(())
  }

  pub fn insert_string(
    &mut self,
    specifier: impl Into<String>,
    source: impl Into<String>,
  ) -> MResult<()> {
    let specifier = specifier.into();
    let source = source.into();

    let resolved = ResolvedSource::new(
      specifier.clone(),
      Self::default_canonical_uri(&specifier),
      MechSourceCode::String(source),
    )
    .with_kind(SourceKind::Mech);

    self.insert_source(specifier, resolved)
  }

  pub fn with_string(
    mut self,
    specifier: impl Into<String>,
    source: impl Into<String>,
  ) -> Self {
    let _ = self.insert_string(specifier, source);
    self
  }

  pub fn with_source(
    mut self,
    specifier: impl Into<String>,
    source: ResolvedSource,
  ) -> Self {
    let _ = self.insert_source(specifier, source);
    self
  }

  pub fn with_alias(
    mut self,
    alias: impl Into<String>,
    target: impl Into<String>,
  ) -> Self {
    self.aliases.insert(alias.into(), target.into());
    self
  }

  pub fn insert_resolution(
    &mut self,
    referrer_source: impl AsRef<str>,
    requested_specifier: impl Into<String>,
    target_source: impl AsRef<str>,
  ) -> MResult<()> {
    let referrer_source = self.resolve_alias(referrer_source.as_ref()).to_string();
    let target_source = self.resolve_alias(target_source.as_ref()).to_string();
    let requested_specifier = requested_specifier.into();

    if requested_specifier.trim().is_empty() {
      return Err(MechError::new(
        super::InvalidSourceRequestError {
          field: "specifier",
          reason: "must not be empty",
        },
        None,
      ));
    }

    let referrer = self.sources.get(&referrer_source).ok_or_else(|| MechError::new(
      InMemorySourceResolutionTargetMissing {
        role: "referrer",
        source: referrer_source.clone(),
      },
      None,
    ))?;
    if !self.sources.contains_key(&target_source) {
      return Err(MechError::new(
        InMemorySourceResolutionTargetMissing {
          role: "target",
          source: target_source,
        },
        None,
      ));
    }

    let key = InMemoryResolutionKey {
      referrer_canonical_uri: referrer.canonical_uri.clone(),
      requested_specifier,
    };
    if let Some(existing_target) = self.resolutions.get(&key) {
      if existing_target == &target_source {
        return Ok(());
      }
      return Err(MechError::new(
        InMemorySourceResolutionConflict {
          referrer_canonical_uri: key.referrer_canonical_uri,
          requested_specifier: key.requested_specifier,
          existing_target: existing_target.clone(),
          requested_target: target_source,
        },
        None,
      ));
    }
    self.resolutions.insert(key, target_source);
    Ok(())
  }

  pub fn contains(&self, specifier: &str) -> bool {
    let resolved = self.resolve_alias(specifier);
    self.sources.contains_key(resolved)
  }

  pub fn remove(&mut self, specifier: &str) -> Option<ResolvedSource> {
    let resolved = self.resolve_alias(specifier).to_string();
    let removed = self.sources.remove(&resolved);
    if let Some(source) = &removed {
      self.resolutions.retain(|key, target| {
        key.referrer_canonical_uri != source.canonical_uri && target != &resolved
      });
    }
    removed
  }

  pub fn clear(&mut self) {
    self.sources.clear();
    self.aliases.clear();
    self.resolutions.clear();
  }

  pub fn len(&self) -> usize {
    self.sources.len()
  }

  pub fn is_empty(&self) -> bool {
    self.sources.is_empty()
  }

  pub fn specifiers(&self) -> impl Iterator<Item = &String> {
    self.sources.keys()
  }

  pub fn aliases(&self) -> impl Iterator<Item = (&String, &String)> {
    self.aliases.iter()
  }

  fn resolve_alias<'a>(&'a self, specifier: &'a str) -> &'a str {
    self
      .aliases
      .get(specifier)
      .map(|target| target.as_str())
      .unwrap_or(specifier)
  }

  fn default_canonical_uri(specifier: &str) -> String {
    format!("memory:{}", specifier)
  }

  fn relative_candidate(
    specifier: &str,
    referrer: Option<&str>,
  ) -> Option<String> {
    if !(specifier.starts_with("./") || specifier.starts_with("../")) {
      return None;
    }

    let referrer = referrer?.strip_prefix("memory:")?;
    let referrer = referrer.strip_prefix("//").unwrap_or(referrer);
    let base = referrer.rsplit_once('/').map(|(base, _)| base).unwrap_or("");
    let mut parts = Vec::new();

    for segment in base.split('/').chain(specifier.split('/')) {
      match segment {
        "" | "." => {}
        ".." => {
          parts.pop()?;
        }
        segment => parts.push(segment),
      }
    }

    Some(parts.join("/"))
  }
}

impl SourceResolver for InMemorySourceResolver {
  fn resolve(&self, request: &SourceRequest) -> MResult<Option<ResolvedSource>> {
    request.validate()?;

    if let Some(referrer_canonical_uri) = request.referrer.as_ref() {
      let key = InMemoryResolutionKey {
        referrer_canonical_uri: referrer_canonical_uri.clone(),
        requested_specifier: request.specifier.clone(),
      };
      if let Some(target) = self.resolutions.get(&key) {
        return Ok(self.sources.get(target).cloned());
      }
    }

    if let Some(source) = self.sources.get(&request.specifier) {
      return Ok(Some(source.clone()));
    }

    if let Some(source) = self.sources.get(self.resolve_alias(&request.specifier)) {
      return Ok(Some(source.clone()));
    }

    let Some(candidate) = Self::relative_candidate(
      &request.specifier,
      request.referrer.as_deref(),
    ) else {
      return Ok(None);
    };

    if let Some(source) = self
      .sources
      .get(self.resolve_alias(&candidate))
    {
      return Ok(Some(source.clone()));
    }

    Ok(None)
  }
}

impl MutableSourceResolver for InMemorySourceResolver {
  fn insert_source(
    &mut self,
    specifier: impl Into<String>,
    source: ResolvedSource,
  ) -> MResult<()> {
    InMemorySourceResolver::insert_source(self, specifier, source)
  }

  fn insert_string(
    &mut self,
    specifier: impl Into<String>,
    source: impl Into<String>,
  ) -> MResult<()> {
    InMemorySourceResolver::insert_string(self, specifier, source)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn resolves_inserted_string() {
    let mut resolver = InMemorySourceResolver::new();

    resolver
      .insert_string("main.mec", "x := 1")
      .unwrap();

    let request = SourceRequest::new("main.mec");
    let resolved = resolver.resolve(&request).unwrap().unwrap();

    assert_eq!(resolved.name, "main.mec");
    assert_eq!(resolved.canonical_uri, "memory:main.mec");
    assert!(resolved.is_executable_mech_source());
  }

  #[test]
  fn returns_none_for_missing_source() {
    let resolver = InMemorySourceResolver::new();

    let request = SourceRequest::new("missing.mec");
    let resolved = resolver.resolve(&request).unwrap();

    assert!(resolved.is_none());
  }

  #[test]
  fn supports_builder_style_insert() {
    let resolver = InMemorySourceResolver::new()
      .with_string("main.mec", "x := 1");

    let request = SourceRequest::new("main.mec");
    let resolved = resolver.resolve(&request).unwrap().unwrap();

    assert_eq!(resolved.name, "main.mec");
  }

  #[test]
  fn supports_aliases() {
    let resolver = InMemorySourceResolver::new()
      .with_string("main.mec", "x := 1")
      .with_alias("main", "main.mec");

    let request = SourceRequest::new("main");
    let resolved = resolver.resolve(&request).unwrap().unwrap();

    assert_eq!(resolved.name, "main.mec");
    assert_eq!(resolved.canonical_uri, "memory:main.mec");
  }

  #[test]
  fn resolves_memory_relative_imports() {
    let resolver = InMemorySourceResolver::new()
      .with_string("lib.mec", "x := 1")
      .with_string("app/lib.mec", "x := 1")
      .with_string("shared/lib.mec", "x := 1")
      .with_string("shared/deep/lib.mec", "x := 1");

    for (specifier, referrer, expected) in [
      ("./lib.mec", "memory:main.mec", "lib.mec"),
      ("./lib.mec", "memory:app/main.mec", "app/lib.mec"),
      ("../shared/lib.mec", "memory:app/main.mec", "shared/lib.mec"),
      ("../../shared/deep/lib.mec", "memory:app/nested/main.mec", "shared/deep/lib.mec"),
    ] {
      let request = SourceRequest::new(specifier).with_referrer(referrer);
      assert_eq!(resolver.resolve(&request).unwrap().unwrap().name, expected);
    }
  }

  #[test]
  fn in_memory_resolution_edge_resolves_exact_request() {
    let mut resolver = InMemorySourceResolver::new()
      .with_string("root", "x := 1")
      .with_string("dependency", "value := 41");
    resolver.insert_resolution("root", "./literal spelling", "dependency").unwrap();

    let resolved = resolver.resolve(
      &SourceRequest::new("./literal spelling").with_referrer("memory:root"),
    ).unwrap().unwrap();

    assert_eq!(resolved.name, "dependency");
  }

  #[test]
  fn in_memory_resolution_edge_uses_referrer_canonical_identity() {
    let mut resolver = InMemorySourceResolver::new();
    resolver.insert_source(
      "root-key",
      ResolvedSource::new(
        "logical-root",
        "document:canonical-root",
        MechSourceCode::String("x := 1".to_string()),
      ).with_kind(SourceKind::Mech),
    ).unwrap();
    resolver.insert_string("target", "value := 41").unwrap();
    resolver.insert_resolution("root-key", "./dep", "target").unwrap();

    assert!(resolver.resolve(
      &SourceRequest::new("./dep").with_referrer("memory:root-key"),
    ).unwrap().is_none());
    assert_eq!(resolver.resolve(
      &SourceRequest::new("./dep").with_referrer("document:canonical-root"),
    ).unwrap().unwrap().name, "target");
  }

  #[test]
  fn in_memory_resolution_edge_precedes_global_exact_source() {
    let mut resolver = InMemorySourceResolver::new()
      .with_string("root", "x := 1")
      .with_string("./dep", "value := 0")
      .with_string("edge-target", "value := 41");
    resolver.insert_resolution("root", "./dep", "edge-target").unwrap();

    let resolved = resolver.resolve(
      &SourceRequest::new("./dep").with_referrer("memory:root"),
    ).unwrap().unwrap();

    assert_eq!(resolved.name, "edge-target");
  }

  #[test]
  fn in_memory_resolution_edge_reuses_target_canonical_uri() {
    let mut resolver = InMemorySourceResolver::new().with_string("root", "x := 1");
    resolver.insert_source(
      "target-key",
      ResolvedSource::new(
        "target-name",
        "document:canonical-target",
        MechSourceCode::String("value := 41".to_string()),
      ).with_kind(SourceKind::Mech),
    ).unwrap();
    resolver.insert_resolution("root", "./dep", "target-key").unwrap();

    let resolved = resolver.resolve(
      &SourceRequest::new("./dep").with_referrer("memory:root"),
    ).unwrap().unwrap();

    assert_eq!(resolved.name, "target-name");
    assert_eq!(resolved.canonical_uri, "document:canonical-target");
  }

  #[test]
  fn in_memory_resolution_edge_rejects_conflicting_target() {
    let mut resolver = InMemorySourceResolver::new()
      .with_string("root", "x := 1")
      .with_string("first", "value := 1")
      .with_string("second", "value := 2");
    resolver.insert_resolution("root", "./dep", "first").unwrap();
    resolver.insert_resolution("root", "./dep", "first").unwrap();

    let error = resolver.insert_resolution("root", "./dep", "second").unwrap_err();
    assert_eq!(error.kind_name(), "InMemorySourceResolutionConflict");
  }

  #[test]
  fn in_memory_resolution_edge_rejects_dangling_source() {
    let mut resolver = InMemorySourceResolver::new()
      .with_string("root", "x := 1")
      .with_string("target", "value := 1");

    let missing_referrer = resolver
      .insert_resolution("missing", "./dep", "target")
      .unwrap_err();
    let missing_target = resolver
      .insert_resolution("root", "./dep", "missing")
      .unwrap_err();

    assert_eq!(missing_referrer.kind_name(), "InMemorySourceResolutionTargetMissing");
    assert_eq!(missing_target.kind_name(), "InMemorySourceResolutionTargetMissing");
  }

  #[test]
  fn ordinary_memory_relative_resolution_remains_available() {
    let resolver = InMemorySourceResolver::new()
      .with_string("app/dep.mec", "value := 41");

    let resolved = resolver.resolve(
      &SourceRequest::new("./dep.mec").with_referrer("memory:app/main.mec"),
    ).unwrap().unwrap();

    assert_eq!(resolved.name, "app/dep.mec");
  }

  #[test]
  fn ascii_relative_import_behavior_is_unchanged() {
    let resolver = InMemorySourceResolver::new()
      .with_string("app/dep.mec", "value := 41");

    let request = SourceRequest::new("./dep.mec")
      .with_referrer("memory:app/main.mec");
    let resolved = resolver.resolve(&request).unwrap().unwrap();

    assert_eq!(resolved.name, "app/dep.mec");
  }

  #[test]
  fn memory_relative_imports_do_not_escape_or_rebase_other_requests() {
    let resolver = InMemorySourceResolver::new()
      .with_string("lib.mec", "x := 1")
      .with_string("dep.mec", "x := 1");

    for request in [
      SourceRequest::new("../../lib.mec").with_referrer("memory:main.mec"),
      SourceRequest::new("./lib.mec").with_referrer("file:///app/main.mec"),
      SourceRequest::new("other.mec").with_referrer("memory:app/main.mec"),
    ] {
      assert!(resolver.resolve(&request).unwrap().is_none());
    }
  }

  #[test]
  fn aliases_apply_to_normalized_memory_relative_imports() {
    let resolver = InMemorySourceResolver::new()
      .with_string("lib.mec", "x := 1")
      .with_alias("app/lib.mec", "lib.mec");

    let request = SourceRequest::new("./lib.mec")
      .with_referrer("memory:app/main.mec");
    assert_eq!(resolver.resolve(&request).unwrap().unwrap().name, "lib.mec");
  }

  #[test]
  fn remove_deletes_source() {
    let mut resolver = InMemorySourceResolver::new()
      .with_string("main.mec", "x := 1");

    assert!(resolver.contains("main.mec"));

    let removed = resolver.remove("main.mec");

    assert!(removed.is_some());
    assert!(!resolver.contains("main.mec"));
  }

  #[test]
  fn insert_source_validates_resolved_source() {
    let mut resolver = InMemorySourceResolver::new();

    let bad = ResolvedSource::new(
      "",
      "memory:bad",
      MechSourceCode::String("x := 1".to_string()),
    );

    assert!(resolver.insert_source("bad", bad).is_err());
  }

  #[test]
  fn len_and_is_empty_work() {
    let mut resolver = InMemorySourceResolver::new();

    assert!(resolver.is_empty());
    assert_eq!(resolver.len(), 0);

    resolver
      .insert_string("main.mec", "x := 1")
      .unwrap();

    assert!(!resolver.is_empty());
    assert_eq!(resolver.len(), 1);
  }
}
