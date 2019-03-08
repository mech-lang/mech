extern crate mech_syntax;
extern crate mech_core;

use mech_syntax::lexer::Token;
use mech_syntax::parser::Node;
use mech_syntax::compiler::Compiler;
use mech_core::Block;
use mech_core::{Change, Transaction};
use mech_core::{Value, Index};
use mech_core::Hasher;
use mech_core::Core;
use mech_core::make_quantity;
use std::time::{Duration, SystemTime};
use mech_syntax::parser::Parser;

fn compile_test(input: String, test: Value) {
  let mut compiler = Compiler::new();
  let mut core = Core::new(1_000_000, 250);
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
/*
fn main() {
  let mut parser = Parser::new();
  let mut compiler = Compiler::new();
  let mut core = Core::new(1_000_000, 250);
  let input = r#"
block
  #test.x{1} := 77

block
  #test = [|x|
            9]"#;
  compiler.compile_string(input.to_string());
  println!("{:?}", compiler.parse_tree);
  println!("{:?}", compiler.syntax_tree);
  let ast = compiler.syntax_tree.clone();
  compiler.compile_blocks(ast);
  core.register_blocks(compiler.blocks.clone());
  core.step();
  println!("{:?}", core);
  println!("{:?}", core.runtime);

      let table = Hasher::hash_str("test");
    let row = Index::Index(1);
    let column = Index::Index(1);
    let actual = core.index(table, &row, &column);
}*/



fn main() {
  let input = String::from("
block
  #test.x{1} := 77

block
  #test = [|x|
            9]");
  let value = Value::Number(make_quantity(77,0,0));

  compile_test(input.clone(), value);


/*
  let mut compiler = Compiler::new();
  let mut core = Core::new(1_000_000, 250);
  compiler.compile_string(input.clone());
  core.register_blocks(compiler.blocks.clone());
  //println!("{:?}", compiler.parse_tree);
  println!("{:?}", compiler.syntax_tree);
  //println!("{:?}", core.runtime);
  core.step();
  println!("{:?}", core);
  println!("{:?}", core.runtime);*/

  
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