// # Parer

// ## Prelude

use lexer::Token;
use lexer::Token::{HashTag, Identifier};


// ### Some utility macros

// Returns true if *any* of the supplied matches evaluate to true
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
macro_rules! and_combinator {
  ($e:expr) => {{
    {
      let val: bool = $e;
      val
    }
  }};
  ($e:expr, $($es:expr),+) => {{
    let mut result = true;
    let result = if !and_combinator! { $e } {
      false
    } else if !and_combinator! { $($es), + } {
      false
    } else {
      true
    };
    result
  }};
}

pub fn _true() -> bool {
  true
}

pub fn _false() -> bool {
  false
}

// ## Node

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
    let result = and_combinator! {
      _true(), _true(), _true()
    };
    println!("{:?}", result);
    while {
      if self.match_table(2) {
        true
      } else {
        false
      }
    } { };
  }

  
  pub fn match_alpha(&mut self, size: usize) -> bool {
    false
  }


  pub fn match_table(&mut self, size: usize) -> bool {
    let hash = &self.tokens[self.position];
    let identifier = &self.tokens[self.position + 1];
    match (hash, identifier) {
      (&HashTag, &Identifier{ref name}) => {
        self.position += size;
        true
      },
      _ => false,
    }
  }

}





