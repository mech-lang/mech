use mech_core::{BrowserAuthority, MResult};
use mech_runtime::{
  encode_bool, encode_option_string, encode_option_u64, encode_string, encode_u64, ConfigValue,
  HostDelegationEnvelope, HostDelegationPayload, VerifiedHostDelegation,
};
#[cfg(feature = "delegation_signing")]
use mech_runtime::{
  sign_host_delegation, verify_host_delegation, HostDelegationHeader, HostDelegationSigningKey,
  HostDelegationVerificationRequest,
};

use crate::{BrowserHostConfig, BrowserHostResourceConfig, BrowserRuntimeInjectionConfig};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Clone, Debug, PartialEq)]
pub struct BrowserHostDelegationPayload {
  #[cfg_attr(feature = "serde", serde(flatten))]
  pub runtime_injection: BrowserRuntimeInjectionConfig,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BrowserVerifiedAuthority {
  pub runtime_config: mech_runtime::RuntimeConfig,
  pub browser_authority: BrowserAuthority,
  pub runtime_injection: BrowserRuntimeInjectionConfig,
}

pub type BrowserHostDelegationEnvelope = HostDelegationEnvelope<BrowserHostDelegationPayload>;
pub type VerifiedBrowserHostDelegation = VerifiedHostDelegation<BrowserHostDelegationPayload, BrowserVerifiedAuthority>;

impl HostDelegationPayload for BrowserHostDelegationPayload {
  type Authority = BrowserVerifiedAuthority;

  fn kind(&self) -> &'static str {
    "browser"
  }

  fn validate_payload(&self) -> MResult<Self::Authority> {
    let runtime_config = self.runtime_injection.into_runtime_config()?;
    let browser_authority = self
      .runtime_injection
      .browser_host_config()?
      .into_browser_authority()?;
    Ok(BrowserVerifiedAuthority {
      runtime_config,
      browser_authority,
      runtime_injection: self.runtime_injection.clone(),
    })
  }

  fn encode_payload(&self, out: &mut Vec<u8>) {
    encode_browser_runtime_injection_config(out, &self.runtime_injection);
  }
}

#[cfg(feature = "delegation_signing")]
pub fn sign_browser_host_delegation(
  header: HostDelegationHeader,
  runtime_injection: BrowserRuntimeInjectionConfig,
  signing_key: &HostDelegationSigningKey,
) -> MResult<BrowserHostDelegationEnvelope> {
  sign_host_delegation(header, BrowserHostDelegationPayload { runtime_injection }, signing_key)
}

#[cfg(feature = "delegation_signing")]
pub fn verify_browser_host_delegation(
  envelope: &BrowserHostDelegationEnvelope,
  request: HostDelegationVerificationRequest,
) -> MResult<VerifiedBrowserHostDelegation> {
  verify_host_delegation(envelope, request)
}

pub fn encode_browser_host_config(out: &mut Vec<u8>, config: &BrowserHostConfig) {
  encode_string(out, &config.runtime.name);
  encode_option_u64(out, config.runtime.limits.max_steps_per_turn);
  encode_option_u64(out, config.runtime.limits.max_turn_duration_ms);
  encode_option_u64(out, config.runtime.limits.max_memory_bytes);
  encode_option_u64(out, config.runtime.limits.max_tasks);
  encode_option_u64(out, config.runtime.limits.max_actors);
  encode_option_u64(out, config.runtime.limits.max_actor_mailbox_len);
  encode_option_u64(out, config.runtime.limits.max_source_bytes);
  encode_option_u64(out, config.runtime.limits.max_in_memory_events);
  encode_bool(out, config.runtime.diagnostics.trace_enabled);
  encode_bool(out, config.runtime.diagnostics.profile_enabled);
  encode_bool(out, config.runtime.diagnostics.debug_enabled);
  encode_string(out, &config.runtime.diagnostics.log_level);

  let mut grants = config.browser.grants.clone();
  grants.sort_by(|left, right| grant_sort_key(left).cmp(&grant_sort_key(right)));
  encode_u64(out, grants.len() as u64);
  for grant in grants {
    push_resource(out, &grant.resource);
    let mut allow = grant.allow.clone();
    allow.sort();
    encode_u64(out, allow.len() as u64);
    for operation in allow {
      encode_string(out, &operation);
    }
  }

  let mut entries = config.browser.dom_manifest.clone();
  entries.sort_by(|left, right| {
    (&left.path, &left.selector, &left.property, &left.attribute)
      .cmp(&(&right.path, &right.selector, &right.property, &right.attribute))
  });
  encode_u64(out, entries.len() as u64);
  for entry in entries {
    encode_string(out, &entry.path);
    encode_string(out, &entry.selector);
    encode_string(out, &entry.property);
    encode_option_string(out, entry.attribute.as_deref());
  }
}

pub fn encode_browser_runtime_injection_config(
  out: &mut Vec<u8>,
  config: &BrowserRuntimeInjectionConfig,
) {
  encode_string(out, &config.runtime.name);
  encode_option_u64(out, config.runtime.limits.max_steps_per_turn);
  encode_option_u64(out, config.runtime.limits.max_turn_duration_ms);
  encode_option_u64(out, config.runtime.limits.max_memory_bytes);
  encode_option_u64(out, config.runtime.limits.max_tasks);
  encode_option_u64(out, config.runtime.limits.max_actors);
  encode_option_u64(out, config.runtime.limits.max_actor_mailbox_len);
  encode_option_u64(out, config.runtime.limits.max_source_bytes);
  encode_option_u64(out, config.runtime.limits.max_in_memory_events);
  encode_bool(out, config.runtime.diagnostics.trace_enabled);
  encode_bool(out, config.runtime.diagnostics.profile_enabled);
  encode_bool(out, config.runtime.diagnostics.debug_enabled);
  encode_string(out, &config.runtime.diagnostics.log_level);

  let mut hosts = config.hosts.clone();
  hosts.sort_by(|left, right| {
    (&left.name, &left.provider).cmp(&(&right.name, &right.provider))
  });
  encode_u64(out, hosts.len() as u64);
  for host in hosts {
    encode_string(out, &host.name);
    encode_string(out, &host.provider);
    encode_config_value(out, &host.settings);
  }

  let mut run_grants = config.run_grants.clone();
  run_grants.sort_by(|left, right| {
    (&left.target, &left.operations, &left.paths).cmp(&(&right.target, &right.operations, &right.paths))
  });
  encode_u64(out, run_grants.len() as u64);
  for grant in run_grants {
    encode_string(out, &grant.target);
    let mut operations = grant.operations;
    operations.sort();
    encode_u64(out, operations.len() as u64);
    for operation in operations {
      encode_string(out, &operation);
    }
    let mut paths = grant.paths;
    paths.sort();
    encode_u64(out, paths.len() as u64);
    for path in paths {
      encode_string(out, &path);
    }
  }
}

fn encode_config_value(out: &mut Vec<u8>, value: &ConfigValue) {
  match value {
    ConfigValue::Null => out.push(0),
    ConfigValue::Bool(value) => {
      out.push(1);
      encode_bool(out, *value);
    }
    ConfigValue::Integer(value) => {
      out.push(2);
      encode_string(out, &value.to_string());
    }
    ConfigValue::Float(value) => {
      out.push(3);
      encode_u64(out, value.to_bits());
    }
    ConfigValue::String(value) => {
      out.push(4);
      encode_string(out, value);
    }
    ConfigValue::List(values) => {
      out.push(5);
      encode_u64(out, values.len() as u64);
      for value in values {
        encode_config_value(out, value);
      }
    }
    ConfigValue::Map(values) => {
      out.push(6);
      encode_u64(out, values.len() as u64);
      for (key, value) in values {
        encode_string(out, key);
        encode_config_value(out, value);
      }
    }
  }
}

fn grant_sort_key(grant: &crate::BrowserHostBrowserGrant) -> (String, Vec<String>) {
  let mut allow = grant.allow.clone();
  allow.sort();
  (resource_sort_key(&grant.resource), allow)
}

fn resource_sort_key(resource: &BrowserHostResourceConfig) -> String {
  match resource {
    BrowserHostResourceConfig::Dom { selector } => format!("dom\0{selector}"),
    BrowserHostResourceConfig::Clipboard => "clipboard".to_string(),
    BrowserHostResourceConfig::Network { origin, methods } => {
      let mut methods = methods.clone().unwrap_or_default();
      methods.sort();
      format!("network\0{origin}\0{}", methods.join("\0"))
    }
    BrowserHostResourceConfig::Storage { backend, scope, recursive } => {
      format!("storage\0{backend}\0{scope}\0{recursive}")
    }
  }
}

fn push_resource(out: &mut Vec<u8>, resource: &BrowserHostResourceConfig) {
  match resource {
    BrowserHostResourceConfig::Dom { selector } => {
      encode_string(out, "dom");
      encode_string(out, selector);
    }
    BrowserHostResourceConfig::Clipboard => {
      encode_string(out, "clipboard");
    }
    BrowserHostResourceConfig::Network { origin, methods } => {
      encode_string(out, "network");
      encode_string(out, origin);
      match methods {
        Some(methods) => {
          out.push(1);
          let mut methods = methods.clone();
          methods.sort();
          encode_u64(out, methods.len() as u64);
          for method in methods {
            encode_string(out, &method);
          }
        }
        None => out.push(0),
      }
    }
    BrowserHostResourceConfig::Storage { backend, scope, recursive } => {
      encode_string(out, "storage");
      encode_string(out, backend);
      encode_string(out, scope);
      encode_bool(out, *recursive);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::collections::BTreeMap;
  use crate::{
    BrowserHostBrowserConfig, BrowserHostBrowserGrant, BrowserHostDiagnosticsConfig,
    BrowserHostDomManifestEntry, BrowserHostRuntimeConfig, BrowserHostRuntimeLimits,
  };
  use mech_core::BrowserOperation;
  use mech_runtime::{
    ConfigValue, HostInstanceConfig, RunResourceGrantConfig,
    HOST_DELEGATION_ALGORITHM_ED25519, HostDelegationKeyStore, HostDelegationPublicKey,
  };

  const PRIVATE_KEY: [u8; 32] = [
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
  ];

  fn host_config() -> BrowserHostConfig {
    BrowserHostConfig {
      runtime: BrowserHostRuntimeConfig {
        name: "demo".to_string(),
        limits: BrowserHostRuntimeLimits {
          max_steps_per_turn: Some(100),
          max_turn_duration_ms: None,
          max_memory_bytes: None,
          max_tasks: None,
          max_actors: None,
          max_actor_mailbox_len: None,
          max_source_bytes: None,
          max_in_memory_events: None,
        },
        diagnostics: BrowserHostDiagnosticsConfig {
          trace_enabled: false,
          profile_enabled: false,
          debug_enabled: false,
          log_level: "info".to_string(),
        },
      },
      browser: BrowserHostBrowserConfig {
        grants: vec![
          BrowserHostBrowserGrant {
            resource: BrowserHostResourceConfig::Dom { selector: "#out".to_string() },
            allow: vec!["write".to_string(), "read".to_string()],
          },
          BrowserHostBrowserGrant {
            resource: BrowserHostResourceConfig::Dom { selector: "#source".to_string() },
            allow: vec!["read".to_string()],
          },
          BrowserHostBrowserGrant {
            resource: BrowserHostResourceConfig::Network {
              origin: "https://example.com".to_string(),
              methods: Some(vec!["post".to_string(), "get".to_string()]),
            },
            allow: vec!["read".to_string()],
          },
        ],
        dom_manifest: vec![
          BrowserHostDomManifestEntry {
            path: "out/_text".to_string(),
            selector: "#out".to_string(),
            property: "text".to_string(),
            attribute: None,
          },
          BrowserHostDomManifestEntry {
            path: "source/_value".to_string(),
            selector: "#source".to_string(),
            property: "value".to_string(),
            attribute: None,
          },
        ],
      },
    }
  }

  fn runtime_injection_config() -> BrowserRuntimeInjectionConfig {
    BrowserRuntimeInjectionConfig {
      runtime: host_config().runtime,
      hosts: vec![HostInstanceConfig {
        name: "ui".to_string(),
        provider: "browser".to_string(),
        settings: browser_settings(),
      }],
      run_grants: vec![RunResourceGrantConfig {
        target: "ui/dom".to_string(),
        operations: vec!["write".to_string()],
        paths: vec!["ui/_value".to_string()],
      }],
    }
  }

  fn browser_settings() -> ConfigValue {
    ConfigValue::Map(BTreeMap::from([(
      "dom".to_string(),
      ConfigValue::List(vec![
        dom_setting("out/_text", "#out", "text", &["read", "write"]),
        dom_setting("source/_value", "#source", "value", &["read"]),
      ]),
    )]))
  }

  fn dom_setting(path: &str, selector: &str, property: &str, operations: &[&str]) -> ConfigValue {
    ConfigValue::Map(BTreeMap::from([
      ("path".to_string(), ConfigValue::String(path.to_string())),
      ("selector".to_string(), ConfigValue::String(selector.to_string())),
      ("property".to_string(), ConfigValue::String(property.to_string())),
      (
        "operations".to_string(),
        ConfigValue::List(operations.iter().map(|operation| ConfigValue::String((*operation).to_string())).collect()),
      ),
    ]))
  }

  fn header() -> HostDelegationHeader {
    HostDelegationHeader {
      issuer: "host://mech-cli".to_string(),
      subject: "wasm://browser".to_string(),
      audience: "browser://test".to_string(),
      key_id: "dev".to_string(),
      algorithm: HOST_DELEGATION_ALGORITHM_ED25519.to_string(),
      issued_at_ms: 1000,
      expires_at_ms: Some(10_000),
      nonce: Some("nonce".to_string()),
    }
  }

  #[test]
  fn browser_payload_is_deterministic_when_grants_are_reordered() {
    let config = host_config();
    let mut reordered = config.clone();
    reordered.browser.grants.reverse();
    assert_eq!(
      encode_browser_host_config_for_test(config),
      encode_browser_host_config_for_test(reordered),
    );
  }

  #[test]
  fn browser_payload_is_deterministic_when_allow_lists_are_reordered() {
    let config = host_config();
    let mut reordered = config.clone();
    reordered.browser.grants[0].allow.reverse();
    if let BrowserHostResourceConfig::Network { methods: Some(methods), .. } = &mut reordered.browser.grants[2].resource {
      methods.reverse();
    }
    assert_eq!(
      encode_browser_host_config_for_test(config),
      encode_browser_host_config_for_test(reordered),
    );
  }

  #[test]
  fn browser_payload_is_deterministic_when_dom_manifest_is_reordered() {
    let config = host_config();
    let mut reordered = config.clone();
    reordered.browser.dom_manifest.reverse();
    assert_eq!(
      encode_browser_host_config_for_test(config),
      encode_browser_host_config_for_test(reordered),
    );
  }

  fn encode_browser_host_config_for_test(config: BrowserHostConfig) -> Vec<u8> {
    let mut out = Vec::new();
    encode_browser_host_config(&mut out, &config);
    out
  }

  #[test]
  fn browser_runtime_injection_payload_includes_hosts_and_run_grants() {
    let config = runtime_injection_config();
    let payload = BrowserHostDelegationPayload { runtime_injection: config.clone() };
    assert!(payload.runtime_injection.hosts.iter().any(|host| host.name == "ui"));
    assert!(payload.runtime_injection.run_grants.iter().any(|grant| grant.target == "ui/dom"));
  }

  #[cfg(feature = "delegation_signing")]
  fn signing_key() -> HostDelegationSigningKey {
    HostDelegationSigningKey::from_ed25519_private_key_bytes(&PRIVATE_KEY).unwrap()
  }

  #[cfg(feature = "delegation_signing")]
  fn request(signing_key: &HostDelegationSigningKey) -> HostDelegationVerificationRequest {
    HostDelegationVerificationRequest {
      now_ms: 2000,
      expected_audience: "browser://test".to_string(),
      trusted_keys: HostDelegationKeyStore::new([HostDelegationPublicKey {
        issuer: "host://mech-cli".to_string(),
        key_id: "dev".to_string(),
        algorithm: HOST_DELEGATION_ALGORITHM_ED25519.to_string(),
        public_key: signing_key.public_key_bytes(),
      }]),
      max_clock_skew_ms: 0,
    }
  }

  #[cfg(feature = "delegation_signing")]
  #[test]
  fn valid_browser_host_delegation_verifies() {
    let key = signing_key();
    let envelope = sign_browser_host_delegation(header(), runtime_injection_config(), &key).unwrap();
    let verified = verify_browser_host_delegation(&envelope, request(&key)).unwrap();
    assert_eq!(verified.issuer, "host://mech-cli");
    assert!(verified.authority.runtime_injection.hosts.iter().any(|host| host.name == "ui"));
    assert!(verified.authority.runtime_injection.run_grants.iter().any(|grant| grant.target == "ui/dom"));
  }

  #[cfg(feature = "delegation_signing")]
  #[test]
  fn signed_delegation_payload_serializes_runtime_hosts_and_run_grants() {
    let key = signing_key();
    let envelope = sign_browser_host_delegation(header(), runtime_injection_config(), &key).unwrap();
    let json = serde_json::to_value(&envelope).unwrap();
    assert_eq!(json["payload"]["hosts"][0]["name"], "ui");
    assert_eq!(json["payload"]["runGrants"][0]["target"], "ui/dom");
    assert!(json["payload"].get("runtime").is_some());
    assert!(json["payload"].get("browser").is_none());
  }

  #[cfg(feature = "delegation_signing")]
  #[test]
  fn modified_browser_host_config_fails_signature_verification() {
    let key = signing_key();
    let mut envelope = sign_browser_host_delegation(header(), runtime_injection_config(), &key).unwrap();
    envelope.payload.runtime_injection.run_grants[0].paths.push("other/_value".to_string());
    assert!(verify_browser_host_delegation(&envelope, request(&key)).is_err());
  }

  #[cfg(feature = "delegation_signing")]
  #[test]
  fn verified_browser_authority_enforces_denied_dom_write() {
    let key = signing_key();
    let envelope = sign_browser_host_delegation(header(), runtime_injection_config(), &key).unwrap();
    let verified = verify_browser_host_delegation(&envelope, request(&key)).unwrap();
    let error = verified
      .authority
      .browser_authority
      .allows_dom("#source", BrowserOperation::Write)
      .unwrap_err();
    assert!(format!("{:?}", error).contains("OperationDenied"));
  }
}
