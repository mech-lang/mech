// # Parer

// ## Prelude

use lexer::Token;
use lexer::Token::{HashTag, Alpha, Period, LeftBracket, RightBracket, Digit, Space, Equal, Plus, EndOfStream, Dash, Asterisk, Backslash};
use mech::{Hasher, Function};
use alloc::{String, Vec, fmt};

// ## Helper Macros

macro_rules! leaf {
  ($func:ident, $token:expr) => (
    pub fn $func(s: &mut ParseState) -> &mut ParseState {
      let result = token(s, $token);
      result
    }
  )
}

macro_rules! node {
  ($func:ident, $node:tt, $production:expr, $label: expr) => (
    pub fn $func(s: &mut ParseState) -> &mut ParseState {
      let old_depth = s.depth.clone();
      //s.depth += 1; spacer(s.depth); println!($label);
      let previous = s.last_match.clone();
      let result = $production(s);
      let node = Node::$node{ children: result.node_stack.drain(previous..).collect() };
      if result.ok() {
        result.node_stack.push(node);
        result.last_match = result.node_stack.len();
        //spacer(old_depth + 1); print!($label); println!(" √");
      } else { 
        //spacer(old_depth + 1); print!($label); println!(" X");
      }
      result.depth = old_depth;
      result
    }
  )
}

// ## Node

#[derive(Clone, PartialEq)]
pub enum Node {
  Root{ children: Vec<Node> },
  Block{ children: Vec<Node> },
  Constraint{ children: Vec<Node> },
  Select { children: Vec<Node> },
  Insert { children: Vec<Node> },
  ColumnDefine { children: Vec<Node> },
  Table { children: Vec<Node> },
  Number { children: Vec<Node> },
  MathExpression { children: Vec<Node> },
  InfixOperation { children: Vec<Node>},
  Repeat{ children: Vec<Node> },
  Identifier{ children: Vec<Node> },
  Alpha{ children: Vec<Node> },
  DotIndex{ children: Vec<Node> },
  BracketIndex{ children: Vec<Node> },
  Index{ children: Vec<Node> },
  Data{ children: Vec<Node> },
  Equality{ children: Vec<Node> },
  Expression{ children: Vec<Node> },
  Constant{ children: Vec<Node> },
  Infix{ children: Vec<Node> },
  Token{token: Token},
}

impl fmt::Debug for Node {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    print_recurse(self, 0);
    Ok(())
  }
}

pub fn print_recurse(node: &Node, level: usize) {
  spacer(level);
  let children: Option<&Vec<Node>> = match node {
    Node::Root{children} => {print!("Root\n"); Some(children)},
    Node::Block{children} => {print!("Block\n"); Some(children)},
    Node::Constraint{children} => {print!("Constraint\n"); Some(children)},
    Node::Select{children} => {print!("Select\n"); Some(children)},
    Node::Insert{children} => {print!("Insert\n"); Some(children)},
    Node::MathExpression{children} => {print!("Math\n"); Some(children)},
    Node::Table{children} => {print!("Table\n"); Some(children)},
    Node::Number{children} => {print!("Number\n"); Some(children)},
    Node::ColumnDefine{children} => {print!("ColumnDefine\n"); Some(children)},
    Node::InfixOperation{children} => {print!("Infix\n"); Some(children)},
    Node::Repeat{children} => {print!("Repeat\n"); Some(children)},
    Node::Identifier{children} => {print!("Identifier\n"); Some(children)},
    Node::DotIndex{children} => {print!("DotIndex\n"); Some(children)},
    Node::BracketIndex{children} => {print!("BracketIndex\n"); Some(children)},
    Node::Index{children} => {print!("Index\n"); Some(children)},
    Node::Equality{children} => {print!("Equality\n"); Some(children)},
    Node::Data{children} => {print!("Data\n"); Some(children)},
    Node::Infix{children} => {print!("Infix\n"); Some(children)},
    Node::Expression{children} => {print!("Expression\n"); Some(children)},
    Node::Constant{children} => {print!("Constant\n"); Some(children)},
    Node::Token{token} => {print!("Token({:?})\n", token); None},
    _ => {print!("Unhandled Node"); None},
  };  
  match children {
    Some(childs) => {
      for child in childs {
        print_recurse(child, level + 1)
      }
    },
    _ => (),
  }    
}

pub fn spacer(width: usize) {
  let limit = if width > 0 {
    width - 1
  } else {
    width
  };
  for _ in 0..limit {
    print!("│");
  }
  print!("├");
}

// ## Parser

#[derive(Debug, Clone, PartialEq)]
pub enum ParseStatus {
  Ready,
  Parsing,
  Error(ParseError),
  Complete,
}

#[derive(Debug, Clone)]
pub struct ParseState {
  pub status: ParseStatus,
  pub token_stack: Vec<Token>,
  pub node_stack: Vec<Node>,
  last_match: usize,
  pub position: usize,
  pub committed: usize,
  depth: usize,
}

impl ParseState {
  pub fn new() -> ParseState {
    ParseState {
      status: ParseStatus::Parsing,
      node_stack: Vec::new(), 
      token_stack: Vec::new(),
      last_match: 0,
      position: 0,
      committed: 0,
      depth: 0,
    }
  }

  pub fn ok(&self) -> bool {
    if self.status == ParseStatus::Parsing {
      true
    } else {
      false
    }
  }

  pub fn and<F>(&mut self, production: F) -> &mut ParseState
    where F: Fn(&mut ParseState) -> &mut ParseState {
    if !self.ok() {
      self
    } else {
      let mut before = self.clone();
      //spacer(self.depth); println!("And");
      let result = production(self);
      result.depth = before.depth;
      result
    }
  }

  pub fn or<F>(&mut self, production: F) -> &mut ParseState
    where F: Fn(&mut ParseState) -> &mut ParseState {
    if self.ok() {
      self
    } else {
      let mut before = self.clone();
      self.depth += 1;
      //spacer(self.depth); println!("OR");
      //println!("Before: {:?}", &before);
      self.status = ParseStatus::Parsing;
      let result = production(self);
      //println!("Result: {:?}", &before);
      result.depth = before.depth;
      result
    }
  }

  pub fn optional<F>(&mut self, production: F) -> &mut ParseState
    where F: Fn(&mut ParseState) -> &mut ParseState {
    let before_depth = self.depth;
    self.depth += 1;
    //spacer(self.depth); println!("Optional");
    if self.ok() {
      let result = production(self);
      if result.ok() {
        return result
      } else {
        result.status = ParseStatus::Parsing;
        result.depth = before_depth;
        return result
      }
    } else {
      self.depth = before_depth;
      return self
    }
  }

}

#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
  pub position: usize,
  pub token: Token,
  pub code: u64,
  pub node_stack: Vec<Node>,
}

#[derive(Clone)]
pub struct Parser {
  pub status: ParseStatus,
  pub tokens: Vec<Token>,
  pub ast: Node,
}

impl Parser {

  pub fn new() -> Parser {
    Parser {
      status: ParseStatus::Ready,
      tokens: Vec::new(),
      ast: Node::Root{ children: Vec::new()  },
    }
  }

  pub fn add_tokens(&mut self, tokens: &mut Vec<Token>) {
    self.tokens.append(tokens);
  }

  pub fn build_ast(&mut self) {
    let mut s = ParseState::new();
    s.token_stack.append(&mut self.tokens);
    let result = data(&mut s).and(end);
    //println!("{:?}",result);
    if result.ok() {
      self.status = ParseStatus::Ready;
      self.ast = result.node_stack.pop().unwrap();
    } else {
      self.status = result.status.clone();
    }
  }
   
}

impl fmt::Debug for Parser {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    
    write!(f, "┌───────────────────────────────────────┐\n").unwrap();
    write!(f, "│ Parser\n").unwrap();
    write!(f, "│ Status: {:?}\n", self.status).unwrap();
    write!(f, "│ Length: {:?}\n", self.tokens.len()).unwrap();
    write!(f, "├───────────────────────────────────────┤\n").unwrap();
    for (ix, token) in self.tokens.iter().enumerate() {
      let c1 = " "; //if self.position == ix + 1 { ">" } else { " " };
      let c2 = " "; //if self.last_match == ix + 1 { ">" } else { " " };
      write!(f, "│ {:}{:} {:?}\n", c1, c2, token).unwrap();
    }
    write!(f, "├───────────────────────────────────────┤\n").unwrap();
    write!(f, "{:?}", self.ast);
    write!(f, "└───────────────────────────────────────┘\n").unwrap();
    Ok(())
  }
}

node!{constraint, Constraint, |s|{ equality(s) }, "Constraint"}
node!{constant, Constant, |s|{ digit(s) }, "Constant"}
node!{infix, Infix, |s|{ plus(s).or(dash).or(asterisk).or(backslash) }, "Infix"}
node!{math_expression, MathExpression, |s|{ data(s).and(space).and(infix).and(space).and(data) }, "Math Expression"}
node!{expression, Expression, |s|{ math_expression(s).or(data).or(constant) }, "Expression"}
node!{equality, Equality, |s| { data(s).and(space).and(equal).and(space).and(expression) }, "Equality"}
node!{data, Data, |s| { table(s).or(identifier).or(constant).optional(index) }, "Data"}
node!{index, Index, |s| { dot_index(s).or(bracket_index) }, "Index"}
node!{bracket_index, BracketIndex, |s| { left_bracket(s).and(digit).and(right_bracket) }, "Bracket Index"}
node!{dot_index, DotIndex, |s| { period(s).and(digit) }, "Dot Index"}
node!{table, Table, |s| { hashtag(s).and(identifier) }, "Table"}
node!{identifier, Identifier, |s| { repeat(alpha, s) }, "Identifier"}

pub fn repeat<F>(production: F, s: &mut ParseState) -> &mut ParseState 
  where F: Fn(&mut ParseState) -> &mut ParseState
{
  s.depth += 1; 
  //spacer(s.depth); println!("Repeat");
  let mut once = false;
  let mut result = s;
  let start_pos = result.last_match.clone();
  while result.ok() {
    let result = production(result);
    if result.ok() {
      result.depth -= 1;
      once = true;
    }
  }
  if once {
    result.status = ParseStatus::Parsing;
    let node = Node::Repeat{ children: result.node_stack.drain(start_pos..).collect() };
    result.node_stack.push(node);
    result.last_match = result.node_stack.len();
  }
  result
}

leaf!{alpha, Token::Alpha}
leaf!{hashtag, Token::HashTag}
leaf!{period, Token::Period}
leaf!{left_bracket, Token::LeftBracket}
leaf!{right_bracket, Token::RightBracket}
leaf!{equal, Token::Equal}
leaf!{plus, Token::Plus}
leaf!{dash, Token::Dash}
leaf!{asterisk, Token::Asterisk}
leaf!{backslash, Token::Backslash}
leaf!{space, Token::Space}
leaf!{digit, Token::Digit}
 
pub fn end(s: &mut ParseState) -> &mut ParseState {
  let old_depth = s.depth;
  s.depth += 1; 
  //spacer(s.depth); println!("End");
  let result = token(s, Token::EndOfStream);
  if result.ok() {
    result.node_stack.pop();
    let node = Node::Root{children: result.node_stack.drain(..).collect()};
    result.node_stack.push(node);
  }
  result.depth = old_depth;
  result
}

pub fn token(s: &mut ParseState, token: Token) -> &mut ParseState {
  s.depth += 1; 
  //spacer(s.depth); print!("Token: [{:?}] = {:?}?", s.token_stack[s.position], token);
  if s.token_stack[s.position] == token {
    s.position += 1;
    s.last_match += 1;
    s.node_stack.push(Node::Token{token});
    //println!(" √");
  } else {
    s.status = ParseStatus::Error(ParseError{code: 1, position: s.position, token: s.token_stack[s.position].clone(), node_stack: s.node_stack.clone() });
    //println!(" X");
  }
  s
}