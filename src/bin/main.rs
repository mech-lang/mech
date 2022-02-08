use mech_syntax::parser;
use mech_syntax::ast::Ast;
use mech_syntax::compiler::Compiler;
use mech_core::{Core,MechError};

use std::cell::RefCell;
use std::rc::Rc;

fn main() -> Result<(),MechError> {

  let mut ast = Ast::new();
  let mut compiler = Compiler::new();
  let mut core = Core::new();

  let parse_tree = parser::parse(r#"
block
  #x = [1 2; 3 4]

block
  #y = #x{:}"#)?;

  println!("{:#?}", parse_tree);

  ast.build_syntax_tree(&parse_tree);

  println!("{:?}", ast.syntax_tree);

  let blocks = compiler.compile_blocks(&vec![ast.syntax_tree.clone()]).unwrap();

  core.insert_blocks(blocks)?;

  println!("{:#?}", core);

  //println!("{:#?}", core.get_table("test"));

  Ok(())
}