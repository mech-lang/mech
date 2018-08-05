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
      parser.build_parse_tree();
      assert_eq!(parser.status, ParseStatus::Ready);
    }
  )
}

// ## Variable

parse_string!(variable, "variable");
parse_string!(variable_index_dot, "var.1");
parse_string!(variable_index_bracket, "var[1]");

// ## Math

parse_string!(math_add, "1 + 1");
parse_string!(math_subtract, "1 - 1");
parse_string!(math_multiply, "1 * 1");
parse_string!(math_divide, "1 / 1");
parse_string!(math_add_variable, "x + y");
parse_string!(math_add_tables, "#x + #y");
parse_string!(math_add_constant, "x + 1");
parse_string!(math_add_index, "#x[1] + #y.1");

// ## Statement

parse_string!(statement_variable_define, "x = 1");

// ## Programs

parse_string!(program_simplest_block, "  1");
parse_string!(program_title_section, "# Title
  1");
parse_string!(program_title_longer, "# Two Words Title
  1");
parse_string!(program_title_subtitle, "# Title
## Subtitle
  1");
parse_string!(program_arbitrary_whitespace, "# Title
   

     



   
## Subtitle
  1");
parse_string!(program_paragraph, "# Title

## Subtitle

A block title
  1"); 
parse_string!(program_paragraph_repeat, "# Bouncing Balls
## Subtitle
This is the bouncing ball program

Set up the environment

  #ball = 1"); 
parse_string!(program_constraints, "# Bouncing Balls
 
## Section

Paragraph this is some text

And I can have more than one
  #ball = 0
  #gravity = 9"); 
parse_string!(program_blocks, "# Bouncing Balls
 
## Section

Paragraph this is some text

And I can have more than one
  #ball = 0

  #gravity = 9
  
And this is another paragraph

  #scene = #add.1");