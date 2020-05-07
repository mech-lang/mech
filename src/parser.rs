// # Parser

// ## Prelude

use lexer::Token;
use mech_core::{Hasher};
#[cfg(not(feature = "no-std"))] use core::fmt;
#[cfg(feature = "no-std")] use alloc::fmt;
#[cfg(feature = "no-std")] use alloc::string::String;
#[cfg(feature = "no-std")] use alloc::vec::Vec;
use nom::{
  IResult,
  branch::alt,
  sequence::tuple,
  combinator::opt,
  error::{context, convert_error, ErrorKind, ParseError, VerboseError},
  multi::{many1, many0},
  bytes::complete::{tag},
  character::complete::{alphanumeric1, alpha1, digit1, space0, space1},
};

// ## Parser Node

#[derive(Clone, PartialEq)]
pub enum Node {
  Root{ children: Vec<Node> },
  Block{ children: Vec<Node> },
  Constraint{ children: Vec<Node> },
  Select { children: Vec<Node> },
  Whenever { children: Vec<Node> },
  AsSoonAs { children: Vec<Node> },
  Until { children: Vec<Node> },
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
  Range,
  SelectAll,
  Index{ children: Vec<Node> },
  Data{ children: Vec<Node> },
  SetData{ children: Vec<Node> },
  SetOperator{ children: Vec<Node> },
  SplitData{ children: Vec<Node> },
  JoinData{ children: Vec<Node> },
  AddOperator{ children: Vec<Node> },
  WatchOperator {children: Vec<Node>},
  Equality{ children: Vec<Node> },
  Expression{ children: Vec<Node> },
  AnonymousTable{ children: Vec<Node> },
  TableRow{ children: Vec<Node> },
  Binding{ children: Vec<Node> },
  FunctionBinding{ children: Vec<Node> },
  Attribute{ children: Vec<Node> },
  TableHeader{ children: Vec<Node> },
  InlineTable{ children: Vec<Node> },
  Constant{ children: Vec<Node> },
  Infix{ children: Vec<Node> },
  Program{ children: Vec<Node> },
  Title{ children: Vec<Node> },
  Subtitle{ children: Vec<Node> },
  SectionTitle{ children: Vec<Node> },
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
  ParagraphText{ children: Vec<Node> },
  FormattedText{ children: Vec<Node> },
  InlineMechCode{ children: Vec<Node> },
  InlineCode{ children: Vec<Node> },
  Bold{ children: Vec<Node> },
  Italic{ children: Vec<Node> },
  Hyperlink{ children: Vec<Node> },
  BlockQuote{ children: Vec<Node> },
  CodeBlock{ children: Vec<Node> },
  MechCodeBlock{ children: Vec<Node> },
  UnorderedList{ children: Vec<Node> },
  ListItem{ children: Vec<Node> },
  String{ children: Vec<Node> },
  StringInterpolation{ children: Vec<Node> },
  Word{ children: Vec<Node> },
  Section{ children: Vec<Node> },
  ProseOrCode{ children: Vec<Node> },
  Whitespace{ children: Vec<Node> },
  SpaceOrTab{ children: Vec<Node> },
  NewLine{ children: Vec<Node> },
  Text{ children: Vec<Node> },
  Punctuation{ children: Vec<Node> },
  L0Infix{ children: Vec<Node> },
  L1Infix{ children: Vec<Node> },
  L2Infix{ children: Vec<Node> },
  L3Infix{ children: Vec<Node> },
  L4Infix{ children: Vec<Node> },
  L5Infix{ children: Vec<Node> },
  L0{ children: Vec<Node> },
  L1{ children: Vec<Node> },
  L2{ children: Vec<Node> },
  L3{ children: Vec<Node> },
  L4{ children: Vec<Node> },
  L5{ children: Vec<Node> },
  L6{ children: Vec<Node> },
  Function{ children: Vec<Node> },
  Negation{ children: Vec<Node> },
  ParentheticalExpression{ children: Vec<Node> },
  CommentSigil{ children: Vec<Node> },
  Comment{children: Vec<Node>},
  Any{children: Vec<Node>},
  Symbol{children: Vec<Node>},
  StateMachine{children: Vec<Node>},
  Transitions{children: Vec<Node>},
  Transition{children: Vec<Node>},
  Quantity{children: Vec<Node>},
  Token{token: Token, byte: u8},
  Add,
  Subtract,
  Multiply,
  Divide,
  Exponent,
  LessThanEqual,
  GreaterThanEqual,
  Equal,
  NotEqual,
  LessThan,
  GreaterThan,
  And,
  Or,
  Empty,
  Null,
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
    Node::Whenever{children} => {print!("Whenever\n"); Some(children)},
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
    Node::ParagraphText{children} => {print!("ParagraphText\n"); Some(children)},
    Node::FormattedText{children} => {print!("FormattedText\n"); Some(children)},
    Node::InlineMechCode{children} => {print!("InlineMechCode\n"); Some(children)},
    Node::InlineCode{children} => {print!("InlineCode\n"); Some(children)},
    Node::MechCodeBlock{children} => {print!("MechCodeBlock\n"); Some(children)},
    Node::Bold{children} => {print!("Bold\n"); Some(children)},
    Node::Italic{children} => {print!("Italic\n"); Some(children)},
    Node::Hyperlink{children} => {print!("Hyperlink\n"); Some(children)},
    Node::BlockQuote{children} => {print!("BlockQuote\n"); Some(children)},
    Node::CodeBlock{children} => {print!("CodeBlock\n"); Some(children)},
    Node::UnorderedList{children} => {print!("UnorderedList\n"); Some(children)},
    Node::ListItem{children} => {print!("ListItem\n"); Some(children)},
    Node::String{children} => {print!("String\n"); Some(children)},
    Node::StringInterpolation{children} => {print!("StringInterpolation\n"); Some(children)},
    Node::VariableDefine{children} => {print!("VariableDefine\n"); Some(children)},
    Node::TableDefine{children} => {print!("TableDefine\n"); Some(children)},
    Node::AddRow{children} => {print!("AddRow\n"); Some(children)},
    Node::Column{children} => {print!("Column\n"); Some(children)},
    Node::Binding{children} => {print!("Binding\n"); Some(children)},
    Node::FunctionBinding{children} => {print!("FunctionBinding\n"); Some(children)},
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
    Node::Range => {print!("Range\n"); None},
    Node::SelectAll => {print!("SelectAll\n"); None},
    Node::Index{children} => {print!("Index\n"); Some(children)},
    Node::Equality{children} => {print!("Equality\n"); Some(children)},
    Node::Data{children} => {print!("Data\n"); Some(children)},
    Node::SetData{children} => {print!("SetData\n"); Some(children)},
    Node::SplitData{children} => {print!("SplitData\n"); Some(children)},
    Node::JoinData{children} => {print!("JoinData\n"); Some(children)},
    Node::AsSoonAs{children} => {print!("AsSoonAs\n"); Some(children)},
    Node::Until{children} => {print!("Until\n"); Some(children)},
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
    Node::SectionTitle{children} => {print!("SectionTitle\n"); Some(children)},
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
    Node::Punctuation{children} => {print!("Punctuation\n"); Some(children)},
    Node::L0Infix{children} => {print!("L0Infix\n"); Some(children)},
    Node::L1Infix{children} => {print!("L1Infix\n"); Some(children)},
    Node::L2Infix{children} => {print!("L2Infix\n"); Some(children)},
    Node::L3Infix{children} => {print!("L3Infix\n"); Some(children)},
    Node::L4Infix{children} => {print!("L4Infix\n"); Some(children)},
    Node::L5Infix{children} => {print!("L5Infix\n"); Some(children)},
    Node::L0{children} => {print!("L0\n"); Some(children)},
    Node::L1{children} => {print!("L1\n"); Some(children)},
    Node::L2{children} => {print!("L2\n"); Some(children)},
    Node::L3{children} => {print!("L3\n"); Some(children)},
    Node::L4{children} => {print!("L4\n"); Some(children)},
    Node::L5{children} => {print!("L5\n"); Some(children)},
    Node::L6{children} => {print!("L6\n"); Some(children)},
    Node::Function{children} => {print!("Function\n"); Some(children)},
    Node::Negation{children} => {print!("Negation\n"); Some(children)},
    Node::ParentheticalExpression{children} => {print!("ParentheticalExpression\n"); Some(children)},
    Node::ProseOrCode{children} => {print!("ProseOrCode\n"); Some(children)},
    Node::Whitespace{children} => {print!("Whitespace\n"); Some(children)},
    Node::SpaceOrTab{children} => {print!("SpaceOrTab\n"); Some(children)},
    Node::NewLine{children} => {print!("NewLine\n"); Some(children)},
    Node::Token{token, byte} => {print!("Token({:?} ({:?}))\n", token, byte); None},
    Node::CommentSigil{children} => {print!("CommentSigil\n"); Some(children)},
    Node::Comment{children} => {print!("Comment\n"); Some(children)},
    Node::Any{children} => {print!("Any\n"); Some(children)},
    Node::Symbol{children} => {print!("Symbol\n"); Some(children)},
    Node::Quantity{children} => {print!("Quantity\n"); Some(children)},
    Node::StateMachine{children} => {print!("StateMachine\n"); Some(children)},
    Node::Transitions{children} => {print!("Transitions\n"); Some(children)},
    Node::Transition{children} => {print!("Transition\n"); Some(children)},
    Node::Add => {print!("Add\n",); None},
    Node::Subtract => {print!("Subtract\n",); None},
    Node::Multiply => {print!("Multiply\n",); None},
    Node::Divide => {print!("Divide\n",); None},
    Node::Exponent => {print!("Exponent\n",); None},
    Node::LessThan => {print!("LessThan\n",); None},
    Node::GreaterThan => {print!("GreaterThan\n",); None},
    Node::GreaterThanEqual => {print!("GreaterThanEqual\n",); None},
    Node::LessThanEqual => {print!("LessThanEqual\n",); None},
    Node::Equal => {print!("Equal\n",); None},
    Node::NotEqual => {print!("NotEqual\n",); None},
    Node::And => {print!("And\n",); None},
    Node::Or => {print!("Or\n",); None},
    Node::Empty => {print!("Empty\n",); None},
    Node::Null => {print!("Null\n",); None},
    Node::Alpha{children} => {print!("Alpha\n"); Some(children)},
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

#[derive(Clone)]
pub struct Parser {
  pub tokens: Vec<Token>,
  pub parse_tree: Node,
  pub unparsed: String,
  pub text: String,
}

impl Parser {

  pub fn new() -> Parser {
    Parser {
      text: String::from(""),
      tokens: Vec::new(),
      unparsed: String::from(""),
      parse_tree: Node::Root{ children: Vec::new()  },
    }
  }

  pub fn add_tokens(&mut self, tokens: &mut Vec<Token>) {
    self.tokens.append(tokens);
  }

  pub fn parse(&mut self, text: &str) {
    let parse_tree = parse_mech(text);
    match parse_tree {
      Ok((rest, tree)) => {
        self.unparsed = rest.to_string();
        self.parse_tree = tree;
      },
      Err(q) => (), 
    }
  }

  pub fn parse_block(&mut self, text: &str) {
    let parse_tree = parse_block(text);
    match parse_tree {
      Ok((rest, tree)) => {
        self.unparsed = rest.to_string();
        self.parse_tree = tree;
      },
      _ => (), 
    }
  }

  pub fn parse_fragment(&mut self, text: &str) -> Result<(),()> {
    let parse_tree = parse_fragment(text);
    match parse_tree {
      Ok((rest, tree)) => {
        self.unparsed = rest.to_string();
        self.parse_tree = tree;
        Ok(())
      },
      Err(x) => Err(()), 
    }
  }
}

impl fmt::Debug for Parser {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    
    write!(f, "┌───────────────────────────────────────┐\n").unwrap();
    write!(f, "│ Parser\n").unwrap();
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

macro_rules! leaf {
  ($name:ident, $byte:expr, $token:expr) => (
    fn $name(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
      let (input, byte) = tag($byte)(input)?;
      Ok((input, Node::Token{token: $token, byte: (byte.as_bytes())[0]}))
    }
  )
}

leaf!{at, "@", Token::At}
leaf!{hashtag, "#", Token::HashTag}
leaf!{period, ".", Token::Period}
leaf!{colon, ":", Token::Colon}
leaf!{comma, ",", Token::Comma}
leaf!{apostrophe, "'", Token::Apostrophe}
leaf!{left_bracket, "[", Token::LeftBracket}
leaf!{right_bracket, "]", Token::RightBracket}
leaf!{left_parenthesis, "(", Token::LeftParenthesis}
leaf!{right_parenthesis, ")", Token::RightParenthesis}
leaf!{left_brace, "{", Token::LeftBrace}
leaf!{right_brace, "}", Token::RightBrace}
leaf!{equal, "=", Token::Equal}
leaf!{left_angle, "<", Token::LessThan}
leaf!{right_angle, ">", Token::GreaterThan}
leaf!{exclamation, "!", Token::Exclamation}
leaf!{question, "?", Token::Question}
leaf!{plus, "+", Token::Plus}
leaf!{dash, "-", Token::Dash}
leaf!{underscore, "_", Token::Underscore}
leaf!{asterisk, "*", Token::Asterisk}
leaf!{slash, "/", Token::Slash}
leaf!{caret, "^", Token::Caret}
leaf!{space, " ", Token::Space}
leaf!{tab, "\t", Token::Tab}
leaf!{tilde, "~", Token::Tilde}
leaf!{grave, "`", Token::Grave}
leaf!{bar, "|", Token::Bar}
leaf!{quote, "\"", Token::Quote}
leaf!{ampersand, "&", Token::Ampersand}
leaf!{semicolon, ";", Token::Semicolon}
leaf!{new_line_char, "\n", Token::Newline}
leaf!{carriage_return, "\r", Token::CarriageReturn}

// ## The Basics

fn word(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, bytes) = alpha1(input)?;
  let chars = bytes.chars().map(|b| Node::Token{token: Token::Alpha, byte: b as u8}).collect();
  Ok((input, Node::Word{children: chars}))
}

fn number(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, bytes) = digit1(input)?;
  let digits = bytes.chars().map(|b| Node::Token{token: Token::Digit, byte: b as u8}).collect();
  Ok((input, Node::Number{children: digits}))
}

fn punctuation(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, punctuation) = alt((period , exclamation , question , comma , colon , semicolon , dash , apostrophe , left_parenthesis , right_parenthesis , left_angle , right_angle , left_brace , right_brace))(input)?;
  Ok((input, Node::Punctuation{children: vec![punctuation]}))
}

fn symbol(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, punctuation) = alt((ampersand , bar , at , slash , hashtag , equal , tilde , plus , asterisk , caret , underscore))(input)?;
  Ok((input, Node::Symbol{children: vec![punctuation]}))
}

fn single_text(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, word) = alt((word , space , number , punctuation , symbol))(input)?;
  Ok((input, Node::Text{children: vec![word]}))
}

fn text(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, word) = many1(alt((word , space , number , punctuation , symbol)))(input)?;
  Ok((input, Node::Text{children: word}))
}

fn paragraph_rest(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, word) = many1(alt((word , space , number , punctuation , symbol , quote)))(input)?;
  Ok((input, Node::Text{children: word}))
}

fn paragraph_starter(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, word) = many1(alt((word, number, quote, left_angle, right_angle, left_bracket, right_bracket, period, exclamation , question , comma , colon , semicolon , left_parenthesis, right_parenthesis)))(input)?;
  Ok((input, Node::Text{children: word}))
}

fn identifier(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, (word, mut rest)) = tuple((word, many0(alt((word, number, dash, slash)))))(input)?;
  let mut id = vec![word];
  id.append(&mut rest);
  Ok((input, Node::Identifier{children: id}))
}

fn carriage_newline(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, _) = tag("\r\n")(input)?;
  Ok((input, Node::Null))
}

fn newline(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, _) = alt((new_line_char, carriage_newline))(input)?;
  Ok((input, Node::Null))
}

fn whitespace(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, _) = many0(space)(input)?;
  let (input, _) = newline(input)?;
  Ok((input, Node::Null))
}

fn floating_point(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input,_) = period(input)?;
  let (input, bytes) = digit1(input)?;
  let digits = bytes.chars().map(|b| Node::Token{token: Token::Digit, byte: b as u8}).collect();
  Ok((input, Node::FloatingPoint{children: digits}))
}

fn quantity(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, number) = number(input)?;
  let (input, float) = opt(floating_point)(input)?;
  let (input, unit) = opt(identifier)(input)?;
  let mut quantity = vec![number];
  match float {
    Some(fp) => quantity.push(fp),
    _ => (),
  };
  match unit {
    Some(unit) => quantity.push(unit),
    _ => (),
  };
  Ok((input, Node::Quantity{children: quantity}))
}

fn constant(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, constant) = alt((string, quantity))(input)?;
  Ok((input, Node::Constant{children: vec![constant]}))
}

fn empty(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, _) = many1(underscore)(input)?;
  Ok((input, Node::Empty))
}

// ## Blocks

// ### Data

fn select_all(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, _) = colon(input)?;
  Ok((input, Node::SelectAll))
}

fn subscript(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, subscript) = alt((select_all, expression))(input)?;
  let (input, _) = tuple((space0, opt(comma), space0))(input)?;
  Ok((input, Node::Subscript{children: vec![subscript]}))
}

fn subscript_index(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, _) = left_brace(input)?;
  let (input, subscripts) = many1(subscript)(input)?;
  let (input, _) = right_brace(input)?;
  Ok((input, Node::SubscriptIndex{children: subscripts}))
}

fn dot_index(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, _) = period(input)?;
  let (input, identifier) = identifier(input)?;
  let mut index = vec![identifier];
  Ok((input, Node::DotIndex{children: index}))
}

fn index(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, index) = alt((dot_index, subscript_index))(input)?;
  Ok((input, Node::Index{children: vec![index]}))
}

fn data(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, source) = alt((table, identifier))(input)?;
  let (input, mut indices) = many0(index)(input)?;
  let mut data = vec![source];
  data.append(&mut indices);
  Ok((input, Node::Data{children: data}))
}

// ### Tables

fn table(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, _) = hashtag(input)?;
  let (input, table_identifier) = identifier(input)?;
  Ok((input, Node::Table{children: vec![table_identifier]}))
}

fn binding(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, binding_id) = identifier(input)?;
  let (input, _) = tuple((colon, space0))(input)?;
  let (input, bound) = alt((empty, expression, identifier, constant))(input)?;
  let (input, _) = tuple((space0, opt(comma), space0))(input)?;
  Ok((input, Node::Binding{children: vec![binding_id, bound]}))
}

fn function_binding(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, binding_id) = identifier(input)?;
  let (input, _) = tuple((colon, space0))(input)?;
  let (input, bound) = alt((empty, expression, identifier, constant))(input)?;
  let (input, _) = tuple((space0, opt(comma), space0))(input)?;
  Ok((input, Node::FunctionBinding{children: vec![binding_id, bound]}))
}


fn table_column(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, _) = many0(alt((space, tab)))(input)?;
  let (input, item) = alt((empty, data, expression, quantity))(input)?;
  let (input, _) = tuple((opt(comma), opt(alt((space, tab)))))(input)?;
  Ok((input, Node::Column{children: vec![item]}))
}

fn table_row(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, _) = many0(alt((space, tab)))(input)?;
  let (input, columns) = many1(table_column)(input)?;
  let (input, _) = tuple((opt(semicolon), opt(newline)))(input)?;
  Ok((input, Node::TableRow{children: columns}))
}

fn attribute(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, identifier) = identifier(input)?;
  let (input, _) = tuple((space0, opt(comma), space0))(input)?;
  Ok((input, Node::Attribute{children: vec![identifier]}))
}

fn table_header(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, _) = bar(input)?;
  let (input, attributes) = many1(attribute)(input)?;
  let (input, _) = tuple((bar, space0, opt(newline)))(input)?;
  Ok((input, Node::TableHeader{children: attributes}))
}

fn anonymous_table(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, _) = left_bracket(input)?;
  let (input, _) = space0(input)?;
  let (input, table_header) = opt(table_header)(input)?;
  let (input, mut table_rows) = many0(table_row)(input)?;
  let (input, _) = right_bracket(input)?;
  let mut table = vec![];
  match table_header {
    Some(table_header) => table.push(table_header),
    _ => (),
  };
  table.append(&mut table_rows);
  Ok((input, Node::AnonymousTable{children: table}))
}

fn inline_table(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, _) = left_bracket(input)?;
  let (input, bindings) = many1(binding)(input)?;
  let (input, _) = right_bracket(input)?;
  Ok((input, Node::InlineTable{children: bindings}))
}

// ### Statements

fn comment_sigil(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, _) = tag("//")(input)?;
  Ok((input, Node::Null))
}

fn comment(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, _) = comment_sigil(input)?;
  let (input, comment) = text(input)?;
  Ok((input, Node::Comment{children: vec![comment]}))
}

fn add_row_operator(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, _) = tag("+=")(input)?;
  Ok((input, Node::Null))
}

fn add_row(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, table_id) = table(input)?;
  let (input, _) = tuple((space, add_row_operator, space))(input)?;
  let (input, table) = alt((inline_table, anonymous_table))(input)?;
  Ok((input, Node::AddRow{children: vec![table_id, table]}))
}

fn set_operator(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, _) = tag(":=")(input)?;
  Ok((input, Node::Null))
}

fn set_data(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, table) = data(input)?;
  let (input, _) = tuple((space, set_operator, space))(input)?;
  let (input, expression) = expression(input)?;
  Ok((input, Node::SetData{children: vec![table, expression]}))
}

fn split_data(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, table) = identifier(input)?;
  let (input, _) = tuple((space, split_operator, space))(input)?;
  let (input, expression) = expression(input)?;
  Ok((input, Node::SplitData{children: vec![table, expression]}))
}

fn join_data(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, table) = identifier(input)?;
  let (input, _) = tuple((space, join_operator, space))(input)?;
  let (input, expression) = expression(input)?;
  Ok((input, Node::JoinData{children: vec![table, expression]}))
}

fn variable_define(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, variable) = identifier(input)?;
  let (input, _) = tuple((space, equal, space))(input)?;
  let (input, expression) = expression(input)?;
  Ok((input, Node::VariableDefine{children: vec![variable, expression]}))
}

fn table_define(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, table) = table(input)?;
  let (input, _) = tuple((space, equal, space))(input)?;
  let (input, expression) = expression(input)?;
  Ok((input, Node::TableDefine{children: vec![table, expression]}))
}

fn split_operator(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, _) = tag(">-")(input)?;
  Ok((input, Node::Null))
}

fn join_operator(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, _) = tag("-<")(input)?;
  Ok((input, Node::Null))
}

fn whenever_operator(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, _) = tag("~")(input)?;
  Ok((input, Node::Null))
}

fn until_operator(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, _) = tag("~|")(input)?;
  Ok((input, Node::Null))
}

fn as_soon_as_operator(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, _) = tag("|~")(input)?;
  Ok((input, Node::Null))
}

fn whenever_data(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, _) = whenever_operator(input)?;
  let (input, _) = space(input)?;
  let (input, watch) = alt((variable_define, expression, data))(input)?;
  Ok((input, Node::Whenever{children: vec![watch]}))
}

fn as_soon_as_data(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, _) = as_soon_as_operator(input)?;
  let (input, _) = space(input)?;
  let (input, watch) = alt((variable_define, expression, data))(input)?;
  Ok((input, Node::AsSoonAs{children: vec![watch]}))
}

fn until_data(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, _) = until_operator(input)?;
  let (input, _) = space(input)?;
  let (input, watch) = alt((variable_define, expression, data))(input)?;
  Ok((input, Node::Until{children: vec![watch]}))
}

fn statement(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, statement) = alt((table_define, variable_define, split_data, join_data, whenever_data, as_soon_as_data, until_data, set_data, add_row, comment))(input)?;
  Ok((input, Node::Statement{children: vec![statement]}))
}

// ### Expressions

// #### Math Expressions

fn parenthetical_expression(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, _) = left_parenthesis(input)?;
  let (input, l0) = l0(input)?;
  let (input, _) = right_parenthesis(input)?;
  Ok((input, l0))
}

fn negation(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, _) = dash(input)?;
  let (input, negated) = alt((data, constant))(input)?;
  Ok((input, Node::Negation { children: vec![negated] }))
}

fn function(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, identifier) = identifier(input)?;
  let (input, _) = left_parenthesis(input)?;
  let (input, mut bindings) = many1(function_binding)(input)?;
  let (input, _) = right_parenthesis(input)?;
  let mut function = vec![identifier];
  function.append(&mut bindings);
  Ok((input, Node::Function { children: function }))
}

fn matrix_multiply(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, _) = tag("**")(input)?;
  Ok((input, Node::Null))
}

fn add(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, _) = tag("+")(input)?;
  Ok((input, Node::Add))
}

fn subtract(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, _) = tag("-")(input)?;
  Ok((input, Node::Subtract))
}

fn multiply(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, _) = tag("*")(input)?;
  Ok((input, Node::Multiply))
}

fn divide(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, _) = tag("/")(input)?;
  Ok((input, Node::Divide))
}

fn exponent(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, _) = tag("^")(input)?;
  Ok((input, Node::Exponent))
}

fn range_op(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, _) = tag(":")(input)?;
  Ok((input, Node::Range))
}

fn l0(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, l1) = l1(input)?;
  let (input, mut infix) = many0(l0_infix)(input)?;
  let mut math = vec![l1];
  math.append(&mut infix);
  Ok((input, Node::L0 { children: math }))
}

fn l0_infix(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, _) = space0(input)?;
  let (input, op) = range_op(input)?;
  let (input, _) = space0(input)?;
  let (input, l1) = l1(input)?;
  Ok((input, Node::L0Infix { children: vec![op, l1] }))
}

fn l1(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, l2) = l2(input)?;
  let (input, mut infix) = many0(l1_infix)(input)?;
  let mut math = vec![l2];
  math.append(&mut infix);
  Ok((input, Node::L1 { children: math }))
}

fn l1_infix(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, _) = space(input)?;
  let (input, op) = alt((add, subtract))(input)?;
  let (input, _) = space(input)?;
  let (input, l2) = l2(input)?;
  Ok((input, Node::L1Infix { children: vec![op, l2] }))
}

fn l2(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, l3) = l3(input)?;
  let (input, mut infix) = many0(l2_infix)(input)?;
  let mut math = vec![l3];
  math.append(&mut infix);
  Ok((input, Node::L2 { children: math }))
}

fn l2_infix(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, _) = space(input)?;
  let (input, op) = alt((multiply, divide, matrix_multiply))(input)?;
  let (input, _) = space(input)?;
  let (input, l3) = l3(input)?;
  Ok((input, Node::L2Infix { children: vec![op, l3] }))
}

fn l3(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, l4) = l4(input)?;
  let (input, mut infix) = many0(l3_infix)(input)?;
  let mut math = vec![l4];
  math.append(&mut infix);
  Ok((input, Node::L3 { children: math }))
}

fn l3_infix(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, _) = space(input)?;
  let (input, op) = exponent(input)?;
  let (input, _) = space(input)?;
  let (input, l4) = l4(input)?;
  Ok((input, Node::L3Infix { children: vec![op, l4] }))
}

fn l4(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, l5) = l5(input)?;
  let (input, mut infix) = many0(l4_infix)(input)?;
  let mut math = vec![l5];
  math.append(&mut infix);
  Ok((input, Node::L4 { children: math }))
}

fn l4_infix(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, _) = space(input)?;
  let (input, op) = alt((and, or))(input)?;
  let (input, _) = space(input)?;
  let (input, l5) = l5(input)?;
  Ok((input, Node::L4Infix { children: vec![op, l5] }))
}

fn l5(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, l6) = l6(input)?;
  let (input, mut infix) = many0(l5_infix)(input)?;
  let mut math = vec![l6];
  math.append(&mut infix);
  Ok((input, Node::L5 { children: math }))
}

fn l5_infix(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, _) = space(input)?;
  let (input, op) = alt((not_equal,equal_to, greater_than_equal, greater_than, less_than_equal, less_than))(input)?;
  let (input, _) = space(input)?;
  let (input, l6) = l6(input)?;
  Ok((input, Node::L5Infix { children: vec![op, l6] }))
}

fn l6(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, l6) = alt((anonymous_table, function, data, string, quantity, negation, parenthetical_expression))(input)?;
  Ok((input, Node::L6 { children: vec![l6] }))
}

fn math_expression(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, l0) = l0(input)?;
  Ok((input, Node::MathExpression { children: vec![l0] }))
}

// #### Filter Expressions

fn not_equal(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, _) = tag("!=")(input)?;
  Ok((input, Node::NotEqual))
}

fn equal_to(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, _) = tag("==")(input)?;
  Ok((input, Node::Equal))
}

fn greater_than(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, _) = tag(">")(input)?;
  Ok((input, Node::GreaterThan))
}

fn less_than(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, _) = tag("<")(input)?;
  Ok((input, Node::LessThan))
}

fn greater_than_equal(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, _) = tag(">=")(input)?;
  Ok((input, Node::GreaterThanEqual))
}

fn less_than_equal(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, _) = tag("<=")(input)?;
  Ok((input, Node::LessThanEqual))
}

// State Machine

/*
named!(state_machine<CompleteStr, Node>, do_parse!(
  source: data >> question >> whitespace >> transitions: transitions >> whitespace >>
  (Node::StateMachine { children: vec![source, transitions] })));

named!(transitions<CompleteStr, Node>, do_parse!(
  transitions: many1!(transition) >>
  (Node::Transitions { children:transitions })));

named!(transition<CompleteStr, Node>, do_parse!(
  many1!(space) >> state: alt!(string | constant | empty) >> many1!(space) >> tag!("=>") >> many1!(space) >> next: alt!(identifier | string | constant | empty) >> many0!(space) >> opt!(newline) >>
  (Node::Transition { children: vec![state, next] })));*/

// #### Logic Expressions

fn or(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, _) = tag("|")(input)?;
  Ok((input, Node::Or))
}

fn and(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, _) = tag("&")(input)?;
  Ok((input, Node::And))
}

// #### Other Expressions

fn string_interpolation(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, _) = tag("{{")(input)?;
  let (input, expression) = expression(input)?;
  let (input, _) = tag("}}")(input)?;
  Ok((input, Node::StringInterpolation { children: vec![expression] }))
}

fn string(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, _) = quote(input)?;
  let (input, text) = many0(alt((string_interpolation, text)))(input)?;
  let (input, _) = quote(input)?;
  Ok((input, Node::String { children: text }))
}

fn expression(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, expression) = alt((string, inline_table, math_expression, anonymous_table))(input)?;
  Ok((input, Node::Expression { children: vec![expression] }))
}

// ### Block Basics

fn constraint(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, _) = tuple((space,space))(input)?;
  let (input, statement) = statement(input)?;
  let (input, _) = tuple((space0,opt(newline)))(input)?;
  Ok((input, Node::Constraint { children: vec![statement] }))
}

fn block(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, constraints) = many1(constraint)(input)?; 
  let (input, _) = many0(whitespace)(input)?;
  Ok((input, Node::Block { children: constraints }))
}

// ## Markdown

fn title(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, _) = hashtag(input)?;
  let (input, _) = space1(input)?;
  let (input, text) = text(input)?; 
  let (input, _) = many0(whitespace)(input)?;
  Ok((input, Node::Title { children: vec![text] }))
}

fn subtitle(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, _) = hashtag(input)?;
  let (input, _) = hashtag(input)?;
  let (input, _) = space1(input)?;
  let (input, text) = text(input)?; 
  let (input, _) = many0(whitespace)(input)?;
  Ok((input, Node::Subtitle { children: vec![text] }))
}

fn section_title(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, _) = hashtag(input)?;
  let (input, _) = hashtag(input)?;
  let (input, _) = hashtag(input)?;
  let (input, _) = space1(input)?;
  let (input, text) = text(input)?; 
  let (input, _) = many0(whitespace)(input)?;
  Ok((input, Node::SectionTitle { children: vec![text] }))
}

fn inline_code(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, _) = grave(input)?;
  let (input, text) = text(input)?; 
  let (input, _) = grave(input)?;
  let (input, _) = space0(input)?;
  Ok((input, Node::InlineCode { children: vec![text] }))
}

fn paragraph_text(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, word) = paragraph_starter(input)?;
  let (input, text) = opt(paragraph_rest)(input)?; 
  let mut paragraph = vec![word];
  match text {
    Some(text) => paragraph.push(text),
    _ => (),
  };
  Ok((input, Node::ParagraphText { children: paragraph }))
}

fn paragraph(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, paragraph_elements) = many1(alt((inline_mech_code, inline_code, paragraph_text)))(input)?;
  let (input, _) = opt(newline)(input)?;
  let (input, _) = many0(whitespace)(input)?;
  Ok((input, Node::Paragraph { children: paragraph_elements }))
}

fn unordered_list(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, list_items) = many1(list_item)(input)?;
  let (input, _) = opt(newline)(input)?;
  let (input, _) = many0(whitespace)(input)?;
  Ok((input, Node::UnorderedList { children: list_items }))
}

fn list_item(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, _) = dash(input)?;
  let (input, _) = space1(input)?;
  let (input, list_item) = paragraph(input)?;
  let (input, _) = opt(newline)(input)?;
  Ok((input, Node::ListItem { children: vec![list_item] }))
}

fn formatted_text(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, formatted) = many0(alt((paragraph_rest, carriage_return, new_line_char)))(input)?;
  Ok((input, Node::FormattedText { children: formatted }))
}

fn code_block(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, _) = tuple((grave, grave, grave, newline))(input)?;
  let (input, text) = formatted_text(input)?;
  let (input, _) = tuple((grave, grave, grave, newline, many0(whitespace)))(input)?;
  Ok((input, Node::CodeBlock { children: vec![text] }))
}

// Mechdown

fn inline_mech_code(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, _) = tuple((left_bracket,left_bracket))(input)?;
  let (input, expression) = expression(input)?;
  let (input, _) = tuple((right_bracket,right_bracket,opt(space)))(input)?;
  Ok((input, Node::InlineMechCode{ children: vec![expression] }))
}

fn mech_code_block(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, _) = tuple((grave,grave,grave,tag("mech:")))(input)?;
  let (input, directive) = opt(text)(input)?;
  let (input, _) = newline(input)?;
  let (input, mech_block) = block(input)?;
  let (input, _) = tuple((grave,grave,grave))(input)?;
  let (input, _) = newline(input)?;
  let (input, _) = many0(whitespace)(input)?;
  let mut elements = vec![];
  match directive {
    Some(directive) => elements.push(directive),
    _ => (),
  }
  elements.push(mech_block);
  Ok((input, Node::MechCodeBlock{ children: elements }))
}

// ## Start Here

fn section(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, section_title) = opt(subtitle)(input)?;
  let (input, mut section_elements) = many1(alt((block , code_block , mech_code_block , paragraph , unordered_list)))(input)?;
  let (input, _) = many0(whitespace)(input)?;
  let mut section = vec![];
  match section_title {
    Some(subtitle) => section.push(subtitle),
    _ => (),
  };
  section.append(&mut section_elements);
  Ok((input, Node::Section{ children: section }))
}

fn body(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, _) = many0(whitespace)(input)?;
  let (input, sections) = many1(section)(input)?;
  Ok((input, Node::Body { children: sections }))
}

fn fragment(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, statement) = statement(input)?;
  Ok((input, Node::Fragment { children:  vec![statement] }))
}

pub fn program(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let mut program = vec![];
  let (input, title) = opt(title)(input)?;
  match title {
    Some(title) => program.push(title),
    None => (),
  };
  let (input, body) = body(input)?;
  program.push(body);
  let (input, _) = opt(whitespace)(input)?;
  Ok((input, Node::Program { children: program }))
}

fn parse_mech(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, program) = alt((program, fragment))(input)?;
  Ok((input, Node::Root { children:  vec![program] }))
}

fn raw_constraint(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, statement) = statement(input)?;
  let (input, _) = space0(input)?;
  let (input, _) = opt(newline)(input)?;
  Ok((input, Node::Constraint { children:  vec![statement] }))
}

pub fn parse_block(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, constraints) = many1(raw_constraint)(input)?;
  let (input, _) = many0(whitespace)(input)?;
  Ok((input, Node::Block { children:  constraints }))
}

pub fn parse_fragment(input: &str) -> IResult<&str, Node, VerboseError<&str>> {
  let (input, statement) = statement(input)?;
  Ok((input, Node::Fragment { children:  vec![statement] }))
}