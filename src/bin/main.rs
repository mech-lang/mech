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

  parser.parse(r#"
block
  ball = [1 [2 3]]
  line = [4 [5 6]]
  #out = [ball; line]
  
block
  #test = #out{1,2}{2}"#);

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
/*
││││├Transformation
│││││├Statement
││││││├VariableDefine
│││││││├Identifier(['t', 'e', 's', 't'](tst-don-hit-vid))
│││││││├Expression
││││││││├AnonymousTableDefine
│││││││││├TableRow
││││││││││├TableColumn
│││││││││││├SelectData(['b', 'a', 'l', 'l'] Local(pas-nor-one-olf)))
││││││││││││├Null
│││││││││├TableRow
││││││││││├TableColumn
│││││││││││├SelectData(['l', 'i', 'n', 'e'] Local(cup-pey-som-rom)))
││││││││││││├Null

││││├Transformation
│││││├Statement
││││││├TableDefine
│││││││├Table(#['t', 'e', 's', 't'](0x3fa3332bea97e4))
│││││││├Expression
││││││││├AnonymousTableDefine
│││││││││├TableRow
││││││││││├TableColumn
│││││││││││├SelectData(['b', 'a', 'l', 'l'] Local(pas-nor-one-olf)))
││││││││││││├Null
│││││││││├TableRow
││││││││││├TableColumn
│││││││││││├SelectData(['l', 'i', 'n', 'e'] Local(cup-pey-som-rom)))
││││││││││││├Null*/