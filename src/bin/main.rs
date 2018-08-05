extern crate core;
extern crate time;
extern crate rand;
extern crate mech_syntax;
extern crate mech;

use std::time::SystemTime;
use std::thread::{self};
use std::time::*;
use rand::{Rng, thread_rng};
use mech_syntax::lexer::Lexer;
use mech_syntax::parser::{Parser, ParseStatus, Node};
use mech_syntax::compiler::Compiler;
use mech::Block;
use mech::{Change, Transaction};
use mech::{Value};
use mech::Hasher;
use mech::Core;

fn main() {
  println!("system/timer ({:#x})", Hasher::hash_str("system/timer"));
  let mut compiler = Compiler::new();
  let mut core = Core::new(10, 100);
  let input = String::from("
block 
  #x = [x: 5001 y: 456]
  #boundary = 5000
block
  ix = #x.x > #boundary
  iy = #x.y > #boundary");
  compiler.compile_string(input);
  core.register_blocks(compiler.blocks);
  println!("{:?}", compiler.parse_tree);
  println!("{:?}", compiler.syntax_tree);
  core.step();
  println!("{:?}", core);
  println!("{:?}", core.runtime);
  
  
}   