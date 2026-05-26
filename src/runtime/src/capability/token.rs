#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

use crate::*;
use mech_core::*;
use std::sync::Arc;

const BASIC_CAPABILITY_TOKEN_DOMAIN: &[u8] = b"mech.capability-token.v1";

/// Optional portable capability representation.
///
/// Tokens are useful when authority crosses a trust boundary. Local runtime
/// authority should normally be represented in the kernel/store, not only as a
/// bearer token.
///
/// Signature verification is not authorization. A valid signature proves only
/// that the token was signed by a trusted issuer key. Runtime authorization
/// still needs to resolve and check the referenced capabilities.
pub trait CapabilityToken: std::fmt::Debug + Send + Sync {
  fn id(&self) -> CapabilityId;

  fn subject_key(&self) -> &str;

  fn issuer_key(&self) -> &str;

  fn key_id(&self) -> &str;

  fn audience(&self) -> Option<&str>;

  fn issued_at_ms(&self) -> u64;

  fn not_before_ms(&self) -> Option<u64>;

  fn expires_at_ms(&self) -> Option<u64>;

  fn capability_ids(&self) -> &[CapabilityId];

  fn signature(&self) -> Option<&[u8]>;

  fn validate_unsigned(&self) -> MResult<()>;

  fn signing_payload(&self) -> MResult<Vec<u8>>;
}

pub trait CapabilitySigner: std::fmt::Debug + Send + Sync {
  fn sign(&self, payload: &[u8]) -> MResult<Vec<u8>>;
}

// -----------------------------------------------------------------------------
// Token Support: Basic Signed Token Shape
// -----------------------------------------------------------------------------

/// Default token payload.
///
/// This is not required by the kernel. It is a simple serializable token shape
/// for hosts that want portable capability tokens. Signing/verification remains
/// trait-based.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BasicCapabilityToken {
  pub id: CapabilityId,
  pub subject: String,
  pub capability_ids: Vec<CapabilityId>,
  pub issuer: String,
  pub audience: Option<String>,
  pub key_id: String,
  pub issued_at_ms: u64,
  pub not_before_ms: Option<u64>,
  pub expires_at_ms: Option<u64>,
  pub nonce: Option<Vec<u8>>,
  pub signature: Option<Vec<u8>>,
}

impl BasicCapabilityToken {
  pub fn new(
    id: CapabilityId,
    subject: &dyn Subject,
    issuer: &dyn Subject,
    key_id: impl Into<String>,
    issued_at_ms: u64,
    capability_ids: Vec<CapabilityId>,
  ) -> Self {
    Self {
      id,
      subject: subject.key().to_string(),
      capability_ids,
      issuer: issuer.key().to_string(),
      audience: None,
      key_id: key_id.into(),
      issued_at_ms,
      not_before_ms: None,
      expires_at_ms: None,
      nonce: None,
      signature: None,
    }
  }

  pub fn with_audience(mut self, audience: impl Into<String>) -> Self {
    self.audience = Some(audience.into());
    self
  }

  pub fn with_not_before_ms(mut self, not_before_ms: u64) -> Self {
    self.not_before_ms = Some(not_before_ms);
    self
  }

  pub fn with_expiration_ms(mut self, expires_at_ms: u64) -> Self {
    self.expires_at_ms = Some(expires_at_ms);
    self
  }

  pub fn with_nonce(mut self, nonce: Vec<u8>) -> Self {
    self.nonce = Some(nonce);
    self
  }

  pub fn sign(&mut self, signer: &dyn CapabilitySigner) -> MResult<()> {
    if self.signature.is_some() {
      return Err(MechError::new(
        CapabilityTokenAlreadySignedError { token: self.id },
        None,
      ));
    }

    let payload = self.signing_payload()?;
    let signature = signer.sign(&payload)?;
    self.signature = Some(signature);
    Ok(())
  }

  pub fn clear_signature(&mut self) {
    self.signature = None;
  }
}

impl CapabilityToken for BasicCapabilityToken {
  fn id(&self) -> CapabilityId {
    self.id
  }

  fn subject_key(&self) -> &str {
    &self.subject
  }

  fn issuer_key(&self) -> &str {
    &self.issuer
  }

  fn key_id(&self) -> &str {
    &self.key_id
  }

  fn audience(&self) -> Option<&str> {
    self.audience.as_deref()
  }

  fn issued_at_ms(&self) -> u64 {
    self.issued_at_ms
  }

  fn not_before_ms(&self) -> Option<u64> {
    self.not_before_ms
  }

  fn expires_at_ms(&self) -> Option<u64> {
    self.expires_at_ms
  }

  fn capability_ids(&self) -> &[CapabilityId] {
    &self.capability_ids
  }

  fn signature(&self) -> Option<&[u8]> {
    self.signature.as_deref()
  }

  fn validate_unsigned(&self) -> MResult<()> {
    if self.id.is_zero() {
      return Err(MechError::new(
        InvalidCapabilityTokenError {
          token: self.id,
          reason: "token id must not be zero".to_string(),
        },
        None,
      ));
    }

    if self.subject.trim().is_empty() {
      return Err(MechError::new(
        InvalidCapabilityTokenError {
          token: self.id,
          reason: "subject must not be empty".to_string(),
        },
        None,
      ));
    }

    if self.issuer.trim().is_empty() {
      return Err(MechError::new(
        InvalidCapabilityTokenError {
          token: self.id,
          reason: "issuer must not be empty".to_string(),
        },
        None,
      ));
    }

    if self.key_id.trim().is_empty() {
      return Err(MechError::new(
        InvalidCapabilityTokenError {
          token: self.id,
          reason: "key_id must not be empty".to_string(),
        },
        None,
      ));
    }

    if let Some(audience) = &self.audience {
      if audience.trim().is_empty() {
        return Err(MechError::new(
          InvalidCapabilityTokenError {
            token: self.id,
            reason: "audience must not be empty when present".to_string(),
          },
          None,
        ));
      }
    }

    if self.capability_ids.is_empty() {
      return Err(MechError::new(
        InvalidCapabilityTokenError {
          token: self.id,
          reason: "token must reference at least one capability".to_string(),
        },
        None,
      ));
    }

    for capability_id in &self.capability_ids {
      if capability_id.is_zero() {
        return Err(MechError::new(
          InvalidCapabilityTokenError {
            token: self.id,
            reason: "token must not reference zero capability ids".to_string(),
          },
          None,
        ));
      }
    }

    if let Some(not_before) = self.not_before_ms {
      if not_before < self.issued_at_ms {
        return Err(MechError::new(
          InvalidCapabilityTokenError {
            token: self.id,
            reason: "not_before_ms must be greater than or equal to issued_at_ms".to_string(),
          },
          None,
        ));
      }
    }

    if let Some(expires_at) = self.expires_at_ms {
      if expires_at < self.issued_at_ms {
        return Err(MechError::new(
          InvalidCapabilityTokenError {
            token: self.id,
            reason: "expires_at_ms must be greater than or equal to issued_at_ms".to_string(),
          },
          None,
        ));
      }

      if let Some(not_before) = self.not_before_ms {
        if expires_at < not_before {
          return Err(MechError::new(
            InvalidCapabilityTokenError {
              token: self.id,
              reason: "expires_at_ms must be greater than or equal to not_before_ms".to_string(),
            },
            None,
          ));
        }
      }
    }

    if let Some(nonce) = &self.nonce {
      if nonce.is_empty() {
        return Err(MechError::new(
          InvalidCapabilityTokenError {
            token: self.id,
            reason: "nonce must not be empty when present".to_string(),
          },
          None,
        ));
      }
    }

    Ok(())
  }

  fn signing_payload(&self) -> MResult<Vec<u8>> {
    self.validate_unsigned()?;

    let mut out = Vec::new();

    push_len_prefixed(&mut out, BASIC_CAPABILITY_TOKEN_DOMAIN);
    out.extend_from_slice(&self.id.as_u128().to_le_bytes());
    push_len_prefixed(&mut out, self.subject.as_bytes());
    push_len_prefixed(&mut out, self.issuer.as_bytes());
    push_len_prefixed(&mut out, self.key_id.as_bytes());
    out.extend_from_slice(&self.issued_at_ms.to_le_bytes());

    match self.not_before_ms {
      Some(value) => {
        out.push(1);
        out.extend_from_slice(&value.to_le_bytes());
      }
      None => out.push(0),
    }

    match self.expires_at_ms {
      Some(value) => {
        out.push(1);
        out.extend_from_slice(&value.to_le_bytes());
      }
      None => out.push(0),
    }

    match &self.audience {
      Some(value) => {
        out.push(1);
        push_len_prefixed(&mut out, value.as_bytes());
      }
      None => out.push(0),
    }

    match &self.nonce {
      Some(value) => {
        out.push(1);
        push_len_prefixed(&mut out, value);
      }
      None => out.push(0),
    }

    let mut ids = self.capability_ids.clone();
    ids.sort();

    out.extend_from_slice(&(ids.len() as u64).to_le_bytes());
    for id in ids {
      out.extend_from_slice(&id.as_u128().to_le_bytes());
    }

    Ok(out)
  }
}

fn push_len_prefixed(out: &mut Vec<u8>, bytes: &[u8]) {
  out.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
  out.extend_from_slice(bytes);
}


// -----------------------------------------------------------------------------
// Token, Signing, Verification, Key Resolution, and Validation Traits
// -----------------------------------------------------------------------------

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CapabilitySigningKeyStatus {
  Active,
  Retired,
  Revoked,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapabilitySigningKeyRecord {
  pub issuer: String,
  pub key_id: String,
  pub algorithm: String,
  pub public_key: Vec<u8>,
  pub status: CapabilitySigningKeyStatus,
  pub created_at_ms: u64,
  pub not_before_ms: Option<u64>,
  pub not_after_ms: Option<u64>,
}

impl CapabilitySigningKeyRecord {
  pub fn validate(&self) -> MResult<()> {
    if self.issuer.trim().is_empty() {
      return invalid_key("issuer", "must not be empty");
    }

    if self.key_id.trim().is_empty() {
      return invalid_key("key_id", "must not be empty");
    }

    if self.algorithm.trim().is_empty() {
      return invalid_key("algorithm", "must not be empty");
    }

    if self.public_key.is_empty() {
      return invalid_key("public_key", "must not be empty");
    }

    if let (Some(start), Some(end)) = (self.not_before_ms, self.not_after_ms) {
      if start > end {
        return invalid_key("not_before_ms", "must be less than or equal to not_after_ms");
      }
    }

    Ok(())
  }

  pub fn may_verify_at(&self, now_ms: u64) -> bool {
    if self.status == CapabilitySigningKeyStatus::Revoked {
      return false;
    }

    if let Some(not_before) = self.not_before_ms {
      if now_ms < not_before {
        return false;
      }
    }

    if let Some(not_after) = self.not_after_ms {
      if now_ms > not_after {
        return false;
      }
    }

    true
  }

  pub fn may_sign_at(&self, now_ms: u64) -> bool {
    self.status == CapabilitySigningKeyStatus::Active && self.may_verify_at(now_ms)
  }
}

/// Resolves token-referenced capabilities and token revocation state.
pub trait CapabilityTokenResolver: std::fmt::Debug + Send + Sync {
  fn is_token_revoked(&self, token: CapabilityId) -> MResult<bool>;

  fn resolve_capability(
    &self,
    capability: CapabilityId,
  ) -> MResult<Option<Arc<dyn Capability>>>;

  fn is_capability_revoked(&self, capability: CapabilityId) -> MResult<bool>;
}

#[derive(Clone, Debug)]
pub struct CapabilityTokenValidationRequest<'a> {
  pub token: &'a dyn CapabilityToken,
  pub now_ms: u64,
  pub audience: Option<&'a str>,
  pub max_clock_skew_ms: u64,
}

#[derive(Clone, Debug)]
pub struct ValidatedCapabilityToken {
  pub token: CapabilityId,
  pub subject: String,
  pub issuer: String,
  pub key_id: String,
  pub capability_ids: Vec<CapabilityId>,
  pub capabilities: Vec<Arc<dyn Capability>>,
}

pub trait CapabilityTokenValidator: std::fmt::Debug + Send + Sync {
  fn validate_token(
    &self,
    request: CapabilityTokenValidationRequest<'_>,
  ) -> MResult<ValidatedCapabilityToken>;
}

#[derive(Debug)]
pub struct BasicCapabilityTokenValidator<K, R> {
  pub keys: K,
  pub resolver: R,
}

impl<K, R> BasicCapabilityTokenValidator<K, R> {
  pub fn new(keys: K, resolver: R) -> Self {
    Self { keys, resolver }
  }
}

impl<K, R> CapabilityTokenValidator for BasicCapabilityTokenValidator<K, R>
where
  K: CapabilityKeyResolver,
  R: CapabilityTokenResolver,
{
  fn validate_token(
    &self,
    request: CapabilityTokenValidationRequest<'_>,
  ) -> MResult<ValidatedCapabilityToken> {
    let token = request.token;
    token.validate_unsigned()?;

    let key_record = self
      .keys
      .key_record(token.issuer_key(), token.key_id())?
      .ok_or_else(|| {
        MechError::new(
          CapabilityKeyNotFoundError {
            issuer: token.issuer_key().to_string(),
            key_id: token.key_id().to_string(),
          },
          None,
        )
      })?;

    key_record.validate()?;

    if !key_record.may_verify_at(request.now_ms) {
      return Err(MechError::new(
        CapabilityKeyNotUsableError {
          issuer: token.issuer_key().to_string(),
          key_id: token.key_id().to_string(),
          reason: "key is not usable for verification at this time".to_string(),
        },
        None,
      ));
    }

    let verifier = self
      .keys
      .verifier_for(token.issuer_key(), token.key_id())?
      .ok_or_else(|| {
        MechError::new(
          CapabilityKeyNotFoundError {
            issuer: token.issuer_key().to_string(),
            key_id: token.key_id().to_string(),
          },
          None,
        )
      })?;

    let signature = token.signature().ok_or_else(|| {
      MechError::new(
        InvalidCapabilityTokenError {
          token: token.id(),
          reason: "missing signature".to_string(),
        },
        None,
      )
    })?;

    let payload = token.signing_payload()?;
    verifier.verify(&payload, signature)?;

    if self.resolver.is_token_revoked(token.id())? {
      return Err(MechError::new(
        CapabilityTokenRevokedError { token: token.id() },
        None,
      ));
    }

    if let Some(audience) = token.audience() {
      let Some(expected) = request.audience else {
        return Err(MechError::new(
          InvalidCapabilityTokenError {
            token: token.id(),
            reason: "token has an audience but no expected audience was supplied".to_string(),
          },
          None,
        ));
      };

      if audience != expected {
        return Err(MechError::new(
          InvalidCapabilityTokenError {
            token: token.id(),
            reason: format!("wrong audience: expected `{}`, actual `{}`", expected, audience),
          },
          None,
        ));
      }
    }

    let earliest = token
      .not_before_ms()
      .unwrap_or(token.issued_at_ms())
      .saturating_sub(request.max_clock_skew_ms);

    if request.now_ms < earliest {
      return Err(MechError::new(
        InvalidCapabilityTokenError {
          token: token.id(),
          reason: "token is not yet valid".to_string(),
        },
        None,
      ));
    }

    if let Some(expires_at) = token.expires_at_ms() {
      let latest = expires_at.saturating_add(request.max_clock_skew_ms);
      if request.now_ms > latest {
        return Err(MechError::new(
          InvalidCapabilityTokenError {
            token: token.id(),
            reason: "token has expired".to_string(),
          },
          None,
        ));
      }
    }

    let mut capabilities = Vec::with_capacity(token.capability_ids().len());

    for capability_id in token.capability_ids() {
      if self.resolver.is_capability_revoked(*capability_id)? {
        return Err(MechError::new(
          CapabilityRevokedError {
            capability: *capability_id,
          },
          None,
        ));
      }

      let capability = self
        .resolver
        .resolve_capability(*capability_id)?
        .ok_or_else(|| {
          MechError::new(
            CapabilityNotFoundError {
              capability: *capability_id,
            },
            None,
          )
        })?;

      capability.validate()?;

      if capability.subject_key() != token.subject_key() {
        return Err(MechError::new(
          InvalidCapabilityTokenError {
            token: token.id(),
            reason: format!(
              "capability `{}` belongs to `{}`, not token subject `{}`",
              capability.id(),
              capability.subject_key(),
              token.subject_key(),
            ),
          },
          None,
        ));
      }

      capabilities.push(capability);
    }

    Ok(ValidatedCapabilityToken {
      token: token.id(),
      subject: token.subject_key().to_string(),
      issuer: token.issuer_key().to_string(),
      key_id: token.key_id().to_string(),
      capability_ids: token.capability_ids().to_vec(),
      capabilities,
    })
  }
}

fn invalid_key<T>(field: &'static str, reason: &'static str) -> MResult<T> {
  Err(MechError::new(
    InvalidCapabilityKeyError { field, reason },
    None,
  ))
}