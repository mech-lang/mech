//extern crate mech_core;
extern crate wasm_bindgen;
extern crate hashbrown;
//extern crate web_sys;

//use mech_syntax::compiler::Compiler;
use wasm_bindgen::prelude::*;
use hashbrown::hash_set::HashSet;


/*macro_rules! log {
    ( $( $t:tt )* ) => {
        web_sys::console::log_1(&format!( $( $t )* ).into());
    }
}*/

pub struct Core {
  pub id: u64,
  pub epoch: usize,
  pub offset: usize, // this is an offset from now. 0 means now, 1 means 1 txn ago, etc.
  pub round: usize,
  pub changes: usize,
  pub change_capacity: usize,
  pub table_capacity: usize,
  pub input: HashSet<usize>,
  pub output: HashSet<usize>,
  transaction_boundaries: Vec<usize>,
}

#[wasm_bindgen]
extern {
    fn alert(s: &str);
}

#[wasm_bindgen]
pub fn greet(input: String) {
    //let mut compiler = Compiler::new();
    //compiler.compile_string(input);
    //log!("HEllo world WORKING!");
}
