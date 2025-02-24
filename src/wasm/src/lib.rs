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
  let mut intrp = Interpreter::new();
  let parse_result = parser::parse("1 + 2");
  match parse_result {
    Ok(tree) => { 
      let result = intrp.interpret(&tree);
      log!("{:?}", result);
    },
    Err(err) => {
      if let MechErrorKind::ParserError(report, _) = err.kind {
        //parser::print_err_report(&s, &report);
      } else {
        //panic!("Unexpected error type");
      }
    }
  }
  Ok(())
}

#[wasm_bindgen]
pub fn run_program(src: &str) { 
  // Decompress the string into a Program
  let tree: Program = decode_and_decompress(&src);
  let mut intrp = Interpreter::new();
  match intrp.interpret(&tree) {
    Ok(result) => {
      log!("{:?}", result.pretty_print());
    },
    Err(err) => {
      log!("{:?}", err);
    }
  }
}
