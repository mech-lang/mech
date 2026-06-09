use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use mech_core::{MResult, MechError};

use crate::{
  HostDelegationEnvelope, HostDelegationExpiredError, HostDelegationHeader,
  HostDelegationKeyNotFoundError, HostDelegationPayload, HostDelegationPublicKey,
  HostDelegationSignature, HostDelegationSignatureError, HostDelegationVerificationRequest,
  HostDelegationWrongAudienceError, VerifiedHostDelegation, HOST_DELEGATION_ALGORITHM_ED25519,
};

#[derive(Clone, Debug)]
pub struct HostDelegationSigningKey {
  signing_key: SigningKey,
}

#[derive(Clone, Debug)]
pub struct Ed25519HostDelegationSigner {
  signing_key: HostDelegationSigningKey,
}

impl HostDelegationSigningKey {
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

impl Ed25519HostDelegationSigner {
  pub fn new(signing_key: HostDelegationSigningKey) -> Self {
    Self { signing_key }
  }

  pub fn sign(&self, payload: &[u8]) -> Vec<u8> {
    self.signing_key.signing_key.sign(payload).to_bytes().to_vec()
  }
}

pub fn sign_host_delegation<P: HostDelegationPayload>(
  header: HostDelegationHeader,
  payload: P,
  signing_key: &HostDelegationSigningKey,
) -> MResult<HostDelegationEnvelope<P>> {
  let mut envelope = HostDelegationEnvelope::unsigned(header, payload);
  envelope.validate_unsigned()?;
  let payload = envelope.signing_payload()?;
  let signer = Ed25519HostDelegationSigner::new(signing_key.clone());
  envelope.signature = Some(HostDelegationSignature { bytes: signer.sign(&payload) });
  Ok(envelope)
}

pub fn verify_host_delegation<P: HostDelegationPayload>(
  envelope: &HostDelegationEnvelope<P>,
  request: HostDelegationVerificationRequest,
) -> MResult<VerifiedHostDelegation<P, P::Authority>> {
  let authority = envelope.validate_unsigned()?;

  let signature = envelope
    .signature
    .as_ref()
    .ok_or_else(|| signature_error("missing signature"))?;

  if envelope.header.audience != request.expected_audience {
    return Err(MechError::new(
      HostDelegationWrongAudienceError {
        expected: request.expected_audience,
        actual: envelope.header.audience.clone(),
      },
      None,
    ));
  }

  let skew = request.max_clock_skew_ms;
  if request.now_ms.saturating_add(skew) < envelope.header.issued_at_ms {
    return Err(MechError::new(
      HostDelegationExpiredError {
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
        HostDelegationExpiredError {
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
        HostDelegationKeyNotFoundError {
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

  Ok(VerifiedHostDelegation {
    issuer: envelope.header.issuer.clone(),
    subject: envelope.header.subject.clone(),
    audience: envelope.header.audience.clone(),
    key_id: envelope.header.key_id.clone(),
    payload: envelope.payload.clone(),
    authority,
  })
}

fn validate_public_key(key: &HostDelegationPublicKey) -> MResult<()> {
  if key.algorithm != HOST_DELEGATION_ALGORITHM_ED25519 {
    return Err(signature_error(format!(
      "trusted key algorithm must be `{}`, got `{}`",
      HOST_DELEGATION_ALGORITHM_ED25519, key.algorithm,
    )));
  }
  if key.public_key.is_empty() {
    return Err(signature_error("trusted key publicKey must not be empty"));
  }
  Ok(())
}

fn signature_error(reason: impl Into<String>) -> MechError {
  MechError::new(HostDelegationSignatureError { reason: reason.into() }, None)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::host_delegation::tests::{header, test_payload, TestPayload};
  use crate::HostDelegationKeyStore;

  const PRIVATE_KEY: [u8; 32] = [
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
  ];

  fn signing_key() -> HostDelegationSigningKey {
    HostDelegationSigningKey::from_ed25519_private_key_bytes(&PRIVATE_KEY).unwrap()
  }

  fn key_store(signing_key: &HostDelegationSigningKey) -> HostDelegationKeyStore {
    HostDelegationKeyStore::new([HostDelegationPublicKey {
      issuer: "host://mech-cli".to_string(),
      key_id: "dev".to_string(),
      algorithm: HOST_DELEGATION_ALGORITHM_ED25519.to_string(),
      public_key: signing_key.public_key_bytes(),
    }])
  }

  fn request(signing_key: &HostDelegationSigningKey) -> HostDelegationVerificationRequest {
    HostDelegationVerificationRequest {
      now_ms: 2000,
      expected_audience: "browser://test".to_string(),
      trusted_keys: key_store(signing_key),
      max_clock_skew_ms: 0,
    }
  }

  fn signed_test_envelope() -> (HostDelegationSigningKey, HostDelegationEnvelope<TestPayload>) {
    let key = signing_key();
    let envelope = sign_host_delegation(header(), test_payload(), &key).unwrap();
    (key, envelope)
  }

  #[test]
  fn wrong_payload_kind_cannot_reuse_signature() {
    let (key, mut envelope) = signed_test_envelope();
    envelope.payload.kind = "other";
    assert!(verify_host_delegation(&envelope, request(&key)).is_err());
  }

  #[test]
  fn valid_generic_envelope_verifies() {
    let (key, envelope) = signed_test_envelope();
    let verified = verify_host_delegation(&envelope, request(&key)).unwrap();
    assert_eq!(verified.authority, "payload");
  }

  #[test]
  fn wrong_audience_fails_validation() {
    let (key, envelope) = signed_test_envelope();
    let mut request = request(&key);
    request.expected_audience = "browser://other".to_string();
    let error = format!("{:?}", verify_host_delegation(&envelope, request).unwrap_err());
    assert!(error.contains("HostDelegationWrongAudienceError"));
  }

  #[test]
  fn wrong_trusted_key_fails_verification() {
    let (_, envelope) = signed_test_envelope();
    let wrong_key = HostDelegationSigningKey::from_ed25519_private_key_bytes(&[2; 32]).unwrap();
    assert!(verify_host_delegation(&envelope, request(&wrong_key)).is_err());
  }

  #[test]
  fn missing_signature_fails_verification() {
    let key = signing_key();
    let envelope = HostDelegationEnvelope::unsigned(header(), test_payload());
    assert!(verify_host_delegation(&envelope, request(&key)).is_err());
  }

  #[test]
  fn expired_envelope_fails_verification() {
    let (key, envelope) = signed_test_envelope();
    let mut request = request(&key);
    request.now_ms = 20_000;
    assert!(verify_host_delegation(&envelope, request).is_err());
  }

  #[test]
  fn not_yet_valid_envelope_fails_verification() {
    let key = signing_key();
    let mut header = header();
    header.issued_at_ms = 5000;
    header.expires_at_ms = Some(10_000);
    let envelope = sign_host_delegation(header, test_payload(), &key).unwrap();
    assert!(verify_host_delegation(&envelope, request(&key)).is_err());
  }
}
