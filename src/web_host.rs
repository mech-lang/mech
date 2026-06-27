use std::io::{Error, ErrorKind};

use mech_core::*;
use mech_host_browser::{BrowserHostConfig, BrowserRuntimeInjectionConfig};

#[cfg(feature = "host_delegation_signing")]
use base64::Engine;
#[cfg(feature = "host_delegation_signing")]
use mech_host_browser::{sign_browser_host_delegation, BrowserHostDelegationEnvelope};
#[cfg(feature = "host_delegation_signing")]
use mech_runtime::{
  HostDelegationHeader, HostDelegationPublicKey, HostDelegationSigningKey,
  HOST_DELEGATION_ALGORITHM_ED25519,
};
#[cfg(feature = "host_delegation_signing")]
use serde::Deserialize;

#[derive(Clone, Debug)]
pub enum HostAuthorityInjection {
  BrowserUnsigned(BrowserRuntimeInjectionConfig),
  #[cfg(feature = "host_delegation_signing")]
  BrowserSigned {
    envelope: BrowserHostDelegationEnvelope,
    trusted_keys: Vec<HostDelegationPublicKey>,
    audience: String,
  },
}

#[cfg(feature = "host_delegation_signing")]
#[derive(Clone, Debug)]
pub struct HostDelegationSigningOptions {
  pub private_key_path: std::path::PathBuf,
  pub public_key_path: std::path::PathBuf,
  pub key_id: String,
  pub issuer: String,
  pub subject: String,
  pub audience: String,
  pub expires_ms: Option<u64>,
}

#[cfg(feature = "host_delegation_signing")]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct HostDelegationPrivateKeyFile {
  algorithm: String,
  key_id: String,
  private_key: String,
}

#[cfg(feature = "host_delegation_signing")]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct HostDelegationPublicKeyFile {
  issuer: String,
  algorithm: String,
  key_id: String,
  public_key: String,
}

pub fn browser_host_config_script(host_config: &BrowserHostConfig) -> MResult<String> {
  let json = json_for_script(host_config)?;
  Ok(format!("<script>window.__MECH_HOST_CONFIG = {json};</script>"))
}

pub fn host_authority_injection_script(injection: &HostAuthorityInjection) -> MResult<String> {
  match injection {
    HostAuthorityInjection::BrowserUnsigned(host_config) => {
      let json = json_for_script(host_config)?;
      Ok(format!("<script>window.__MECH_HOST_CONFIG = {json};</script>"))
    }
    #[cfg(feature = "host_delegation_signing")]
    HostAuthorityInjection::BrowserSigned { envelope, trusted_keys, audience } => {
      let envelope_json = json_for_script(envelope)?;
      let trusted_keys_json = json_for_script(&trusted_keys_for_js(trusted_keys))?;
      let audience_json = json_for_script(audience)?;
      Ok(format!(
        "<script>window.__MECH_HOST_CONFIG = {envelope_json};window.__MECH_TRUSTED_HOST_KEYS = {trusted_keys_json};window.__MECH_HOST_DELEGATION_AUDIENCE = {audience_json};</script>",
      ))
    }
  }
}

pub fn inject_browser_host_config_script(
  html: &str,
  host_config: &BrowserRuntimeInjectionConfig,
) -> MResult<String> {
  inject_host_authority_injection_script(
    html,
    &HostAuthorityInjection::BrowserUnsigned(host_config.clone()),
  )
}

pub fn inject_host_authority_injection_script(
  html: &str,
  injection: &HostAuthorityInjection,
) -> MResult<String> {
  let script = host_authority_injection_script(injection)?;
  if let Some(index) = html.find("</head>") {
    let mut out = html.to_string();
    out.insert_str(index, &script);
    Ok(out)
  } else {
    Ok(format!("{script}\n{html}"))
  }
}

#[cfg(feature = "host_delegation_signing")]
pub fn signed_browser_host_config_injection(
  host_config: BrowserHostConfig,
  options: &HostDelegationSigningOptions,
  now_ms: u64,
) -> MResult<HostAuthorityInjection> {
  let private_key = read_private_key_file(options)?;
  let public_key = read_public_key_file(options)?;
  let header = HostDelegationHeader {
    issuer: options.issuer.clone(),
    subject: options.subject.clone(),
    audience: options.audience.clone(),
    key_id: options.key_id.clone(),
    algorithm: HOST_DELEGATION_ALGORITHM_ED25519.to_string(),
    issued_at_ms: now_ms,
    expires_at_ms: options.expires_ms.map(|expires_ms| now_ms.saturating_add(expires_ms)),
    nonce: None,
  };
  let envelope = sign_browser_host_delegation(header, host_config, &private_key)?;
  Ok(HostAuthorityInjection::BrowserSigned {
    envelope,
    trusted_keys: vec![public_key],
    audience: options.audience.clone(),
  })
}

#[cfg(feature = "host_delegation_signing")]
fn read_private_key_file(options: &HostDelegationSigningOptions) -> MResult<HostDelegationSigningKey> {
  let text = std::fs::read_to_string(&options.private_key_path)?;
  let file: HostDelegationPrivateKeyFile = serde_json::from_str(&text)
    .map_err(|error| Error::new(ErrorKind::InvalidData, error.to_string()))?;
  if file.algorithm != HOST_DELEGATION_ALGORITHM_ED25519 {
    return Err(Error::new(ErrorKind::InvalidData, "private key algorithm must be ed25519").into());
  }
  if file.key_id != options.key_id {
    return Err(Error::new(ErrorKind::InvalidData, "private key keyId does not match --host-delegation-key-id").into());
  }
  let bytes = base64::engine::general_purpose::STANDARD
    .decode(file.private_key.as_bytes())
    .map_err(|error| Error::new(ErrorKind::InvalidData, error.to_string()))?;
  HostDelegationSigningKey::from_ed25519_private_key_bytes(&bytes)
}

#[cfg(feature = "host_delegation_signing")]
fn read_public_key_file(options: &HostDelegationSigningOptions) -> MResult<HostDelegationPublicKey> {
  let text = std::fs::read_to_string(&options.public_key_path)?;
  let file: HostDelegationPublicKeyFile = serde_json::from_str(&text)
    .map_err(|error| Error::new(ErrorKind::InvalidData, error.to_string()))?;
  if file.algorithm != HOST_DELEGATION_ALGORITHM_ED25519 {
    return Err(Error::new(ErrorKind::InvalidData, "public key algorithm must be ed25519").into());
  }
  if file.key_id != options.key_id {
    return Err(Error::new(ErrorKind::InvalidData, "public key keyId does not match --host-delegation-key-id").into());
  }
  if file.issuer != options.issuer {
    return Err(Error::new(ErrorKind::InvalidData, "public key issuer does not match --host-delegation-issuer").into());
  }
  let public_key = base64::engine::general_purpose::STANDARD
    .decode(file.public_key.as_bytes())
    .map_err(|error| Error::new(ErrorKind::InvalidData, error.to_string()))?;
  Ok(HostDelegationPublicKey {
    issuer: file.issuer,
    key_id: file.key_id,
    algorithm: file.algorithm,
    public_key,
  })
}

#[cfg(feature = "host_delegation_signing")]
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct JsTrustedHostKey<'a> {
  issuer: &'a str,
  key_id: &'a str,
  algorithm: &'a str,
  public_key: String,
}

#[cfg(feature = "host_delegation_signing")]
fn trusted_keys_for_js(keys: &[HostDelegationPublicKey]) -> Vec<JsTrustedHostKey<'_>> {
  keys
    .iter()
    .map(|key| JsTrustedHostKey {
      issuer: &key.issuer,
      key_id: &key.key_id,
      algorithm: &key.algorithm,
      public_key: base64::engine::general_purpose::STANDARD.encode(&key.public_key),
    })
    .collect()
}

fn json_for_script<T: serde::Serialize>(value: &T) -> MResult<String> {
  Ok(serde_json::to_string(value)
    .map_err(|error| Error::new(ErrorKind::InvalidData, error.to_string()))?
    .replace('<', "\\u003c"))
}

#[cfg(test)]
mod tests {
  use super::*;
  use mech_runtime::RuntimeConfig;

  fn empty_host_config() -> BrowserHostConfig {
    BrowserHostConfig {
      runtime: mech_host_browser::BrowserHostRuntimeConfig::from(&RuntimeConfig::default()),
      browser: mech_host_browser::BrowserHostBrowserConfig {
        grants: Vec::new(),
        dom_manifest: Vec::new(),
      },
    }
  }

  #[test]
  fn browser_host_config_script_uses_mech_host_config_global() {
    let script = browser_host_config_script(&empty_host_config()).unwrap();
    assert!(script.contains("window.__MECH_HOST_CONFIG ="));
  }

  #[test]
  fn browser_host_config_script_escapes_less_than() {
    let mut config = empty_host_config();
    config.runtime.name = "</script>".to_string();
    let script = browser_host_config_script(&config).unwrap();
    assert!(script.contains("\\u003c/script>"));
    assert!(!script.contains("</script>\""));
  }
}
