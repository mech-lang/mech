use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use mech_core::{MResult, MechError, MechErrorKind, Value};

use crate::{
  PreparedRuntimeEffect, RuntimeCapabilityOperation,
  RuntimeCompensatableEffect, RuntimeEffectCost, RuntimeEffectMetadata,
  RuntimeEffectSource,
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

  fn base_uris(&self) -> Vec<String> { Vec::new() }

  fn equivalent_base_uri_groups(&self) -> Vec<Vec<String>> { Vec::new() }

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

  fn stage_write(
    &mut self,
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
  provider: Box<dyn RuntimeResourceProvider>,
}

#[derive(Debug, Default)]
pub struct RuntimeResourceRegistry {
  providers: Vec<RuntimeResourceProviderEntry>,
}

impl RuntimeResourceRegistry {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn register_provider(
    &mut self,
    provider: Box<dyn RuntimeResourceProvider>,
  ) -> MResult<()> {
    let scheme = provider.scheme().to_string();
    if scheme.is_empty() {
      return Err(MechError::new(
        RuntimeResourceInvalidUri {
          uri: String::new(),
          reason: "resource provider scheme cannot be empty".to_string(),
        },
        None,
      ));
    }

    let bases = provider.base_uris();
    for base in &bases {
      let base_scheme = resource_uri_scheme(base)?;
      if base_scheme != scheme {
        return Err(MechError::new(
          RuntimeResourceInvalidUri {
            uri: base.clone(),
            reason: format!("resource provider base URI scheme must be `{scheme}`"),
          },
          None,
        ));
      }
      resource_uri_origin(base)?;
      if self.providers.iter().any(|entry| entry.bases.iter().any(|existing| existing == base)) {
        return Err(MechError::new(
          RuntimeResourceProviderConflict { scheme: scheme.clone() },
          None,
        ));
      }
    }

    if bases.is_empty() && self.providers.iter().any(|entry| entry.scheme == scheme && entry.bases.is_empty()) {
      return Err(MechError::new(
        RuntimeResourceProviderConflict { scheme: scheme.clone() },
        None,
      ));
    }

    self.providers.push(RuntimeResourceProviderEntry { scheme, bases, provider });
    Ok(())
  }

  pub fn has_provider(&self, scheme: &str) -> bool {
    self.providers.iter().any(|entry| entry.scheme == scheme)
  }

  pub fn provider_base_uri_for(&self, candidate: &str) -> MResult<Option<String>> {
    let scheme = resource_uri_scheme(candidate)?.to_string();
    let Some(entry) = self.provider_entry_for(&scheme, candidate) else {
      return Ok(None);
    };
    if let Some(base) = entry.bases.iter().filter(|base| resource_base_matches(base, candidate)).max_by_key(|base| base.len()) {
      return Ok(Some(base.clone()));
    }
    Ok(Some(resource_uri_origin(candidate)?.to_string()))
  }

  pub fn base_uris_equivalent(&self, left: &str, right: &str) -> bool {
    let left = left.trim_end_matches('/');
    let right = right.trim_end_matches('/');

    if left == right {
      return true;
    }

    self.providers.iter().any(|entry| {
      entry.provider.equivalent_base_uri_groups().iter().any(|group| {
        let has_left = group.iter().any(|base| base.trim_end_matches('/') == left);
        let has_right = group.iter().any(|base| base.trim_end_matches('/') == right);
        has_left && has_right
      })
    })
  }

  fn provider_entry_for(&self, scheme: &str, uri: &str) -> Option<&RuntimeResourceProviderEntry> {
    self.providers
      .iter()
      .filter(|entry| entry.scheme == scheme && entry.bases.iter().any(|base| resource_base_matches(base, uri)))
      .max_by_key(|entry| entry.bases.iter().filter(|base| resource_base_matches(base, uri)).map(|base| base.len()).max().unwrap_or(0))
      .or_else(|| self.providers.iter().find(|entry| entry.scheme == scheme && entry.bases.is_empty()))
  }

  fn provider_entry_for_mut(&mut self, scheme: &str, uri: &str) -> Option<&mut RuntimeResourceProviderEntry> {
    let index = self.providers
      .iter()
      .enumerate()
      .filter(|(_, entry)| entry.scheme == scheme && entry.bases.iter().any(|base| resource_base_matches(base, uri)))
      .max_by_key(|(_, entry)| entry.bases.iter().filter(|base| resource_base_matches(base, uri)).map(|base| base.len()).max().unwrap_or(0))
      .map(|(index, _)| index)
      .or_else(|| self.providers.iter().position(|entry| entry.scheme == scheme && entry.bases.is_empty()))?;
    self.providers.get_mut(index)
  }

  pub fn read(&self, request: RuntimeResourceReadRequest) -> MResult<Value> {
    let scheme = resource_uri_scheme(&request.base_uri)?.to_string();
    let Some(entry) = self.provider_entry_for(&scheme, &request.base_uri) else {
      return Err(MechError::new(
        RuntimeResourceProviderNotFound { scheme, uri: request.base_uri },
        None,
      ));
    };
    entry.provider.read(request)
  }

  pub fn preflight_write(&self, request: RuntimeResourceWritePreflightRequest) -> MResult<()> {
    let scheme = resource_uri_scheme(&request.base_uri)?.to_string();
    let Some(entry) = self.provider_entry_for(&scheme, &request.base_uri) else {
      return Err(MechError::new(
        RuntimeResourceProviderNotFound { scheme, uri: request.base_uri },
        None,
      ));
    };
    entry.provider.preflight_write(request)
  }

  pub fn stage_write(
    &mut self,
    request: RuntimeResourceWriteRequest,
  ) -> MResult<PreparedRuntimeEffect> {
    let scheme = resource_uri_scheme(&request.base_uri)?.to_string();
    let Some(entry) = self.provider_entry_for_mut(&scheme, &request.base_uri) else {
      return Err(MechError::new(
        RuntimeResourceProviderNotFound { scheme, uri: request.base_uri },
        None,
      ));
    };
    entry.provider.stage_write(request)
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
    self
      .documents
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
}

impl RuntimeResourceProvider for InMemoryDocsProvider {
  fn scheme(&self) -> &str {
    "docs"
  }

  fn base_uris(&self) -> Vec<String> {
    self
      .documents
      .lock()
      .map(|documents| documents.keys().cloned().collect())
      .unwrap_or_default()
  }

  fn read(&self, request: RuntimeResourceReadRequest) -> MResult<Value> {
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
    Ok(value.clone())
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

  fn stage_write(
    &mut self,
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
    .with_cost(RuntimeEffectCost {
      bytes: 0,
      items: 1,
    })
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
        let remove_base = if let Some(document) =
          documents.get_mut(&self.base_uri)
        {
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

pub fn resource_base_matches(base: &str, candidate: &str) -> bool {
  candidate == base || candidate.strip_prefix(base).is_some_and(|suffix| suffix.starts_with('/'))
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
      self.scheme,
      self.uri,
    )
  }
}

#[derive(Debug, Clone)]
pub struct RuntimeResourceWriteUnsupported {
  pub scheme: String,
  pub base_uri: String,
  pub path: String,
}

impl MechErrorKind for RuntimeResourceWriteUnsupported {
  fn name(&self) -> &str {
    "RuntimeResourceWriteUnsupported"
  }

  fn message(&self) -> String {
    format!(
      "runtime resource provider for scheme `{}` does not support writes to `{}` under `{}`",
      self.scheme,
      self.path,
      self.base_uri,
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
    format!("runtime resource provider for scheme `{}` is already registered", self.scheme)
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
    format!("resource path `{}` was not found under `{}`", self.path, self.base_uri)
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
      self.context_name,
      self.operation,
      self.path,
    )
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use mech_core::Ref;

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

  #[test]
  fn docs_write_is_invisible_until_explicit_commit() {
    let provider = InMemoryDocsProvider::new();
    let observed = provider.clone();
    let mut runtime = crate::MechRuntime::builder()
      .resource_provider(Box::new(provider))
      .build()
      .unwrap();
    let mut context = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut context).unwrap();

    let effect_id = runtime
      .write_resource_with_context(
        &mut context,
        write_request("intro/enabled", true),
      )
      .unwrap();

    assert_eq!(effect_id.sequence, 0);
    assert!(observed.read(read_request("intro/enabled")).is_err());

    runtime
      .commit_runtime_transaction(&mut context)
      .unwrap();
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
    let mut context = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut context).unwrap();
    runtime
      .write_resource_with_context(
        &mut context,
        write_request("intro/enabled", true),
      )
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
    let mut context = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut context).unwrap();
    runtime
      .write_resource_with_context(
        &mut context,
        write_request("intro/enabled", true),
      )
      .unwrap();
    runtime
      .write_resource_with_context(
        &mut context,
        write_request("intro/created", true),
      )
      .unwrap();
    runtime
      .update_object_with_context(
        &mut context,
        crate::ObjectRecord::text(
          crate::ObjectId(990),
          "missing",
          "update",
        ),
      )
      .unwrap();

    assert!(runtime
      .commit_runtime_transaction(&mut context)
      .is_err());

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

    runtime
      .write_resource(write_request("intro/enabled", true))
      .unwrap();

    assert_eq!(
      observed.read(read_request("intro/enabled")).unwrap(),
      bool_value(true),
    );
  }
}
