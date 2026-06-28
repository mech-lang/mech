use std::collections::HashMap;

use mech_core::{MResult, MechError, MechErrorKind, Value};

use crate::RuntimeCapabilityOperation;

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

  fn write(&mut self, request: RuntimeResourceWriteRequest) -> MResult<()> {
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
      let has_left = entry.bases.iter().any(|base| base.trim_end_matches('/') == left);
      let has_right = entry.bases.iter().any(|base| base.trim_end_matches('/') == right);
      has_left && has_right
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

  pub fn write(&mut self, request: RuntimeResourceWriteRequest) -> MResult<()> {
    let scheme = resource_uri_scheme(&request.base_uri)?.to_string();
    let Some(entry) = self.provider_entry_for_mut(&scheme, &request.base_uri) else {
      return Err(MechError::new(
        RuntimeResourceProviderNotFound { scheme, uri: request.base_uri },
        None,
      ));
    };
    entry.provider.write(request)
  }
}

#[derive(Clone, Debug, Default)]
pub struct InMemoryDocsProvider {
  documents: HashMap<String, HashMap<String, Value>>,
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
    self.documents.entry(base_uri).or_default().insert(path, value);
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
    self.documents.keys().cloned().collect()
  }

  fn read(&self, request: RuntimeResourceReadRequest) -> MResult<Value> {
    let Some(document) = self.documents.get(&request.base_uri) else {
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

  fn write(&mut self, request: RuntimeResourceWriteRequest) -> MResult<()> {
    self.preflight_write(RuntimeResourceWritePreflightRequest {
      base_uri: request.base_uri.clone(),
      path: request.path.clone(),
      context_name: request.context_name.clone(),
      operation: request.operation.clone(),
      intent: request.intent,
    })?;

    self.documents
      .entry(request.base_uri)
      .or_default()
      .insert(request.path, request.value);

    Ok(())
  }
}

pub fn resource_base_matches(base: &str, candidate: &str) -> bool {
  candidate == base || candidate.strip_prefix(base).is_some_and(|suffix| suffix.starts_with('/'))
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
