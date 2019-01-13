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
  let mut core = Core::new(100, 100);
  let mut compiler = Compiler::new();
  let input = String::from("# Bouncing Balls

Define the environment
  #html/event/click = [x: 0 y: 0]
  #ball = [x: 15 y: 9 vx: 40 vy: 9]
  #system/timer = [resolution: 15, tick: 0]
  #gravity = 2
  #boundary = 5000

## Update condition

Now update the block positions
  ~ #system/timer.tick
  #ball.x := #ball.x + #ball.vx
  #ball.y := #ball.y + #ball.vy
  #ball.vy := #ball.vy + #gravity

## Boundary Condition

Keep the balls within the y boundary
  ~ #system/timer.tick
  iy = #ball.y > #boundary
  #ball.y{iy} := #boundary
  #ball.vy{iy} := #ball.vy * 80

Keep the balls within the x boundary
  ~ #system/timer.tick
  ix = #ball.x > #boundary
  ixx = #ball.x < 0
  #ball.x{ix} := #boundary
  #ball.x{ixx} := 0
  #ball.vx{ix | ixx} := #ball.vx * 80

## Create More Balls

Create ball on click
  ~ #html/event/click.x
  #ball += [x: 10 y: 10 vx: 40 vy: 0]
  
block
  #test = #ball{1,2");

  compiler.compile_string(input);
  core.register_blocks(compiler.blocks.clone());
  //println!("{:?}", compiler.parse_tree);
  //println!("{:?}", compiler.syntax_tree);
  core.step();
  println!("{:?}", core);
  //println!("{:?}", core.runtime); 
}