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
    Self { interpreter: Interpreter::new(0) }
  }

  #[wasm_bindgen]
  pub fn out_string(&self) -> String {
    self.interpreter.out.to_string()
  }

  #[wasm_bindgen]
  pub fn clear(&mut self) {
    self.interpreter = Interpreter::new(0);
  }
  
  #[wasm_bindgen]
  pub fn init(&self) {
    let window = web_sys::window().expect("global window does not exists");    
		let document = window.document().expect("expecting a document on window");
    
    // Set up a click event listener for all elements with the class "mech-clickable"
    let clickable_elements = document.get_elements_by_class_name("mech-clickable");
    for i in 0..clickable_elements.length() {
      let element = clickable_elements.get_with_index(i).unwrap();

      // the element id is formed like this : let id = format!("{}:{}",hash_str(&name),self.interpreter_id);
      // so we need to parse it to get the id and the interpreter id
      let id = element.id();
      let parsed_id: Vec<&str> = id.split(":").collect();
      let element_id = parsed_id[0].parse::<u64>().unwrap();
      let interpreter_id = parsed_id[1].parse::<u64>().unwrap();
      let symbols = match interpreter_id {
        // if the interpreter id is 0, we are in the main interpreter
        0 => self.interpreter.symbols(), 
        // if the interpreter id is not 0, we are in a sub interpreter
        id => self.interpreter.sub_interpreters.borrow().get(&id).unwrap().symbols(),
      };
      
      let closure = Closure::wrap(Box::new(move || {
        match symbols.borrow().get(element_id) {
          Some(value) => {
            log!("{}", value.borrow().pretty_print());
          },
          None => {
            log!("No value found for element id: {}", element_id);
          }
        }
      }) as Box<dyn Fn()>);
  
      element.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref());
      closure.forget();
    }

    // Write block output each element that needs it, rendering it appropriately
    // based on its data type.
    let output_elements = document.get_elements_by_class_name("mech-block-output");
    for i in 0..output_elements.length() {
      let block = output_elements.get_with_index(i).unwrap();
      // the id looks like this
      // output_id:interpreter_id
      // so we need to parse it to get the id and the interpreter id
      let id = block.id();
      let parsed_id: Vec<&str> = id.split(":").collect();
      let output_id = parsed_id[0].parse::<u64>().unwrap();
      let interpreter_id = parsed_id[1].parse::<u64>().unwrap();
      // get the interpreter id from the block id
      let out_values = match interpreter_id {
        // if the interpreter id is 0, we are in the main interpreter
        0 => self.interpreter.out_values.clone(), 
        // if the interpreter id is not 0, we are in a sub interpreter
        id => self.interpreter.sub_interpreters.borrow().get(&id).unwrap().out_values.clone(),
      };

      // get the output id from the block id
      let out_value_brrw = out_values.borrow();
      let output = match out_value_brrw.get(&output_id) {
        Some(value) => value,
        None => {
          log!("No value found for output id: {}", output_id);
          continue;
        }
      };

      // set the inner html of the block to the output value html
      let formatted_output = format!("<div class=\"mech-output-kind\">{:?}</div><div class=\"mech-output-value\">{}</div>", output.kind(), output.to_html());
      block.set_inner_html(&formatted_output);
    }

    let inline_elements = document.get_elements_by_class_name("mech-inline-mech-code");

    let out_values_brrw = self.interpreter.out_values.borrow();

    for j in 0..inline_elements.length() {
      let inline_block = inline_elements.get_with_index(j).unwrap();
      let inline_id = inline_block.id();
      let inline_id: u64 = inline_id.parse().unwrap();
      
      let inline_output = match out_values_brrw.get(&inline_id) {
        Some(value) => value,
        None => {
          log!("No value found for inline output id: {}", inline_id);
          continue;
        }
      };
      let formatted_output = format!("{}", inline_output.to_string());
      inline_block.set_inner_html(&formatted_output.trim());
    }

  }

  #[wasm_bindgen]
  pub fn run_program(&mut self, src: &str) { 
    // Decompress the string into a Program
    match decode_and_decompress(&src) {
      Ok(tree) => {
        match self.interpreter.interpret(&tree) {
          Ok(result) => {
            log!("{}", result.pretty_print());
          },
          Err(err) => {
            log!("{:?}", err);
          }
        }
      },
      Err(err) => {
        match parse(src) {
          Ok(tree) => {
            match self.interpreter.interpret(&tree) {
              Ok(result) => {
                log!("{}", result.pretty_print());
              },
              Err(err) => {
                log!("{:?}", err);
              }
            }
          },
          Err(parse_err) => {
            log!("Error parsing program: {:?}", parse_err);
          }
        }
        
      }
    }
  
  }

}