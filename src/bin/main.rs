use mech_syntax::parser::Parser;
use mech_syntax::ast::Ast;

fn main() {

  let mut parser = Parser::new();
  let mut ast = Ast::new();

  parser.parse("block 
  🤦🏼‍♂️ = 1
  😃 = 2
  y̆és = 🤦🏼‍♂️ + 😃");

  ast.build_syntax_tree(&parser.parse_tree);

  println!("{:?}", ast.syntax_tree);

}