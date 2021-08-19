use mech_syntax::parser::Parser;

fn main() {

  let mut parser = Parser::new();

  parser.parse("block
  y̆és = 1 + 1");

  println!("{:?}", parser.parse_tree);

}