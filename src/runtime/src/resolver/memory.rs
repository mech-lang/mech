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

use std::collections::{BTreeMap, BTreeSet, HashMap};

use mech_core::{MResult, MechError, MechErrorKind, MechSourceCode};

#[cfg(feature = "source")]
use super::SourceDocument;
use super::{MutableSourceResolver, ResolvedSource, SourceKind, SourceRequest, SourceResolver};
#[cfg(feature = "source")]
use mech_syntax::document::{ParseConfig, Revision};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct InMemoryResolutionKey {
    referrer_canonical_uri: String,
    requested_specifier: String,
}

/// One authoritative source-resolution edge for a detached source graph.
///
/// The edge records the exact request spelling used by `referrer` and the
/// registered source identity selected by the resolver. Consumers should not
/// reconstruct extension, index, symlink, or other resolver behavior from the
/// three source names.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SourceResolutionEntry {
    pub referrer: String,
    pub specifier: String,
    pub target: String,
}

impl SourceResolutionEntry {
    pub fn new(
        referrer: impl Into<String>,
        specifier: impl Into<String>,
        target: impl Into<String>,
    ) -> Self {
        Self {
            referrer: referrer.into(),
            specifier: specifier.into(),
            target: target.into(),
        }
    }

    pub fn validate(&self) -> MResult<()> {
        for (field, value) in [
            ("referrer", self.referrer.as_str()),
            ("specifier", self.specifier.as_str()),
            ("target", self.target.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(MechError::new(
                    SourceResolutionGraphInvalid {
                        reason: format!("resolution `{field}` must not be empty"),
                    },
                    None,
                ));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct SourceResolutionGraphInvalid {
    pub reason: String,
}

impl MechErrorKind for SourceResolutionGraphInvalid {
    fn name(&self) -> &str {
        "SourceResolutionGraphInvalid"
    }

    fn message(&self) -> String {
        format!("source resolution graph is invalid: {}", self.reason)
    }
}

/// Validates detached source identities and their authoritative resolution
/// edges before they cross a transport boundary.
pub fn validate_source_resolution_entries<'a>(
    sources: impl IntoIterator<Item = &'a str>,
    resolutions: &[SourceResolutionEntry],
) -> MResult<()> {
    let sources = sources.into_iter().collect::<BTreeSet<_>>();
    let mut unique = BTreeMap::<(&str, &str), &str>::new();

    for resolution in resolutions {
        resolution.validate()?;
        if !sources.contains(resolution.referrer.as_str()) {
            return Err(MechError::new(
                SourceResolutionGraphInvalid {
                    reason: format!(
                        "resolution referrer `{}` is not a registered source",
                        resolution.referrer,
                    ),
                },
                None,
            ));
        }
        if !sources.contains(resolution.target.as_str()) {
            return Err(MechError::new(
                SourceResolutionGraphInvalid {
                    reason: format!(
                        "resolution target `{}` is not a registered source",
                        resolution.target,
                    ),
                },
                None,
            ));
        }

        let key = (resolution.referrer.as_str(), resolution.specifier.as_str());
        if let Some(existing) = unique.get(&key) {
            if *existing != resolution.target {
                return Err(MechError::new(
                    SourceResolutionGraphInvalid {
                        reason: format!(
                            "resolution `{}` from `{}` conflicts: `{existing}` and `{}`",
                            resolution.specifier, resolution.referrer, resolution.target,
                        ),
                    },
                    None,
                ));
            }
            continue;
        }
        unique.insert(key, resolution.target.as_str());
    }

    Ok(())
}

#[derive(Clone, Debug)]
pub struct InMemorySourceResolutionConflict {
    pub referrer_canonical_uri: String,
    pub requested_specifier: String,
    pub existing_target: String,
    pub requested_target: String,
}

impl MechErrorKind for InMemorySourceResolutionConflict {
    fn name(&self) -> &str {
        "InMemorySourceResolutionConflict"
    }

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
    fn name(&self) -> &str {
        "InMemorySourceResolutionTargetMissing"
    }

    fn message(&self) -> String {
        format!(
            "in-memory source resolution {} source `{}` is not registered",
            self.role, self.source,
        )
    }
}

#[cfg(feature = "source")]
type SourceRevisionHistory = HashMap<String, BTreeMap<Revision, SourceDocument>>;

/// Clones have independent source entries but share accepted revision history.
/// Removing or clearing entries does not release their document identities.
#[derive(Clone, Debug, Default)]
pub struct InMemorySourceResolver {
    sources: HashMap<String, ResolvedSource>,
    // Accepted identity history outlives entry deletion and is shared by clones.
    #[cfg(feature = "source")]
    source_revisions: std::sync::Arc<std::sync::Mutex<SourceRevisionHistory>>,
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
        #[cfg(feature = "source")]
        {
            let mut history = self
                .source_revisions
                .lock()
                .map_err(|_| Self::revision_error("revision history is unavailable"))?;
            Self::remember_source_revision(&mut history, &source)?;
        }
        self.sources.insert(specifier, source);
        Ok(())
    }

    #[cfg(feature = "source")]
    fn revision_error(reason: &'static str) -> MechError {
        MechError::new(
            super::InvalidResolvedSourceError {
                field: "source_document.revision",
                reason,
            },
            None,
        )
    }

    #[cfg(feature = "source")]
    fn remember_source_revision(
        history: &mut SourceRevisionHistory,
        source: &ResolvedSource,
    ) -> MResult<()> {
        if let Some(document) = source.source_document() {
            let revisions = history.entry(source.canonical_uri.clone()).or_default();
            let revision = document.source().revision();
            if revisions
                .get(&revision)
                .is_some_and(|known| known != document)
            {
                return Err(Self::revision_error(
                    "revision already identifies another document",
                ));
            }
            revisions
                .entry(revision)
                .or_insert_with(|| document.clone());
        }
        Ok(())
    }

    #[cfg(feature = "source")]
    fn insert_prepared_string(
        &mut self,
        specifier: String,
        source: String,
        admit: impl FnOnce(ResolvedSource) -> MResult<ResolvedSource>,
    ) -> MResult<()> {
        let uri = Self::default_canonical_uri(&specifier);
        let shared_history = self.source_revisions.clone();
        // Selection, candidate admission and publication are one operation across
        // clones. Failed admission leaves both the entry and history unchanged.
        let mut history = shared_history
            .lock()
            .map_err(|_| Self::revision_error("revision history is unavailable"))?;
        let revision = match history
            .get(&uri)
            .and_then(|revisions| revisions.last_key_value())
        {
            None => Revision(0),
            Some((previous, _)) => previous
                .0
                .checked_add(1)
                .map(Revision)
                .ok_or_else(|| Self::revision_error("revision identity is exhausted"))?,
        };
        let resolved = ResolvedSource::new(specifier.clone(), uri, MechSourceCode::String(source))
            .with_kind(SourceKind::Mech)
            .retain_source_document(revision, ParseConfig::default())?;
        let resolved = admit(resolved)?;
        resolved.validate()?;
        Self::remember_source_revision(&mut history, &resolved)?;
        self.sources.insert(specifier, resolved);
        Ok(())
    }

    pub fn insert_string(
        &mut self,
        specifier: impl Into<String>,
        source: impl Into<String>,
    ) -> MResult<()> {
        let specifier = specifier.into();
        let source = source.into();
        #[cfg(feature = "source")]
        {
            self.insert_prepared_string(specifier, source, ResolvedSource::admit_canonical_document)
        }
        #[cfg(not(feature = "source"))]
        self.insert_source(
            specifier.clone(),
            ResolvedSource::new(
                specifier.clone(),
                Self::default_canonical_uri(&specifier),
                MechSourceCode::String(source),
            )
            .with_kind(SourceKind::Mech),
        )
    }

    /// Insert one strictly admitted canonical revision without constructing a
    /// competing legacy Program tree. Candidate validation completes before
    /// the previous accepted source can be replaced.
    #[cfg(feature = "source")]
    pub fn insert_canonical_string(
        &mut self,
        specifier: impl Into<String>,
        source: impl Into<String>,
    ) -> MResult<()> {
        self.insert_prepared_string(
            specifier.into(),
            source.into(),
            ResolvedSource::admit_canonical_document,
        )
    }

    pub fn with_string(mut self, specifier: impl Into<String>, source: impl Into<String>) -> Self {
        let specifier = specifier.into();
        let source = source.into();
        #[cfg(feature = "source")]
        let result = self.insert_prepared_string(specifier, source, Ok);
        #[cfg(not(feature = "source"))]
        let result = self.insert_string(specifier, source);
        if result.is_err() {
            return self;
        }
        self
    }

    /// Retain one canonical revision for later diagnostics without claiming
    /// strict admission. Rejected identity allocation preserves the old entry.
    #[cfg(feature = "source")]
    pub fn with_canonical_string(
        mut self,
        specifier: impl Into<String>,
        source: impl Into<String>,
    ) -> Self {
        if self
            .insert_prepared_string(specifier.into(), source.into(), Ok)
            .is_err()
        {
            return self;
        }
        self
    }

    pub fn try_with_source(
        mut self,
        specifier: impl Into<String>,
        source: ResolvedSource,
    ) -> MResult<Self> {
        self.insert_source(specifier, source)?;
        Ok(self)
    }

    pub fn with_source(mut self, specifier: impl Into<String>, source: ResolvedSource) -> Self {
        if self.insert_source(specifier, source).is_err() {
            // Preserve source compatibility for the historical infallible
            // builder. New callers that need validation use try_with_source.
            return self;
        }
        self
    }

    pub fn with_alias(mut self, alias: impl Into<String>, target: impl Into<String>) -> Self {
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

        let referrer = self.sources.get(&referrer_source).ok_or_else(|| {
            MechError::new(
                InMemorySourceResolutionTargetMissing {
                    role: "referrer",
                    source: referrer_source.clone(),
                },
                None,
            )
        })?;
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

    pub fn insert_resolution_entry(&mut self, resolution: &SourceResolutionEntry) -> MResult<()> {
        resolution.validate()?;
        self.insert_resolution(
            &resolution.referrer,
            resolution.specifier.clone(),
            &resolution.target,
        )
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
        self.aliases
            .get(specifier)
            .map(|target| target.as_str())
            .unwrap_or(specifier)
    }

    fn default_canonical_uri(specifier: &str) -> String {
        format!("memory:{}", specifier)
    }

    fn relative_candidate(specifier: &str, referrer: Option<&str>) -> Option<String> {
        if !(specifier.starts_with("./") || specifier.starts_with("../")) {
            return None;
        }

        let referrer = referrer?.strip_prefix("memory:")?;
        let referrer = referrer.strip_prefix("//").unwrap_or(referrer);
        let base = referrer
            .rsplit_once('/')
            .map(|(base, _)| base)
            .unwrap_or("");
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

        let Some(candidate) =
            Self::relative_candidate(&request.specifier, request.referrer.as_deref())
        else {
            return Ok(None);
        };

        if let Some(source) = self.sources.get(self.resolve_alias(&candidate)) {
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

    #[cfg(feature = "source")]
    #[test]
    fn cloned_resolvers_allocate_concurrent_revisions_atomically() {
        let resolver =
            InMemorySourceResolver::new().with_canonical_string("main.mec", "value := 0\n");
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));
        let handles = (1..=8)
            .map(|value| {
                let mut resolver = resolver.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    let text = format!("value := {value}\n");
                    resolver.insert_canonical_string("main.mec", &text).unwrap();
                    let resolved = resolver
                        .resolve(&SourceRequest::new("main.mec"))
                        .unwrap()
                        .unwrap();
                    let document = resolved.source_document().unwrap();
                    assert_eq!(document.source().to_contiguous_string(), text);
                    document.source().revision()
                })
            })
            .collect::<Vec<_>>();
        let mut revisions = handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect::<Vec<_>>();
        revisions.sort();
        assert_eq!(revisions, (1..=8).map(Revision).collect::<Vec<_>>());
    }

    #[cfg(feature = "source")]
    #[test]
    fn incoming_memory_documents_reserve_uri_history_and_reject_identity_aliases() {
        let uri = "memory:main.mec";
        let original = ResolvedSource::new(
            "main.mec",
            uri,
            MechSourceCode::String("value := 1\n".into()),
        )
        .with_kind(SourceKind::Mech)
        .retain_source_document(Revision(7), ParseConfig::default())
        .unwrap();
        let mut resolver = InMemorySourceResolver::new();
        resolver
            .insert_source("different-entry", original.clone())
            .unwrap();
        resolver.clear();
        let conflicting = ResolvedSource::new(
            "main.mec",
            uri,
            MechSourceCode::String("value := 2\n".into()),
        )
        .with_kind(SourceKind::Mech)
        .retain_source_document(Revision(7), ParseConfig::default())
        .unwrap();
        assert!(resolver.insert_source("main.mec", conflicting).is_err());
        resolver.insert_source("historical", original).unwrap();
        let shipping = resolver.clone().with_string("main.mec", "value := 3\n");
        let canonical = resolver.with_canonical_string("main.mec", "value := [\n");
        for (resolver, revision) in [(shipping, 8), (canonical, 9)] {
            let resolved = resolver
                .resolve(&SourceRequest::new("main.mec"))
                .unwrap()
                .unwrap();
            assert_eq!(
                resolved.source_document().unwrap().source().revision(),
                Revision(revision)
            );
        }
    }

    #[cfg(feature = "source")]
    #[test]
    fn revision_history_survives_deletion_and_diverging_clones() {
        for canonical in [false, true] {
            let mut resolver = InMemorySourceResolver::new();
            let insert = |resolver: &mut InMemorySourceResolver, value| {
                let source = format!("value := {value}\n");
                if canonical {
                    resolver.insert_canonical_string("main.mec", source)
                } else {
                    resolver.insert_string("main.mec", source)
                }
                .unwrap();
                resolver
                    .resolve(&SourceRequest::new("main.mec"))
                    .unwrap()
                    .unwrap()
            };
            let first = insert(&mut resolver, 0);
            resolver.remove("main.mec");
            let second = insert(&mut resolver, 1);
            resolver.clear();
            let third = insert(&mut resolver, 2);
            let mut clone = resolver.clone();
            let fourth = insert(&mut resolver, 3);
            let fifth = insert(&mut clone, 4);
            for (revision, resolved) in [first, second, third, fourth, fifth].iter().enumerate() {
                assert_eq!(
                    resolved.source_document().unwrap().source().revision(),
                    Revision(revision as u64)
                );
                assert_eq!(
                    resolved
                        .source_document()
                        .unwrap()
                        .source()
                        .to_contiguous_string(),
                    format!("value := {revision}\n")
                );
            }
        }
    }

    #[test]
    fn resolves_inserted_string() {
        let mut resolver = InMemorySourceResolver::new();

        resolver.insert_string("main.mec", "  x := 1\r\n").unwrap();

        let request = SourceRequest::new("main.mec");
        let resolved = resolver.resolve(&request).unwrap().unwrap();

        assert_eq!(resolved.name, "main.mec");
        assert_eq!(resolved.canonical_uri, "memory:main.mec");
        assert!(resolved.is_executable_mech_source());
        #[cfg(feature = "source")]
        assert_eq!(
            resolved
                .source_document()
                .unwrap()
                .source()
                .to_contiguous_string(),
            "  x := 1\r\n"
        );
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
        let resolver = InMemorySourceResolver::new().with_string("main.mec", "x := 1");

        let request = SourceRequest::new("main.mec");
        let resolved = resolver.resolve(&request).unwrap().unwrap();

        assert_eq!(resolved.name, "main.mec");
    }

    #[cfg(feature = "source")]
    #[test]
    fn insert_string_strictly_admits_one_retained_document() {
        let source = r#"delta := 0.25
rows := |id<string> x<f64>|
  | "row-a" 1 + delta |
  | "row-b" 2 + delta |"#;
        let mut resolver = InMemorySourceResolver::new();

        resolver.insert_string("table.mec", source).unwrap();

        let resolved = resolver
            .resolve(&SourceRequest::new("table.mec"))
            .unwrap()
            .unwrap();
        assert_eq!(
            resolved
                .source_document()
                .unwrap()
                .source()
                .to_contiguous_string(),
            source
        );
        assert!(resolved.canonical_document_index().is_ok());
    }

    #[cfg(feature = "source")]
    #[test]
    fn builder_preserves_invalid_source_for_canonical_parse_diagnostics() {
        let resolver = InMemorySourceResolver::new().with_string("broken.mec", "x := [");

        let resolved = resolver
            .resolve(&SourceRequest::new("broken.mec"))
            .unwrap()
            .expect("malformed source must remain resolvable");

        let document = resolved
            .source_document()
            .expect("malformed source keeps its canonical diagnostic owner");
        assert!(!document.is_strictly_clean());
        assert!(!document.snapshot().diagnostics.is_empty());
        assert!(matches!(
            resolved.source,
            MechSourceCode::String(ref source) if source == "x := ["
        ));
    }

    #[cfg(feature = "source")]
    #[test]
    fn malformed_replacement_is_transactional_and_preserves_the_prior_revision() {
        let mut resolver = InMemorySourceResolver::new();
        resolver.insert_string("main.mec", "value := 1\n").unwrap();
        let before = resolver
            .resolve(&SourceRequest::new("main.mec"))
            .unwrap()
            .unwrap();
        assert!(resolver.insert_string("main.mec", "value := [\n").is_err());
        let after = resolver
            .resolve(&SourceRequest::new("main.mec"))
            .unwrap()
            .unwrap();
        assert_eq!(after.source, before.source);
        assert_eq!(
            after.source_document().unwrap().source().revision(),
            before.source_document().unwrap().source().revision(),
        );
        assert!(std::ptr::eq(
            after.source_document().unwrap().snapshot(),
            before.source_document().unwrap().snapshot(),
        ));
    }

    #[cfg(feature = "source")]
    #[test]
    fn successful_replacement_advances_the_retained_revision() {
        let mut resolver = InMemorySourceResolver::new();
        resolver.insert_string("main.mec", "value := 1\n").unwrap();
        let before = resolver
            .resolve(&SourceRequest::new("main.mec"))
            .unwrap()
            .unwrap();
        resolver.insert_string("main.mec", "value := 2\n").unwrap();
        let after = resolver
            .resolve(&SourceRequest::new("main.mec"))
            .unwrap()
            .unwrap();

        let before = before.source_document().unwrap();
        let after = after.source_document().unwrap();
        assert_eq!(before.source().document(), after.source().document());
        assert_eq!(before.source().revision(), Revision(0));
        assert_eq!(after.source().revision(), Revision(1));
        assert_eq!(before.source().to_contiguous_string(), "value := 1\n");
        assert_eq!(after.source().to_contiguous_string(), "value := 2\n");
        assert!(before.index().is_ok());
        assert!(after.index().is_ok());
    }

    #[cfg(feature = "source")]
    #[test]
    fn canonical_insertion_fails_closed_before_replacing_the_accepted_revision() {
        let mut resolver = InMemorySourceResolver::new();
        resolver
            .insert_canonical_string("main.mec", "value := 1\n")
            .unwrap();
        let before = resolver
            .resolve(&SourceRequest::new("main.mec"))
            .unwrap()
            .unwrap();

        assert!(
            resolver
                .insert_canonical_string("main.mec", "value := [\n")
                .is_err()
        );
        let after = resolver
            .resolve(&SourceRequest::new("main.mec"))
            .unwrap()
            .unwrap();
        assert_eq!(after.source, before.source);
        assert_eq!(
            after.source_document().unwrap().source().revision(),
            Revision(0),
        );
    }

    #[cfg(feature = "source")]
    #[test]
    fn canonical_builder_retains_malformed_source_without_publishing_facts() {
        let resolver = InMemorySourceResolver::new()
            .with_canonical_string("broken.mec", "use ./ready.mec\nx := [\n");
        let resolved = resolver
            .resolve(&SourceRequest::new("broken.mec"))
            .unwrap()
            .expect("malformed canonical source remains available to diagnostics");

        assert!(resolved.source_document().is_some());
        assert!(!resolved.source_document().unwrap().is_strictly_clean());
        assert!(resolved.canonical_document_index().is_err());
        assert!(resolved.imports.is_empty());
        assert!(resolved.exports.is_empty());
        assert!(resolved.contexts.is_empty());
        assert!(resolved.address_references.is_empty());
        assert!(resolved.scopes.is_empty());
        assert!(resolved.dependencies.is_empty());
    }

    #[cfg(all(feature = "source", feature = "mika"))]
    #[test]
    fn canonical_insertion_hands_off_root_dependencies_and_preserves_local_owners() {
        let source = "+> ./root.mec\n\n~∘~⸢+> ./child.mec\nx := @env/HOME\n⸥\n";
        let mut resolver = InMemorySourceResolver::new();
        resolver
            .insert_canonical_string("main.mec", source)
            .unwrap();
        let resolved = resolver
            .resolve(&SourceRequest::new("main.mec"))
            .unwrap()
            .unwrap();

        assert_eq!(resolved.imports.len(), 1);
        assert_eq!(resolved.dependencies.len(), 1);
        assert_eq!(resolved.dependencies[0].specifier, "./root.mec");
        assert_eq!(
            resolved.dependencies[0].referrer.as_deref(),
            Some("memory:main.mec")
        );
        let index = resolved.canonical_document_index().unwrap();
        assert_eq!(index.root.imports.len(), 1);
        assert_eq!(index.mika.len(), 1);
        assert_eq!(index.mika[0].index.imports.len(), 1);
        assert_eq!(
            index.mika[0].index.program_address_references()[0].target,
            "env"
        );
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
            (
                "../../shared/deep/lib.mec",
                "memory:app/nested/main.mec",
                "shared/deep/lib.mec",
            ),
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
        resolver
            .insert_resolution("root", "./literal spelling", "dependency")
            .unwrap();

        let resolved = resolver
            .resolve(&SourceRequest::new("./literal spelling").with_referrer("memory:root"))
            .unwrap()
            .unwrap();

        assert_eq!(resolved.name, "dependency");
    }

    #[test]
    fn in_memory_resolution_edge_uses_referrer_canonical_identity() {
        let mut resolver = InMemorySourceResolver::new();
        resolver
            .insert_source(
                "root-key",
                ResolvedSource::new(
                    "logical-root",
                    "document:canonical-root",
                    MechSourceCode::String("x := 1".to_string()),
                )
                .with_kind(SourceKind::Mech),
            )
            .unwrap();
        resolver.insert_string("target", "value := 41").unwrap();
        resolver
            .insert_resolution("root-key", "./dep", "target")
            .unwrap();

        assert!(
            resolver
                .resolve(&SourceRequest::new("./dep").with_referrer("memory:root-key"),)
                .unwrap()
                .is_none()
        );
        assert_eq!(
            resolver
                .resolve(&SourceRequest::new("./dep").with_referrer("document:canonical-root"),)
                .unwrap()
                .unwrap()
                .name,
            "target"
        );
    }

    #[test]
    fn in_memory_resolution_edge_precedes_global_exact_source() {
        let mut resolver = InMemorySourceResolver::new()
            .with_string("root", "x := 1")
            .with_string("./dep", "value := 0")
            .with_string("edge-target", "value := 41");
        resolver
            .insert_resolution("root", "./dep", "edge-target")
            .unwrap();

        let resolved = resolver
            .resolve(&SourceRequest::new("./dep").with_referrer("memory:root"))
            .unwrap()
            .unwrap();

        assert_eq!(resolved.name, "edge-target");
    }

    #[test]
    fn in_memory_resolution_edge_reuses_target_canonical_uri() {
        let mut resolver = InMemorySourceResolver::new().with_string("root", "x := 1");
        resolver
            .insert_source(
                "target-key",
                ResolvedSource::new(
                    "target-name",
                    "document:canonical-target",
                    MechSourceCode::String("value := 41".to_string()),
                )
                .with_kind(SourceKind::Mech),
            )
            .unwrap();
        resolver
            .insert_resolution("root", "./dep", "target-key")
            .unwrap();

        let resolved = resolver
            .resolve(&SourceRequest::new("./dep").with_referrer("memory:root"))
            .unwrap()
            .unwrap();

        assert_eq!(resolved.name, "target-name");
        assert_eq!(resolved.canonical_uri, "document:canonical-target");
    }

    #[test]
    fn in_memory_resolution_edge_rejects_conflicting_target() {
        let mut resolver = InMemorySourceResolver::new()
            .with_string("root", "x := 1")
            .with_string("first", "value := 1")
            .with_string("second", "value := 2");
        resolver
            .insert_resolution("root", "./dep", "first")
            .unwrap();
        resolver
            .insert_resolution("root", "./dep", "first")
            .unwrap();

        let error = resolver
            .insert_resolution("root", "./dep", "second")
            .unwrap_err();
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

        assert_eq!(
            missing_referrer.kind_name(),
            "InMemorySourceResolutionTargetMissing"
        );
        assert_eq!(
            missing_target.kind_name(),
            "InMemorySourceResolutionTargetMissing"
        );
    }

    #[test]
    fn ordinary_memory_relative_resolution_remains_available() {
        let resolver = InMemorySourceResolver::new().with_string("app/dep.mec", "value := 41");

        let resolved = resolver
            .resolve(&SourceRequest::new("./dep.mec").with_referrer("memory:app/main.mec"))
            .unwrap()
            .unwrap();

        assert_eq!(resolved.name, "app/dep.mec");
    }

    #[test]
    fn ascii_relative_import_behavior_is_unchanged() {
        let resolver = InMemorySourceResolver::new().with_string("app/dep.mec", "value := 41");

        let request = SourceRequest::new("./dep.mec").with_referrer("memory:app/main.mec");
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

        let request = SourceRequest::new("./lib.mec").with_referrer("memory:app/main.mec");
        assert_eq!(resolver.resolve(&request).unwrap().unwrap().name, "lib.mec");
    }

    #[test]
    fn remove_deletes_source() {
        let mut resolver = InMemorySourceResolver::new().with_string("main.mec", "x := 1");

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
    fn try_with_source_reports_invalid_resolved_source() {
        let bad = ResolvedSource::new(
            "",
            "memory:bad",
            MechSourceCode::String("x := 1".to_string()),
        );

        assert!(
            InMemorySourceResolver::new()
                .try_with_source("bad", bad)
                .is_err()
        );
    }

    #[test]
    fn with_source_preserves_non_panicking_compatibility_behavior() {
        let bad = ResolvedSource::new(
            "",
            "memory:bad",
            MechSourceCode::String("x := 1".to_string()),
        );

        let resolver = InMemorySourceResolver::new().with_source("bad", bad);

        assert!(!resolver.contains("bad"));
    }

    #[test]
    fn len_and_is_empty_work() {
        let mut resolver = InMemorySourceResolver::new();

        assert!(resolver.is_empty());
        assert_eq!(resolver.len(), 0);

        resolver.insert_string("main.mec", "x := 1").unwrap();

        assert!(!resolver.is_empty());
        assert_eq!(resolver.len(), 1);
    }
}
