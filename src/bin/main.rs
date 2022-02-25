use mech_syntax::parser;
use mech_syntax::ast::Ast;
use mech_syntax::compiler::Compiler;
use mech_core::*;

use std::cell::RefCell;
use std::rc::Rc;

fn main() -> Result<(),MechError> {

  let mut ast = Ast::new();
  let mut compiler = Compiler::new();
  let mut core = Core::new();

  let parse_tree = parser::parse(r#"
block
  #range = 5 : 14
block
  #test = stats/sum(column: #range)"#)?;

  println!("{:#?}", parse_tree);

  ast.build_syntax_tree(&parse_tree);

  println!("{:?}", ast.syntax_tree);

  let blocks = compiler.compile_blocks(&vec![ast.syntax_tree.clone()]).unwrap();

  core.insert_blocks(blocks)?;

  core.schedule_blocks()?;

  let ticks = 2;
 // println!("{:#?}", core.get_table("balls").unwrap().borrow());

  /*for i in 1..=ticks {
    let txn = vec![
      Change::Set((hash_str("time/timer"), vec![(TableIndex::Index(1), TableIndex::Index(2), Value::U64(i as u64))])),
    ];
    core.process_transaction(&txn)?;
    println!("{:#?}", core.get_table("ball").unwrap().borrow());
  }
  println!("{:#?}", core.get_table("test").unwrap().borrow());*/


  println!("{:#?}", core.blocks);

  println!("{:#?}", core);

  
  if let Ok(table) = core.get_table("test") {
    println!("Answer:");
    println!("{:#?}", table.borrow());
  }

  Ok(())
}