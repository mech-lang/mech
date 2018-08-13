extern crate core;
extern crate time;
extern crate rand;
extern crate mech_syntax;
extern crate mech_core;

use std::time::SystemTime;
use std::thread::{self};
use std::time::*;
use rand::{Rng, thread_rng};
use mech_syntax::lexer::Lexer;
use mech_syntax::parser::{Parser, ParseStatus, Node};
use mech_syntax::compiler::Compiler;
use mech_core::Block;
use mech_core::{Change, Transaction};
use mech_core::{Value};
use mech_core::Hasher;
use mech_core::Core;

fn main() {
  println!("system/timer ({:#x})", Hasher::hash_str("system/timer"));
  let mut compiler = Compiler::new();
  let mut core = Core::new(10, 100);
  let input = String::from("# Bouncing Balls

Define the environment
  #y = [x: 1]
  #ball = [x: 15 y: 10]
  
Set ball to click
  ~ #y.x
  #ball += [x: 2 y: 3]
  #ball += [x: 4 y: 5]");
  compiler.compile_string(input);
  core.register_blocks(compiler.blocks);
  //println!("{:?}", compiler.parse_tree);
  println!("{:?}", compiler.syntax_tree);
  core.step();
  println!("{:?}", core);
  println!("{:?}", core.runtime);
  
  
}   