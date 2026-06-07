#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{MechConfigDocument, RuntimeConfig};
use mech_core::BrowserResource;

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
