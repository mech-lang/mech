use wasm_bindgen::prelude::*;

#[macro_use]
extern crate mech_wasm;
use mech_wasm::WasmCore;
//use mech_core::Core;
//use mech_syntax::compiler::Compiler;

use web_sys::XmlHttpRequest;

#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
  /*let mut mech_core = Core::new(100,100);

  log!("Here we go!");

  let window = web_sys::window().unwrap();
  let document = window.document().unwrap();
  let body = document.body().unwrap();

  let val = document.create_element("div")?;
  val.set_attribute("id","mech-root");
  body.append_child(&val)?;

  let xhr = XmlHttpRequest::new()?;
  xhr.open_with_async("GET", "./website/index.mec", false);
  xhr.send();
  let program = xhr.response_text();

  match program {
    Ok(Some(program)) => {
      mech_core.compile_code(program);
      mech_core.add_application();
    }, 
    _ => (),
  }*/

  

  

  Ok(())
}

#[wasm_bindgen]
pub fn new_core() -> WasmCore {
  log!("Creating a new core.");
  WasmCore::new(1000, 1000)
}
