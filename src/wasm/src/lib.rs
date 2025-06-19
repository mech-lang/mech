use wasm_bindgen::prelude::*;
use mech_core::*;
use mech_syntax::*;
use mech_interpreter::*;
use wasm_bindgen::JsCast;
use web_sys::{window, HtmlElement, HtmlInputElement, Node};
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
  repl_history: Vec<String>,
  repl_history_index: Option<usize>,
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
    let mech_output = container.clone();
    let mech_output_for_event = mech_output.clone();


    let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
      if let Some(input) = mech_output
        .owner_document()
        .unwrap()
        .get_element_by_id("repl-active-input")
      {
        let _ = input
          .dyn_ref::<web_sys::HtmlElement>()
          .unwrap()
          .focus();
      }
    }) as Box<dyn FnMut(_)>);
    mech_output_for_event.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref()).unwrap();
    closure.forget();

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
      input.set_id("repl-active-input");
      input.unchecked_ref::<HtmlElement>().set_autofocus(true);
            
      line.append_child(&prompt).unwrap();
      line.append_child(&input).unwrap();
      container_clone.append_child(&line).unwrap();
      let _ = input.focus();
            
      let document_inner = document_clone.clone();
      let container_inner = container_clone.clone();
      let create_prompt_inner = create_prompt_clone.clone();

      let closure = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
        match event.key().as_str() {
          "Enter" => {
            let code = input_for_closure.value();

            // Replace input field with text
            let input_parent = input_for_closure.parent_node().expect("input should have a parent");

            let input_span = document_inner.create_element("span").unwrap();
            input_span.set_class_name("repl-code");
            input_span.set_text_content(Some(&code));

            // Replace the input element in the DOM
            input_parent.replace_child(&input_span, &input_for_closure).unwrap();

            let _ = input_for_closure.focus();
            input_for_closure.set_id("repl-active-input");

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
                  let mech = &mut *ptr;
                  mech.repl_history.push(code.clone());
                  mech.repl_history_index = None;
                  mech.eval(&code)
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
          "ArrowUp" => {
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
                      input_for_closure.set_value(&mech.repl_history[i]);
                      mech.repl_history_index = Some(i);
                    }
                  }
                }
              }
            });
          }
          "ArrowDown" => {
            event.prevent_default(); // prevent cursor jump
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
                      input_for_closure.set_value(&mech.repl_history[i]);
                      mech.repl_history_index = Some(i);
                    } else {
                      input_for_closure.set_value("");
                      mech.repl_history_index = None;
                    }
                  }
                }
              }
            });
          }
          _ => (),
        }
      }) as Box<dyn FnMut(_)>);

      input.add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref()).unwrap();
      closure.forget();
    }));

    if let Some(cb) = &*create_prompt.borrow() {
      cb();
    };
  }

  fn run_mech_code(&mut self, code: &Vec<(String,MechSourceCode)>) -> MResult<Value> {
    for (file, source) in code {
      match source {
        MechSourceCode::String(s) => {
          let parse_result = parser::parse(&s.trim());
          match parse_result {
            Ok(tree) => { 
              let result = self.interpreter.interpret(&tree);
              return result;
            },
            Err(err) => return Err(err),
          }
        }
        _ => todo!(),
      }
    }
    Ok(Value::Empty)
  }

  fn execute_repl_command(&mut self, repl_cmd: ReplCommand) -> String {
    let mut intrp = &mut self.interpreter;
    match repl_cmd {
      ReplCommand::Clear(_) => {
        *intrp = Interpreter::new(intrp.id);
        "".to_string()
      }
      ReplCommand::Clc => {
        let window = web_sys::window().expect("global window does not exists");    
        let document = window.document().expect("expecting a document on window");
        let output_element = document.get_element_by_id("mech-output").expect("REPL output element not found");
        // Remove all children except the last one
        while output_element.child_nodes().length() > 0 {
          let first_child = output_element
            .first_child()
            .expect("Expected a child node");
          output_element
            .remove_child(&first_child)
            .expect("Failed to remove child");
        }
        "".to_string()
      }
      ReplCommand::Code(code) => {
        match self.run_mech_code(&code)  {
          Ok(output) => { 
            return format!("<div class=\"mech-output-kind\">{:?}</div><div class=\"mech-output-value\">{}</div>", output.kind(), output.to_html());
          },
          Err(err) => { return format!("{:?}",err); }
        }
      }
      _ => todo!("Implement other REPL commands"),
    }
  }

  pub fn eval(&mut self, input: &str) -> String {
    if input.chars().nth(0) == Some(':') {
      match parse_repl_command(&input.to_string()) {
        Ok((_, repl_command)) => {
          self.execute_repl_command(repl_command)
        }
        Err(x) => {
          format!("Unrecognized command: {}", x)
        }
      }
    } else if input.trim() == "" {
      "".to_string()
    } else {
      let cmd = ReplCommand::Code(vec![("repl".to_string(),MechSourceCode::String(input.to_string()))]);
      self.execute_repl_command(cmd)
    }
  }

  

  #[wasm_bindgen(constructor)]
  pub fn new() -> Self {
    Self { 
      interpreter: Interpreter::new(0),
      repl_history: Vec::new(), 
      repl_history_index: None,
    }
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
        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        let mech_output = document.get_element_by_id("mech-output").unwrap();
        let last_child = mech_output.last_child().unwrap();
        let symbols_brrw = symbols.borrow();

        match symbols_brrw.get(element_id) {
          Some(output) => {
            let output_brrw = output.borrow();
            let result_html = format!(
              "<div class=\"mech-output-kind\">{:?}</div><div class=\"mech-output-value\">{}</div>",
              output_brrw.kind(),
              output_brrw.to_html()
            );

            let symbol_name = symbols_brrw.get_symbol_name_by_id(element_id).unwrap();

            let prompt_line = document.create_element("div").unwrap();
            prompt_line.set_class_name("repl-line");
            let prompt_span = document.create_element("span").unwrap();
            prompt_span.set_class_name("repl-prompt");
            prompt_span.set_inner_html("&gt;: ");
            prompt_line.append_child(&prompt_span).unwrap();
            let input_span = document.create_element("span").unwrap();
            input_span.set_class_name("repl-code");
            input_span.set_inner_html(&symbol_name);
            prompt_line.append_child(&input_span).unwrap();
            mech_output.insert_before(&prompt_line, Some(&last_child)).unwrap();

            let result_line = document.create_element("div").unwrap();
            result_line.set_class_name("repl-result");
            result_line.set_inner_html(&result_html);
            mech_output.insert_before(&result_line, Some(&last_child)).unwrap();

            let output = CURRENT_MECH.with(|mech_ref| {
              if let Some(ptr) = *mech_ref.borrow() {
                unsafe {
                  (*ptr).repl_history.push(symbol_name.clone());
                }
              } else {
                log!("[no interpreter]");
              }
            });

          },
          None => {
            let error_message = format!("No value found for element id: {}", element_id);
            let result_line = document.create_element("div").unwrap();
            result_line.set_class_name("repl-result");
            result_line.set_inner_html(&error_message);
            mech_output.insert_before(&result_line, Some(&last_child)).unwrap();
          }
        }
        mech_output.set_scroll_top(mech_output.scroll_height());
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