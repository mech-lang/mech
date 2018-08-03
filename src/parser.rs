// # Parer

// ## Prelude

use lexer::Token;
use lexer::Token::{HashTag, Alpha, Period, LeftBracket, RightBracket, Newline,
                   Digit, Space, Equal, Plus, EndOfStream, Dash, Asterisk, Slash};
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
      // spacer(s.depth); println!($label);
      let previous = s.last_match.clone();
      let old_position = s.position;
      let result = $production(s);
      let node = Node::$node{ children: result.node_stack.drain(previous..).collect() };
      if result.ok() {
        result.node_stack.push(node);
        result.last_match = result.node_stack.len();
        // spacer(old_depth + 1); print!($label); println!(" √");
      } else { 
        // spacer(old_depth + 1); print!($label); println!(" X");
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
  DataWatch { children: Vec<Node> },
  Insert { children: Vec<Node> },
  ColumnDefine { children: Vec<Node> },
  TableDefine { children: Vec<Node> },
  TableDefineRHS { children: Vec<Node> },
  RowDefine { children: Vec<Node> },
  Column { children: Vec<Node> },
  Binding { children: Vec<Node> },
  IdentifierOrNumber { children: Vec<Node> },
  Table { children: Vec<Node> },
  Number { children: Vec<Node> },
  MathExpression { children: Vec<Node> },
  SelectExpression { children: Vec<Node> },
  InfixOperation { children: Vec<Node>},
  Repeat{ children: Vec<Node> },
  Identifier{ children: Vec<Node> },
  Alpha{ children: Vec<Node> },
  DotIndex{ children: Vec<Node> },
  BracketIndex{ children: Vec<Node> },
  Index{ children: Vec<Node> },
  Data{ children: Vec<Node> },
  SelectData{ children: Vec<Node> },
  SetData{ children: Vec<Node> },
  SetOperator{ children: Vec<Node> },
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
  IdentifierCharacter{ children: Vec<Node> },
  Fragment{ children: Vec<Node> },
  Node{ children: Vec<Node> },
  NewLineOrEnd{ children: Vec<Node> },
  Alphanumeric{ children: Vec<Node> },
  Paragraph{ children: Vec<Node> },
  Word{ children: Vec<Node> },
  Section{ children: Vec<Node> },
  LHS{ children: Vec<Node> },
  RHS{ children: Vec<Node> },
  ProseOrCode{ children: Vec<Node> },
  Whitespace{ children: Vec<Node> },
  Text{ children: Vec<Node> },
  L1Infix{ children: Vec<Node> },
  L2Infix{ children: Vec<Node> },
  L3Infix{ children: Vec<Node> },
  L1{ children: Vec<Node> },
  L2{ children: Vec<Node> },
  L3{ children: Vec<Node> },
  L4{ children: Vec<Node> },
  Token{token: Token, byte: u8},
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
    Node::DataWatch{children} => {print!("DataWatch\n"); Some(children)},
    Node::Insert{children} => {print!("Insert\n"); Some(children)},
    Node::MathExpression{children} => {print!("Math Expression\n"); Some(children)},
    Node::SelectExpression{children} => {print!("Select Expression\n"); Some(children)},
    Node::Table{children} => {print!("Table\n"); Some(children)},
    Node::Number{children} => {print!("Number\n"); Some(children)},
    Node::Alphanumeric{children} => {print!("Alphanumeric\n"); Some(children)},
    Node::Word{children} => {print!("Word\n"); Some(children)},
    Node::Paragraph{children} => {print!("Paragraph\n"); Some(children)},
    Node::ColumnDefine{children} => {print!("ColumnDefine\n"); Some(children)},
    Node::TableDefine{children} => {print!("TableDefine\n"); Some(children)},
    Node::TableDefineRHS{children} => {print!("TableDefineRHS\n"); Some(children)},
    Node::RowDefine{children} => {print!("RowDefine\n"); Some(children)},
    Node::Column{children} => {print!("Column\n"); Some(children)},
    Node::Binding{children} => {print!("Binding\n"); Some(children)},
    Node::IdentifierOrNumber{children} => {print!("IdentifierOrNumber\n"); Some(children)},
    Node::InfixOperation{children} => {print!("Infix\n"); Some(children)},
    Node::Repeat{children} => {print!("Repeat\n"); Some(children)},
    Node::Identifier{children} => {print!("Identifier\n"); Some(children)},
    Node::DotIndex{children} => {print!("DotIndex\n"); Some(children)},
    Node::BracketIndex{children} => {print!("BracketIndex\n"); Some(children)},
    Node::Index{children} => {print!("Index\n"); Some(children)},
    Node::Equality{children} => {print!("Equality\n"); Some(children)},
    Node::Data{children} => {print!("Data\n"); Some(children)},
    Node::SetData{children} => {print!("SetData\n"); Some(children)},
    Node::SetOperator{children} => {print!("SetOperator\n"); Some(children)},
    Node::SelectData{children} => {print!("SelectData\n"); Some(children)},
    Node::Infix{children} => {print!("Infix\n"); Some(children)},
    Node::Expression{children} => {print!("Expression\n"); Some(children)},
    Node::Constant{children} => {print!("Constant\n"); Some(children)},
    Node::Program{children} => {print!("Program\n"); Some(children)},
    Node::IdentifierCharacter{children} => {print!("IdentifierCharacter\n"); Some(children)},
    Node::Title{children} => {print!("Title\n"); Some(children)},
    Node::Subtitle{children} => {print!("Subtitle\n"); Some(children)},
    Node::Section{children} => {print!("Section\n"); Some(children)},
    Node::Statement{children} => {print!("Statement\n"); Some(children)},
    Node::StatementOrExpression{children} => {print!("StatementOrExpression\n"); Some(children)},
    Node::NewLineOrEnd{children} => {print!("NewLineOrEnd\n"); Some(children)},
    Node::Fragment{children} => {print!("Fragment\n"); Some(children)},
    Node::Body{children} => {print!("Body\n"); Some(children)},
    Node::Head{children} => {print!("Head\n"); Some(children)},
    Node::Node{children} => {print!("Node\n"); Some(children)},
    Node::Text{children} => {print!("Text\n"); Some(children)},
    Node::RHS{children} => {print!("RHS\n"); Some(children)},
    Node::LHS{children} => {print!("LHS\n"); Some(children)},    
    Node::L1Infix{children} => {print!("L1Infix\n"); Some(children)},
    Node::L2Infix{children} => {print!("L2Infix\n"); Some(children)},
    Node::L3Infix{children} => {print!("L3Infix\n"); Some(children)},
    Node::L1{children} => {print!("L1\n"); Some(children)},
    Node::L2{children} => {print!("L2\n"); Some(children)},
    Node::L3{children} => {print!("L3\n"); Some(children)},
    Node::L4{children} => {print!("L4\n"); Some(children)},
    Node::ProseOrCode{children} => {print!("ProseOrCode\n"); Some(children)},
    Node::Whitespace{children} => {print!("Whitespace\n"); Some(children)},
    Node::Token{token, byte} => {print!("Token({:?})\n", token); None},
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
  pub text: String,
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
      text: String::from(""),
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
      // spacer(self.depth); println!("And");
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
      // spacer(self.depth); println!("OR");
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
    // spacer(self.depth); println!("Optional");
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
    let before_depth = self.depth;
    // spacer(self.depth); println!("Repeat");
    let mut once = false;
    let mut result = self;
    let start_pos = result.last_match.clone();
    while result.ok() {
      let result = production(result);
      if result.ok() {
        result.depth = before_depth;
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
    let before_status = self.status.clone();
    let before_depth = self.depth;
    // spacer(self.depth); println!("Optional Repeat");
    let mut result = self;
    let start_pos = result.last_match.clone();
    while result.ok() {
      let result = production(result);
      if result.ok() {
        result.depth = before_depth;
      }
    }
    let node = Node::Repeat{ children: result.node_stack.drain(start_pos..).collect() };
    result.status = before_status;
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
  pub text: String,
}

impl Parser {

  pub fn new() -> Parser {
    Parser {
      status: ParseStatus::Ready,
      text: String::from(""),
      tokens: Vec::new(),
      parse_tree: Node::Root{ children: Vec::new()  },
    }
  }

  pub fn add_tokens(&mut self, tokens: &mut Vec<Token>) {
    self.tokens.append(tokens);
  }

  pub fn build_parse_tree(&mut self) {
    let mut s = ParseState::new();
    s.text = self.text.clone();
    s.token_stack.append(&mut self.tokens);
    let result = root(&mut s).or(program);
    
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

node!{root, Root, |s|{ program(s).or(fragment) }, "Root"}
node!{program, Program, |s|{ node(s).optional(head).and(body) }, "Program"}
node!{head, Head, |s|{ title(s) }, "Head"}
node!{title, Title, |s|{ hashtag(s).and(space).and(text).optional_repeat(whitespace) }, "Title"}
node!{paragraph, Paragraph, |s|{ text(s).repeat(whitespace) }, "Paragraph"}
node!{text, Text, |s|{ word(s).optional(space).optional(text) }, "Text"}
node!{word, Word, |s|{ node(s).repeat(alphanumeric) }, "Word"}
node!{alphanumeric, Alphanumeric, |s|{ alpha(s).or(digit) }, "Alphanumeric"}
node!{whitespace, Whitespace, |s|{ node(s).optional_repeat(space).and(newline) }, "Whitespace"}
node!{body, Body, |s|{ node(s).repeat(section) }, "Body"}
node!{section, Section, |s|{ node(s).optional(subtitle).optional_repeat(whitespace).repeat(prose_or_code)}, "Section"}
node!{subtitle, Subtitle, |s|{ hashtag(s).and(hashtag).and(space).and(text).repeat(whitespace) }, "Subtitle"}
node!{prose_or_code, ProseOrCode, |s|{ block(s).or(paragraph).optional_repeat(whitespace) }, "ProseOrCode"}

node!{block, Block, |s|{ node(s).repeat(constraint) }, "Block"}
node!{constraint, Constraint, |s|{ space(s).and(space).optional(statement_or_expression).optional_repeat(newline) }, "Constraint"}
node!{fragment, Fragment, |s|{ statement_or_expression(s).or(end) }, "Fragment"}
node!{statement_or_expression, StatementOrExpression, |s|{ statement(s).or(expression) }, "StatementOrExpression"}
node!{statement, Statement, |s|{ table_define(s).or(column_define).or(data_watch).or(set_data) }, "Statement"}
node!{set_data, SetData, |s|{ data(s).and(space).and(set_operator).and(space).and(expression) }, "SetData"}
node!{set_operator, SetOperator, |s|{ colon(s).and(equal) }, "SetOperator"}
node!{data_watch, DataWatch, |s|{ tilde(s).and(space).and(data) }, "DataWatch"}
node!{column_define, ColumnDefine, |s|{ lhs(s).and(space).and(equal).and(space).and(rhs) }, "ColumnDefine"}
node!{table_define, TableDefine, |s|{ table(s).and(space).and(equal).and(space).and(table_define_rhs) }, "TableDefine"}
node!{constant, Constant, |s|{ number(s) }, "Constant"}
node!{number, Number, |s|{ node(s).repeat(digit) }, "Number"}
node!{lhs, LHS, |s|{ data(s) }, "LHS"}
node!{rhs, RHS, |s|{ expression(s) }, "RHS"}
node!{table_define_rhs, TableDefineRHS, |s|{ expression(s).or(row_define) }, "TableDefineRHS"}
node!{row_define, RowDefine, |s|{ left_bracket(s).optional_repeat(column).and(right_bracket) }, "RowDefine"}
node!{column, Column, |s|{ identifier(s).optional(binding).optional(comma).optional(space) }, "Column"}
node!{binding, Binding, |s|{ colon(s).and(space).and(identifier_or_number) }, "Binding"}
node!{identifier_or_number, IdentifierOrNumber, |s|{ identifier(s).or(number) }, "IdentifierOrNumber"}
node!{newline_or_end, NewLineOrEnd, |s|{ newline(s).or(end) }, "NewLineOrEnd"}

node!{l1_infix, L1Infix, |s|{ space(s).and(plus).or(dash).and(space).and(l2) }, "L1Infix"}
node!{l2_infix, L2Infix, |s|{ space(s).and(asterisk).or(slash).and(space).and(l3) }, "L2Infix"}
node!{l3_infix, L3Infix, |s|{ space(s).and(caret).and(space).and(l4) }, "L3Infix"}

node!{l1, L1, |s|{ l2(s).optional_repeat(l1_infix) }, "L1"}
node!{l2, L2, |s|{ l3(s).optional_repeat(l2_infix) }, "L2"}
node!{l3, L3, |s|{ l4(s).optional_repeat(l3_infix) }, "L3"}
node!{l4, L4, |s|{ select_data(s).or(constant) }, "L4"}

node!{math_expression, MathExpression, |s|{ l1(s).and(newline_or_end) }, "Math Expression"}
node!{select_expression, SelectExpression, |s|{ data(s).and(newline_or_end) }, "Select Expression"}
node!{expression, Expression, |s|{ select_expression(s).or(math_expression) }, "Expression"}
node!{equality, Equality, |s| { data(s).and(space).and(equal).and(space).and(expression) }, "Equality"}
node!{select_data, SelectData, |s| { data(s) }, "SelectData"}
node!{data, Data, |s| { table(s).or(identifier).optional(index) }, "Data"}
node!{index, Index, |s| { dot_index(s).or(bracket_index) }, "Index"}
node!{bracket_index, BracketIndex, |s| { left_bracket(s).and(number).and(right_bracket) }, "Bracket Index"}
node!{dot_index, DotIndex, |s| { period(s).and(number).or(identifier) }, "Dot Index"}
node!{table, Table, |s| { hashtag(s).and(identifier) }, "Table"}
node!{identifier_character, IdentifierCharacter, |s| { alphanumeric(s).or(slash).or(dash) }, "IdentifierCharacter"}
node!{identifier, Identifier, |s| { alpha(s).optional_repeat(identifier_character) }, "Identifier"}

// ## Parse Leaves

leaf!{alpha, Token::Alpha}
leaf!{digit, Token::Digit}
leaf!{hashtag, Token::HashTag}
leaf!{period, Token::Period}
leaf!{colon, Token::Colon}
leaf!{comma, Token::Comma}
leaf!{left_bracket, Token::LeftBracket}
leaf!{right_bracket, Token::RightBracket}
leaf!{left_parenthesis, Token::LeftParenthesis}
leaf!{right_parenthesis, Token::RightParenthesis}
leaf!{equal, Token::Equal}
leaf!{plus, Token::Plus}
leaf!{dash, Token::Dash}
leaf!{asterisk, Token::Asterisk}
leaf!{slash, Token::Slash}
leaf!{caret, Token::Caret}
leaf!{space, Token::Space}
leaf!{tilde, Token::Tilde}
leaf!{newline, Token::Newline}
leaf!{end, Token::EndOfStream}


// A dummy node that returns itself.
pub fn node(s: &mut ParseState) -> &mut ParseState {
  s
}

// Matches a token from the lexer step.
pub fn token(s: &mut ParseState, token: Token) -> &mut ParseState {
  s.depth += 1; 
  // spacer(s.depth); print!("Token: [{:?}] = {:?}?", s.token_stack[s.position], token);
  if s.token_stack[s.position] == token {
    let byte = if s.position < s.text.len() {
      s.text.as_bytes()[s.position]
    } else {
      0
    };
    match token {
      Token::EndOfStream => (),
      _ => {
        s.position += 1;
        s.last_match += 1;
      },
    };
    s.node_stack.push(Node::Token{token, byte});
    // spacer(0); println!(" √");
  } else {
    s.status = ParseStatus::Error(ParseError{code: 1, position: s.position, token: s.token_stack[s.position].clone(), node_stack: s.node_stack.clone() });
    // spacer(0); println!(" X");
  }
  s
}
