use mech_core::MResult;
use wasm_bindgen::JsValue;

use crate::{ConsoleBackend, ConsoleHostFactory};

#[derive(Clone, Copy, Debug, Default)]
pub struct BrowserConsoleBackend;

impl ConsoleBackend for BrowserConsoleBackend {
  fn write_line(&mut self, text: &str) -> MResult<()> {
    web_sys::console::log_1(&JsValue::from_str(text));
    Ok(())
  }
}

pub type BrowserConsoleHostFactory = ConsoleHostFactory<BrowserConsoleBackend>;

impl BrowserConsoleHostFactory {
  pub fn new() -> MResult<Self> { Self::with_backend(BrowserConsoleBackend) }
}
