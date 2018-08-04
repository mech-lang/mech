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
  let input = String::from("# Bouncing Balls
Define the environment
  #ball = [x: 15 y: 9 vx: 18 vy: 9]
  #system/timer = [tick: 1]
  #gravity = 98

Now update the block positions
  ~ #system/timer.tick
  #ball.x := #ball.x + #ball.vx
  #ball.y := #ball.y + #ball.vy
  #ball.vy := #ball.vy + #gravity");
  compiler.compile_string(input);
  core.register_blocks(compiler.blocks);
  println!("{:?}", compiler.parse_tree);
  println!("{:?}", compiler.syntax_tree);
  core.step();
  println!("{:?}", core);
  println!("{:?}", core.runtime);
  
}   