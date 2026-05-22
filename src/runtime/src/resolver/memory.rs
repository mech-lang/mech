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

use mech_core::{MResult, MechSourceCode};

use super::{
  MutableSourceResolver, ResolvedSource, SourceRequest, SourceResolver,
};

#[derive(Clone, Debug, Default)]
pub struct InMemorySourceResolver {
  sources: HashMap<String, ResolvedSource>,
  aliases: HashMap<String, String>,
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

    let resolved = ResolvedSource::new(
      specifier.clone(),
      Self::default_canonical_uri(&specifier),
      MechSourceCode::String(source.into()),
    );

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

  pub fn contains(&self, specifier: &str) -> bool {
    let resolved = self.resolve_alias(specifier);
    self.sources.contains_key(resolved)
  }

  pub fn remove(&mut self, specifier: &str) -> Option<ResolvedSource> {
    let resolved = self.resolve_alias(specifier).to_string();
    self.sources.remove(&resolved)
  }

  pub fn clear(&mut self) {
    self.sources.clear();
    self.aliases.clear();
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
}

impl SourceResolver for InMemorySourceResolver {
  fn resolve(&self, request: &SourceRequest) -> MResult<Option<ResolvedSource>> {
    request.validate()?;

    let specifier = self.resolve_alias(&request.specifier);

    Ok(self.sources.get(specifier).cloned())
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