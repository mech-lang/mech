// # Parer

// ## Prelude

use lexer::Token;
use lexer::Token::{HashTag, Identifier, Period, LeftBracket, RightBracket, Digit, Space, Equal, Plus, EndOfStream};
use mech::indexes::Hasher;
use mech::operations::Function;
use alloc::{String, Vec, fmt};

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
      println!("evaluate");
      let val: bool = $e;
      val
    }
  }};
  ($e:expr, $($es:expr),+) => {{
    let mut result = true;
    result = if !and_combinator! { $e } {
      println!("and - false1");
      false
    } else if !and_combinator! { $($es), + } {
      println!("and - false2");
      false
    } else {
      println!("and - true");
      true
    };
    result
  }};
}

// Creates a function that tests for a token
#[macro_export]
macro_rules! production_rule {
  ($func_name:ident, $token:ident) => (
    fn $func_name(&mut self) -> bool {
      println!("HERE IN TOKEN  {:?}", self.position >= self.tokens.len());
      let token = if self.position < self.tokens.len() {
        &self.tokens[self.position]
      } else { 
        println!("End Of Stream");
        &EndOfStream 
      };
      println!("The token is {:?}", token);
      let last_match = self.last_match;
      let old_position = self.position;
      match token {
        &$token{..} => {
          self.token_stack.push(token.clone());
          self.position += 1;
          self.last_match = self.position;
          true
        },
        _ => {
          self.last_match = last_match;
          self.position = old_position;
          false
        },
      }
    }
  )
}

// ## Node

#[derive(Clone)]
pub enum Node {
  Select { children: Vec<Node> },
  ColumnDefine { parts: Vec<Node> },
  Table { id: u64, children: Vec<Node>, token: Token },
  Number { value: u64, token: Token },
  MathExpression { operation: Function, arguments: Vec<Node> },
}

impl fmt::Debug for Node {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Node::Select{..} => write!(f, "Select").unwrap(),
      Node::MathExpression{..} => write!(f, "Math").unwrap(),
      Node::Table{..} => write!(f, "Table").unwrap(),
      Node::Number{..} => write!(f, "Number").unwrap(),
      Node::ColumnDefine{..} => write!(f, "ColumnDefine").unwrap(),
    }   
    Ok(())
  }
}

// ## Parser

#[derive(Debug, Clone)]
pub enum ParseStatus {
  Waiting,
  Parsing,
  Error,
  Complete,
}

#[derive(Clone)]
pub struct Parser {
  pub parse_status: ParseStatus,
  pub tokens: Vec<Token>,
  pub token_stack: Vec<Token>,
  pub node_stack: Vec<Node>,
  last_match: usize,
  pub position: usize,
  pub committed: usize,
}

impl Parser {

  pub fn new() -> Parser {
    Parser {
      parse_status: ParseStatus::Waiting,
      tokens: Vec::new(),
      token_stack: Vec::new(),
      node_stack: Vec::new(),
      last_match: 0,
      position: 0,
      committed: 0,
    }
  }

  pub fn reset(&mut self) {
    self.last_match = self.committed;
    self.position = self.committed;
  }

  pub fn add_tokens(&mut self, tokens: &mut Vec<Token>) {
    self.tokens.append(tokens);
  }

  pub fn build_ast(&mut self) {
    self.parse_status = ParseStatus::Parsing;
    'parse_loop: while {
      println!("move");
      let result = and_combinator!{
        self.expression()
        //,self.select()
      };
      self.committed = self.last_match;
      if self.position == self.tokens.len() {
        self.parse_status = ParseStatus::Complete;
        break 'parse_loop
      }
      result
    } { };
    match self.parse_status {
      ParseStatus::Complete => (), 
      _ => self.parse_status = ParseStatus::Waiting,
    }
  }

  pub fn select(&mut self) -> bool {
    let result = or_combinator!(self.bracket_index());
    if !result { self.reset(); }
    else {
      let table = self.node_stack.pop().unwrap();
      self.node_stack.push(Node::Select{children: vec![table]})
    }
    result
  }


  pub fn expression(&mut self) -> bool {
    println!("Expression");
    let result = and_combinator!(
      self.column_define()
      //,self.dot_select()
    );
    println!("HERE2");
    if !result { self.reset(); }
    result
  }

  // #add[3] = #add[1] + #add[2]
  pub fn column_define(&mut self) -> bool {
    let result = and_combinator!(
      self.bracket_index(),
      self.space(), 
      self.equal(), 
      self.space(), 
      self.math_expression()   
    );
    if !result { self.reset(); }
    else { 
      let math_expression = self.node_stack.pop().unwrap();
      let sink = self.node_stack.pop().unwrap();
      println!("sstack {:?}", sink);
      self.node_stack.push(Node::ColumnDefine{ parts: vec![sink, math_expression] })
    }
    result
  }

  // #add[1] + #add[2]
  pub fn math_expression(&mut self) -> bool {
    let result = and_combinator!(
      self.select(), 
      self.space(), 
      self.plus(), 
      self.space(), 
      self.select()
    );
    if !result { self.reset(); }
    else { 
      let left = self.node_stack.pop().unwrap();
      let right = self.node_stack.pop().unwrap();
      self.node_stack.push(Node::MathExpression{operation: Function::Add, arguments: vec![left, right] })
    }
    result
  }




  // #student
  pub fn table(&mut self) -> bool {
    println!("Table");
    let result = and_combinator!(self.hash_tag(), self.identifier());
    if !result { self.reset(); }
    else { 
      let token = self.token_stack.pop().unwrap();
      match token {
        Identifier{ref name} =>  {
          let id = Hasher::hash_byte_vector(name);
          self.node_stack.push(Node::Table{id, children: vec![], token: token.clone()})
        },
        _ => (),
      }

    }
    result
  }

  // #student.grade
  pub fn dot_index(&mut self) -> bool {
    let result = and_combinator!(self.table(), self.period(), self.identifier());
    if !result { self.reset(); }
    result
  }

  // #student[1]
  pub fn bracket_index(&mut self) -> bool {
    println!("Index");
    let result = and_combinator!(self.table(), self.left_bracket(), self.digit(), self.right_bracket());
    println!("The result of index was {:?}", result);
    if !result { self.reset(); }
    else {
      println!("{:?}", self.token_stack);
      self.token_stack.pop().unwrap();
      println!("{:?}", self.token_stack);
      let digit = self.token_stack.pop().unwrap();
      let ix = self.node_stack.len() - 1;
      
      match self.node_stack[ix] {
        Node::Table{ref id, ref token,ref mut children} => {
          let value = get_value(&digit).unwrap();
          children.push(Node::Number{value, token: digit.clone()})
        },
        _ => (),
      }
    }
    result
  }

  production_rule!{plus, Plus}
  production_rule!{equal, Equal}
  production_rule!{space, Space}
  production_rule!{period, Period}
  production_rule!{left_bracket, LeftBracket}
  production_rule!{right_bracket, RightBracket}
  production_rule!{hash_tag, HashTag}
  production_rule!{identifier, Identifier}
  production_rule!{digit, Digit}



}

impl fmt::Debug for Parser {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    
    write!(f, "┌───────────────────────────────────────┐\n").unwrap();
    write!(f, "│ Parser\n").unwrap();
    write!(f, "│ Status: {:?}\n", self.parse_status).unwrap();
    write!(f, "│ Length: {:?}\n", self.tokens.len()).unwrap();
    write!(f, "│ Position: {:?}\n", self.position).unwrap();
    write!(f, "│ Last Match: {:?}\n", self.last_match).unwrap();
    write!(f, "│ Committed: {:?}\n", self.committed).unwrap();
    write!(f, "├───────────────────────────────────────┤\n").unwrap();
    for (ix, token) in self.tokens.iter().enumerate() {
      let c1 = if self.position == ix + 1 { ">" } else { " " };
      let c2 = if self.last_match == ix + 1 { ">" } else { " " };
      write!(f, "│ {:}{:} {:?}\n", c1, c2, token).unwrap();
    }
    write!(f, "└───────────────────────────────────────┘\n").unwrap();
    Ok(())
  }
}

pub fn get_value(token: &Token) -> Option<u64> {
  match token {
    Digit{value} => {
      let the_value: u64 = *value as u64;
      Some(the_value)
    },
    _ => None,
  }
}



