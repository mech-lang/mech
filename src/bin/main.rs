use mech_syntax::parser::Parser;
use mech_syntax::ast::Ast;
use mech_syntax::compiler::Compiler;
use mech_core::{Core,MechError};

use std::cell::RefCell;
use std::rc::Rc;

fn main() -> Result<(),MechError> {

  let mut parser = Parser::new();
  let mut ast = Ast::new();
  let mut compiler = Compiler::new();
  let mut core = Core::new();

  parser.parse(r#"#test = [first: 12, second: _, third: 45]"#);

  //println!("{:#?}", parser.parse_tree);

  ast.build_syntax_tree(&parser.parse_tree);

  println!("{:?}", ast.syntax_tree);

  let blocks = compiler.compile_blocks(&vec![ast.syntax_tree.clone()]).unwrap();

  for block in blocks {
    match core.insert_block(Rc::new(RefCell::new(block.clone()))) {
      Ok(()) => (),
      Err(mech_error) => println!("ERROR: {:?}", mech_error),
    }
  }
  
  /*for t in blocks {
    println!("{:#?}", t);
  }*/

  println!("{:#?}", core);

  println!("{:#?}", core.get_table("test"));

  Ok(())
}