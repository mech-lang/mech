#![allow(warnings)]

pub mod host;

use wasm_bindgen::prelude::*;
use mech_core::*;
use mech_syntax::*;
use mech_runtime::{
  ConfigProfileOptions, MechConfigDocument, MechRuntime, RuntimeConfig,
  parse_config_document,
};
use crate::host::{
  BrowserCapabilityRequest, BrowserDomScope, BrowserHost, BrowserHostError,
  BrowserNetworkScope, BrowserOperation, BrowserStorageBackend, BrowserStorageScope,
};
use wasm_bindgen::JsCast;
use web_sys::{window, HtmlElement, HtmlInputElement, Node, Element, HashChangeEvent, HtmlTextAreaElement, Url};
use js_sys::decode_uri_component;
use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Arc;
use gloo_net::http::Request;
use wasm_bindgen_futures::spawn_local;

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


fn browser_host_from_config(document: Option<&MechConfigDocument>) -> BrowserHost {
  match document {
    Some(document) => BrowserHost::new(document.browser.clone()),
    None => BrowserHost::deny_by_default(),
  }
}

fn runtime_from_config_document(document: Option<&MechConfigDocument>) -> MechRuntime {
  let config = match document {
    Some(document) => RuntimeConfig::default()
      .apply_patch(&document.runtime)
      .expect("failed to apply wasm runtime config"),
    None => RuntimeConfig::default(),
  };

  MechRuntime::new(config)
    .expect("failed to initialize MechRuntime for wasm")
}

fn wasm_parts_from_config_document(
  document: Option<&MechConfigDocument>,
) -> (MechRuntime, BrowserHost) {
  (
    runtime_from_config_document(document),
    browser_host_from_config(document),
  )
}

#[wasm_bindgen]
pub struct WasmMech {
  runtime: MechRuntime,
  browser_host: BrowserHost,
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
      .map_err(|error| JsValue::from_str(&format!("{error:?}")))
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
    let (runtime, browser_host) = wasm_parts_from_config_document(Some(&document));

    Ok(Self {
      runtime,
      browser_host,
      repl_history: Vec::new(),
      repl_history_index: None,
      repl_id: None,
    })
  }

  fn with_default_runtime() -> Self {
    let (runtime, browser_host) = wasm_parts_from_config_document(None);

    Self {
      runtime,
      browser_host,
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
    self.runtime = MechRuntime::new(self.runtime.config().clone())
      .expect("failed to reset MechRuntime for wasm");
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
        Ok(output) => {
          let kind_str = html_escape(&format!("{}", output.kind()));
          format!(
            "<div class=\"mech-output-kind\">{}</div><div class=\"mech-output-value\">{}</div>",
            kind_str,
            output.to_html()
          )
        }
        Err(err) => {
          format!(
            "<div class=\"mech-output-kind\">Error</div><div class=\"mech-output-value\">{}</div>",
            err.to_html()
          )
        }
      }
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
  fn wasm_mech_clear_preserves_browser_host() {
    let mut mech = WasmMech::try_from_config("test.mcfg", CLIPBOARD_WRITE_CONFIG).unwrap();

    mech.clear();

    assert!(mech.can_write_clipboard());
  }
}
