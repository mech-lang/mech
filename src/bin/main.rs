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

  let parse_tree = parser::parse(r#"#test = 400<m> + 1<km>"#)?;

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
      Change::Set((hash_str("time/timer"), vec![(TableIndex::Index(0), TableIndex::Index(1), Value::U8(i))])),
    ];
    core.process_transaction(&txn)?;
    println!("{:#?}", core.get_table("balls").unwrap().borrow());
  }
  println!("{:#?}", core.get_table("test").unwrap().borrow());*/


  println!("{:#?}", core.blocks);

  println!("{:#?}", core);

  println!("Answer:");
  println!("{:#?}", core.get_table("test").unwrap().borrow());

  Ok(())
}