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
  let input = String::from("# Bouncing Balls

Define the environment
  #html/event/click = [x: 123 y: 456]
  x = 1:5
  v = x * 0
  #ball = [|x y vx vy| x x v v]
  #system/timer = [resolution: 15, tick: 0]
  #gravity = 2
  #boundary = 420

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
  #ball.vy{iy} := -#ball.vy

## Create More Balls

Create ball on click
  ~ #html/event/click.x
  x = #html/event/click.x
  y = #html/event/click.y
  #ball += [x: x, y: y, vx: 0, vy: 0]");

  compiler.compile_string(input);
  core.register_blocks(compiler.blocks.clone());
  //println!("{:?}", compiler.parse_tree);
  //println!("{:?}", compiler.syntax_tree);
  core.step();
  println!("{:?}", core);
  println!("{:?}", core.runtime); 
}