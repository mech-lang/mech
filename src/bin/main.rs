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
use mech_core::make_quantity;
use std::time::{Duration, SystemTime};


fn compile_test(input: String, test: Value) {
  let mut compiler = Compiler::new();
  let mut core = Core::new(10, 10);
  compiler.compile_string(input);
  core.register_blocks(compiler.blocks);
  core.step();
  let table = Hasher::hash_str("test");
  let row = Index::Index(1);
  let column = Index::Index(1);
  let actual = core.index(table, &row, &column);
  match actual {
    Some(value) => {
      assert_eq!(*value, test);
    },
    None => assert_eq!(0,1),
  }
  
}

fn main() {
  let input = String::from("# Bouncing Balls

Define the environment
  #html/event/click = [|x y|]
  #ball = [x: 50 y: 9 vx: 40 vy: 9]
  #system/timer = [resolution: 15, tick: 0]
  #gravity = 2
  #boundary = [x: 60 y: 60]

## Update condition

Now update the block positions
  ~ #system/timer.tick
  #ball.x := #ball.x + #ball.vx
  #ball.y := #ball.y + #ball.vy
  #ball.vy := #ball.vy + #gravity

## Boundary Condition

Keep the balls within the y boundary
  ~ #ball.y
  iy = #ball.y > #boundary.y
  #ball.y{iy} := #boundary.y
  #ball.vy{iy} := -#ball.vy * 80 / 100

Keep the balls within the x boundary
  ~ #ball.x
  ix = #ball.x > #boundary.x
  ixx = #ball.x < 0
  #ball.x{ix} := #boundary.x
  #ball.x{ixx} := 0
  #ball.vx{ix | ixx} := -#ball.vx * 80 / 100

## Create More Balls

Create ball on click
  ~ #html/event/click.x
  #ball += [x: 10 y: 10 vx: 40 vy: 0]
  
block
  #test = #ball{1,1} + #ball{1,3} + #ball{2,1} + #ball{2,3}
  
## Bouncing Balls

Define the environment
  #html/event/click = [|x y|]
  #ball = [x: 50 y: 9 vx: 40 vy: 9]
  #system/timer = [resolution: 15, tick: 0]
  #gravity = 2
  #boundary = [x: 60 y: 60]

## Update condition

Now update the block positions
  ~ #system/timer.tick
  #ball.x := #ball.x + #ball.vx
  #ball.y := #ball.y + #ball.vy
  #ball.vy := #ball.vy + #gravity

## Boundary Condition

Keep the balls within the y boundary
  ~ #ball.y
  iy = #ball.y > #boundary.y
  #ball.y{iy} := #boundary.y
  #ball.vy{iy} := -#ball.vy * 80 / 100

Keep the balls within the x boundary
  ~ #ball.x
  ix = #ball.x > #boundary.x
  ixx = #ball.x < 0
  #ball.x{ix} := #boundary.x
  #ball.x{ixx} := 0
  #ball.vx{ix | ixx} := -#ball.vx * 80 / 100

## Create More Balls

Create ball on click
  ~ #html/event/click.x
  #ball += [x: 10 y: 10 vx: 40 vy: 0]
  
block
  #test = #ball{1,1} + #ball{1,3} + #ball{2,1} + #ball{2,3}");
  let value = Value::Number(make_quantity(780000,-4,0));

  //compile_test(input.clone(), value);


  let mut compiler = Compiler::new();
  let mut core = Core::new(1_000_000, 250);
  compiler.compile_string(input.clone());
  core.register_blocks(compiler.blocks.clone());
  //println!("{:?}", compiler.parse_tree);
  println!("{:?}", compiler.syntax_tree);
  //println!("{:?}", core.runtime);
  //core.step();
  //println!("{:?}", core);
  //println!("{:?}", core.runtime);

  
  /*
  let now = SystemTime::now();
  let n = 100;
  for i in 0..n {
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
  match now.elapsed() {
    Ok(elapsed) => {
      // it prints '2'
      let time: f32 = elapsed.as_millis() as f32;
      println!("{}ms", time / n as f32);
    }
    Err(e) => {
      // an error occurred!
      println!("Error: {:?}", e);
    }
  }
  println!("{:?}", core);*/

}