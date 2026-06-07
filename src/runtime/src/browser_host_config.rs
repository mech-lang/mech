#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{MechConfigDocument, RuntimeConfig};
use mech_core::{
  BrowserAuthority, BrowserCapabilityGrant, BrowserDomManifestEntry as RuntimeBrowserDomManifestEntry,
  BrowserDomPath, BrowserDomProperty, BrowserDomScope, BrowserNetworkScope, BrowserOperation,
  BrowserResource, BrowserStorageBackend, BrowserStorageScope, MResult, MechError, MechErrorKind,
};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserHostConfig {
  pub runtime: BrowserHostRuntimeConfig,
  pub browser: BrowserHostBrowserConfig,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserHostRuntimeConfig {
  pub name: String,
  pub limits: BrowserHostRuntimeLimits,
  pub diagnostics: BrowserHostDiagnosticsConfig,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserHostRuntimeLimits {
  pub max_steps_per_turn: Option<u64>,
  pub max_turn_duration_ms: Option<u64>,
  pub max_memory_bytes: Option<u64>,
  pub max_tasks: Option<u64>,
  pub max_actors: Option<u64>,
  pub max_actor_mailbox_len: Option<u64>,
  pub max_source_bytes: Option<u64>,
  pub max_in_memory_events: Option<u64>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserHostDiagnosticsConfig {
  pub trace_enabled: bool,
  pub profile_enabled: bool,
  pub debug_enabled: bool,
  pub log_level: String,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserHostBrowserConfig {
  pub grants: Vec<BrowserHostBrowserGrant>,
  pub dom: Vec<BrowserHostDomManifestEntry>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserHostBrowserGrant {
  pub resource: BrowserHostResourceConfig,
  pub allow: Vec<String>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "kind", rename_all = "kebab-case"))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BrowserHostResourceConfig {
  Dom { selector: String },
  Clipboard,
  Network { origin: String, methods: Option<Vec<String>> },
  Storage { backend: String, scope: String, recursive: bool },
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserHostDomManifestEntry {
  pub path: String,
  pub selector: String,
  pub property: String,
  #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
  pub attribute: Option<String>,
}

impl BrowserHostConfig {
  pub fn from_document_and_runtime(
    document: &MechConfigDocument,
    runtime_config: &RuntimeConfig,
  ) -> Self {
    let grants = document
      .browser
      .grants()
      .iter()
      .map(|grant| BrowserHostBrowserGrant {
        resource: BrowserHostResourceConfig::from(&grant.resource),
        allow: grant
          .allow
          .iter()
          .map(|operation| operation.as_str().to_string())
          .collect(),
      })
      .collect();

    let dom = document
      .browser
      .dom_manifest()
      .iter()
      .map(|entry| BrowserHostDomManifestEntry {
        path: entry.path.as_str().to_string(),
        selector: entry.selector.selector.clone(),
        property: entry.property.config_name().to_string(),
        attribute: entry.property.config_attribute().map(str::to_string),
      })
      .collect();

    Self {
      runtime: BrowserHostRuntimeConfig::from(runtime_config),
      browser: BrowserHostBrowserConfig { grants, dom },
    }
  }

  pub fn into_runtime_config(&self) -> MResult<RuntimeConfig> {
    let log_level = match self.runtime.diagnostics.log_level.as_str() {
      "error" => crate::LogLevel::Error,
      "warn" => crate::LogLevel::Warn,
      "info" => crate::LogLevel::Info,
      "debug" => crate::LogLevel::Debug,
      "trace" => crate::LogLevel::Trace,
      other => return Err(invalid_host_config(format!("unknown runtime log level `{other}`"))),
    };
    let config = RuntimeConfig {
      name: self.runtime.name.clone(),
      limits: crate::RuntimeLimits {
        max_steps_per_turn: self.runtime.limits.max_steps_per_turn,
        max_turn_duration_ms: self.runtime.limits.max_turn_duration_ms,
        max_memory_bytes: self.runtime.limits.max_memory_bytes,
        max_tasks: self.runtime.limits.max_tasks,
        max_actors: self.runtime.limits.max_actors,
        max_actor_mailbox_len: self.runtime.limits.max_actor_mailbox_len,
        max_source_bytes: self.runtime.limits.max_source_bytes,
        max_in_memory_events: self.runtime.limits.max_in_memory_events,
      },
      diagnostics: crate::DiagnosticsConfig {
        trace_enabled: self.runtime.diagnostics.trace_enabled,
        profile_enabled: self.runtime.diagnostics.profile_enabled,
        debug_enabled: self.runtime.diagnostics.debug_enabled,
        log_level,
      },
    };
    config.validate()?;
    Ok(config)
  }

  pub fn into_browser_authority(&self) -> MResult<BrowserAuthority> {
    let mut authority = BrowserAuthority::default();

    for entry in &self.browser.dom {
      authority.bind_dom_path(RuntimeBrowserDomManifestEntry::new(
        BrowserDomPath::new(entry.path.clone()).map_err(host_config_error)?,
        BrowserDomScope::new(entry.selector.clone()).map_err(host_config_error)?,
        BrowserDomProperty::parse_config_name(&entry.property, entry.attribute.as_deref())
          .map_err(host_config_error)?,
      ));
    }

    for grant in &self.browser.grants {
      let allow = grant
        .allow
        .iter()
        .map(|operation| {
          BrowserOperation::parse(operation)
            .ok_or_else(|| invalid_host_config(format!("unknown browser operation `{operation}`")))
        })
        .collect::<MResult<Vec<_>>>()?;
      authority.grant(BrowserCapabilityGrant::new(grant.resource.to_browser_resource()?, allow));
    }

    Ok(authority)
  }

  pub fn into_runtime_and_browser_authority(self) -> MResult<(RuntimeConfig, BrowserAuthority)> {
    Ok((self.into_runtime_config()?, self.into_browser_authority()?))
  }
}

impl From<&RuntimeConfig> for BrowserHostRuntimeConfig {
  fn from(config: &RuntimeConfig) -> Self {
    Self {
      name: config.name.clone(),
      limits: BrowserHostRuntimeLimits {
        max_steps_per_turn: config.limits.max_steps_per_turn,
        max_turn_duration_ms: config.limits.max_turn_duration_ms,
        max_memory_bytes: config.limits.max_memory_bytes,
        max_tasks: config.limits.max_tasks,
        max_actors: config.limits.max_actors,
        max_actor_mailbox_len: config.limits.max_actor_mailbox_len,
        max_source_bytes: config.limits.max_source_bytes,
        max_in_memory_events: config.limits.max_in_memory_events,
      },
      diagnostics: BrowserHostDiagnosticsConfig {
        trace_enabled: config.diagnostics.trace_enabled,
        profile_enabled: config.diagnostics.profile_enabled,
        debug_enabled: config.diagnostics.debug_enabled,
        log_level: config.diagnostics.log_level.as_str().to_string(),
      },
    }
  }
}

impl From<&BrowserResource> for BrowserHostResourceConfig {
  fn from(resource: &BrowserResource) -> Self {
    match resource {
      BrowserResource::Dom(scope) => Self::Dom {
        selector: scope.selector.clone(),
      },
      BrowserResource::Clipboard => Self::Clipboard,
      BrowserResource::Network(scope) => Self::Network {
        origin: scope.origin.clone(),
        methods: scope
          .methods
          .as_ref()
          .map(|methods| methods.iter().cloned().collect()),
      },
      BrowserResource::Storage(scope) => Self::Storage {
        backend: scope.backend.to_string(),
        scope: scope.scope.clone(),
        recursive: scope.recursive,
      },
    }
  }
}

impl BrowserHostResourceConfig {
  pub fn to_browser_resource(&self) -> MResult<BrowserResource> {
    match self {
      Self::Dom { selector } => Ok(BrowserResource::Dom(
        BrowserDomScope::new(selector.clone()).map_err(host_config_error)?,
      )),
      Self::Clipboard => Ok(BrowserResource::Clipboard),
      Self::Network { origin, methods } => Ok(BrowserResource::Network(
        BrowserNetworkScope::new(origin.clone(), methods.clone()).map_err(host_config_error)?,
      )),
      Self::Storage { backend, scope, recursive } => {
        let backend = BrowserStorageBackend::parse(backend)
          .ok_or_else(|| invalid_host_config(format!("unknown storage backend `{backend}`")))?;
        Ok(BrowserResource::Storage(
          BrowserStorageScope::new(backend, scope.clone())
            .map_err(host_config_error)?
            .with_recursive(*recursive),
        ))
      }
    }
  }
}

#[derive(Debug, Clone)]
pub struct InvalidBrowserHostConfigError {
  pub reason: String,
}

impl MechErrorKind for InvalidBrowserHostConfigError {
  fn name(&self) -> &str {
    "InvalidBrowserHostConfig"
  }

  fn message(&self) -> String {
    format!("Invalid browser host config: {}", self.reason)
  }
}

fn invalid_host_config(reason: impl Into<String>) -> MechError {
  MechError::new(InvalidBrowserHostConfigError { reason: reason.into() }, None)
}

fn host_config_error(error: impl std::fmt::Display) -> MechError {
  invalid_host_config(error.to_string())
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{parse_config_document, ConfigProfileOptions, LogLevel, RuntimeConfig};
  use mech_core::{BrowserDomProperty, BrowserOperation, BrowserResourceKind};

  #[test]
  fn enum_string_mappings_live_on_domain_types() {
    assert_eq!(BrowserOperation::Read.as_str(), "read");
    assert_eq!(BrowserResourceKind::Network.as_str(), "network");
    assert_eq!(BrowserDomProperty::InnerHtml.config_name(), "inner-html");
    assert_eq!(LogLevel::Debug.as_str(), "debug");
  }

  #[test]
  fn browser_host_config_projects_runtime_browser_grants_and_dom_manifest() {
    let source = r##"config := {
  runtime: {
    name: "browser-test"
    limits: {max-steps-per-turn: 42}
    diagnostics: {debug-enabled: true log-level: "debug"}
  }
  browser: {
    dom: [
      {
        path: "body/content/title"
        selector: "#title"
        property: "text"
        allow: ["write"]
      }
      {
        path: "body/content/status/_class"
        selector: "#status"
        property: "attribute"
        attribute: "class"
        allow: ["write"]
      }
    ]
  }
}"##;
    let document = parse_config_document(
      "test.mcfg".to_string(),
      source,
      ConfigProfileOptions::default(),
    )
    .unwrap();
    let runtime = RuntimeConfig::default().apply_patch(&document.runtime).unwrap();
    let host = BrowserHostConfig::from_document_and_runtime(&document, &runtime);

    assert_eq!(host.runtime.name, "browser-test");
    assert_eq!(host.runtime.limits.max_steps_per_turn, Some(42));
    assert!(host.runtime.diagnostics.debug_enabled);
    assert_eq!(host.runtime.diagnostics.log_level, "debug");
    assert_eq!(host.browser.grants.len(), 2);
    assert!(host.browser.grants.iter().any(|grant| grant.allow == ["write"]));
    assert!(host.browser.dom.iter().any(|entry| {
      entry.path == "body/content/title"
        && entry.selector == "#title"
        && entry.property == "text"
        && entry.attribute.is_none()
    }));
    assert!(host.browser.dom.iter().any(|entry| {
      entry.path == "body/content/status/_class"
        && entry.selector == "#status"
        && entry.property == "attribute"
        && entry.attribute.as_deref() == Some("class")
    }));
  }

  #[test]
  fn browser_host_config_converts_back_to_runtime_and_authority() {
    let source = r##"config := {
  runtime: {name: "browser-test" diagnostics: {log-level: "trace"}}
  browser: {
    dom: [
      {path: "body/title" selector: "#title" property: "text" allow: ["write"]}
    ]
  }
}"##;
    let document = parse_config_document(
      "test.mcfg".to_string(),
      source,
      ConfigProfileOptions::default(),
    )
    .unwrap();
    let runtime = RuntimeConfig::default().apply_patch(&document.runtime).unwrap();
    let host = BrowserHostConfig::from_document_and_runtime(&document, &runtime);
    let (runtime, authority) = host.into_runtime_and_browser_authority().unwrap();

    assert_eq!(runtime.name, "browser-test");
    assert_eq!(runtime.diagnostics.log_level, LogLevel::Trace);
    assert_eq!(authority.grants().len(), 1);
    assert_eq!(authority.dom_manifest().len(), 1);
    authority.allows_dom("#title", BrowserOperation::Write).unwrap();
  }

  #[cfg(feature = "serde")]
  #[test]
  fn browser_host_config_serializes_expected_shape() {
    let source = r##"config := {
  browser: {
    dom: [
      {path: "body/title" selector: "#title" property: "text" allow: ["write"]}
    ]
  }
}"##;
    let document = parse_config_document(
      "test.mcfg".to_string(),
      source,
      ConfigProfileOptions::default(),
    )
    .unwrap();
    let host = BrowserHostConfig::from_document_and_runtime(&document, &RuntimeConfig::default());
    let value = serde_json::to_value(&host).unwrap();

    assert_eq!(value["runtime"]["limits"]["maxStepsPerTurn"], 10_000);
    assert_eq!(value["runtime"]["diagnostics"]["logLevel"], "info");
    assert_eq!(value["browser"]["dom"][0]["property"], "text");
    assert_eq!(value["browser"]["grants"][0]["resource"]["kind"], "dom");
    assert_eq!(value["browser"]["grants"][0]["allow"][0], "write");
  }
}
