extern crate mech_syntax;
extern crate mech_core;

use mech_syntax::lexer::Lexer;
use mech_syntax::parser::{Parser, ParseStatus, Node};
use mech_syntax::compiler::Compiler;
use mech_core::Block;
use mech_core::{Change, Transaction};
use mech_core::{Value};
use mech_core::Hasher;
use mech_core::Core;

fn main() {
  let mut core = Core::new(100, 100);
  let mut compiler = Compiler::new();
  let input = String::from("

block
  #start = [x: 3]

block
  ~ #start.x
  x = 1:10
  xp = x * 0 + 3
  y = 11:20
  #z = [xp y]
  #qrs += [y: 33]
  
block
  #test = #z{1,1} + #z{1,2} + #z{2,1} + #z{1,1}

block
  #qrs = [y|1]
");

  compiler.compile_string(input);
  core.register_blocks(compiler.blocks.clone());
  //println!("{:?}", compiler.parse_tree);
  //println!("{:?}", compiler.syntax_tree);
  core.step();
  println!("{:?}", core);
  println!("{:?}", core.runtime); 
}