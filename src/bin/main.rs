extern crate mech_syntax;
extern crate mech_core;

use mech_syntax::parser::Parser;

fn main() {

  let mut parser = Parser::new();

  parser.parse("안녕 = 1 + 1");

  println!("{:?}", parser.parse_tree);

}