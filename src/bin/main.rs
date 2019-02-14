extern crate mech_syntax;
extern crate mech_core;

use mech_syntax::lexer::Lexer;
use mech_syntax::parser::{Parser, ParseStatus, Node};
use mech_syntax::compiler::Compiler;
use mech_core::Block;
use mech_core::{Change, Transaction};
use mech_core::{Value, Index};
use mech_core::Hasher;
use mech_core::Core;

fn main() {
  let mut core = Core::new(100, 100);
  let mut compiler = Compiler::new();
  let input = String::from("# Bouncing Balls

Define the environment
  #html/event/click = [|x y|]
  range = 1:5
  x = range * 30
  v = x * 0
  #ball = [|x y vx vy| x x v v]
  #system/timer = [resolution: 15, tick: 0]
  #gravity = 1
  #boundary-y = 820
  #boundary-x = 500

Update the block positions on each tick of the timer
  ~ #system/timer.tick
  #ball.x := #ball.x + #ball.vx
  #ball.y := #ball.y + #ball.vy
  #ball.vy := #ball.vy + #gravity

Keep the balls within the y boundary
  ~ #system/timer.tick
  iy = #ball.y > #boundary-y
  #ball.y{iy} := #boundary-y
  #ball.vy{iy} := -#ball.vy * 0.80");

  compiler.compile_string(input);
  core.register_blocks(compiler.blocks.clone());
  //println!("{:?}", compiler.parse_tree);
  //println!("{:?}", compiler.syntax_tree);
  core.step();
  println!("{:?}", core);
  //println!("{:?}", core.runtime); 

  for i in 0..296 {
    println!("--------------------------------------------");
    let table_id = Hasher::hash_str("system/timer");
    let change = Change::Set{table: table_id, 
                              row: Index::Index(1 as u64), 
                              column: Index::Index(2 as u64),
                              value: Value::from_u64(i as u64),
                            };
    let txn = Transaction::from_change(change.clone());
    core.process_transaction(&txn);
  }
  println!("{:?}", core);
  println!("{:?}", core.runtime);


}