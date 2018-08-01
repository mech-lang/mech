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
  let mut compiler = Compiler::new();
  let mut core = Core::new(10, 100);
  let input = String::from("# Bouncing Balls

Set up the environment
  #ball = [x: 15 y: 0 vx: 1 vy: 0 ]

## Update Condition

The positions of the balls update on every tick
  #y = #ball.x + 10");
  compiler.compile_string(input);
  core.register_blocks(compiler.blocks);
  core.step();
  //println!("{:?}", compiler.parse_tree);
  println!("{:?}", compiler.syntax_tree);
  println!("{:?}", core);
  println!("{:?}", core.runtime);
  
}   