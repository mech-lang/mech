use mech_syntax::parser::Parser;
use mech_syntax::ast::Ast;
use mech_syntax::compiler::Compiler;

fn main() {

  let mut parser = Parser::new();
  let mut ast = Ast::new();
  let mut compiler = Compiler::new();

  parser.parse("block 
  🤦🏼‍♂️ = 1
  😃 = 2
  y̆és = 🤦🏼‍♂️ + 😃");

  ast.build_syntax_tree(&parser.parse_tree);

  let tfms = compiler.compile_transformation(&ast.syntax_tree);

  println!("{:?}", ast.syntax_tree);
  for t in tfms {
    println!("{:?}", t);
  }

}