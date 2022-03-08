// # Parser

// ## Prelude

use crate::lexer::Token;
use mech_core::*;

#[cfg(not(feature = "no-std"))] use core::fmt;
#[cfg(feature = "no-std")] use alloc::fmt;
#[cfg(feature = "no-std")] use alloc::string::String;
#[cfg(feature = "no-std")] use alloc::vec::Vec;
use nom::{
  IResult,
  branch::alt,
  sequence::tuple,
  combinator::{opt,eof},
  multi::{many1, many_till, many0, separated_list1},
};

use unicode_segmentation::*;

// ## Parser Node

#[derive(Clone, PartialEq)]
pub enum Node {
  Root{ children: Vec<Node> },
  Block{ children: Vec<Node> },
  Transformation{ children: Vec<Node> },
  Select { children: Vec<Node> },
  Whenever { children: Vec<Node> },
  Wait { children: Vec<Node> },
  Until { children: Vec<Node> },
  Insert { children: Vec<Node> },
  VariableDefine { children: Vec<Node> },
  TableDefine { children: Vec<Node> },
  TableSelect { children: Vec<Node> },
  AddRow { children: Vec<Node> },
  Column { children: Vec<Node> },
  IdentifierOrConstant { children: Vec<Node> },
  Table { children: Vec<Node> },
  Number { children: Vec<Node> },
  DigitOrComma {children: Vec<Node> },
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
  EmptyTable{ children: Vec<Node> },
  AnonymousMatrix{ children: Vec<Node> },
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
  Emoji{ children: Vec<Node> },
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
  Not{ children: Vec<Node> },
  ParentheticalExpression{ children: Vec<Node> },
  CommentSigil{ children: Vec<Node> },
  Comment{children: Vec<Node>},
  Any{children: Vec<Node>},
  Symbol{children: Vec<Node>},
  StateMachine{children: Vec<Node>},
  StateTransition{children: Vec<Node>},
  Quantity{children: Vec<Node>},
  Value{children: Vec<Node>},
  BooleanLiteral{children: Vec<Node>},
  NumberLiteral{children: Vec<Node>},
  FloatLiteral{chars: Vec<char>},
  DecimalLiteral{chars: Vec<char>},
  HexadecimalLiteral{chars: Vec<char>},
  OctalLiteral{chars: Vec<char>},
  BinaryLiteral{chars: Vec<char>},
  RationalNumber{children: Vec<Node>},
  Token{token: Token, chars: Vec<char>},
  KindAnnotation{children: Vec<Node>},
  ReshapeColumn,
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
  Xor,
  Empty,
  Null,
  True,
  False,
}

impl fmt::Debug for Node {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    print_recurse(self, 1);
    Ok(())
  }
}

pub fn print_recurse(node: &Node, level: usize) {
  spacer(level);
  let children: Option<&Vec<Node>> = match node {
    Node::Root{children} => {print!("Root\n"); Some(children)},
    Node::Block{children} => {print!("Block\n"); Some(children)},
    Node::Transformation{children} => {print!("Transformation\n"); Some(children)},
    Node::Select{children} => {print!("Select\n"); Some(children)},
    Node::Whenever{children} => {print!("Whenever\n"); Some(children)},
    Node::Insert{children} => {print!("Insert\n"); Some(children)},
    Node::MathExpression{children} => {print!("MathExpression\n"); Some(children)},
    Node::SelectExpression{children} => {print!("SelectExpression\n"); Some(children)},
    Node::Comparator{children} => {print!("Comparator\n"); Some(children)},
    Node::FilterExpression{children} => {print!("FilterExpression\n"); Some(children)},
    Node::AnonymousTable{children} => {print!("AnonymousTable\n"); Some(children)},
    Node::EmptyTable{children} => {print!("EmptyTable\n"); Some(children)},
    Node::AnonymousMatrix{children} => {print!("AnonymousMatrix\n"); Some(children)},
    Node::TableRow{children} => {print!("TableRow\n"); Some(children)},
    Node::Table{children} => {print!("Table\n"); Some(children)},
    Node::Number{children} => {print!("Number\n"); Some(children)},
    Node::DigitOrComma{children} => {print!("DigitOrComma\n"); Some(children)},
    Node::Alphanumeric{children} => {print!("Alphanumeric\n"); Some(children)},
    Node::Word{children} => {print!("Word\n"); Some(children)},
    Node::Emoji{children} => {print!("Emoji\n"); Some(children)},
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
    Node::TableSelect{children} => {print!("TableSelect\n"); Some(children)},
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
    Node::Wait{children} => {print!("Wait\n"); Some(children)},
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
    Node::Not{children} => {print!("Not\n"); Some(children)},
    Node::ParentheticalExpression{children} => {print!("ParentheticalExpression\n"); Some(children)},
    Node::ProseOrCode{children} => {print!("ProseOrCode\n"); Some(children)},
    Node::Whitespace{children} => {print!("Whitespace\n"); Some(children)},
    Node::SpaceOrTab{children} => {print!("SpaceOrTab\n"); Some(children)},
    Node::NewLine{children} => {print!("NewLine\n"); Some(children)},
    Node::Token{token, chars} => {print!("Token({:?} ({:?}))\n", token, chars); None},
    Node::CommentSigil{children} => {print!("CommentSigil\n"); Some(children)},
    Node::Comment{children} => {print!("Comment\n"); Some(children)},
    Node::Any{children} => {print!("Any\n"); Some(children)},
    Node::Symbol{children} => {print!("Symbol\n"); Some(children)},
    Node::Quantity{children} => {print!("Quantity\n"); Some(children)},
    Node::NumberLiteral{children} => {print!("NumberLiteral\n"); Some(children)},
    Node::FloatLiteral{chars} => {print!("FloatLiteral({:?})\n", chars); None},
    Node::DecimalLiteral{chars} => {print!("DecimalLiteral({:?})\n", chars); None},
    Node::HexadecimalLiteral{chars} => {print!("HexadecimalLiteral({:?})\n", chars); None},
    Node::OctalLiteral{chars} => {print!("OctalLiteral({:?})\n", chars); None},
    Node::BinaryLiteral{chars} => {print!("BinaryLiteral({:?})\n", chars); None},
    Node::RationalNumber{children} => {print!("RationalNumber\n"); Some(children)},
    Node::StateMachine{children} => {print!("StateMachine\n"); Some(children)},
    Node::StateTransition{children} => {print!("StateTransition\n"); Some(children)},
    Node::Value{children} => {print!("Value\n"); Some(children)},
    Node::KindAnnotation{children} => {print!("KindAnnotation\n"); Some(children)},
    Node::BooleanLiteral{children} => {print!("BooleanLiteral\n"); Some(children)},
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
    Node::Xor => {print!("Xor\n",); None},
    Node::Empty => {print!("Empty\n",); None},
    Node::Null => {print!("Null\n",); None},
    Node::ReshapeColumn => {print!("ReshapeColumn\n",); None},
    Node::False => {print!("True\n",); None},
    Node::True => {print!("False\n",); None},
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



pub fn parse(text: &str) -> Result<Node,MechError> {

  let graphemes = UnicodeSegmentation::graphemes(text, true).collect::<Vec<&str>>();

  let parse_tree = parse_mech(graphemes);
  match parse_tree {
    Ok((rest, tree)) => {
      let unparsed = rest.iter().map(|s| String::from(*s)).collect::<String>();
      if unparsed != "" {
        println!("{:?}", tree);
        println!("{:?}", unparsed);
        Err(MechError{id: 0000, kind: MechErrorKind::None})
      } else { 
        Ok(tree)
      }
    },
    Err(q) => {
      println!("{:?}", q);
      Err(MechError{id: 0000, kind: MechErrorKind::None})
    }
  }
}

pub fn parse_fragment(text: &str) -> Result<Node,MechError> {

  let graphemes = UnicodeSegmentation::graphemes(text, true).collect::<Vec<&str>>();

  let parse_tree = parse_mech_fragment(graphemes);
  match parse_tree {
    Ok((rest, tree)) => {
      let unparsed = rest.iter().map(|s| String::from(*s)).collect::<String>();
      if unparsed != "" {
        Err(MechError{id: 0000, kind: MechErrorKind::None})
      } else { 
        Ok(tree)
      }
    },
    Err(q) => {
      Err(MechError{id: 0000, kind: MechErrorKind::None})
    }
  }
}

pub fn tag(tag: &str) -> impl Fn(Vec<&str>) -> IResult<Vec<&str>, Vec<&str>>  {
  let tag = tag.to_string();
  move |mut input: Vec<&str>| {
    if input.len() == 0 {
      return Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Tag)))
    }
    let tag_graphemes = tag.graphemes(true).collect::<Vec<&str>>();
    let tag_len = tag_graphemes.len();
    if tag_graphemes.iter().zip(input.iter().take(tag_len)).all(|(t,i)| t==i) {
      let rest = input.split_off(tag_len);
      Ok((rest, input))
    } else {
      Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Tag)))
    }
  }
}

pub fn ascii_tag(tag: &str) -> impl Fn(Vec<&str>) -> IResult<Vec<&str>, &str>  {
  let tag = tag.to_string();
  move |mut input: Vec<&str>| {
    if input.len() == 0 {
      return Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Tag)))
    }
    let tag_graphemes = tag.graphemes(true).collect::<Vec<&str>>();
    let tag_len = tag_graphemes.len();
    if tag_graphemes.iter().zip(input.iter().take(tag_len)).all(|(t,i)| t==i) {
      let rest = input.split_off(tag_len);
      Ok((rest, input[0]))
    } else {
      Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Tag)))
    }
  }
}

macro_rules! leaf {
  ($name:ident, $byte:expr, $token:expr) => (
    fn $name(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
      let (input, _) = ascii_tag($byte)(input)?;
      Ok((input, Node::Token{token: $token, chars: $byte.chars().collect::<Vec<char>>()}))
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
leaf!{backslash, "\\", Token::Backslash}
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
fn emoji_grapheme(mut input: Vec<&str>) -> IResult<Vec<&str>, &str> {
  if input.len() >= 1 {
    let rest = input.split_off(1);
    let chars = input[0].chars();
    match chars.peekable().peek() {
      Some(c) => {
        if !c.is_ascii() && !c.is_alphabetic() {
          Ok((rest, input[0]))
        } else {
          Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Tag)))
        }
      }
      None => Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Tag)))
    }
  } else {
    Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Tag)))
  }
}

fn emoji(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, matching) = many1(emoji_grapheme)(input)?;
  let chars: Vec<Node> = matching.iter().map(|b| Node::Token{token: Token::Emoji, chars: b.chars().collect::<Vec<char>>()}).collect();
  Ok((input, Node::Emoji{children: chars}))
}

fn word(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, matching) = many1(alpha)(input)?;
  let chars: Vec<Node> = matching.iter().map(|b| Node::Token{token: Token::Alpha, chars: b.chars().collect::<Vec<char>>()}).collect();
  Ok((input, Node::Word{children: chars}))
}

fn alpha(mut input: Vec<&str>) -> IResult<Vec<&str>, &str> {
  if input.len() >= 1 {
    let rest = input.split_off(1);
    let chars = input[0].chars();
    match chars.peekable().peek() {
      Some(c) => {
        if c.is_alphabetic() {
          Ok((rest, input[0]))
        } else {
          Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Tag)))
        }
      }
      None => Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Tag)))
    }
  } else {
    Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Tag)))
  }
}

fn digit(mut input: Vec<&str>) -> IResult<Vec<&str>, &str> {
  if input.len() >= 1 {
    let rest = input.split_off(1);
    let chars = input[0].chars();
    match chars.peekable().peek() {
      Some(c) => {
        if c.is_numeric() {
          Ok((rest, input[0]))
        } else {
          Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Tag)))
        }
      }
      None => Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Tag)))
    }
  } else {
    Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Tag)))
  }
}

fn digit1(input: Vec<&str>) -> IResult<Vec<&str>, Vec<&str>> {
  let result = many1(digit)(input)?;
  Ok(result)
}

fn digit0(input: Vec<&str>) -> IResult<Vec<&str>, Vec<&str>> {
  let result = many0(digit)(input)?;
  Ok(result)
}

fn bin_digit(input: Vec<&str>) -> IResult<Vec<&str>, &str> {
  let result = alt((ascii_tag("1"),ascii_tag("0")))(input)?;
  Ok(result)
}

fn hex_digit(input: Vec<&str>) -> IResult<Vec<&str>, &str> {
  let result = alt((digit, ascii_tag("a"), ascii_tag("b"), ascii_tag("c"), ascii_tag("d"), ascii_tag("e"), ascii_tag("f"), 
                           ascii_tag("A"), ascii_tag("B"), ascii_tag("C"), ascii_tag("D"), ascii_tag("E"), ascii_tag("F")))(input)?;
  Ok(result)
}

fn oct_digit(input: Vec<&str>) -> IResult<Vec<&str>, &str> {
  let result = alt((ascii_tag("0"),ascii_tag("1"),ascii_tag("2"),ascii_tag("3"),ascii_tag("4"),ascii_tag("5"),ascii_tag("6"),ascii_tag("7")))(input)?;
  Ok(result)
}

fn number(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, matching) = digit1(input)?;
  let chars: Vec<Node> = matching.iter().map(|b| Node::Token{token: Token::Digit, chars: b.chars().collect::<Vec<char>>()}).collect();
  Ok((input, Node::Number{children: chars}))
}

fn punctuation(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, punctuation) = alt((period, exclamation, question, comma, colon, semicolon, dash, apostrophe, left_parenthesis, right_parenthesis, left_angle, right_angle, left_brace, right_brace))(input)?;
  Ok((input, Node::Punctuation{children: vec![punctuation]}))
}

fn symbol(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, symbol) = alt((ampersand, bar, at, slash, backslash, hashtag, equal, tilde, plus, asterisk, caret, underscore))(input)?;
  Ok((input, Node::Symbol{children: vec![symbol]}))
}

fn paragraph_symbol(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, symbol) = alt((ampersand, at, slash, backslash, asterisk, caret, underscore))(input)?;
  Ok((input, Node::Symbol{children: vec![symbol]}))
}

fn text(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, word) = many1(alt((word, space, number, punctuation, symbol, emoji)))(input)?;
  Ok((input, Node::Text{children: word}))
}

fn paragraph_rest(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, word) = many1(alt((word, space, number, punctuation, paragraph_symbol, quote, emoji)))(input)?;
  Ok((input, Node::Text{children: word}))
}

fn paragraph_starter(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, word) = many1(alt((word, number, quote, left_angle, right_angle, left_bracket, right_bracket, period, exclamation, question, comma, colon, semicolon, left_parenthesis, right_parenthesis, emoji)))(input)?;
  Ok((input, Node::Text{children: word}))
}

fn identifier(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = many0(space)(input)?;
  let (input, (word, mut rest)) = tuple((alt((word,emoji)), many0(alt((word, number, dash, slash, emoji)))))(input)?;
  let mut id = vec![word];
  id.append(&mut rest);
  Ok((input, Node::Identifier{children: id}))
}

fn carriage_newline(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = tag("\r\n")(input)?;
  Ok((input, Node::Null))
}

fn boolean_literal(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, boolean) = alt((true_literal, false_literal))(input)?;
  Ok((input, Node::BooleanLiteral{children: vec![boolean]}))
}

fn true_literal(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = ascii_tag("t")(input)?;
  let (input, _) = ascii_tag("r")(input)?;
  let (input, _) = ascii_tag("u")(input)?;
  let (input, _) = ascii_tag("e")(input)?;
  Ok((input, Node::True))
}

fn false_literal(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = ascii_tag("f")(input)?;
  let (input, _) = ascii_tag("a")(input)?;
  let (input, _) = ascii_tag("l")(input)?;
  let (input, _) = ascii_tag("s")(input)?;
  let (input, _) = ascii_tag("e")(input)?;
  Ok((input, Node::False))
}

fn newline(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = alt((new_line_char, carriage_newline))(input)?;
  Ok((input, Node::Null))
}

fn whitespace(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = many0(space)(input)?;
  let (input, _) = newline(input)?;
  Ok((input, Node::Null))
}

fn floating_point(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input,_) = period(input)?;
  let (input, chars) = digit1(input)?;
  Ok((input, Node::Null))
}

fn quantity(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, number) = number(input)?;
  let (input, float) = opt(floating_point)(input)?;
  let (input, unit) = identifier(input)?;
  let mut quantity = vec![number];
  match float {
    Some(fp) => quantity.push(fp),
    _ => (),
  };
  quantity.push(unit);
  Ok((input, Node::Quantity{children: quantity}))
}

fn number_literal(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, number_variant) = alt((hexadecimal_literal, octal_literal, binary_literal, decimal_literal, float_literal))(input)?;
  let (input, kind_id) = opt(kind_annotation)(input)?;
  let mut children = vec![number_variant];
  match kind_id {
    Some(kind_id) => children.push(kind_id),
    _ => (),
  }
  Ok((input, Node::NumberLiteral{children}))
}

fn rational_number(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, numerator) = alt((quantity, number_literal))(input)?;
  let (input, _) = tag("/")(input)?;
  let (input, denominator) = alt((quantity, number_literal))(input)?;
  Ok((input, Node::Null))
}

fn float_literal(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, p1) = opt(ascii_tag("."))(input)?;
  let (input, p2) = digit1(input)?;
  let (input, p3) = opt(ascii_tag("."))(input)?;
  let (input, p4) = digit0(input)?;
  let mut whole: Vec<char> = vec![];
  if let Some(_) = p1 {
    whole.push('.');
  }
  let mut digits = p2.iter().flat_map(|c| c.chars()).collect();
  whole.append(&mut digits);
  if let Some(_) = p3 {
    whole.push('.');
  }
  let mut digits = p4.iter().flat_map(|c| c.chars()).collect();
  whole.append(&mut digits);
  Ok((input, Node::FloatLiteral{chars: whole}))
}

fn decimal_literal(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = ascii_tag("0d")(input)?;
  let (input, chars) = digit1(input)?;
  Ok((input, Node::DecimalLiteral{chars: chars.iter().flat_map(|c| c.chars()).collect()}))
}

fn hexadecimal_literal(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = ascii_tag("0x")(input)?;
  let (input, chars) = many1(hex_digit)(input)?;
  Ok((input, Node::HexadecimalLiteral{chars: chars.iter().flat_map(|c| c.chars()).collect()}))
}

fn octal_literal(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = ascii_tag("0o")(input)?;
  let (input, chars) = many1(oct_digit)(input)?;
  Ok((input, Node::OctalLiteral{chars: chars.iter().flat_map(|c| c.chars()).collect()}))
}

fn binary_literal(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = ascii_tag("0b")(input)?;
  let (input, chars) = many1(bin_digit)(input)?;
  Ok((input, Node::BinaryLiteral{chars: chars.iter().flat_map(|c| c.chars()).collect()}))
}

fn value(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, value) = alt((empty, boolean_literal, number_literal, quantity, number_literal, string))(input)?;
  Ok((input, Node::Value{children: vec![value]}))
}

fn empty(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = many1(underscore)(input)?;
  Ok((input, Node::Empty))
}

// ## Blocks

// ### Data

fn select_all(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = colon(input)?;
  Ok((input, Node::SelectAll))
}

fn subscript(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, subscript) = alt((select_all, expression, tilde))(input)?;
  let (input, _) = tuple((many0(space), opt(comma), many0(space)))(input)?;
  Ok((input, Node::Subscript{children: vec![subscript]}))
}

fn subscript_index(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = left_brace(input)?;
  let (input, subscripts) = many1(subscript)(input)?;
  let (input, _) = right_brace(input)?;
  Ok((input, Node::SubscriptIndex{children: subscripts}))
}

fn single_subscript_index(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = left_brace(input)?;
  let (input, subscript) = subscript(input)?;
  let (input, _) = right_brace(input)?;
  Ok((input, Node::SubscriptIndex{children: vec![subscript]}))
}

fn dot_index(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = period(input)?;
  let (input, identifier) = identifier(input)?;
  let (input, subscript) = opt(single_subscript_index)(input)?;
  let index = match subscript {
    Some(subscript) =>vec![subscript, identifier],
    None => vec![Node::Null, identifier],
  };
  Ok((input, Node::DotIndex{children: index}))
}

fn reshape_column(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = left_brace(input)?;
  let (input, _) = colon(input)?;
  let (input, _) = right_brace(input)?;
  Ok((input, Node::ReshapeColumn))
}

fn index(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, index) = alt((dot_index, reshape_column, subscript_index))(input)?;
  Ok((input, Node::Index{children: vec![index]}))
}

fn data(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, source) = alt((table, identifier))(input)?;
  let (input, mut indices) = many0(index)(input)?;
  let mut data = vec![source];
  data.append(&mut indices);
  Ok((input, Node::Data{children: data}))
}

fn kind_annotation(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = left_angle(input)?;
  let (input, kind_id) = separated_list1(tag(","),alt((identifier,underscore)))(input)?;
  let (input, _) = right_angle(input)?;
  Ok((input, Node::KindAnnotation{children: kind_id}))
}

// ### Tables

fn table(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = hashtag(input)?;
  let (input, table_identifier) = identifier(input)?;
  Ok((input, Node::Table{children: vec![table_identifier]}))
}

fn binding(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let mut children = vec![];
  let (input, _) = many0(alt((space, newline, tab)))(input)?;
  let (input, binding_id) = identifier(input)?;
  let (input, kind) = opt(kind_annotation)(input)?;
  let (input, _) = colon(input)?;
  let (input, _) = many0(alt((space, newline, tab)))(input)?;
  let (input, bound) = alt((empty, expression, identifier, value))(input)?;
  let (input, _) = many0(alt((space, newline, tab)))(input)?;
  let (input, _) = opt(comma)(input)?;
  let (input, _) = many0(alt((space, newline, tab)))(input)?;
  children.push(binding_id);
  children.push(bound);
  if let Some(kind) = kind { children.push(kind); }
  Ok((input, Node::Binding{children}))
}

fn function_binding(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, binding_id) = identifier(input)?;
  let (input, _) = tuple((colon, many0(space)))(input)?;
  let (input, bound) = alt((expression, identifier, value))(input)?;
  let (input, _) = tuple((many0(space), opt(comma), many0(space)))(input)?;
  Ok((input, Node::FunctionBinding{children: vec![binding_id, bound]}))
}


fn table_column(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = many0(alt((space, tab)))(input)?;
  let (input, item) = alt((expression, value, data, ))(input)?;
  let (input, _) = tuple((opt(comma), many0(alt((space, tab)))))(input)?;
  Ok((input, Node::Column{children: vec![item]}))
}

fn table_row(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = many0(alt((space, tab)))(input)?;
  let (input, columns) = many1(table_column)(input)?;
  let (input, _) = tuple((opt(semicolon), opt(newline)))(input)?;
  Ok((input, Node::TableRow{children: columns}))
}

fn attribute(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let mut children = vec![];
  let (input, identifier) = identifier(input)?;
  children.push(identifier);
  let (input, kind) = opt(kind_annotation)(input)?;
  let (input, _) = tuple((many0(space), opt(comma), many0(space)))(input)?;
  if let Some(kind) = kind { children.push(kind); }
  Ok((input, Node::Attribute{children}))
}

fn table_header(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = bar(input)?;
  let (input, attributes) = many1(attribute)(input)?;
  let (input, _) = tuple((bar, many0(space), opt(newline)))(input)?;
  Ok((input, Node::TableHeader{children: attributes}))
}

fn anonymous_table(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = left_bracket(input)?;
  let (input, _) = many0(alt((space, newline, tab)))(input)?;
  let (input, _) = many0(space)(input)?;
  let (input, table_header) = opt(table_header)(input)?;
  let (input, mut table_rows) = many0(table_row)(input)?;
  let (input, _) = many0(alt((space, newline, tab)))(input)?;
  let (input, _) = right_bracket(input)?;
  let mut table = vec![];
  match table_header {
    Some(table_header) => table.push(table_header),
    _ => (),
  };
  table.append(&mut table_rows);
  Ok((input, Node::AnonymousTable{children: table}))
}

fn empty_table(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = left_bracket(input)?;
  let (input, _) = many0(alt((space, newline, tab)))(input)?;
  let (input, _) = many0(space)(input)?;
  let (input, table_header) = opt(table_header)(input)?;
  let (input, _) = many0(alt((space, newline, tab)))(input)?;
  let (input, _) = right_bracket(input)?;
  let mut table = vec![];
  match table_header {
    Some(table_header) => table.push(table_header),
    _ => (),
  };
  Ok((input, Node::EmptyTable{children: table}))
}

fn anonymous_matrix(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = left_angle(input)?;
  let (input, _) = many0(alt((space, newline, tab)))(input)?;
  let (input, _) = many0(space)(input)?;
  let (input, table_header) = opt(table_header)(input)?;
  let (input, mut table_rows) = many0(table_row)(input)?;
  let (input, _) = many0(alt((space, newline, tab)))(input)?;
  let (input, _) = right_angle(input)?;
  let mut table = vec![];
  match table_header {
    Some(table_header) => table.push(table_header),
    _ => (),
  };
  table.append(&mut table_rows);
  Ok((input, Node::AnonymousMatrix{children: table}))
}

fn inline_table(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = left_bracket(input)?;
  let (input, bindings) = many1(binding)(input)?;
  let (input, _) = right_bracket(input)?;
  Ok((input, Node::InlineTable{children: bindings}))
}

// ### Statements

fn comment_sigil(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = tag("--")(input)?;
  Ok((input, Node::Null))
}

fn comment(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = comment_sigil(input)?;
  let (input, comment) = text(input)?;
  Ok((input, Node::Comment{children: vec![comment]}))
}

fn add_row_operator(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = tag("+=")(input)?;
  Ok((input, Node::Null))
}

fn add_row(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, table_id) = table(input)?;
  let (input, _) = tuple((many1(space), add_row_operator, many1(space)))(input)?;
  let (input, table) = alt((expression, inline_table, anonymous_table))(input)?;
  Ok((input, Node::AddRow{children: vec![table_id, table]}))
}

fn set_operator(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = tag(":=")(input)?;
  Ok((input, Node::Null))
}

fn set_data(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, table) = data(input)?;
  let (input, _) = tuple((many1(space), set_operator, many1(space)))(input)?;
  let (input, expression) = expression(input)?;
  Ok((input, Node::SetData{children: vec![table, expression]}))
}

fn split_data(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, table) = alt((identifier, table))(input)?;
  let (input, _) = tuple((many1(space), split_operator, many1(space)))(input)?;
  let (input, expression) = expression(input)?;
  Ok((input, Node::SplitData{children: vec![table, expression]}))
}

fn join_data(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, table) = identifier(input)?;
  let (input, _) = tuple((many1(space), join_operator, many1(space)))(input)?;
  let (input, expression) = expression(input)?;
  Ok((input, Node::JoinData{children: vec![table, expression]}))
}

fn variable_define(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, variable) = identifier(input)?;
  let (input, _) = tuple((many1(space), equal, many1(space)))(input)?;
  let (input, expression) = expression(input)?;
  Ok((input, Node::VariableDefine{children: vec![variable, expression]}))
}

fn table_define(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let mut children = vec![];
  let (input, table) = table(input)?;
  children.push(table);
  let (input, kind_id) = opt(kind_annotation)(input)?;
  if let Some(kind_id) = kind_id { children.push(kind_id); }
  let (input, _) = tuple((many1(space), equal, many1(space)))(input)?;
  let (input, expression) = expression(input)?;
  children.push(expression);
  Ok((input, Node::TableDefine{children}))
}

fn table_select(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, expression) = expression(input)?;
  Ok((input, Node::TableSelect{children: vec![expression]}))
}

fn split_operator(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = tag(">-")(input)?;
  Ok((input, Node::Null))
}

fn join_operator(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = tag("-<")(input)?;
  Ok((input, Node::Null))
}

fn whenever_operator(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = tag("~")(input)?;
  Ok((input, Node::Null))
}

fn until_operator(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = tag("~|")(input)?;
  Ok((input, Node::Null))
}

fn wait_operator(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = tag("|~")(input)?;
  Ok((input, Node::Null))
}

fn whenever_data(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = whenever_operator(input)?;
  let (input, _) = space(input)?;
  let (input, watch) = alt((variable_define, expression, data))(input)?;
  Ok((input, Node::Whenever{children: vec![watch]}))
}

fn wait_data(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = wait_operator(input)?;
  let (input, _) = space(input)?;
  let (input, watch) = alt((variable_define, expression, data))(input)?;
  Ok((input, Node::Wait{children: vec![watch]}))
}

fn until_data(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = until_operator(input)?;
  let (input, _) = space(input)?;
  let (input, watch) = alt((variable_define, expression, data))(input)?;
  Ok((input, Node::Until{children: vec![watch]}))
}

fn statement(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, statement) = alt((table_define, variable_define, split_data, join_data, whenever_data, wait_data, until_data, set_data, add_row, comment))(input)?;
  Ok((input, Node::Statement{children: vec![statement]}))
}

// ### Expressions

// #### Math Expressions

fn parenthetical_expression(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = left_parenthesis(input)?;
  let (input, l0) = l0(input)?;
  let (input, _) = right_parenthesis(input)?;
  Ok((input, l0))
}

fn negation(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = dash(input)?;
  let (input, negated) = alt((data, value))(input)?;
  Ok((input, Node::Negation { children: vec![negated] }))
}

fn function(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, identifier) = identifier(input)?;
  let (input, _) = left_parenthesis(input)?;
  let (input, mut bindings) = many1(function_binding)(input)?;
  let (input, _) = right_parenthesis(input)?;
  let mut function = vec![identifier];
  function.append(&mut bindings);
  Ok((input, Node::Function { children: function }))
}

fn matrix_multiply(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = tag("**")(input)?;
  Ok((input, Node::Null))
}

fn add(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = tag("+")(input)?;
  Ok((input, Node::Add))
}

fn subtract(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = tag("-")(input)?;
  Ok((input, Node::Subtract))
}

fn multiply(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = tag("*")(input)?;
  Ok((input, Node::Multiply))
}

fn divide(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = tag("/")(input)?;
  Ok((input, Node::Divide))
}

fn exponent(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = tag("^")(input)?;
  Ok((input, Node::Exponent))
}

fn range_op(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = tag(":")(input)?;
  Ok((input, Node::Range))
}

fn l0(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, l1) = l1(input)?;
  let (input, mut infix) = many0(l0_infix)(input)?;
  let mut math = vec![l1];
  math.append(&mut infix);
  Ok((input, Node::L0 { children: math }))
}

fn l0_infix(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = many0(space)(input)?;
  let (input, op) = range_op(input)?;
  let (input, _) = many0(space)(input)?;
  let (input, l1) = l1(input)?;
  Ok((input, Node::L0Infix { children: vec![op, l1] }))
}

fn l1(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, l2) = l2(input)?;
  let (input, mut infix) = many0(l1_infix)(input)?;
  let mut math = vec![l2];
  math.append(&mut infix);
  Ok((input, Node::L1 { children: math }))
}

fn l1_infix(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = space(input)?;
  let (input, op) = alt((add, subtract))(input)?;
  let (input, _) = space(input)?;
  let (input, l2) = l2(input)?;
  Ok((input, Node::L1Infix { children: vec![op, l2] }))
}

fn l2(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, l3) = l3(input)?;
  let (input, mut infix) = many0(l2_infix)(input)?;
  let mut math = vec![l3];
  math.append(&mut infix);
  Ok((input, Node::L2 { children: math }))
}

fn l2_infix(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = space(input)?;
  let (input, op) = alt((multiply, divide, matrix_multiply))(input)?;
  let (input, _) = space(input)?;
  let (input, l3) = l3(input)?;
  Ok((input, Node::L2Infix { children: vec![op, l3] }))
}

fn l3(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, l4) = l4(input)?;
  let (input, mut infix) = many0(l3_infix)(input)?;
  let mut math = vec![l4];
  math.append(&mut infix);
  Ok((input, Node::L3 { children: math }))
}

fn l3_infix(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = space(input)?;
  let (input, op) = exponent(input)?;
  let (input, _) = space(input)?;
  let (input, l4) = l4(input)?;
  Ok((input, Node::L3Infix { children: vec![op, l4] }))
}

fn l4(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, l5) = l5(input)?;
  let (input, mut infix) = many0(l4_infix)(input)?;
  let mut math = vec![l5];
  math.append(&mut infix);
  Ok((input, Node::L4 { children: math }))
}

fn l4_infix(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = space(input)?;
  let (input, op) = alt((and, or, xor))(input)?;
  let (input, _) = space(input)?;
  let (input, l5) = l5(input)?;
  Ok((input, Node::L4Infix { children: vec![op, l5] }))
}

fn l5(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, l6) = l6(input)?;
  let (input, mut infix) = many0(l5_infix)(input)?;
  let mut math = vec![l6];
  math.append(&mut infix);
  Ok((input, Node::L5 { children: math }))
}

fn l5_infix(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = space(input)?;
  let (input, op) = alt((not_equal,equal_to, greater_than_equal, greater_than, less_than_equal, less_than))(input)?;
  let (input, _) = space(input)?;
  let (input, l6) = l6(input)?;
  Ok((input, Node::L5Infix { children: vec![op, l6] }))
}

fn l6(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, l6) = alt((empty_table, anonymous_table, function, value, not, data, negation, parenthetical_expression))(input)?;
  Ok((input, Node::L6 { children: vec![l6] }))
}

fn math_expression(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, l0) = l0(input)?;
  Ok((input, Node::MathExpression { children: vec![l0] }))
}

// #### Filter Expressions

fn not_equal(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = tag("!=")(input)?;
  Ok((input, Node::NotEqual))
}

fn equal_to(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = tag("==")(input)?;
  Ok((input, Node::Equal))
}

fn greater_than(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = tag(">")(input)?;
  Ok((input, Node::GreaterThan))
}

fn less_than(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = tag("<")(input)?;
  Ok((input, Node::LessThan))
}

fn greater_than_equal(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = tag(">=")(input)?;
  Ok((input, Node::GreaterThanEqual))
}

fn less_than_equal(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = tag("<=")(input)?;
  Ok((input, Node::LessThanEqual))
}

// State Machine

/*
named!(state_machine<CompleteStr, Node>, do_parse!(
  source: data >> question >> whitespace >> transitions: transitions >> whitespace >>
  (Node::StateMachine { children: vec![source, transitions] })));

fn next_state_operator(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = tag("->")(input)?;
  Ok((input, Node::Null))
}

  #timer? x -> x + 1





fn state_transition(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = many1(space)(input)?;


  many1!(space) >> state: alt!(identifier, string | constant | empty) >> many1!(space) >>  >> many1!(space) >> next: alt!(identifier | string | constant | empty) >> many0!(space) >> opt!(newline) >>
  (Node::StateTransition { children: vec![state, next] })));
}*/

// #### Logic Expressions

fn or(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = tag("|")(input)?;
  Ok((input, Node::Or))
}

fn and(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = tag("&")(input)?;
  Ok((input, Node::And))
}

fn not(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = alt((tag("!"), tag("¬")))(input)?;
  let (input, negated) = alt((data, true_literal, false_literal))(input)?;
  Ok((input, Node::Not { children: vec![negated] }))
}

fn xor(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = alt((tag("xor"), tag("⊕"), tag("⊻")))(input)?;
  Ok((input, Node::Xor))
}

// #### Other Expressions

fn string_interpolation(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = tag("{{")(input)?;
  let (input, expression) = expression(input)?;
  let (input, _) = tag("}}")(input)?;
  Ok((input, Node::StringInterpolation { children: vec![expression] }))
}

fn string(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = quote(input)?;
  let (input, text) = many0(alt((string_interpolation, text)))(input)?;
  let (input, _) = quote(input)?;
  Ok((input, Node::String { children: text }))
}

fn expression(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, expression) = alt((string, inline_table, math_expression, empty_table, anonymous_table))(input)?;
  Ok((input, Node::Expression { children: vec![expression] }))
}

// ### Block Basics

fn transformation(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, statement) = statement(input)?;
  let (input, _) = tuple((many0(space),opt(newline)))(input)?;
  Ok((input, Node::Transformation { children: vec![statement] }))
}

fn block(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, transformations) = many1(tuple((tuple((space,space)),transformation)))(input)?;
  let (input, _) = many0(whitespace)(input)?;
  let tfms: Vec<Node> = transformations.iter().map(|(_,tfm)| tfm).cloned().collect();
  Ok((input, Node::Block { children: tfms }))
}

// ## Markdown

fn title(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = hashtag(input)?;
  let (input, _) = many1(space)(input)?;
  let (input, text) = text(input)?;
  let (input, _) = many0(whitespace)(input)?;
  Ok((input, Node::Title { children: vec![text] }))
}

fn subtitle(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = many1(hashtag)(input)?;
  let (input, _) = many1(space)(input)?;
  let (input, text) = text(input)?;
  let (input, _) = many0(whitespace)(input)?;
  Ok((input, Node::Subtitle { children: vec![text] }))
}

fn section_title(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = hashtag(input)?;
  let (input, _) = hashtag(input)?;
  let (input, _) = hashtag(input)?;
  let (input, _) = many1(space)(input)?;
  let (input, text) = text(input)?;
  let (input, _) = many0(whitespace)(input)?;
  Ok((input, Node::SectionTitle { children: vec![text] }))
}

fn inline_code(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = grave(input)?;
  let (input, text) = text(input)?;
  let (input, _) = grave(input)?;
  let (input, _) = many0(space)(input)?;
  Ok((input, Node::InlineCode { children: vec![text] }))
}

fn paragraph_text(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, word) = paragraph_starter(input)?;
  let (input, text) = opt(paragraph_rest)(input)?;
  let mut paragraph = vec![word];
  match text {
    Some(text) => paragraph.push(text),
    _ => (),
  };
  Ok((input, Node::ParagraphText { children: paragraph }))
}

fn paragraph(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, (paragraph_elements,_)) = many_till(
    alt((inline_mech_code, inline_code, paragraph_text)),
    newline
  )(input)?;
  let (input, _) = many0(whitespace)(input)?;
  Ok((input, Node::Paragraph { children: paragraph_elements }))
}

fn unordered_list(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, list_items) = many1(list_item)(input)?;
  let (input, _) = opt(newline)(input)?;
  let (input, _) = many0(whitespace)(input)?;
  Ok((input, Node::UnorderedList { children: list_items }))
}

fn list_item(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = dash(input)?;
  let (input, _) = many1(space)(input)?;
  let (input, list_item) = paragraph(input)?;
  let (input, _) = opt(newline)(input)?;
  Ok((input, Node::ListItem { children: vec![list_item] }))
}

fn formatted_text(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, formatted) = many0(alt((paragraph_rest, carriage_return, new_line_char)))(input)?;
  Ok((input, Node::FormattedText { children: formatted }))
}

fn code_block(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = tuple((grave, grave, grave, newline))(input)?;
  let (input, text) = formatted_text(input)?;
  let (input, _) = tuple((grave, grave, grave, newline, many0(whitespace)))(input)?;
  Ok((input, Node::CodeBlock { children: vec![text] }))
}

// Mechdown

fn inline_mech_code(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = tuple((left_bracket,left_bracket))(input)?;
  let (input, expression) = expression(input)?;
  let (input, _) = tuple((right_bracket,right_bracket,opt(space)))(input)?;
  Ok((input, Node::InlineMechCode{ children: vec![expression] }))
}

fn mech_code_block(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
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

fn section(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, section_title) = opt(subtitle)(input)?;
  let (input, mut section_elements) = many1(
    tuple((
      alt((block, code_block, mech_code_block, paragraph, statement, unordered_list)),
      opt(whitespace),
    ))
  )(input)?;
  let mut section = vec![];
  match section_title {
    Some(subtitle) => section.push(subtitle),
    _ => (),
  };
  section.append(&mut section_elements.iter().map(|(x,_)|x).cloned().collect());
  Ok((input, Node::Section{ children: section }))
}

fn body(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, _) = many0(whitespace)(input)?;
  let (input, sections) = many1(section)(input)?;
  Ok((input, Node::Body { children: sections }))
}

fn fragment(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, statement) = statement(input)?;
  Ok((input, Node::Fragment { children:  vec![statement] }))
}

pub fn program(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let mut program = vec![];
  let (input, _) = opt(whitespace)(input)?;
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

fn parse_mech(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, mech) = alt((program,statement))(input)?;
  Ok((input, Node::Root { children: vec![mech] }))
}

fn raw_transformation(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, statement) = statement(input)?;
  let (input, _) = many0(space)(input)?;
  let (input, _) = opt(newline)(input)?;
  Ok((input, Node::Transformation { children:  vec![statement] }))
}

pub fn parse_block(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, transformations) = many1(raw_transformation)(input)?;
  let (input, _) = many0(whitespace)(input)?;
  Ok((input, Node::Block { children:  transformations }))
}

pub fn parse_mech_fragment(input: Vec<&str>) -> IResult<Vec<&str>, Node> {
  let (input, statement) = statement(input)?;
  Ok((input, Node::Root { children:  vec![statement] }))
}
