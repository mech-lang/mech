// # Parer

// ## Prelude

use lexer::Token;
use lexer::Token::{HashTag, Alpha, Period, LeftBracket, RightBracket, Newline,
                   Digit, Space, Equal, Plus, EndOfStream, Dash, Asterisk, Backslash};
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
      s.depth += 1; 
      spacer(s.depth); println!($label);
      let previous = s.last_match.clone();
      let old_position = s.position;
      let result = $production(s);
      let node = Node::$node{ children: result.node_stack.drain(previous..).collect() };
      if result.ok() {
        result.node_stack.push(node);
        result.last_match = result.node_stack.len();
        spacer(old_depth + 1); print!($label); println!(" √");
      } else { 
        spacer(old_depth + 1); print!($label); println!(" X");
        result.position = old_position;
        result.last_match = previous;
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
  Program{ children: Vec<Node> },
  Title{ children: Vec<Node> },
  Subtitle{ children: Vec<Node> },
  Head{ children: Vec<Node> },
  Body{ children: Vec<Node> },
  Statement{ children: Vec<Node> },
  StatementOrExpression{ children: Vec<Node> },
  Node{ children: Vec<Node> },
  Alphanumeric{ children: Vec<Node> },
  Paragraph{ children: Vec<Node> },
  Word{ children: Vec<Node> },
  Section{ children: Vec<Node> },
  Whitespace{ children: Vec<Node> },
  SpaceOrNewline{ children: Vec<Node> },
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
    Node::Alphanumeric{children} => {print!("Alphanumeric\n"); Some(children)},
    Node::Word{children} => {print!("Word\n"); Some(children)},
    Node::Paragraph{children} => {print!("Paragraph\n"); Some(children)},
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
    Node::Program{children} => {print!("Program\n"); Some(children)},
    Node::Title{children} => {print!("Title\n"); Some(children)},
    Node::Subtitle{children} => {print!("Subtitle\n"); Some(children)},
    Node::Section{children} => {print!("Section\n"); Some(children)},
    Node::Statement{children} => {print!("Statement\n"); Some(children)},
    Node::StatementOrExpression{children} => {print!("StatementOrExpression\n"); Some(children)},
    Node::Body{children} => {print!("Body\n"); Some(children)},
    Node::Head{children} => {print!("Head\n"); Some(children)},
    Node::Node{children} => {print!("Node\n"); Some(children)},
    Node::Whitespace{children} => {print!("Whitespace\n"); Some(children)},
    Node::SpaceOrNewline{children} => {print!("SpaceOrNewline\n"); Some(children)},
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
    where F: Fn(&mut ParseState) -> &mut ParseState 
  {
    if !self.ok() {
      self
    } else {
      let mut before = self.clone();
      spacer(self.depth); println!("And");
      let result = production(self);
      result.depth = before.depth;
      result
    }
  }

  pub fn or<F>(&mut self, production: F) -> &mut ParseState
    where F: Fn(&mut ParseState) -> &mut ParseState 
  {
    if self.ok() {
      self
    } else {
      let mut before = self.clone();
      let old_depth = self.depth.clone();
      let previous = self.last_match.clone();
      let old_position = self.position;


      self.depth += 1;
      spacer(self.depth); println!("OR");
      self.status = ParseStatus::Parsing;
      let result = production(self);

      if result.ok() {
        result.last_match = result.node_stack.len();
      } else { 
        result.position = old_position;
        result.last_match = previous;
      }      



      result
    }
  }

  pub fn optional<F>(&mut self, production: F) -> &mut ParseState
    where F: Fn(&mut ParseState) -> &mut ParseState 
  {
    let before_depth = self.depth;
    self.depth += 1;
    spacer(self.depth); println!("Optional");
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

  pub fn repeat<F>(&mut self, production: F) -> &mut ParseState 
    where F: Fn(&mut ParseState) -> &mut ParseState
  {
    self.depth += 1; 
    spacer(self.depth); println!("Repeat");
    let mut once = false;
    let mut result = self;
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

  pub fn optional_repeat<F>(&mut self, production: F) -> &mut ParseState 
    where F: Fn(&mut ParseState) -> &mut ParseState
  {
    self.depth += 1; 
    let before_depth = self.depth;
    spacer(self.depth); println!("Optional Repeat");
    let mut result = self;
    let start_pos = result.last_match.clone();
    while result.ok() {
      let result = production(result);
      if result.ok() {
        result.depth = before_depth;
      }
    }
    result.status = ParseStatus::Parsing;
    let node = Node::Repeat{ children: result.node_stack.drain(start_pos..).collect() };
    result.node_stack.push(node);
    result.last_match = result.node_stack.len();
    result
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
  pub parse_tree: Node,
}

impl Parser {

  pub fn new() -> Parser {
    Parser {
      status: ParseStatus::Ready,
      tokens: Vec::new(),
      parse_tree: Node::Root{ children: Vec::new()  },
    }
  }

  pub fn add_tokens(&mut self, tokens: &mut Vec<Token>) {
    self.tokens.append(tokens);
  }

  pub fn build_parse_tree(&mut self) {
    let mut s = ParseState::new();
    s.token_stack.append(&mut self.tokens);
    let result = node(&mut s).and(statement_or_expression).or(program).and(end);
    //println!("{:?}",result);
    if result.ok() {
      self.status = ParseStatus::Ready;
      self.parse_tree = result.node_stack.pop().unwrap();
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
    write!(f, "{:?}", self.parse_tree);
    write!(f, "└───────────────────────────────────────┘\n").unwrap();
    Ok(())
  }
}

// ## Parse Nodes

// These nodes represent interior connections in the parse tree.

node!{whitespace, Whitespace, |s|{ node(s).optional_repeat(space).and(newline) }, "Whitespace"}
node!{space_or_newline, SpaceOrNewline, |s|{ space(s).or(newline) }, "SpaceOrNewline"}
node!{program, Program, |s|{ node(s).optional(head).and(body) }, "Program"}
node!{head, Head, |s|{ node(s).and(title).and(newline).optional_repeat(whitespace) }, "Head"}
node!{alphanumeric, Alphanumeric, |s|{ alpha(s).or(digit) }, "Alphanumeric"}
node!{word, Word, |s|{ node(s).repeat(alphanumeric) }, "Word"}
node!{paragraph, Paragraph, |s|{ word(s).optional(space).optional(paragraph) }, "Paragraph"}



node!{title, Title, |s|{ hashtag(s).and(space).and(paragraph) }, "Title"}
node!{subtitle, Subtitle, |s|{ hashtag(s).and(hashtag).and(space).and(identifier).and(newline) }, "Subtitle"}
node!{body, Body, |s|{ node(s).repeat(section) }, "Body"}
node!{section, Section, |s|{ node(s).optional(subtitle).repeat(block) }, "Section"}
node!{block, Block, |s|{ node(s).repeat(constraint) }, "Block"}
node!{constraint, Constraint, |s|{ node(s).and(space).and(space).and(statement_or_expression).optional(newline) }, "Constraint"}
node!{statement_or_expression, StatementOrExpression, |s|{ statement(s).or(expression) }, "StatementOrExpression"}
node!{statement, Statement, |s|{ column_define(s) }, "Statement"}
node!{column_define, ColumnDefine, |s|{ data(s).and(space).and(equal).and(space).and(expression) }, "ColumnDefine"}
node!{constant, Constant, |s|{ digit(s) }, "Constant"}
node!{infix, Infix, |s|{ plus(s).or(dash).or(asterisk).or(backslash) }, "Infix"}
node!{math_expression, MathExpression, |s|{ data(s).and(space).and(infix).and(space).and(data) }, "Math Expression"}
node!{expression, Expression, |s|{ math_expression(s).or(data).or(constant) }, "Expression"}
node!{equality, Equality, |s| { data(s).and(space).and(equal).and(space).and(expression) }, "Equality"}
node!{data, Data, |s| { table(s).or(identifier).or(constant).optional(index) }, "Data"}
node!{index, Index, |s| { dot_index(s).or(bracket_index) }, "Index"}
node!{bracket_index, BracketIndex, |s| { left_bracket(s).and(digit).and(right_bracket) }, "Bracket Index"}
node!{dot_index, DotIndex, |s| { period(s).and(digit).or(identifier) }, "Dot Index"}
node!{table, Table, |s| { hashtag(s).and(identifier) }, "Table"}
node!{identifier, Identifier, |s| { node(s).repeat(alpha) }, "Identifier"}

// ## Parse Leaves

leaf!{alpha, Token::Alpha}
leaf!{digit, Token::Digit}
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
leaf!{newline, Token::Newline}
 
// A dummy node that returns itself.
pub fn node(s: &mut ParseState) -> &mut ParseState {
  s
}

// Matches the end of stream token
pub fn end(s: &mut ParseState) -> &mut ParseState {
  let old_depth = s.depth;
  s.depth += 1; 
  spacer(s.depth); println!("End");
  let result = token(s, Token::EndOfStream);
  if result.ok() {
    result.node_stack.pop();
    let node = Node::Root{children: result.node_stack.drain(..).collect()};
    result.node_stack.push(node);
  }
  result.depth = old_depth;
  result
}

// Matches and token from the lexer step.
pub fn token(s: &mut ParseState, token: Token) -> &mut ParseState {
  s.depth += 1; 
  spacer(s.depth); print!("Token: [{:?}] = {:?}?", s.token_stack[s.position], token);
  if s.token_stack[s.position] == token {
    s.position += 1;
    s.last_match += 1;
    s.node_stack.push(Node::Token{token});
    println!(" √");
  } else {
    s.status = ParseStatus::Error(ParseError{code: 1, position: s.position, token: s.token_stack[s.position].clone(), node_stack: s.node_stack.clone() });
    println!(" X");
  }
  s
}
