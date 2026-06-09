#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use mech_core::{BrowserAuthority, MResult, MechError, MechErrorKind};

use crate::{
  BrowserHostConfig, BrowserHostResourceConfig, RuntimeConfig,
};

pub const HOST_DELEGATION_FORMAT: &str = "mech.host-delegation.v1";
pub const HOST_DELEGATION_ALGORITHM_ED25519: &str = "ed25519";
const HOST_DELEGATION_DOMAIN: &[u8] = b"mech.host-delegation.v1";

pub trait HostDelegationPayload: Clone + std::fmt::Debug {
  type Authority;

  fn kind(&self) -> &'static str;
  fn validate_payload(&self) -> MResult<Self::Authority>;
  fn encode_payload(&self, out: &mut Vec<u8>);
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostDelegationEnvelope<P> {
  pub format: String,
  pub header: HostDelegationHeader,
  pub payload: P,
  pub signature: Option<HostDelegationSignature>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostDelegationHeader {
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
pub struct HostDelegationSignature {
  pub bytes: Vec<u8>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostDelegationPublicKey {
  pub issuer: String,
  pub key_id: String,
  pub algorithm: String,
  pub public_key: Vec<u8>,
}

pub type HostDelegationKeyRecord = HostDelegationPublicKey;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostDelegationKeyStore {
  keys: Vec<HostDelegationPublicKey>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostDelegationVerificationRequest {
  pub now_ms: u64,
  pub expected_audience: String,
  pub trusted_keys: HostDelegationKeyStore,
  pub max_clock_skew_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedHostDelegation<P, A> {
  pub issuer: String,
  pub subject: String,
  pub audience: String,
  pub key_id: String,
  pub payload: P,
  pub authority: A,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserHostDelegationPayload {
  pub host_config: BrowserHostConfig,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserVerifiedAuthority {
  pub runtime_config: RuntimeConfig,
  pub browser_authority: BrowserAuthority,
  pub host_config: BrowserHostConfig,
}

pub type BrowserHostDelegationEnvelope = HostDelegationEnvelope<BrowserHostDelegationPayload>;
pub type VerifiedBrowserHostDelegation = VerifiedHostDelegation<BrowserHostDelegationPayload, BrowserVerifiedAuthority>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidHostDelegationEnvelopeError {
  pub field: &'static str,
  pub reason: String,
}

impl MechErrorKind for InvalidHostDelegationEnvelopeError {
  fn name(&self) -> &str {
    "InvalidHostDelegationEnvelopeError"
  }

  fn message(&self) -> String {
    format!("Invalid host delegation field `{}`: {}", self.field, self.reason)
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostDelegationSignatureError {
  pub reason: String,
}

impl MechErrorKind for HostDelegationSignatureError {
  fn name(&self) -> &str {
    "HostDelegationSignatureError"
  }

  fn message(&self) -> String {
    format!("Host delegation signature verification failed: {}", self.reason)
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostDelegationKeyNotFoundError {
  pub issuer: String,
  pub key_id: String,
}

impl MechErrorKind for HostDelegationKeyNotFoundError {
  fn name(&self) -> &str {
    "HostDelegationKeyNotFoundError"
  }

  fn message(&self) -> String {
    format!(
      "Host delegation key not found for issuer `{}` and key `{}`",
      self.issuer, self.key_id,
    )
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostDelegationExpiredError {
  pub now_ms: u64,
  pub issued_at_ms: u64,
  pub expires_at_ms: Option<u64>,
}

impl MechErrorKind for HostDelegationExpiredError {
  fn name(&self) -> &str {
    "HostDelegationExpiredError"
  }

  fn message(&self) -> String {
    format!(
      "Host delegation is outside its valid time window: now={}, issuedAt={}, expiresAt={:?}",
      self.now_ms, self.issued_at_ms, self.expires_at_ms,
    )
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostDelegationWrongAudienceError {
  pub expected: String,
  pub actual: String,
}

impl MechErrorKind for HostDelegationWrongAudienceError {
  fn name(&self) -> &str {
    "HostDelegationWrongAudienceError"
  }

  fn message(&self) -> String {
    format!(
      "Host delegation audience mismatch: expected `{}`, got `{}`",
      self.expected, self.actual,
    )
  }
}

impl<P: HostDelegationPayload> HostDelegationEnvelope<P> {
  pub fn unsigned(header: HostDelegationHeader, payload: P) -> Self {
    Self {
      format: HOST_DELEGATION_FORMAT.to_string(),
      header,
      payload,
      signature: None,
    }
  }

  pub fn validate_unsigned(&self) -> MResult<P::Authority> {
    if self.format != HOST_DELEGATION_FORMAT {
      return invalid("format", format!("must equal `{}`", HOST_DELEGATION_FORMAT));
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
    if self.header.algorithm != HOST_DELEGATION_ALGORITHM_ED25519 {
      return invalid("header.algorithm", format!("must equal `{}`", HOST_DELEGATION_ALGORITHM_ED25519));
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
    self.payload.validate_payload()
  }

  pub fn signing_payload(&self) -> MResult<Vec<u8>> {
    self.validate_unsigned()?;
    let mut out = Vec::new();
    push_len_prefixed(&mut out, HOST_DELEGATION_DOMAIN);
    push_string(&mut out, &self.format);
    push_string(&mut out, self.payload.kind());
    push_string(&mut out, &self.header.issuer);
    push_string(&mut out, &self.header.subject);
    push_string(&mut out, &self.header.audience);
    push_string(&mut out, &self.header.key_id);
    push_string(&mut out, &self.header.algorithm);
    push_u64(&mut out, self.header.issued_at_ms);
    push_option_u64(&mut out, self.header.expires_at_ms);
    push_option_string(&mut out, self.header.nonce.as_deref());
    self.payload.encode_payload(&mut out);
    Ok(out)
  }
}

impl HostDelegationPayload for BrowserHostDelegationPayload {
  type Authority = BrowserVerifiedAuthority;

  fn kind(&self) -> &'static str {
    "browser"
  }

  fn validate_payload(&self) -> MResult<Self::Authority> {
    let runtime_config = self.host_config.into_runtime_config()?;
    let browser_authority = self.host_config.into_browser_authority()?;
    Ok(BrowserVerifiedAuthority {
      runtime_config,
      browser_authority,
      host_config: self.host_config.clone(),
    })
  }

  fn encode_payload(&self, out: &mut Vec<u8>) {
    encode_browser_host_config(out, &self.host_config);
  }
}

impl HostDelegationKeyStore {
  pub fn new(keys: impl IntoIterator<Item = HostDelegationPublicKey>) -> Self {
    Self { keys: keys.into_iter().collect() }
  }

  pub fn key(&self, issuer: &str, key_id: &str) -> Option<&HostDelegationPublicKey> {
    self
      .keys
      .iter()
      .find(|key| key.issuer == issuer && key.key_id == key_id)
  }

  pub fn keys(&self) -> &[HostDelegationPublicKey] {
    &self.keys
  }
}

pub fn encode_browser_host_config(out: &mut Vec<u8>, config: &BrowserHostConfig) {
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
    InvalidHostDelegationEnvelopeError { field, reason: reason.into() },
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

  #[derive(Clone, Debug, PartialEq, Eq)]
  pub struct TestPayload {
    pub kind: &'static str,
    pub value: String,
  }

  impl HostDelegationPayload for TestPayload {
    type Authority = String;

    fn kind(&self) -> &'static str {
      self.kind
    }

    fn validate_payload(&self) -> MResult<Self::Authority> {
      Ok(self.value.clone())
    }

    fn encode_payload(&self, out: &mut Vec<u8>) {
      push_string(out, &self.value);
    }
  }

  pub fn test_payload() -> TestPayload {
    TestPayload { kind: "test", value: "payload".to_string() }
  }

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

  pub fn browser_payload() -> BrowserHostDelegationPayload {
    BrowserHostDelegationPayload { host_config: host_config() }
  }

  pub fn header() -> HostDelegationHeader {
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
  fn generic_payload_kind_is_part_of_signing_payload() {
    let first = HostDelegationEnvelope::unsigned(header(), TestPayload { kind: "first", value: "same".to_string() });
    let second = HostDelegationEnvelope::unsigned(header(), TestPayload { kind: "second", value: "same".to_string() });
    assert_ne!(first.signing_payload().unwrap(), second.signing_payload().unwrap());
  }

  #[test]
  fn generic_signing_payload_is_deterministic() {
    let first = HostDelegationEnvelope::unsigned(header(), test_payload());
    let second = HostDelegationEnvelope::unsigned(header(), test_payload());
    assert_eq!(first.signing_payload().unwrap(), second.signing_payload().unwrap());
  }

  #[test]
  fn browser_payload_is_deterministic_when_grants_are_reordered() {
    let config = host_config();
    let mut reordered = config.clone();
    reordered.browser.grants.reverse();
    assert_eq!(
      HostDelegationEnvelope::unsigned(header(), BrowserHostDelegationPayload { host_config: config }).signing_payload().unwrap(),
      HostDelegationEnvelope::unsigned(header(), BrowserHostDelegationPayload { host_config: reordered }).signing_payload().unwrap(),
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
      HostDelegationEnvelope::unsigned(header(), BrowserHostDelegationPayload { host_config: config }).signing_payload().unwrap(),
      HostDelegationEnvelope::unsigned(header(), BrowserHostDelegationPayload { host_config: reordered }).signing_payload().unwrap(),
    );
  }

  #[test]
  fn browser_payload_is_deterministic_when_dom_manifest_is_reordered() {
    let config = host_config();
    let mut reordered = config.clone();
    reordered.browser.dom_manifest.reverse();
    assert_eq!(
      HostDelegationEnvelope::unsigned(header(), BrowserHostDelegationPayload { host_config: config }).signing_payload().unwrap(),
      HostDelegationEnvelope::unsigned(header(), BrowserHostDelegationPayload { host_config: reordered }).signing_payload().unwrap(),
    );
  }
}
