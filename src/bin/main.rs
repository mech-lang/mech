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
use std::time::{Duration, SystemTime};

fn main() {
  let mut core = Core::new(10000000, 100);
  let mut compiler = Compiler::new();
  let input = String::from("# Bouncing Balls

Define the environment
  #test = 10000 + 1");

  compiler.compile_string(input);
  core.register_blocks(compiler.blocks.clone());
  //println!("{:?}", compiler.parse_tree);
  println!("{:?}", compiler.syntax_tree);
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
  //println!("{:?}", core);
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