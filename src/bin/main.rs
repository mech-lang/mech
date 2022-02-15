extern crate mech_syntax;
extern crate mech_core;

use mech_syntax::compiler::{Compiler, Node, Element};
use mech_syntax::formatter::Formatter;
use mech_core::Block;
use mech_core::{Change, Transaction};
use mech_core::{Value, TableIndex};
use mech_core::hash_string;
use mech_core::Core;
use mech_core::{Quantity, ValueMethods, ToQuantity, QuantityMath};
use std::time::{Duration, SystemTime};
use std::mem;

use std::rc::Rc;

fn main() {

  // Some primitives
  let input = String::from(r#"
whatever you want here
  x = 1:10
  y = 11:20
  #z = [x y]"#);

  //compile_test(input.clone(), value);

  let mut compiler = Compiler::new();
  let mut formatter = Formatter::new();
  let mut core = Core::new(1_000_000, 20);
  core.load_standard_library();
  let programs = compiler.compile_string(input.clone());

  //println!("{:?}", programs);
  //println!("{:?}", compiler.blocks);
  //println!("{:?}", compiler.parse_tree);
  println!("{:?}", compiler.syntax_tree);
  for block in &compiler.blocks {
    println!("{:?}", block);
  }
  core.runtime.register_blocks(programs[0].blocks.clone());
  //core.runtime.register_block(compiler.blocks[0].clone());
  //core.runtime.register_block(compiler.blocks[1].clone());
  //core.runtime.register_block(compiler.blocks[2].clone());
  //core.runtime.register_block(compiler.blocks[3].clone());
  core.step();
  println!("{:?}", core);

}
