#![feature(alloc)]
#![feature(drain_filter)]
//extern crate mech_core;
extern crate wasm_bindgen;
extern crate hashbrown;
//extern crate web_sys;
#[macro_use]
extern crate alloc;
#[macro_use]
extern crate serde_derive;
extern crate core;
extern crate web_sys;

//use mech_syntax::compiler::Compiler;
use wasm_bindgen::prelude::*;
use hashbrown::hash_set::HashSet;
use alloc::vec::Vec;
use core::fmt;

macro_rules! log {
    ( $( $t:tt )* ) => {
        web_sys::console::log_1(&format!( $( $t )* ).into());
    }
}

// ## Modules

mod mechcore;
mod mechsyntax;

// ## Exported Modules

pub use self::mechcore::Core;
pub use self::mechsyntax::compiler::Compiler;

#[wasm_bindgen]
extern {
    fn alert(s: &str);
}

#[wasm_bindgen]
pub fn compile(input: String) {
  let mut core = Core::new(100, 100);
  let mut compiler = Compiler::new();
  compiler.compile_string(input);
  core.register_blocks(compiler.blocks.clone());
  //println!("{:?}", compiler.parse_tree);
  log!("{:?}", compiler.syntax_tree);
  core.step();
  log!("{:?}", core);
  log!("{:?}", core.runtime); 
}
