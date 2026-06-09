#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use mech_core::{BrowserAuthority, MResult, MechError, MechErrorKind};

use crate::{
  BrowserHostConfig, BrowserHostResourceConfig, RuntimeConfig,
};

pub const BROWSER_DELEGATION_FORMAT: &str = "mech.browser-delegation.v1";
pub const BROWSER_DELEGATION_ALGORITHM_ED25519: &str = "ed25519";
const BROWSER_DELEGATION_DOMAIN: &[u8] = b"mech.browser-delegation.v1";

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserDelegationEnvelope {
  pub format: String,
  pub header: BrowserDelegationHeader,
  pub host_config: BrowserHostConfig,
  pub signature: Option<BrowserDelegationSignature>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserDelegationHeader {
  pub issuer: String,
  pub subject: String,
  pub audience: String,
  pub key_id: String,
  pub algorithm: String,
  pub issued_at_ms: u64,
  pub expires_at_ms: Option<u64>,
  pub nonce: Option<String>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserDelegationSignature {
  pub bytes: Vec<u8>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserDelegationPublicKey {
  pub issuer: String,
  pub key_id: String,
  pub algorithm: String,
  pub public_key: Vec<u8>,
}

pub type BrowserDelegationKeyRecord = BrowserDelegationPublicKey;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserDelegationKeyStore {
  keys: Vec<BrowserDelegationPublicKey>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserDelegationVerificationRequest {
  pub now_ms: u64,
  pub expected_audience: String,
  pub trusted_keys: BrowserDelegationKeyStore,
  pub max_clock_skew_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedBrowserDelegation {
  pub issuer: String,
  pub subject: String,
  pub audience: String,
  pub key_id: String,
  pub runtime_config: RuntimeConfig,
  pub browser_authority: BrowserAuthority,
  pub host_config: BrowserHostConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidBrowserDelegationEnvelopeError {
  pub field: &'static str,
  pub reason: String,
}

impl MechErrorKind for InvalidBrowserDelegationEnvelopeError {
  fn name(&self) -> &str {
    "InvalidBrowserDelegationEnvelopeError"
  }

  fn message(&self) -> String {
    format!("Invalid browser delegation field `{}`: {}", self.field, self.reason)
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserDelegationSignatureError {
  pub reason: String,
}

impl MechErrorKind for BrowserDelegationSignatureError {
  fn name(&self) -> &str {
    "BrowserDelegationSignatureError"
  }

  fn message(&self) -> String {
    format!("Browser delegation signature verification failed: {}", self.reason)
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserDelegationKeyNotFoundError {
  pub issuer: String,
  pub key_id: String,
}

impl MechErrorKind for BrowserDelegationKeyNotFoundError {
  fn name(&self) -> &str {
    "BrowserDelegationKeyNotFoundError"
  }

  fn message(&self) -> String {
    format!(
      "Browser delegation key not found for issuer `{}` and key `{}`",
      self.issuer, self.key_id,
    )
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserDelegationExpiredError {
  pub now_ms: u64,
  pub issued_at_ms: u64,
  pub expires_at_ms: Option<u64>,
}

impl MechErrorKind for BrowserDelegationExpiredError {
  fn name(&self) -> &str {
    "BrowserDelegationExpiredError"
  }

  fn message(&self) -> String {
    format!(
      "Browser delegation is outside its valid time window: now={}, issuedAt={}, expiresAt={:?}",
      self.now_ms, self.issued_at_ms, self.expires_at_ms,
    )
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserDelegationWrongAudienceError {
  pub expected: String,
  pub actual: String,
}

impl MechErrorKind for BrowserDelegationWrongAudienceError {
  fn name(&self) -> &str {
    "BrowserDelegationWrongAudienceError"
  }

  fn message(&self) -> String {
    format!(
      "Browser delegation audience mismatch: expected `{}`, got `{}`",
      self.expected, self.actual,
    )
  }
}

impl BrowserDelegationEnvelope {
  pub fn unsigned(header: BrowserDelegationHeader, host_config: BrowserHostConfig) -> Self {
    Self {
      format: BROWSER_DELEGATION_FORMAT.to_string(),
      header,
      host_config,
      signature: None,
    }
  }

  pub fn validate_unsigned(&self) -> MResult<()> {
    if self.format != BROWSER_DELEGATION_FORMAT {
      return invalid("format", format!("must equal `{}`", BROWSER_DELEGATION_FORMAT));
    }
    if self.header.issuer.trim().is_empty() {
      return invalid("header.issuer", "must not be empty");
    }
    if self.header.subject.trim().is_empty() {
      return invalid("header.subject", "must not be empty");
    }
    if self.header.audience.trim().is_empty() {
      return invalid("header.audience", "must not be empty");
    }
    if self.header.key_id.trim().is_empty() {
      return invalid("header.keyId", "must not be empty");
    }
    if self.header.algorithm != BROWSER_DELEGATION_ALGORITHM_ED25519 {
      return invalid("header.algorithm", format!("must equal `{}`", BROWSER_DELEGATION_ALGORITHM_ED25519));
    }
    if let Some(expires_at_ms) = self.header.expires_at_ms {
      if expires_at_ms < self.header.issued_at_ms {
        return invalid("header.expiresAtMs", "must be greater than or equal to issuedAtMs");
      }
    }
    if let Some(nonce) = &self.header.nonce {
      if nonce.is_empty() {
        return invalid("header.nonce", "must not be empty when present");
      }
    }
    self.host_config.into_runtime_config()?;
    self.host_config.into_browser_authority()?;
    Ok(())
  }

  pub fn signing_payload(&self) -> MResult<Vec<u8>> {
    self.validate_unsigned()?;
    let mut out = Vec::new();
    push_len_prefixed(&mut out, BROWSER_DELEGATION_DOMAIN);
    push_string(&mut out, &self.format);
    push_string(&mut out, &self.header.issuer);
    push_string(&mut out, &self.header.subject);
    push_string(&mut out, &self.header.audience);
    push_string(&mut out, &self.header.key_id);
    push_string(&mut out, &self.header.algorithm);
    push_u64(&mut out, self.header.issued_at_ms);
    push_option_u64(&mut out, self.header.expires_at_ms);
    push_option_string(&mut out, self.header.nonce.as_deref());
    push_host_config(&mut out, &self.host_config);
    Ok(out)
  }
}

impl BrowserDelegationKeyStore {
  pub fn new(keys: impl IntoIterator<Item = BrowserDelegationPublicKey>) -> Self {
    Self { keys: keys.into_iter().collect() }
  }

  pub fn key(&self, issuer: &str, key_id: &str) -> Option<&BrowserDelegationPublicKey> {
    self
      .keys
      .iter()
      .find(|key| key.issuer == issuer && key.key_id == key_id)
  }

  pub fn keys(&self) -> &[BrowserDelegationPublicKey] {
    &self.keys
  }
}

fn push_host_config(out: &mut Vec<u8>, config: &BrowserHostConfig) {
  push_string(out, &config.runtime.name);
  push_option_u64(out, config.runtime.limits.max_steps_per_turn);
  push_option_u64(out, config.runtime.limits.max_turn_duration_ms);
  push_option_u64(out, config.runtime.limits.max_memory_bytes);
  push_option_u64(out, config.runtime.limits.max_tasks);
  push_option_u64(out, config.runtime.limits.max_actors);
  push_option_u64(out, config.runtime.limits.max_actor_mailbox_len);
  push_option_u64(out, config.runtime.limits.max_source_bytes);
  push_option_u64(out, config.runtime.limits.max_in_memory_events);
  push_bool(out, config.runtime.diagnostics.trace_enabled);
  push_bool(out, config.runtime.diagnostics.profile_enabled);
  push_bool(out, config.runtime.diagnostics.debug_enabled);
  push_string(out, &config.runtime.diagnostics.log_level);

  let mut grants = config.browser.grants.clone();
  grants.sort_by(|left, right| grant_sort_key(left).cmp(&grant_sort_key(right)));
  push_u64(out, grants.len() as u64);
  for grant in grants {
    push_resource(out, &grant.resource);
    let mut allow = grant.allow.clone();
    allow.sort();
    push_u64(out, allow.len() as u64);
    for operation in allow {
      push_string(out, &operation);
    }
  }

  let mut entries = config.browser.dom_manifest.clone();
  entries.sort_by(|left, right| {
    (&left.path, &left.selector, &left.property, &left.attribute)
      .cmp(&(&right.path, &right.selector, &right.property, &right.attribute))
  });
  push_u64(out, entries.len() as u64);
  for entry in entries {
    push_string(out, &entry.path);
    push_string(out, &entry.selector);
    push_string(out, &entry.property);
    push_option_string(out, entry.attribute.as_deref());
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
      push_string(out, "dom");
      push_string(out, selector);
    }
    BrowserHostResourceConfig::Clipboard => {
      push_string(out, "clipboard");
    }
    BrowserHostResourceConfig::Network { origin, methods } => {
      push_string(out, "network");
      push_string(out, origin);
      match methods {
        Some(methods) => {
          out.push(1);
          let mut methods = methods.clone();
          methods.sort();
          push_u64(out, methods.len() as u64);
          for method in methods {
            push_string(out, &method);
          }
        }
        None => out.push(0),
      }
    }
    BrowserHostResourceConfig::Storage { backend, scope, recursive } => {
      push_string(out, "storage");
      push_string(out, backend);
      push_string(out, scope);
      push_bool(out, *recursive);
    }
  }
}

fn push_len_prefixed(out: &mut Vec<u8>, bytes: &[u8]) {
  out.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
  out.extend_from_slice(bytes);
}

fn push_string(out: &mut Vec<u8>, value: &str) {
  push_len_prefixed(out, value.as_bytes());
}

fn push_option_string(out: &mut Vec<u8>, value: Option<&str>) {
  match value {
    Some(value) => {
      out.push(1);
      push_string(out, value);
    }
    None => out.push(0),
  }
}

fn push_u64(out: &mut Vec<u8>, value: u64) {
  out.extend_from_slice(&value.to_le_bytes());
}

fn push_option_u64(out: &mut Vec<u8>, value: Option<u64>) {
  match value {
    Some(value) => {
      out.push(1);
      push_u64(out, value);
    }
    None => out.push(0),
  }
}

fn push_bool(out: &mut Vec<u8>, value: bool) {
  out.push(u8::from(value));
}

fn invalid<T>(field: &'static str, reason: impl Into<String>) -> MResult<T> {
  Err(MechError::new(
    InvalidBrowserDelegationEnvelopeError { field, reason: reason.into() },
    None,
  ))
}

#[cfg(test)]
pub(crate) mod tests {
  use super::*;
  use crate::{
    BrowserHostBrowserConfig, BrowserHostBrowserGrant, BrowserHostDiagnosticsConfig,
    BrowserHostDomManifestEntry, BrowserHostRuntimeConfig, BrowserHostRuntimeLimits,
  };

  pub fn host_config() -> BrowserHostConfig {
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

  pub fn header() -> BrowserDelegationHeader {
    BrowserDelegationHeader {
      issuer: "host://mech-cli".to_string(),
      subject: "wasm://browser".to_string(),
      audience: "browser://test".to_string(),
      key_id: "dev".to_string(),
      algorithm: BROWSER_DELEGATION_ALGORITHM_ED25519.to_string(),
      issued_at_ms: 1000,
      expires_at_ms: Some(10_000),
      nonce: Some("nonce".to_string()),
    }
  }

  #[test]
  fn signing_payload_is_deterministic_when_grants_are_reordered() {
    let config = host_config();
    let mut reordered = config.clone();
    reordered.browser.grants.reverse();
    assert_eq!(
      BrowserDelegationEnvelope::unsigned(header(), config).signing_payload().unwrap(),
      BrowserDelegationEnvelope::unsigned(header(), reordered).signing_payload().unwrap(),
    );
  }

  #[test]
  fn signing_payload_is_deterministic_when_allow_lists_are_reordered() {
    let config = host_config();
    let mut reordered = config.clone();
    reordered.browser.grants[0].allow.reverse();
    if let BrowserHostResourceConfig::Network { methods: Some(methods), .. } = &mut reordered.browser.grants[1].resource {
      methods.reverse();
    }
    assert_eq!(
      BrowserDelegationEnvelope::unsigned(header(), config).signing_payload().unwrap(),
      BrowserDelegationEnvelope::unsigned(header(), reordered).signing_payload().unwrap(),
    );
  }
}
