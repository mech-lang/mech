use std::collections::BTreeSet;

use mech_core::{MResult, MechError, MechErrorKind};

use crate::{
  Capability, CapabilityDecision, CapabilityId, CapabilityRequest,
  RuntimeCapabilityGrantSpec,
};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RuntimeResourceKey {
  pub base_uri: String,
  pub path: String,
}

impl RuntimeResourceKey {
  pub fn new(
    base_uri: impl Into<String>,
    path: impl Into<String>,
  ) -> MResult<Self> {
    let base_uri = normalize_base_uri(&base_uri.into())?;
    let path = normalize_resource_path(&path.into())?;
    Ok(Self { base_uri, path })
  }

  pub fn capability_resource(&self) -> String {
    if self.path.is_empty() {
      self.base_uri.clone()
    } else {
      format!("{}/{}", self.base_uri, self.path)
    }
  }
}

impl std::fmt::Display for RuntimeResourceKey {
  fn fmt(
    &self,
    formatter: &mut std::fmt::Formatter<'_>,
  ) -> std::fmt::Result {
    formatter.write_str(&self.capability_resource())
  }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResourcePathScope {
  Exact(String),
  Prefix(String),
  Wildcard,
}

impl ResourcePathScope {
  fn from_config_path(path: &str) -> MResult<Self> {
    if path.trim().is_empty() {
      return invalid_resource_capability(
        "configuration path must not be empty",
      );
    }
    if path == "*" {
      return Ok(Self::Wildcard);
    }
    if let Some(prefix) = path.strip_suffix("/*") {
      return Ok(Self::Prefix(normalize_resource_path(prefix)?));
    }
    Ok(Self::Exact(normalize_resource_path(path)?))
  }

  fn matches(&self, path: &str) -> bool {
    match self {
      Self::Exact(exact) => exact == path,
      Self::Prefix(prefix) => {
        path == prefix
          || path
            .strip_prefix(prefix)
            .is_some_and(|remainder| remainder.starts_with('/'))
      }
      Self::Wildcard => true,
    }
  }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourcePathCapability {
  id: CapabilityId,
  subject: String,
  base_uri: String,
  operations: BTreeSet<String>,
  scopes: Vec<ResourcePathScope>,
  revocable: bool,
}

impl ResourcePathCapability {
  pub fn new(
    id: CapabilityId,
    subject: impl Into<String>,
    base_uri: impl Into<String>,
    operations: impl IntoIterator<Item = impl Into<String>>,
    scopes: impl IntoIterator<Item = ResourcePathScope>,
  ) -> MResult<Self> {
    let capability = Self {
      id,
      subject: subject.into(),
      base_uri: normalize_base_uri(&base_uri.into())?,
      operations: operations.into_iter().map(Into::into).collect(),
      scopes: scopes.into_iter().collect(),
      revocable: true,
    };
    capability.validate()?;
    Ok(capability)
  }

  pub fn from_spec(
    id: CapabilityId,
    spec: &RuntimeCapabilityGrantSpec,
  ) -> MResult<Self> {
    let scopes = spec
      .paths
      .iter()
      .map(|path| ResourcePathScope::from_config_path(path))
      .collect::<MResult<Vec<_>>>()?;
    Self::new(
      id,
      spec.subject.clone(),
      spec.resource.clone(),
      spec
        .operations
        .iter()
        .map(|operation| operation.name().to_string()),
      scopes,
    )
  }

  pub fn exact(
    id: CapabilityId,
    subject: impl Into<String>,
    base_uri: impl Into<String>,
    operations: impl IntoIterator<Item = impl Into<String>>,
    path: impl Into<String>,
  ) -> MResult<Self> {
    Self::new(
      id,
      subject,
      base_uri,
      operations,
      [ResourcePathScope::Exact(normalize_resource_path(
        &path.into(),
      )?)],
    )
  }

  pub fn prefix(
    id: CapabilityId,
    subject: impl Into<String>,
    base_uri: impl Into<String>,
    operations: impl IntoIterator<Item = impl Into<String>>,
    path: impl Into<String>,
  ) -> MResult<Self> {
    Self::new(
      id,
      subject,
      base_uri,
      operations,
      [ResourcePathScope::Prefix(normalize_resource_path(
        &path.into(),
      )?)],
    )
  }

  pub fn wildcard(
    id: CapabilityId,
    subject: impl Into<String>,
    base_uri: impl Into<String>,
    operations: impl IntoIterator<Item = impl Into<String>>,
  ) -> MResult<Self> {
    Self::new(
      id,
      subject,
      base_uri,
      operations,
      [ResourcePathScope::Wildcard],
    )
  }

  pub fn base_uri(&self) -> &str { &self.base_uri }
  pub fn operations(&self) -> &BTreeSet<String> { &self.operations }
  pub fn scopes(&self) -> &[ResourcePathScope] { &self.scopes }

  pub fn revocable(mut self, revocable: bool) -> Self {
    self.revocable = revocable;
    self
  }

  fn check_request(
    &self,
    request: &CapabilityRequest,
  ) -> MResult<CapabilityDecision> {
    self.validate()?;
    if request.subject != self.subject {
      return Ok(CapabilityDecision::deny(
        "capability belongs to another subject",
      ));
    }
    if !self.operations.contains(&request.operation) {
      return Ok(CapabilityDecision::deny(
        "operation is not allowed",
      ));
    }
    let base_prefix = format!("{}/", self.base_uri);
    let path = if request.resource == self.base_uri {
      ""
    } else if let Some(path) =
      request.resource.strip_prefix(&base_prefix)
    {
      path
    } else {
      return Ok(CapabilityDecision::deny(
        "resource base URI is not allowed",
      ));
    };
    let path = normalize_resource_path(path)?;
    if self.scopes.iter().any(|scope| scope.matches(&path)) {
      Ok(CapabilityDecision::allow())
    } else {
      Ok(CapabilityDecision::deny(
        "resource path is outside the capability scope",
      ))
    }
  }
}

impl Capability for ResourcePathCapability {
  fn id(&self) -> CapabilityId { self.id }

  fn subject_key(&self) -> &str { &self.subject }

  fn validate(&self) -> MResult<()> {
    if self.id.is_zero() {
      return invalid_resource_capability("id must not be zero");
    }
    if self.subject.trim().is_empty() {
      return invalid_resource_capability(
        "subject must not be empty",
      );
    }
    if self.base_uri.trim().is_empty() {
      return invalid_resource_capability(
        "base URI must not be empty",
      );
    }
    if self.operations.is_empty()
      || self.operations.iter().any(|operation| {
        operation.trim().is_empty()
      })
    {
      return invalid_resource_capability(
        "operations must contain non-empty names",
      );
    }
    if self.scopes.is_empty() {
      return invalid_resource_capability(
        "at least one path scope is required",
      );
    }
    Ok(())
  }

  fn check(
    &self,
    request: &CapabilityRequest,
  ) -> MResult<CapabilityDecision> {
    self.check_request(request)
  }

  fn preview_check(
    &self,
    request: &CapabilityRequest,
  ) -> MResult<CapabilityDecision> {
    self.check_request(request)
  }

  fn is_revocable(&self) -> bool { self.revocable }
}

fn normalize_base_uri(base_uri: &str) -> MResult<String> {
  crate::canonicalize_resource_base_uri(base_uri.trim()).map_err(
    |error| {
      MechError::new(
        RuntimeCapabilityGrantInvalid {
          reason: format!(
            "invalid base URI: {}",
            error.full_chain_message(),
          ),
        },
        None,
      )
    },
  )
}

fn normalize_resource_path(path: &str) -> MResult<String> {
  let mut segments = Vec::new();
  for segment in path.trim().split('/') {
    match segment {
      "" | "." => {}
      ".." => {
        return invalid_resource_capability(
          "resource path must not contain `..`",
        );
      }
      segment => segments.push(segment),
    }
  }
  Ok(segments.join("/"))
}

fn invalid_resource_capability<T>(
  reason: impl Into<String>,
) -> MResult<T> {
  Err(MechError::new(
    RuntimeCapabilityGrantInvalid {
      reason: reason.into(),
    },
    None,
  ))
}

#[derive(Debug, Clone)]
pub struct RuntimeCapabilityGrantInvalid {
  pub reason: String,
}

impl MechErrorKind for RuntimeCapabilityGrantInvalid {
  fn name(&self) -> &str {
    "RuntimeCapabilityGrantInvalid"
  }

  fn message(&self) -> String {
    format!("invalid runtime capability grant: {}", self.reason)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn prefix_scope_checks_path_boundaries() {
    let capability = ResourcePathCapability::prefix(
      CapabilityId(1),
      "task:1",
      "docs://manual",
      ["read"],
      "chapter",
    )
    .unwrap();

    assert!(capability
      .preview_check(&CapabilityRequest::from_keys(
        "task:1",
        "read",
        "docs://manual/chapter/one",
      ))
      .unwrap()
      .allowed);
    assert!(!capability
      .preview_check(&CapabilityRequest::from_keys(
        "task:1",
        "read",
        "docs://manual/chapter-two",
      ))
      .unwrap()
      .allowed);
  }

  #[test]
  fn exact_and_wildcard_scopes_use_canonical_full_resources() {
    let exact = ResourcePathCapability::exact(
      CapabilityId(1),
      "task:1",
      "docs://manual/intro",
      ["read"],
      "title",
    )
    .unwrap();
    let wildcard = ResourcePathCapability::wildcard(
      CapabilityId(2),
      "task:1",
      "docs://manual",
      ["read"],
    )
    .unwrap();

    let title = CapabilityRequest::from_keys(
      "task:1",
      "read",
      "docs://manual/intro/title",
    );
    let sibling = CapabilityRequest::from_keys(
      "task:1",
      "read",
      "docs://manual/intro/other",
    );

    assert!(exact.preview_check(&title).unwrap().allowed);
    assert!(!exact.preview_check(&sibling).unwrap().allowed);
    assert!(wildcard.preview_check(&title).unwrap().allowed);
  }

  #[test]
  fn resource_key_rejects_parent_segments() {
    let error = RuntimeResourceKey::new(
      "docs://manual/",
      "intro/../secret",
    )
    .unwrap_err();
    assert_eq!(error.kind_name(), "RuntimeCapabilityGrantInvalid");
  }
}
