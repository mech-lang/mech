#[macro_use]
extern crate mech_syntax;

use mech_syntax::lexer::{Lexer, Token};
use mech_syntax::lexer::Token::{HashTag, Alpha, LeftBracket, RightBracket, Digit, Space, Comma, Plus, Equal};
use mech_syntax::parser::{Parser, ParseStatus};

macro_rules! parse_string {
  ($func:ident, $test:tt) => (
    #[test]
    fn $func() {
      let mut lexer = Lexer::new();
      let mut parser = Parser::new();
      lexer.add_string(String::from($test));
      let tokens = lexer.get_tokens();
      parser.add_tokens(&mut tokens.clone());
      parser.build_ast();
      assert_eq!(parser.status, ParseStatus::Ready);
    }
  )
}

//parse_string!(equal_constant, "x = 1");
//parse_string!(add_columns, "#add.3 = #add.1 + #add.2");

// ## Constant

parse_string!(constant_digit, "1");

// ## Table

parse_string!(table, "#table");
parse_string!(table_index_dot_index, "#table.1");
parse_string!(table_index_bracket_index, "#table[1]");

// ## Variable

parse_string!(variable, "variable");
parse_string!(variable_index_dot, "var.1");
parse_string!(variable_index_bracket, "var[1]");
