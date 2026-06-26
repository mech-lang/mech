#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use mech_core::{
  BrowserAuthority, BrowserCapabilityGrant, BrowserDomManifestEntry, BrowserDomPath,
  BrowserDomProperty, BrowserDomScope, BrowserNetworkScope, BrowserOperation, BrowserResource,
  BrowserStorageBackend, BrowserStorageScope, MResult, MechError,
  MechErrorKind,
};

use mech_runtime::{
  ConfigValue, DiagnosticsConfig, LogLevel, MechConfigDocument, RuntimeConfig, RuntimeLimits,
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
  pub dom_manifest: Vec<BrowserHostDomManifestEntry>,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidBrowserHostConfigError {
  pub field: &'static str,
  pub reason: String,
}

impl MechErrorKind for InvalidBrowserHostConfigError {
  fn name(&self) -> &str {
    "InvalidBrowserHostConfigError"
  }

  fn message(&self) -> String {
    format!("Invalid browser host config field `{}`: {}", self.field, self.reason)
  }
}

impl BrowserHostConfig {
  pub fn from_document_and_runtime(
    document: &MechConfigDocument,
    runtime_config: &RuntimeConfig,
  ) -> Self {
    let browser = browser_config_from_hosts(document).unwrap_or_else(|_| BrowserHostBrowserConfig {
      grants: Vec::new(),
      dom_manifest: Vec::new(),
    });
    Self {
      runtime: BrowserHostRuntimeConfig::from(runtime_config),
      browser,
    }
  }


  pub fn into_runtime_config(&self) -> MResult<RuntimeConfig> {
    let log_level = match self.runtime.diagnostics.log_level.as_str() {
      "error" => LogLevel::Error,
      "warn" => LogLevel::Warn,
      "info" => LogLevel::Info,
      "debug" => LogLevel::Debug,
      "trace" => LogLevel::Trace,
      other => return invalid("runtime.diagnostics.logLevel", format!("unknown log level `{other}`")),
    };
    let config = RuntimeConfig {
      name: self.runtime.name.clone(),
      limits: RuntimeLimits {
        max_steps_per_turn: self.runtime.limits.max_steps_per_turn,
        max_turn_duration_ms: self.runtime.limits.max_turn_duration_ms,
        max_memory_bytes: self.runtime.limits.max_memory_bytes,
        max_tasks: self.runtime.limits.max_tasks,
        max_actors: self.runtime.limits.max_actors,
        max_actor_mailbox_len: self.runtime.limits.max_actor_mailbox_len,
        max_source_bytes: self.runtime.limits.max_source_bytes,
        max_in_memory_events: self.runtime.limits.max_in_memory_events,
      },
      diagnostics: DiagnosticsConfig {
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
    for entry in &self.browser.dom_manifest {
      let path = BrowserDomPath::new(&entry.path).map_err(|error| invalid_error(
        "browser.domManifest.path",
        format!("invalid DOM path `{}`: {error}", entry.path),
      ))?;
      let selector = BrowserDomScope::new(&entry.selector).map_err(|error| invalid_error(
        "browser.domManifest.selector",
        format!("invalid DOM selector `{}`: {error}", entry.selector),
      ))?;
      let property = BrowserDomProperty::parse_config_name(
        &entry.property,
        entry.attribute.as_deref(),
      ).map_err(|error| invalid_error(
        "browser.domManifest.property",
        format!("invalid DOM property `{}`: {error}", entry.property),
      ))?;
      authority.bind_dom_path(BrowserDomManifestEntry::new(path, selector, property));
    }
    for grant in &self.browser.grants {
      let resource = grant.resource.to_browser_resource()?;
      let allow = grant
        .allow
        .iter()
        .map(|operation| {
          BrowserOperation::parse(operation).ok_or_else(|| invalid_error(
            "browser.grants.allow",
            format!("unknown browser operation `{operation}`"),
          ))
        })
        .collect::<MResult<Vec<_>>>()?;
      if allow.is_empty() {
        return invalid("browser.grants.allow", "must contain at least one operation");
      }
      authority.grant(BrowserCapabilityGrant::new(resource, allow));
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
        log_level: log_level_as_str(&config.diagnostics.log_level).to_string(),
      },
    }
  }
}

impl From<&BrowserResource> for BrowserHostResourceConfig {
  fn from(resource: &BrowserResource) -> Self {
    match resource {
      BrowserResource::Dom(scope) => Self::Dom { selector: scope.selector.clone() },
      BrowserResource::Clipboard => Self::Clipboard,
      BrowserResource::Network(scope) => Self::Network {
        origin: scope.origin.clone(),
        methods: scope.methods.as_ref().map(|methods| methods.iter().cloned().collect()),
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
      Self::Dom { selector } => BrowserDomScope::new(selector)
        .map(BrowserResource::Dom)
        .map_err(|error| invalid_error("browser.grants.resource.selector", format!("invalid DOM selector `{selector}`: {error}"))),
      Self::Clipboard => Ok(BrowserResource::Clipboard),
      Self::Network { origin, methods } => BrowserNetworkScope::new(origin, methods.clone())
        .map(BrowserResource::Network)
        .map_err(|error| invalid_error("browser.grants.resource.origin", format!("invalid network origin `{origin}`: {error}"))),
      Self::Storage { backend, scope, recursive } => {
        let backend = BrowserStorageBackend::parse(backend).ok_or_else(|| invalid_error(
          "browser.grants.resource.backend",
          format!("unknown storage backend `{backend}`"),
        ))?;
        BrowserStorageScope::new(backend, scope)
          .map(|scope| BrowserResource::Storage(scope.with_recursive(*recursive)))
          .map_err(|error| invalid_error("browser.grants.resource.scope", format!("invalid storage scope `{scope}`: {error}")))
      }
    }
  }
}

fn log_level_as_str(log_level: &LogLevel) -> &'static str {
  match log_level {
    LogLevel::Error => "error",
    LogLevel::Warn => "warn",
    LogLevel::Info => "info",
    LogLevel::Debug => "debug",
    LogLevel::Trace => "trace",
  }
}

fn invalid_error(field: &'static str, reason: impl Into<String>) -> MechError {
  MechError::new(InvalidBrowserHostConfigError { field, reason: reason.into() }, None)
}

fn invalid<T>(field: &'static str, reason: impl Into<String>) -> MResult<T> {
  Err(invalid_error(field, reason))
}

fn browser_config_from_hosts(document: &MechConfigDocument) -> MResult<BrowserHostBrowserConfig> {
  let Some(host) = document.hosts.iter().find(|host| host.provider == "browser" || host.name == "browser") else {
    return Ok(BrowserHostBrowserConfig { grants: Vec::new(), dom_manifest: Vec::new() });
  };
  browser_config_from_settings(&host.settings)
}

pub fn browser_config_from_settings(settings: &ConfigValue) -> MResult<BrowserHostBrowserConfig> {
  let map = match settings {
    ConfigValue::Map(map) => map,
    _ => return invalid("browser.settings", "must be a map"),
  };
  let dom = match map.get("dom") {
    Some(ConfigValue::List(items)) => items,
    None => return Ok(BrowserHostBrowserConfig { grants: Vec::new(), dom_manifest: Vec::new() }),
    Some(_) => return invalid("browser.settings.dom", "must be a list"),
  };
  let mut grants = Vec::new();
  let mut dom_manifest = Vec::new();
  for (idx, item) in dom.iter().enumerate() {
    let where_ = format!("browser.settings.dom[{idx}]");
    let ConfigValue::Map(item) = item else { return invalid("browser.settings.dom", "items must be maps"); };
    let path = config_string(item.get("path"), "path")?;
    let selector = config_string(item.get("selector"), "selector")?;
    let property = config_string(item.get("property"), "property")?;
    let attribute = match item.get("attribute") {
      Some(ConfigValue::String(value)) => Some(value.clone()),
      Some(_) => return invalid("browser.settings.dom.attribute", "must be a string"),
      None => None,
    };
    let operations = config_string_list(item.get("operations"), "operations")?;
    if operations.is_empty() { return invalid("browser.settings.dom.operations", "must not be empty"); }
    BrowserDomPath::new(&path).map_err(|error| invalid_error("browser.settings.dom.path", format!("invalid DOM path `{path}`: {error}")))?;
    BrowserDomScope::new(&selector).map_err(|error| invalid_error("browser.settings.dom.selector", format!("invalid DOM selector `{selector}`: {error}")))?;
    BrowserDomProperty::parse_config_name(&property, attribute.as_deref()).map_err(|error| invalid_error("browser.settings.dom.property", format!("invalid DOM property `{property}`: {error}")))?;
    for operation in &operations {
      BrowserOperation::parse(operation).ok_or_else(|| invalid_error("browser.settings.dom.operations", format!("unknown browser operation `{operation}`")))?;
    }
    dom_manifest.push(BrowserHostDomManifestEntry { path, selector: selector.clone(), property, attribute });
    grants.push(BrowserHostBrowserGrant { resource: BrowserHostResourceConfig::Dom { selector }, allow: operations });
    let _ = where_;
  }
  Ok(BrowserHostBrowserConfig { grants, dom_manifest })
}

fn config_string(value: Option<&ConfigValue>, field: &'static str) -> MResult<String> {
  match value {
    Some(ConfigValue::String(value)) => Ok(value.clone()),
    Some(_) => invalid(field, "must be a string"),
    None => invalid(field, "is required"),
  }
}

fn config_string_list(value: Option<&ConfigValue>, field: &'static str) -> MResult<Vec<String>> {
  match value {
    Some(ConfigValue::List(items)) => items.iter().map(|item| match item {
      ConfigValue::String(value) => Ok(value.clone()),
      _ => invalid(field, "must contain only strings"),
    }).collect(),
    Some(_) => invalid(field, "must be a list"),
    None => invalid(field, "is required"),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use mech_runtime::{parse_config_document, ConfigProfileOptions};
  use mech_core::BrowserResourceKind;

  fn config_document() -> MechConfigDocument {
    parse_config_document(
      "test.mcfg",
      r##"
config := {
  runtime: {
    name: "demo"
    diagnostics: {
      log-level: "debug"
      trace-enabled: true
    }
  }
  browser: {
    dom: [
      {
        path: "counter/_text"
        selector: "#counter"
        property: "text"
        allow: ["read", "write"]
      }
      {
        path: "name/_value"
        selector: "#name"
        property: "value"
        allow: ["read"]
      }
      {
        path: "button/_aria-label"
        selector: "#button"
        property: "attribute"
        attribute: "aria-label"
        allow: ["read"]
      }
    ]
    clipboard: [
      { allow: ["read"] }
    ]
    network: [
      {
        origin: "https://example.com"
        methods: ["get"]
        allow: ["read"]
      }
    ]
    storage: [
      {
        backend: "local-storage"
        scope: "/demo"
        recursive: true
        allow: ["read", "list"]
      }
    ]
  }
}
"##,
      ConfigProfileOptions::default(),
    ).unwrap()
  }

  #[test]
  fn enum_string_mappings_live_on_domain_types() {
    assert_eq!(BrowserOperation::Write.as_str(), "write");
    assert_eq!(BrowserOperation::parse("list"), Some(BrowserOperation::List));
    assert_eq!(BrowserResourceKind::Dom.as_str(), "dom");
    assert_eq!(BrowserResourceKind::parse("storage"), Some(BrowserResourceKind::Storage));
    assert_eq!(BrowserDomProperty::Attribute("href".to_string()).config_name(), "attribute");
    assert_eq!(BrowserDomProperty::Attribute("href".to_string()).config_attribute(), Some("href"));
    assert_eq!(BrowserDomProperty::parse_config_name("inner-html", None).unwrap(), BrowserDomProperty::InnerHtml);
  }

  #[test]
  fn browser_host_config_projects_runtime_browser_grants_and_dom_manifest() {
    let document = config_document();
    let runtime_config = RuntimeConfig::default().apply_patch(&document.runtime).unwrap();
    let host_config = BrowserHostConfig::from_document_and_runtime(&document, &runtime_config);
    assert_eq!(host_config.runtime.name, "demo");
    assert_eq!(host_config.runtime.diagnostics.log_level, "debug");
    assert_eq!(host_config.browser.grants.len(), 6);
    assert!(host_config.browser.grants.iter().any(|grant| matches!(grant.resource, BrowserHostResourceConfig::Dom { ref selector } if selector == "#counter") && grant.allow == vec!["read", "write"]));
    assert!(host_config.browser.dom_manifest.iter().any(|entry| entry.path == "button/_aria-label" && entry.property == "attribute" && entry.attribute.as_deref() == Some("aria-label")));
  }

  #[test]
  fn browser_host_config_converts_back_to_runtime_and_authority() {
    let document = config_document();
    let runtime_config = RuntimeConfig::default().apply_patch(&document.runtime).unwrap();
    let host_config = BrowserHostConfig::from_document_and_runtime(&document, &runtime_config);
    let (runtime, authority) = host_config.into_runtime_and_browser_authority().unwrap();
    assert_eq!(runtime.name, "demo");
    assert_eq!(runtime.diagnostics.log_level, LogLevel::Debug);
    assert!(authority.allows_dom("#counter", BrowserOperation::Write).is_ok());
    assert!(authority.allows_clipboard(BrowserOperation::Read).is_ok());
    assert!(authority.allows_network("https://example.com", Some("GET"), BrowserOperation::Read).is_ok());
    assert!(authority.allows_storage(BrowserStorageBackend::LocalStorage, "/demo/file", BrowserOperation::List).is_ok());
    assert_eq!(authority.dom_manifest().len(), 3);
  }

  #[cfg(feature = "serde")]
  #[test]
  fn browser_host_config_serializes_expected_shape() {
    let document = config_document();
    let runtime_config = RuntimeConfig::default().apply_patch(&document.runtime).unwrap();
    let host_config = BrowserHostConfig::from_document_and_runtime(&document, &runtime_config);
    let json = serde_json::to_value(&host_config).unwrap();
    assert_eq!(json["runtime"]["diagnostics"]["logLevel"], "debug");
    assert_eq!(json["browser"]["grants"][0]["resource"]["kind"], "dom");
    assert!(json["browser"].get("dom").is_none());
    assert!(json["browser"]["domManifest"].as_array().unwrap().iter().any(|entry| {
      entry["property"] == "attribute" && entry["attribute"] == "aria-label"
    }));

    let round_tripped: BrowserHostConfig = serde_json::from_value(json).unwrap();
    assert_eq!(round_tripped.browser.dom_manifest, host_config.browser.dom_manifest);
  }

  #[test]
  fn browser_host_config_rejects_invalid_values() {
    let mut config = BrowserHostConfig {
      runtime: BrowserHostRuntimeConfig::from(&RuntimeConfig::default()),
      browser: BrowserHostBrowserConfig { grants: Vec::new(), dom_manifest: Vec::new() },
    };
    config.runtime.diagnostics.log_level = "verbose".to_string();
    assert!(config.into_runtime_config().is_err());

    let config = BrowserHostConfig {
      runtime: BrowserHostRuntimeConfig::from(&RuntimeConfig::default()),
      browser: BrowserHostBrowserConfig {
        grants: vec![BrowserHostBrowserGrant {
          resource: BrowserHostResourceConfig::Network { origin: "https://example.com".to_string(), methods: None },
          allow: vec!["write".to_string()],
        }],
        dom_manifest: vec![BrowserHostDomManifestEntry {
          path: "bad path".to_string(),
          selector: "#ok".to_string(),
          property: "text".to_string(),
          attribute: None,
        }],
      },
    };
    assert!(config.into_browser_authority().is_err());
  }
}
