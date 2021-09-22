use mech_syntax::parser::Parser;
use mech_syntax::ast::Ast;
use mech_syntax::compiler::Compiler;
use mech_core::Core;

fn main() {

  let mut parser = Parser::new();
  let mut ast = Ast::new();
  let mut compiler = Compiler::new();
  let mut core = Core::new();

  parser.parse("#test = 10");

  println!("{:?}", parser.parse_tree);

  ast.build_syntax_tree(&parser.parse_tree);

  println!("{:?}", ast.syntax_tree);

  let blocks = compiler.compile_blocks(&vec![ast.syntax_tree.clone()]);

  core.insert_block(blocks[0].clone());
  
  for t in blocks {
    println!("{:#?}", t);
  }

  println!("{:#?}", core);

  println!("{:#?}", core.get_table("test").unwrap().borrow().get(0, 0));

}




int x = 10; // Stack allocated variable
int size = stdin();

char val = 5;
char* val2 = malloc(sizeof(char)); // Heap allocated
*val2 = val; 
free(val2);

let val: u8 = 5;
let boxed: Box<u8> = Box::new(val);




fn main() {
  println!("{:?}", x);

  let x = 10;
  {
    let y = x;
  }

  println!("{:?}", x);

}




