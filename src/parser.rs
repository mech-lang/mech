// # Parer

// ## Prelude

use lexer::Token;
use lexer::Token::{HashTag, Alpha, Period, LeftBracket, RightBracket, Newline,
                   Digit, Space, Equal, Plus, EndOfStream, Dash, Asterisk, Slash};
use mech_core::{Hasher, Function};
use alloc::fmt;
use alloc::string::String;
use alloc::vec::Vec;

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
  VariableDefine { children: Vec<Node> },
  TableDefine { children: Vec<Node> },
  AddRow { children: Vec<Node> },
  Column { children: Vec<Node> },
  IdentifierOrConstant { children: Vec<Node> },
  Table { children: Vec<Node> },
  Number { children: Vec<Node> },
  DigitOrComma {children: Vec<Node> },
  FloatingPoint {children: Vec<Node> },
  MathExpression { children: Vec<Node> },
  SelectExpression { children: Vec<Node> },
  FilterExpression { children: Vec<Node> },
  Comparator { children: Vec<Node> },
  InfixOperation { children: Vec<Node>},
  Repeat{ children: Vec<Node> },
  TableIdentifier{ children: Vec<Node> },
  Identifier{ children: Vec<Node> },
  Alpha{ children: Vec<Node> },
  DotIndex{ children: Vec<Node> },
  SubscriptIndex{ children: Vec<Node> },
  SubscriptList{ children: Vec<Node> },
  Subscript{ children: Vec<Node> },
  LogicOperator{ children: Vec<Node> },
  LogicExpression{ children: Vec<Node> },
  Range{ children: Vec<Node> },
  SelectAll{ children: Vec<Node> },
  Index{ children: Vec<Node> },
  Data{ children: Vec<Node> },
  SetData{ children: Vec<Node> },
  SetOperator{ children: Vec<Node> },
  AddOperator{ children: Vec<Node> },
  WatchOperator {children: Vec<Node>},
  Equality{ children: Vec<Node> },
  Expression{ children: Vec<Node> },
  AnonymousTable{ children: Vec<Node> },
  TableRow{ children: Vec<Node> },
  Binding{ children: Vec<Node> },
  Attribute{ children: Vec<Node> },
  TableHeader{ children: Vec<Node> },
  InlineTable{ children: Vec<Node> },
  Constant{ children: Vec<Node> },
  Infix{ children: Vec<Node> },
  Program{ children: Vec<Node> },
  Title{ children: Vec<Node> },
  Subtitle{ children: Vec<Node> },
  Head{ children: Vec<Node> },
  Body{ children: Vec<Node> },
  Statement{ children: Vec<Node> },
  StatementOrExpression{ children: Vec<Node> },
  DataOrConstant{ children: Vec<Node> },
  IdentifierCharacter{ children: Vec<Node> },
  Fragment{ children: Vec<Node> },
  Node{ children: Vec<Node> },
  NewLineOrEnd{ children: Vec<Node> },
  Alphanumeric{ children: Vec<Node> },
  Paragraph{ children: Vec<Node> },
  String{ children: Vec<Node> },
  Word{ children: Vec<Node> },
  Section{ children: Vec<Node> },
  ProseOrCode{ children: Vec<Node> },
  Whitespace{ children: Vec<Node> },
  SpaceOrTab{ children: Vec<Node> },
  NewLine{ children: Vec<Node> },
  Text{ children: Vec<Node> },
  L1Infix{ children: Vec<Node> },
  L2Infix{ children: Vec<Node> },
  L3Infix{ children: Vec<Node> },
  L1{ children: Vec<Node> },
  L2{ children: Vec<Node> },
  L3{ children: Vec<Node> },
  L4{ children: Vec<Node> },
  Negation{ children: Vec<Node> },
  ParentheticalExpression{ children: Vec<Node> },
  CommentSigil{ children: Vec<Node> },
  Comment{children: Vec<Node>},
  Any{children: Vec<Node>},
  Symbol{children: Vec<Node>},
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
    Node::MathExpression{children} => {print!("MathExpression\n"); Some(children)},
    Node::SelectExpression{children} => {print!("SelectExpression\n"); Some(children)},
    Node::Comparator{children} => {print!("Comparator\n"); Some(children)},
    Node::FilterExpression{children} => {print!("FilterExpression\n"); Some(children)},
    Node::AnonymousTable{children} => {print!("AnonymousTable\n"); Some(children)},
    Node::TableRow{children} => {print!("TableRow\n"); Some(children)},
    Node::Table{children} => {print!("Table\n"); Some(children)},
    Node::Number{children} => {print!("Number\n"); Some(children)},
    Node::DigitOrComma{children} => {print!("DigitOrComma\n"); Some(children)},
    Node::FloatingPoint{children} => {print!("FloatingPoint\n"); Some(children)},
    Node::Alphanumeric{children} => {print!("Alphanumeric\n"); Some(children)},
    Node::Word{children} => {print!("Word\n"); Some(children)},
    Node::Paragraph{children} => {print!("Paragraph\n"); Some(children)},
    Node::String{children} => {print!("String\n"); Some(children)},
    Node::VariableDefine{children} => {print!("VariableDefine\n"); Some(children)},
    Node::TableDefine{children} => {print!("TableDefine\n"); Some(children)},
    Node::AddRow{children} => {print!("AddRow\n"); Some(children)},
    Node::Column{children} => {print!("Column\n"); Some(children)},
    Node::Binding{children} => {print!("Binding\n"); Some(children)},
    Node::InlineTable{children} => {print!("InlineTable\n"); Some(children)},
    Node::TableHeader{children} => {print!("TableHeader\n"); Some(children)},
    Node::Attribute{children} => {print!("Attribute\n"); Some(children)},
    Node::IdentifierOrConstant{children} => {print!("IdentifierOrConstant\n"); Some(children)},
    Node::InfixOperation{children} => {print!("Infix\n"); Some(children)},
    Node::Repeat{children} => {print!("Repeat\n"); Some(children)},
    Node::Identifier{children} => {print!("Identifier\n"); Some(children)},
    Node::TableIdentifier{children} => {print!("TableIdentifier\n"); Some(children)},
    Node::DotIndex{children} => {print!("DotIndex\n"); Some(children)},
    Node::SubscriptIndex{children} => {print!("SubscriptIndex\n"); Some(children)},
    Node::SubscriptList{children} => {print!("SubscriptList\n"); Some(children)},
    Node::Subscript{children} => {print!("Subscript\n"); Some(children)},
    Node::LogicOperator{children} => {print!("LogicOperator\n"); Some(children)},
    Node::LogicExpression{children} => {print!("LogicExpression\n"); Some(children)},
    Node::Range{children} => {print!("Range\n"); Some(children)},
    Node::SelectAll{children} => {print!("SelectAll\n"); Some(children)},
    Node::Index{children} => {print!("Index\n"); Some(children)},
    Node::Equality{children} => {print!("Equality\n"); Some(children)},
    Node::Data{children} => {print!("Data\n"); Some(children)},
    Node::SetData{children} => {print!("SetData\n"); Some(children)},
    Node::SetOperator{children} => {print!("SetOperator\n"); Some(children)},
    Node::AddOperator{children} => {print!("AddOperator\n"); Some(children)},
    Node::WatchOperator{children} => {print!("WatchOperator\n"); Some(children)},
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
    Node::DataOrConstant{children} => {print!("DataOrConstant\n"); Some(children)},
    Node::NewLineOrEnd{children} => {print!("NewLineOrEnd\n"); Some(children)},
    Node::Fragment{children} => {print!("Fragment\n"); Some(children)},
    Node::Body{children} => {print!("Body\n"); Some(children)},
    Node::Head{children} => {print!("Head\n"); Some(children)},
    Node::Node{children} => {print!("Node\n"); Some(children)},
    Node::Text{children} => {print!("Text\n"); Some(children)},
    Node::L1Infix{children} => {print!("L1Infix\n"); Some(children)},
    Node::L2Infix{children} => {print!("L2Infix\n"); Some(children)},
    Node::L3Infix{children} => {print!("L3Infix\n"); Some(children)},
    Node::L1{children} => {print!("L1\n"); Some(children)},
    Node::L2{children} => {print!("L2\n"); Some(children)},
    Node::L3{children} => {print!("L3\n"); Some(children)},
    Node::L4{children} => {print!("L4\n"); Some(children)},
    Node::Negation{children} => {print!("Negation\n"); Some(children)},
    Node::ParentheticalExpression{children} => {print!("ParentheticalExpression\n"); Some(children)},
    Node::ProseOrCode{children} => {print!("ProseOrCode\n"); Some(children)},
    Node::Whitespace{children} => {print!("Whitespace\n"); Some(children)},
    Node::SpaceOrTab{children} => {print!("SpaceOrTab\n"); Some(children)},
    Node::NewLine{children} => {print!("NewLine\n"); Some(children)},
    Node::Token{token, byte} => {print!("Token({:?})\n", token); None},
    Node::CommentSigil{children} => {print!("CommentSigil\n"); Some(children)},
    Node::Comment{children} => {print!("Comment\n"); Some(children)},
    Node::Any{children} => {print!("Any\n"); Some(children)},
    Node::Symbol{children} => {print!("Symbol\n"); Some(children)},
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

node!{comment_sigil, CommentSigil, |s|{ slash(s).and(slash) }, "CommentSigil"}
node!{symbol, Symbol, |s|{ plus(s).or(hashtag).or(left_bracket).or(right_bracket).or(colon).or(comma).or(equal).or(slash).or(greater_than).or(less_than).or(tilde).or(left_brace).or(right_brace).or(asterisk).or(period) }, "Symbol"} // TODO fill out rest
node!{any, Any, |s|{ alphanumeric(s).or(space_or_tab).or(symbol) }, "Any"}
node!{comment, Comment, |s|{ comment_sigil(s).repeat(any) }, "CommentSigil"}

node!{root, Root, |s|{ program(s).or(fragment) }, "Root"}
node!{program, Program, |s|{ node(s).optional(head).and(body) }, "Program"}
node!{head, Head, |s|{ title(s) }, "Head"}
node!{title, Title, |s|{ hashtag(s).and(space).and(text).optional_repeat(whitespace) }, "Title"}
node!{paragraph, Paragraph, |s|{ text(s).repeat(whitespace) }, "Paragraph"}
node!{text, Text, |s|{ word(s).optional(space).optional(text) }, "Text"}
node!{word, Word, |s|{ node(s).repeat(alphanumeric) }, "Word"}
node!{alphanumeric, Alphanumeric, |s|{ alpha(s).or(digit) }, "Alphanumeric"}
node!{whitespace, Whitespace, |s|{ node(s).optional_repeat(space).and(newline) }, "Whitespace"}
node!{space_or_tab, SpaceOrTab, |s|{ space(s).or(tab) }, "SpaceOrTab"}
node!{body, Body, |s|{ node(s).repeat(section) }, "Body"}
node!{section, Section, |s|{ node(s).optional(subtitle).optional_repeat(whitespace).repeat(prose_or_code)}, "Section"}
node!{subtitle, Subtitle, |s|{ hashtag(s).and(hashtag).and(space).and(text).repeat(whitespace) }, "Subtitle"}
node!{prose_or_code, ProseOrCode, |s|{ block(s).or(paragraph).optional_repeat(whitespace) }, "ProseOrCode"}

node!{string, String, |s|{ quote(s).and(text).and(quote) }, "String"}

node!{fragment, Fragment, |s|{ statement_or_expression(s).or(end) }, "Fragment"}
node!{statement_or_expression, StatementOrExpression, |s|{ statement(s).or(expression) }, "StatementOrExpression"}
node!{expression, Expression, |s|{ filter_expression(s).or(string).or(range).or(logic_expression).or(inline_table).or(anonymous_table).or(math_expression) }, "Expression"}
node!{statement, Statement, |s|{ table_define(s).or(add_row).or(variable_define).or(data_watch).or(set_data) }, "Statement"}

node!{block, Block, |s|{ node(s).repeat(constraint) }, "Block"}
node!{constraint, Constraint, |s|{ space(s).and(space).optional(statement_or_expression).optional(comment).optional_repeat(newline) }, "Constraint"}

node!{set_data, SetData, |s|{ table(s).optional(index).and(space).and(set_operator).and(space).and(expression) }, "SetData"}
node!{set_operator, SetOperator, |s|{ colon(s).and(equal) }, "SetOperator"}
node!{add_operator, AddOperator, |s|{ plus(s).and(equal) }, "AddOperator"}
node!{watch_operator, WatchOperator, |s|{ tilde(s) }, "WatchOperator"}
node!{data_watch, DataWatch, |s|{ watch_operator(s).and(space).and(data) }, "DataWatch"}
node!{variable_define, VariableDefine, |s|{ identifier(s).and(space).and(equal).and(space).and(expression) }, "VariableDefine"}
node!{table_define, TableDefine, |s|{ table(s).and(space).and(equal).and(space).and(expression) }, "TableDefine"}
node!{add_row, AddRow, |s|{ table(s).and(space).and(add_operator).and(space).and(expression) }, "AddRow"}
node!{constant, Constant, |s|{ number(s) }, "Constant"}
node!{number, Number, |s|{ node(s).repeat(digit).optional_repeat(digit_or_comma).optional(floating_point) }, "Number"}
node!{floating_point, FloatingPoint, |s|{ period(s).repeat(digit) }, "FloatingPoint"}
node!{digit_or_comma, DigitOrComma, |s|{ comma(s).and(digit).and(digit).and(digit) }, "DigitOrComma"}
node!{identifier_or_constant, IdentifierOrConstant, |s|{ identifier(s).or(constant) }, "IdentifierOrConstant"}
node!{newline_or_end, NewLineOrEnd, |s|{ newline(s).or(end) }, "NewLineOrEnd"}

node!{l1_infix, L1Infix, |s|{ space(s).and(plus).or(dash).and(space).and(l2) }, "L1Infix"}
node!{l2_infix, L2Infix, |s|{ space(s).and(asterisk).or(slash).and(space).and(l3) }, "L2Infix"}
node!{l3_infix, L3Infix, |s|{ space(s).and(caret).and(space).and(l4) }, "L3Infix"}

node!{l1, L1, |s|{ l2(s).optional_repeat(l1_infix) }, "L1"}
node!{l2, L2, |s|{ l3(s).optional_repeat(l2_infix) }, "L2"}
node!{l3, L3, |s|{ l4(s).optional_repeat(l3_infix) }, "L3"}
node!{l4, L4, |s|{ data(s).or(constant).or(negation).or(parenthetical_expression) }, "L4"}

node!{negation, Negation, |s|{ dash(s).and(data).or(constant) }, "Negation"}
node!{parenthetical_expression, ParentheticalExpression, |s|{ left_parenthesis(s).and(l1).and(right_parenthesis) }, "ParentheticalExpression"}

node!{inline_table, InlineTable, |s|{ left_bracket(s).repeat(binding).and(right_bracket) }, "InlineTable"}

node!{anonymous_table, AnonymousTable, |s|{ left_bracket(s).optional_repeat(space).optional(table_header).optional_repeat(table_row).and(right_bracket) }, "AnonymousTable"}
node!{binding, Binding, |s|{ identifier(s).and(colon).optional_repeat(space).and(identifier_or_constant).optional_repeat(space).optional(comma).optional_repeat(space) }, "Binding"}
node!{attribute, Attribute, |s|{ identifier(s).optional_repeat(space).optional(comma).optional_repeat(space) }, "Attribute"}
node!{table_header, TableHeader, |s|{ node(s).and(bar).repeat(attribute).and(bar).optional_repeat(space).optional(newline) }, "TableHeader"}
node!{table_row, TableRow, |s|{ node(s).optional_repeat(space_or_tab).repeat(column).optional(semicolon).optional(newline) }, "TableRow"}
node!{column, Column, |s|{ node(s).optional_repeat(space_or_tab).and(data).or(expression).or(number).optional(comma).optional(space_or_tab) }, "Column"}
node!{math_expression, MathExpression, |s|{ l1(s) }, "MathExpression"}

node!{logic_expression, LogicExpression, |s|{ data_or_constant(s).and(space).and(logic_operator).and(space).and(data_or_constant)  }, "LogicExpression"}
node!{logic_operator, LogicOperator, |s|{ ampersand(s).or(bar) }, "LogicOperator"}

node!{filter_expression, FilterExpression, |s|{ data_or_constant(s).and(space).and(comparator).and(space).and(data_or_constant) }, "FilterExpression"}
node!{comparator, Comparator, |s|{ greater_than(s).or(less_than) }, "Comparator"}

node!{data_or_constant, DataOrConstant, |s|{ data(s).or(constant) }, "DataOrConstant"}
node!{equality, Equality, |s| { data(s).and(space).and(equal).and(space).and(expression) }, "Equality"}
node!{data, Data, |s| { table(s).or(identifier).optional(index) }, "Data"}
node!{index, Index, |s| { dot_index(s).or(subscript_index) }, "Index"}
node!{subscript_index, SubscriptIndex, |s| { left_brace(s).repeat(subscript).and(right_brace) }, "Subscript Index"}
node!{subscript_list, SubscriptList, |s| { node(s).repeat(subscript) }, "SubscriptList"} 
node!{subscript, Subscript, |s| { select_all(s).or(range).or(expression).optional_repeat(space).optional(comma).optional_repeat(space)   }, "Subscript"} 
node!{range, Range, |s| { math_expression(s).optional_repeat(space).and(colon).optional_repeat(space).and(math_expression) }, "Range"}
node!{select_all, SelectAll, |s| { colon(s) }, "SelectAll"}
node!{dot_index, DotIndex, |s| { period(s).and(identifier).optional(subscript_index) }, "Dot Index"}
node!{table, Table, |s| { hashtag(s).and(table_identifier) }, "Table"}
node!{identifier_character, IdentifierCharacter, |s| { alphanumeric(s).or(slash).or(dash) }, "IdentifierCharacter"}
node!{identifier, Identifier, |s| { alpha(s).optional_repeat(identifier_character) }, "Identifier"}
node!{table_identifier, TableIdentifier, |s| { alpha(s).optional_repeat(identifier_character) }, "TableIdentifier"}
node!{newline, NewLine, |s| { node(s).optional(carriage_return).and(new_line_char) }, "NewLine"}

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
leaf!{left_brace, Token::LeftBrace}
leaf!{right_brace, Token::RightBrace}
leaf!{equal, Token::Equal}
leaf!{less_than, Token::LessThan}
leaf!{greater_than, Token::GreaterThan}
leaf!{exclamation, Token::Exclamation}
leaf!{plus, Token::Plus}
leaf!{dash, Token::Dash}
leaf!{asterisk, Token::Asterisk}
leaf!{slash, Token::Slash}
leaf!{caret, Token::Caret}
leaf!{space, Token::Space}
leaf!{tab, Token::Tab}
leaf!{tilde, Token::Tilde}
leaf!{grave, Token::Grave}
leaf!{bar, Token::Bar}
leaf!{quote, Token::Quote}
leaf!{ampersand, Token::Ampersand}
leaf!{semicolon, Token::Semicolon}
leaf!{new_line_char, Token::Newline}
leaf!{carriage_return, Token::CarriageReturn}
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
