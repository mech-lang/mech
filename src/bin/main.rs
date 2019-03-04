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
  let input = String::from("# Clock

Create a timer that ticks every second. This is the time source.
  #system/timer = [resolution: 1000, tick: 0]

Set up a clock hands table. Degrees is the deflection from noon.
x and y are the coordinates of the end point of the clock hand
  #clock-hands = [|degrees length stroke     x y |
                   0       30     \"#023963\" 0 0
                   0       40     \"#023963\" 0 0
                   0       40     \"#ce0b46\" 0 0 ]

## Update the clock

Calculate clock hand angles every time the clock ticks
  ~ #system/timer.tick 
  #clock-hands{1}.degrees := 30 * #system/timer.hours
  #clock-hands{2}.degrees := 6 * #system/timer.minutes
  #clock-hands{3}.degrees := 6 * #system/timer.seconds

Calculate x and y endpoints
  ~ #clock-hands.degrees
  #clock-hands.x := 50 + (30 * math/sin(degrees: #clock-hands.degrees))
  #clock-hands.y := 50 - (30 * math/cos(degrees: #clock-hands.degrees))");
  let value = Value::Number(make_quantity(780000,-4,0));

  //compile_test(input.clone(), value);


  let mut compiler = Compiler::new();
  let mut core = Core::new(1_000_000, 250);
  compiler.compile_string(input.clone());
  core.register_blocks(compiler.blocks.clone());
  //println!("{:?}", compiler.parse_tree);
  println!("{:?}", compiler.syntax_tree);
  //println!("{:?}", core.runtime);
  core.step();
  println!("{:?}", core);
  println!("{:?}", core.runtime);

  
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