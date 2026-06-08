use std::io::{Error, ErrorKind};

use mech_core::*;
use mech_runtime::BrowserHostConfig;

pub fn browser_host_config_script(host_config: &BrowserHostConfig) -> MResult<String> {
  let json = serde_json::to_string(host_config)
    .map_err(|error| Error::new(ErrorKind::InvalidData, error.to_string()))?
    .replace('<', "\\u003c");
  Ok(format!("<script>window.__MECH_HOST_CONFIG = {json};</script>"))
}

pub fn inject_browser_host_config_script(
  html: &str,
  host_config: &BrowserHostConfig,
) -> MResult<String> {
  let script = browser_host_config_script(host_config)?;
  if let Some(index) = html.find("</head>") {
    let mut out = html.to_string();
    out.insert_str(index, &script);
    Ok(out)
  } else {
    Ok(format!("{script}\n{html}"))
  }
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
