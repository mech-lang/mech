// # Parser

// ## Temp
macro_rules! error_token {
    () => (Node::Token {token: Token::Space, chars: vec![' ']})
}

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
  Err,
};

use unicode_segmentation::UnicodeSegmentation;

// ## Parser nodes

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
  Swizzle{ children: Vec<Node> },
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
  UpdateData{ children: Vec<Node> },
  SetOperator{ children: Vec<Node> },
  SplitData{ children: Vec<Node> },
  FlattenData{ children: Vec<Node> },
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
  AddUpdate,
  SubtractUpdate,
  MultiplyUpdate,
  DivideUpdate,
  ExponentUpdate,
  Empty,
  Null,
  True,
  False,
}

// ## Debug

impl fmt::Debug for Node {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    print_recurse(self, 1);
    Ok(())
  }
}

fn print_recurse(node: &Node, level: usize) {
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
    Node::Swizzle{children} => {print!("Swizzle\n"); Some(children)},
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
    Node::UpdateData{children} => {print!("UpdateData\n"); Some(children)},
    Node::SplitData{children} => {print!("SplitData\n"); Some(children)},
    Node::FlattenData{children} => {print!("FlattenData\n"); Some(children)},
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
    Node::AddUpdate => {print!("AddUpdate\n",); None},
    Node::SubtractUpdate => {print!("SubtractUpdate\n",); None},
    Node::MultiplyUpdate => {print!("MultiplyUpdate\n",); None},
    Node::DivideUpdate => {print!("DivideUpdate\n",); None},
    Node::ExponentUpdate => {print!("ExponentUpdate\n",); None},
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

fn spacer(width: usize) {
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

// ## Parser utilities

type ParseStringRange = (usize, usize);   // [a, b)

type ParseResult<'a, O> = IResult<ParseString<'a>, O, ParseError<'a>>;

#[derive(Clone)]
struct ParseString<'a> {
  graphemes: &'a Vec<&'a str>,
  error_log: Vec<(ParseStringRange, ParseErrorDetail)>,
  cursor: usize,
}

impl<'a> ParseString<'a> {
  fn new(graphemes: &'a Vec<&'a str>) -> Self {
    ParseString {
      graphemes,
      error_log: vec![],
      cursor: 0,
    }
  }

  fn consume_tag(&mut self, tag: &'static str) -> Option<String> {
    let gs = tag.graphemes(true).collect::<Vec<&str>>();
    let gs_len = gs.len();
    if self.len() < gs_len {
      return None;
    }
    for i in 0..gs_len {
      if self.graphemes[self.cursor + i] != gs[i] {
        return None;
      }
    }
    self.cursor += gs_len;
    return Some(tag.to_string());
  }

  fn consume_emoji(&mut self) -> Option<String> {
    if self.len() == 0 {
      return None;
    }
    let g = self.graphemes[self.cursor];
    if let Some(c) = g.chars().next() {
      if !c.is_ascii() && !c.is_alphabetic() {
        self.cursor += 1;
        return Some(g.to_string());
      }
    }
    None
  }

  fn consume_alpha(&mut self) -> Option<String> {
    if self.len() == 0 {
      return None;
    }
    let g = self.graphemes[self.cursor];
    if let Some(c) = g.chars().next() {
      if c.is_alphabetic() {
        self.cursor += 1;
        return Some(g.to_string());
      }
    }
    None
  }

  fn consume_digit(&mut self) -> Option<String> {
    if self.len() == 0 {
      return None;
    }
    let g = self.graphemes[self.cursor];
    if let Some(c) = g.chars().next() {
      if c.is_numeric() {
        self.cursor += 1;
        return Some(g.to_string());
      }
    }
    None
  }

  fn had_error(&self) -> bool {
    self.error_log.len() != 0
  }

  fn len(&self) -> usize {
    self.graphemes.len() - self.cursor
  }
}

impl<'a> nom::InputLength for ParseString<'a> {
  fn input_len(&self) -> usize {
    self.len()
  }
}

#[derive(Clone)]
struct ParseErrorDetail {
  message: &'static str,
  annotation_rngs: Vec<ParseStringRange>,
}

struct ParseError<'a> {
  cause_range: ParseStringRange,
  remaining_input: ParseString<'a>,
  error_detail: ParseErrorDetail,
}

impl<'a> ParseError<'a> {
  fn new(input: ParseString<'a>, msg: &'static str) -> Self {
    ParseError {
      cause_range: (input.cursor, input.cursor + 1),
      remaining_input: input,
      error_detail: ParseErrorDetail {
        message: msg,
        annotation_rngs: vec![],
      }
    }
  }

  fn log(&mut self) {
    self.remaining_input.error_log.push((self.cause_range, self.error_detail.clone()));
  }
}

impl<'a> nom::error::ParseError<ParseString<'a>> for ParseError<'a> {
  fn from_error_kind(input: ParseString<'a>,
                     _kind: nom::error::ErrorKind) -> Self {
    ParseError::new(input, "Unexpected error")
  }

  fn append(_input: ParseString<'a>,
            _kind: nom::error::ErrorKind,
            other: Self) -> Self {
    other
  }

  fn or(self, other: Self) -> Self {
    // Choose the branch with larger depth,
    // while prioritizing the other one when depths are equal
    let (self_index, _) = self.cause_range;
    let (other_index, _) = other.cause_range;
    if self_index > other_index {
      self
    } else {
      other
    }
  }
}

// ## Parser combinators

fn range<'a, F, O>(mut parser: F) ->
  impl FnMut(ParseString<'a>) -> ParseResult<(O, ParseStringRange)>
where
  F: FnMut(ParseString<'a>) -> ParseResult<O>
{
  move |input: ParseString| {
    let a = input.cursor;
    match parser(input) {
      Ok((remaining, o)) => {
        let rng = (a, remaining.cursor);
        Ok((remaining, (o, rng)))
      },
      Err(e) => Err(e),
    }
  }
}

macro_rules! label {
  // didn't use the `*` repeater because it still tries to match the ','
  // when there's no tail. label!(p, msg) would have to be written as label!(p, msg,).
  // Not sure how to resolve this problem without having a new macro rule.

  ($parser:expr, $msg:expr) => {
    (label_without_recovery($parser, ParseErrorDetail {
      message: $msg, annotation_rngs: vec![]
    }))
  };

  ($parser:expr, $msg:expr, $($rngs:expr),+) => {
    (label_without_recovery($parser, ParseErrorDetail {
      message: $msg, annotation_rngs: vec![$($rngs),+]
    }))
  };
}

macro_rules! labelr {
  ($parser:expr, $recovery_fn:expr, $msg:expr) => {
    (label_with_recovery($parser, $recovery_fn, ParseErrorDetail {
      message: $msg, annotation_rngs: vec![]
    }))
  };

  ($parser:expr, $recovery_fn:expr, $msg:expr, $($rngs:expr),+) => {
    (label_with_recovery($parser, $recovery_fn, ParseErrorDetail {
      message: $msg, annotation_rngs: vec![$($rngs),+]
    }))
  };
}

fn label_without_recovery<'a, F, O>(
  mut parser: F,
  error_detail: ParseErrorDetail,
) ->
  impl FnMut(ParseString<'a>) -> ParseResult<O>
where
  F: FnMut(ParseString<'a>) -> ParseResult<O>
{
  move |mut input: ParseString| {
    let index_before_parser = input.cursor;
    match parser(input) {
      Err(Err::Error(mut e)) => {
        e.cause_range = (index_before_parser, e.cause_range.1);
        e.error_detail = error_detail.clone();
        Err(Err::Failure(e))
      }
      x => x,
    }
  }
}

fn label_with_recovery<'a, F, O>(
  mut parser: F,
  mut recovery_fn: fn(ParseString<'a>) -> ParseResult<O>,
  error_detail: ParseErrorDetail,
) ->
  impl FnMut(ParseString<'a>) -> ParseResult<O>
where
  F: FnMut(ParseString<'a>) -> ParseResult<O>
{
  move |mut input: ParseString| {
    let index_before_parser = input.cursor;
    match parser(input) {
      Err(Err::Error(mut e)) => {
        e.cause_range = (index_before_parser, e.cause_range.1);
        e.error_detail = error_detail.clone();
        e.log();
        recovery_fn(e.remaining_input)
      }
      Err(Err::Failure(mut e)) => {
        e.log();
        recovery_fn(e.remaining_input)
      },
      x => x,
    }
  }
}

fn tag(tag: &'static str) -> impl Fn(ParseString) -> ParseResult<String> {
  move |mut input: ParseString| {
    if let Some(matched) = input.consume_tag(tag) {
      Ok((input, matched))
    } else {
      Err(nom::Err::Error(ParseError::new(input, "Unexpected character")))
    }
  }
}

// ## Recovery functions

fn skip_nil(input: ParseString) -> ParseResult<Node> {
  Ok((input, error_token!()))
}

// ## Primitive parsers

fn emoji_grapheme(mut input: ParseString) -> ParseResult<String> {
  if let Some(matched) = input.consume_emoji() {
    Ok((input, matched))
  } else {
    Err(nom::Err::Error(ParseError::new(input, "Unexpected character")))
  }
}

fn alpha(mut input: ParseString) -> ParseResult<String> {
  if let Some(matched) = input.consume_alpha() {
    Ok((input, matched))
  } else {
    Err(nom::Err::Error(ParseError::new(input, "Unexpected character")))
  }
}

fn digit(mut input: ParseString) -> ParseResult<String> {
  if let Some(matched) = input.consume_digit() {
    Ok((input, matched))
  } else {
    Err(nom::Err::Error(ParseError::new(input, "Unexpected character")))
  }
}

// ## Parsers

// ### The basics

macro_rules! leaf {
  ($name:ident, $byte:expr, $token:expr) => (
    fn $name(input: ParseString) -> ParseResult<Node> {
      let (input, _) = tag($byte)(input)?;
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

// emoji ::= emoji_grapheme+ ;
fn emoji(input: ParseString) -> ParseResult<Node> {
  let (input, matching) = many1(emoji_grapheme)(input)?;
  let chars: Vec<Node> = matching.iter().map(|b| Node::Token{token: Token::Emoji, chars: b.chars().collect::<Vec<char>>()}).collect();
  Ok((input, Node::Emoji{children: chars}))
}

// word ::= alpha+ ;
fn word(input: ParseString) -> ParseResult<Node> {
  let (input, matching) = many1(alpha)(input)?;
  let chars: Vec<Node> = matching.iter().map(|b| Node::Token{token: Token::Alpha, chars: b.chars().collect::<Vec<char>>()}).collect();
  Ok((input, Node::Word{children: chars}))
}

// digit1 ::= digit+ ;
fn digit1(input: ParseString) -> ParseResult<Vec<String>> {
  let result = many1(digit)(input)?;
  Ok(result)
}

// digit0 ::= digit* ;
fn digit0(input: ParseString) -> ParseResult<Vec<String>> {
  let result = many0(digit)(input)?;
  Ok(result)
}

fn bin_digit(input: ParseString) -> ParseResult<String> {
  let result = alt((tag("1"),tag("0")))(input)?;
  Ok(result)
}

fn hex_digit(input: ParseString) -> ParseResult<String> {
  let result = alt((digit, tag("a"), tag("b"), tag("c"), tag("d"), tag("e"), tag("f"), 
                           tag("A"), tag("B"), tag("C"), tag("D"), tag("E"), tag("F")))(input)?;
  Ok(result)
}

fn oct_digit(input: ParseString) -> ParseResult<String> {
  let result = alt((tag("0"),tag("1"),tag("2"),tag("3"),tag("4"),tag("5"),tag("6"),tag("7")))(input)?;
  Ok(result)
}

// number ::= digit1 ;
fn number(input: ParseString) -> ParseResult<Node> {
  let (input, matching) = digit1(input)?;
  let chars: Vec<Node> = matching.iter().map(|b| Node::Token{token: Token::Digit, chars: b.chars().collect::<Vec<char>>()}).collect();
  Ok((input, Node::Number{children: chars}))
}

// punctuation ::= period | exclamation | question | comma | colon | semicolon | dash | apostrophe | left_parenthesis | right_parenthesis | left_angle | right_angle | left_brace | right_brace | left_bracket | right_bracket ;
fn punctuation(input: ParseString) -> ParseResult<Node> {
  let (input, punctuation) = alt((period, exclamation, question, comma, colon, semicolon, dash, apostrophe, left_parenthesis, right_parenthesis, left_angle, right_angle, left_brace, right_brace, left_bracket, right_bracket))(input)?;
  Ok((input, Node::Punctuation{children: vec![punctuation]}))
}

// symbol ::= ampersand | bar | at | slash | backslash | hashtag | equal | tilde | plus | asterisk | asterisk | caret | underscore ;
fn symbol(input: ParseString) -> ParseResult<Node> {
  let (input, symbol) = alt((ampersand, bar, at, slash, backslash, hashtag, equal, tilde, plus, asterisk, caret, underscore))(input)?;
  Ok((input, Node::Symbol{children: vec![symbol]}))
}

fn paragraph_symbol(input: ParseString) -> ParseResult<Node> {
  let (input, symbol) = alt((ampersand, at, slash, backslash, asterisk, caret, underscore))(input)?;
  Ok((input, Node::Symbol{children: vec![symbol]}))
}

// text ::= (word | space | number | punctuation | symbol | emoji)+ ;
fn text(input: ParseString) -> ParseResult<Node> {
  let (input, word) = many1(alt((word, space, number, punctuation, symbol, emoji)))(input)?;
  Ok((input, Node::Text{children: word}))
}

fn paragraph_rest(input: ParseString) -> ParseResult<Node> {
  let (input, word) = many1(alt((word, space, number, punctuation, paragraph_symbol, quote, emoji)))(input)?;
  Ok((input, Node::Text{children: word}))
}

fn paragraph_starter(input: ParseString) -> ParseResult<Node> {
  let (input, word) = many1(alt((word, number, quote, left_angle, right_angle, left_bracket, right_bracket, period, exclamation, question, comma, colon, semicolon, left_parenthesis, right_parenthesis, emoji)))(input)?;
  Ok((input, Node::Text{children: word}))
}

fn identifier(input: ParseString) -> ParseResult<Node> {
  let (input, _) = many0(space)(input)?;
  let (input, (word, mut rest)) = tuple((alt((word,emoji)), many0(alt((word, number, dash, slash, emoji)))))(input)?;
  let mut id = vec![word];
  id.append(&mut rest);
  Ok((input, Node::Identifier{children: id}))
}

fn boolean_literal(input: ParseString) -> ParseResult<Node> {
  let (input, boolean) = alt((true_literal, false_literal))(input)?;
  Ok((input, Node::BooleanLiteral{children: vec![boolean]}))
}

fn true_literal(input: ParseString) -> ParseResult<Node> {
  let (input, _) = alt((english_true_literal, true_symbol))(input)?;
  Ok((input, Node::True))
}

fn false_literal(input: ParseString) -> ParseResult<Node> {
  let (input, _) = alt((english_false_literal, false_symbol))(input)?;
  Ok((input, Node::False))
}

fn true_symbol(input: ParseString) -> ParseResult<Node> {
  let (input, _) = tag("✓")(input)?;
  Ok((input, Node::False))
}

fn false_symbol(input: ParseString) -> ParseResult<Node> {
  let (input, _) = tag("✗")(input)?;
  Ok((input, Node::False))
}

fn english_true_literal(input: ParseString) -> ParseResult<Node> {
  let (input, _) = tag("t")(input)?;
  let (input, _) = tag("r")(input)?;
  let (input, _) = tag("u")(input)?;
  let (input, _) = tag("e")(input)?;
  Ok((input, Node::True))
}

fn english_false_literal(input: ParseString) -> ParseResult<Node> {
  let (input, _) = tag("f")(input)?;
  let (input, _) = tag("a")(input)?;
  let (input, _) = tag("l")(input)?;
  let (input, _) = tag("s")(input)?;
  let (input, _) = tag("e")(input)?;
  Ok((input, Node::False))
}

// carriage_newline ::= "\r\n" ;
fn carriage_newline(input: ParseString) -> ParseResult<Node> {
  let (input, _) = tag("\r\n")(input)?;
  Ok((input, Node::Null))
}

// newline ::= new_line_char | carriage_newline ;
fn newline(input: ParseString) -> ParseResult<Node> {
  let (input, _) = alt((new_line_char, carriage_newline))(input)?;
  Ok((input, Node::Null))
}

// whitespace ::= space*, newline+ ;
fn whitespace(input: ParseString) -> ParseResult<Node> {
  let (input, _) = many0(space)(input)?;
  let (input, _) = many1(newline)(input)?;
  Ok((input, Node::Null))
}

fn floating_point(input: ParseString) -> ParseResult<Node> {
  let (input,_) = period(input)?;
  let (input, chars) = digit1(input)?;
  Ok((input, Node::Null))
}

fn number_literal(input: ParseString) -> ParseResult<Node> {
  let (input, number_variant) = alt((hexadecimal_literal, octal_literal, binary_literal, decimal_literal, float_literal))(input)?;
  let (input, kind_id) = opt(kind_annotation)(input)?;
  let mut children = vec![number_variant];
  match kind_id {
    Some(kind_id) => children.push(kind_id),
    _ => (),
  }
  Ok((input, Node::NumberLiteral{children}))
}

fn float_literal(input: ParseString) -> ParseResult<Node> {
  let (input, p1) = opt(tag("."))(input)?;
  let (input, p2) = digit1(input)?;
  let (input, p3) = opt(tag("."))(input)?;
  let (input, p4) = digit0(input)?;
  let mut whole: Vec<char> = vec![];
  if let Some(_) = p1 {
    whole.push('.');
  }
  let mut digits = p2.iter().flat_map(|c| c.chars()).collect::<Vec<char>>();
  whole.append(&mut digits);
  if let Some(_) = p3 {
    whole.push('.');
  }
  let mut digits = p4.iter().flat_map(|c| c.chars()).collect::<Vec<char>>();
  whole.append(&mut digits);
  Ok((input, Node::FloatLiteral{chars: whole}))
}

fn decimal_literal(input: ParseString) -> ParseResult<Node> {
  let (input, _) = tag("0d")(input)?;
  let (input, chars) = digit1(input)?;
  Ok((input, Node::DecimalLiteral{chars: chars.iter().flat_map(|c| c.chars()).collect()}))
}

fn hexadecimal_literal(input: ParseString) -> ParseResult<Node> {
  let (input, _) = tag("0x")(input)?;
  let (input, chars) = many1(hex_digit)(input)?;
  Ok((input, Node::HexadecimalLiteral{chars: chars.iter().flat_map(|c| c.chars()).collect()}))
}

fn octal_literal(input: ParseString) -> ParseResult<Node> {
  let (input, _) = tag("0o")(input)?;
  let (input, chars) = many1(oct_digit)(input)?;
  Ok((input, Node::OctalLiteral{chars: chars.iter().flat_map(|c| c.chars()).collect()}))
}

fn binary_literal(input: ParseString) -> ParseResult<Node> {
  let (input, _) = tag("0b")(input)?;
  let (input, chars) = many1(bin_digit)(input)?;
  Ok((input, Node::BinaryLiteral{chars: chars.iter().flat_map(|c| c.chars()).collect()}))
}

fn value(input: ParseString) -> ParseResult<Node> {
  let (input, value) = alt((empty, boolean_literal, number_literal, string))(input)?;
  Ok((input, Node::Value{children: vec![value]}))
}

fn empty(input: ParseString) -> ParseResult<Node> {
  let (input, _) = many1(underscore)(input)?;
  Ok((input, Node::Empty))
}

// ### Blocks

// #### Data

fn select_all(input: ParseString) -> ParseResult<Node> {
  let (input, _) = colon(input)?;
  Ok((input, Node::SelectAll))
}

fn subscript(input: ParseString) -> ParseResult<Node> {
  let (input, subscript) = alt((select_all, expression, tilde))(input)?;
  let (input, _) = tuple((many0(space), opt(comma), many0(space)))(input)?;
  Ok((input, Node::Subscript{children: vec![subscript]}))
}

fn subscript_index(input: ParseString) -> ParseResult<Node> {
  let (input, _) = range(left_brace)(input)?;
  let (input, subscripts) = many1(subscript)(input)?;
  let (input, _) = right_brace(input)?;
  Ok((input, Node::SubscriptIndex{children: subscripts}))
}

fn single_subscript_index(input: ParseString) -> ParseResult<Node> {
  let (input, _) = left_brace(input)?;
  let (input, subscript) = subscript(input)?;
  let (input, _) = right_brace(input)?;
  Ok((input, Node::SubscriptIndex{children: vec![subscript]}))
}

fn dot_index(input: ParseString) -> ParseResult<Node> {
  let (input, _) = period(input)?;
  let (input, identifier) = identifier(input)?;
  let (input, subscript) = opt(single_subscript_index)(input)?;
  let index = match subscript {
    Some(subscript) =>vec![subscript, identifier],
    None => vec![Node::Null, identifier],
  };
  Ok((input, Node::DotIndex{children: index}))
}

fn swizzle(input: ParseString) -> ParseResult<Node> {
  let (input, _) = period(input)?;
  let (input, first) = identifier(input)?;
  let (input, _) = comma(input)?;
  let (input, mut rest) = separated_list1(tag(","), identifier)(input)?;
  let mut cols = vec![first];
  cols.append(&mut rest);
  Ok((input, Node::Swizzle{children: cols}))
}

fn reshape_column(input: ParseString) -> ParseResult<Node> {
  let (input, _) = left_brace(input)?;
  let (input, _) = colon(input)?;
  let (input, _) = right_brace(input)?;
  Ok((input, Node::ReshapeColumn))
}

fn index(input: ParseString) -> ParseResult<Node> {
  let (input, index) = alt((swizzle, dot_index, reshape_column, subscript_index))(input)?;
  Ok((input, Node::Index{children: vec![index]}))
}

fn data(input: ParseString) -> ParseResult<Node> {
  let (input, source) = alt((table, identifier))(input)?;
  let (input, mut indices) = many0(index)(input)?;
  let mut data = vec![source];
  data.append(&mut indices);
  Ok((input, Node::Data{children: data}))
}

fn kind_annotation(input: ParseString) -> ParseResult<Node> {
  let (input, _) = left_angle(input)?;
  let (input, kind_id) = separated_list1(tag(","), alt((identifier, underscore)))(input)?;
  let (input, _) = right_angle(input)?;
  Ok((input, Node::KindAnnotation{children: kind_id}))
}

// #### Tables

fn table(input: ParseString) -> ParseResult<Node> {
  let (input, _) = hashtag(input)?;
  let (input, table_identifier) = identifier(input)?;
  Ok((input, Node::Table{children: vec![table_identifier]}))
}

fn binding(input: ParseString) -> ParseResult<Node> {
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

fn function_binding(input: ParseString) -> ParseResult<Node> {
  let (input, binding_id) = identifier(input)?;
  let (input, _) = tuple((colon, many0(space)))(input)?;
  let (input, bound) = alt((expression, identifier, value))(input)?;
  let (input, _) = tuple((many0(space), opt(comma), many0(space)))(input)?;
  Ok((input, Node::FunctionBinding{children: vec![binding_id, bound]}))
}

fn table_column(input: ParseString) -> ParseResult<Node> {
  let (input, _) = many0(alt((space, tab)))(input)?;
  let (input, item) = alt((expression, value, data, ))(input)?;
  let (input, _) = tuple((opt(comma), many0(alt((space, tab)))))(input)?;
  Ok((input, Node::Column{children: vec![item]}))
}

fn table_row(input: ParseString) -> ParseResult<Node> {
  let (input, _) = many0(alt((space, tab)))(input)?;
  let (input, columns) = many1(table_column)(input)?;
  let (input, _) = tuple((opt(semicolon), opt(newline)))(input)?;
  Ok((input, Node::TableRow{children: columns}))
}

fn attribute(input: ParseString) -> ParseResult<Node> {
  let mut children = vec![];
  let (input, identifier) = identifier(input)?;
  children.push(identifier);
  let (input, kind) = opt(kind_annotation)(input)?;
  let (input, _) = tuple((many0(space), opt(comma), many0(space)))(input)?;
  if let Some(kind) = kind { children.push(kind); }
  Ok((input, Node::Attribute{children}))
}

fn table_header(input: ParseString) -> ParseResult<Node> {
  let (input, _) = bar(input)?;
  let (input, attributes) = many1(attribute)(input)?;
  let (input, _) = tuple((bar, many0(space), opt(newline)))(input)?;
  Ok((input, Node::TableHeader{children: attributes}))
}

fn anonymous_table(input: ParseString) -> ParseResult<Node> {
  let (input, _) = left_bracket(input)?;
  let (input, _) = many0(alt((space, newline, tab)))(input)?;
  let (input, table_header) = opt(table_header)(input)?;
  let (input, mut table_rows) = many0(alt((comment,table_row)))(input)?;
  let (input, _) = many0(alt((space, newline, tab)))(input)?;
  let (input, _) = right_bracket(input)?;
  let mut table = vec![];
  let mut just_rows = table_rows.iter().filter(|n| 
    match n {
      Node::Comment{..} => false,
      _ => true
    }
  ).cloned().collect();
  match table_header {
    Some(table_header) => table.push(table_header),
    _ => (),
  };
  table.append(&mut just_rows);
  Ok((input, Node::AnonymousTable{children: table}))
}

fn empty_table(input: ParseString) -> ParseResult<Node> {
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

fn anonymous_matrix(input: ParseString) -> ParseResult<Node> {
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

fn inline_table(input: ParseString) -> ParseResult<Node> {
  let (input, _) = left_bracket(input)?;
  let (input, bindings) = many1(binding)(input)?;
  let (input, _) = right_bracket(input)?;
  Ok((input, Node::InlineTable{children: bindings}))
}

// #### Statements

fn comment_sigil(input: ParseString) -> ParseResult<Node> {
  let (input, _) = tag("--")(input)?;
  Ok((input, Node::Null))
}

fn comment(input: ParseString) -> ParseResult<Node> {
  let (input, _) = many0(alt((space, tab)))(input)?;
  let (input, _) = comment_sigil(input)?;
  let (input, comment) = text(input)?;
  let (input, _) = many0(alt((space, tab, newline)))(input)?;
  Ok((input, Node::Comment{children: vec![comment]}))
}

fn add_row_operator(input: ParseString) -> ParseResult<Node> {
  let (input, _) = tag("+=")(input)?;
  Ok((input, Node::Null))
}

fn add_row(input: ParseString) -> ParseResult<Node> {
  let (input, table_id) = table(input)?;
  let (input, _) = tuple((many1(space), add_row_operator, many1(space)))(input)?;
  let (input, table) = alt((expression, inline_table, anonymous_table))(input)?;
  Ok((input, Node::AddRow{children: vec![table_id, table]}))
}

fn add_update_operator(input: ParseString) -> ParseResult<Node> {
  let (input, _) = tag(":+=")(input)?;
  Ok((input, Node::AddUpdate))
}

fn subtract_update_operator(input: ParseString) -> ParseResult<Node> {
  let (input, _) = tag(":-=")(input)?;
  Ok((input, Node::SubtractUpdate))
}

fn multiply_update_operator(input: ParseString) -> ParseResult<Node> {
  let (input, _) = tag(":*=")(input)?;
  Ok((input, Node::MultiplyUpdate))
}

fn divide_update_operator(input: ParseString) -> ParseResult<Node> {
  let (input, _) = tag(":/=")(input)?;
  Ok((input, Node::DivideUpdate))
}

fn update_exponent_operator(input: ParseString) -> ParseResult<Node> {
  let (input, _) = tag(":^=")(input)?;
  Ok((input, Node::ExponentUpdate))
}

fn update_matrix_multiply_operator(input: ParseString) -> ParseResult<Node> {
  let (input, _) = tag(":**=")(input)?;
  Ok((input, Node::Null))
}

fn update_data(input: ParseString) -> ParseResult<Node> {
  let (input, table) = data(input)?;
  let (input, (_,op,_)) = tuple((many1(space), alt((add_update_operator,subtract_update_operator,multiply_update_operator,divide_update_operator)), many1(space)))(input)?;
  let (input, expression) = expression(input)?;
  Ok((input, Node::UpdateData{children: vec![op, table, expression]}))
}

fn set_operator(input: ParseString) -> ParseResult<Node> {
  let (input, _) = tag(":=")(input)?;
  Ok((input, Node::Null))
}

fn set_data(input: ParseString) -> ParseResult<Node> {
  let (input, table) = data(input)?;
  let (input, _) = tuple((many1(space), set_operator, many1(space)))(input)?;
  let (input, expression) = expression(input)?;
  Ok((input, Node::SetData{children: vec![table, expression]}))
}

fn split_data(input: ParseString) -> ParseResult<Node> {
  let (input, table) = alt((identifier, table))(input)?;
  let (input, _) = tuple((many1(space), split_operator, many1(space)))(input)?;
  let (input, expression) = expression(input)?;
  Ok((input, Node::SplitData{children: vec![table, expression]}))
}

fn flatten_data(input: ParseString) -> ParseResult<Node> {
  let (input, table) = identifier(input)?;
  let (input, _) = tuple((many1(space), flatten_operator, many1(space)))(input)?;
  let (input, expression) = expression(input)?;
  Ok((input, Node::FlattenData{children: vec![table, expression]}))
}

fn variable_define(input: ParseString) -> ParseResult<Node> {
  let (input, variable) = identifier(input)?;
  let (input, _) = tuple((many1(space), equal, many1(space)))(input)?;
  let (input, expression) = expression(input)?;
  Ok((input, Node::VariableDefine{children: vec![variable, expression]}))
}

fn table_define(input: ParseString) -> ParseResult<Node> {
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

fn table_select(input: ParseString) -> ParseResult<Node> {
  let (input, expression) = expression(input)?;
  Ok((input, Node::TableSelect{children: vec![expression]}))
}

fn split_operator(input: ParseString) -> ParseResult<Node> {
  let (input, _) = tag(">-")(input)?;
  Ok((input, Node::Null))
}

fn flatten_operator(input: ParseString) -> ParseResult<Node> {
  let (input, _) = tag("-<")(input)?;
  Ok((input, Node::Null))
}

fn whenever_operator(input: ParseString) -> ParseResult<Node> {
  let (input, _) = tag("~")(input)?;
  Ok((input, Node::Null))
}

fn until_operator(input: ParseString) -> ParseResult<Node> {
  let (input, _) = tag("~|")(input)?;
  Ok((input, Node::Null))
}

fn wait_operator(input: ParseString) -> ParseResult<Node> {
  let (input, _) = tag("|~")(input)?;
  Ok((input, Node::Null))
}

fn whenever_data(input: ParseString) -> ParseResult<Node> {
  let (input, _) = whenever_operator(input)?;
  let (input, _) = space(input)?;
  let (input, watch) = alt((variable_define, expression, data))(input)?;
  Ok((input, Node::Whenever{children: vec![watch]}))
}

fn wait_data(input: ParseString) -> ParseResult<Node> {
  let (input, _) = wait_operator(input)?;
  let (input, _) = space(input)?;
  let (input, watch) = alt((variable_define, expression, data))(input)?;
  Ok((input, Node::Wait{children: vec![watch]}))
}

fn until_data(input: ParseString) -> ParseResult<Node> {
  let (input, _) = until_operator(input)?;
  let (input, _) = space(input)?;
  let (input, watch) = alt((variable_define, expression, data))(input)?;
  Ok((input, Node::Until{children: vec![watch]}))
}

fn statement(input: ParseString) -> ParseResult<Node> {
  let (input, statement) = alt((table_define, variable_define, split_data, flatten_data, whenever_data, wait_data, until_data, set_data, update_data, add_row, comment))(input)?;
  Ok((input, Node::Statement{children: vec![statement]}))
}

// #### Expressions

// ##### Math expressions

fn parenthetical_expression(input: ParseString) -> ParseResult<Node> {
  let (input, _) = left_parenthesis(input)?;
  let (input, l0) = l0(input)?;
  let (input, _) = right_parenthesis(input)?;
  Ok((input, l0))
}

fn negation(input: ParseString) -> ParseResult<Node> {
  let (input, _) = dash(input)?;
  let (input, negated) = alt((data, value))(input)?;
  Ok((input, Node::Negation { children: vec![negated] }))
}

fn function(input: ParseString) -> ParseResult<Node> {
  let (input, identifier) = identifier(input)?;
  let (input, _) = left_parenthesis(input)?;
  let (input, mut bindings) = many1(function_binding)(input)?;
  let (input, _) = right_parenthesis(input)?;
  let mut function = vec![identifier];
  function.append(&mut bindings);
  Ok((input, Node::Function { children: function }))
}

fn matrix_multiply(input: ParseString) -> ParseResult<Node> {
  let (input, _) = tag("**")(input)?;
  Ok((input, Node::Null))
}

fn add(input: ParseString) -> ParseResult<Node> {
  let (input, _) = tag("+")(input)?;
  Ok((input, Node::Add))
}

fn subtract(input: ParseString) -> ParseResult<Node> {
  let (input, _) = tag("-")(input)?;
  Ok((input, Node::Subtract))
}

fn multiply(input: ParseString) -> ParseResult<Node> {
  let (input, _) = tag("*")(input)?;
  Ok((input, Node::Multiply))
}

fn divide(input: ParseString) -> ParseResult<Node> {
  let (input, _) = tag("/")(input)?;
  Ok((input, Node::Divide))
}

fn exponent(input: ParseString) -> ParseResult<Node> {
  let (input, _) = tag("^")(input)?;
  Ok((input, Node::Exponent))
}

fn range_op(input: ParseString) -> ParseResult<Node> {
  let (input, _) = tag(":")(input)?;
  Ok((input, Node::Range))
}

fn l0(input: ParseString) -> ParseResult<Node> {
  let (input, l1) = l1(input)?;
  let (input, mut infix) = many0(l0_infix)(input)?;
  let mut math = vec![l1];
  math.append(&mut infix);
  Ok((input, Node::L0 { children: math }))
}

fn l0_infix(input: ParseString) -> ParseResult<Node> {
  let (input, _) = many0(space)(input)?;
  let (input, op) = range_op(input)?;
  let (input, _) = many0(space)(input)?;
  let (input, l1) = l1(input)?;
  Ok((input, Node::L0Infix { children: vec![op, l1] }))
}

fn l1(input: ParseString) -> ParseResult<Node> {
  let (input, l2) = l2(input)?;
  let (input, mut infix) = many0(l1_infix)(input)?;
  let mut math = vec![l2];
  math.append(&mut infix);
  Ok((input, Node::L1 { children: math }))
}

fn l1_infix(input: ParseString) -> ParseResult<Node> {
  let (input, _) = space(input)?;
  let (input, op) = alt((add, subtract))(input)?;
  let (input, _) = space(input)?;
  let (input, l2) = l2(input)?;
  Ok((input, Node::L1Infix { children: vec![op, l2] }))
}

fn l2(input: ParseString) -> ParseResult<Node> {
  let (input, l3) = l3(input)?;
  let (input, mut infix) = many0(l2_infix)(input)?;
  let mut math = vec![l3];
  math.append(&mut infix);
  Ok((input, Node::L2 { children: math }))
}

fn l2_infix(input: ParseString) -> ParseResult<Node> {
  let (input, _) = space(input)?;
  let (input, op) = alt((multiply, divide, matrix_multiply))(input)?;
  let (input, _) = space(input)?;
  let (input, l3) = l3(input)?;
  Ok((input, Node::L2Infix { children: vec![op, l3] }))
}

fn l3(input: ParseString) -> ParseResult<Node> {
  let (input, l4) = l4(input)?;
  let (input, mut infix) = many0(l3_infix)(input)?;
  let mut math = vec![l4];
  math.append(&mut infix);
  Ok((input, Node::L3 { children: math }))
}

fn l3_infix(input: ParseString) -> ParseResult<Node> {
  let (input, _) = space(input)?;
  let (input, op) = exponent(input)?;
  let (input, _) = space(input)?;
  let (input, l4) = l4(input)?;
  Ok((input, Node::L3Infix { children: vec![op, l4] }))
}

fn l4(input: ParseString) -> ParseResult<Node> {
  let (input, l5) = l5(input)?;
  let (input, mut infix) = many0(l4_infix)(input)?;
  let mut math = vec![l5];
  math.append(&mut infix);
  Ok((input, Node::L4 { children: math }))
}

fn l4_infix(input: ParseString) -> ParseResult<Node> {
  let (input, _) = space(input)?;
  let (input, op) = alt((and, or, xor))(input)?;
  let (input, _) = space(input)?;
  let (input, l5) = l5(input)?;
  Ok((input, Node::L4Infix { children: vec![op, l5] }))
}

fn l5(input: ParseString) -> ParseResult<Node> {
  let (input, l6) = l6(input)?;
  let (input, mut infix) = many0(l5_infix)(input)?;
  let mut math = vec![l6];
  math.append(&mut infix);
  Ok((input, Node::L5 { children: math }))
}

fn l5_infix(input: ParseString) -> ParseResult<Node> {
  let (input, _) = space(input)?;
  let (input, op) = alt((not_equal,equal_to, greater_than_equal, greater_than, less_than_equal, less_than))(input)?;
  let (input, _) = space(input)?;
  let (input, l6) = l6(input)?;
  Ok((input, Node::L5Infix { children: vec![op, l6] }))
}

fn l6(input: ParseString) -> ParseResult<Node> {
  let (input, l6) = alt((empty_table, string, anonymous_table, function, value, not, data, negation, parenthetical_expression))(input)?;
  Ok((input, Node::L6 { children: vec![l6] }))
}

fn math_expression(input: ParseString) -> ParseResult<Node> {
  let (input, l0) = l0(input)?;
  Ok((input, Node::MathExpression { children: vec![l0] }))
}

// ##### Filter expressions

fn not_equal(input: ParseString) -> ParseResult<Node> {
  let (input, _) = alt((tag("!="),tag("¬="),tag("≠")))(input)?;
  Ok((input, Node::NotEqual))
}

fn equal_to(input: ParseString) -> ParseResult<Node> {
  let (input, _) = tag("==")(input)?;
  Ok((input, Node::Equal))
}

fn greater_than(input: ParseString) -> ParseResult<Node> {
  let (input, _) = tag(">")(input)?;
  Ok((input, Node::GreaterThan))
}

fn less_than(input: ParseString) -> ParseResult<Node> {
  let (input, _) = tag("<")(input)?;
  Ok((input, Node::LessThan))
}

fn greater_than_equal(input: ParseString) -> ParseResult<Node> {
  let (input, _) = alt((tag(">="),tag("≥")))(input)?;
  Ok((input, Node::GreaterThanEqual))
}

fn less_than_equal(input: ParseString) -> ParseResult<Node> {
  let (input, _) = alt((tag("<="),tag("≤")))(input)?;
  Ok((input, Node::LessThanEqual))
}

// ##### Logic expressions

fn or(input: ParseString) -> ParseResult<Node> {
  let (input, _) = tag("|")(input)?;
  Ok((input, Node::Or))
}

fn and(input: ParseString) -> ParseResult<Node> {
  let (input, _) = tag("&")(input)?;
  Ok((input, Node::And))
}

fn not(input: ParseString) -> ParseResult<Node> {
  let (input, _) = alt((tag("!"), tag("¬")))(input)?;
  let (input, negated) = alt((data, true_literal, false_literal))(input)?;
  Ok((input, Node::Not { children: vec![negated] }))
}

fn xor(input: ParseString) -> ParseResult<Node> {
  let (input, _) = alt((tag("xor"), tag("⊕"), tag("⊻")))(input)?;
  Ok((input, Node::Xor))
}

// ##### Other expressions

fn string(input: ParseString) -> ParseResult<Node> {
  let (input, _) = quote(input)?;
  let (input, text) = many0(text)(input)?;
  let (input, _) = quote(input)?;
  Ok((input, Node::String { children: text }))
}

fn expression(input: ParseString) -> ParseResult<Node> {
  let (input, expression) = alt((inline_table, math_expression, string, empty_table, anonymous_table))(input)?;
  Ok((input, Node::Expression { children: vec![expression] }))
}

// #### Block basics

fn transformation(input: ParseString) -> ParseResult<Node> {
  let (input, statement) = statement(input)?;
  let (input, _) = tuple((many0(space),many0(newline)))(input)?;
  Ok((input, Node::Transformation { children: vec![statement] }))
}

// indented_tfm ::= space, space, transformation ;
fn indented_tfm(input: ParseString) -> ParseResult<Node> {
  let msg1 = "Block indentation has to be exactly 2 spaces";
  let msg2 = "Expect transformation after indentation";
  let (input, (_, r)) = range(tuple((
    space,
    labelr!(space, skip_nil, msg1)
  )))(input)?;
  label!(transformation, msg2, r)(input)
}

// block ::= indented_tfm+, whitespace* ;
fn block(input: ParseString) -> ParseResult<Node> {
  let (input, transformations) = many1(indented_tfm)(input)?;
  let (input, _) = many0(whitespace)(input)?;
  Ok((input, Node::Block { children: transformations }))
}

// ### Markdown

// ul_title ::= space*, text, space*, newline, equal+, space*, newline* ;
fn ul_title(input: ParseString) -> ParseResult<Node> {
  let (input, _) = many0(space)(input)?;
  let (input, text) = text(input)?;
  let (input, _) = many0(space)(input)?;
  let (input, _) = newline(input)?;
  let (input, _) = many1(equal)(input)?;
  let (input, _) = many0(space)(input)?;
  let (input, _) = many0(newline)(input)?;
  Ok((input, Node::Title { children: vec![text] }))
}

// title ::= ul_title ;
fn title(input: ParseString) -> ParseResult<Node> {
  let (input,title) = ul_title(input)?;
  Ok((input, title))
}

fn ul_subtitle(input: ParseString) -> ParseResult<Node> {
  let (input, _) = many0(space)(input)?;
  let (input, text) = text(input)?;
  let (input, _) = many0(space)(input)?;
  let (input, _) = newline(input)?;
  let (input, _) = many1(dash)(input)?;
  let (input, _) = many0(space)(input)?;
  let (input, _) = many0(newline)(input)?;
  Ok((input, Node::Title { children: vec![text] }))
}

fn subtitle(input: ParseString) -> ParseResult<Node> {
  let (input,title) = ul_subtitle(input)?;
  Ok((input, title))
}

fn section_title(input: ParseString) -> ParseResult<Node> {
  let (input, _) = hashtag(input)?;
  let (input, _) = hashtag(input)?;
  let (input, _) = hashtag(input)?;
  let (input, _) = many1(space)(input)?;
  let (input, text) = text(input)?;
  let (input, _) = many0(whitespace)(input)?;
  Ok((input, Node::SectionTitle { children: vec![text] }))
}

fn inline_code(input: ParseString) -> ParseResult<Node> {
  let (input, _) = grave(input)?;
  let (input, text) = text(input)?;
  let (input, _) = grave(input)?;
  let (input, _) = many0(space)(input)?;
  Ok((input, Node::InlineCode { children: vec![text] }))
}

fn paragraph_text(input: ParseString) -> ParseResult<Node> {
  let (input, word) = paragraph_starter(input)?;
  let (input, text) = opt(paragraph_rest)(input)?;
  let mut paragraph = vec![word];
  match text {
    Some(text) => paragraph.push(text),
    _ => (),
  };
  Ok((input, Node::ParagraphText { children: paragraph }))
}

fn paragraph(input: ParseString) -> ParseResult<Node> {
  let (input, paragraph_elements) = many1(
    alt((inline_code, paragraph_text))
  )(input)?;
  let (input, _) = many0(whitespace)(input)?;
  let (input, _) = many0(newline)(input)?;
  Ok((input, Node::Paragraph { children: paragraph_elements }))
}

fn unordered_list(input: ParseString) -> ParseResult<Node> {
  let (input, list_items) = many1(list_item)(input)?;
  let (input, _) = opt(newline)(input)?;
  let (input, _) = many0(whitespace)(input)?;
  Ok((input, Node::UnorderedList { children: list_items }))
}

fn list_item(input: ParseString) -> ParseResult<Node> {
  let (input, _) = dash(input)?;
  let (input, _) = many1(space)(input)?;
  let (input, list_item) = paragraph(input)?;
  let (input, _) = many0(newline)(input)?;
  Ok((input, Node::ListItem { children: vec![list_item] }))
}

fn formatted_text(input: ParseString) -> ParseResult<Node> {
  let (input, formatted) = many0(alt((paragraph_rest, carriage_return, new_line_char)))(input)?;
  Ok((input, Node::FormattedText { children: formatted }))
}

fn code_block(input: ParseString) -> ParseResult<Node> {
  let msg = "";
  let (input, (_, r)) = range(tuple((
    grave,
    label!(grave, msg),
    label!(grave, msg),
  )))(input)?;
  let (input, _) = label!(newline, msg, r)(input)?;
  let (input, text) = formatted_text(input)?;
  let (input, _) = label!(tuple((grave, grave, grave, newline, many0(whitespace))), msg, r)(input)?;
  Ok((input, Node::CodeBlock { children: vec![text] }))
}

// ### Mechdown

fn inline_mech_code(input: ParseString) -> ParseResult<Node> {
  let (input, _) = tuple((left_bracket,left_bracket))(input)?;
  let (input, expression) = expression(input)?;
  let (input, _) = tuple((right_bracket,right_bracket,opt(space)))(input)?;
  Ok((input, Node::InlineMechCode{ children: vec![expression] }))
}

fn mech_code_block(input: ParseString) -> ParseResult<Node> {
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

// ### Start here

// section ::= ((block | code_block | mech_code_block | statement | subtitle | paragraph | unordered_list), whitespace?)+ ;
fn section(input: ParseString) -> ParseResult<Node> {
  let (input, mut section_elements) = many1(
    tuple((
      alt((block, code_block, mech_code_block, statement, subtitle, paragraph, unordered_list)),
      opt(whitespace),
    ))
  )(input)?;
  let mut section = vec![];
  section.append(&mut section_elements.iter().map(|(x,_)|x).cloned().collect());
  Ok((input, Node::Section{ children: section }))
}

// body ::= whitespace*, section+ ;
fn body(input: ParseString) -> ParseResult<Node> {
  let (input, _) = many0(whitespace)(input)?;
  let (input, sections) = many1(section)(input)?;
  Ok((input, Node::Body { children: sections }))
}

fn fragment(input: ParseString) -> ParseResult<Node> {
  let (input, statement) = statement(input)?;
  Ok((input, Node::Fragment { children:  vec![statement] }))
}

// program ::= whitespace?, title?, body, whitespace? ;
fn program(input: ParseString) -> ParseResult<Node> {
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

fn raw_transformation(input: ParseString) -> ParseResult<Node> {
  let (input, statement) = statement(input)?;
  let (input, _) = many0(alt((space,newline,tab)))(input)?;
  Ok((input, Node::Transformation { children:  vec![statement] }))
}

fn parse_block(input: ParseString) -> ParseResult<Node> {
  let (input, transformations) = many1(raw_transformation)(input)?;
  let (input, _) = many0(whitespace)(input)?;
  Ok((input, Node::Block { children:  transformations }))
}

fn parse_mech_fragment(input: ParseString) -> ParseResult<Node> {
  let (input, statement) = statement(input)?;
  Ok((input, Node::Root { children:  vec![statement] }))
}

// parse_mech ::= program | statement ;
fn parse_mech(input: ParseString) -> ParseResult<Node> {
  let (input, mech) = alt((program, statement))(input)?;
  Ok((input, Node::Root { children: vec![mech] }))
}

// ## Parser interfaces

pub type ErrorLog = Vec<(ParseStringRange, ParseErrorDetail)>;

pub fn print_error(error_log: &ErrorLog) -> String {
  let mut result = String::new();
  for (i, (cause_range, err_detail)) in error_log.iter().enumerate() {
    let r = &err_detail.annotation_rngs;
    result.push_str("-------------- Error #{} --------------\n");
    result.push_str(&format!("range: {}, {}\n", cause_range.0, cause_range.1));
    result.push_str(&format!("message: {}\n", err_detail.message));
    result.push_str(&format!("annotations: {:?}\n", err_detail.annotation_rngs));
    result.push('\n');
  }
  result
}

pub fn parse(text: &str) -> Result<Node,MechError> {
  let graphemes = UnicodeSegmentation::graphemes(text, true).collect::<Vec<&str>>();
  let parse_tree = parse_mech(ParseString::new(&graphemes));
  match parse_tree {
    Ok((rest, tree)) => {
      let unparsed = rest.graphemes[rest.cursor..].iter().map(|s| String::from(*s)).collect::<String>();
      if unparsed != "" {
        println!("Unparsed: {}", unparsed);
        if rest.had_error() {
          println!("Error log: \n{}", print_error(&rest.error_log));
        }
        Err(MechError{id: 3302, kind: MechErrorKind::GenericError(unparsed)})
      } else { 
        Ok(tree)
      }
    },
    Err(q) => {
      match q {
        Err::Error(e) | Err::Failure(e) => {
          if e.remaining_input.had_error() {
            println!("Error log: \n{}", print_error(&e.remaining_input.error_log));
          }
          print!("-------------- Last error --------------\n");
          print!("{}", &format!("range: {}, {}\n", e.cause_range.0, e.cause_range.1));
          print!("{}", &format!("message: {}\n", e.error_detail.message));
          print!("{}", &format!("annotations: {:?}\n", e.error_detail.annotation_rngs));
        },
        _ => (),
      }
      Err(MechError{id: 3303, kind: MechErrorKind::None})
    }
  }
}

pub fn parse_fragment(text: &str) -> Result<Node,MechError> {
  let graphemes = UnicodeSegmentation::graphemes(text, true).collect::<Vec<&str>>();
  let parse_tree = parse_mech_fragment(ParseString::new(&graphemes));
  match parse_tree {
    Ok((rest, tree)) => {
      let unparsed = rest.graphemes.iter().map(|s| String::from(*s)).collect::<String>();
      if unparsed != "" {
        Err(MechError{id: 3402, kind: MechErrorKind::GenericError(unparsed)})
      } else { 
        Ok(tree)
      }
    },
    Err(q) => {
      Err(MechError{id: 3403, kind: MechErrorKind::None})
    }
  }
}

