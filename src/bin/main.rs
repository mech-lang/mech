use mech_syntax::parser::Parser;
use mech_syntax::ast::Ast;
use mech_syntax::compiler::Compiler;
use mech_core::Core;

fn main() {

  let mut parser = Parser::new();
  let mut ast = Ast::new();
  let mut compiler = Compiler::new();
  let mut core = Core::new();

  parser.parse("block 
  🤦🏼‍♂️ = 4
  😃 = 2
  y̆és = 🤦🏼‍♂️ + 😃");

  ast.build_syntax_tree(&parser.parse_tree);

  let blocks = compiler.compile_blocks(&vec![ast.syntax_tree.clone()]);

  core.insert_block(blocks[0].clone());



  println!("{:?}", ast.syntax_tree);
  for t in blocks {
    println!("{:?}", t);
  }

  println!("{:?}", core);

}