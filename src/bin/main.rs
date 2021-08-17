use mech_syntax::parser::Parser;

fn main() {

  let mut parser = Parser::new();

  parser.parse("y̆és");
  //parser.parse("y̆es = 😃 + 1");

  println!("{:?}", parser.parse_tree);

}