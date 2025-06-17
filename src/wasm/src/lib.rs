use wasm_bindgen::prelude::*;
use mech_core::*;
use mech_syntax::*;
use mech_interpreter::*;
use wasm_bindgen::JsCast;
use web_sys::{HtmlElement, HtmlInputElement};
use std::rc::Rc;
use std::cell::RefCell;

thread_local! {
    static CURRENT_MECH: RefCell<Option<*mut WasmMech>> = RefCell::new(None);
}

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

  #[wasm_bindgen]
  pub fn attach_repl(&mut self, repl_id: &str) {
    CURRENT_MECH.with(|c| *c.borrow_mut() = Some(self as *mut _));
    let window = web_sys::window().expect("global window does not exists");    
    let document = window.document().expect("should have a document");
    let container = document
      .get_element_by_id(repl_id)
      .expect("REPL element not found")
      .dyn_into::<HtmlElement>()
      .expect("Element should be HtmlElement");

    let create_prompt: Rc<RefCell<Option<Box<dyn Fn()>>>> = Rc::new(RefCell::new(None));
    let create_prompt_clone = create_prompt.clone();
    let document_clone = document.clone();
    let container_clone = container.clone();

    *create_prompt.borrow_mut() = Some(Box::new(move || {
      let line = document_clone.create_element("div").unwrap();
      line.set_class_name("repl-line");

      let prompt = document_clone.create_element("span").unwrap();
      prompt.set_inner_html("&gt;: ");
      prompt.set_class_name("repl-prompt");

      let input = document_clone.create_element("input")
                                .unwrap()
                                .dyn_into::<HtmlInputElement>()
                                .unwrap();
      let input_for_closure = input.clone();
      input.set_class_name("repl-input");
      input.unchecked_ref::<HtmlElement>()
            .set_autofocus(true);
            
      line.append_child(&prompt).unwrap();
      line.append_child(&input).unwrap();
      container_clone.append_child(&line).unwrap();
      let _ = input.focus();
            
      let document_inner = document_clone.clone();
      let container_inner = container_clone.clone();
      let create_prompt_inner = create_prompt_clone.clone();

      let closure = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
        if event.key() == "Enter" {
          let code = input_for_closure.value();

          // Replace input field with text
          let input_parent = input_for_closure
            .parent_node()
            .expect("input should have a parent");

          let input_span = document_inner.create_element("span").unwrap();
          input_span.set_class_name("repl-code");
          input_span.set_text_content(Some(&code));

          // Replace the input element in the DOM
          input_parent
            .replace_child(&input_span, &input_for_closure)
            .unwrap();

          let _ = input_for_closure.focus();


          if code.trim().is_empty() {
            return;
          }

          let result_line = document_inner.create_element("div").unwrap();
          result_line.set_class_name("repl-result");

          // SAFELY call back into WasmMech
          let output = CURRENT_MECH.with(|mech_ref| {
            if let Some(ptr) = *mech_ref.borrow() {
              // UNSAFE but valid: we trust that `self` lives
              unsafe {
                (*ptr).eval(&code)
              }
            } else {
              "[no interpreter]".to_string()
            }
          });

          result_line.set_inner_html(&output);
          container_inner.append_child(&result_line).unwrap();

          if let Some(cb) = &*create_prompt_inner.borrow() {
            cb();
          }
        }
      }) as Box<dyn FnMut(_)>);

      input.add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref())
            .unwrap();
      closure.forget();
    }));

    if let Some(cb) = &*create_prompt.borrow() {
      cb();
    };
  }

  pub fn eval(&mut self, input: &str) -> String {
    // Parse the input code
    match parse(input) {
      Ok(tree) => {
        // Interpret the parsed tree
        match self.interpreter.interpret(&tree) {
          Ok(result) => {
            log!("{}", result.pretty_print());
            result.pretty_print()
          },
          Err(err) => {
            log!("{:?}", err);
            format!("Error: {:?}", err)
          }
        }
      },
      Err(parse_err) => {
        log!("Error parsing program: {:?}", parse_err);
        format!("Parse Error: {:?}", parse_err)
      }
    }
  }

  

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
  }

  // Write block output each element that needs it, rendering it appropriately
  // based on its data type.
  #[wasm_bindgen]
  pub fn render_codeblock_output_values(&mut self) {
    let window = web_sys::window().expect("global window does not exists");    
		let document = window.document().expect("expecting a document on window"); 
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
  }

  #[wasm_bindgen]
  pub fn render_inline_values(&mut self) {
    let window = web_sys::window().expect("global window does not exists");    
		let document = window.document().expect("expecting a document on window"); 
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