#[macro_use]
extern crate mech_syntax;

use mech_syntax::lexer::Lexer;
use mech_syntax::parser::{Parser, ParseStatus, Node};
use mech_syntax::compiler::Compiler;

macro_rules! compile_string {
  ($func:ident, $test:tt) => (
    #[test]
    fn $func() {
      let mut lexer = Lexer::new();
      let mut parser = Parser::new();
      let mut compiler = Compiler::new();
      lexer.add_string(String::from($test));
      let tokens = lexer.get_tokens();
      parser.text = String::from($test);
      parser.add_tokens(&mut tokens.clone());
      parser.build_parse_tree();
      assert_eq!(parser.status, ParseStatus::Ready);
      compiler.build_syntax_tree(parser.parse_tree);
      let ast = compiler.syntax_tree.clone();
      compiler.compile_blocks(ast);
      //assert_eq!(parser.status, ParseStatus::Ready);
    }
  )
}


compile_string!(empty, "");


compile_string!(column_define, "# A Working Program

## Section Two

  #gravity = 9");