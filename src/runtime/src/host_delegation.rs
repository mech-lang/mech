#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use mech_core::{MResult, MechError, MechErrorKind};

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

  pub fn validate_unsigned_header(&self) -> MResult<()> {
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
    Ok(())
  }

  pub fn validate_unsigned(&self) -> MResult<P::Authority> {
    self.validate_unsigned_header()?;
    self.payload.validate_payload()
  }

  pub fn signing_payload(&self) -> MResult<Vec<u8>> {
    self.validate_unsigned_header()?;
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

pub fn encode_len_prefixed(out: &mut Vec<u8>, bytes: &[u8]) {
  push_len_prefixed(out, bytes);
}

pub fn encode_string(out: &mut Vec<u8>, value: &str) {
  push_string(out, value);
}

pub fn encode_option_string(out: &mut Vec<u8>, value: Option<&str>) {
  push_option_string(out, value);
}

pub fn encode_u64(out: &mut Vec<u8>, value: u64) {
  push_u64(out, value);
}

pub fn encode_option_u64(out: &mut Vec<u8>, value: Option<u64>) {
  push_option_u64(out, value);
}

pub fn encode_bool(out: &mut Vec<u8>, value: bool) {
  push_bool(out, value);
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
      encode_string(out, &self.value);
    }
  }

  pub fn test_payload() -> TestPayload {
    TestPayload { kind: "test", value: "payload".to_string() }
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
}
