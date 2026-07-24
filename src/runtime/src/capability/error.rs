use crate::*;

use mech_core::*;

// -----------------------------------------------------------------------------
// Errors
// -----------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct InvalidCapabilityError {
  pub field: &'static str,
  pub reason: &'static str,
}

impl MechErrorKind for InvalidCapabilityError {
  fn name(&self) -> &str {
    "InvalidCapability"
  }

  fn message(&self) -> String {
    format!("Invalid capability field `{}`: {}", self.field, self.reason)
  }
}

#[derive(Debug, Clone)]
pub struct CapabilityDeniedError {
  pub subject: String,
  pub operation: String,
  pub resource: String,
  pub reason: String,
}

impl MechErrorKind for CapabilityDeniedError {
  fn name(&self) -> &str {
    "CapabilityDenied"
  }

  fn message(&self) -> String {
    format!(
      "Capability denied: subject `{}` cannot `{}` resource `{}`: {}",
      self.subject, self.operation, self.resource, self.reason
    )
  }
}

#[derive(Debug, Clone)]
pub struct CapabilityAlreadyExistsError {
  pub capability: CapabilityId,
}

impl MechErrorKind for CapabilityAlreadyExistsError {
  fn name(&self) -> &str {
    "CapabilityAlreadyExists"
  }

  fn message(&self) -> String {
    format!("Capability already exists: {}", self.capability)
  }
}

#[derive(Debug, Clone)]
pub struct CapabilityNotFoundError {
  pub capability: CapabilityId,
}

impl MechErrorKind for CapabilityNotFoundError {
  fn name(&self) -> &str {
    "CapabilityNotFound"
  }

  fn message(&self) -> String {
    format!("Capability not found: {}", self.capability)
  }
}

#[derive(Debug, Clone)]
pub struct CapabilityRevokedError {
  pub capability: CapabilityId,
}

impl MechErrorKind for CapabilityRevokedError {
  fn name(&self) -> &str {
    "CapabilityRevoked"
  }

  fn message(&self) -> String {
    format!("Capability has been revoked: {}", self.capability)
  }
}

#[derive(Debug, Clone)]
pub struct CapabilityNotRevocableError {
  pub capability: CapabilityId,
}

impl MechErrorKind for CapabilityNotRevocableError {
  fn name(&self) -> &str {
    "CapabilityNotRevocable"
  }

  fn message(&self) -> String {
    format!("Capability is not revocable: {}", self.capability)
  }
}

#[derive(Debug, Clone)]
pub struct CapabilityNotDelegableError {
  pub capability: CapabilityId,
}

impl MechErrorKind for CapabilityNotDelegableError {
  fn name(&self) -> &str {
    "CapabilityNotDelegable"
  }

  fn message(&self) -> String {
    format!("Capability is not delegable: {}", self.capability)
  }
}

#[derive(Debug, Clone)]
pub struct CapabilityNotAttenuableError {
  pub capability: CapabilityId,
}

impl MechErrorKind for CapabilityNotAttenuableError {
  fn name(&self) -> &str {
    "CapabilityNotAttenuable"
  }

  fn message(&self) -> String {
    format!("Capability is not attenuable: {}", self.capability)
  }
}

#[derive(Debug, Clone)]
pub struct CapabilityDerivationUnsupportedError {
  pub capability: CapabilityId,
}

impl MechErrorKind for CapabilityDerivationUnsupportedError {
  fn name(&self) -> &str {
    "CapabilityDerivationUnsupported"
  }

  fn message(&self) -> String {
    format!("Capability does not support derivation: {}", self.capability)
  }
}

#[derive(Debug, Clone)]
pub struct InvalidCapabilityDerivationError {
  pub reason: String,
}

impl MechErrorKind for InvalidCapabilityDerivationError {
  fn name(&self) -> &str {
    "InvalidCapabilityDerivation"
  }

  fn message(&self) -> String {
    format!("Invalid capability derivation: {}", self.reason)
  }
}

#[derive(Debug, Clone)]
pub struct InvalidCapabilityTokenError {
  pub token: CapabilityId,
  pub reason: String,
}

impl MechErrorKind for InvalidCapabilityTokenError {
  fn name(&self) -> &str {
    "InvalidCapabilityToken"
  }

  fn message(&self) -> String {
    format!("Invalid capability token `{}`: {}", self.token, self.reason)
  }
}

#[derive(Debug, Clone)]
pub struct CapabilityTokenAlreadySignedError {
  pub token: CapabilityId,
}

impl MechErrorKind for CapabilityTokenAlreadySignedError {
  fn name(&self) -> &str {
    "CapabilityTokenAlreadySigned"
  }

  fn message(&self) -> String {
    format!("Capability token already signed: {}", self.token)
  }
}

#[derive(Debug, Clone)]
pub struct CapabilityTokenRevokedError {
  pub token: CapabilityId,
}

impl MechErrorKind for CapabilityTokenRevokedError {
  fn name(&self) -> &str {
    "CapabilityTokenRevoked"
  }

  fn message(&self) -> String {
    format!("Capability token has been revoked: {}", self.token)
  }
}

#[derive(Debug, Clone)]
pub struct InvalidCapabilityKeyError {
  pub field: &'static str,
  pub reason: &'static str,
}

impl MechErrorKind for InvalidCapabilityKeyError {
  fn name(&self) -> &str {
    "InvalidCapabilityKey"
  }

  fn message(&self) -> String {
    format!("Invalid capability key field `{}`: {}", self.field, self.reason)
  }
}

#[derive(Debug, Clone)]
pub struct CapabilityKeyNotFoundError {
  pub issuer: String,
  pub key_id: String,
}

impl MechErrorKind for CapabilityKeyNotFoundError {
  fn name(&self) -> &str {
    "CapabilityKeyNotFound"
  }

  fn message(&self) -> String {
    format!(
      "Capability signing key not found: issuer `{}`, key `{}`",
      self.issuer, self.key_id,
    )
  }
}

#[derive(Debug, Clone)]
pub struct CapabilityKeyNotUsableError {
  pub issuer: String,
  pub key_id: String,
  pub reason: String,
}

impl MechErrorKind for CapabilityKeyNotUsableError {
  fn name(&self) -> &str {
    "CapabilityKeyNotUsable"
  }

  fn message(&self) -> String {
    format!(
      "Capability signing key is not usable: issuer `{}`, key `{}`: {}",
      self.issuer, self.key_id, self.reason,
    )
  }
}

pub fn invalid_capability<T>(field: &'static str, reason: &'static str) -> MResult<T> {
  Err(MechError::new(
    InvalidCapabilityError { field, reason },
    None,
  ))
}

pub fn require_nonzero_opt(field: &'static str, value: Option<u64>) -> MResult<()> {
  if matches!(value, Some(0)) {
    return invalid_capability(field, "must be greater than zero");
  }

  Ok(())
}

pub fn invalid_key<T>(field: &'static str, reason: &'static str) -> MResult<T> {
  Err(MechError::new(
    InvalidCapabilityKeyError { field, reason },
    None,
  ))
}

pub fn require_limit_not_relaxed(
  field: &'static str,
  source: Option<u64>,
  derived: Option<u64>,
) -> MResult<()> {
  match (source, derived) {
    (Some(_), None) => Err(MechError::new(
      InvalidCapabilityDerivationError {
        reason: format!("derived constraints cannot remove `{}` limit", field),
      },
      None,
    )),
    (Some(source), Some(derived)) if derived > source => Err(MechError::new(
      InvalidCapabilityDerivationError {
        reason: format!(
          "derived constraints cannot increase `{}` limit: source {}, derived {}",
          field, source, derived,
        ),
      },
      None,
    )),
    _ => Ok(()),
  }
}
