use wasm_bindgen::prelude::*;
use mech_core::*;
use mech_syntax::*;
use mech_interpreter::*;

#[macro_export]
macro_rules! log {
  ( $( $t:tt )* ) => {
    web_sys::console::log_1(&format!( $( $t )* ).into());
  }
}

#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
  //let mut wasm_mech = WasmMech::new();
  //wasm_mech.init();
  //wasm_mech.run_program("1 + 1");
  Ok(())
}

#[wasm_bindgen]
pub struct WasmMech {
  interpreter: Interpreter,
}

#[wasm_bindgen]
impl WasmMech {

  #[wasm_bindgen(constructor)]
  pub fn new() -> Self {
    Self { interpreter: Interpreter::new() }
  }
  
  #[wasm_bindgen]
  pub fn init(&self) {
    let window = web_sys::window().expect("global window does not exists");    
		let document = window.document().expect("expecting a document on window");
  
    let clickable_elements = document.get_elements_by_tag_name("clickable");
    for i in 0..clickable_elements.length() {
      let element = clickable_elements.get_with_index(i).unwrap();
      let div_element: web_sys::HtmlDivElement = element
                    .dyn_into::<web_sys::HtmlDivElement>()
                    .map_err(|_| ())
                    .unwrap();
      log!("{:?}", div_element);
    }
  }

  #[wasm_bindgen]
  pub fn run_program(&mut self, src: &str) { 
    // Decompress the string into a Program
    let tree: Program = decode_and_decompress(&src);
    match self.interpreter.interpret(&tree) {
      Ok(result) => {
        log!("{:?}", result.pretty_print());
      },
      Err(err) => {
        log!("{:?}", err);
      }
    }
  }

}