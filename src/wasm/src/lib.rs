#![allow(warnings)]

pub mod host;

use wasm_bindgen::prelude::*;
use mech_core::*;
use mech_syntax::*;
use mech_host_browser::{
  BrowserHostFactory, BrowserRuntimeInjectionConfig,
};
#[cfg(feature = "host_delegation_signing")]
use mech_host_browser::{verify_browser_host_delegation, BrowserHostDelegationEnvelope};
use mech_runtime::{
  ConfigProfileOptions, ConfigValue, HostInstanceConfig, MechConfigDocument, MechRuntime, RuntimeBuilder, RuntimeConfig, parse_config_document,
};
#[cfg(feature = "host_delegation_signing")]
use mech_runtime::{HostDelegationKeyStore, HostDelegationPublicKey, HostDelegationVerificationRequest};
use crate::host::{
  BrowserCapabilityRequest, BrowserDomScope, BrowserHost, BrowserHostError,
  BrowserNetworkScope, BrowserOperation, BrowserStorageBackend, BrowserStorageScope,
  WasmBrowserDomBackend,
};
use wasm_bindgen::JsCast;
use web_sys::{window, HtmlElement, HtmlInputElement, Node, Element, HashChangeEvent, HtmlTextAreaElement, Url};
use js_sys::decode_uri_component;
use std::collections::HashMap;
#[cfg(feature = "host-robot-arm")]
use std::io::{Error, ErrorKind};
use std::rc::Rc;
use std::cell::RefCell;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Arc;
use gloo_net::http::{Method, Request, RequestBuilder};
use wasm_bindgen_futures::spawn_local;
#[cfg(feature = "host_delegation_signing")]
use base64::Engine;
#[cfg(feature = "host_delegation_signing")]
use serde::Deserialize;

#[cfg(feature = "repl")]
pub mod repl;


#[cfg(feature = "repl")]
pub use crate::repl::*;

// This monstrosity lets us pass a references to WasmMech to callbacks and such.
// Using it is unsafe. But we trust that the WasmMech instance will be around
// for the lifetime of the website.
thread_local! {
  pub static CURRENT_MECH: RefCell<Option<*mut WasmMech>> = RefCell::new(None);
}

const MECH_ERROR_HTML_PREFIX: &str = "__MECH_ERROR_HTML__:";

#[macro_export]
macro_rules! log {
  ( $( $t:tt )* ) => {
    web_sys::console::log_1(&format!( $( $t )* ).into());
  }
}


fn js_error(message: impl Into<String>) -> JsValue {
  JsValue::from_str(&message.into())
}

fn browser_host_error_to_js(error: BrowserHostError) -> JsValue {
  js_error(error.to_string())
}

fn browser_origin_for_url(url: &str) -> Result<String, JsValue> {
  let parsed = web_sys::Url::new(url)
    .map_err(|error| js_error(format!("invalid URL `{url}`: {:?}", error)))?;

  let protocol = parsed.protocol();
  if protocol != "http:" && protocol != "https:" {
    return Err(js_error(format!(
      "network URLs must use http or https, got `{protocol}`"
    )));
  }

  let host = parsed.host();
  if host.is_empty() {
    return Err(js_error(format!("network URL `{url}` has no host")));
  }

  Ok(format!("{}//{}", protocol, host))
}

fn browser_storage_for_backend(backend: &str) -> Result<web_sys::Storage, JsValue> {
  let window = web_sys::window()
    .ok_or_else(|| js_error("global window does not exist"))?;

  match BrowserStorageBackend::parse(backend) {
    Some(BrowserStorageBackend::LocalStorage) => window
      .local_storage()
      .map_err(|error| js_error(format!("localStorage unavailable: {:?}", error)))?
      .ok_or_else(|| js_error("localStorage is not available")),
    Some(BrowserStorageBackend::SessionStorage) => window
      .session_storage()
      .map_err(|error| js_error(format!("sessionStorage unavailable: {:?}", error)))?
      .ok_or_else(|| js_error("sessionStorage is not available")),
    Some(BrowserStorageBackend::IndexedDb) => Err(js_error(
      "indexed-db storage backend is configured but not implemented yet",
    )),
    Some(BrowserStorageBackend::Opfs) => Err(js_error(
      "opfs storage backend is configured but not implemented yet",
    )),
    None => Err(js_error(format!("unknown storage backend `{backend}`"))),
  }
}


fn format_output_value_html(output: &Value) -> String {
  #[cfg(any(feature = "string", feature = "variable_define"))]
  if let Value::String(text) = output {
    if let Some(error_html) = text.borrow().strip_prefix(MECH_ERROR_HTML_PREFIX) {
      return format!(
        "<div class=\"mech-output-kind\">Error</div><div class=\"mech-output-value\">{}</div>",
        error_html
      );
    }
  }
  let kind_str = html_escape(&format!("{}",output.kind()));
  format!(
    "<div class=\"mech-output-kind\">{}</div><div class=\"mech-output-value\">{}</div>",
    kind_str,
    output.to_html()
  )
}

#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
  //let mut wasm_mech = WasmMech::new();
  //wasm_mech.init();
  //wasm_mech.run_program("1 + 1");
  Ok(())
}



struct WasmRuntimeParts {
  runtime: MechRuntime,
  browser_host: BrowserHost,
  runtime_injection: BrowserRuntimeInjectionConfig,
}

fn wasm_parts_from_config_document(
  document: Option<&MechConfigDocument>,
) -> MResult<WasmRuntimeParts> {
  let injected = match document {
    Some(document) => {
      let config = RuntimeConfig::default().apply_patch(&document.runtime)?;
      wasm_runtime_injection_config_from_document(document, &config)?
    }
    None => default_browser_runtime_injection_config(),
  };
  wasm_parts_from_runtime_injection_config_result(injected)
}

fn default_browser_runtime_injection_config() -> BrowserRuntimeInjectionConfig {
  BrowserRuntimeInjectionConfig {
    runtime: mech_host_browser::BrowserHostRuntimeConfig::from(&RuntimeConfig::default()),
    hosts: vec![HostInstanceConfig {
      name: "browser".to_string(),
      provider: "browser".to_string(),
      settings: ConfigValue::Map(Default::default()),
    }],
    run_grants: Vec::new(),
  }
}

fn wasm_runtime_injection_config_from_document(
  document: &MechConfigDocument,
  runtime_config: &RuntimeConfig,
) -> MResult<BrowserRuntimeInjectionConfig> {
  let mut config =
    BrowserRuntimeInjectionConfig::from_document_and_runtime(document, runtime_config)?;
  append_feature_enabled_injected_hosts(document, &mut config)?;
  Ok(config)
}

fn append_feature_enabled_injected_hosts(
  document: &MechConfigDocument,
  config: &mut BrowserRuntimeInjectionConfig,
) -> MResult<()> {
  #[cfg(feature = "host-robot-arm")]
  append_robot_arm_injected_hosts(document, config)?;

  Ok(())
}

#[cfg(feature = "host-robot-arm")]
fn append_robot_arm_injected_hosts(
  document: &MechConfigDocument,
  config: &mut BrowserRuntimeInjectionConfig,
) -> MResult<()> {
  use mech_runtime::{materialize_host_manifest, parse_host_context_target, RuntimeHostFactory};

  let factory = mech_host_robot_arm::RobotArmHostFactory::new()?;
  for host in document.hosts.iter().filter(|host| host.provider == "robot-arm") {
    factory.validate_settings(&host.name, &host.settings)?;
    let interface = materialize_host_manifest(&host.name, factory.manifest())?;

    if let Some(run) = &document.run {
      for grant in &run.grants {
        let (instance, context_name) = parse_host_context_target(&grant.target)?;
        if instance != host.name {
          continue;
        }
        let Some(context) = interface.contexts.iter().find(|context| context.name == context_name) else {
          return Err(Error::new(ErrorKind::InvalidInput, format!(
            "host instance `{}` provider `{}` does not expose context `{}`",
            host.name,
            host.provider,
            context_name,
          )).into());
        };
        for operation in &grant.operations {
          if !context.operations.iter().any(|allowed| allowed == operation) {
            return Err(Error::new(ErrorKind::InvalidInput, format!(
              "host context `{}` does not expose operation `{}`",
              grant.target,
              operation,
            )).into());
          }
        }
        config.run_grants.push(grant.clone());
      }
    }

    config.hosts.push(host.clone());
  }
  Ok(())
}

#[cfg(feature = "host_delegation_signing")]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct JsTrustedHostKey {
  issuer: String,
  key_id: String,
  algorithm: String,
  public_key: String,
}

#[cfg(feature = "host_delegation_signing")]
fn trusted_host_key_store_from_js(value: JsValue) -> Result<HostDelegationKeyStore, JsValue> {
  let keys: Vec<JsTrustedHostKey> = serde_wasm_bindgen::from_value(value)
    .map_err(|error| js_error(format!("failed to deserialize trusted host keys: {error}")))?;
  let mut decoded = Vec::with_capacity(keys.len());
  for key in keys {
    let public_key = base64::engine::general_purpose::STANDARD
      .decode(key.public_key.as_bytes())
      .map_err(|error| js_error(format!("failed to decode trusted host key `{}`: {error}", key.key_id)))?;
    decoded.push(HostDelegationPublicKey {
      issuer: key.issuer,
      key_id: key.key_id,
      algorithm: key.algorithm,
      public_key,
    });
  }
  Ok(HostDelegationKeyStore::new(decoded))
}

#[cfg(feature = "host_delegation_signing")]
fn wasm_parts_from_delegated_host_config(
  config: JsValue,
  trusted_keys: JsValue,
  expected_audience: String,
) -> Result<WasmRuntimeParts, JsValue> {
  let envelope: BrowserHostDelegationEnvelope = serde_wasm_bindgen::from_value(config)
    .map_err(|error| js_error(format!("failed to deserialize host delegation envelope: {error}")))?;
  let trusted_keys = trusted_host_key_store_from_js(trusted_keys)?;
  let now_ms = js_sys::Date::now() as u64;
  let request = HostDelegationVerificationRequest {
    now_ms,
    expected_audience,
    trusted_keys,
    max_clock_skew_ms: 60_000,
  };
  let verified = verify_browser_host_delegation(&envelope, request)
    .map_err(|error| js_error(format!("host delegation verification failed: {error:?}")))?;
  wasm_parts_from_runtime_injection_config(verified.authority.runtime_injection)
}

fn wasm_parts_from_host_config(config: JsValue) -> Result<WasmRuntimeParts, JsValue> {
  let injected: BrowserRuntimeInjectionConfig = serde_wasm_bindgen::from_value(config)
    .map_err(|error| js_error(format!("failed to deserialize host config: {error}")))?;
  wasm_parts_from_runtime_injection_config(injected)
}

fn wasm_parts_from_runtime_injection_config(
  injected: BrowserRuntimeInjectionConfig,
) -> Result<WasmRuntimeParts, JsValue> {
  wasm_parts_from_runtime_injection_config_result(injected)
    .map_err(|error| js_error(format!("invalid host config: {error:?}")))
}

fn wasm_parts_from_runtime_injection_config_result(
  injected: BrowserRuntimeInjectionConfig,
) -> MResult<WasmRuntimeParts> {
  let runtime_config = injected.into_runtime_config()?;
  let mut builder = RuntimeBuilder::new()
    .config(runtime_config)
    .host_factory(Box::new(BrowserHostFactory::new(WasmBrowserDomBackend::new())?))?;
  #[cfg(feature = "host-robot-arm")]
  {
    builder = builder.host_factory(Box::new(mech_host_robot_arm::RobotArmHostFactory::new()?))?;
  }
  let mut saw_default_browser_instance = false;
  for host in &injected.hosts {
    if host.provider == "browser" {
      if host.name == "browser" {
        saw_default_browser_instance = true;
      }
    }
    builder = builder.host_instance(host.clone());
  }
  if !saw_default_browser_instance {
    builder = builder.host_instance(HostInstanceConfig {
      name: "browser".to_string(),
      provider: "browser".to_string(),
      settings: ConfigValue::Map(Default::default()),
    });
  }
  for grant in &injected.run_grants {
    builder = builder.run_resource_grant(grant.clone());
  }
  let mut runtime = builder.build()?;
  runtime.bind_resource_root("browser", "browser://dom/")?;
  let authority = injected
    .browser_host_config()
    .and_then(|host_config| host_config.into_browser_authority())?;
  Ok(WasmRuntimeParts {
    runtime,
    browser_host: BrowserHost::new(authority),
    runtime_injection: injected,
  })
}

#[wasm_bindgen]
pub struct WasmMech {
  runtime: MechRuntime,
  browser_host: BrowserHost,
  runtime_injection: BrowserRuntimeInjectionConfig,
  repl_history: Vec<String>,
  repl_history_index: Option<usize>,
  repl_id: Option<String>,
}

#[wasm_bindgen]
impl WasmMech {

  #[wasm_bindgen(constructor)]
  pub fn new() -> Self {
    Self::with_default_runtime()
  }

  #[wasm_bindgen(js_name = "fromConfig")]
  pub fn from_config(source: &str) -> Result<WasmMech, JsValue> {
    Self::try_from_config("wasm://mech.mcfg", source)
      .map_err(|error| js_error(format!("{error:?}")))
  }

  #[wasm_bindgen(js_name = "fromHostConfig")]
  pub fn from_host_config() -> Result<WasmMech, JsValue> {
    let config = js_sys::Reflect::get(&js_sys::global(), &JsValue::from_str("__MECH_HOST_CONFIG"))
      .map_err(|error| js_error(format!("failed to read host config: {error:?}")))?;
    if config.is_undefined() || config.is_null() {
      return Err(js_error("host config was not provided by mech serve"));
    }
    let parts = wasm_parts_from_host_config(config)?;
    Ok(Self {
      runtime: parts.runtime,
      browser_host: parts.browser_host,
      runtime_injection: parts.runtime_injection,
      repl_history: Vec::new(),
      repl_history_index: None,
      repl_id: None,
    })
  }


  #[cfg(feature = "host_delegation_signing")]
  #[wasm_bindgen(js_name = "fromDelegatedHostConfig")]
  pub fn from_delegated_host_config(expected_audience: Option<String>) -> Result<WasmMech, JsValue> {
    let config = js_sys::Reflect::get(&js_sys::global(), &JsValue::from_str("__MECH_HOST_CONFIG"))
      .map_err(|error| js_error(format!("failed to read delegated host config: {error:?}")))?;
    if config.is_undefined() || config.is_null() {
      return Err(js_error("delegated host config was not provided by mech serve"));
    }
    let trusted_keys = js_sys::Reflect::get(&js_sys::global(), &JsValue::from_str("__MECH_TRUSTED_HOST_KEYS"))
      .map_err(|error| js_error(format!("failed to read trusted host keys: {error:?}")))?;
    if trusted_keys.is_undefined() || trusted_keys.is_null() {
      return Err(js_error("trusted host keys were not provided for delegated host config"));
    }
    let expected_audience = match expected_audience {
      Some(value) if !value.is_empty() => value,
      _ => {
        let value = js_sys::Reflect::get(&js_sys::global(), &JsValue::from_str("__MECH_HOST_DELEGATION_AUDIENCE"))
          .map_err(|error| js_error(format!("failed to read host delegation audience: {error:?}")))?;
        value
          .as_string()
          .filter(|value| !value.is_empty())
          .ok_or_else(|| js_error("host delegation audience was not provided"))?
      }
    };
    let parts = wasm_parts_from_delegated_host_config(config, trusted_keys, expected_audience)?;
    Ok(Self {
      runtime: parts.runtime,
      browser_host: parts.browser_host,
      runtime_injection: parts.runtime_injection,
      repl_history: Vec::new(),
      repl_history_index: None,
      repl_id: None,
    })
  }

  fn try_from_config(
    source_name: &str,
    source: &str,
  ) -> MResult<Self> {
    let document = parse_config_document(
      source_name,
      source,
      ConfigProfileOptions::default(),
    )?;
    let parts = wasm_parts_from_config_document(Some(&document))?;

    Ok(Self {
      runtime: parts.runtime,
      browser_host: parts.browser_host,
      runtime_injection: parts.runtime_injection,
      repl_history: Vec::new(),
      repl_history_index: None,
      repl_id: None,
    })
  }

  fn with_default_runtime() -> Self {
    let parts = wasm_parts_from_config_document(None)
      .expect("default wasm runtime config should be valid");

    Self {
      runtime: parts.runtime,
      browser_host: parts.browser_host,
      runtime_injection: parts.runtime_injection,
      repl_history: Vec::new(),
      repl_history_index: None,
      repl_id: None,
    }
  }

  #[wasm_bindgen]
  pub fn out_string(&self) -> String {
    self.runtime.out_string()
  }

  #[wasm_bindgen]
  pub fn clear(&mut self) {
    let parts = wasm_parts_from_runtime_injection_config_result(self.runtime_injection.clone())
      .expect("failed to reset wasm runtime from injected host config");
    self.runtime = parts.runtime;
    self.browser_host = parts.browser_host;
    self.runtime_injection = parts.runtime_injection;
  }

  #[wasm_bindgen(js_name = "readDomText")]
  pub fn read_dom_text(&self, selector: &str) -> Result<String, JsValue> {
    self
      .check_dom(selector, BrowserOperation::Read)
      .map_err(browser_host_error_to_js)?;

    let window = web_sys::window()
      .ok_or_else(|| js_error("global window does not exist"))?;
    let document = window
      .document()
      .ok_or_else(|| js_error("document is not available"))?;

    let element = document
      .query_selector(selector)
      .map_err(|error| js_error(format!("failed to query selector `{selector}`: {:?}", error)))?
      .ok_or_else(|| js_error(format!("DOM selector `{selector}` did not match an element")))?;

    Ok(element.text_content().unwrap_or_default())
  }

  #[wasm_bindgen(js_name = "writeDomText")]
  pub fn write_dom_text(&self, selector: &str, text: &str) -> Result<(), JsValue> {
    self
      .check_dom(selector, BrowserOperation::Write)
      .map_err(browser_host_error_to_js)?;

    let window = web_sys::window()
      .ok_or_else(|| js_error("global window does not exist"))?;
    let document = window
      .document()
      .ok_or_else(|| js_error("document is not available"))?;

    let element = document
      .query_selector(selector)
      .map_err(|error| js_error(format!("failed to query selector `{selector}`: {:?}", error)))?
      .ok_or_else(|| js_error(format!("DOM selector `{selector}` did not match an element")))?;

    element.set_text_content(Some(text));
    Ok(())
  }

  #[wasm_bindgen(js_name = "readClipboardText")]
  pub async fn read_clipboard_text(&self) -> Result<String, JsValue> {
    self
      .browser_host
      .check(BrowserCapabilityRequest::Clipboard {
        operation: BrowserOperation::Read,
      })
      .map_err(browser_host_error_to_js)?;

    let window = web_sys::window()
      .ok_or_else(|| js_error("global window does not exist"))?;
    let clipboard = window.navigator().clipboard();

    let promise = clipboard.read_text();
    let value = wasm_bindgen_futures::JsFuture::from(promise)
      .await
      .map_err(|error| js_error(format!("clipboard read failed: {:?}", error)))?;

    Ok(value.as_string().unwrap_or_default())
  }

  #[wasm_bindgen(js_name = "writeClipboardText")]
  pub async fn write_clipboard_text(&self, text: &str) -> Result<(), JsValue> {
    self
      .browser_host
      .check(BrowserCapabilityRequest::Clipboard {
        operation: BrowserOperation::Write,
      })
      .map_err(browser_host_error_to_js)?;

    let window = web_sys::window()
      .ok_or_else(|| js_error("global window does not exist"))?;
    let clipboard = window.navigator().clipboard();

    let promise = clipboard.write_text(text);
    wasm_bindgen_futures::JsFuture::from(promise)
      .await
      .map_err(|error| js_error(format!("clipboard write failed: {:?}", error)))?;

    Ok(())
  }

  #[wasm_bindgen(js_name = "fetchText")]
  pub async fn fetch_text(&self, url: &str, method: Option<String>) -> Result<String, JsValue> {
    let method = method.unwrap_or_else(|| "GET".to_string());
    let origin = browser_origin_for_url(url)?;

    self
      .check_network(&origin, Some(&method), BrowserOperation::Read)
      .map_err(browser_host_error_to_js)?;

    let method = method
      .parse::<Method>()
      .map_err(|error| js_error(format!("invalid HTTP method `{method}`: {error}")))?;
    let response = RequestBuilder::new(url)
      .method(method)
      .send()
      .await
      .map_err(|error| js_error(format!("browser fetch failed: {error}")))?;

    response
      .text()
      .await
      .map_err(|error| js_error(format!("failed to read fetch response text: {error}")))
  }

  #[wasm_bindgen(js_name = "readStorageText")]
  pub fn read_storage_text(
    &self,
    backend: &str,
    scope: &str,
  ) -> Result<Option<String>, JsValue> {
    self
      .check_storage(backend, scope, BrowserOperation::Read)
      .map_err(browser_host_error_to_js)?;

    let storage = browser_storage_for_backend(backend)?;
    Ok(storage
      .get_item(scope)
      .map_err(|error| js_error(format!("storage read failed: {:?}", error)))?)
  }

  #[wasm_bindgen(js_name = "writeStorageText")]
  pub fn write_storage_text(
    &self,
    backend: &str,
    scope: &str,
    value: &str,
  ) -> Result<(), JsValue> {
    self
      .check_storage(backend, scope, BrowserOperation::Write)
      .map_err(browser_host_error_to_js)?;

    let storage = browser_storage_for_backend(backend)?;
    storage
      .set_item(scope, value)
      .map_err(|error| js_error(format!("storage write failed: {:?}", error)))?;
    Ok(())
  }

  #[wasm_bindgen(js_name = "removeStorageItem")]
  pub fn remove_storage_item(
    &self,
    backend: &str,
    scope: &str,
  ) -> Result<(), JsValue> {
    self
      .check_storage(backend, scope, BrowserOperation::Write)
      .map_err(browser_host_error_to_js)?;

    let storage = browser_storage_for_backend(backend)?;
    storage
      .remove_item(scope)
      .map_err(|error| js_error(format!("storage remove failed: {:?}", error)))?;
    Ok(())
  }

  #[wasm_bindgen(js_name = "canReadClipboard")]
  pub fn can_read_clipboard(&self) -> bool {
    self.browser_host
      .check(BrowserCapabilityRequest::Clipboard {
        operation: BrowserOperation::Read,
      })
      .is_ok()
  }

  #[wasm_bindgen(js_name = "canWriteClipboard")]
  pub fn can_write_clipboard(&self) -> bool {
    self.browser_host
      .check(BrowserCapabilityRequest::Clipboard {
        operation: BrowserOperation::Write,
      })
      .is_ok()
  }

  #[wasm_bindgen(js_name = "canReadDom")]
  pub fn can_read_dom(&self, selector: &str) -> bool {
    self.check_dom(selector, BrowserOperation::Read).is_ok()
  }

  #[wasm_bindgen(js_name = "canWriteDom")]
  pub fn can_write_dom(&self, selector: &str) -> bool {
    self.check_dom(selector, BrowserOperation::Write).is_ok()
  }

  #[wasm_bindgen(js_name = "canReadNetwork")]
  pub fn can_read_network(&self, origin: &str, method: Option<String>) -> bool {
    self.check_network(origin, method.as_deref(), BrowserOperation::Read)
      .is_ok()
  }

  #[wasm_bindgen(js_name = "canReadStorage")]
  pub fn can_read_storage(&self, backend: &str, scope: &str) -> bool {
    self.check_storage(backend, scope, BrowserOperation::Read).is_ok()
  }

  #[wasm_bindgen(js_name = "canWriteStorage")]
  pub fn can_write_storage(&self, backend: &str, scope: &str) -> bool {
    self.check_storage(backend, scope, BrowserOperation::Write).is_ok()
  }

  #[wasm_bindgen(js_name = "canListStorage")]
  pub fn can_list_storage(&self, backend: &str, scope: &str) -> bool {
    self.check_storage(backend, scope, BrowserOperation::List).is_ok()
  }

  #[wasm_bindgen(js_name = "browserGrantCount")]
  pub fn browser_grant_count(&self) -> usize {
    self.browser_host.authority().grants().len()
  }

  fn check_dom(
    &self,
    selector: &str,
    operation: BrowserOperation,
  ) -> Result<(), BrowserHostError> {
    BrowserDomScope::new(selector.to_string())
      .map_err(|error| BrowserHostError::BrowserDeniedOrUnavailable {
        reason: error.to_string(),
      })?;
    self.browser_host.check(BrowserCapabilityRequest::Dom {
      selector: selector.to_string(),
      operation,
    })
  }

  fn check_network(
    &self,
    origin: &str,
    method: Option<&str>,
    operation: BrowserOperation,
  ) -> Result<(), BrowserHostError> {
    let methods = method.map(|method| vec![method.to_string()]);
    BrowserNetworkScope::new(origin.to_string(), methods)
      .map_err(|error| BrowserHostError::BrowserDeniedOrUnavailable {
        reason: error.to_string(),
      })?;
    self.browser_host.check(BrowserCapabilityRequest::Network {
      origin: origin.to_string(),
      method: method.map(str::to_string),
      operation,
    })
  }

  fn check_storage(
    &self,
    backend: &str,
    scope: &str,
    operation: BrowserOperation,
  ) -> Result<(), BrowserHostError> {
    let parsed_backend = BrowserStorageBackend::parse(backend)
      .ok_or_else(|| BrowserHostError::BrowserDeniedOrUnavailable {
        reason: format!("unknown storage backend `{backend}`"),
      })?;
    BrowserStorageScope::new(parsed_backend, scope.to_string())
      .map_err(|error| BrowserHostError::BrowserDeniedOrUnavailable {
        reason: error.to_string(),
      })?;
    self.browser_host.check(BrowserCapabilityRequest::Storage {
      backend: parsed_backend,
      scope: scope.to_string(),
      operation,
    })
  }

  #[cfg(test)]
  fn runtime_config_for_test(&self) -> &RuntimeConfig {
    self.runtime.config()
  }

#[cfg(feature = "repl")]
#[wasm_bindgen]
pub fn attach_repl(&mut self, repl_id: &str) {
  self.repl_id = Some(repl_id.to_string());

  // Assign self to the CURRENT_MECH thread-local variable for callbacks
  CURRENT_MECH.with(|c| *c.borrow_mut() = Some(self as *mut _));

  let window = web_sys::window().expect("global window does not exist");
  let document = window.document().expect("should have a document");
  let container = document
    .get_element_by_id(repl_id)
    .expect("REPL element not found")
    .dyn_into::<HtmlElement>()
    .expect("Element should be HtmlElement");

  // Remove "hidden" from REPL container and the resizer.
  let resizer = document
    .get_element_by_id("resizer")
    .expect("Resizer element not found");
  resizer.class_list().remove_1("hidden").unwrap();
  let repl_container = document
    .get_element_by_id("mech-output")
    .expect("REPL container element not found");
  repl_container.class_list().remove_1("hidden").unwrap();

  // Rc<RefCell> to store the create_prompt callback
  let create_prompt: Rc<RefCell<Option<Box<dyn Fn()>>>> = Rc::new(RefCell::new(None));
  let create_prompt_clone = create_prompt.clone();
  let document_clone = document.clone();
  let container_clone = container.clone();

  // Helper to create a new REPL line and input
  *create_prompt.borrow_mut() = Some(Box::new(move || {
    let line = document_clone.create_element("div").unwrap();
    line.set_class_name("repl-line");

    //let prompt = document_clone.create_element("span").unwrap();
    //prompt.set_inner_html("&gt;: ");
    //prompt.set_class_name("repl-prompt");

    let document = web_sys::window().unwrap().document().unwrap();

    let input = document
        .create_element("div")
        .unwrap()
        .dyn_into::<HtmlElement>()
        .unwrap();
    input.set_class_name("repl-input");
    input.set_id("repl-active-input");
    input.set_attribute("contenteditable", "true").unwrap();
    input.set_attribute("spellcheck", "false").unwrap();
    input.set_attribute("autocomplete", "off").unwrap();
    input.set_autofocus(true);
    let input_for_closure = input.clone();


    //line.append_child(&prompt).unwrap();
    line.append_child(&input).unwrap();
    container_clone.append_child(&line).unwrap();
    let _ = input.focus();

    let document_inner = document_clone.clone();
    let container_inner = container_clone.clone();
    let create_prompt_inner = create_prompt_clone.clone();

    // Keyboard handling for Enter and history
    let closure = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
      match event.key().as_str() {
        "Enter" => {
          if event.shift_key() {
            return;
          }
          event.prevent_default();
          let code = input_for_closure.text_content().unwrap_or_default();

          // Replace input field with text
          let input_parent = input_for_closure.parent_node().expect("input should have a parent");
          let input_span = document_inner.create_element("span").unwrap();
          input_span.set_class_name("repl-code");
          input_span.set_text_content(Some(&code));
          input_parent.replace_child(&input_span, &input_for_closure).unwrap();

          let result_line = document_inner.create_element("div").unwrap();
          result_line.set_class_name("repl-result");

          CURRENT_MECH.with(|mech_ref| {
            if let Some(ptr) = *mech_ref.borrow() {
              unsafe {
                let mech = &mut *ptr;
                let output = if !code.trim().is_empty() {
                  mech.repl_history.push(code.clone());
                  mech.repl_history_index = None;
                  mech.eval(&code)
                } else {
                  "".to_string()
                };
                result_line.set_inner_html(&output);
                container_inner.append_child(&result_line).unwrap();
                mech.init();
                mech.render_inline_values();
                mech.render_codeblock_output_values();
              }
            }
          });

          if let Some(cb) = &*create_prompt_inner.borrow() {
            cb();
          }
        }
        "ArrowUp" => {
          if event.ctrl_key() {
            event.prevent_default();
            CURRENT_MECH.with(|mech_ref| {
              if let Some(ptr) = *mech_ref.borrow() {
                unsafe {
                  let mech = &mut *ptr;
                  if !mech.repl_history.is_empty() {
                    let new_index = match mech.repl_history_index {
                      Some(i) if i > 0 => Some(i - 1),
                      None => Some(mech.repl_history.len().saturating_sub(1)),
                      Some(0) => Some(0),
                      _ => None,
                    };
                    if let Some(i) = new_index {
                      input_for_closure.set_text_content(Some(&mech.repl_history[i]));
                      mech.repl_history_index = Some(i);
                    }
                  }
                }
              }
            });
          } else {
            let selection = web_sys::window().unwrap().get_selection().unwrap().unwrap();
            let srange = selection.get_range_at(0).unwrap();
            let caret_pos = srange.start_offset().unwrap() as usize;

            let text = input_for_closure.text_content().unwrap_or_default();
            let lines: Vec<&str> = text.split('\n').collect();
            let caret_line = text[..caret_pos].matches('\n').count();

            if caret_line == 0 {
              event.prevent_default();
              CURRENT_MECH.with(|mech_ref| {
                if let Some(ptr) = *mech_ref.borrow() {
                  unsafe {
                    let mech = &mut *ptr;
                    if !mech.repl_history.is_empty() {
                      let new_index = match mech.repl_history_index {
                        Some(i) if i > 0 => Some(i - 1),
                        None => Some(mech.repl_history.len().saturating_sub(1)),
                        Some(0) => Some(0),
                        _ => None,
                      };
                      if let Some(i) = new_index {
                        input_for_closure.set_text_content(Some(&mech.repl_history[i]));
                        mech.repl_history_index = Some(i);
                      }
                    }
                  }
                }
              });
            }
          }
        },
        "ArrowDown" => {
          if event.ctrl_key() {
            event.prevent_default();
            CURRENT_MECH.with(|mech_ref| {
              if let Some(ptr) = *mech_ref.borrow() {
                unsafe {
                  let mech = &mut *ptr;
                  if let Some(i) = mech.repl_history_index {
                    let new_index = if i + 1 < mech.repl_history.len() {
                      Some(i + 1)
                    } else {
                      None
                    };
                    if let Some(i) = new_index {
                      input_for_closure.set_text_content(Some(&mech.repl_history[i]));
                      mech.repl_history_index = Some(i);
                    } else {
                      input_for_closure.set_text_content(Some(""));
                      mech.repl_history_index = None;
                    }
                  }
                }
              }
            });
          } else {
            let selection = web_sys::window().unwrap().get_selection().unwrap().unwrap();
            let srange = selection.get_range_at(0).unwrap();
            let caret_pos = srange.start_offset().unwrap() as usize;

            let text = input_for_closure.text_content().unwrap_or_default();
            let lines: Vec<&str> = text.split('\n').collect();
            let caret_line = text[..caret_pos].matches('\n').count();

            if caret_line == lines.len() - 1 {
              event.prevent_default();
              CURRENT_MECH.with(|mech_ref| {
                if let Some(ptr) = *mech_ref.borrow() {
                  unsafe {
                    let mech = &mut *ptr;
                    if let Some(i) = mech.repl_history_index {
                      let new_index = if i + 1 < mech.repl_history.len() {
                        Some(i + 1)
                      } else {
                        None
                      };
                      if let Some(i) = new_index {
                        input_for_closure.set_text_content(Some(&mech.repl_history[i]));
                        mech.repl_history_index = Some(i);
                      } else {
                        input_for_closure.set_text_content(Some(""));
                        mech.repl_history_index = None;
                      }
                    }
                  }
                }
              });
            }
          }
        },
        _ => (),
      }
    }) as Box<dyn FnMut(_)>);

    input.add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref()).unwrap();
    closure.forget();
  }));

  let intro_line = document.create_element("div").unwrap();
  intro_line.set_class_name("repl-result");
  intro_line.set_inner_html(
    "<div class=\"mech-output-value\">Enter <code class=\"mech-inline-code\">:help</code> for a list of all commands.</div>",
  );
  container.append_child(&intro_line).unwrap();

  // Initial prompt
  if let Some(cb) = &*create_prompt.borrow() {
    cb();
  }

  // Click handler to focus input if selection is collapsed
  let mech_output = container.clone();
  let mech_output_for_event = mech_output.clone();
  let click_closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
    let window = web_sys::window().unwrap();
    let selection = window.get_selection().unwrap().unwrap();
    if selection.is_collapsed() {
      if let Some(input) = mech_output.owner_document().unwrap().get_element_by_id("repl-active-input") {
        let _ = input.dyn_ref::<HtmlElement>().unwrap().focus();
      }
    }
  }) as Box<dyn FnMut(_)>);
  mech_output_for_event.add_event_listener_with_callback("click", click_closure.as_ref().unchecked_ref()).unwrap();
  click_closure.forget();

  // Hashchange handler: acts like entering the value into the REPL
  let create_prompt_clone2 = create_prompt.clone();
  let hashchange_closure = Closure::wrap(Box::new(move |event: HashChangeEvent| {
    let new_url = event.new_url();
    let url = match Url::new(&new_url) {
      Ok(u) => u,
      Err(_) => {
        log!("Failed to parse URL from hashchange event {:?}", new_url);
        return;
      }
    };
    let hash = url.hash();
    let decoded: String = match decode_uri_component(hash.trim_start_matches('#')).ok() {
        Some(h) if h.starts_with(":", 0) => h.into(),
        _ => return,
    };
    CURRENT_MECH.with(|mech_ref| {
      if let Some(ptr) = *mech_ref.borrow() {
        unsafe {
          let mech = &mut *ptr;
          if let Some(repl_id) = &mech.repl_id {
            if let Some(doc) = web_sys::window().unwrap().document() {
              if let Some(container) = doc.get_element_by_id(repl_id) {
                if let Some(input) = doc.get_element_by_id("repl-active-input") {
                  let input = input.dyn_into::<web_sys::HtmlElement>().unwrap();
                  input.set_text_content(Some(&decoded)); // fill with hash

                  let output = mech.eval(&decoded); // evaluate
                  let result_line = doc.create_element("div").unwrap();
                  result_line.set_class_name("repl-result");
                  result_line.set_inner_html(&output);
                  container.append_child(&result_line).unwrap();

                  mech.init();
                  mech.render_inline_values();
                  mech.render_codeblock_output_values();

                  // Replace previous prompt with a span
                  if let Some(old_input) = doc.get_element_by_id("repl-active-input") {
                    let old_input_parent = old_input.parent_node().expect("input should have a parent");
                    let input_span = doc.create_element("span").unwrap();
                    input_span.set_class_name("repl-code");
                    input_span.set_text_content(Some(&decoded));
                    old_input_parent.replace_child(&input_span, &old_input).unwrap();
                  }

                  // Create next prompt
                  if let Some(cb) = &*create_prompt_clone2.borrow() {
                    cb();
                  }
                }
              }
            }
          }
        }
      }
    });
  }) as Box<dyn FnMut(HashChangeEvent)>);
  window.add_event_listener_with_callback("hashchange", hashchange_closure.as_ref().unchecked_ref()).unwrap();
  hashchange_closure.forget();
}


  #[cfg(feature = "eval")]
  fn format_eval_error_html(error: impl std::fmt::Display) -> String {
    format!(
      "<div class=\"mech-output-kind\">Error</div><div class=\"mech-output-value\">{}</div>",
      html_escape(&error.to_string())
    )
  }

  #[cfg(feature = "eval")]
  fn format_eval_mech_error_html(error: &MechError) -> String {
    format!(
      "<div class=\"mech-output-kind\">Error</div><div class=\"mech-output-value\">{}</div>",
      error.to_html()
    )
  }

  #[cfg(feature = "eval")]
  fn eval_tree(&mut self, tree: &Program) -> String {
    match self.runtime.run_tree(tree) {
      Ok(output) => format_output_value_html(&output),
      Err(err) => Self::format_eval_mech_error_html(&err),
    }
  }

  #[cfg(feature = "eval")]
  pub fn eval(&mut self, input: &str) -> String {
    if input.chars().nth(0) == Some(':') {
      #[cfg(feature = "repl")]
      match parse_repl_command(&input.to_string()) {
        Ok((_, repl_command)) => {
          execute_repl_command(repl_command)
        }
        Err(x) => {
          let message = html_escape(&format!("Unrecognized command: {}", x));
          format!(
            "<div class=\"mech-output-kind\">Error</div><div class=\"mech-output-value\"><div class=\"mech-runtime-error\">\
              <div class=\"mech-runtime-error-header\">\
                <span class=\"mech-runtime-error-icon\" aria-hidden=\"true\"></span>\
                <div>\
                  <div class=\"mech-runtime-error-title\">UnrecognizedCommand</div>\
                  <div class=\"mech-runtime-error-message\">{}</div>\
                </div>\
              </div>\
            </div></div>",
            message
          )
        }
      }
      #[cfg(not(feature = "repl"))]
      {
        "REPL commands not supported. Rebuild with the 'repl' feature.".to_string()
      }
    } else {
      match self.runtime.run_string(input) {
        Ok(output) => format_output_value_html(&output),
        Err(err) => Self::format_eval_mech_error_html(&err),
      }
    }
  }

  #[cfg(feature = "eval")]
  #[wasm_bindgen(js_name = "evalCompiled")]
  pub fn eval_compiled(&mut self, input: &str) -> String {
    match decode_and_decompress::<Program>(input) {
      Ok(tree) => self.eval_tree(&tree),
      Err(err) => Self::format_eval_error_html(format!("failed to decode compiled Mech code: {err}")),
    }
  }

  #[cfg(feature = "clickable_symbol_listeners")]
  #[wasm_bindgen]
  pub fn add_clickable_event_listeners(&self) {
    let window = web_sys::window().expect("global window does not exist");
    let document = window.document().expect("expecting a document on window");

    // Set up a click event listener for all elements with the class "mech-clickable"
    let clickable_elements = document.get_elements_by_class_name("mech-clickable");

    for i in 0..clickable_elements.length() {
      let element = clickable_elements.get_with_index(i).unwrap();

      // Skip if listener already added
      if element.get_attribute("data-click-bound").is_some() {
        continue;
      }

      // Mark it as handled
      element.set_attribute("data-click-bound", "true").unwrap();

      // Parse element id
      let id = element.id();
      let parsed_id: Vec<&str> = id.split(":").collect();
      let (element_id, interpreter_id) = match parsed_id.as_slice() {
        [output_id, interpreter_id] => {
          match (output_id.parse::<u64>(), interpreter_id.parse::<u64>()) {
            (Ok(output_id), Ok(interpreter_id)) => (output_id, interpreter_id),
            _ => {
              log!("Invalid clickable symbol id format: {}", id);
              continue;
            }
          }
        }
        [output_id] => match output_id.parse::<u64>() {
          Ok(output_id) => (output_id, 0),
          Err(_) => {
            log!("Invalid clickable symbol id format: {}", id);
            continue;
          }
        },
        _ => {
          log!("Invalid clickable symbol id format: {}", id);
          continue;
        }
      };
      let symbol_text = element.text_content().unwrap_or_default();
      let symbol_name_hint = element
        .get_attribute("data-var")
        .unwrap_or_else(|| symbol_text.clone());

      // Create click closure
      let closure = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        let mech_output = document.get_element_by_id("mech-output").unwrap();
        let last_child = mech_output.last_child();

        CURRENT_MECH.with(|mech_ref| {
          let Some(ptr) = *mech_ref.borrow() else {
            log!("Clickable click: no current mech instance");
            return;
          };
          unsafe {
            let mech = &mut *ptr;
            let output = match mech.runtime.output_value_for_interpreter(interpreter_id, element_id) {
              Some(value) => value,
              None => {
                let error_message = format!("No value found for element id: {}", element_id);
                log!(
                  "Clickable click: missing symbol output for element_id={} interpreter_id={} id='{}'",
                  element_id,
                  interpreter_id,
                  id
                );
                let result_line = document.create_element("div").unwrap();
                result_line.set_class_name("repl-result");
                result_line.set_inner_html(&error_message);
                if let Some(last_child) = last_child {
                  mech_output.insert_before(&result_line, Some(&last_child)).unwrap();
                } else {
                  mech_output.append_child(&result_line).unwrap();
                }
                return;
              }
            };
            let symbol_name = if symbol_text.trim().is_empty() {
              mech
                .runtime
                .symbol_name_for_interpreter_output(interpreter_id, element_id)
                .or_else(|| {
                  let trimmed = symbol_name_hint.trim();
                  if trimmed.is_empty() {
                    None
                  } else {
                    Some(trimmed.to_string())
                  }
                })
                .unwrap_or_else(|| format!("symbol_{}", element_id))
            } else {
              let trimmed_hint = symbol_name_hint.trim();
              if !trimmed_hint.is_empty() && trimmed_hint != symbol_text.trim() {
                trimmed_hint.to_string()
              } else {
                symbol_text.clone()
              }
            };
            let repl_width = mech_output.client_width();
            // If REPL is "closed", show modal only (do not write to REPL).
            if repl_width == 0 {
              let modal = document.create_element("div").unwrap();
              modal.set_class_name("mech-modal");
              modal.set_inner_html(&format_output_value_html(&output));

              let x = event.client_x();
              let y = event.client_y();
              modal.set_attribute(
                "style",
                &format!(
                  "position:absolute; top:{}px; left:{}px;",
                  y, x
                )
              ).unwrap();

              document.body().unwrap().append_child(&modal).unwrap();

              // Click to close modal
              let modal_clone = modal.clone();
              let close_closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
                modal_clone.remove();
              }) as Box<dyn FnMut(_)>);
              modal.add_event_listener_with_callback("click", close_closure.as_ref().unchecked_ref()).unwrap();
              close_closure.forget();
              return;
            }

            let result_html = format_output_value_html(&output);

            // Add prompt line
            let prompt_line = document.create_element("div").unwrap();
            prompt_line.set_class_name("repl-line");
            let input_span = document.create_element("span").unwrap();
            input_span.set_class_name("repl-code");
            input_span.set_inner_html(&symbol_name);
            prompt_line.append_child(&input_span).unwrap();
            if let Some(last_child) = last_child.clone() {
              mech_output.insert_before(&prompt_line, Some(&last_child)).unwrap();
            } else {
              mech_output.append_child(&prompt_line).unwrap();
            }

            // Add result line
            let result_line = document.create_element("div").unwrap();
            result_line.set_class_name("repl-result");
            result_line.set_inner_html(&result_html);
            if let Some(last_child) = last_child {
              mech_output.insert_before(&result_line, Some(&last_child)).unwrap();
            } else {
              mech_output.append_child(&result_line).unwrap();
            }

            // Update REPL history
            mech.repl_history.push(symbol_name.clone());

            // Update variable "ans" with the value of the clicked symbol
            let _ = mech.runtime.bind_ans_for_interpreter(interpreter_id, &output);
          }
        });

        mech_output.set_scroll_top(mech_output.scroll_height());
      }) as Box<dyn FnMut(_)>);

      element.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref()).unwrap();
      closure.forget();
    }
  }

  #[wasm_bindgen]
  pub fn init(&self) {
    #[cfg(feature = "clickable_symbol_listeners")]
    self.add_clickable_event_listeners();
  }

   #[wasm_bindgen]
   pub fn render_values(&mut self) {
    #[cfg(feature = "codeblock_output_values")]
    self.render_codeblock_output_values();
    #[cfg(feature = "inline_output_values")]
    self.render_inline_values();
  }

  // Write block output each element that needs it, rendering it appropriately
  // based on its data type.
  #[cfg(feature = "codeblock_output_values")]
  #[wasm_bindgen]
  pub fn render_codeblock_output_values(&mut self) {
    let window = web_sys::window().expect("global window does not exists");
    let document = window.document().expect("expecting a document on window");
    // Get all elements with an attribute of "mech-interpreter-id"
    let programs = document.query_selector_all("[mech-interpreter-id]");
    if let Ok(programs) = programs {
      for i in 0..programs.length() {
        let program_node = programs.item(i).expect("No node at index");
        let program_el = program_node
            .dyn_into::<Element>()
            .expect("Node was not an Element");

        // Get the mech-interpreter-id attribute from the element
        let interpreter_id: String = program_el.get_attribute("mech-interpreter-id").unwrap();
        let interpreter_id: u64 = interpreter_id.parse().unwrap();
        let root_interpreter_id = interpreter_id;
        if !self.runtime.has_interpreter(root_interpreter_id) {
          log!("No sub interpreter found for id: {}", root_interpreter_id);
          continue;
        }

        // Get all elements with the class "mech-block-output" that are children of the program element
        let output_elements = program_el.query_selector_all(".mech-block-output");
        if let Ok(output_elements) = output_elements {
          for j in 0..output_elements.length() {
            let block_node = output_elements.item(j).expect("No output element at index");
            let block = block_node
                .dyn_into::<web_sys::Element>()
                .expect("Output node was not an Element");

            // the id looks like this
            // output_id:interpreter_id
            // so we need to parse it to get the id and the interpreter id
            let id = block.id();
            let parsed_id: Vec<&str> = id.split(":").collect();
            let output_id = parsed_id[0].parse::<u64>().unwrap();
            let interpreter_id = parsed_id[1].parse::<u64>().unwrap();
            // get the interpreter id from the block id
            let effective_interpreter_id = if interpreter_id == 0 {
              root_interpreter_id
            } else {
              interpreter_id
            };
            let output = match self.runtime.output_value_for_interpreter(effective_interpreter_id, output_id) {
              Some(value) => value,
              None => {
                log!("No value found for output id: {}", output_id);
                continue;
              }
            };
            // set the inner html of the block to the output value html
            block.set_inner_html(&format_output_value_html(&output));
          }
        }
      }
    }
  }

  #[cfg(feature = "inline_output_values")]
  #[wasm_bindgen]
  pub fn render_inline_values(&mut self) {
    let window = web_sys::window().expect("global window does not exists");
    let document = window.document().expect("expecting a document on window");
    let inline_elements = document.get_elements_by_class_name("mech-inline-mech-code");
    for j in 0..inline_elements.length() {
      let inline_block = inline_elements.get_with_index(j).unwrap();
      let inline_id = inline_block.id();
      let parsed_id: Vec<&str> = inline_id.split(":").collect();
      let (inline_output_id, inline_interpreter_id) = match parsed_id.as_slice() {
        [output_id, interpreter_id] => {
          match (output_id.parse::<u64>(), interpreter_id.parse::<u64>()) {
            (Ok(output_id), Ok(interpreter_id)) => (output_id, interpreter_id),
            _ => {
              log!("Invalid inline output id format: {}", inline_id);
              continue;
            }
          }
        }
        [output_id] => {
          match output_id.parse::<u64>() {
            Ok(output_id) => (output_id, 0),
            Err(_) => {
              log!("Invalid inline output id format: {}", inline_id);
              continue;
            }
          }
        }
        _ => {
          log!("Invalid inline output id format: {}", inline_id);
          continue;
        }
      };
      let inline_output = match self.runtime.output_value_for_interpreter(inline_interpreter_id, inline_output_id) {
        Some(value) => value,
        None => {
          log!(
            "No value found for inline output id: {} in interpreter {}",
            inline_output_id,
            inline_interpreter_id
          );
          continue;
        }
      };
      let formatted_output = inline_output.format_value_inline();
      let is_scalar = matches!(
        inline_output,
        Value::U8(_)
          | Value::U16(_)
          | Value::U32(_)
          | Value::U64(_)
          | Value::U128(_)
          | Value::I8(_)
          | Value::I16(_)
          | Value::I32(_)
          | Value::I64(_)
          | Value::I128(_)
          | Value::F32(_)
          | Value::F64(_)
          | Value::Bool(_)
          | Value::String(_)
          | Value::C64(_)
          | Value::R64(_)
          | Value::Index(_)
          | Value::Id(_)
          | Value::Kind(_)
          | Value::IndexAll
          | Value::Empty
      );
      if is_scalar {
        inline_block.set_inner_html(&formatted_output.trim());
      } else {
        let compact = if formatted_output.chars().count() > 40 {
          let prefix = formatted_output.chars().take(40).collect::<String>();
          format!("{} ... ", prefix.trim_end())
        } else {
          format!("{} ", formatted_output.trim())
        };
        let inline_html = format!(
          "<span>{}</span><span class=\"mech-inline-expand\" id=\"{}:{}\">›</span>",
          compact,
          inline_output_id,
          inline_interpreter_id
        );
        inline_block.set_inner_html(&inline_html);
      }
    }
    #[cfg(feature = "symbol_table")]
    let var_elements = document.get_elements_by_class_name("mech-var-placeholder");
    #[cfg(feature = "symbol_table")]
    for j in 0..var_elements.length() {
      let var_element = var_elements.get_with_index(j).unwrap();
      let var_name = match var_element.get_attribute("data-var-name") {
        Some(value) => value,
        None => continue,
      };
      let var_id = hash_str(&var_name);
      let interpreter_id = match var_element.get_attribute("data-interpreter-name") {
        Some(value) => hash_str(&value),
        None => match var_element.get_attribute("data-interpreter-id") {
          Some(value) => value.parse::<u64>().unwrap_or(0),
          None => 0,
        },
      };
      let output = match self.runtime.output_value_for_interpreter(interpreter_id, var_id) {
        Some(value) => value,
        None => {
          log!(
            "VAR placeholder unresolved variable (yet?): {} (hash: {}, interpreter: {})",
            var_name,
            var_id,
            interpreter_id
          );
          continue;
        }
      };
      let formatted = output.to_html();
      let existing_class = var_element.get_attribute("class").unwrap_or_default();
      let clickable_class = if existing_class.is_empty() {
        "mech-clickable".to_string()
      } else if existing_class.split_whitespace().any(|name| name == "mech-clickable") {
        existing_class
      } else {
        format!("{} mech-clickable", existing_class)
      };
      let _ = var_element.set_attribute("class", &clickable_class);
      let _ = var_element.set_attribute("id", &format!("{}:{}", var_id, interpreter_id));
      let _ = var_element.set_attribute("data-var", &var_name);
      var_element.set_inner_html(formatted.trim());
    }
    #[cfg(not(feature = "symbol_table"))]
    log!("VAR placeholders require feature 'symbol_table' to resolve values.");
    #[cfg(feature = "clickable_symbol_listeners")]
    self.add_inline_value_clickable_listeners();
  }

  #[cfg(all(feature = "inline_output_values", feature = "clickable_symbol_listeners"))]
  #[wasm_bindgen]
  pub fn add_inline_value_clickable_listeners(&self) {
    let window = web_sys::window().expect("global window does not exist");
    let document = window.document().expect("expecting a document on window");
    let clickable_elements = document.get_elements_by_class_name("mech-inline-expand");

    for i in 0..clickable_elements.length() {
      let element = clickable_elements.get_with_index(i).unwrap();
      if element.get_attribute("data-click-bound").is_some() {
        continue;
      }
      element.set_attribute("data-click-bound", "true").unwrap();
      let id = element.id();
      let parsed_id: Vec<&str> = id.split(":").collect();
      if parsed_id.len() != 2 {
        continue;
      }
      let output_id = parsed_id[0].parse::<u64>().unwrap();
      let interpreter_id = parsed_id[1].parse::<u64>().unwrap();

      let closure = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        let mech_output = document.get_element_by_id("mech-output").unwrap();
        let last_child = mech_output.last_child();

        let output = CURRENT_MECH.with(|mech_ref| {
          if let Some(ptr) = *mech_ref.borrow() {
            unsafe {
              let mech = &*ptr;
              return mech.runtime.output_value_for_interpreter(interpreter_id, output_id);
            }
          }
          None
        });

        if let Some(output_value) = output {
          let result_html = format_output_value_html(&output_value);
          let repl_width = mech_output.client_width();

          CURRENT_MECH.with(|mech_ref| {
            if let Some(ptr) = *mech_ref.borrow() {
              unsafe {
                let mech = &mut *ptr;
                let _ = mech.runtime.bind_ans_for_interpreter(interpreter_id, &output_value);
              }
            }
          });

          if repl_width == 0 {
            let modal = document.create_element("div").unwrap();
            modal.set_class_name("mech-modal");
            modal.set_inner_html(&result_html);
            let x = event.client_x();
            let y = event.client_y();
            modal
              .set_attribute("style", &format!("position:absolute; top:{}px; left:{}px;", y, x))
              .unwrap();
            document.body().unwrap().append_child(&modal).unwrap();
            let modal_clone = modal.clone();
            let close_closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
              modal_clone.remove();
            }) as Box<dyn FnMut(_)>);
            modal
              .add_event_listener_with_callback("click", close_closure.as_ref().unchecked_ref())
              .unwrap();
            close_closure.forget();
            return;
          }

          let prompt_line = document.create_element("div").unwrap();
          prompt_line.set_class_name("repl-line");
          let input_span = document.create_element("span").unwrap();
          input_span.set_class_name("repl-code");
          input_span.set_inner_html("ans");
          prompt_line.append_child(&input_span).unwrap();
          if let Some(last_child) = last_child.clone() {
            mech_output.insert_before(&prompt_line, Some(&last_child)).unwrap();
          } else {
            mech_output.append_child(&prompt_line).unwrap();
          }

          let result_line = document.create_element("div").unwrap();
          result_line.set_class_name("repl-result");
          result_line.set_inner_html(&result_html);
          if let Some(last_child) = last_child {
            mech_output.insert_before(&result_line, Some(&last_child)).unwrap();
          } else {
            mech_output.append_child(&result_line).unwrap();
          }

          CURRENT_MECH.with(|mech_ref| {
            if let Some(ptr) = *mech_ref.borrow() {
              unsafe {
                (*ptr).repl_history.push("ans".to_string());
              }
            }
          });

          mech_output.set_scroll_top(mech_output.scroll_height());
        }
      }) as Box<dyn FnMut(_)>);

      element
        .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .unwrap();
      closure.forget();
    }
  }

  #[cfg(feature = "run_program")]
  fn format_runtime_error_html(&self, error: &MechError) -> String {
    format!(
      "<div class=\"mech-output-kind\">Error</div><div class=\"mech-output-value\">{}</div>",
      error.to_html()
    )
  }

  #[cfg(feature = "run_program")]
  fn emit_runtime_error(&self, error: &MechError) {
    let mut rendered_to_page = false;
    let formatted_error = self.format_runtime_error_html(error);

    if let Some(window) = web_sys::window() {
      if let Some(document) = window.document() {
        if let Ok(output_blocks) = document.query_selector_all(".mech-block-output") {
          for i in 0..output_blocks.length() {
            if let Some(output_node) = output_blocks.item(i) {
              if let Ok(output_el) = output_node.dyn_into::<web_sys::Element>() {
                output_el.set_inner_html(&formatted_error);
                rendered_to_page = true;
              }
            }
          }
        }

        if !rendered_to_page {
          if let Some(root) = document.get_element_by_id("mech-root") {
            root.set_inner_html(&formatted_error);
            rendered_to_page = true;
          }
        }
      }
    }

    if !rendered_to_page {
      web_sys::console::error_1(&format!("Runtime error: {}", error.full_chain_message()).into());
    }
  }

  #[cfg(feature = "run_program")]
  fn interpret_with_runtime_error_handling(&mut self, tree: &Program) {
    match catch_unwind(AssertUnwindSafe(|| self.runtime.run_tree(tree))) {
      Ok(Ok(result)) => {
        log!("{}", result.pretty_print());
      }
      Ok(Err(err)) => {
        self.emit_runtime_error(&err);
      }
      Err(panic_payload) => {
        let panic_message = if let Some(message) = panic_payload.downcast_ref::<&str>() {
          (*message).to_string()
        } else if let Some(message) = panic_payload.downcast_ref::<String>() {
          message.clone()
        } else {
          "Unknown panic while running Mech program".to_string()
        };
        self.emit_runtime_error(
          &MechError::new(GenericError { msg: panic_message }, None).with_compiler_loc()
        );
      }
    }
  }

  #[cfg(feature = "run_program")]
  #[wasm_bindgen]
  pub fn run_program(&mut self, src: &str) {
    // Decompress the string into a Program
    match decode_and_decompress(&src) {
      Ok(tree) => {
        self.interpret_with_runtime_error_handling(&tree);
      },
      Err(err) => {
        match parse(src) {
          Ok(tree) => {
            self.interpret_with_runtime_error_handling(&tree);
          },
          Err(parse_err) => {
            self.emit_runtime_error(
              &MechError::new(
                GenericError { msg: format!("Error parsing program: {:?}", parse_err) },
                None,
              )
              .with_compiler_loc()
            );
          }
        }
      }
    }
  }
}

#[cfg(all(test, feature = "eval", feature = "serde"))]
mod eval_tests {
  use super::*;

  #[test]
  fn eval_is_source_only_and_compiled_eval_decodes_payload() {
    let source = "x := 1";
    let tree = parse(source).unwrap();
    let encoded = compress_and_encode(&tree).unwrap();
    assert_ne!(encoded, source);

    let mut source_mech = WasmMech::new();
    let source_output = source_mech.eval(source);
    assert!(!source_output.contains("ParserErrorContext"));

    let mut encoded_as_source_mech = WasmMech::new();
    let encoded_as_source_output = encoded_as_source_mech.eval(&encoded);
    assert!(encoded_as_source_output.contains("<div class=\"mech-output-kind\">Error</div>"));

    let mut compiled_mech = WasmMech::new();
    let compiled_output = compiled_mech.eval_compiled(&encoded);
    assert!(!compiled_output.contains("ParserErrorContext"));
  }

  #[test]
  fn eval_compiled_reports_decode_errors_without_parsing_input() {
    let mut mech = WasmMech::new();
    let output = mech.eval_compiled("x := 1");
    assert!(output.contains("failed to decode compiled Mech code"));
    assert!(!output.contains("ParserErrorContext"));
  }
}

#[cfg(feature = "docs")]
pub fn load_doc(doc: &str, element_id: String) {
  let doc = doc.to_string();
  spawn_local(async move {
    let doc_mec = fetch_docs(&doc).await;
    let doc_hash = hash_str(&doc_mec);
    let window = web_sys::window().expect("global window does not exists");
    let document = window.document().expect("expecting a document on window");
    match parser::parse(&doc_mec) {
      Ok(tree) => {
        let mut formatter = Formatter::new();
        formatter.html = true;
        let doc_html = formatter.program(&tree);
        CURRENT_MECH.with(|mech_ref| {
          if let Some(ptr) = *mech_ref.borrow() {
            unsafe {
              let mech = &mut *ptr;
              let _ = mech.runtime.run_string(&doc_mec);
            }
          }
        });
        let output_element = document.get_element_by_id(&element_id).expect("REPL output element not found");
        // Get the second to last element of mech-output. It should be a repl-result from when teh user pressed enter.
        // Set the inner html of the repl result element to be the formatted doc.
        let children = output_element.children();
        let len = children.length();
        if len >= 2 {
            let repl_result = children.item(len - 2).expect("Failed to get second-to-last child");
            repl_result.set_attribute("mech-interpreter-id", "0").unwrap();
            let repl_html = repl_result.dyn_ref::<HtmlElement>().expect("Expected an HtmlElement");
            repl_html.class_list().add_1("compact").unwrap();
            repl_html.set_inner_html(&doc_html);
            CURRENT_MECH.with(|mech_ref| {
              if let Some(ptr) = *mech_ref.borrow() {
                unsafe {
                  let mech = &mut *ptr;
                  #[cfg(feature = "codeblock_output_values")]
                  mech.render_codeblock_output_values();
                }
              }
            })
        } else {
            web_sys::console::log_1(&"Not enough children in #mech-output to update.".into());
        }
      },
      Err(err) => {
        web_sys::console::log_1(&format!("Error formatting doc: {:?}", err).into());
      }
    }
  });
}

#[cfg(feature = "docs")]
async fn fetch_docs(doc: &str) -> String {
  // the doc will be formatted as machine/doc
  let parts: Vec<&str> = doc.split('/').collect();
  if parts.len() >= 2 {
      let machine = parts[0];
      let doc = parts[1];
      let url = format!("https://raw.githubusercontent.com/mech-machines/{}/main/docs/{}.mec", machine, doc);
      match Request::get(&url).send().await {
        Ok(response) => match response.text().await {
          Ok(text) => {
            text
          }
          Err(e) => {
            web_sys::console::log_1(&format!("Error reading response text: {:?}", e).into());
            "".to_string()
          }
        },
        Err(err) => {
          web_sys::console::log_1(&format!("Fetch error: {:?}", err).into());
          "".to_string()
        }
      }
  } else {
    web_sys::console::log_1(&format!("Invalid doc format: {}", doc).into());
    "".to_string()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  const CLIPBOARD_WRITE_CONFIG: &str = r#"config := {
  browser: {
    clipboard: [
      {allow: ["write"]}
    ]
  }
}
"#;

  const DOM_WRITE_CONFIG: &str = r##"config := {
  browser: {
    dom: [
      {selector: "#mech-output", allow: ["write"]}
    ]
  }
}
"##;


  const DOM_READ_PATH_CONFIG: &str = r##"config := {
  browser: {
    dom: [
      {path: "counter/_text", selector: "#counter", allow: ["read"]}
    ]
  }
}
"##;

  const DOM_WRITE_PATH_CONFIG: &str = r##"config := {
  browser: {
    dom: [
      {path: "counter/_text", selector: "#counter", allow: ["write"]}
    ]
  }
}
"##;

  const NETWORK_GET_CONFIG: &str = r#"config := {
  browser: {
    network: [
      {origin: "https://example.com", methods: ["GET"], allow: ["read"]}
    ]
  }
}
"#;

  const STORAGE_EXACT_CONFIG: &str = r#"config := {
  browser: {
    storage: [
      {backend: "opfs", scope: "/workspace", allow: ["read", "write", "list"]}
    ]
  }
}
"#;

  const STORAGE_RECURSIVE_CONFIG: &str = r#"config := {
  browser: {
    storage: [
      {backend: "opfs", scope: "/workspace", recursive: true, allow: ["read", "write", "list"]}
    ]
  }
}
"#;

  #[test]
  fn wasm_mech_default_browser_host_is_deny_by_default() {
    let mech = WasmMech::new();

    assert_eq!(mech.browser_grant_count(), 0);
    assert!(!mech.can_read_clipboard());
  }

  #[test]
  fn wasm_mech_from_config_loads_browser_grants() {
    let mech = WasmMech::try_from_config("test.mcfg", CLIPBOARD_WRITE_CONFIG).unwrap();

    assert!(mech.browser_grant_count() > 0);
    assert!(mech.can_write_clipboard());
    assert!(!mech.can_read_clipboard());
  }

  #[test]
  fn wasm_mech_denies_dom_write_without_grant() {
    let mech = WasmMech::new();

    assert!(!mech.can_write_dom("#mech-output"));
  }

  #[test]
  fn wasm_mech_allows_dom_write_with_grant() {
    let mech = WasmMech::try_from_config("test.mcfg", DOM_WRITE_CONFIG).unwrap();

    assert!(mech.can_write_dom("#mech-output"));
  }


  #[test]
  fn browser_config_dom_read_installs_runtime_grant() {
    let mut mech = WasmMech::try_from_config("test.mcfg", DOM_READ_PATH_CONFIG).unwrap();
    let result = mech.runtime.run_string("+> @ui := browser/dom\ntitle := @ui/counter/_text\n");
    if let Err(error) = result {
      let error = format!("{error:?}");
      assert!(!error.contains("RuntimeCapabilityGrantDenied"), "read should pass runtime grant preflight: {error}");
    }
  }

  #[test]
  fn browser_config_without_dom_read_runtime_grant_denies() {
    let mut mech = WasmMech::try_from_config("test.mcfg", DOM_WRITE_PATH_CONFIG).unwrap();
    let result = mech.runtime.run_string("+> @ui := browser/dom\ntitle := @ui/counter/_text\n");
    let error = format!("{:?}", result.err().unwrap());
    assert!(error.contains("RuntimeCapabilityGrantDenied"), "read should be denied without read grant: {error}");
  }

  #[test]
  fn browser_config_dom_grants_are_operation_specific() {
    let mut write_only = WasmMech::try_from_config("test.mcfg", DOM_WRITE_PATH_CONFIG).unwrap();
    let read_result = write_only.runtime.run_string("+> @ui := browser/dom\ntitle := @ui/counter/_text\n");
    let read_error = format!("{:?}", read_result.err().unwrap());
    assert!(read_error.contains("RuntimeCapabilityGrantDenied"), "write-only grant must not allow reads: {read_error}");

    let mut read_only = WasmMech::try_from_config("test.mcfg", DOM_READ_PATH_CONFIG).unwrap();
    let write_result = read_only.runtime.run_string("+> @ui := browser/dom\n@ui/counter/_text = \"hello\"\n");
    let write_error = format!("{:?}", write_result.err().unwrap());
    assert!(write_error.contains("RuntimeCapabilityGrantDenied"), "read-only grant must not allow writes: {write_error}");
  }

  #[test]
  fn wasm_mech_denies_network_method_not_granted() {
    let mech = WasmMech::try_from_config("test.mcfg", NETWORK_GET_CONFIG).unwrap();

    assert!(!mech.can_read_network("https://example.com", Some("POST".to_string())));
  }

  #[test]
  fn wasm_mech_allows_network_method_granted() {
    let mech = WasmMech::try_from_config("test.mcfg", NETWORK_GET_CONFIG).unwrap();

    assert!(mech.can_read_network("https://example.com", Some("GET".to_string())));
  }

  #[test]
  fn wasm_mech_denies_recursive_storage_child_without_recursive() {
    let mech = WasmMech::try_from_config("test.mcfg", STORAGE_EXACT_CONFIG).unwrap();

    assert!(!mech.can_read_storage("opfs", "/workspace/main.mec"));
  }

  #[test]
  fn wasm_mech_allows_recursive_storage_child_with_recursive() {
    let mech = WasmMech::try_from_config("test.mcfg", STORAGE_RECURSIVE_CONFIG).unwrap();

    assert!(mech.can_read_storage("opfs", "/workspace/main.mec"));
  }

  #[test]
  fn wasm_mech_from_config_applies_runtime_config() {
    let source = r#"config := {
  runtime: {
    name: "wasm-test-runtime"
    limits: {
      max-steps-per-turn: 123
    }
  }
}
"#;
    let mech = WasmMech::try_from_config("test.mcfg", source).unwrap();
    let config = mech.runtime_config_for_test();

    assert_eq!(config.name, "wasm-test-runtime");
    assert_eq!(config.limits.max_steps_per_turn, Some(123));
  }

  #[test]
  fn wasm_from_config_filters_non_browser_run_grants() {
    let source = r#"config := {
  hosts: [
    {name: "browser", provider: "browser", settings: {}}
    {name: "arm", provider: "fake-robot", settings: {}}
  ]
  run: {
    grants: [
      {target: "browser/dom", operations: ["read"], paths: ["counter/_text"]}
      {target: "arm/commands", operations: ["write"], paths: ["move"]}
    ]
  }
}
"#;
    let mech = WasmMech::try_from_config("test.mcfg", source).unwrap();

    assert_eq!(mech.runtime_injection.run_grants.len(), 1);
    assert_eq!(mech.runtime_injection.run_grants[0].target, "browser/dom");
    assert!(mech.runtime_injection.hosts.iter().all(|host| host.provider == "browser"));
  }

  #[test]
  fn wasm_from_config_keeps_browser_alias_run_grant() {
    let source = r#"config := {
  hosts: [
    {name: "ui", provider: "browser", settings: {}}
  ]
  run: {
    grants: [
      {target: "ui/dom", operations: ["read"], paths: ["counter/_text"]}
    ]
  }
}
"#;
    let mech = WasmMech::try_from_config("test.mcfg", source).unwrap();

    assert!(mech.runtime_injection.hosts.iter().any(|host| host.name == "ui"));
    assert_eq!(mech.runtime_injection.run_grants.len(), 1);
    assert_eq!(mech.runtime_injection.run_grants[0].target, "ui/dom");
  }

  #[test]
  fn clear_preserves_browser_alias() {
    let source = r##"config := {
  hosts: [
    {
      name: "ui"
      provider: "browser"
      settings: {
        dom: [
          {path: "counter/_text", selector: "#counter", property: "text", operations: ["read"]}
        ]
      }
    }
  ]
  run: {
    grants: [
      {target: "ui/dom", operations: ["read"], paths: ["counter/_text"]}
    ]
  }
}
"##;
    let mut mech = WasmMech::try_from_config("test.mcfg", source).unwrap();
    mech.clear();

    assert!(mech.runtime_injection.hosts.iter().any(|host| host.name == "ui"));
    let result = mech.runtime.run_string("+> @ui := ui/dom\ntitle := @ui/counter/_text\n");
    if let Err(error) = result {
      let error = format!("{error:?}");
      assert!(!error.contains("HostInterfaceUnknownInstance"), "ui alias should survive clear: {error}");
      assert!(!error.contains("RuntimeResourceProviderNotFound"), "ui provider should survive clear: {error}");
      assert!(!error.contains("RuntimeCapabilityGrantDenied"), "ui read grant should survive clear: {error}");
    }
  }

  #[test]
  fn clear_preserves_narrowed_run_grants() {
    let source = r##"config := {
  hosts: [
    {
      name: "browser"
      provider: "browser"
      settings: {
        dom: [
          {path: "counter/_text", selector: "#counter", property: "text", operations: ["read", "write"]}
        ]
      }
    }
  ]
  run: {
    grants: [
      {target: "browser/dom", operations: ["read"], paths: ["counter/_text"]}
    ]
  }
}
"##;
    let mut mech = WasmMech::try_from_config("test.mcfg", source).unwrap();
    mech.clear();

    let write_result = mech.runtime.run_string("+> @ui := browser/dom\n@ui/counter/_text = \"hello\"\n");
    let write_error = format!("{:?}", write_result.err().unwrap());
    assert!(write_error.contains("RuntimeCapabilityGrantDenied"), "clear must preserve narrowed run grants: {write_error}");
  }

  #[test]
  fn clear_default_runtime_still_works() {
    let mut mech = WasmMech::new();
    mech.clear();

    assert_eq!(mech.browser_grant_count(), 0);
    assert!(!mech.can_read_clipboard());
  }

  #[test]
  fn wasm_mech_clear_preserves_browser_host() {
    let mut mech = WasmMech::try_from_config("test.mcfg", CLIPBOARD_WRITE_CONFIG).unwrap();

    mech.clear();

    assert!(mech.can_write_clipboard());
  }
}
