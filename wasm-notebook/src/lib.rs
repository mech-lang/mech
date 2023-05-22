#[macro_use]
extern crate mech_wasm;
use mech_wasm::WasmCore;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
  Ok(())
}

#[wasm_bindgen]
pub fn new_core() -> WasmCore {
  WasmCore::new(1000, 1000)
}
