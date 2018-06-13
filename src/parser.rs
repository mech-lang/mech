// # Parer

// ## Prelude

use lexer::Token;
use lexer::Token::{HashTag, Identifier, Period, LeftBracket, RightBracket, Digit};


// ### Some utility macros

// Returns true if *any* of the supplied matches evaluate to true
#[macro_export]
macro_rules! or_combinator {
  ($e:expr) => {{
    {
      let val: bool = $e;
      val
    }
  }};
  ($e:expr, $($es:expr),+) => {{
    let result: bool = if or_combinator! { $e } {
      true
    } else if or_combinator! { $($es), + } {
      true
    } else {
      false
    };
    result
  }};
}

// Returns true if *every* supplied match evaluates to true
#[macro_export]
macro_rules! and_combinator {
  ($e:expr) => {{
    {
      let val: bool = $e;
      val
    }
  }};
  ($e:expr, $($es:expr),+) => {{
    let mut result = true;
    result = if !and_combinator! { $e } {
      false
    } else if !and_combinator! { $($es), + } {
      false
    } else {
      true
    };
    result
  }};
}

// Creates a function that tests for a token
#[macro_export]
macro_rules! create_function {
  // This macro takes an argument of designator `ident` and
  // creates a function named `$func_name`.
  // The `ident` designator is used for variable/function names.
  ($func_name:ident, $token:ident) => (
    fn $func_name(&mut self) -> bool {
      let token = &self.tokens[self.position];
      match token {
        &$token => {
          self.position += 1;
          self.last_match = self.position;
          true
        },
        _ => {
          self.position = self.last_match;
          false
        },
      }
    }
  )
}

// ## Node

pub enum Node {
  Select
}

// ## Parser

#[derive(Debug, Clone)]
pub struct Parser {
  pub tokens: Vec<Token>,
  last_match: usize,
  pub position: usize
}

impl Parser {

  pub fn new() -> Parser {
    Parser {
      tokens: Vec::new(),
      last_match: 0,
      position: 0,
    }
  }

  pub fn add_tokens(&mut self, tokens: &mut Vec<Token>) {
    self.tokens.append(tokens);
  }

  pub fn build_ast(&mut self) {
    //self.match_table();
    //self.match_left_bracket();
    //while {
      let result = or_combinator!{
        self.index(),
        self.dot_select()
      };
      println!("{:?}", result);
    //} { };
  }

  // #student
  pub fn table(&mut self) -> bool {
    and_combinator!(self.hash_tag(), self.identifier())
  }

  // #student.grade
  pub fn dot_select(&mut self) -> bool {
    and_combinator!(self.table(), self.period(), self.identifier())
  }

  // #student[1]
  pub fn index(&mut self) -> bool {
    and_combinator!(self.table(), self.left_bracket(), self.digit(), self.right_bracket())
  }

  // .
  create_function!{period, Period}
  create_function!{left_bracket, LeftBracket}
  create_function!{right_bracket, RightBracket}
  create_function!{hash_tag, HashTag}
    
  pub fn identifier(&mut self) -> bool {
    let token = &self.tokens[self.position];
    match token {
      &Identifier{ref name} => {
        self.position += 1;
        self.last_match = self.position;
        true
      },
      _ => {
        self.position = self.last_match;
        false
      },
    }
  }

  pub fn digit(&mut self) -> bool {
    let token = &self.tokens[self.position];
    match token {
      &Digit{ref value} => {
        self.position += 1;
        self.last_match = self.position;
        true
      },
      _ => {
        self.position = self.last_match;
        false
      },
    }
  }

}





