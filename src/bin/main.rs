use mech_syntax::parser::Parser;
use mech_syntax::ast::Ast;
use mech_syntax::compiler::Compiler;
use mech_core::Core;

fn main() {

  let mut parser = Parser::new();
  let mut ast = Ast::new();
  let mut compiler = Compiler::new();
  let mut core = Core::new();

  parser.parse("
# Title

block
  #x = 1
  
## Subtitle

block
  #y = 2

### Subsubtitle

block 
  #test = #x + #y");

  //println!("{:#?}", parser.parse_tree);

  ast.build_syntax_tree(&parser.parse_tree);

  println!("{:?}", ast.syntax_tree);

  let blocks = compiler.compile_blocks(&vec![ast.syntax_tree.clone()]);

  core.insert_block(blocks[0].clone());
  core.insert_block(blocks[1].clone());
  core.insert_block(blocks[2].clone());

  
  /*for t in blocks {
    println!("{:#?}", t);
  }*/

  println!("{:#?}", core);

  println!("{:#?}", core.get_table("test"));

}