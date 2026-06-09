use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use mech_core::{MResult, MechError};

use crate::{
  BrowserDelegationEnvelope, BrowserDelegationExpiredError, BrowserDelegationHeader,
  BrowserDelegationKeyNotFoundError, BrowserDelegationPublicKey,
  BrowserDelegationSignature, BrowserDelegationSignatureError, BrowserDelegationVerificationRequest,
  BrowserDelegationWrongAudienceError, BrowserHostConfig, VerifiedBrowserDelegation,
  BROWSER_DELEGATION_ALGORITHM_ED25519,
};

#[derive(Clone, Debug)]
pub struct BrowserDelegationSigningKey {
  signing_key: SigningKey,
}

#[derive(Clone, Debug)]
pub struct Ed25519BrowserDelegationSigner {
  signing_key: BrowserDelegationSigningKey,
}

impl BrowserDelegationSigningKey {
  pub fn from_ed25519_private_key_bytes(bytes: &[u8]) -> MResult<Self> {
    let bytes: [u8; 32] = bytes.try_into().map_err(|_| signature_error(
      "ed25519 private key must contain exactly 32 bytes",
    ))?;
    Ok(Self { signing_key: SigningKey::from_bytes(&bytes) })
  }

  pub fn public_key_bytes(&self) -> Vec<u8> {
    self.signing_key.verifying_key().to_bytes().to_vec()
  }
}

impl Ed25519BrowserDelegationSigner {
  pub fn new(signing_key: BrowserDelegationSigningKey) -> Self {
    Self { signing_key }
  }

  pub fn sign(&self, payload: &[u8]) -> Vec<u8> {
    self.signing_key.signing_key.sign(payload).to_bytes().to_vec()
  }
}

pub fn sign_browser_delegation(
  header: BrowserDelegationHeader,
  host_config: BrowserHostConfig,
  signing_key: &BrowserDelegationSigningKey,
) -> MResult<BrowserDelegationEnvelope> {
  let mut envelope = BrowserDelegationEnvelope::unsigned(header, host_config);
  envelope.validate_unsigned()?;
  let payload = envelope.signing_payload()?;
  let signer = Ed25519BrowserDelegationSigner::new(signing_key.clone());
  envelope.signature = Some(BrowserDelegationSignature { bytes: signer.sign(&payload) });
  Ok(envelope)
}

pub fn verify_browser_delegation(
  envelope: &BrowserDelegationEnvelope,
  request: BrowserDelegationVerificationRequest,
) -> MResult<VerifiedBrowserDelegation> {
  envelope.validate_unsigned()?;

  let signature = envelope
    .signature
    .as_ref()
    .ok_or_else(|| signature_error("missing signature"))?;

  if envelope.header.audience != request.expected_audience {
    return Err(MechError::new(
      BrowserDelegationWrongAudienceError {
        expected: request.expected_audience,
        actual: envelope.header.audience.clone(),
      },
      None,
    ));
  }

  let skew = request.max_clock_skew_ms;
  if request.now_ms.saturating_add(skew) < envelope.header.issued_at_ms {
    return Err(MechError::new(
      BrowserDelegationExpiredError {
        now_ms: request.now_ms,
        issued_at_ms: envelope.header.issued_at_ms,
        expires_at_ms: envelope.header.expires_at_ms,
      },
      None,
    ));
  }
  if let Some(expires_at_ms) = envelope.header.expires_at_ms {
    if request.now_ms > expires_at_ms.saturating_add(skew) {
      return Err(MechError::new(
        BrowserDelegationExpiredError {
          now_ms: request.now_ms,
          issued_at_ms: envelope.header.issued_at_ms,
          expires_at_ms: envelope.header.expires_at_ms,
        },
        None,
      ));
    }
  }

  let key = request
    .trusted_keys
    .key(&envelope.header.issuer, &envelope.header.key_id)
    .ok_or_else(|| {
      MechError::new(
        BrowserDelegationKeyNotFoundError {
          issuer: envelope.header.issuer.clone(),
          key_id: envelope.header.key_id.clone(),
        },
        None,
      )
    })?;
  validate_public_key(key)?;

  let public_key: [u8; 32] = key.public_key.as_slice().try_into().map_err(|_| signature_error(
    "ed25519 public key must contain exactly 32 bytes",
  ))?;
  let verifying_key = VerifyingKey::from_bytes(&public_key)
    .map_err(|error| signature_error(format!("invalid ed25519 public key: {error}")))?;
  let signature_bytes: [u8; 64] = signature.bytes.as_slice().try_into().map_err(|_| signature_error(
    "ed25519 signature must contain exactly 64 bytes",
  ))?;
  let signature = Signature::from_bytes(&signature_bytes);
  let payload = envelope.signing_payload()?;
  verifying_key
    .verify(&payload, &signature)
    .map_err(|error| signature_error(format!("ed25519 verification failed: {error}")))?;

  let runtime_config = envelope.host_config.into_runtime_config()?;
  let browser_authority = envelope.host_config.into_browser_authority()?;

  Ok(VerifiedBrowserDelegation {
    issuer: envelope.header.issuer.clone(),
    subject: envelope.header.subject.clone(),
    audience: envelope.header.audience.clone(),
    key_id: envelope.header.key_id.clone(),
    runtime_config,
    browser_authority,
    host_config: envelope.host_config.clone(),
  })
}

fn validate_public_key(key: &BrowserDelegationPublicKey) -> MResult<()> {
  if key.algorithm != BROWSER_DELEGATION_ALGORITHM_ED25519 {
    return Err(signature_error(format!(
      "trusted key algorithm must be `{}`, got `{}`",
      BROWSER_DELEGATION_ALGORITHM_ED25519, key.algorithm,
    )));
  }
  if key.public_key.is_empty() {
    return Err(signature_error("trusted key publicKey must not be empty"));
  }
  Ok(())
}

fn signature_error(reason: impl Into<String>) -> MechError {
  MechError::new(BrowserDelegationSignatureError { reason: reason.into() }, None)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::browser_delegation::tests::{header, host_config};
  use crate::BrowserDelegationKeyStore;
  use mech_core::BrowserOperation;

  const PRIVATE_KEY: [u8; 32] = [
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
  ];

  fn signing_key() -> BrowserDelegationSigningKey {
    BrowserDelegationSigningKey::from_ed25519_private_key_bytes(&PRIVATE_KEY).unwrap()
  }

  fn key_store(signing_key: &BrowserDelegationSigningKey) -> BrowserDelegationKeyStore {
    BrowserDelegationKeyStore::new([BrowserDelegationPublicKey {
      issuer: "host://mech-cli".to_string(),
      key_id: "dev".to_string(),
      algorithm: BROWSER_DELEGATION_ALGORITHM_ED25519.to_string(),
      public_key: signing_key.public_key_bytes(),
    }])
  }

  fn request(signing_key: &BrowserDelegationSigningKey) -> BrowserDelegationVerificationRequest {
    BrowserDelegationVerificationRequest {
      now_ms: 2000,
      expected_audience: "browser://test".to_string(),
      trusted_keys: key_store(signing_key),
      max_clock_skew_ms: 0,
    }
  }

  fn signed_envelope() -> (BrowserDelegationSigningKey, BrowserDelegationEnvelope) {
    let key = signing_key();
    let envelope = sign_browser_delegation(header(), host_config(), &key).unwrap();
    (key, envelope)
  }

  #[test]
  fn valid_signed_envelope_verifies() {
    let (key, envelope) = signed_envelope();
    let verified = verify_browser_delegation(&envelope, request(&key)).unwrap();
    assert_eq!(verified.issuer, "host://mech-cli");
  }

  #[test]
  fn modified_host_config_fails_signature_verification() {
    let (key, mut envelope) = signed_envelope();
    envelope.host_config.browser.grants[0].allow.push("list".to_string());
    assert!(verify_browser_delegation(&envelope, request(&key)).is_err());
  }

  #[test]
  fn modified_audience_fails_wrong_audience_validation() {
    let (key, envelope) = signed_envelope();
    let mut request = request(&key);
    request.expected_audience = "browser://other".to_string();
    let error = format!("{:?}", verify_browser_delegation(&envelope, request).unwrap_err());
    assert!(error.contains("BrowserDelegationWrongAudienceError"));
  }

  #[test]
  fn wrong_trusted_key_fails_verification() {
    let (_, envelope) = signed_envelope();
    let wrong_key = BrowserDelegationSigningKey::from_ed25519_private_key_bytes(&[2; 32]).unwrap();
    assert!(verify_browser_delegation(&envelope, request(&wrong_key)).is_err());
  }

  #[test]
  fn missing_signature_fails_verification() {
    let key = signing_key();
    let envelope = BrowserDelegationEnvelope::unsigned(header(), host_config());
    assert!(verify_browser_delegation(&envelope, request(&key)).is_err());
  }

  #[test]
  fn expired_envelope_fails_verification() {
    let (key, envelope) = signed_envelope();
    let mut request = request(&key);
    request.now_ms = 20_000;
    assert!(verify_browser_delegation(&envelope, request).is_err());
  }

  #[test]
  fn not_yet_valid_envelope_fails_verification() {
    let key = signing_key();
    let mut header = header();
    header.issued_at_ms = 5000;
    header.expires_at_ms = Some(10_000);
    let envelope = sign_browser_delegation(header, host_config(), &key).unwrap();
    assert!(verify_browser_delegation(&envelope, request(&key)).is_err());
  }

  #[test]
  fn verified_envelope_converts_into_runtime_and_authority() {
    let (key, envelope) = signed_envelope();
    let verified = verify_browser_delegation(&envelope, request(&key)).unwrap();
    assert_eq!(verified.runtime_config.name, "demo");
    assert!(verified.browser_authority.allows_dom("#out", BrowserOperation::Write).is_ok());
  }

  #[test]
  fn verified_authority_enforces_denied_dom_write() {
    let (key, envelope) = signed_envelope();
    let verified = verify_browser_delegation(&envelope, request(&key)).unwrap();
    let error = verified
      .browser_authority
      .allows_dom("#source", BrowserOperation::Write)
      .unwrap_err();
    assert!(format!("{:?}", error).contains("OperationDenied"));
  }
}
