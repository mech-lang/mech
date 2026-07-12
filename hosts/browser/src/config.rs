#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use std::collections::BTreeMap;

use mech_core::{
  BrowserAuthority, BrowserCapabilityGrant, BrowserDomManifestEntry, BrowserDomPath,
  BrowserDomProperty, BrowserDomScope, BrowserNetworkScope, BrowserOperation, BrowserResource,
  BrowserStorageBackend, BrowserStorageScope, MResult, MechError,
  MechErrorKind,
};

use mech_runtime::{
  parse_host_context_target, ConfigValue, DiagnosticsConfig, HostInstanceConfig, LogLevel,
  MechConfigDocument, RunResourceGrantConfig, RuntimeConfig, RuntimeLimits,
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
#[derive(Clone, Debug, PartialEq)]
pub struct BrowserRuntimeInjectionConfig {
  pub runtime: BrowserHostRuntimeConfig,
  pub hosts: Vec<HostInstanceConfig>,
  pub run_grants: Vec<RunResourceGrantConfig>,
}

impl BrowserRuntimeInjectionConfig {
  pub fn from_document_and_runtime(
    document: &MechConfigDocument,
    runtime_config: &RuntimeConfig,
  ) -> MResult<Self> {
    for host in &document.hosts {
      if host.name == "browser" && host.provider != "browser" {
        return Err(invalid_error(
          "hosts",
          format!(
            "host instance `browser` is reserved for provider `browser` in browser runtime injection and cannot be configured as provider `{}`",
            host.provider,
          ),
        ));
      }
    }

    let mut hosts: Vec<HostInstanceConfig> = document
      .hosts
      .iter()
      .filter(|host| host.provider == "browser")
      .cloned()
      .collect();
    if !hosts.iter().any(|host| host.name == "browser") {
      hosts.push(HostInstanceConfig {
        name: "browser".to_string(),
        provider: "browser".to_string(),
        settings: ConfigValue::Map(Default::default()),
      });
    }
    for host in &hosts {
      validate_injected_host_settings(host)?;
    }
    let injected_hosts_by_name: BTreeMap<String, HostInstanceConfig> =
      hosts.iter().map(|host| (host.name.clone(), host.clone())).collect();
    let mut run_grants = Vec::new();
    if let Some(run) = &document.run {
      for grant in &run.grants {
        let (instance, _) = parse_host_context_target(&grant.target)?;
        let Some(host) = injected_hosts_by_name.get(instance) else {
          continue;
        };
        validate_injected_run_grant(host, grant)?;
        run_grants.push(grant.clone());
      }
    }
    Ok(Self {
      runtime: BrowserHostRuntimeConfig::from(runtime_config),
      hosts,
      run_grants,
    })
  }

  pub fn into_runtime_config(&self) -> MResult<RuntimeConfig> {
    BrowserHostConfig {
      runtime: self.runtime.clone(),
      browser: BrowserHostBrowserConfig {
        grants: Vec::new(),
        dom_manifest: Vec::new(),
      },
    }
    .into_runtime_config()
  }

  pub fn browser_config(&self) -> MResult<BrowserHostBrowserConfig> {
    browser_config_from_browser_hosts(self.hosts.iter().filter(|host| host.provider == "browser"))
  }

  pub fn browser_host_config(&self) -> MResult<BrowserHostConfig> {
    Ok(BrowserHostConfig {
      runtime: self.runtime.clone(),
      browser: self.browser_config()?,
    })
  }
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
  pub operations: Vec<String>,
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
  ) -> MResult<Self> {
    let browser = browser_config_from_hosts(document)?;
    Ok(Self {
      runtime: BrowserHostRuntimeConfig::from(runtime_config),
      browser,
    })
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
      let operations = entry
        .operations
        .iter()
        .map(|operation| {
          BrowserOperation::parse(operation).ok_or_else(|| invalid_error(
            "browser.domManifest.operations",
            format!("unknown browser operation `{operation}`"),
          ))
        })
        .collect::<MResult<Vec<_>>>()?;
      if operations.is_empty() {
        return invalid("browser.domManifest.operations", "must contain at least one operation");
      }
      authority.bind_dom_path(BrowserDomManifestEntry::new(path, selector, property, operations));
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
  browser_config_from_browser_hosts(document.hosts.iter().filter(|host| host.provider == "browser"))
}

fn browser_config_from_browser_hosts<'a>(
  hosts: impl IntoIterator<Item = &'a HostInstanceConfig>,
) -> MResult<BrowserHostBrowserConfig> {
  let mut grants = Vec::new();
  let mut dom_manifest = Vec::new();

  for host in hosts {
    let config = browser_config_from_settings(&host.settings)?;
    grants.extend(config.grants);
    dom_manifest.extend(config.dom_manifest);
  }

  Ok(BrowserHostBrowserConfig { grants, dom_manifest })
}

fn validate_injected_run_grant(
  host: &HostInstanceConfig,
  grant: &RunResourceGrantConfig,
) -> MResult<()> {
  let (instance, context_name) = parse_host_context_target(&grant.target)?;

  if instance != host.name {
    return Err(invalid_error(
      "run.grants.target",
      format!(
        "run grant target `{}` does not match injected host instance `{}`",
        grant.target,
        host.name,
      ),
    ));
  }

  let operations = injected_host_context_operations(host, context_name)?;

  for operation in &grant.operations {
    if !operations.iter().any(|allowed| allowed == operation) {
      return Err(invalid_error(
        "run.grants.operations",
        format!(
          "host context `{}` does not expose operation `{}`",
          grant.target,
          operation,
        ),
      ));
    }
  }

  Ok(())
}

fn injected_host_context_operations(
  host: &HostInstanceConfig,
  context_name: &str,
) -> MResult<Vec<String>> {
  match host.provider.as_str() {
    "browser" => {
      if context_name != "dom" {
        return Err(invalid_error(
          "run.grants.target",
          format!(
            "browser host instance `{}` does not expose context `{}`; supported browser contexts: dom",
            host.name,
            context_name,
          ),
        ));
      }
      Ok(vec!["read".to_string(), "write".to_string()])
    }

    other => Err(invalid_error(
      "hosts.provider",
      format!(
        "cannot validate injected host provider `{other}` because no injection validator is registered"
      ),
    )),
  }
}

fn validate_injected_host_settings(host: &HostInstanceConfig) -> MResult<()> {
  match host.provider.as_str() {
    "browser" => browser_config_from_settings(&host.settings).map(|_| ()),
    _ => Ok(()),
  }
}

pub fn browser_config_from_settings(settings: &ConfigValue) -> MResult<BrowserHostBrowserConfig> {
  let map = match settings {
    ConfigValue::Map(map) => map,
    _ => return invalid("browser.settings", "must be a map"),
  };

  for key in map.keys() {
    match key.as_str() {
      "dom" | "clipboard" | "network" | "storage" => {}
      other => return invalid("browser.settings", format!("unknown browser settings key `{other}`")),
    }
  }

  let mut grants = Vec::new();
  let mut dom_manifest = Vec::new();

  if let Some(value) = map.get("dom") {
    let dom = match value {
      ConfigValue::List(items) => items,
      _ => return invalid("browser.settings.dom", "must be a list"),
    };
    for item in dom {
      let ConfigValue::Map(item) = item else { return invalid("browser.settings.dom", "items must be maps"); };
      reject_unknown_fields(item, &["path", "selector", "property", "attribute", "operations"], "browser.settings.dom")?;
      let path = config_string(item.get("path"), "browser.settings.dom.path")?;
      let selector = config_string(item.get("selector"), "browser.settings.dom.selector")?;
      let property = config_string(item.get("property"), "browser.settings.dom.property")?;
      let attribute = match item.get("attribute") {
        Some(ConfigValue::String(value)) => Some(value.clone()),
        Some(_) => return invalid("browser.settings.dom.attribute", "must be a string"),
        None => None,
      };
      let operations = config_operations(
        item.get("operations"),
        "browser.settings.dom.operations",
        &[BrowserOperation::Read, BrowserOperation::Write],
      )?;
      BrowserDomPath::new(&path).map_err(|error| invalid_error("browser.settings.dom.path", format!("invalid DOM path `{path}`: {error}")))?;
      BrowserDomScope::new(&selector).map_err(|error| invalid_error("browser.settings.dom.selector", format!("invalid DOM selector `{selector}`: {error}")))?;
      BrowserDomProperty::parse_config_name(&property, attribute.as_deref()).map_err(|error| invalid_error("browser.settings.dom.property", format!("invalid DOM property `{property}`: {error}")))?;
      dom_manifest.push(BrowserHostDomManifestEntry { path, selector: selector.clone(), property, attribute, operations: operations.clone() });
      grants.push(BrowserHostBrowserGrant { resource: BrowserHostResourceConfig::Dom { selector }, allow: operations });
    }
  }

  if let Some(value) = map.get("clipboard") {
    let clipboard = match value {
      ConfigValue::List(items) => items,
      _ => return invalid("browser.settings.clipboard", "must be a list"),
    };
    for item in clipboard {
      let ConfigValue::Map(item) = item else { return invalid("browser.settings.clipboard", "items must be maps"); };
      reject_unknown_fields(item, &["operations"], "browser.settings.clipboard")?;
      let operations = config_operations(
        item.get("operations"),
        "browser.settings.clipboard.operations",
        &[BrowserOperation::Read, BrowserOperation::Write],
      )?;
      grants.push(BrowserHostBrowserGrant { resource: BrowserHostResourceConfig::Clipboard, allow: operations });
    }
  }

  if let Some(value) = map.get("network") {
    let network = match value {
      ConfigValue::List(items) => items,
      _ => return invalid("browser.settings.network", "must be a list"),
    };
    for item in network {
      let ConfigValue::Map(item) = item else { return invalid("browser.settings.network", "items must be maps"); };
      reject_unknown_fields(item, &["origin", "operations", "methods"], "browser.settings.network")?;
      let origin = config_string(item.get("origin"), "browser.settings.network.origin")?;
      let operations = config_operations(
        item.get("operations"),
        "browser.settings.network.operations",
        &[BrowserOperation::Read],
      )?;
      let methods = optional_config_string_list(item.get("methods"), "browser.settings.network.methods")?;
      let scope = BrowserNetworkScope::new(&origin, methods.clone()).map_err(|error| invalid_error(
        "browser.settings.network.origin",
        format!("invalid network origin `{origin}`: {error}"),
      ))?;
      grants.push(BrowserHostBrowserGrant {
        resource: BrowserHostResourceConfig::Network {
          origin: scope.origin,
          methods: scope.methods.map(|methods| methods.into_iter().collect()),
        },
        allow: operations,
      });
    }
  }

  if let Some(value) = map.get("storage") {
    let storage = match value {
      ConfigValue::List(items) => items,
      _ => return invalid("browser.settings.storage", "must be a list"),
    };
    for item in storage {
      let ConfigValue::Map(item) = item else { return invalid("browser.settings.storage", "items must be maps"); };
      reject_unknown_fields(item, &["backend", "scope", "recursive", "operations"], "browser.settings.storage")?;
      let backend_name = config_string(item.get("backend"), "browser.settings.storage.backend")?;
      let scope = config_string(item.get("scope"), "browser.settings.storage.scope")?;
      let recursive = config_bool(item.get("recursive"), "browser.settings.storage.recursive", false)?;
      let operations = config_operations(
        item.get("operations"),
        "browser.settings.storage.operations",
        &[BrowserOperation::Read, BrowserOperation::Write, BrowserOperation::List],
      )?;
      let backend = BrowserStorageBackend::parse(&backend_name).ok_or_else(|| invalid_error(
        "browser.settings.storage.backend",
        format!("unknown storage backend `{backend_name}`"),
      ))?;
      BrowserStorageScope::new(backend, &scope).map_err(|error| invalid_error(
        "browser.settings.storage.scope",
        format!("invalid storage scope `{scope}`: {error}"),
      ))?;
      grants.push(BrowserHostBrowserGrant {
        resource: BrowserHostResourceConfig::Storage { backend: backend.to_string(), scope, recursive },
        allow: operations,
      });
    }
  }

  Ok(BrowserHostBrowserConfig { grants, dom_manifest })
}

fn reject_unknown_fields(
  map: &std::collections::BTreeMap<String, ConfigValue>,
  allowed: &[&str],
  field: &'static str,
) -> MResult<()> {
  for key in map.keys() {
    if !allowed.iter().any(|allowed| *allowed == key) {
      return invalid(field, format!("unknown field `{key}`"));
    }
  }
  Ok(())
}

fn config_operations(
  value: Option<&ConfigValue>,
  field: &'static str,
  allowed: &[BrowserOperation],
) -> MResult<Vec<String>> {
  let operations = config_string_list(value, field)?;
  if operations.is_empty() { return invalid(field, "must not be empty"); }
  for operation in &operations {
    let parsed = BrowserOperation::parse(operation)
      .ok_or_else(|| invalid_error(field, format!("unknown browser operation `{operation}`")))?;
    if !allowed.contains(&parsed) {
      return Err(invalid_error(
        field,
        format!("browser operation `{operation}` is not supported for this resource"),
      ));
    }
  }
  Ok(operations)
}

fn config_bool(value: Option<&ConfigValue>, field: &'static str, default: bool) -> MResult<bool> {
  match value {
    Some(ConfigValue::Bool(value)) => Ok(*value),
    Some(_) => invalid(field, "must be a bool"),
    None => Ok(default),
  }
}

fn optional_config_string_list(value: Option<&ConfigValue>, field: &'static str) -> MResult<Option<Vec<String>>> {
  match value {
    Some(value) => config_string_list(Some(value), field).map(Some),
    None => Ok(None),
  }
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
  use mech_core::{BrowserResourceKind, Value};
  use mech_runtime::{
    materialize_host_manifest, parse_config_document, ConfigProfileOptions, RuntimeHostFactory,
    RuntimeHostInstallation, RuntimeResourceProvider, RuntimeResourceReadRequest,
    RuntimeResourceWriteIntent, RuntimeResourceWritePreflightRequest, RuntimeResourceWriteRequest,
  };

  fn cfg_string(value: &str) -> ConfigValue {
    ConfigValue::String(value.to_string())
  }

  fn cfg_bool(value: bool) -> ConfigValue {
    ConfigValue::Bool(value)
  }

  fn cfg_list(items: Vec<ConfigValue>) -> ConfigValue {
    ConfigValue::List(items)
  }

  fn cfg_map(items: Vec<(&str, ConfigValue)>) -> ConfigValue {
    ConfigValue::Map(items.into_iter().map(|(key, value)| (key.to_string(), value)).collect())
  }

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
  hosts: [
    {
      name: "browser"
      provider: "browser"
      settings: {
        dom: [
          {
            path: "counter/_text"
            selector: "#counter"
            property: "text"
            operations: ["read", "write"]
          }
          {
            path: "name/_value"
            selector: "#name"
            property: "value"
            operations: ["read"]
          }
          {
            path: "button/_aria-label"
            selector: "#button"
            property: "attribute"
            attribute: "aria-label"
            operations: ["read"]
          }
        ]
      }
    }
  ]
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
    let host_config = BrowserHostConfig::from_document_and_runtime(&document, &runtime_config).unwrap();
    assert_eq!(host_config.runtime.name, "demo");
    assert_eq!(host_config.runtime.diagnostics.log_level, "debug");
    assert_eq!(host_config.browser.grants.len(), 3);
    assert!(host_config.browser.grants.iter().any(|grant| matches!(grant.resource, BrowserHostResourceConfig::Dom { ref selector } if selector == "#counter") && grant.allow == vec!["read", "write"]));
    assert!(host_config.browser.dom_manifest.iter().any(|entry| entry.path == "button/_aria-label" && entry.property == "attribute" && entry.attribute.as_deref() == Some("aria-label")));
  }

  #[test]
  fn browser_config_from_hosts_merges_multiple_browser_instances() {
    let document = parse_config_document(
      "test.mcfg",
      r##"
config := {
  hosts: [
    {
      name: "browser"
      provider: "browser"
      settings: {
        dom: [
          {
            path: "body/content/default/_value"
            selector: "#default"
            property: "value"
            operations: ["write"]
          }
        ]
      }
    }
    {
      name: "ui"
      provider: "browser"
      settings: {
        dom: [
          {
            path: "body/content/ui/_value"
            selector: "#ui"
            property: "value"
            operations: ["read", "write"]
          }
        ]
      }
    }
  ]
}
"##,
      ConfigProfileOptions::default(),
    ).unwrap();

    let config = browser_config_from_hosts(&document).unwrap();
    assert_eq!(config.dom_manifest.len(), 2);
    assert_eq!(config.grants.len(), 2);
    assert!(config.dom_manifest.iter().any(|entry| entry.selector == "#default"));
    assert!(config.dom_manifest.iter().any(|entry| entry.selector == "#ui"));
  }

  #[test]
  fn browser_host_from_config_allows_later_browser_instance_selector() {
    let document = parse_config_document(
      "test.mcfg",
      r##"
config := {
  hosts: [
    {
      name: "browser"
      provider: "browser"
      settings: {
        dom: [
          {
            path: "body/content/default/_value"
            selector: "#default"
            property: "value"
            operations: ["write"]
          }
        ]
      }
    }
    {
      name: "ui"
      provider: "browser"
      settings: {
        dom: [
          {
            path: "body/content/ui/_value"
            selector: "#ui"
            property: "value"
            operations: ["read", "write"]
          }
        ]
      }
    }
  ]
}
"##,
      ConfigProfileOptions::default(),
    ).unwrap();
    let runtime_config = RuntimeConfig::default().apply_patch(&document.runtime).unwrap();
    let authority = BrowserHostConfig::from_document_and_runtime(&document, &runtime_config)
      .unwrap()
      .into_browser_authority()
      .unwrap();

    assert!(authority.allows_dom("#ui", BrowserOperation::Write).is_ok());
  }

  #[test]
  fn browser_runtime_injection_config_merges_same_browser_settings() {
    let document = parse_config_document(
      "test.mcfg",
      r##"
config := {
  hosts: [
    {
      name: "browser"
      provider: "browser"
      settings: {
        dom: [
          {
            path: "body/content/default/_value"
            selector: "#default"
            property: "value"
            operations: ["read"]
          }
        ]
      }
    }
    {
      name: "ui"
      provider: "browser"
      settings: {
        dom: [
          {
            path: "body/content/ui/_value"
            selector: "#ui"
            property: "value"
            operations: ["write"]
          }
        ]
      }
    }
  ]
}
"##,
      ConfigProfileOptions::default(),
    ).unwrap();
    let injected =
      BrowserRuntimeInjectionConfig::from_document_and_runtime(&document, &RuntimeConfig::default()).unwrap();
    let host_config = injected.browser_host_config().unwrap();

    assert_eq!(host_config.browser.dom_manifest.len(), 2);
    assert_eq!(host_config.browser.grants.len(), 2);
    assert!(host_config.browser.dom_manifest.iter().any(|entry| entry.selector == "#default"));
    assert!(host_config.browser.dom_manifest.iter().any(|entry| entry.selector == "#ui"));
  }

  #[test]
  fn browser_host_config_converts_back_to_runtime_and_authority() {
    let document = config_document();
    let runtime_config = RuntimeConfig::default().apply_patch(&document.runtime).unwrap();
    let host_config = BrowserHostConfig::from_document_and_runtime(&document, &runtime_config).unwrap();
    let (runtime, authority) = host_config.into_runtime_and_browser_authority().unwrap();
    assert_eq!(runtime.name, "demo");
    assert_eq!(runtime.diagnostics.log_level, LogLevel::Debug);
    assert!(authority.allows_dom("#counter", BrowserOperation::Write).is_ok());
    assert_eq!(authority.dom_manifest().len(), 3);
  }

  #[test]
  fn browser_settings_dom_only_parses() {
    let config = browser_config_from_settings(&cfg_map(vec![
      ("dom", cfg_list(vec![cfg_map(vec![
        ("path", cfg_string("body/content/input/_value")),
        ("selector", cfg_string("#input")),
        ("property", cfg_string("value")),
        ("operations", cfg_list(vec![cfg_string("read"), cfg_string("write")])),
      ])])),
    ])).unwrap();

    assert_eq!(config.dom_manifest.len(), 1);
    assert_eq!(config.grants.len(), 1);
    assert_eq!(config.dom_manifest[0].path, "body/content/input/_value");
    assert!(matches!(config.grants[0].resource, BrowserHostResourceConfig::Dom { ref selector } if selector == "#input"));
    assert_eq!(config.grants[0].allow, vec!["read".to_string(), "write".to_string()]);
  }

  #[test]
  fn browser_settings_clipboard_only_parses() {
    let config = browser_config_from_settings(&cfg_map(vec![
      ("clipboard", cfg_list(vec![cfg_map(vec![
        ("operations", cfg_list(vec![cfg_string("read"), cfg_string("write")]))
      ])])),
    ])).unwrap();

    assert!(config.dom_manifest.is_empty());
    assert_eq!(config.grants.len(), 1);
    assert!(matches!(config.grants[0].resource, BrowserHostResourceConfig::Clipboard));
    assert_eq!(config.grants[0].allow, vec!["read".to_string(), "write".to_string()]);
  }

  #[test]
  fn browser_settings_network_only_parses() {
    let config = browser_config_from_settings(&cfg_map(vec![
      ("network", cfg_list(vec![cfg_map(vec![
        ("origin", cfg_string("https://docs.mech-lang.org")),
        ("operations", cfg_list(vec![cfg_string("read")])),
        ("methods", cfg_list(vec![cfg_string("GET")])),
      ])])),
    ])).unwrap();

    assert!(config.dom_manifest.is_empty());
    assert_eq!(config.grants.len(), 1);
    assert!(matches!(
      &config.grants[0].resource,
      BrowserHostResourceConfig::Network { origin, methods }
        if origin == "https://docs.mech-lang.org" && methods.as_ref().unwrap() == &vec!["GET".to_string()]
    ));
    assert_eq!(config.grants[0].allow, vec!["read".to_string()]);
  }

  #[test]
  fn browser_settings_storage_only_parses() {
    let config = browser_config_from_settings(&cfg_map(vec![
      ("storage", cfg_list(vec![cfg_map(vec![
        ("backend", cfg_string("opfs")),
        ("scope", cfg_string("/workspace")),
        ("recursive", cfg_bool(true)),
        ("operations", cfg_list(vec![cfg_string("read"), cfg_string("write"), cfg_string("list")])),
      ])])),
    ])).unwrap();

    assert!(config.dom_manifest.is_empty());
    assert_eq!(config.grants.len(), 1);
    assert!(matches!(
      &config.grants[0].resource,
      BrowserHostResourceConfig::Storage { backend, scope, recursive }
        if backend == "opfs" && scope == "/workspace" && *recursive
    ));
    assert_eq!(config.grants[0].allow, vec!["read".to_string(), "write".to_string(), "list".to_string()]);
  }

  #[test]
  fn browser_settings_dom_absent_does_not_drop_other_grants() {
    let config = browser_config_from_settings(&cfg_map(vec![
      ("clipboard", cfg_list(vec![cfg_map(vec![
        ("operations", cfg_list(vec![cfg_string("read")]))
      ])])),
      ("network", cfg_list(vec![cfg_map(vec![
        ("origin", cfg_string("https://docs.mech-lang.org")),
        ("operations", cfg_list(vec![cfg_string("read")])),
      ])])),
    ])).unwrap();

    assert!(config.dom_manifest.is_empty());
    assert_eq!(config.grants.len(), 2);
    assert!(config.grants.iter().any(|grant| matches!(grant.resource, BrowserHostResourceConfig::Clipboard)));
    assert!(config.grants.iter().any(|grant| matches!(grant.resource, BrowserHostResourceConfig::Network { .. })));
  }

  #[test]
  fn browser_settings_mixed_resources_parse() {
    let config = browser_config_from_settings(&cfg_map(vec![
      ("dom", cfg_list(vec![cfg_map(vec![
        ("path", cfg_string("body/content/output/_value")),
        ("selector", cfg_string("#output")),
        ("property", cfg_string("value")),
        ("operations", cfg_list(vec![cfg_string("write")])),
      ])])),
      ("clipboard", cfg_list(vec![cfg_map(vec![
        ("operations", cfg_list(vec![cfg_string("read")]))
      ])])),
    ])).unwrap();

    assert_eq!(config.dom_manifest.len(), 1);
    assert_eq!(config.grants.len(), 2);
    assert!(config.grants.iter().any(|grant| matches!(grant.resource, BrowserHostResourceConfig::Dom { .. })));
    assert!(config.grants.iter().any(|grant| matches!(grant.resource, BrowserHostResourceConfig::Clipboard)));
  }

  #[test]
  fn browser_settings_unknown_key_rejected() {
    let err = browser_config_from_settings(&cfg_map(vec![
      ("cookies", cfg_list(vec![])),
    ])).unwrap_err();
    let error = format!("{err:?}");
    assert!(error.contains("unknown browser settings key `cookies`"), "got {error}");
  }

  #[test]
  fn browser_settings_allow_field_rejected() {
    let err = browser_config_from_settings(&cfg_map(vec![
      ("clipboard", cfg_list(vec![cfg_map(vec![
        ("allow", cfg_list(vec![cfg_string("read")]))
      ])])),
    ])).unwrap_err();
    let error = format!("{err:?}");
    assert!(error.contains("allow"), "got {error}");
  }

  #[test]
  fn network_rejects_write_operation() {
    let err = browser_config_from_settings(&cfg_map(vec![
      ("network", cfg_list(vec![cfg_map(vec![
        ("origin", cfg_string("https://example.com")),
        ("operations", cfg_list(vec![cfg_string("write")])),
      ])])),
    ])).unwrap_err();
    let error = format!("{err:?}");
    assert!(error.contains("network"), "got {error}");
    assert!(error.contains("write"), "got {error}");
    assert!(error.contains("not supported"), "got {error}");
  }

  #[test]
  fn dom_rejects_list_operation() {
    let err = browser_config_from_settings(&cfg_map(vec![
      ("dom", cfg_list(vec![cfg_map(vec![
        ("path", cfg_string("counter/_text")),
        ("selector", cfg_string("#counter")),
        ("property", cfg_string("text")),
        ("operations", cfg_list(vec![cfg_string("list")])),
      ])])),
    ])).unwrap_err();
    let error = format!("{err:?}");
    assert!(error.contains("dom.operations"), "got {error}");
    assert!(error.contains("list"), "got {error}");
    assert!(error.contains("not supported"), "got {error}");
  }

  #[test]
  fn storage_allows_list_operation() {
    let config = browser_config_from_settings(&cfg_map(vec![
      ("storage", cfg_list(vec![cfg_map(vec![
        ("backend", cfg_string("local-storage")),
        ("scope", cfg_string("/app")),
        ("operations", cfg_list(vec![cfg_string("read"), cfg_string("write"), cfg_string("list")])),
      ])])),
    ])).unwrap();

    assert_eq!(config.grants.len(), 1);
    assert_eq!(config.grants[0].allow, vec!["read".to_string(), "write".to_string(), "list".to_string()]);
  }

  #[test]
  fn clipboard_allows_read_write() {
    let config = browser_config_from_settings(&cfg_map(vec![
      ("clipboard", cfg_list(vec![cfg_map(vec![
        ("operations", cfg_list(vec![cfg_string("read"), cfg_string("write")])),
      ])])),
    ])).unwrap();

    assert_eq!(config.grants.len(), 1);
    assert_eq!(config.grants[0].allow, vec!["read".to_string(), "write".to_string()]);
  }

  #[test]
  fn browser_settings_unknown_operation_rejected() {
    let err = browser_config_from_settings(&cfg_map(vec![
      ("clipboard", cfg_list(vec![cfg_map(vec![
        ("operations", cfg_list(vec![cfg_string("teleport")])),
      ])])),
    ])).unwrap_err();
    let error = format!("{err:?}");
    assert!(error.contains("unknown browser operation `teleport`"), "got {error}");
  }

  #[test]
  fn browser_from_document_and_runtime_rejects_invalid_host_settings() {
    let document = parse_config_document(
      "test.mcfg",
      r##"
config := {
  hosts: [
    {
      name: "browser"
      provider: "browser"
      settings: {
        dom: [
          {
            path: "bad path"
            selector: "#input"
            property: "value"
            operations: ["read"]
          }
        ]
      }
    }
  ]
}
"##,
      ConfigProfileOptions::default(),
    ).unwrap();
    let err = BrowserHostConfig::from_document_and_runtime(&document, &RuntimeConfig::default()).unwrap_err();
    let error = format!("{err:?}");
    assert!(error.contains("browser.settings.dom.path"), "got {error}");
  }

  #[test]
  fn browser_runtime_injection_preserves_alias_and_default_browser() {
    let document = parse_config_document(
      "test.mcfg",
      r##"
config := {
  hosts: [
    {
      name: "ui"
      provider: "browser"
      settings: {
        dom: [
          {
            path: "body/content/output/_value"
            selector: "#output"
            property: "value"
            operations: ["write"]
          }
        ]
      }
    }
  ]
  run: {
    grants: [
      {
        target: "ui/dom"
        operations: ["write"]
        paths: ["body/content/output/_value"]
      }
    ]
  }
}
"##,
      ConfigProfileOptions::default(),
    ).unwrap();
    let injected =
      BrowserRuntimeInjectionConfig::from_document_and_runtime(&document, &RuntimeConfig::default()).unwrap();

    assert!(injected.hosts.iter().any(|host| host.name == "ui" && host.provider == "browser"));
    assert!(injected.hosts.iter().any(|host| host.name == "browser" && host.provider == "browser"));
    assert_eq!(injected.run_grants.len(), 1);
    assert_eq!(injected.run_grants[0].target, "ui/dom");
  }

  #[test]
  fn browser_runtime_injection_filters_non_browser_run_grants() {
    let document = parse_config_document(
      "test.mcfg",
      r##"
config := {
  hosts: [
    {
      name: "browser"
      provider: "browser"
      settings: {}
    }
    {
      name: "arm"
      provider: "fake-robot"
      settings: {}
    }
  ]
  run: {
    grants: [
      {
        target: "browser/dom"
        operations: ["write"]
        paths: ["body/content/output/_value"]
      }
      {
        target: "arm/commands"
        operations: ["write"]
        paths: ["joints/shoulder/target"]
      }
    ]
  }
}
"##,
      ConfigProfileOptions::default(),
    ).unwrap();
    let injected =
      BrowserRuntimeInjectionConfig::from_document_and_runtime(&document, &RuntimeConfig::default()).unwrap();

    assert!(injected.hosts.iter().all(|host| host.provider == "browser"));
    assert_eq!(injected.run_grants.len(), 1);
    assert_eq!(injected.run_grants[0].target, "browser/dom");

    let mut builder = mech_runtime::RuntimeBuilder::new()
      .host_factory(Box::new(RecordingBrowserFactory {
        manifest: crate::browser_host_manifest().unwrap(),
        writes: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
      }))
      .unwrap();
    for host in injected.hosts {
      builder = builder.host_instance(host);
    }
    for grant in injected.run_grants {
      builder = builder.run_resource_grant(grant);
    }
    builder.build().unwrap();
  }

  #[test]
  fn browser_runtime_injection_rejects_unsupported_injected_context() {
    let document = parse_config_document(
      "test.mcfg",
      r##"
config := {
  hosts: [
    {
      name: "browser"
      provider: "browser"
      settings: {}
    }
  ]
  run: {
    grants: [
      {
        target: "browser/clipboard"
        operations: ["read"]
        paths: ["*"]
      }
    ]
  }
}
"##,
      ConfigProfileOptions::default(),
    ).unwrap();
    let err =
      BrowserRuntimeInjectionConfig::from_document_and_runtime(&document, &RuntimeConfig::default()).unwrap_err();
    let error = format!("{err:?}");
    assert!(error.contains("browser"), "got {error}");
    assert!(error.contains("clipboard"), "got {error}");
    assert!(error.contains("dom"), "got {error}");
  }

  #[test]
  fn browser_runtime_injection_rejects_unknown_browser_context_grant() {
    let document = parse_config_document(
      "test.mcfg",
      r##"
config := {
  hosts: [
    { name: "browser" provider: "browser" settings: {} }
  ]
  run: {
    grants: [
      { target: "browser/clipboard" operations: ["read"] paths: ["*"] }
    ]
  }
}
"##,
      ConfigProfileOptions::default(),
    ).unwrap();
    let err =
      BrowserRuntimeInjectionConfig::from_document_and_runtime(&document, &RuntimeConfig::default()).unwrap_err();
    let error = format!("{err:?}");
    assert!(error.contains("browser"), "got {error}");
    assert!(error.contains("clipboard"), "got {error}");
    assert!(error.contains("dom"), "got {error}");
  }

  #[test]
  fn browser_runtime_injection_rejects_unsupported_browser_grant_operation() {
    let document = parse_config_document(
      "test.mcfg",
      r##"
config := {
  hosts: [
    { name: "browser" provider: "browser" settings: {} }
  ]
  run: {
    grants: [
      { target: "browser/dom" operations: ["list"] paths: ["*"] }
    ]
  }
}
"##,
      ConfigProfileOptions::default(),
    ).unwrap();
    let err =
      BrowserRuntimeInjectionConfig::from_document_and_runtime(&document, &RuntimeConfig::default()).unwrap_err();
    let error = format!("{err:?}");
    assert!(error.contains("browser/dom"), "got {error}");
    assert!(error.contains("list"), "got {error}");
  }

  #[test]
  fn browser_runtime_injection_still_filters_non_injected_native_grant() {
    let document = parse_config_document(
      "test.mcfg",
      r##"
config := {
  hosts: [
    { name: "native" provider: "fake-robot" settings: {} }
  ]
  run: {
    grants: [
      { target: "native/commands" operations: ["move"] paths: ["move"] }
    ]
  }
}
"##,
      ConfigProfileOptions::default(),
    ).unwrap();
    let injected =
      BrowserRuntimeInjectionConfig::from_document_and_runtime(&document, &RuntimeConfig::default()).unwrap();

    assert!(!injected.hosts.iter().any(|host| host.name == "native"));
    assert!(injected.run_grants.is_empty());
  }

  #[test]
  fn browser_runtime_injection_rejects_non_browser_host_named_browser() {
    let document = parse_config_document(
      "test.mcfg",
      r##"
config := {
  hosts: [
    { name: "browser" provider: "fake-robot" settings: {} }
  ]
  run: {
    grants: [
      { target: "browser/dom" operations: ["read"] paths: ["counter/_text"] }
    ]
  }
}
"##,
      ConfigProfileOptions::default(),
    ).unwrap();
    let err =
      BrowserRuntimeInjectionConfig::from_document_and_runtime(&document, &RuntimeConfig::default()).unwrap_err();
    let error = format!("{err:?}");
    assert!(error.contains("browser"), "got {error}");
    assert!(error.contains("fake-robot"), "got {error}");
    assert!(error.contains("reserved") || error.contains("provider"), "got {error}");
  }

  #[test]
  fn browser_runtime_injection_filters_non_browser_only_run_grant() {
    let document = parse_config_document(
      "test.mcfg",
      r##"
config := {
  hosts: [
    {
      name: "browser"
      provider: "browser"
      settings: {}
    }
    {
      name: "arm"
      provider: "fake-robot"
      settings: {}
    }
  ]
  run: {
    grants: [
      {
        target: "arm/commands"
        operations: ["write"]
        paths: ["move"]
      }
    ]
  }
}
"##,
      ConfigProfileOptions::default(),
    ).unwrap();
    let injected =
      BrowserRuntimeInjectionConfig::from_document_and_runtime(&document, &RuntimeConfig::default()).unwrap();

    assert!(injected.hosts.iter().all(|host| host.provider == "browser"));
    assert!(injected.run_grants.is_empty());
  }

  #[test]
  fn browser_injection_filters_unavailable_provider() {
    let document = parse_config_document(
      "test.mcfg",
      r##"
config := {
  hosts: [
    {
      name: "ui"
      provider: "browser"
      settings: {}
    }
    {
      name: "native"
      provider: "some-native-host"
      settings: {}
    }
  ]
  run: {
    grants: [
      {
        target: "native/commands"
        operations: ["move"]
        paths: ["move"]
      }
    ]
  }
}
"##,
      ConfigProfileOptions::default(),
    ).unwrap();
    let injected =
      BrowserRuntimeInjectionConfig::from_document_and_runtime(&document, &RuntimeConfig::default()).unwrap();

    assert!(!injected.hosts.iter().any(|host| host.name == "native"));
    assert!(injected.run_grants.is_empty());
  }

  #[test]
  fn browser_runtime_injection_keeps_alias_run_grant() {
    let document = parse_config_document(
      "test.mcfg",
      r##"
config := {
  hosts: [
    {
      name: "ui"
      provider: "browser"
      settings: {}
    }
  ]
  run: {
    grants: [
      {
        target: "ui/dom"
        operations: ["write"]
        paths: ["body/content/output/_value"]
      }
    ]
  }
}
"##,
      ConfigProfileOptions::default(),
    ).unwrap();
    let injected =
      BrowserRuntimeInjectionConfig::from_document_and_runtime(&document, &RuntimeConfig::default()).unwrap();

    assert!(injected.hosts.iter().any(|host| host.name == "ui"));
    assert_eq!(injected.run_grants.len(), 1);
    assert_eq!(injected.run_grants[0].target, "ui/dom");
  }

  #[test]
  fn browser_fallback_still_added_when_no_browser_host_declared() {
    let document = parse_config_document(
      "test.mcfg",
      r##"
config := {
  hosts: [
    {
      name: "arm"
      provider: "fake-robot"
      settings: {}
    }
  ]
}
"##,
      ConfigProfileOptions::default(),
    ).unwrap();
    let injected =
      BrowserRuntimeInjectionConfig::from_document_and_runtime(&document, &RuntimeConfig::default()).unwrap();

    assert!(injected.hosts.iter().any(|host| host.name == "browser" && host.provider == "browser"));
  }

  #[test]
  fn explicit_browser_provider_still_accepts_browser_grant() {
    let document = parse_config_document(
      "test.mcfg",
      r##"
config := {
  hosts: [
    {
      name: "browser"
      provider: "browser"
      settings: {
        dom: [
          {
            path: "counter/_text"
            selector: "#counter"
            property: "text"
            operations: ["read"]
          }
        ]
      }
    }
  ]
  run: {
    grants: [
      {
        target: "browser/dom"
        operations: ["read"]
        paths: ["counter/_text"]
      }
    ]
  }
}
"##,
      ConfigProfileOptions::default(),
    ).unwrap();
    let injected =
      BrowserRuntimeInjectionConfig::from_document_and_runtime(&document, &RuntimeConfig::default()).unwrap();

    assert_eq!(injected.run_grants.len(), 1);
    assert_eq!(injected.run_grants[0].target, "browser/dom");
    assert_eq!(injected.hosts.iter().filter(|host| host.name == "browser").count(), 1);
  }


  #[derive(Clone, Debug, Default)]
  struct RecordingBrowserProvider {
    instance: String,
    writes: std::sync::Arc<std::sync::Mutex<Vec<(String, String)>>>,
  }

  impl RuntimeResourceProvider for RecordingBrowserProvider {
    fn scheme(&self) -> &str {
      "browser"
    }

    fn base_uris(&self) -> Vec<String> {
      vec![format!("browser://{}/dom", self.instance)]
    }

    fn read(&self, _request: RuntimeResourceReadRequest) -> MResult<Value> {
      panic!("configured browser instance test does not read")
    }

    fn preflight_write(&self, request: RuntimeResourceWritePreflightRequest) -> MResult<()> {
      assert_eq!(request.base_uri, format!("browser://{}/dom", self.instance));
      assert!(request.path.starts_with("body/content/"));
      assert_eq!(request.intent, RuntimeResourceWriteIntent::Assign);
      Ok(())
    }

    fn write(&mut self, request: RuntimeResourceWriteRequest) -> MResult<()> {
      self.preflight_write(RuntimeResourceWritePreflightRequest {
        base_uri: request.base_uri,
        path: request.path.clone(),
        context_name: request.context_name,
        operation: request.operation.clone(),
        intent: request.intent,
      })?;
      let Value::String(value) = request.value else {
        panic!("configured browser instance test writes a string")
      };
      self.writes.lock().unwrap().push((request.path, value.borrow().as_str().to_string()));
      Ok(())
    }
  }

  #[derive(Debug)]
  struct RecordingBrowserFactory {
    manifest: mech_runtime::HostManifestConfig,
    writes: std::sync::Arc<std::sync::Mutex<Vec<(String, String)>>>,
  }

  impl RuntimeHostFactory for RecordingBrowserFactory {
    fn provider_name(&self) -> &str {
      "browser"
    }

    fn manifest(&self) -> &mech_runtime::HostManifestConfig {
      &self.manifest
    }

    fn validate_settings(&self, _instance_name: &str, settings: &ConfigValue) -> MResult<()> {
      browser_config_from_settings(settings).map(|_| ())
    }

    fn instantiate(&self, instance_name: &str, settings: &ConfigValue) -> MResult<RuntimeHostInstallation> {
      self.validate_settings(instance_name, settings)?;
      Ok(RuntimeHostInstallation {
        interface: materialize_host_manifest(instance_name, &self.manifest)?,
        resource_providers: vec![Box::new(RecordingBrowserProvider {
          instance: instance_name.to_string(),
          writes: self.writes.clone(),
        })],
      })
    }
  }

  #[test]
  fn configured_browser_instance_name_resolves_dom_context() {
    let settings = cfg_map(vec![
      ("dom", cfg_list(vec![cfg_map(vec![
        ("path", cfg_string("body/content/output/_value")),
        ("selector", cfg_string("#output")),
        ("property", cfg_string("value")),
        ("operations", cfg_list(vec![cfg_string("write")])),
      ])])),
    ]);
    let writes = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let mut runtime = mech_runtime::RuntimeBuilder::new()
      .host_factory(Box::new(RecordingBrowserFactory {
        manifest: crate::browser_host_manifest().unwrap(),
        writes: writes.clone(),
      }))
      .unwrap()
      .host_instance(mech_runtime::HostInstanceConfig {
        name: "ui".to_string(),
        provider: "browser".to_string(),
        settings,
      })
      .run_resource_grant(mech_runtime::RunResourceGrantConfig {
        target: "ui/dom".to_string(),
        operations: vec!["write".to_string()],
        paths: vec!["body/content/output/_value".to_string()],
      })
      .build()
      .unwrap();

    runtime.run_string(r#"+> @ui := ui/dom
@ui/body/content/output/_value = "ok"
"#).unwrap();

    let writes = writes.lock().unwrap();
    assert_eq!(writes.len(), 1);
    assert_eq!(writes[0], ("body/content/output/_value".to_string(), "ok".to_string()));
  }

  #[test]
  fn browser_runtime_grants_do_not_broaden_from_settings() {
    let settings = cfg_map(vec![
      ("dom", cfg_list(vec![
        cfg_map(vec![
          ("path", cfg_string("body/content/allowed/_value")),
          ("selector", cfg_string("#allowed")),
          ("property", cfg_string("value")),
          ("operations", cfg_list(vec![cfg_string("write")])),
        ]),
        cfg_map(vec![
          ("path", cfg_string("body/content/denied/_value")),
          ("selector", cfg_string("#denied")),
          ("property", cfg_string("value")),
          ("operations", cfg_list(vec![cfg_string("write")])),
        ]),
      ])),
    ]);
    let writes = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let mut runtime = mech_runtime::RuntimeBuilder::new()
      .host_factory(Box::new(RecordingBrowserFactory {
        manifest: crate::browser_host_manifest().unwrap(),
        writes,
      }))
      .unwrap()
      .host_instance(mech_runtime::HostInstanceConfig {
        name: "ui".to_string(),
        provider: "browser".to_string(),
        settings,
      })
      .run_resource_grant(mech_runtime::RunResourceGrantConfig {
        target: "ui/dom".to_string(),
        operations: vec!["write".to_string()],
        paths: vec!["body/content/allowed/_value".to_string()],
      })
      .build()
      .unwrap();

    runtime.run_string(r#"+> @ui := ui/dom
@ui/body/content/allowed/_value = "ok"
"#).unwrap();

    let result = runtime.run_string(r#"+> @ui := ui/dom
@ui/body/content/denied/_value = "no"
"#);
    assert!(result.is_err());
    let error = format!("{:?}", result.err().unwrap());
    assert!(error.contains("RuntimeCapabilityGrantDenied"), "got {error}");
  }

  #[cfg(feature = "serde")]
  #[test]
  fn browser_host_config_serializes_expected_shape() {
    let document = config_document();
    let runtime_config = RuntimeConfig::default().apply_patch(&document.runtime).unwrap();
    let host_config = BrowserHostConfig::from_document_and_runtime(&document, &runtime_config).unwrap();
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
          operations: vec!["read".to_string()],
        }],
      },
    };
    assert!(config.into_browser_authority().is_err());
  }
}
