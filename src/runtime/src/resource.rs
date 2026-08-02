use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use mech_core::{MResult, MechError, MechErrorKind, Value};

use crate::extension::{catch_extension, invoke_extension};
use crate::{
    PreparedRuntimeEffect, RuntimeCapabilityOperation, RuntimeCompensatableEffect,
    RuntimeEffectCost, RuntimeEffectMetadata, RuntimeEffectSource,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeResourceReadRequest {
    pub base_uri: String,
    pub path: String,
    pub context_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeResourceWriteIntent {
    Assign,
    Send,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeResourceWritePreflightRequest {
    pub base_uri: String,
    pub path: String,
    pub context_name: String,
    pub operation: RuntimeCapabilityOperation,
    pub intent: RuntimeResourceWriteIntent,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeResourceWriteRequest {
    pub base_uri: String,
    pub path: String,
    pub context_name: String,
    pub operation: RuntimeCapabilityOperation,
    pub value: Value,
    pub intent: RuntimeResourceWriteIntent,
}

pub trait RuntimeResourceProvider: std::fmt::Debug {
    fn scheme(&self) -> &str;

    fn base_uris(&self) -> Vec<String> {
        Vec::new()
    }

    /// Declares sets of provider base URIs that identify the same protected
    /// resource. This is compatibility metadata, not general URI rewriting.
    fn equivalent_base_uri_groups(&self) -> Vec<Vec<String>> {
        Vec::new()
    }

    fn plan_read(&self, request: RuntimeResourceReadRequest) -> MResult<Value> {
        Err(MechError::new(
            RuntimeResourceReadNotPlannable {
                scheme: self.scheme().to_string(),
                base_uri: request.base_uri,
                path: request.path,
            },
            None,
        ))
    }

    fn read(&self, request: RuntimeResourceReadRequest) -> MResult<Value>;

    fn preflight_write(&self, request: RuntimeResourceWritePreflightRequest) -> MResult<()> {
        Err(MechError::new(
            RuntimeResourceWriteUnsupported {
                scheme: self.scheme().to_string(),
                base_uri: request.base_uri,
                path: request.path,
            },
            None,
        ))
    }

    fn prepare_write(
        &self,
        request: RuntimeResourceWriteRequest,
    ) -> MResult<PreparedRuntimeEffect> {
        Err(MechError::new(
            RuntimeResourceWriteUnsupported {
                scheme: self.scheme().to_string(),
                base_uri: request.base_uri,
                path: request.path,
            },
            None,
        ))
    }
}

#[derive(Debug)]
struct RuntimeResourceProviderEntry {
    scheme: String,
    bases: Vec<String>,
    equivalent_base_uri_groups: Vec<Vec<String>>,
    provider: Box<dyn RuntimeResourceProvider>,
}

#[derive(Debug, Default)]
pub(crate) struct RuntimeResourceRegistry {
    providers: Vec<RuntimeResourceProviderEntry>,
}

impl RuntimeResourceRegistry {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn register_provider(
        &mut self,
        provider: Box<dyn RuntimeResourceProvider>,
    ) -> MResult<()> {
        let (scheme, bases, equivalent_groups) =
            catch_extension("resource provider", "registration metadata", || {
                (
                    provider.scheme().to_string(),
                    provider.base_uris(),
                    provider.equivalent_base_uri_groups(),
                )
            })
            .map_err(|panic| panic.into_error())?;
        if scheme.is_empty() {
            return Err(MechError::new(
                RuntimeResourceInvalidUri {
                    uri: String::new(),
                    reason: "resource provider scheme cannot be empty".to_string(),
                },
                None,
            ));
        }

        let bases = normalize_provider_bases(&scheme, bases)?;
        let equivalent_base_uri_groups =
            normalize_provider_equivalence_groups(&scheme, &bases, equivalent_groups)?;

        for base in &bases {
            if self
                .providers
                .iter()
                .any(|entry| entry.bases.iter().any(|existing| existing == base))
            {
                return Err(MechError::new(
                    RuntimeResourceProviderConflict {
                        scheme: scheme.clone(),
                    },
                    None,
                ));
            }
        }

        if bases.is_empty()
            && self
                .providers
                .iter()
                .any(|entry| entry.scheme == scheme && entry.bases.is_empty())
        {
            return Err(MechError::new(
                RuntimeResourceProviderConflict {
                    scheme: scheme.clone(),
                },
                None,
            ));
        }

        self.providers.push(RuntimeResourceProviderEntry {
            scheme,
            bases,
            equivalent_base_uri_groups,
            provider,
        });
        Ok(())
    }

    pub(crate) fn has_provider(&self, scheme: &str) -> bool {
        self.providers.iter().any(|entry| entry.scheme == scheme)
    }

    pub(crate) fn provider_base_uri_for(&self, candidate: &str) -> MResult<Option<String>> {
        let scheme = resource_uri_scheme(candidate)?.to_string();
        let Some(entry) = self.provider_entry_for(&scheme, candidate) else {
            return Ok(None);
        };
        if let Some(base) = entry
            .bases
            .iter()
            .filter(|base| resource_base_matches(base, candidate))
            .max_by_key(|base| base.len())
        {
            return Ok(Some(base.clone()));
        }
        Ok(Some(resource_uri_origin(candidate)?.to_string()))
    }

    pub(crate) fn equivalent_base_uris_for(&self, base_uri: &str) -> MResult<Vec<String>> {
        let normalized = canonicalize_resource_base_uri(base_uri)?;
        let Some(entry) = self
            .providers
            .iter()
            .find(|entry| entry.bases.iter().any(|base| base == &normalized))
        else {
            return Ok(vec![normalized]);
        };
        Ok(entry
            .equivalent_base_uri_groups
            .iter()
            .find(|group| group.iter().any(|base| base == &normalized))
            .cloned()
            .unwrap_or_else(|| vec![normalized]))
    }

    /// Returns the stable transaction-journal identity for a provider base URI.
    ///
    /// Equivalent bases share the first normalized member declared by their
    /// provider. Bases outside an equivalence group retain their own identity.
    pub(crate) fn staged_resource_identity_for(&self, base_uri: &str) -> MResult<String> {
        let normalized = canonicalize_resource_base_uri(base_uri)?;
        let equivalent_base_uris = self.equivalent_base_uris_for(&normalized)?;
        Ok(equivalent_base_uris
            .into_iter()
            .next()
            .unwrap_or(normalized))
    }

    fn provider_entry_for(&self, scheme: &str, uri: &str) -> Option<&RuntimeResourceProviderEntry> {
        self.providers
            .iter()
            .filter(|entry| {
                entry.scheme == scheme
                    && entry
                        .bases
                        .iter()
                        .any(|base| resource_base_matches(base, uri))
            })
            .max_by_key(|entry| {
                entry
                    .bases
                    .iter()
                    .filter(|base| resource_base_matches(base, uri))
                    .map(|base| base.len())
                    .max()
                    .unwrap_or(0)
            })
            .or_else(|| {
                self.providers
                    .iter()
                    .find(|entry| entry.scheme == scheme && entry.bases.is_empty())
            })
    }

    pub(crate) fn read(&self, request: RuntimeResourceReadRequest) -> MResult<Value> {
        let scheme = resource_uri_scheme(&request.base_uri)?.to_string();
        let Some(entry) = self.provider_entry_for(&scheme, &request.base_uri) else {
            return Err(MechError::new(
                RuntimeResourceProviderNotFound {
                    scheme,
                    uri: request.base_uri,
                },
                None,
            ));
        };
        invoke_extension(format!("resource provider `{scheme}`"), "read", || {
            entry.provider.read(request)
        })
    }

    pub(crate) fn plan_read(&self, request: RuntimeResourceReadRequest) -> MResult<Value> {
        let scheme = resource_uri_scheme(&request.base_uri)?.to_string();
        let Some(entry) = self.provider_entry_for(&scheme, &request.base_uri) else {
            return Err(MechError::new(
                RuntimeResourceProviderNotFound {
                    scheme,
                    uri: request.base_uri,
                },
                None,
            ));
        };
        invoke_extension(format!("resource provider `{scheme}`"), "plan_read", || {
            entry.provider.plan_read(request)
        })
    }

    pub(crate) fn preflight_write(
        &self,
        request: RuntimeResourceWritePreflightRequest,
    ) -> MResult<()> {
        let scheme = resource_uri_scheme(&request.base_uri)?.to_string();
        let Some(entry) = self.provider_entry_for(&scheme, &request.base_uri) else {
            return Err(MechError::new(
                RuntimeResourceProviderNotFound {
                    scheme,
                    uri: request.base_uri,
                },
                None,
            ));
        };
        invoke_extension(
            format!("resource provider `{scheme}`"),
            "preflight_write",
            || entry.provider.preflight_write(request),
        )
    }

    pub(crate) fn prepare_write(
        &self,
        request: RuntimeResourceWriteRequest,
    ) -> MResult<PreparedRuntimeEffect> {
        let scheme = resource_uri_scheme(&request.base_uri)?.to_string();
        let Some(entry) = self.provider_entry_for(&scheme, &request.base_uri) else {
            return Err(MechError::new(
                RuntimeResourceProviderNotFound {
                    scheme,
                    uri: request.base_uri,
                },
                None,
            ));
        };
        invoke_extension(
            format!("resource provider `{scheme}`"),
            "prepare_write",
            || entry.provider.prepare_write(request),
        )
    }
}

#[derive(Clone, Debug, Default)]
pub struct InMemoryDocsProvider {
    documents: Arc<Mutex<HashMap<String, HashMap<String, Value>>>>,
}

impl InMemoryDocsProvider {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(
        &mut self,
        base_uri: impl Into<String>,
        path: impl Into<String>,
        value: Value,
    ) -> MResult<()> {
        let base_uri = base_uri.into();
        let path = path.into();
        let scheme = resource_uri_scheme(&base_uri)?;
        if scheme != "docs" {
            return Err(MechError::new(
                RuntimeResourceInvalidUri {
                    uri: base_uri,
                    reason: "in-memory docs resources require the `docs` scheme".to_string(),
                },
                None,
            ));
        }
        if path.is_empty() {
            return Err(MechError::new(
                RuntimeResourceInvalidUri {
                    uri: base_uri,
                    reason: "resource path cannot be empty".to_string(),
                },
                None,
            ));
        }
        self.documents
            .lock()
            .map_err(|_| in_memory_docs_lock_error(&base_uri))?
            .entry(base_uri)
            .or_default()
            .insert(path, value);
        Ok(())
    }

    pub fn with_value(
        mut self,
        base_uri: impl Into<String>,
        path: impl Into<String>,
        value: Value,
    ) -> MResult<Self> {
        self.insert(base_uri, path, value)?;
        Ok(self)
    }

    fn snapshot_value(&self, request: RuntimeResourceReadRequest) -> MResult<Value> {
        let documents = self
            .documents
            .lock()
            .map_err(|_| in_memory_docs_lock_error(&request.base_uri))?;
        let Some(document) = documents.get(&request.base_uri) else {
            return Err(MechError::new(
                RuntimeResourcePathNotFound {
                    base_uri: request.base_uri,
                    path: request.path,
                },
                None,
            ));
        };
        let Some(value) = document.get(&request.path) else {
            return Err(MechError::new(
                RuntimeResourcePathNotFound {
                    base_uri: request.base_uri,
                    path: request.path,
                },
                None,
            ));
        };
        value.try_deep_snapshot()
    }
}

impl RuntimeResourceProvider for InMemoryDocsProvider {
    fn scheme(&self) -> &str {
        "docs"
    }

    fn base_uris(&self) -> Vec<String> {
        self.documents
            .lock()
            .map(|documents| documents.keys().cloned().collect())
            .unwrap_or_default()
    }

    fn read(&self, request: RuntimeResourceReadRequest) -> MResult<Value> {
        self.snapshot_value(request)
    }

    fn plan_read(&self, request: RuntimeResourceReadRequest) -> MResult<Value> {
        // In-memory documents are deterministic build inputs. Planning reads a
        // detached snapshot directly from the configured document set without
        // entering the provider's runtime `read` operation.
        self.snapshot_value(request)
    }

    fn preflight_write(&self, request: RuntimeResourceWritePreflightRequest) -> MResult<()> {
        if request.intent == RuntimeResourceWriteIntent::Send {
            return Err(MechError::new(
                RuntimeResourceWriteUnsupported {
                    scheme: self.scheme().to_string(),
                    base_uri: request.base_uri,
                    path: request.path,
                },
                None,
            ));
        }

        let scheme = resource_uri_scheme(&request.base_uri)?;
        if scheme != "docs" {
            return Err(MechError::new(
                RuntimeResourceInvalidUri {
                    uri: request.base_uri,
                    reason: "in-memory docs resources require the `docs` scheme".to_string(),
                },
                None,
            ));
        }

        if request.path.is_empty() {
            return Err(MechError::new(
                RuntimeResourceInvalidUri {
                    uri: request.base_uri,
                    reason: "resource path cannot be empty".to_string(),
                },
                None,
            ));
        }

        Ok(())
    }

    fn prepare_write(
        &self,
        request: RuntimeResourceWriteRequest,
    ) -> MResult<PreparedRuntimeEffect> {
        self.preflight_write(RuntimeResourceWritePreflightRequest {
            base_uri: request.base_uri.clone(),
            path: request.path.clone(),
            context_name: request.context_name.clone(),
            operation: request.operation.clone(),
            intent: request.intent,
        })?;

        Ok(PreparedRuntimeEffect::Compensatable(Box::new(
            InMemoryDocsWriteEffect {
                documents: self.documents.clone(),
                base_uri: request.base_uri,
                path: request.path,
                value: request.value,
                previous: None,
                base_existed: false,
                applied: false,
            },
        )))
    }
}

#[derive(Debug)]
struct InMemoryDocsWriteEffect {
    documents: Arc<Mutex<HashMap<String, HashMap<String, Value>>>>,
    base_uri: String,
    path: String,
    value: Value,
    previous: Option<Value>,
    base_existed: bool,
    applied: bool,
}

impl RuntimeCompensatableEffect for InMemoryDocsWriteEffect {
    fn metadata(&self) -> RuntimeEffectMetadata {
        RuntimeEffectMetadata::new(
            RuntimeEffectSource::ResourceProvider {
                scheme: "docs".to_string(),
            },
            "write",
        )
        .with_resource(format!("{}/{}", self.base_uri, self.path))
        .with_cost(RuntimeEffectCost { bytes: 0, items: 1 })
    }

    fn apply(&mut self) -> MResult<()> {
        let mut documents = self
            .documents
            .lock()
            .map_err(|_| in_memory_docs_lock_error(&self.base_uri))?;
        self.base_existed = documents.contains_key(&self.base_uri);
        self.previous = documents
            .entry(self.base_uri.clone())
            .or_default()
            .insert(self.path.clone(), self.value.clone());
        self.applied = true;
        Ok(())
    }

    fn compensate(&mut self) -> MResult<()> {
        if !self.applied {
            return Ok(());
        }
        let mut documents = self
            .documents
            .lock()
            .map_err(|_| in_memory_docs_lock_error(&self.base_uri))?;
        match self.previous.take() {
            Some(previous) => {
                documents
                    .entry(self.base_uri.clone())
                    .or_default()
                    .insert(self.path.clone(), previous);
            }
            None => {
                let remove_base = if let Some(document) = documents.get_mut(&self.base_uri) {
                    document.remove(&self.path);
                    !self.base_existed && document.is_empty()
                } else {
                    false
                };
                if remove_base {
                    documents.remove(&self.base_uri);
                }
            }
        }
        self.applied = false;
        Ok(())
    }
}

fn in_memory_docs_lock_error(base_uri: &str) -> MechError {
    MechError::new(
        RuntimeResourceInvalidUri {
            uri: base_uri.to_string(),
            reason: "in-memory docs resource lock is poisoned".to_string(),
        },
        None,
    )
}

fn normalize_provider_bases(scheme: &str, bases: Vec<String>) -> MResult<Vec<String>> {
    let mut normalized = Vec::new();
    let mut seen = HashSet::new();
    for base in bases {
        let canonical = canonicalize_resource_base_uri(&base)?;
        let base_scheme = resource_uri_scheme(&canonical)?;
        if base_scheme != scheme {
            return Err(MechError::new(
                RuntimeResourceInvalidUri {
                    uri: base,
                    reason: format!("resource provider base URI scheme must be `{scheme}`",),
                },
                None,
            ));
        }
        if seen.insert(canonical.clone()) {
            normalized.push(canonical);
        }
    }
    Ok(normalized)
}

fn normalize_provider_equivalence_groups(
    scheme: &str,
    bases: &[String],
    groups: Vec<Vec<String>>,
) -> MResult<Vec<Vec<String>>> {
    if !groups.is_empty() && bases.is_empty() {
        return Err(MechError::new(
            RuntimeResourceInvalidUri {
                uri: format!("{scheme}://"),
                reason:
                    "resource provider without advertised bases cannot declare equivalent base URIs"
                        .to_string(),
            },
            None,
        ));
    }

    let mut normalized_groups = Vec::new();
    let mut grouped_bases = HashSet::new();
    for group in groups {
        if group.is_empty() {
            return Err(MechError::new(
                RuntimeResourceInvalidUri {
                    uri: format!("{scheme}://"),
                    reason: "resource provider equivalent base URI group cannot be empty"
                        .to_string(),
                },
                None,
            ));
        }

        let mut normalized_group = Vec::new();
        let mut seen_in_group = HashSet::new();
        for member in group {
            let canonical = canonicalize_resource_base_uri(&member)?;
            let member_scheme = resource_uri_scheme(&canonical)?;
            if member_scheme != scheme {
                return Err(MechError::new(
                    RuntimeResourceInvalidUri {
                        uri: member,
                        reason: format!(
                            "equivalent resource provider base URI scheme must be `{scheme}`",
                        ),
                    },
                    None,
                ));
            }
            if !bases.iter().any(|base| base == &canonical) {
                return Err(MechError::new(
          RuntimeResourceInvalidUri {
            uri: member,
            reason: "equivalent resource provider base URI must be advertised by the same provider".to_string(),
          },
          None,
        ));
            }
            if seen_in_group.insert(canonical.clone()) {
                normalized_group.push(canonical);
            }
        }

        if normalized_group.len() < 2 {
            return Err(MechError::new(
        RuntimeResourceInvalidUri {
          uri: normalized_group
            .first()
            .cloned()
            .unwrap_or_else(|| format!("{scheme}://")),
          reason: "resource provider equivalent base URI group must contain at least two distinct members".to_string(),
        },
        None,
      ));
        }
        for member in &normalized_group {
            if !grouped_bases.insert(member.clone()) {
                return Err(MechError::new(
                    RuntimeResourceInvalidUri {
                        uri: member.clone(),
                        reason:
                            "resource provider base URI may belong to only one equivalence group"
                                .to_string(),
                    },
                    None,
                ));
            }
        }
        normalized_groups.push(normalized_group);
    }
    Ok(normalized_groups)
}

pub fn resource_base_matches(base: &str, candidate: &str) -> bool {
    candidate == base
        || candidate
            .strip_prefix(base)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

pub(crate) fn canonicalize_resource_base_uri(uri: &str) -> MResult<String> {
    let canonical = uri.trim_end_matches('/');
    resource_uri_origin(canonical)?;
    Ok(canonical.to_string())
}

fn resource_uri_origin(uri: &str) -> MResult<&str> {
    let scheme = resource_uri_scheme(uri)?;
    let rest = &uri[scheme.len() + 3..];
    let authority_end = rest.find('/').unwrap_or(rest.len());
    if authority_end == 0 {
        return Err(MechError::new(
            RuntimeResourceInvalidUri {
                uri: uri.to_string(),
                reason: "resource URI authority cannot be empty".to_string(),
            },
            None,
        ));
    }
    Ok(&uri[..scheme.len() + 3 + authority_end])
}

fn resource_uri_scheme(uri: &str) -> MResult<&str> {
    let Some((scheme, _rest)) = uri.split_once("://") else {
        return Err(MechError::new(
            RuntimeResourceInvalidUri {
                uri: uri.to_string(),
                reason: "resource URI must contain `://`".to_string(),
            },
            None,
        ));
    };
    if scheme.is_empty() {
        return Err(MechError::new(
            RuntimeResourceInvalidUri {
                uri: uri.to_string(),
                reason: "resource URI scheme cannot be empty".to_string(),
            },
            None,
        ));
    }
    Ok(scheme)
}

#[derive(Debug, Clone)]
pub struct RuntimeResourceInvalidUri {
    pub uri: String,
    pub reason: String,
}

impl MechErrorKind for RuntimeResourceInvalidUri {
    fn name(&self) -> &str {
        "RuntimeResourceInvalidUri"
    }

    fn message(&self) -> String {
        format!("invalid resource URI `{}`: {}", self.uri, self.reason)
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeResourceProviderNotFound {
    pub scheme: String,
    pub uri: String,
}

impl MechErrorKind for RuntimeResourceProviderNotFound {
    fn name(&self) -> &str {
        "RuntimeResourceProviderNotFound"
    }

    fn message(&self) -> String {
        format!(
            "no runtime resource provider registered for scheme `{}` while reading `{}`",
            self.scheme, self.uri,
        )
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeResourceWriteUnsupported {
    pub scheme: String,
    pub base_uri: String,
    pub path: String,
}

#[derive(Debug, Clone)]
pub struct RuntimeResourceReadNotPlannable {
    pub scheme: String,
    pub base_uri: String,
    pub path: String,
}

impl MechErrorKind for RuntimeResourceReadNotPlannable {
    fn name(&self) -> &str {
        "RuntimeResourceReadNotPlannable"
    }

    fn message(&self) -> String {
        format!(
            "runtime resource provider for scheme `{}` cannot plan a read of `{}` under `{}`",
            self.scheme, self.path, self.base_uri,
        )
    }
}

impl MechErrorKind for RuntimeResourceWriteUnsupported {
    fn name(&self) -> &str {
        "RuntimeResourceWriteUnsupported"
    }

    fn message(&self) -> String {
        format!(
            "runtime resource provider for scheme `{}` does not support writes to `{}` under `{}`",
            self.scheme, self.path, self.base_uri,
        )
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeResourceProviderConflict {
    pub scheme: String,
}

impl MechErrorKind for RuntimeResourceProviderConflict {
    fn name(&self) -> &str {
        "RuntimeResourceProviderConflict"
    }

    fn message(&self) -> String {
        format!(
            "runtime resource provider for scheme `{}` is already registered",
            self.scheme
        )
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeResourcePathNotFound {
    pub base_uri: String,
    pub path: String,
}

impl MechErrorKind for RuntimeResourcePathNotFound {
    fn name(&self) -> &str {
        "RuntimeResourcePathNotFound"
    }

    fn message(&self) -> String {
        format!(
            "resource path `{}` was not found under `{}`",
            self.path, self.base_uri
        )
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeResourceCapabilityDenied {
    pub context_name: String,
    pub operation: String,
    pub path: String,
}

impl MechErrorKind for RuntimeResourceCapabilityDenied {
    fn name(&self) -> &str {
        "RuntimeResourceCapabilityDenied"
    }

    fn message(&self) -> String {
        format!(
            "context `{}` does not allow `{}` on `{}`",
            self.context_name, self.operation, self.path,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mech_core::Ref;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    #[derive(Debug)]
    struct DefaultPlanningProvider {
        reads: Arc<AtomicUsize>,
    }

    impl RuntimeResourceProvider for DefaultPlanningProvider {
        fn scheme(&self) -> &str {
            "default-plan"
        }

        fn read(&self, _request: RuntimeResourceReadRequest) -> MResult<Value> {
            self.reads.fetch_add(1, Ordering::SeqCst);
            Ok(Value::F64(Ref::new(1.0)))
        }
    }

    #[test]
    fn default_plan_read_is_structured_and_never_calls_read() {
        let reads = Arc::new(AtomicUsize::new(0));
        let provider = DefaultPlanningProvider {
            reads: Arc::clone(&reads),
        };

        let error = provider
            .plan_read(RuntimeResourceReadRequest {
                base_uri: "default-plan://clock".to_string(),
                path: "value".to_string(),
                context_name: "clock".to_string(),
            })
            .unwrap_err();

        assert_eq!(error.kind_name(), "RuntimeResourceReadNotPlannable");
        assert_eq!(reads.load(Ordering::SeqCst), 0);
    }

    #[derive(Debug, Default)]
    struct PlanningProviderCounters {
        planned_reads: AtomicUsize,
        reads: AtomicUsize,
        preflight_writes: AtomicUsize,
        prepared_writes: AtomicUsize,
    }

    #[derive(Debug)]
    struct SyntheticLivePlanningProvider {
        counters: Arc<PlanningProviderCounters>,
    }

    impl RuntimeResourceProvider for SyntheticLivePlanningProvider {
        fn scheme(&self) -> &str {
            "synthetic-live"
        }

        fn base_uris(&self) -> Vec<String> {
            vec!["synthetic-live://clock/clock".to_string()]
        }

        fn plan_read(&self, request: RuntimeResourceReadRequest) -> MResult<Value> {
            assert_eq!(request.path, "value");
            self.counters.planned_reads.fetch_add(1, Ordering::SeqCst);
            Ok(Value::F64(Ref::new(0.0)))
        }

        fn read(&self, request: RuntimeResourceReadRequest) -> MResult<Value> {
            assert_eq!(request.path, "value");
            self.counters.reads.fetch_add(1, Ordering::SeqCst);
            Ok(Value::F64(Ref::new(7.0)))
        }

        fn preflight_write(&self, request: RuntimeResourceWritePreflightRequest) -> MResult<()> {
            assert_eq!(request.path, "value");
            self.counters
                .preflight_writes
                .fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        fn prepare_write(
            &self,
            _request: RuntimeResourceWriteRequest,
        ) -> MResult<PreparedRuntimeEffect> {
            self.counters.prepared_writes.fetch_add(1, Ordering::SeqCst);
            panic!("planning must not prepare resource effects")
        }
    }

    fn grant_synthetic_live(runtime: &mut crate::MechRuntime, operation: &str) {
        let subject = runtime.runtime_context().unwrap().subject().to_string();
        let capability = crate::ResourcePathCapability::wildcard(
            runtime.next_capability_id(),
            subject,
            "synthetic-live://clock/clock",
            [operation],
        )
        .unwrap();
        runtime.grant_capability(Arc::new(capability)).unwrap();
    }

    #[test]
    fn planning_runtime_uses_only_provider_planning_and_preflight_methods() {
        let counters = Arc::new(PlanningProviderCounters::default());
        let provider = SyntheticLivePlanningProvider {
            counters: Arc::clone(&counters),
        };
        let mut runtime = crate::RuntimeBuilder::new()
            .planning()
            .resource_provider(Box::new(provider))
            .build()
            .unwrap();
        grant_synthetic_live(&mut runtime, "read");
        grant_synthetic_live(&mut runtime, "write");

        let planned = runtime
            .read_resource(RuntimeResourceReadRequest {
                base_uri: "synthetic-live://clock/clock".to_string(),
                path: "value".to_string(),
                context_name: "clock".to_string(),
            })
            .unwrap()
            .to_value();
        assert_eq!(planned, Value::F64(Ref::new(0.0)));

        runtime
            .write_resource(RuntimeResourceWriteRequest {
                base_uri: "synthetic-live://clock/clock".to_string(),
                path: "value".to_string(),
                context_name: "clock".to_string(),
                operation: RuntimeCapabilityOperation::Write,
                value: Value::F64(Ref::new(9.0)),
                intent: RuntimeResourceWriteIntent::Assign,
            })
            .unwrap();

        runtime
            .write_resource(RuntimeResourceWriteRequest {
                base_uri: "synthetic-live://clock/clock".to_string(),
                path: "value".to_string(),
                context_name: "clock".to_string(),
                operation: RuntimeCapabilityOperation::Write,
                value: Value::F64(Ref::new(10.0)),
                intent: RuntimeResourceWriteIntent::Send,
            })
            .unwrap();

        assert_eq!(counters.planned_reads.load(Ordering::SeqCst), 1);
        assert_eq!(counters.reads.load(Ordering::SeqCst), 0);
        assert_eq!(counters.preflight_writes.load(Ordering::SeqCst), 2);
        assert_eq!(counters.prepared_writes.load(Ordering::SeqCst), 0);
    }

    #[derive(Debug)]
    struct EquivalentBaseProvider {
        scheme: &'static str,
        bases: Vec<String>,
        groups: Vec<Vec<String>>,
    }

    impl RuntimeResourceProvider for EquivalentBaseProvider {
        fn scheme(&self) -> &str {
            self.scheme
        }

        fn base_uris(&self) -> Vec<String> {
            self.bases.clone()
        }

        fn equivalent_base_uri_groups(&self) -> Vec<Vec<String>> {
            self.groups.clone()
        }

        fn read(&self, _request: RuntimeResourceReadRequest) -> MResult<Value> {
            Ok(Value::Empty)
        }
    }

    #[test]
    fn provider_equivalence_groups_are_normalized_and_captured() {
        let mut registry = RuntimeResourceRegistry::new();
        registry
            .register_provider(Box::new(EquivalentBaseProvider {
                scheme: "browser",
                bases: vec![
                    "browser://browser/dom/".to_string(),
                    "browser://dom".to_string(),
                    "browser://dom/".to_string(),
                ],
                groups: vec![vec![
                    "browser://browser/dom/".to_string(),
                    "browser://dom".to_string(),
                    "browser://dom/".to_string(),
                ]],
            }))
            .unwrap();

        assert_eq!(
            registry.equivalent_base_uris_for("browser://dom/").unwrap(),
            vec![
                "browser://browser/dom".to_string(),
                "browser://dom".to_string(),
            ],
        );
        assert_eq!(
            registry
                .equivalent_base_uris_for("browser://browser/dom")
                .unwrap(),
            vec![
                "browser://browser/dom".to_string(),
                "browser://dom".to_string(),
            ],
        );
        assert_eq!(
            registry
                .staged_resource_identity_for("browser://dom/")
                .unwrap(),
            "browser://browser/dom",
        );
        assert_eq!(
            registry
                .staged_resource_identity_for("browser://browser/dom")
                .unwrap(),
            "browser://browser/dom",
        );
    }

    #[test]
    fn provider_equivalence_rejects_unadvertised_base() {
        let mut registry = RuntimeResourceRegistry::new();
        let error = registry
            .register_provider(Box::new(EquivalentBaseProvider {
                scheme: "test",
                bases: vec!["test://canonical".to_string()],
                groups: vec![vec![
                    "test://canonical".to_string(),
                    "test://undeclared".to_string(),
                ]],
            }))
            .unwrap_err();

        assert_eq!(error.kind_name(), "RuntimeResourceInvalidUri");
        assert!(
            error
                .full_chain_message()
                .contains("must be advertised by the same provider")
        );
    }

    #[test]
    fn provider_equivalence_rejects_duplicate_group_membership() {
        let mut registry = RuntimeResourceRegistry::new();
        let error = registry
            .register_provider(Box::new(EquivalentBaseProvider {
                scheme: "test",
                bases: vec![
                    "test://canonical".to_string(),
                    "test://legacy".to_string(),
                    "test://other".to_string(),
                ],
                groups: vec![
                    vec!["test://canonical".to_string(), "test://legacy".to_string()],
                    vec!["test://canonical/".to_string(), "test://other".to_string()],
                ],
            }))
            .unwrap_err();

        assert_eq!(error.kind_name(), "RuntimeResourceInvalidUri");
        assert!(
            error
                .full_chain_message()
                .contains("may belong to only one equivalence group")
        );
    }

    fn bool_value(value: bool) -> Value {
        Value::Bool(Ref::new(value))
    }

    fn write_request(path: &str, value: bool) -> RuntimeResourceWriteRequest {
        RuntimeResourceWriteRequest {
            base_uri: "docs://manual".to_string(),
            path: path.to_string(),
            context_name: "manual".to_string(),
            operation: RuntimeCapabilityOperation::Write,
            value: bool_value(value),
            intent: RuntimeResourceWriteIntent::Assign,
        }
    }

    fn read_request(path: &str) -> RuntimeResourceReadRequest {
        RuntimeResourceReadRequest {
            base_uri: "docs://manual".to_string(),
            path: path.to_string(),
            context_name: "manual".to_string(),
        }
    }

    fn grant_docs_write(runtime: &mut crate::MechRuntime) {
        let subject = runtime.runtime_context().unwrap().subject().to_string();
        let capability = crate::ResourcePathCapability::wildcard(
            runtime.next_capability_id(),
            subject,
            "docs://manual",
            ["write"],
        )
        .unwrap();
        runtime.grant_capability(Arc::new(capability)).unwrap();
    }

    #[derive(Debug)]
    struct PanickingProvider;

    impl RuntimeResourceProvider for PanickingProvider {
        fn scheme(&self) -> &str {
            "panic"
        }

        fn base_uris(&self) -> Vec<String> {
            vec!["panic://provider".to_string()]
        }

        fn read(&self, _request: RuntimeResourceReadRequest) -> MResult<Value> {
            panic!("deliberate provider read panic");
        }

        fn preflight_write(&self, _request: RuntimeResourceWritePreflightRequest) -> MResult<()> {
            Ok(())
        }

        fn prepare_write(
            &self,
            _request: RuntimeResourceWriteRequest,
        ) -> MResult<PreparedRuntimeEffect> {
            panic!("deliberate provider prepare panic");
        }
    }

    fn grant_panic_resource(runtime: &mut crate::MechRuntime, operation: &str) {
        let subject = runtime.runtime_context().unwrap().subject().to_string();
        let capability = crate::ResourcePathCapability::wildcard(
            runtime.next_capability_id(),
            subject,
            "panic://provider",
            [operation],
        )
        .unwrap();
        runtime.grant_capability(Arc::new(capability)).unwrap();
    }

    #[test]
    fn docs_write_is_invisible_until_explicit_commit() {
        let provider = InMemoryDocsProvider::new();
        let observed = provider.clone();
        let mut runtime = crate::MechRuntime::builder()
            .resource_provider(Box::new(provider))
            .build()
            .unwrap();
        grant_docs_write(&mut runtime);
        let mut context = runtime.runtime_context().unwrap();
        runtime.begin_transaction(&mut context).unwrap();

        let effect_id = runtime
            .write_resource_with_context(&mut context, write_request("intro/enabled", true))
            .unwrap();

        assert_eq!(effect_id.sequence, 0);
        assert!(observed.read(read_request("intro/enabled")).is_err());

        runtime.commit_runtime_transaction(&mut context).unwrap();
        assert_eq!(
            observed.read(read_request("intro/enabled")).unwrap(),
            bool_value(true),
        );
    }

    #[test]
    fn docs_write_is_discarded_by_explicit_abort() {
        let mut provider = InMemoryDocsProvider::new();
        provider
            .insert("docs://manual", "intro/enabled", bool_value(false))
            .unwrap();
        let observed = provider.clone();
        let mut runtime = crate::MechRuntime::builder()
            .resource_provider(Box::new(provider))
            .build()
            .unwrap();
        grant_docs_write(&mut runtime);
        let mut context = runtime.runtime_context().unwrap();
        runtime.begin_transaction(&mut context).unwrap();
        runtime
            .write_resource_with_context(&mut context, write_request("intro/enabled", true))
            .unwrap();

        runtime
            .abort_runtime_transaction(&mut context, "discard docs write")
            .unwrap();

        assert_eq!(
            observed.read(read_request("intro/enabled")).unwrap(),
            bool_value(false),
        );
    }

    #[test]
    fn store_failure_compensates_docs_overwrite_and_creation() {
        let mut provider = InMemoryDocsProvider::new();
        provider
            .insert("docs://manual", "intro/enabled", bool_value(false))
            .unwrap();
        let observed = provider.clone();
        let mut runtime = crate::MechRuntime::builder()
            .resource_provider(Box::new(provider))
            .build()
            .unwrap();
        grant_docs_write(&mut runtime);
        let mut context = runtime.runtime_context().unwrap();
        runtime.begin_transaction(&mut context).unwrap();
        runtime
            .write_resource_with_context(&mut context, write_request("intro/enabled", true))
            .unwrap();
        runtime
            .write_resource_with_context(&mut context, write_request("intro/created", true))
            .unwrap();
        runtime
            .update_object_with_context(
                &mut context,
                crate::ObjectRecord::text(crate::ObjectId(990), "missing", "update"),
            )
            .unwrap();

        assert!(runtime.commit_runtime_transaction(&mut context).is_err());

        assert_eq!(
            observed.read(read_request("intro/enabled")).unwrap(),
            bool_value(false),
        );
        assert!(observed.read(read_request("intro/created")).is_err());

        runtime
            .abort_runtime_transaction(&mut context, "store failure cleanup")
            .unwrap();
    }

    #[test]
    fn administrative_docs_write_executes_immediately() {
        let provider = InMemoryDocsProvider::new();
        let observed = provider.clone();
        let mut runtime = crate::MechRuntime::builder()
            .resource_provider(Box::new(provider))
            .build()
            .unwrap();
        grant_docs_write(&mut runtime);

        runtime
            .write_resource(write_request("intro/enabled", true))
            .unwrap();

        assert_eq!(
            observed.read(read_request("intro/enabled")).unwrap(),
            bool_value(true),
        );
    }

    #[test]
    fn provider_panics_are_converted_before_external_effects_exist() {
        let mut runtime = crate::MechRuntime::builder()
            .resource_provider(Box::new(PanickingProvider))
            .build()
            .unwrap();
        grant_panic_resource(&mut runtime, "read");
        grant_panic_resource(&mut runtime, "write");

        let read_error = runtime
            .read_resource(RuntimeResourceReadRequest {
                base_uri: "panic://provider".to_string(),
                path: "value".to_string(),
                context_name: "panic".to_string(),
            })
            .unwrap_err();
        assert_eq!(read_error.kind_name(), "RuntimeExtensionPanicked");
        assert!(format!("{read_error:?}").contains("deliberate provider read panic"));

        let write_error = runtime
            .write_resource(RuntimeResourceWriteRequest {
                base_uri: "panic://provider".to_string(),
                path: "value".to_string(),
                context_name: "panic".to_string(),
                operation: RuntimeCapabilityOperation::Write,
                value: bool_value(true),
                intent: RuntimeResourceWriteIntent::Assign,
            })
            .unwrap_err();
        assert_eq!(write_error.kind_name(), "RuntimeExtensionPanicked");
        assert!(format!("{write_error:?}").contains("deliberate provider prepare panic"));
        assert!(!runtime.is_poisoned());
        runtime.list_events(None).unwrap();
    }
}
