use std::io::{Error, ErrorKind};

use mech_core::*;
use mech_runtime::BrowserHostConfig;

#[cfg(feature = "browser_delegation_signing")]
use base64::Engine;
#[cfg(feature = "browser_delegation_signing")]
use mech_runtime::{
  sign_browser_delegation, BrowserDelegationHeader, BrowserDelegationPublicKey,
  BrowserDelegationSigningKey, BROWSER_DELEGATION_ALGORITHM_ED25519,
};
#[cfg(feature = "browser_delegation_signing")]
use serde::Deserialize;

#[derive(Clone, Debug)]
pub enum BrowserHostConfigInjection {
  Unsigned(BrowserHostConfig),
  #[cfg(feature = "browser_delegation_signing")]
  Signed {
    envelope: mech_runtime::BrowserDelegationEnvelope,
    trusted_keys: Vec<BrowserDelegationPublicKey>,
  },
}

#[cfg(feature = "browser_delegation_signing")]
#[derive(Clone, Debug)]
pub struct BrowserDelegationSigningOptions {
  pub private_key_path: std::path::PathBuf,
  pub public_key_path: std::path::PathBuf,
  pub key_id: String,
  pub issuer: String,
  pub subject: String,
  pub audience: String,
  pub expires_ms: Option<u64>,
}

#[cfg(feature = "browser_delegation_signing")]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BrowserDelegationPrivateKeyFile {
  algorithm: String,
  key_id: String,
  private_key: String,
}

#[cfg(feature = "browser_delegation_signing")]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BrowserDelegationPublicKeyFile {
  issuer: String,
  algorithm: String,
  key_id: String,
  public_key: String,
}

pub fn browser_host_config_script(host_config: &BrowserHostConfig) -> MResult<String> {
  browser_host_config_injection_script(&BrowserHostConfigInjection::Unsigned(host_config.clone()))
}

pub fn browser_host_config_injection_script(injection: &BrowserHostConfigInjection) -> MResult<String> {
  match injection {
    BrowserHostConfigInjection::Unsigned(host_config) => {
      let json = json_for_script(host_config)?;
      Ok(format!("<script>window.__MECH_HOST_CONFIG = {json};</script>"))
    }
    #[cfg(feature = "browser_delegation_signing")]
    BrowserHostConfigInjection::Signed { envelope, trusted_keys } => {
      let envelope_json = json_for_script(envelope)?;
      let trusted_keys_json = json_for_script(&trusted_keys_for_js(trusted_keys))?;
      Ok(format!(
        "<script>window.__MECH_HOST_CONFIG = {envelope_json};window.__MECH_TRUSTED_BROWSER_KEYS = {trusted_keys_json};</script>",
      ))
    }
  }
}

pub fn inject_browser_host_config_script(
  html: &str,
  host_config: &BrowserHostConfig,
) -> MResult<String> {
  inject_browser_host_config_injection_script(
    html,
    &BrowserHostConfigInjection::Unsigned(host_config.clone()),
  )
}

pub fn inject_browser_host_config_injection_script(
  html: &str,
  injection: &BrowserHostConfigInjection,
) -> MResult<String> {
  let script = browser_host_config_injection_script(injection)?;
  if let Some(index) = html.find("</head>") {
    let mut out = html.to_string();
    out.insert_str(index, &script);
    Ok(out)
  } else {
    Ok(format!("{script}\n{html}"))
  }
}

#[cfg(feature = "browser_delegation_signing")]
pub fn signed_browser_host_config_injection(
  host_config: BrowserHostConfig,
  options: &BrowserDelegationSigningOptions,
  now_ms: u64,
) -> MResult<BrowserHostConfigInjection> {
  let private_key = read_private_key_file(options)?;
  let public_key = read_public_key_file(options)?;
  let header = BrowserDelegationHeader {
    issuer: options.issuer.clone(),
    subject: options.subject.clone(),
    audience: options.audience.clone(),
    key_id: options.key_id.clone(),
    algorithm: BROWSER_DELEGATION_ALGORITHM_ED25519.to_string(),
    issued_at_ms: now_ms,
    expires_at_ms: options.expires_ms.map(|expires_ms| now_ms.saturating_add(expires_ms)),
    nonce: None,
  };
  let envelope = sign_browser_delegation(header, host_config, &private_key)?;
  Ok(BrowserHostConfigInjection::Signed {
    envelope,
    trusted_keys: vec![public_key],
  })
}

#[cfg(feature = "browser_delegation_signing")]
fn read_private_key_file(options: &BrowserDelegationSigningOptions) -> MResult<BrowserDelegationSigningKey> {
  let text = std::fs::read_to_string(&options.private_key_path)?;
  let file: BrowserDelegationPrivateKeyFile = serde_json::from_str(&text)
    .map_err(|error| Error::new(ErrorKind::InvalidData, error.to_string()))?;
  if file.algorithm != BROWSER_DELEGATION_ALGORITHM_ED25519 {
    return Err(Error::new(ErrorKind::InvalidData, "private key algorithm must be ed25519").into());
  }
  if file.key_id != options.key_id {
    return Err(Error::new(ErrorKind::InvalidData, "private key keyId does not match --browser-delegation-key-id").into());
  }
  let bytes = base64::engine::general_purpose::STANDARD
    .decode(file.private_key.as_bytes())
    .map_err(|error| Error::new(ErrorKind::InvalidData, error.to_string()))?;
  BrowserDelegationSigningKey::from_ed25519_private_key_bytes(&bytes)
}

#[cfg(feature = "browser_delegation_signing")]
fn read_public_key_file(options: &BrowserDelegationSigningOptions) -> MResult<BrowserDelegationPublicKey> {
  let text = std::fs::read_to_string(&options.public_key_path)?;
  let file: BrowserDelegationPublicKeyFile = serde_json::from_str(&text)
    .map_err(|error| Error::new(ErrorKind::InvalidData, error.to_string()))?;
  if file.algorithm != BROWSER_DELEGATION_ALGORITHM_ED25519 {
    return Err(Error::new(ErrorKind::InvalidData, "public key algorithm must be ed25519").into());
  }
  if file.key_id != options.key_id {
    return Err(Error::new(ErrorKind::InvalidData, "public key keyId does not match --browser-delegation-key-id").into());
  }
  if file.issuer != options.issuer {
    return Err(Error::new(ErrorKind::InvalidData, "public key issuer does not match --browser-delegation-issuer").into());
  }
  let public_key = base64::engine::general_purpose::STANDARD
    .decode(file.public_key.as_bytes())
    .map_err(|error| Error::new(ErrorKind::InvalidData, error.to_string()))?;
  Ok(BrowserDelegationPublicKey {
    issuer: file.issuer,
    key_id: file.key_id,
    algorithm: file.algorithm,
    public_key,
  })
}

#[cfg(feature = "browser_delegation_signing")]
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct JsTrustedBrowserKey<'a> {
  issuer: &'a str,
  key_id: &'a str,
  algorithm: &'a str,
  public_key: String,
}

#[cfg(feature = "browser_delegation_signing")]
fn trusted_keys_for_js(keys: &[BrowserDelegationPublicKey]) -> Vec<JsTrustedBrowserKey<'_>> {
  keys
    .iter()
    .map(|key| JsTrustedBrowserKey {
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
      runtime: mech_runtime::BrowserHostRuntimeConfig::from(&RuntimeConfig::default()),
      browser: mech_runtime::BrowserHostBrowserConfig {
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
