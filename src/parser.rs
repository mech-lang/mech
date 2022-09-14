// # Parser

// ## Prelude

use mech_core::*;
use mech_core::node::*;

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

use std::cmp::Ordering;
use std::collections::HashMap;
use unicode_segmentation::UnicodeSegmentation;
use colored::*;

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

  fn match_tag(&self, tag: &str) -> (bool, usize) {
    let gs = tag.graphemes(true).collect::<Vec<&str>>();
    let gs_len = gs.len();
    if self.len() < gs_len {
      return (false, 0);
    }
    for i in 0..gs_len {
      if self.graphemes[self.cursor + i] != gs[i] {
        return (false, 0);
      }
    }
    (true, gs_len)
  }

  fn consume_one(&mut self) -> Option<String> {
    if self.len() == 0 {
      return None;
    }
    let g = self.graphemes[self.cursor];
    self.cursor += 1;
    Some(g.to_string())
  }

  fn consume_tag(&mut self, tag: &str) -> Option<String> {
    let (matched, gs_len) = self.match_tag(tag);
    if matched {
      self.cursor += gs_len;
      Some(tag.to_string())
    } else {
      None
    }
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
    let index_before_parsing = input.cursor;
    match parser(input) {
      Err(Err::Error(mut e)) => {
        e.cause_range = (index_before_parsing, e.cause_range.1);
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

fn is<'a, F, O>(mut parser: F) ->
  impl FnMut(ParseString<'a>) -> ParseResult<O>
where
  F: FnMut(ParseString<'a>) -> ParseResult<O>
{
  move |input: ParseString| {
    let input_clone = input.clone();
    match parser(input_clone) {
      Ok((_, o)) => Ok((input, o)),
      x => x,
    }
  }
}

fn is_not<'a, F, E>(mut parser: F) ->
  impl FnMut(ParseString<'a>) -> ParseResult<()>
where
  F: FnMut(ParseString<'a>) -> ParseResult<E>
{
  move |input: ParseString| {
    let input_clone = input.clone();
    match parser(input_clone) {
      Err(Err::Error(_)) => Ok((input, ())),
      Err(Err::Failure(e)) => Err(Err::Failure(e)),  // switch to input.clone(), which should be fine
      _ => Err(Err::Error(ParseError::new(input, "Unexpected character")))
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

fn skip_spaces(input: ParseString) -> ParseResult<()> {
  let (input, _) = many0(space)(input)?;
  Ok((input, ()))
}

fn skip_nil(input: ParseString) -> ParseResult<ParserNode> {
  Ok((input, ParserNode::Null))
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

fn any(mut input: ParseString) -> ParseResult<String> {
  if let Some(matched) = input.consume_one() {
    Ok((input, matched))
  } else {
    Err(nom::Err::Error(ParseError::new(input, "Unexpected eof")))
  }
}

// ## Parsers

// ### The basics

macro_rules! leaf {
  ($name:ident, $byte:expr, $token:expr) => (
    fn $name(input: ParseString) -> ParseResult<ParserNode> {
      let (input, _) = tag($byte)(input)?;
      Ok((input, ParserNode::Token{token: $token, chars: $byte.chars().collect::<Vec<char>>()}))
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
fn emoji(input: ParseString) -> ParseResult<ParserNode> {
  let (input, matching) = many1(emoji_grapheme)(input)?;
  let chars: Vec<ParserNode> = matching.iter().map(|b| ParserNode::Token{token: Token::Emoji, chars: b.chars().collect::<Vec<char>>()}).collect();
  Ok((input, ParserNode::Emoji{children: chars}))
}

// word ::= alpha+ ;
fn word(input: ParseString) -> ParseResult<ParserNode> {
  let (input, matching) = many1(alpha)(input)?;
  let chars: Vec<ParserNode> = matching.iter().map(|b| ParserNode::Token{token: Token::Alpha, chars: b.chars().collect::<Vec<char>>()}).collect();
  Ok((input, ParserNode::Word{children: chars}))
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

// bin_digit ::= "0" | "1" ;
fn bin_digit(input: ParseString) -> ParseResult<String> {
  let result = alt((tag("1"),tag("0")))(input)?;
  Ok(result)
}

// hex_digit ::= digit | "a" | "b" | "c" | "d" | "e" | "f" | "A" | "B" | "C" | "D" | "E" | "F" ;
fn hex_digit(input: ParseString) -> ParseResult<String> {
  let result = alt((digit, tag("a"), tag("b"), tag("c"), tag("d"), tag("e"), tag("f"), 
                           tag("A"), tag("B"), tag("C"), tag("D"), tag("E"), tag("F")))(input)?;
  Ok(result)
}

// oct_digit ::= "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" ;
fn oct_digit(input: ParseString) -> ParseResult<String> {
  let result = alt((tag("0"),tag("1"),tag("2"),tag("3"),tag("4"),tag("5"),tag("6"),tag("7")))(input)?;
  Ok(result)
}

// number ::= digit1 ;
fn number(input: ParseString) -> ParseResult<ParserNode> {
  let (input, matching) = digit1(input)?;
  let chars: Vec<ParserNode> = matching.iter().map(|b| ParserNode::Token{token: Token::Digit, chars: b.chars().collect::<Vec<char>>()}).collect();
  Ok((input, ParserNode::Number{children: chars}))
}

// punctuation ::= period | exclamation | question | comma | colon | semicolon | dash | apostrophe | left_parenthesis | right_parenthesis | left_angle | right_angle | left_brace | right_brace | left_bracket | right_bracket ;
fn punctuation(input: ParseString) -> ParseResult<ParserNode> {
  let (input, punctuation) = alt((period, exclamation, question, comma, colon, semicolon, dash, apostrophe, left_parenthesis, right_parenthesis, left_angle, right_angle, left_brace, right_brace, left_bracket, right_bracket))(input)?;
  Ok((input, ParserNode::Punctuation{children: vec![punctuation]}))
}

// symbol ::= ampersand | bar | at | slash | backslash | hashtag | equal | tilde | plus | asterisk | asterisk | caret | underscore ;
fn symbol(input: ParseString) -> ParseResult<ParserNode> {
  let (input, symbol) = alt((ampersand, bar, at, slash, backslash, hashtag, equal, tilde, plus, asterisk, caret, underscore))(input)?;
  Ok((input, ParserNode::Symbol{children: vec![symbol]}))
}

// paragraph_symbol ::= ampersand | at | slash | backslash | asterisk | caret | underscore ;
fn paragraph_symbol(input: ParseString) -> ParseResult<ParserNode> {
  let (input, symbol) = alt((ampersand, at, slash, backslash, asterisk, caret, underscore))(input)?;
  Ok((input, ParserNode::Symbol{children: vec![symbol]}))
}

// text ::= (word | space | number | punctuation | symbol | emoji)+ ;
fn text(input: ParseString) -> ParseResult<ParserNode> {
  let (input, word) = many1(alt((word, space, number, punctuation, symbol, emoji)))(input)?;
  Ok((input, ParserNode::Text{children: word}))
}

// paragraph_rest ::= (word | space | number | punctuation | paragraph_symbol | quote | emoij)+ ;
fn paragraph_rest(input: ParseString) -> ParseResult<ParserNode> {
  let (input, word) = many1(alt((word, space, number, punctuation, paragraph_symbol, quote, emoji)))(input)?;
  Ok((input, ParserNode::Text{children: word}))
}

// paragraph_starter ::= (word | number | quote | left_angle | right_angle | left_bracket | right_bracket | period | exclamation | question | comma | colon | semicolon | left_parenthesis | right_parenthesis | emoji)+ ;
fn paragraph_starter(input: ParseString) -> ParseResult<ParserNode> {
  let (input, word) = many1(alt((word, number, quote, left_angle, right_angle, left_bracket, right_bracket, period, exclamation, question, comma, colon, semicolon, left_parenthesis, right_parenthesis, emoji)))(input)?;
  Ok((input, ParserNode::Text{children: word}))
}

// identifier ::= space*, (word | emoji), (word | number | dash | slash | emoji)* ;
fn identifier(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = many0(space)(input)?;
  let (input, (word, mut rest)) = tuple((alt((word,emoji)), many0(alt((word, number, dash, slash, emoji)))))(input)?;
  let mut id = vec![word];
  id.append(&mut rest);
  Ok((input, ParserNode::Identifier{children: id}))
}

// boolean_literal ::= true_literal | false_literal ;
fn boolean_literal(input: ParseString) -> ParseResult<ParserNode> {
  let (input, boolean) = alt((true_literal, false_literal))(input)?;
  Ok((input, ParserNode::BooleanLiteral{children: vec![boolean]}))
}

// true_literal ::= english_true_literal | true_symbol ;
fn true_literal(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = alt((english_true_literal, true_symbol))(input)?;
  Ok((input, ParserNode::True))
}

// false_literal ::= english_false_literal | false_symbol ;
fn false_literal(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = alt((english_false_literal, false_symbol))(input)?;
  Ok((input, ParserNode::False))
}

// true_symbol ::= "✓" ;
fn true_symbol(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag("✓")(input)?;
  Ok((input, ParserNode::False))
}

// false_symbol ::= "✗" ;
fn false_symbol(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag("✗")(input)?;
  Ok((input, ParserNode::False))
}

// english_true_literal ::= "true" ;
fn english_true_literal(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag("true")(input)?;
  Ok((input, ParserNode::True))
}

// english_false_literal ::= "false" ;
fn english_false_literal(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag("false")(input)?;
  Ok((input, ParserNode::False))
}

// carriage_newline ::= "\r\n" ;
fn carriage_newline(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag("\r\n")(input)?;
  Ok((input, ParserNode::Null))
}

// newline ::= new_line_char | carriage_newline ;
fn newline(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = alt((new_line_char, carriage_newline))(input)?;
  Ok((input, ParserNode::Null))
}

// whitespace ::= space*, newline+ ;
fn whitespace(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = many0(space)(input)?;
  let (input, _) = many1(newline)(input)?;
  Ok((input, ParserNode::Null))
}

// fn floating_point(input: ParseString) -> ParseResult<ParserNode> {
//   let (input,_) = period(input)?;
//   let (input, chars) = digit1(input)?;
//   Ok((input, ParserNode::Null))
// }

// fn quantity(input: ParseString) -> IResult<ParseString, ParserNode> {
//   let (input, number) = number(input)?;
//   let (input, float) = opt(floating_point)(input)?;
//   let (input, unit) = identifier(input)?;
//   let mut quantity = vec![number];
//   match float {
//     Some(fp) => quantity.push(fp),
//     _ => (),
//   };
//   quantity.push(unit);
//   Ok((input, ParserNode::Quantity{children: quantity}))
// }

// number_literal ::= (hexadecimal_literal | octal_literal | binary_literal | decimal_literal | float_literal), kind_annotation? ;
fn number_literal(input: ParseString) -> ParseResult<ParserNode> {
  let (input, number_variant) = alt((hexadecimal_literal, octal_literal, binary_literal, decimal_literal, float_literal))(input)?;
  let (input, kind_id) = opt(kind_annotation)(input)?;
  let mut children = vec![number_variant];
  match kind_id {
    Some(kind_id) => children.push(kind_id),
    _ => (),
  }
  Ok((input, ParserNode::NumberLiteral{children}))
}

// fn rational_number(input: ParseString) -> IResult<ParseString, ParserNode> {
//   let (input, numerator) = alt((quantity, number_literal))(input)?;
//   let (input, _) = tag("/")(input)?;
//   let (input, denominator) = alt((quantity, number_literal))(input)?;
//   Ok((input, ParserNode::Null))
// }

// float_literal ::= "."?, digit1, "."?, digit0 ;
fn float_literal(input: ParseString) -> ParseResult<ParserNode> {
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
  Ok((input, ParserNode::FloatLiteral{chars: whole}))
}

// decimal_literal ::= "0d", <digit1> ;
fn decimal_literal(input: ParseString) -> ParseResult<ParserNode> {
  let msg = "Expect decimal digits after \"0d\"";
  let (input, _) = tag("0d")(input)?;
  let (input, chars) = label!(digit1, msg)(input)?;
  Ok((input, ParserNode::DecimalLiteral{chars: chars.iter().flat_map(|c| c.chars()).collect()}))
}

// hexadecimal_literal ::= "0x", <hex_digit+> ;
fn hexadecimal_literal(input: ParseString) -> ParseResult<ParserNode> {
  let msg = "Expect hexadecimal digits after \"0x\"";
  let (input, _) = tag("0x")(input)?;
  let (input, chars) = label!(many1(hex_digit), msg)(input)?;
  Ok((input, ParserNode::HexadecimalLiteral{chars: chars.iter().flat_map(|c| c.chars()).collect()}))
}

// octal_literal ::= "0o", <oct_digit+> ;
fn octal_literal(input: ParseString) -> ParseResult<ParserNode> {
  let msg = "Expect octal digits after \"0o\"";
  let (input, _) = tag("0o")(input)?;
  let (input, chars) = label!(many1(oct_digit), msg)(input)?;
  Ok((input, ParserNode::OctalLiteral{chars: chars.iter().flat_map(|c| c.chars()).collect()}))
}

// binary_literal ::= "0b", <bin_digit+> ;
fn binary_literal(input: ParseString) -> ParseResult<ParserNode> {
  let msg = "Expect binary digits after \"0b\"";
  let (input, _) = tag("0b")(input)?;
  let (input, chars) = label!(many1(bin_digit), msg)(input)?;
  Ok((input, ParserNode::BinaryLiteral{chars: chars.iter().flat_map(|c| c.chars()).collect()}))
}

// value ::= empty | boolean_literal | number_literal | string ;
fn value(input: ParseString) -> ParseResult<ParserNode> {
  let (input, value) = alt((empty, boolean_literal, number_literal, string))(input)?;
  Ok((input, ParserNode::Value{children: vec![value]}))
}

// empty ::= underscore+ ;
fn empty(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = many1(underscore)(input)?;
  Ok((input, ParserNode::Empty))
}

// ### Blocks

// #### Data

// select_all ::= colon ;
fn select_all(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = colon(input)?;
  Ok((input, ParserNode::SelectAll))
}

// subscript ::= (select_all | expression | tilde), space*, comma?, space* ;
fn subscript(input: ParseString) -> ParseResult<ParserNode> {
  let (input, subscript) = alt((select_all, expression, tilde))(input)?;
  let (input, _) = tuple((many0(space), opt(comma), many0(space)))(input)?;
  Ok((input, ParserNode::Subscript{children: vec![subscript]}))
}

// TODO
// subscript_index ::= left_brace, subscript+, right_brace ;
fn subscript_index(input: ParseString) -> ParseResult<ParserNode> {
  let (input, (_, r)) = range(left_brace)(input)?;
  let (input, subscripts) = many1(subscript)(input)?;
  let (input, _) = right_brace(input)?;
  Ok((input, ParserNode::SubscriptIndex{children: subscripts}))
}

fn single_subscript_index(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = left_brace(input)?;
  let (input, subscript) = subscript(input)?;
  let (input, _) = right_brace(input)?;
  Ok((input, ParserNode::SubscriptIndex{children: vec![subscript]}))
}

fn dot_index(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = period(input)?;
  let (input, identifier) = identifier(input)?;
  let (input, subscript) = opt(single_subscript_index)(input)?;
  let index = match subscript {
    Some(subscript) =>vec![subscript, identifier],
    None => vec![ParserNode::Null, identifier],
  };
  Ok((input, ParserNode::DotIndex{children: index}))
}

fn swizzle(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = period(input)?;
  let (input, first) = identifier(input)?;
  let (input, _) = comma(input)?;
  let (input, mut rest) = separated_list1(tag(","), identifier)(input)?;
  let mut cols = vec![first];
  cols.append(&mut rest);
  Ok((input, ParserNode::Swizzle{children: cols}))
}

fn reshape_column(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = left_brace(input)?;
  let (input, _) = colon(input)?;
  let (input, _) = right_brace(input)?;
  Ok((input, ParserNode::ReshapeColumn))
}

fn index(input: ParseString) -> ParseResult<ParserNode> {
  let (input, index) = alt((swizzle, dot_index, reshape_column, subscript_index))(input)?;
  Ok((input, ParserNode::Index{children: vec![index]}))
}

fn data(input: ParseString) -> ParseResult<ParserNode> {
  let (input, source) = alt((table, identifier))(input)?;
  let (input, mut indices) = many0(index)(input)?;
  let mut data = vec![source];
  data.append(&mut indices);
  Ok((input, ParserNode::Data{children: data}))
}

// kind_annotation ::= left_angle, <(identifier | underscore), (",", (identifier | underscore))*>, <right_angle> ;
fn kind_annotation(input: ParseString) -> ParseResult<ParserNode> {
  let msg2 = "Expect at least one unit in kind annotation";
  let msg3 = "Expect right angle";
  let (input, (_, r)) = range(left_angle)(input)?;
  let (input, kind_id) = label!(separated_list1(tag(","), alt((identifier, underscore))), msg2)(input)?;
  let (input, _) = label!(right_angle, msg3, r)(input)?;
  Ok((input, ParserNode::KindAnnotation{children: kind_id}))
}

// #### Tables

// table ::= hashtag, <identifier> ;
fn table(input: ParseString) -> ParseResult<ParserNode> {
  let msg = "Expect identifier after hashtag";
  let (input, _) = hashtag(input)?;
  let (input, table_identifier) = label!(identifier, msg)(input)?;
  Ok((input, ParserNode::Table{children: vec![table_identifier]}))
}

fn binding(input: ParseString) -> ParseResult<ParserNode> {
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
  Ok((input, ParserNode::Binding{children}))
}

// function_binding ::= identifier, <colon, space*>, <expression | identifier | value>, space*, comma?, space* ;
fn function_binding(input: ParseString) -> ParseResult<ParserNode> {
  let msg1 = "Expect colon ':'";
  let msg2 = "Expect expression, identifier, or value";
  let (input, binding_id) = identifier(input)?;
  let (input, _) = label!(tuple((colon, many0(space))), msg1)(input)?;
  let (input, bound) = label!(alt((expression, identifier, value)), msg2)(input)?;
  let (input, _) = tuple((many0(space), opt(comma), many0(space)))(input)?;
  Ok((input, ParserNode::FunctionBinding{children: vec![binding_id, bound]}))
}

fn table_column(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = many0(alt((space, tab)))(input)?;
  let (input, item) = alt((expression, value, data, ))(input)?;
  let (input, _) = tuple((opt(comma), many0(alt((space, tab)))))(input)?;
  Ok((input, ParserNode::Column{children: vec![item]}))
}

fn table_row(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = many0(alt((space, tab)))(input)?;
  let (input, columns) = many1(table_column)(input)?;
  let (input, _) = tuple((opt(semicolon), opt(newline)))(input)?;
  Ok((input, ParserNode::TableRow{children: columns}))
}

fn attribute(input: ParseString) -> ParseResult<ParserNode> {
  let mut children = vec![];
  let (input, identifier) = identifier(input)?;
  children.push(identifier);
  let (input, kind) = opt(kind_annotation)(input)?;
  let (input, _) = tuple((many0(space), opt(comma), many0(space)))(input)?;
  if let Some(kind) = kind { children.push(kind); }
  Ok((input, ParserNode::Attribute{children}))
}

fn table_header(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = bar(input)?;
  let (input, attributes) = many1(attribute)(input)?;
  let (input, _) = tuple((bar, many0(space), opt(newline)))(input)?;
  Ok((input, ParserNode::TableHeader{children: attributes}))
}

fn anonymous_table(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = left_bracket(input)?;
  let (input, _) = many0(alt((space, newline, tab)))(input)?;
  let (input, table_header) = opt(table_header)(input)?;
  let (input, mut table_rows) = many0(alt((comment,table_row)))(input)?;
  let (input, _) = many0(alt((space, newline, tab)))(input)?;
  let (input, _) = right_bracket(input)?;
  let mut table = vec![];
  let mut just_rows = table_rows.iter().filter(|n| 
    match n {
      ParserNode::Comment{..} => false,
      _ => true
    }
  ).cloned().collect();
  match table_header {
    Some(table_header) => table.push(table_header),
    _ => (),
  };
  table.append(&mut just_rows);
  Ok((input, ParserNode::AnonymousTable{children: table}))
}

fn empty_table(input: ParseString) -> ParseResult<ParserNode> {
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
  Ok((input, ParserNode::EmptyTable{children: table}))
}

fn anonymous_matrix(input: ParseString) -> ParseResult<ParserNode> {
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
  Ok((input, ParserNode::AnonymousMatrix{children: table}))
}

fn inline_table(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = left_bracket(input)?;
  let (input, bindings) = many1(binding)(input)?;
  let (input, _) = right_bracket(input)?;
  Ok((input, ParserNode::InlineTable{children: bindings}))
}

// #### Statements

// stmt_operator ::= split_operator | flatten_operator | set_operator | update_operator | add_row_operator | equal ;
fn stmt_operator(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = alt((split_operator, flatten_operator, set_operator, update_operator, add_row_operator, equal))(input)?;
  Ok((input, ParserNode::Null))
}

// comment_sigil ::= "--" ;
fn comment_sigil(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag("--")(input)?;
  Ok((input, ParserNode::Null))
}

// comment ::= (space | tab)*, comment_sigil, <text>, <tab | newline>, (space | tab | newline)* ;
fn comment(input: ParseString) -> ParseResult<ParserNode> {
  let msg1 = "Expect comment text";
  let msg2 = "Character not allowed in comment text";
  let (input, _) = many0(alt((space, tab)))(input)?;
  let (input, _) = comment_sigil(input)?;
  let (input, comment) = label!(text, msg1)(input)?;
  let (input, _) = label!(alt((tab, newline)), msg2)(input)?;
  let (input, _) = many0(alt((space, tab, newline)))(input)?;
  Ok((input, ParserNode::Comment{children: vec![comment]}))
}

// add_row_operator ::= "+=" ;
fn add_row_operator(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag("+=")(input)?;
  Ok((input, ParserNode::Null))
}

// add_row ::= table, <!stmt_operator>, space+, add_row_operator, <space+>, <expression | inline_table | anonymous_table> ;
fn add_row(input: ParseString) -> ParseResult<ParserNode> {
  let msg1 = "Expect space before operator";
  let msg2 = "Expect space after operator";
  let msg3 = "Expect expression, inline table, or anonymous table";
  let (input, table_id) = table(input)?;
  let (input, _) = label!(is_not(stmt_operator), msg1)(input)?;
  let (input, _) = many1(space)(input)?;
  let (input, _) = add_row_operator(input)?;
  let (input, _) = label!(many1(space), msg2)(input)?;
  let (input, table) = label!(alt((expression, inline_table, anonymous_table)), msg3)(input)?;
  Ok((input, ParserNode::AddRow{children: vec![table_id, table]}))
}

// add_update_operator ::= ":+=" ;
fn add_update_operator(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag(":+=")(input)?;
  Ok((input, ParserNode::AddUpdate))
}

// subtract_update_operator ::= ":-=" ;
fn subtract_update_operator(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag(":-=")(input)?;
  Ok((input, ParserNode::SubtractUpdate))
}

// multiply_update_operator ::= ":*=" ;
fn multiply_update_operator(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag(":*=")(input)?;
  Ok((input, ParserNode::MultiplyUpdate))
}

// divide_update_operator ::= ":/=" ;
fn divide_update_operator(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag(":/=")(input)?;
  Ok((input, ParserNode::DivideUpdate))
}

// fn update_exponent_operator(input: ParseString) -> ParseResult<ParserNode> {
//   let (input, _) = tag(":^=")(input)?;
//   Ok((input, ParserNode::ExponentUpdate))
// }

// fn update_matrix_multiply_operator(input: ParseString) -> ParseResult<ParserNode> {
//   let (input, _) = tag(":**=")(input)?;
//   Ok((input, ParserNode::Null))
// }

// update_operator ::= add_update_operator | subtract_update_operator | multiply_update_operator | divide_update_operator ;
fn update_operator(input: ParseString) -> ParseResult<ParserNode> {
  alt((add_update_operator,subtract_update_operator,multiply_update_operator,divide_update_operator))(input)
}

// update_data ::= data, <!stmt_operator>, space+, update_operator, <space+>, <expression> ;
fn update_data(input: ParseString) -> ParseResult<ParserNode> {
  let msg1 = "Expect space before operator";
  let msg2 = "Expect space after operator";
  let msg3 = "Expect expression";
  let (input, table) = data(input)?;
  let (input, _) = label!(is_not(stmt_operator), msg1)(input)?;
  let (input, _) = many1(space)(input)?;
  let (input, op) = update_operator(input)?;
  let (input, _) = label!(many1(space), msg2)(input)?;
  let (input, expression) = label!(expression, msg3)(input)?;
  Ok((input, ParserNode::UpdateData{children: vec![op, table, expression]}))
}

// set_operator ::= ":=" ;
fn set_operator(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag(":=")(input)?;
  Ok((input, ParserNode::Null))
}

// set_data ::= data, <!stmt_operator>, space+, set_operator, <space+>, <expression> ;
fn set_data(input: ParseString) -> ParseResult<ParserNode> {
  let msg1 = "Expect space before operator";
  let msg2 = "Expect space after operator";
  let msg3 = "Expect expression";
  let (input, table) = data(input)?;
  let (input, _) = label!(is_not(stmt_operator), msg1)(input)?;
  let (input, _) = many1(space)(input)?;
  let (input, _) = set_operator(input)?;
  let (input, _) = label!(many1(space), msg2)(input)?;
  let (input, expression) = label!(expression, msg3)(input)?;
  Ok((input, ParserNode::SetData{children: vec![table, expression]}))
}

// split_data ::= (identifier | table), <!stmt_operator>, space+, split_operator, <space+>, <expression> ;
fn split_data(input: ParseString) -> ParseResult<ParserNode> {
  let msg1 = "Expect space before operator";
  let msg2 = "Expect space after operator";
  let msg3 = "Expect expression";
  let (input, table) = alt((identifier, table))(input)?;
  let (input, _) = label!(is_not(stmt_operator), msg1)(input)?;
  let (input, _) = many1(space)(input)?;
  let (input, _) = split_operator(input)?;
  let (input, _) = label!(many1(space), msg2)(input)?;
  let (input, expression) = label!(expression, msg3)(input)?;
  Ok((input, ParserNode::SplitData{children: vec![table, expression]}))
}

// flatten_data ::= identifier, <!stmt_operator>, space+, flatten_operator, <space+>, <expression> ;
fn flatten_data(input: ParseString) -> ParseResult<ParserNode> {
  let msg1 = "Expect space before operator";
  let msg2 = "Expect space after operator";
  let msg3 = "Expect expression";
  let (input, table) = identifier(input)?;
  let (input, _) = label!(is_not(stmt_operator), msg1)(input)?;
  let (input, _) = many1(space)(input)?;
  let (input, _) = flatten_operator(input)?;
  let (input, _) = label!(many1(space), msg2)(input)?;
  let (input, expression) = label!(expression, msg3)(input)?;
  Ok((input, ParserNode::FlattenData{children: vec![table, expression]}))
}

// variable_define ::= identifier, <!stmt_operator>, space+, equal, <space+>, <expression> ;
fn variable_define(input: ParseString) -> ParseResult<ParserNode> {
  let msg1 = "Expect space before operator";
  let msg2 = "Expect space after operator";
  let msg3 = "Expect expression";
  let (input, variable) = identifier(input)?;
  let (input, _) = label!(is_not(stmt_operator), msg1)(input)?;
  let (input, _) = many1(space)(input)?;
  let (input, _) = equal(input)?;
  let (input, _) = label!(many1(space), msg2)(input)?;
  let (input, expression) = label!(expression, msg3)(input)?;
  Ok((input, ParserNode::VariableDefine{children: vec![variable, expression]}))
}

// table_define ::= table, kind_annotation?, <!stmt_operator>, space+, equal, <space+>, <expression> ;
fn table_define(input: ParseString) -> ParseResult<ParserNode> {
  let msg1 = "Expect space before operator";
  let msg2 = "Expect space after operator";
  let msg3 = "Expect expression";
  let mut children = vec![];
  let (input, table) = table(input)?;
  children.push(table);
  let (input, kind_id) = opt(kind_annotation)(input)?;
  if let Some(kind_id) = kind_id { children.push(kind_id); }
  let (input, _) = label!(is_not(stmt_operator), msg1)(input)?;
  let (input, _) = many1(space)(input)?;
  let (input, _) = equal(input)?;
  let (input, _) = label!(many1(space), msg2)(input)?;
  let (input, expression) = label!(expression, msg3)(input)?;
  children.push(expression);
  Ok((input, ParserNode::TableDefine{children}))
}

// fn table_select(input: ParseString) -> ParseResult<ParserNode> {
//   let (input, expression) = expression(input)?;
//   Ok((input, ParserNode::TableSelect{children: vec![expression]}))
// }

// split_operator ::= ">-" ;
fn split_operator(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag(">-")(input)?;
  Ok((input, ParserNode::Null))
}

// flatten_operator ::= "-<" ;
fn flatten_operator(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag("-<")(input)?;
  Ok((input, ParserNode::Null))
}

// whenever_oeprator ::= "~" ;
fn whenever_operator(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag("~")(input)?;
  Ok((input, ParserNode::Null))
}

// until_operator ::= "~|" ;
fn until_operator(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag("~|")(input)?;
  Ok((input, ParserNode::Null))
}

// wait_operator ::= "|~" ;
fn wait_operator(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag("|~")(input)?;
  Ok((input, ParserNode::Null))
}

// whenever_data ::= whenever_operator, <space>, <!space>, <variable_define | expression | data> ;
fn whenever_data(input: ParseString) -> ParseResult<ParserNode> {
  let msg1 = "Expect exactly 1 space after \"whenever\" operator";
  let msg2 = "Expect variable define, expression, or data";
  let (input, _) = whenever_operator(input)?;
  let (input, _) = label!(space, msg1)(input)?;
  let (input, _) = labelr!(is_not(space), skip_spaces, msg1)(input)?;
  let (input, watch) = label!(alt((variable_define, expression, data)), msg2)(input)?;
  Ok((input, ParserNode::Whenever{children: vec![watch]}))
}

// wait_data ::= wait_operator, <space>, <!space>, <variable_define | expression | data> ;
fn wait_data(input: ParseString) -> ParseResult<ParserNode> {
  let msg1 = "Expect exactly 1 space after \"wait\" operator";
  let msg2 = "Expect variable define, expression, or data";
  let (input, _) = wait_operator(input)?;
  let (input, _) = label!(space, msg1)(input)?;
  let (input, _) = labelr!(is_not(space), skip_spaces, msg1)(input)?;
  let (input, watch) = label!(alt((variable_define, expression, data)), msg2)(input)?;
  Ok((input, ParserNode::Wait{children: vec![watch]}))
}

// until_data ::= until_operator, <space>, <!space>, <variable_define | expression | data> ;
fn until_data(input: ParseString) -> ParseResult<ParserNode> {
  let msg1 = "Expect exactly 1 space after \"until\" operator";
  let msg2 = "Expect variable define, expression, or data";
  let (input, _) = until_operator(input)?;
  let (input, _) = label!(space, msg1)(input)?;
  let (input, _) = labelr!(is_not(space), skip_spaces, msg1)(input)?;
  let (input, watch) = label!(alt((variable_define, expression, data)), msg2)(input)?;
  Ok((input, ParserNode::Until{children: vec![watch]}))
}

// statement ::= table_define | variable_define | split_data | flatten_data | whenever_data | wait_data | until_data | set_data | update_data | add_row | comment ;
fn statement(input: ParseString) -> ParseResult<ParserNode> {
  let (input, statement) = alt((table_define, variable_define, split_data, flatten_data, whenever_data, wait_data, until_data, set_data, update_data, add_row, comment))(input)?;
  Ok((input, ParserNode::Statement{children: vec![statement]}))
}

// #### Expressions

// ##### Math expressions

// parenthetical_expression ::= left_parenthesis, <l0>, <right_parenthesis> ;
fn parenthetical_expression(input: ParseString) -> ParseResult<ParserNode> {
  let msg1 = "Expect expression";
  let msg2 = "Expect right parenthesis ')'";
  let (input, (_, r)) = range(left_parenthesis)(input)?;
  let (input, l0) = label!(l0, msg1)(input)?;
  let (input, _) = label!(right_parenthesis, msg2, r)(input)?;
  Ok((input, l0))
}

fn negation(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = dash(input)?;
  let (input, negated) = alt((data, value))(input)?;
  Ok((input, ParserNode::Negation { children: vec![negated] }))
}

// function ::= identifier, left_parenthesis, <function_binding+>, <right_parenthesis> ;
fn function(input: ParseString) -> ParseResult<ParserNode> {
  let msg1 = "Expect function binding, which should start with identifier";
  let msg2 = "Expect right parenthesis ')'";
  let (input, identifier) = identifier(input)?;
  let (input, (_, r)) = range(left_parenthesis)(input)?;
  let (input, mut bindings) = label!(many1(function_binding), msg1)(input)?;
  let (input, _) = label!(right_parenthesis, msg2, r)(input)?;
  let mut function = vec![identifier];
  function.append(&mut bindings);
  Ok((input, ParserNode::Function { children: function }))
}

// matrix_multiply ::= "**" ;
fn matrix_multiply(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag("**")(input)?;
  Ok((input, ParserNode::Null))
}

// add ::= "+" ;
fn add(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag("+")(input)?;
  Ok((input, ParserNode::Add))
}

// subtract ::= "-" ;
fn subtract(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag("-")(input)?;
  Ok((input, ParserNode::Subtract))
}

// multiply ::= "*" ;
fn multiply(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag("*")(input)?;
  Ok((input, ParserNode::Multiply))
}

// divide ::= "/" ;
fn divide(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag("/")(input)?;
  Ok((input, ParserNode::Divide))
}

// exponent ::= "^" ;
fn exponent(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag("^")(input)?;
  Ok((input, ParserNode::Exponent))
}

// range_op ::= ":" ;
fn range_op(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag(":")(input)?;
  Ok((input, ParserNode::Range))
}

// TODO: l0-l6 needs to be fixed

// l0 ::= l1, l0_infix* ;
fn l0(input: ParseString) -> ParseResult<ParserNode> {
  let (input, l1) = l1(input)?;
  let (input, mut infix) = many0(l0_infix)(input)?;
  let mut math = vec![l1];
  math.append(&mut infix);
  Ok((input, ParserNode::L0 { children: math }))
}

// l0_infix ::= space*, range_op, space*, l1 ;
fn l0_infix(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = many0(space)(input)?;
  let (input, op) = range_op(input)?;
  let (input, _) = many0(space)(input)?;
  let (input, l1) = l1(input)?;
  Ok((input, ParserNode::L0Infix { children: vec![op, l1] }))
}

// l1 ::= l2, l1_infix* ;
fn l1(input: ParseString) -> ParseResult<ParserNode> {
  let (input, l2) = l2(input)?;
  let (input, mut infix) = many0(l1_infix)(input)?;
  let mut math = vec![l2];
  math.append(&mut infix);
  Ok((input, ParserNode::L1 { children: math }))
}

// l1_infix ::= space, (add | subtract), space, l2 ;
fn l1_infix(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = space(input)?;
  let (input, op) = alt((add, subtract))(input)?;
  let (input, _) = space(input)?;
  let (input, l2) = l2(input)?;
  Ok((input, ParserNode::L1Infix { children: vec![op, l2] }))
}

// l2 ::= l3, l2_infix* ;
fn l2(input: ParseString) -> ParseResult<ParserNode> {
  let (input, l3) = l3(input)?;
  let (input, mut infix) = many0(l2_infix)(input)?;
  let mut math = vec![l3];
  math.append(&mut infix);
  Ok((input, ParserNode::L2 { children: math }))
}

// l2_infix ::= space, (multiply | divide | matrix_multiply), space, l3 ;
fn l2_infix(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = space(input)?;
  let (input, op) = alt((multiply, divide, matrix_multiply))(input)?;
  let (input, _) = space(input)?;
  let (input, l3) = l3(input)?;
  Ok((input, ParserNode::L2Infix { children: vec![op, l3] }))
}

// l3 ::= l4, l3_infix* ;
fn l3(input: ParseString) -> ParseResult<ParserNode> {
  let (input, l4) = l4(input)?;
  let (input, mut infix) = many0(l3_infix)(input)?;
  let mut math = vec![l4];
  math.append(&mut infix);
  Ok((input, ParserNode::L3 { children: math }))
}

// l3_infix ::= space, exponent, space, l4 ;
fn l3_infix(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = space(input)?;
  let (input, op) = exponent(input)?;
  let (input, _) = space(input)?;
  let (input, l4) = l4(input)?;
  Ok((input, ParserNode::L3Infix { children: vec![op, l4] }))
}

// l4 ::= l5, l4_infix* ;
fn l4(input: ParseString) -> ParseResult<ParserNode> {
  let (input, l5) = l5(input)?;
  let (input, mut infix) = many0(l4_infix)(input)?;
  let mut math = vec![l5];
  math.append(&mut infix);
  Ok((input, ParserNode::L4 { children: math }))
}

// l4_infix ::= space, (and | or | xor), space, l5 ;
fn l4_infix(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = space(input)?;
  let (input, op) = alt((and, or, xor))(input)?;
  let (input, _) = space(input)?;
  let (input, l5) = l5(input)?;
  Ok((input, ParserNode::L4Infix { children: vec![op, l5] }))
}

// l5 ::= l6, l5_infix* ;
fn l5(input: ParseString) -> ParseResult<ParserNode> {
  let (input, l6) = l6(input)?;
  let (input, mut infix) = many0(l5_infix)(input)?;
  let mut math = vec![l6];
  math.append(&mut infix);
  Ok((input, ParserNode::L5 { children: math }))
}

// l5_infix ::= space, (not_equal | equal_to | greater_than_equal | greater_than | less_than_equal | less_than), space, l6 ;
fn l5_infix(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = space(input)?;
  let (input, op) = alt((not_equal,equal_to, greater_than_equal, greater_than, less_than_equal, less_than))(input)?;
  let (input, _) = space(input)?;
  let (input, l6) = l6(input)?;
  Ok((input, ParserNode::L5Infix { children: vec![op, l6] }))
}

// l6 ::= empty_table | string | anonymous_table | function | value | not | data | negation | parenthetical_expression ;
fn l6(input: ParseString) -> ParseResult<ParserNode> {
  let (input, l6) = alt((empty_table, string, anonymous_table, function, value, not, data, negation, parenthetical_expression))(input)?;
  Ok((input, ParserNode::L6 { children: vec![l6] }))
}

// math_expression ::= l0 ;
fn math_expression(input: ParseString) -> ParseResult<ParserNode> {
  let (input, l0) = l0(input)?;
  Ok((input, ParserNode::MathExpression { children: vec![l0] }))
}

// ##### Filter expressions

// not_equal ::= "!=" | "¬=" | "≠" ;
fn not_equal(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = alt((tag("!="),tag("¬="),tag("≠")))(input)?;
  Ok((input, ParserNode::NotEqual))
}

// equal_to ::= "==" ;
fn equal_to(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag("==")(input)?;
  Ok((input, ParserNode::Equal))
}

// greater_than ::= ">" ;
fn greater_than(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag(">")(input)?;
  Ok((input, ParserNode::GreaterThan))
}

// less_than ::= "<" ;
fn less_than(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag("<")(input)?;
  Ok((input, ParserNode::LessThan))
}

// greater_than_equal ::= ">=" | "≥" ;
fn greater_than_equal(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = alt((tag(">="),tag("≥")))(input)?;
  Ok((input, ParserNode::GreaterThanEqual))
}

// less_than_equal ::= "<=" | "≤" ;
fn less_than_equal(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = alt((tag("<="),tag("≤")))(input)?;
  Ok((input, ParserNode::LessThanEqual))
}

// ##### Logic expressions

// or ::= "|" ;
fn or(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag("|")(input)?;
  Ok((input, ParserNode::Or))
}

// and ::= "&" ;
fn and(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag("&")(input)?;
  Ok((input, ParserNode::And))
}

// not ::= "!" | "¬" ;
fn not(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = alt((tag("!"), tag("¬")))(input)?;
  let (input, negated) = alt((data, true_literal, false_literal))(input)?;
  Ok((input, ParserNode::Not { children: vec![negated] }))
}

// xor ::= "xor" | "⊕" | "⊻" ;
fn xor(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = alt((tag("xor"), tag("⊕"), tag("⊻")))(input)?;
  Ok((input, ParserNode::Xor))
}

// ##### Other expressions

// fn string_interpolation(input: ParseString) -> IResult<ParseString, ParserNode> {
//   let (input, _) = tag("{{")(input)?;
//   let (input, expression) = expression(input)?;
//   let (input, _) = tag("}}")(input)?;
//   Ok((input, ParserNode::StringInterpolation { children: vec![expression] }))
// }

// string ::= quote, (!quote, text)*, quote ;
fn string(input: ParseString) -> ParseResult<ParserNode> {
  let msg = "Character not allowed in string";
  let (input, _) = quote(input)?;
  let (input, matched) = many0(tuple((is_not(quote), label!(text, msg))))(input)?;
  let (input, _) = quote(input)?;
  let (_, text): ((), Vec<_>) = matched.into_iter().unzip();
  Ok((input, ParserNode::String { children: text }))
}

// expression ::= inline_table | math_expression | string | empty_table | anonymous_table ;
fn expression(input: ParseString) -> ParseResult<ParserNode> {
  let (input, expression) = alt((inline_table, math_expression, string, empty_table, anonymous_table))(input)?;
  Ok((input, ParserNode::Expression { children: vec![expression] }))
}

// #### Block basics

// transformation ::= statement, space*, newline* ;
fn transformation(input: ParseString) -> ParseResult<ParserNode> {
  let (input, statement) = statement(input)?;
  let (input, _) = tuple((many0(space),many0(newline)))(input)?;
  Ok((input, ParserNode::Transformation { children: vec![statement] }))
}

// empty_line ::= space*, newline ;
fn empty_line(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tuple((many0(space), newline))(input)?;
  Ok((input, ParserNode::Null))
}

// indented_tfm ::= !empty_line, space, <space>, <!space>, <transformation> ;
fn indented_tfm(input: ParseString) -> ParseResult<ParserNode> {
  let msg1 = "Block indentation has to be exactly 2 spaces";
  let msg2 = "Expect transformation after block indentation";
  let (input, _) = tuple((
    is_not(empty_line),
    space,
    labelr!(space, skip_nil, msg1),
    labelr!(is_not(space), skip_spaces, msg1),
  ))(input)?;
  label!(transformation, msg2)(input)
}

// block ::= indented_tfm+, whitespace* ;
fn block(input: ParseString) -> ParseResult<ParserNode> {
  let (input, transformations) = many1(indented_tfm)(input)?;
  let (input, _) = many0(whitespace)(input)?;
  Ok((input, ParserNode::Block { children: transformations }))
}

// ### Markdown

// ul_title ::= space*, text, space*, newline, equal+, space*, newline* ;
fn ul_title(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = many0(space)(input)?;
  let (input, text) = text(input)?;
  let (input, _) = many0(space)(input)?;
  let (input, _) = newline(input)?;
  let (input, _) = many1(equal)(input)?;
  let (input, _) = many0(space)(input)?;
  let (input, _) = many0(newline)(input)?;
  Ok((input, ParserNode::Title { children: vec![text] }))
}

// title ::= ul_title ;
fn title(input: ParseString) -> ParseResult<ParserNode> {
  let (input,title) = ul_title(input)?;
  Ok((input, title))
}

fn ul_subtitle(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = many0(space)(input)?;
  let (input, text) = text(input)?;
  let (input, _) = many0(space)(input)?;
  let (input, _) = newline(input)?;
  let (input, _) = many1(dash)(input)?;
  let (input, _) = many0(space)(input)?;
  let (input, _) = many0(newline)(input)?;
  Ok((input, ParserNode::Title { children: vec![text] }))
}

fn subtitle(input: ParseString) -> ParseResult<ParserNode> {
  let (input,title) = ul_subtitle(input)?;
  Ok((input, title))
}

// fn section_title(input: ParseString) -> ParseResult<ParserNode> {
//   let (input, _) = hashtag(input)?;
//   let (input, _) = hashtag(input)?;
//   let (input, _) = hashtag(input)?;
//   let (input, _) = many1(space)(input)?;
//   let (input, text) = text(input)?;
//   let (input, _) = many0(whitespace)(input)?;
//   Ok((input, ParserNode::SectionTitle { children: vec![text] }))
// }

fn inline_code(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = grave(input)?;
  let (input, text) = text(input)?;
  let (input, _) = grave(input)?;
  let (input, _) = many0(space)(input)?;
  Ok((input, ParserNode::InlineCode { children: vec![text] }))
}

fn paragraph_text(input: ParseString) -> ParseResult<ParserNode> {
  let (input, word) = paragraph_starter(input)?;
  let (input, text) = opt(paragraph_rest)(input)?;
  let mut paragraph = vec![word];
  match text {
    Some(text) => paragraph.push(text),
    _ => (),
  };
  Ok((input, ParserNode::ParagraphText { children: paragraph }))
}

fn paragraph(input: ParseString) -> ParseResult<ParserNode> {
  let (input, paragraph_elements) = many1(
    alt((inline_code, paragraph_text))
  )(input)?;
  let (input, _) = many0(whitespace)(input)?;
  let (input, _) = many0(newline)(input)?;
  Ok((input, ParserNode::Paragraph { children: paragraph_elements }))
}

fn unordered_list(input: ParseString) -> ParseResult<ParserNode> {
  let (input, list_items) = many1(list_item)(input)?;
  let (input, _) = opt(newline)(input)?;
  let (input, _) = many0(whitespace)(input)?;
  Ok((input, ParserNode::UnorderedList { children: list_items }))
}

fn list_item(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = dash(input)?;
  let (input, _) = many1(space)(input)?;
  let (input, list_item) = paragraph(input)?;
  let (input, _) = many0(newline)(input)?;
  Ok((input, ParserNode::ListItem { children: vec![list_item] }))
}

// formatted_text ::= (!grave, !eof, <paragraph_rest | carriage_return | new_line_char>)* ;
fn formatted_text(input: ParseString) -> ParseResult<ParserNode> {
  let msg = "Character not permitted in formatted text";
  let (input, result) = many0(tuple((
    tuple((is_not(grave), is_not(eof))),
    label!(alt((paragraph_rest, carriage_return, new_line_char)), msg)
  )))(input)?;
  let (_, formatted): (((), ()), Vec<_>) = result.into_iter().unzip();
  Ok((input, ParserNode::FormattedText { children: formatted }))
}

// code_block ::= grave, <grave>, <grave>, <newline>, formatted_text, <grave{3}, newline, whitespace*> ;
fn code_block(input: ParseString) -> ParseResult<ParserNode> {
  let msg1 = "Expect 3 graves to start a code block";
  let msg2 = "Expect newline";
  let msg3 = "Expect 3 graves followed by newline to terminate a code block";
  let (input, (_, r)) = range(tuple((
    grave,
    label!(grave, msg1),
    label!(grave, msg1),
  )))(input)?;
  let (input, _) = label!(newline, msg2)(input)?;
  let (input, text) = formatted_text(input)?;
  let (input, _) = label!(tuple((grave, grave, grave, newline, many0(whitespace))), msg3, r)(input)?;
  Ok((input, ParserNode::CodeBlock { children: vec![text] }))
}

// ### Mechdown

// fn inline_mech_code(input: ParseString) -> ParseResult<ParserNode> {
//   let (input, _) = tuple((left_bracket,left_bracket))(input)?;
//   let (input, expression) = expression(input)?;
//   let (input, _) = tuple((right_bracket,right_bracket,opt(space)))(input)?;
//   Ok((input, ParserNode::InlineMechCode{ children: vec![expression] }))
// }

// mech_code_block ::= grave{3}, !!"mec", <"mech:">, text?, <newline>, <block>, <grave{3}, newline>, whitespace* ;
fn mech_code_block(input: ParseString) -> ParseResult<ParserNode> {
  let msg1 = "Expect newline";
  let msg2 = "Expect mech code block";
  let msg3 = "Expect the \"mech:\" tag";
  let msg4 = "Expect 3 graves followed by newline to terminate the mech code block";
  let (input, (_, r)) = range(tuple((grave, grave, grave)))(input)?;
  let (input, _) = tuple((is(tag("mec")), label!(tag("mech:"), msg3)))(input)?;
  let (input, directive) = opt(text)(input)?;
  let (input, _) = label!(newline, msg1)(input)?;
  let (input, mech_block) = label!(block, msg2)(input)?;
  let (input, _) = label!(tuple((grave, grave, grave, newline)), msg4, r)(input)?;
  let (input, _) = many0(whitespace)(input)?;
  let mut elements = vec![];
  match directive {
    Some(directive) => elements.push(directive),
    _ => (),
  }
  elements.push(mech_block);
  Ok((input, ParserNode::MechCodeBlock{ children: elements }))
}

// ### Start here

// TODO
// section ::= ((block | mech_code_block | code_block | statement | subtitle | paragraph | unordered_list), whitespace?)+ ;
fn section(input: ParseString) -> ParseResult<ParserNode> {
  let (input, mut section_elements) = many1(
    tuple((
      alt((block, mech_code_block, code_block, statement, subtitle, paragraph, unordered_list)),
      opt(whitespace),
    ))
  )(input)?;
  let mut section = vec![];
  section.append(&mut section_elements.iter().map(|(x,_)|x).cloned().collect());
  Ok((input, ParserNode::Section{ children: section }))
}

// body ::= whitespace*, section+ ;
fn body(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = many0(whitespace)(input)?;
  let (input, sections) = many1(section)(input)?;
  Ok((input, ParserNode::Body { children: sections }))
}

// fn fragment(input: ParseString) -> ParseResult<ParserNode> {
//   let (input, statement) = statement(input)?;
//   Ok((input, ParserNode::Fragment { children:  vec![statement] }))
// }

// program ::= whitespace?, title?, body, whitespace? ;
fn program(input: ParseString) -> ParseResult<ParserNode> {
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
  Ok((input, ParserNode::Program { children: program }))
}

// fn raw_transformation(input: ParseString) -> ParseResult<ParserNode> {
//   let (input, statement) = statement(input)?;
//   let (input, _) = many0(alt((space,newline,tab)))(input)?;
//   Ok((input, ParserNode::Transformation { children:  vec![statement] }))
// }

// fn parse_block(input: ParseString) -> ParseResult<ParserNode> {
//   let (input, transformations) = many1(raw_transformation)(input)?;
//   let (input, _) = many0(whitespace)(input)?;
//   Ok((input, ParserNode::Block { children:  transformations }))
// }

// parse_mech_fragment ::= statement ;
fn parse_mech_fragment(input: ParseString) -> ParseResult<ParserNode> {
  let (input, statement) = statement(input)?;
  Ok((input, ParserNode::Root { children:  vec![statement] }))
}

// parse_mech ::= program | statement ;
fn parse_mech(input: ParseString) -> ParseResult<ParserNode> {
  let (input, mech) = alt((program, statement))(input)?;
  Ok((input, ParserNode::Root { children: vec![mech] }))
}

// ## Reporting errors

struct IndexInterpreter<'a> {
  graphemes: &'a Vec<&'a str>,
  line_beginnings: Vec<usize>,
  end_index: usize,
}

impl<'a> IndexInterpreter<'a> {
  fn new(graphemes: &'a Vec<&str>) -> Self {
    let mut line_beginnings = vec![0];
    for i in 0..graphemes.len() {
      if Self::grapheme_is_newline(graphemes[i]) {
        line_beginnings.push(i + 1);
      }
    }
    line_beginnings.pop();
    IndexInterpreter {
      end_index: graphemes.len(),
      graphemes,
      line_beginnings,
    }
  }

  fn index_is_at_line(&self, index: usize, linenum: usize) -> bool {
    let line_index = linenum - 1;
    let line_rng = self.get_line_range(linenum).unwrap();
    if line_rng.1 == self.end_index {  // linenum is the last line
      index >= line_rng.0
    } else {
      index >= line_rng.0 && index < line_rng.1
    }
  }

  fn get_line_range(&self, linenum: usize) -> Option<(usize, usize)> {
    let line_index = linenum - 1;
    if line_index >= self.line_beginnings.len() {
      return None;
    }
    if linenum == self.line_beginnings.len() {  // asking for the last line
      return Some((self.line_beginnings[line_index], self.end_index));
    }
    Some((self.line_beginnings[line_index], self.line_beginnings[linenum]))
  }

  fn get_text_by_linenum(&self, linenum: usize) -> String {
    let (start, end) = self.get_line_range(linenum).unwrap();
    let mut s = self.graphemes[start..end].iter().map(|s| *s).collect::<String>();
    if !s.ends_with("\n") {
      s.push('\n');
    }
    s
  }

  fn get_textlen_by_linenum(&self, linenum: usize) -> usize {
    let (start, end) = self.get_line_range(linenum).unwrap();
    let mut len = 0;
    for i in start..end {
      len += Self::grapheme_width(self.graphemes[i]);
    }
    len + 1
  }

  fn get_location_by_index(&self, index: usize) -> (usize, usize) {
    let a = self.line_beginnings.binary_search_by(
      |n| if n <= &index { Ordering::Equal } else { Ordering::Greater }).unwrap();
    let mut i = 1;
    while !self.index_is_at_line(index, a + i) {
      i += 1;
    }
    let row = a + i;
    let row_beginning = self.line_beginnings[row - 1];
    let mut col = 1;
    for j in row_beginning..index {
      col += Self::grapheme_width(self.graphemes[j]);
    }
    (row, col)
  }

  fn grapheme_width(grapheme: &str) -> usize {
    let mut width = 0;
    for ch in grapheme.chars() {
      if ch.is_ascii() {
        if !ch.is_ascii_control() || ch == '\t' {
          width += 1;
        }  // else width += 0
      } else if ch.is_alphanumeric() {  // TODO: unicode width?
        width += 2;
      } else {
        return 2;
      }
    }
    width
  }

  fn grapheme_is_newline(grapheme: &str) -> bool {
    for ch in grapheme.chars() {
      if ch == '\n' {
        return true;
      }
    }
    false
  }
}

pub struct ParserErrorReport {
  errors: Vec<(ParseStringRange, ParseErrorDetail)>,
}

impl ParserErrorReport {
  fn heading_color(s: &str) -> String {
    s.truecolor(246, 192, 78).bold().to_string()
  }

  fn location_color(s: &str) -> String {
    s.blue().bold().to_string()
  }

  fn linenum_color(s: &str) -> String {
    s.blue().bold().to_string()
  }

  fn text_color(s: &str) -> String {
    s.to_string()
  }

  fn annotation_color(s: &str) -> String {
    s.bright_purple().bold().to_string()
  }

  fn error_color(s: &str) -> String {
    s.red().bold().to_string()
  }

  fn add_err_heading(result: &mut String, index: usize) {
    let n = index + 1;
    let d = "---------------------";
    let s = format!("{} syntax error #{} {}\n", d, n, d);
    result.push_str(&Self::heading_color(&s));
  }

  fn add_err_location(result: &mut String,
                      cause_rng: &ParseStringRange,
                      ii: &IndexInterpreter) {
    let (row, col) = ii.get_location_by_index(cause_rng.1 - 1);
    let s = format!("@location:{}:{}\n", row, col);
    result.push_str(&Self::location_color(&s));
  }

  fn add_err_context(result: &mut String,
                     cause_rng: &ParseStringRange,
                     detail: &ParseErrorDetail,
                     ii: &IndexInterpreter) {
    // cloning simplifies things here, and it shouldn't impact performance
    let mut annotation_rngs = detail.annotation_rngs.clone();
    annotation_rngs.push(*cause_rng);

    // the lines to print
    let mut lines_to_print: Vec<usize> = vec![];
    for (a, b) in &annotation_rngs {
      let (r1, _) = ii.get_location_by_index(*a);
      let (r2, _) = ii.get_location_by_index(b - 1);
      for i in r1..=r2 {
        lines_to_print.push(i);
      }
    }
    lines_to_print.sort();
    lines_to_print.dedup();

    // the annotations on each line
    // <linenum, Vec<(start_col, rng_len, is_major, is_cause)>>
    let mut range_table: HashMap<usize, Vec<(usize, usize, bool, bool)>> = HashMap::new();
    for line in &lines_to_print {
      range_table.insert(*line, vec![]);
    }
    let n = annotation_rngs.len() - 1;  // if i == n, it's the last rng, i.e. the cause rng
    for (i, (a, b)) in annotation_rngs.iter().enumerate() {
      let (r1, c1) = ii.get_location_by_index(*a);
      let (r2, c2) = ii.get_location_by_index(b - 1);
      if r1 == r2 {  // the entire range is on one line
        range_table.get_mut(&r1).unwrap().push((c1, c2 - c1 + 1, true, i == n));
      } else {  // the range spans over multiple lines
        range_table.get_mut(&r1).unwrap().push((c1, usize::MAX, i != n, i == n));
        for r in r1+1..r2 {
          range_table.get_mut(&r).unwrap().push((1, usize::MAX, false, i == n));
        }
        range_table.get_mut(&r2).unwrap().push((1, c2, i == n, i == n));
      }
    }

    // other data for printing
    let dots = "...";
    let indentation = " ";
    let vert_split1 = " |";
    let vert_split2 = "  ";
    let arrow = "^";
    let tilde = "~";
    let lines_str: Vec<String> = lines_to_print.iter().map(|i| i.to_string()).collect();
    let row_str_len = usize::max(lines_str.last().unwrap().len(), dots.len());

    // print source code
    for i in 0..lines_to_print.len() {
      // [... | ]
      if i != 0 && (lines_to_print[i] - lines_to_print[i-1] != 1) {
        result.push_str(indentation);
        for _ in 3..row_str_len { result.push(' '); }
        result.push_str(&Self::linenum_color(dots));
        result.push_str(&Self::linenum_color(vert_split1));
        result.push('\n');
      }

      // [    | ]
      result.push_str(indentation);
      for _ in 0..row_str_len { result.push(' '); }
      result.push_str(&Self::linenum_color(vert_split1));
      result.push('\n');

      // [row |  program text...]
      let text = ii.get_text_by_linenum(lines_to_print[i]);
      result.push_str(indentation);
      for _ in 0..row_str_len-lines_str[i].len() { result.push(' '); }
      result.push_str(&Self::linenum_color(&lines_str[i]));
      result.push_str(&Self::linenum_color(vert_split1));
      result.push_str(&Self::text_color(&text));

      // [    |    ^~~~]
      result.push_str(indentation);
      for _ in 0..row_str_len { result.push(' '); }
      result.push_str(&Self::linenum_color(vert_split1));
      let mut curr_col = 1;
      let line_len = ii.get_textlen_by_linenum(lines_to_print[i]);
      let rngs = range_table.get(&lines_to_print[i]).unwrap();
      for (start, len, major, cause) in rngs {
        let max_len = usize::max(1, usize::min(*len, line_len - curr_col + 1));
        for _ in curr_col..*start { result.push(' '); }
        if *cause {
          for _ in 0..max_len-1 {
            result.push_str(&Self::error_color(tilde));
          }
          if *major {
            result.push_str(&Self::error_color(arrow));
          } else {
            result.push_str(&Self::error_color(tilde));
          }
        } else {
          if *major {
            result.push_str(&Self::annotation_color(arrow));
          } else {
            result.push_str(&Self::annotation_color(tilde));
          }
          for _ in 0..max_len-1 {
            result.push_str(&Self::annotation_color(tilde));
          }
        }
        curr_col = start + max_len;
      }
      result.push('\n');
    }

    // print error message
    let (_cause_row, cause_col) = ii.get_location_by_index(cause_rng.1 - 1);
    result.push_str(indentation);
    for _ in 0..row_str_len { result.push(' '); }
    result.push_str(vert_split2);
    for _ in 0..cause_col-1 { result.push(' '); }
    result.push_str(&Self::error_color(detail.message));
    result.push('\n');
  }

  pub fn construct_msg(&self, text: &str) -> String {
    let graphemes = UnicodeSegmentation::graphemes(text, true).collect::<Vec<&str>>();
    let ii = IndexInterpreter::new(&graphemes);
    let mut result = String::new();
    result.push('\n');
    for (i, (rng, detail)) in self.errors.iter().enumerate() {
      Self::add_err_heading(&mut result, i);
      Self::add_err_location(&mut result, rng, &ii);
      Self::add_err_context(&mut result, rng, detail, &ii);
      result.push('\n');
    }
    result
  }

  pub fn has_error(&self) -> bool {
    self.errors.len() != 0
  }
}

// ## Parser interfaces

pub fn parse(text: &str) -> Result<ParserNode, MechError> {
  let graphemes = UnicodeSegmentation::graphemes(text, true).collect::<Vec<&str>>();
  let mut result_node = ParserNode::Error;
  let mut error_log = ParserErrorReport { errors: vec![] };
  match parse_mech(ParseString::new(&graphemes)) {
    // Got a parse tree, however there may be errors
    Ok((mut remaining_input, parse_tree)) => {
      error_log.errors.append(&mut remaining_input.error_log);
      result_node = parse_tree;
    },
    // Parsing completely failed, and no parse tree was created
    Err(err) => match err {
      Err::Error(mut e) | Err::Failure(mut e) => {
        error_log.errors.append(&mut e.remaining_input.error_log);
        error_log.errors.push((e.cause_range, e.error_detail));
      },
      Err::Incomplete(_) => panic!("nom::Err::Incomplete is not supported!"),
    },
  }
  
  // +-------------------+
  //   temporary
  // +-------------------+
  if error_log.has_error() {
    Err(MechError{id: 3202, kind: MechErrorKind::GenericError(error_log.construct_msg(text))})
  } else {
    Ok(result_node)
  }
  // +-------------------+
}

pub fn parse_fragment(text: &str) -> Result<ParserNode,MechError> {
  let graphemes = UnicodeSegmentation::graphemes(text, true).collect::<Vec<&str>>();
  let mut result_node = ParserNode::Error;
  let mut error_log = ParserErrorReport { errors: vec![] };
  match parse_mech_fragment(ParseString::new(&graphemes)) {
    // Got a parse tree, however there may be errors
    Ok((mut remaining_input, parse_tree)) => {
      error_log.errors.append(&mut remaining_input.error_log);
      result_node = parse_tree;
    },
    // Parsing completely failed, and no parse tree was created
    Err(err) => match err {
      Err::Error(mut e) | Err::Failure(mut e) => {
        error_log.errors.append(&mut e.remaining_input.error_log);
        error_log.errors.push((e.cause_range, e.error_detail));
      },
      Err::Incomplete(_) => panic!("nom::Err::Incomplete is not supported!"),
    },
  }
  
  // +-------------------+
  //   temporary
  // +-------------------+
  if error_log.has_error() {
    Err(MechError{id: 3202, kind: MechErrorKind::GenericError(error_log.construct_msg(text))})
  } else {
    Ok(result_node)
  }
  // +-------------------+
}

