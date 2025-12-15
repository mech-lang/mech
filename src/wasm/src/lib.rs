#![allow(warnings)]

use wasm_bindgen::prelude::*;
use mech_core::*;
use mech_syntax::*;
use mech_interpreter::*;
use wasm_bindgen::JsCast;
use web_sys::{window, HtmlElement, HtmlInputElement, Node, Element, HashChangeEvent, HtmlTextAreaElement, Url};
use js_sys::decode_uri_component;
use std::rc::Rc;
use std::cell::RefCell;
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

#[macro_export]
macro_rules! log {
  ( $( $t:tt )* ) => {
    web_sys::console::log_1(&format!( $( $t )* ).into());
  }
}


fn new_interpreter(id: u64) -> Interpreter {

  let mut intrp = Interpreter::new(id);

  let fxns_ref = intrp.functions();
  let mut fxns = fxns_ref.borrow_mut();
      
  // Preload combinatorics functions
  #[cfg(feature = "combinatorics_n_choose_k")]
  fxns.function_compilers.insert(hash_str("combinatorics/n-choose-k"), &CombinatoricsNChooseK{});

  
  // Preload stats functions
  #[cfg(feature = "stats_sum")]
  fxns.function_compilers.insert(hash_str("stats/sum/row"), &StatsSumRow{});
  #[cfg(feature = "stats_sum")]
  fxns.function_compilers.insert(hash_str("stats/sum/column"), &StatsSumColumn{});

  // Preload ops functions
  #[cfg(feature = "math_add")]
  fxns.function_compilers.insert(hash_str("math/add"), &MathAdd{});
  #[cfg(feature = "math_sub")]
  fxns.function_compilers.insert(hash_str("math/sub"), &MathSub{});
  #[cfg(feature = "math_mul")]
  fxns.function_compilers.insert(hash_str("math/mul"), &MathMul{});
  #[cfg(feature = "math_div")]
  fxns.function_compilers.insert(hash_str("math/div"), &MathDiv{});
  #[cfg(feature = "math_mod")]
  fxns.function_compilers.insert(hash_str("math/mod"), &MathMod{});
  #[cfg(feature = "math_pow")]
  fxns.function_compilers.insert(hash_str("math/pow"), &MathPow{});
  #[cfg(feature = "math_neg")]
  fxns.function_compilers.insert(hash_str("math/neg"), &MathNegate{});
  
  // Preload math functions
  #[cfg(feature = "math_sqrt")]
  fxns.function_compilers.insert(hash_str("math/sqrt"), &MathSqrt{});
  
  // Preload trig functions
  #[cfg(feature = "math_sin")]
  fxns.function_compilers.insert(hash_str("math/sin"), &MathSin{});
  #[cfg(feature = "math_cos")]
  fxns.function_compilers.insert(hash_str("math/cos"), &MathCos{});
  #[cfg(feature = "math_atan2")]
  fxns.function_compilers.insert(hash_str("math/atan2"), &MathAtan2{});
  #[cfg(feature = "math_atan")]
  fxns.function_compilers.insert(hash_str("math/atan"), &MathAtan{});
  #[cfg(feature = "math_acos")]
  fxns.function_compilers.insert(hash_str("math/acos"), &MathAcos{});
  #[cfg(feature = "math_acosh")]
  fxns.function_compilers.insert(hash_str("math/acosh"), &MathAcosh{});
  #[cfg(feature = "math_acot")]
  fxns.function_compilers.insert(hash_str("math/acot"), &MathAcot{});
  #[cfg(feature = "math_acsc")]
  fxns.function_compilers.insert(hash_str("math/acsc"), &MathAcsc{});
  #[cfg(feature = "math_asec")]
  fxns.function_compilers.insert(hash_str("math/asec"), &MathAsec{});
  #[cfg(feature = "math_asin")]
  fxns.function_compilers.insert(hash_str("math/asin"), &MathAsin{});
  #[cfg(feature = "math_sinh")]
  fxns.function_compilers.insert(hash_str("math/sinh"), &MathSinh{});
  #[cfg(feature = "math_cosh")]
  fxns.function_compilers.insert(hash_str("math/cosh"), &MathCosh{});
  #[cfg(feature = "math_tanh")]
  fxns.function_compilers.insert(hash_str("math/tanh"), &MathTanh{});
  #[cfg(feature = "math_atanh")]
  fxns.function_compilers.insert(hash_str("math/atanh"), &MathAtanh{});
  #[cfg(feature = "math_cot")]
  fxns.function_compilers.insert(hash_str("math/cot"), &MathCot{});
  #[cfg(feature = "math_csc")]
  fxns.function_compilers.insert(hash_str("math/csc"), &MathCsc{});
  #[cfg(feature = "math_sec")]
  fxns.function_compilers.insert(hash_str("math/sec"), &MathSec{});
  #[cfg(feature = "math_tan")]
  fxns.function_compilers.insert(hash_str("math/tan"), &MathTan{});

  // Preload io functions
  //#[cfg(feature = "io_print")]
  //fxns.function_compilers.insert(hash_str("io/print"), &IoPrint{});
  //#[cfg(feature = "io_println")]
  //fxns.function_compilers.insert(hash_str("io/println"), &IoPrintln{});

  // Matrix functions
  #[cfg(feature = "matrix_horzcat")]
  fxns.function_compilers.insert(hash_str("matrix/horzcat"), &MatrixHorzCat{});
  #[cfg(feature = "matrix_vertcat")]
  fxns.function_compilers.insert(hash_str("matrix/vertcat"), &MatrixVertCat{});
  #[cfg(feature = "matrix_transpose")]
  fxns.function_compilers.insert(hash_str("matrix/transpose"), &MatrixTranspose{});
  #[cfg(feature = "matrix_matmul")]
  fxns.function_compilers.insert(hash_str("matrix/matmul"), &MatrixMatMul{});
  #[cfg(feature = "matrix_dot")]
  fxns.function_compilers.insert(hash_str("matrix/dot"), &MatrixDot{});
  #[cfg(feature = "matrix_solve")]
  fxns.function_compilers.insert(hash_str("matrix/solve"), &MatrixSolve{});

  // Compare functions
  #[cfg(feature = "compare_eq")]
  fxns.function_compilers.insert(hash_str("compare/eq"), &CompareEqual{});
  #[cfg(feature = "compare_neq")]
  fxns.function_compilers.insert(hash_str("compare/neq"), &CompareNotEqual{});
  #[cfg(feature = "compare_lte")]
  fxns.function_compilers.insert(hash_str("compare/lte"), &CompareLessThanEqual{});
  #[cfg(feature = "compare_gte")]
  fxns.function_compilers.insert(hash_str("compare/gte"), &CompareGreaterThanEqual{});
  #[cfg(feature = "compare_lt")]
  fxns.function_compilers.insert(hash_str("compare/lt"), &CompareLessThan{});
  #[cfg(feature = "compare_gt")]
  fxns.function_compilers.insert(hash_str("compare/gt"), &CompareGreaterThan{});

  // Logic functions
  #[cfg(feature = "logic_and")]
  fxns.function_compilers.insert(hash_str("logic/and"), &LogicAnd{});
  #[cfg(feature = "logic_or")]
  fxns.function_compilers.insert(hash_str("logic/or"), &LogicOr{});
  #[cfg(feature = "logic_not")]
  fxns.function_compilers.insert(hash_str("logic/not"), &LogicNot{});
  #[cfg(feature = "logic_xor")]
  fxns.function_compilers.insert(hash_str("logic/xor"), &LogicXor{});

  // Set Functions
  #[cfg(feature = "set_union")]
  fxns.function_compilers.insert(hash_str("set/union"), &SetUnion{});
  #[cfg(feature = "set_intersection")]
  fxns.function_compilers.insert(hash_str("set/intersection"), &SetIntersection{});
  #[cfg(feature = "set_difference")]
  fxns.function_compilers.insert(hash_str("set/difference"), &SetDifference{});
  #[cfg(feature = "set_subset")]
  fxns.function_compilers.insert(hash_str("set/subset"), &SetSubset{});
  #[cfg(feature = "set_superset")]
  fxns.function_compilers.insert(hash_str("set/superset"), &SetSuperset{});
  #[cfg(feature = "set_proper_subset")]
  fxns.function_compilers.insert(hash_str("set/proper-subset"), &SetProperSubset{});
  #[cfg(feature = "set_proper_superset")]
  fxns.function_compilers.insert(hash_str("set/proper-superset"), &SetProperSuperset{});
  #[cfg(feature = "set_element_of")]
  fxns.function_compilers.insert(hash_str("set/element-of"), &SetElementOf{});
  #[cfg(feature = "set_not_element_of")]
  fxns.function_compilers.insert(hash_str("set/not-element-of"), &SetNotElementOf{});

  intrp

}

#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
  //let mut wasm_mech = WasmMech::new();
  //wasm_mech.init();
  //wasm_mech.run_program("1 + 1");
  Ok(())
}

#[cfg(feature = "eval")]
fn run_mech_code(intrp: &mut Interpreter, code: &Vec<(String,MechSourceCode)>) -> MResult<Value> {
  for (file, source) in code {
    match source {
      MechSourceCode::String(s) => {
        let parse_result = parser::parse(&s.trim());
        match parse_result {
          Ok(tree) => { 
            let result = intrp.interpret(&tree);
            return result;
          },
          Err(err) => return Err(err),
        }
      }
      x => {
        log!("Unsupported source code type: {:?}", x);
        todo!();
      },
    }
  }
  Ok(Value::Empty)
}

#[wasm_bindgen]
pub struct WasmMech {
  interpreter: Interpreter,
  repl_history: Vec<String>,
  repl_history_index: Option<usize>,
  repl_id: Option<String>,
}

#[wasm_bindgen]
impl WasmMech {

  #[wasm_bindgen(constructor)]
  pub fn new() -> Self {
    Self { 
      interpreter: new_interpreter(0),
      repl_history: Vec::new(), 
      repl_history_index: None,
      repl_id: None,
    }
  }

  #[wasm_bindgen]
  pub fn out_string(&self) -> String {
    self.interpreter.out.to_string()
  }

  #[wasm_bindgen]
  pub fn clear(&mut self) {
    self.interpreter = new_interpreter(0);
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
          format!("Unrecognized command: {}", x)
        }
      }
      #[cfg(not(feature = "repl"))]
      {
        "REPL commands not supported. Rebuild with the 'repl' feature.".to_string()
      }
    } else {
      let cmd = vec![("repl".to_string(),MechSourceCode::String(input.to_string()))];
      CURRENT_MECH.with(|mech_ref| {
        if let Some(ptr) = *mech_ref.borrow() {
          unsafe {
            let mut mech = &mut *ptr;
            match run_mech_code(&mut mech.interpreter, &cmd)  {
              Ok(output) => { 
                let kind_str = html_escape(&format!("{}",output.kind()));
                return format!("<div class=\"mech-output-kind\">{}</div><div class=\"mech-output-value\">{}</div>", kind_str, output.to_html());
              },
              Err(err) => { return format!("{:?}",err); }
            }
          }
        }
        "Error: No interpreter found.".to_string()
      })
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
      let element_id = parsed_id[0].parse::<u64>().unwrap();
      let interpreter_id = parsed_id[1].parse::<u64>().unwrap();

      let symbols = match interpreter_id {
        0 => self.interpreter.symbols(),
        id => match self.interpreter.sub_interpreters.borrow().get(&id) {
          Some(sub_interpreter) => sub_interpreter.symbols(),
          None => {
            log!("No sub interpreter found for id: {}", id);
            continue;
          }
        }
      };

      // Create click closure
      let closure = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        let mech_output = document.get_element_by_id("mech-output").unwrap();
        let last_child = mech_output.last_child();

        let symbols_brrw = symbols.borrow();

        match symbols_brrw.get(element_id) {
          Some(output) => {
            let output_brrw = output.borrow();

            let kind_str = html_escape(&format!("{}", output_brrw.kind()));
            let result_html = format!(
              "<div class=\"mech-output-kind\">{}</div><div class=\"mech-output-value\">{}</div>",
              kind_str,
              output_brrw.to_html()
            );

            let symbol_name = symbols_brrw.get_symbol_name_by_id(element_id).unwrap();

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
            CURRENT_MECH.with(|mech_ref| {
              if let Some(ptr) = *mech_ref.borrow() {
                unsafe { (*ptr).repl_history.push(symbol_name.clone()); }
              }
            });

            // If REPL is "closed", show modal at click location
            let repl_width = mech_output.client_width();
            if repl_width == 0 {
              let modal = document.create_element("div").unwrap();
              modal.set_class_name("mech-modal");
              modal.set_inner_html(&format!(
                "<div class=\"mech-output-kind\">{}</div><div class=\"mech-output-value\">{}</div>",
                kind_str,
                output_brrw.to_html()
              ));

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
            }

          },
          None => {
            let error_message = format!("No value found for element id: {}", element_id);
            let result_line = document.create_element("div").unwrap();
            result_line.set_class_name("repl-result");
            result_line.set_inner_html(&error_message);
            if let Some(last_child) = last_child {
              mech_output.insert_before(&result_line, Some(&last_child)).unwrap();
            } else {
              mech_output.append_child(&result_line).unwrap();
            }
          }
        }

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
        let sub_interpreter_brrw = self.interpreter.sub_interpreters.borrow();
        let intrp = match interpreter_id {
          0 => &self.interpreter, 
          id => {
            match sub_interpreter_brrw.get(&id) {
              Some(sub_interpreter) => sub_interpreter,
              None => {
                log!("No sub interpreter found for id: {}", id);
                continue;
              }
            }
          }
        };

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
            let out_values = match interpreter_id {
              // if the interpreter id is 0, we are in the main interpreter
              0 => intrp.out_values.clone(), 
              // if the interpreter id is not 0, we are in a sub interpreter
              id => {
                match intrp.sub_interpreters.borrow().get(&id) {
                  Some(sub_interpreter) => sub_interpreter.out_values.clone(),
                  None => {
                    log!("No sub interpreter found for id: {}", id);
                    continue;
                  }
                }
              }
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
            let kind_str = html_escape(&format!("{}",output.kind()));
            let formatted_output = format!("<div class=\"mech-output-kind\">{}</div><div class=\"mech-output-value\">{}</div>", kind_str, output.to_html());
            block.set_inner_html(&formatted_output);
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

  #[cfg(feature = "run_program")]
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
        let mut doc_intrp = new_interpreter(doc_hash);
        let doc_result = doc_intrp.interpret(&tree);
        let output_element = document.get_element_by_id(&element_id).expect("REPL output element not found");
        // Get the second to last element of mech-output. It should be a repl-result from when teh user pressed enter.
        // Set the inner html of the repl result element to be the formatted doc.
        let children = output_element.children();
        let len = children.length();
        if len >= 2 {
            let repl_result = children.item(len - 2).expect("Failed to get second-to-last child");
            repl_result.set_attribute("mech-interpreter-id", &format!("{}",doc_hash)).unwrap();
            let repl_html = repl_result.dyn_ref::<HtmlElement>().expect("Expected an HtmlElement");
            repl_html.class_list().add_1("compact").unwrap();
            repl_html.set_inner_html(&doc_html);
            CURRENT_MECH.with(|mech_ref| {
              if let Some(ptr) = *mech_ref.borrow() {
                unsafe {
                  let mut mech = &mut *ptr;
                  mech.interpreter.sub_interpreters.borrow_mut().insert(doc_hash, Box::new(doc_intrp));
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