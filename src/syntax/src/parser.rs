// # Parser

/// Sections:
///   1. Prelude
///   2. Parser utilities
///   3. Parser combinators
///   4. Recovery functions
///   5. Primitive parsers
///   6. Parsers
///   7. Reporting errors
///   8. Public interface

// ## Prelude

use mech_core::*;
use mech_core::nodes::*;

#[cfg(not(feature = "no-std"))] use core::fmt;
#[cfg(feature = "no-std")] use alloc::fmt;
#[cfg(feature = "no-std")] use alloc::string::String;
#[cfg(feature = "no-std")] use alloc::vec::Vec;
use nom::{
  IResult,
  branch::alt,
  sequence::tuple,
  combinator::{opt, eof},
  multi::{many1, many_till, many0, separated_list1},
  Err,
};
use nom::character::is_newline;

use std::collections::HashMap;
use colored::*;
use crate::*;

// ## Parser combinators

/// Convert output of any parser into ParserNode::Null.
/// Useful for working with `alt` combinator and error recovery functions.
fn null<'a, F, O>(mut parser: F) ->
  impl FnMut(ParseString<'a>) -> ParseResult<ParserNode>
where
  F: FnMut(ParseString<'a>) -> ParseResult<O>
{
  move |input: ParseString| match parser(input) {
    Ok((remaining, _)) => Ok((remaining, ParserNode::Null)),
    Err(Err::Error(e)) => Err(Err::Error(e)),
    Err(Err::Failure(e)) => Err(Err::Failure(e)),
    x => panic!("Err::Incomplete is not supported"),
  }
}

/// For parser p, run p and also output the range that p has matched
/// upon success.
fn range<'a, F, O>(mut parser: F) ->
  impl FnMut(ParseString<'a>) -> ParseResult<(O, SourceRange)>
where
  F: FnMut(ParseString<'a>) -> ParseResult<O>
{
  move |input: ParseString| {
    let start = input.loc();
    match parser(input) {
      Ok((remaining, o)) => {
        let rng = SourceRange { start, end: remaining.loc(), };
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

/// Label without recovery function. Upgrade Err::Error to Err:Failure
/// and override its context information.
fn label_without_recovery<'a, F, O>(
  mut parser: F,
  error_detail: ParseErrorDetail,
) ->
  impl FnMut(ParseString<'a>) -> ParseResult<O>
where
  F: FnMut(ParseString<'a>) -> ParseResult<O>
{
  move |mut input: ParseString| {
    let start = input.loc();
    match parser(input) {
      Err(Err::Error(mut e)) => {
        e.cause_range = SourceRange { start, end: e.cause_range.end };
        e.error_detail = error_detail.clone();
        Err(Err::Failure(e))
      }
      x => x,
    }
  }
}

/// Label with recovery function. In addition to upgrading errors, the
/// error is logged and recovery function will be run as an attempt to
/// synchronize parser state.
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
    let start = input.loc();
    match parser(input) {
      Err(Err::Error(mut e)) => {
        e.cause_range = SourceRange { start, end: e.cause_range.end };
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

/// For parser p, return the `!!p` peek parsing expression.
fn is<'a, F, O>(mut parser: F) ->
  impl FnMut(ParseString<'a>) -> ParseResult<O>
where
  F: FnMut(ParseString<'a>) -> ParseResult<O>
{
  move |input: ParseString| {
    let input_clone = input.clone();
    match parser(input_clone) {
      Ok((_, o)) => Ok((input, o)),
      _ => Err(Err::Error(ParseError::new(input, "Unexpected character"))),
    }
  }
}

/// For parser p, return the `!p` peek parsing expression.
fn is_not<'a, F, E>(mut parser: F) ->
  impl FnMut(ParseString<'a>) -> ParseResult<()>
where
  F: FnMut(ParseString<'a>) -> ParseResult<E>
{
  move |input: ParseString| {
    let input_clone = input.clone();
    match parser(input_clone) {
      Err(Err::Failure(_)) |
      Err(Err::Error(_)) => Ok((input, ())),
      _ => Err(Err::Error(ParseError::new(input, "Unexpected character")))
    }
  }
}

/// Return a terminal parsing expression that consumes `tag` from input.
pub fn tag(tag: &'static str) -> impl Fn(ParseString) -> ParseResult<String> {
  move |mut input: ParseString| {
    if let Some(matched) = input.consume_tag(tag) {
      Ok((input, matched))
    } else {
      Err(nom::Err::Error(ParseError::new(input, "Unexpected character")))
    }
  }
}

// ## Recovery functions

pub fn skip_till_eol(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = many0(tuple((
    is_not(newline),
    any,
  )))(input)?;
  Ok((input, ParserNode::Null))
}

fn skip_pass_eol(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = skip_till_eol(input)?;
  let (input, _) = newline(input)?;
  Ok((input, ParserNode::Null))
}

fn skip_till_section_element(input: ParseString) -> ParseResult<ParserNode> {
  if input.len() == 0 {
    return Ok((input, ParserNode::Error));
  }
  let (input, _) = skip_pass_eol(input)?;
  let (input, _) = many0(tuple((
    is_not(section_element),
    skip_pass_eol,
  )))(input)?;
  Ok((input, ParserNode::Error))
}

fn skip_till_section_element2(input: ParseString) -> ParseResult<ParserNode> {
  if input.len() == 0 {
    return Ok((input, ParserNode::Error));
  }
  let (input, _) = skip_pass_eol(input)?;
  let (input, _) = many0(tuple((
    is_not(section_element2),
    skip_pass_eol,
  )))(input)?;
  Ok((input, ParserNode::Error))
}

fn skip_till_section_element3(input: ParseString) -> ParseResult<ParserNode> {
  if input.len() == 0 {
    return Ok((input, ParserNode::Error));
  }
  let (input, _) = skip_pass_eol(input)?;
  let (input, _) = many0(tuple((
    is_not(section_element3),
    skip_pass_eol,
  )))(input)?;
  Ok((input, ParserNode::Error))
}

pub fn skip_spaces(input: ParseString) -> ParseResult<()> {
  let (input, _) = many0(space)(input)?;
  Ok((input, ()))
}

pub fn skip_nil(input: ParseString) -> ParseResult<ParserNode> {
  Ok((input, ParserNode::Error))
}

pub fn skip_empty_mech_directive(input: ParseString) -> ParseResult<String> {
  Ok((input, String::from("mech:")))
}

// ## Primitive parsers

pub fn emoji_grapheme(mut input: ParseString) -> ParseResult<String> {
  if let Some(matched) = input.consume_emoji() {
    Ok((input, matched))
  } else {
    Err(nom::Err::Error(ParseError::new(input, "Unexpected character")))
  }
}

pub fn alpha(mut input: ParseString) -> ParseResult<String> {
  if let Some(matched) = input.consume_alpha() {
    Ok((input, matched))
  } else {
    Err(nom::Err::Error(ParseError::new(input, "Unexpected character")))
  }
}

pub fn digit(mut input: ParseString) -> ParseResult<String> {
  if let Some(matched) = input.consume_digit() {
    Ok((input, matched))
  } else {
    Err(nom::Err::Error(ParseError::new(input, "Unexpected character")))
  }
}

pub fn any(mut input: ParseString) -> ParseResult<String> {
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
      let start = input.loc();
      let (input, _) = tag($byte)(input)?;
      let end = input.loc();
      let src_range = SourceRange { start, end };
      Ok((input, ParserNode::Token{token: $token, chars: $byte.chars().collect::<Vec<char>>(), src_range}))
    }
  )
}

leaf!{at, "@", TokenKind::At}
leaf!{hashtag, "#", TokenKind::HashTag}
leaf!{period, ".", TokenKind::Period}
leaf!{colon, ":", TokenKind::Colon}
leaf!{comma, ",", TokenKind::Comma}
leaf!{percent, "%", TokenKind::Percent}
leaf!{apostrophe, "'", TokenKind::Apostrophe}
leaf!{left_bracket, "[", TokenKind::LeftBracket}
leaf!{right_bracket, "]", TokenKind::RightBracket}
leaf!{left_parenthesis, "(", TokenKind::LeftParenthesis}
leaf!{right_parenthesis, ")", TokenKind::RightParenthesis}
leaf!{left_brace, "{", TokenKind::LeftBrace}
leaf!{right_brace, "}", TokenKind::RightBrace}
leaf!{equal, "=", TokenKind::Equal}
leaf!{left_angle, "<", TokenKind::LeftAngle}
leaf!{right_angle, ">", TokenKind::RightAngle}
leaf!{exclamation, "!", TokenKind::Exclamation}
leaf!{question, "?", TokenKind::Question}
leaf!{plus, "+", TokenKind::Plus}
leaf!{dash, "-", TokenKind::Dash}
leaf!{underscore, "_", TokenKind::Underscore}
leaf!{asterisk, "*", TokenKind::Asterisk}
leaf!{slash, "/", TokenKind::Slash}
leaf!{backslash, "\\", TokenKind::Backslash}
leaf!{caret, "^", TokenKind::Caret}
leaf!{space, " ", TokenKind::Space}
leaf!{tab, "\t", TokenKind::Tab}
leaf!{tilde, "~", TokenKind::Tilde}
leaf!{grave, "`", TokenKind::Grave}
leaf!{bar, "|", TokenKind::Bar}
leaf!{quote, "\"", TokenKind::Quote}
leaf!{ampersand, "&", TokenKind::Ampersand}
leaf!{semicolon, ";", TokenKind::Semicolon}
leaf!{new_line_char, "\n", TokenKind::Newline}
leaf!{carriage_return, "\r", TokenKind::CarriageReturn}

// emoji ::= emoji_grapheme+ ;
fn emoji<'a>(input: ParseString<'a>) -> ParseResult<ParserNode> {
  let emoji_token = |input: ParseString<'a>| {
    let start = input.loc();
    let (input, g) = emoji_grapheme(input)?;
    let end = input.loc();
    let src_range = SourceRange { start, end };
    Ok((input, ParserNode::Token{token: TokenKind::Emoji, chars: g.chars().collect::<Vec<char>>(), src_range}))
  };
  let (input, tokens) = many1(emoji_token)(input)?;
  // let chars: Vec<ParserNode> = matching.iter().map(|b| ParserNode::Token{token: TokenKind::Emoji, chars: b.chars().collect::<Vec<char>>()}).collect();
  Ok((input, ParserNode::Emoji{children: tokens}))
}

// word ::= alpha+ ;
pub fn word<'a>(input: ParseString<'a>) -> ParseResult<ParserNode> {
  let alpha_token = |input: ParseString<'a>| {
    let (input, (g, src_range)) = range(alpha)(input)?;
    Ok((input, ParserNode::Token{token: TokenKind::Alpha, chars: g.chars().collect::<Vec<char>>(), src_range}))
  };
  let (input, tokens) = many1(alpha_token)(input)?;
  // let chars: Vec<ParserNode> = matching.iter().map(|b| ParserNode::Token{token: TokenKind::Alpha, chars: b.chars().collect::<Vec<char>>()}).collect();
  Ok((input, ParserNode::Word{children: tokens}))
}

// digit1 ::= digit+ ;
pub fn digit1(input: ParseString) -> ParseResult<Vec<String>> {
  let result = many1(digit)(input)?;
  Ok(result)
}

// digit0 ::= digit* ;
pub fn digit0(input: ParseString) -> ParseResult<Vec<String>> {
  let result = many0(digit)(input)?;
  Ok(result)
}

// bin_digit ::= "0" | "1" ;
pub fn bin_digit(input: ParseString) -> ParseResult<String> {
  let result = alt((tag("1"),tag("0")))(input)?;
  Ok(result)
}

// hex_digit ::= digit | "a" | "b" | "c" | "d" | "e" | "f" | "A" | "B" | "C" | "D" | "E" | "F" ;
pub fn hex_digit(input: ParseString) -> ParseResult<String> {
  let result = alt((digit, tag("a"), tag("b"), tag("c"), tag("d"), tag("e"), tag("f"), 
                           tag("A"), tag("B"), tag("C"), tag("D"), tag("E"), tag("F")))(input)?;
  Ok(result)
}

// oct_digit ::= "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" ;
pub fn oct_digit(input: ParseString) -> ParseResult<String> {
  let result = alt((tag("0"),tag("1"),tag("2"),tag("3"),tag("4"),tag("5"),tag("6"),tag("7")))(input)?;
  Ok(result)
}

// number ::= digit1 ;
pub fn number<'a>(input: ParseString<'a>) -> ParseResult<ParserNode> {
  let digit_token = |input: ParseString<'a>| {
    let (input, (g, src_range)) = range(digit)(input)?;
    Ok((input, ParserNode::Token{token: TokenKind::Digit, chars: g.chars().collect::<Vec<char>>(), src_range}))
  };
  let (input, tokens) = many1(digit_token)(input)?;
  // let chars: Vec<ParserNode> = matching.iter().map(|b| ParserNode::Token{token: TokenKind::Digit, chars: b.chars().collect::<Vec<char>>()}).collect();
  Ok((input, ParserNode::Number{children: tokens}))
}

// punctuation ::= period | exclamation | question | comma | colon | semicolon | dash | apostrophe | left_parenthesis | right_parenthesis | left_angle | right_angle | left_brace | right_brace | left_bracket | right_bracket ;
pub fn punctuation(input: ParseString) -> ParseResult<ParserNode> {
  let (input, punctuation) = alt((period, exclamation, question, comma, colon, semicolon, dash, apostrophe, left_parenthesis, right_parenthesis, left_angle, right_angle, left_brace, right_brace, left_bracket, right_bracket))(input)?;
  Ok((input, ParserNode::Punctuation{children: vec![punctuation]}))
}

// symbol ::= ampersand | bar | at | slash | backslash | hashtag | equal | tilde | plus | asterisk | asterisk | caret | underscore ;
pub fn symbol(input: ParseString) -> ParseResult<ParserNode> {
  let (input, symbol) = alt((ampersand, bar, at, slash, backslash, hashtag, equal, tilde, plus, asterisk, caret, underscore))(input)?;
  Ok((input, ParserNode::Symbol{children: vec![symbol]}))
}

// paragraph_symbol ::= ampersand | at | slash | backslash | asterisk | caret | hashtag | underscore ;
pub fn paragraph_symbol(input: ParseString) -> ParseResult<ParserNode> {
  let (input, symbol) = alt((ampersand, at, slash, backslash, asterisk, caret, hashtag, underscore, equal, tilde, plus, percent))(input)?;
  Ok((input, ParserNode::Symbol{children: vec![symbol]}))
}

// text ::= (word | space | number | punctuation | symbol | emoji)+ ;
pub fn text(input: ParseString) -> ParseResult<ParserNode> {
  let (input, word) = many1(alt((word, space, number, punctuation, symbol, emoji)))(input)?;
  Ok((input, ParserNode::Text{children: word}))
}

// paragraph_rest ::= (word | space | number | punctuation | paragraph_symbol | quote | emoij)+ ;
pub fn paragraph_rest(input: ParseString) -> ParseResult<ParserNode> {
  let (input, word) = many1(alt((word, space, number, punctuation, paragraph_symbol, quote, emoji)))(input)?;
  Ok((input, ParserNode::Text{children: word}))
}

// paragraph_starter ::= (word | number | quote | left_angle | right_angle | left_bracket | right_bracket | period | exclamation | question | comma | colon | semicolon | left_parenthesis | right_parenthesis | emoji)+ ;
pub fn paragraph_starter(input: ParseString) -> ParseResult<ParserNode> {
  let (input, word) = many1(alt((word, number, quote, left_angle, right_angle, left_bracket, right_bracket, period, exclamation, question, comma, colon, semicolon, right_parenthesis, emoji)))(input)?;
  Ok((input, ParserNode::Text{children: word}))
}

// identifier ::= (word | emoji), (word | number | dash | slash | emoji)* ;
pub fn identifier(input: ParseString) -> ParseResult<ParserNode> {
  let (input, (word, mut rest)) = tuple((alt((word,emoji)), many0(alt((word, number, dash, slash, emoji)))))(input)?;
  let mut id = vec![word];
  id.append(&mut rest);
  Ok((input, ParserNode::Identifier{children: id}))
}

// boolean_literal ::= true_literal | false_literal ;
pub fn boolean_literal(input: ParseString) -> ParseResult<ParserNode> {
  let (input, boolean) = alt((true_literal, false_literal))(input)?;
  Ok((input, ParserNode::BooleanLiteral{children: vec![boolean]}))
}

// true_literal ::= english_true_literal | true_symbol ;
pub fn true_literal(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = alt((english_true_literal, true_symbol))(input)?;
  Ok((input, ParserNode::True))
}

// false_literal ::= english_false_literal | false_symbol ;
pub fn false_literal(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = alt((english_false_literal, false_symbol))(input)?;
  Ok((input, ParserNode::False))
}

// true_symbol ::= "✓" ;
pub fn true_symbol(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag("✓")(input)?;
  Ok((input, ParserNode::False))
}

// false_symbol ::= "✗" ;
pub fn false_symbol(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag("✗")(input)?;
  Ok((input, ParserNode::False))
}

// english_true_literal ::= "true" ;
pub fn english_true_literal(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag("true")(input)?;
  Ok((input, ParserNode::True))
}

// english_false_literal ::= "false" ;
pub fn english_false_literal(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag("false")(input)?;
  Ok((input, ParserNode::False))
}

// carriage_newline ::= "\r\n" ;
pub fn carriage_newline(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag("\r\n")(input)?;
  Ok((input, ParserNode::Null))
}

// newline ::= new_line_char | carriage_newline ;
pub fn newline(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = alt((new_line_char, carriage_newline))(input)?;
  Ok((input, ParserNode::Null))
}

// whitespace ::= space*, newline+ ;
pub fn whitespace(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = many0(space)(input)?;
  let (input, _) = many1(newline)(input)?;
  Ok((input, ParserNode::Null))
}

// number_literal ::= (hexadecimal_literal | octal_literal | binary_literal | decimal_literal | float_literal), kind_annotation? ;
pub fn number_literal(input: ParseString) -> ParseResult<ParserNode> {
  let (input, number_variant) = alt((hexadecimal_literal, octal_literal, binary_literal, decimal_literal, float_literal))(input)?;
  let (input, kind_id) = opt(kind_annotation)(input)?;
  let mut children = vec![number_variant];
  match kind_id {
    Some(kind_id) => children.push(kind_id),
    _ => (),
  }
  Ok((input, ParserNode::NumberLiteral{children}))
}

// pub fn rational_number(input: ParseString) -> IResult<ParseString, ParserNode> {
//   let (input, numerator) = alt((quantity, number_literal))(input)?;
//   let (input, _) = tag("/")(input)?;
//   let (input, denominator) = alt((quantity, number_literal))(input)?;
//   Ok((input, ParserNode::Null))
// }

// float_literal ::= "."?, digit1, "."?, digit0 ;
pub fn float_literal(input: ParseString) -> ParseResult<ParserNode> {
  let start = input.loc();
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
  let end = input.loc();
  let src_range = SourceRange { start, end };
  Ok((input, ParserNode::FloatLiteral{chars: whole, src_range}))
}

// decimal_literal ::= "0d", <digit1> ;
pub fn decimal_literal(input: ParseString) -> ParseResult<ParserNode> {
  let msg = "Expect decimal digits after \"0d\"";
  let start = input.loc();
  let (input, _) = tag("0d")(input)?;
  let (input, chars) = label!(digit1, msg)(input)?;
  let end = input.loc();
  let src_range = SourceRange { start, end };
  Ok((input, ParserNode::DecimalLiteral{chars: chars.iter().flat_map(|c| c.chars()).collect(), src_range}))
}

// hexadecimal_literal ::= "0x", <hex_digit+> ;
pub fn hexadecimal_literal(input: ParseString) -> ParseResult<ParserNode> {
  let msg = "Expect hexadecimal digits after \"0x\"";
  let start = input.loc();
  let (input, _) = tag("0x")(input)?;
  let (input, chars) = label!(many1(hex_digit), msg)(input)?;
  let end = input.loc();
  let src_range = SourceRange { start, end };
  Ok((input, ParserNode::HexadecimalLiteral{chars: chars.iter().flat_map(|c| c.chars()).collect(), src_range}))
}

// octal_literal ::= "0o", <oct_digit+> ;
pub fn octal_literal(input: ParseString) -> ParseResult<ParserNode> {
  let msg = "Expect octal digits after \"0o\"";
  let start = input.loc();
  let (input, _) = tag("0o")(input)?;
  let (input, chars) = label!(many1(oct_digit), msg)(input)?;
  let end = input.loc();
  let src_range = SourceRange { start, end };
  Ok((input, ParserNode::OctalLiteral{chars: chars.iter().flat_map(|c| c.chars()).collect(), src_range}))
}

// binary_literal ::= "0b", <bin_digit+> ;
pub fn binary_literal(input: ParseString) -> ParseResult<ParserNode> {
  let msg = "Expect binary digits after \"0b\"";
  let start = input.loc();
  let (input, _) = tag("0b")(input)?;
  let (input, chars) = label!(many1(bin_digit), msg)(input)?;
  let end = input.loc();
  let src_range = SourceRange { start, end };
  Ok((input, ParserNode::BinaryLiteral{chars: chars.iter().flat_map(|c| c.chars()).collect(), src_range}))
}

// value ::= empty | boolean_literal | number_literal | string ;
pub fn value(input: ParseString) -> ParseResult<ParserNode> {
  let (input, value) = alt((empty, boolean_literal, number_literal, string))(input)?;
  Ok((input, ParserNode::Value{children: vec![value]}))
}

// empty ::= underscore+ ;
pub fn empty(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = many1(underscore)(input)?;
  Ok((input, ParserNode::Empty))
}

// #### Enums

// enum_define ::= "<", identifier, ">", space*, "=", space*, enum_list;
pub fn enum_define(input: ParseString) -> ParseResult<ParserNode> {
  let msg2 = "Expect expression";
  let (input, _) = left_angle(input)?;
  let (input, variable) = identifier(input)?;
  let (input, _) = right_angle(input)?;
  let (input, _) = many1(space)(input)?;
  let (input, _) = equal(input)?;
  let (input, _) = many1(space)(input)?;
  let (input, expression) = label!(expression, msg2)(input)?;
  Ok((input, ParserNode::EnumDefine{children: vec![variable]}))
}

// ### Blocks

// #### Data

// select_all ::= colon ;
pub fn select_all(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = colon(input)?;
  Ok((input, ParserNode::SelectAll))
}

// subscript ::= (select_all | expression | tilde), space*, comma?, space* ;
pub fn subscript(input: ParseString) -> ParseResult<ParserNode> {
  let (input, subscript) = alt((select_all, expression, tilde))(input)?;
  let (input, _) = tuple((many0(space), opt(comma), many0(space)))(input)?;
  Ok((input, ParserNode::Subscript{children: vec![subscript]}))
}

// subscript_index ::= left_brace, <subscript+>, <right_brace> ;
pub fn subscript_index(input: ParseString) -> ParseResult<ParserNode> {
  let msg1 = "Expect subscript";
  let msg2 = "Expect right brace '}'";
  let (input, (_, r)) = range(left_brace)(input)?;
  let (input, subscripts) = label!(many1(subscript), msg1)(input)?;
  let (input, _) = label!(right_brace, msg2, r)(input)?;
  Ok((input, ParserNode::SubscriptIndex{children: subscripts}))
}

// single_subscript_index ::= left_brace, <subscript>, <right_brace> ;
pub fn single_subscript_index(input: ParseString) -> ParseResult<ParserNode> {
  let msg1 = "Expect subscript";
  let msg2 = "Expect right brace '}'";
  let (input, _) = left_brace(input)?;
  let (input, subscript) = label!(subscript, msg1)(input)?;
  let (input, _) = label!(right_brace, msg2)(input)?;
  Ok((input, ParserNode::SubscriptIndex{children: vec![subscript]}))
}

// dot_index ::= period, <identifier>, single_subscript_index? ;
pub fn dot_index(input: ParseString) -> ParseResult<ParserNode> {
  let msg = "Expect identifier";
  let (input, _) = period(input)?;
  let (input, identifier) = label!(identifier, msg)(input)?;
  let (input, subscript) = opt(single_subscript_index)(input)?;
  let index = match subscript {
    Some(subscript) => vec![subscript, identifier],
    None => vec![ParserNode::Null, identifier],
  };
  Ok((input, ParserNode::DotIndex{children: index}))
}

// swizzle ::= period, identifier, comma, !space, <identifier, (",", identifier)*> ;
pub fn swizzle(input: ParseString) -> ParseResult<ParserNode> {
  let msg = "Expect identifier for swizzling";
  let (input, _) = period(input)?;
  let (input, first) = identifier(input)?;
  let (input, _) = comma(input)?;
  let (input, _) = is_not(space)(input)?;  // so that it's not a binding
  let (input, mut rest) = label!(separated_list1(tag(","), identifier), msg)(input)?;
  let mut cols = vec![first];
  cols.append(&mut rest);
  Ok((input, ParserNode::Swizzle{children: cols}))
}

// reshape_column ::= left_brace, colon, right_brace ;
pub fn reshape_column(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = left_brace(input)?;
  let (input, _) = colon(input)?;
  let (input, _) = right_brace(input)?;
  Ok((input, ParserNode::ReshapeColumn))
}

// index ::= swizzle | dot_index | reshape_column | subscript_index ;
pub fn index(input: ParseString) -> ParseResult<ParserNode> {
  let (input, index) = alt((swizzle, dot_index, reshape_column, subscript_index))(input)?;
  Ok((input, ParserNode::Index{children: vec![index]}))
}

// data ::= (table | identifier), index*, transpose? ;
pub fn data(input: ParseString) -> ParseResult<ParserNode> {
  let (input, source) = alt((table, identifier))(input)?;
  let (input, mut indices) = many0(index)(input)?;
  let (input, transpose) = opt(transpose)(input)?;
  let mut data = vec![source];
  data.append(&mut indices);
  match transpose {
    Some(transpose) => {
      data.push(transpose);
    }
    _ => (),
  }
  Ok((input, ParserNode::Data{children: data}))
}

// kind_annotation ::= left_angle, <(identifier | underscore), (",", (identifier | underscore))*>, <right_angle> ;
pub fn kind_annotation(input: ParseString) -> ParseResult<ParserNode> {
  let msg2 = "Expect at least one unit in kind annotation";
  let msg3 = "Expect right angle";
  let (input, (_, r)) = range(left_angle)(input)?;
  let (input, kind_id) = label!(separated_list1(tag(","), alt((identifier, underscore))), msg2)(input)?;
  let (input, _) = label!(right_angle, msg3, r)(input)?;
  Ok((input, ParserNode::KindAnnotation{children: kind_id}))
}

// #### Tables

// table ::= hashtag, <identifier> ;
pub fn table(input: ParseString) -> ParseResult<ParserNode> {
  let msg = "Expect identifier after hashtag";
  let (input, _) = hashtag(input)?;
  let (input, table_identifier) = label!(identifier, msg)(input)?;
  Ok((input, ParserNode::Table{children: vec![table_identifier]}))
}

// binding ::= s*, identifier, kind_annotation?, <!(space+, colon)>, colon, s+,
// >>          <empty | expression | identifier | value>, <!!right_bracket | (s*, comma, <s+>) | s+> ;
// >> where s ::= space | newline | tab ;
pub fn binding(input: ParseString) -> ParseResult<ParserNode> {
  let msg1 = "Unexpected space before colon ':'";
  let msg2 = "Expect a value";
  let msg3 = "Expect whitespace or comma followed by whitespace";
  let msg4 = "Expect whitespace";
  let mut children = vec![];
  let (input, _) = many0(alt((space, newline, tab)))(input)?;
  let (input, binding_id) = identifier(input)?;
  let (input, kind) = opt(kind_annotation)(input)?;
  let (input, _) = label!(is_not(tuple((many1(space), colon))), msg1)(input)?;
  let (input, _) = colon(input)?;
  let (input, _) = many1(alt((space, newline, tab)))(input)?;
  let (input, bound) = label!(alt((empty, expression, identifier, value)), msg2)(input)?;
  let (input, _) = label!(alt((
    is(right_bracket),
    null(tuple((
      many0(alt((space, newline, tab))),
      comma,
      label!(many1(alt((space, newline, tab))), msg4),
    ))),
    null(many1(alt((space, newline, tab)))),
  )), msg3)(input)?;
  children.push(binding_id);
  children.push(bound);
  if let Some(kind) = kind { children.push(kind); }
  Ok((input, ParserNode::Binding{children}))
}

// binding_strict ::= s*, identifier, kind_annotation?, <!(space+, colon)>, colon, <s+>,
// >>                 <empty | expression | identifier | value>, <!!right_bracket | (s*, comma, <s+>) | s+> ;
// >> where s ::= space | newline | tab ;
pub fn binding_strict(input: ParseString) -> ParseResult<ParserNode> {
  let msg1 = "Unexpected space before colon ':' for binding";
  let msg2 = "Expect space after ':' for binding";
  let msg3 = "Expect a value";
  let msg4 = "Expect whitespace or comma followed by whitespace";
  let msg5 = "Expect whitespace";
  let mut children = vec![];
  let (input, _) = many0(alt((space, newline, tab)))(input)?;
  let (input, binding_id) = identifier(input)?;
  let (input, _) = label!(is_not(tuple((many1(space), colon))), msg1)(input)?;
  let (input, kind) = opt(kind_annotation)(input)?;
  let (input, _) = label!(is_not(tuple((many1(space), colon))), msg1)(input)?;
  let (input, _) = colon(input)?;
  let (input, _) = label!(many1(alt((space, newline, tab))), msg2)(input)?;
  let (input, bound) = label!(alt((empty, expression, identifier, value)), msg3)(input)?;
  let (input, _) = label!(alt((
    is(right_bracket),
    null(tuple((
      many0(alt((space, newline, tab))),
      comma,
      label!(many1(alt((space, newline, tab))), msg5),
    ))),
    null(many1(alt((space, newline, tab)))),
  )), msg4)(input)?;
  children.push(binding_id);
  children.push(bound);
  if let Some(kind) = kind { children.push(kind); }
  Ok((input, ParserNode::Binding{children}))
}

// function_binding ::= identifier, <colon>, <space+>, <expression | identifier | value>, space*, comma?, space* ;
pub fn function_binding(input: ParseString) -> ParseResult<ParserNode> {
  let msg1 = "Expect colon ':' for function binding";
  let msg2 = "Expect space after colon for function binding";
  let msg3 = "Expect expression, identifier, or value to bind";
  let (input, (binding_id, r)) = range(identifier)(input)?;
  let (input, _) = label!(colon, msg1)(input)?;
  let (input, _) = label!(many1(space), msg2)(input)?;
  let (input, bound) = label!(alt((expression, identifier, value)), msg3, r)(input)?;
  let (input, _) = tuple((many0(space), opt(comma), many0(space)))(input)?;
  Ok((input, ParserNode::FunctionBinding{children: vec![binding_id, bound]}))
}

// table_column ::= (space | tab)*, (expression | value | data), comma?, (space | tab)* ;
pub fn table_column(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = many0(alt((space, tab)))(input)?;
  let (input, item) = alt((expression, value, data))(input)?;
  let (input, _) = tuple((opt(comma), many0(alt((space, tab)))))(input)?;
  Ok((input, ParserNode::Column{children: vec![item]}))
}

// table_row ::= (space | tab)*, table_column+, semicolon?, newline? ;
pub fn table_row(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = many0(alt((space, tab)))(input)?;
  let (input, columns) = many1(table_column)(input)?;
  let (input, _) = tuple((opt(semicolon), opt(newline)))(input)?;
  Ok((input, ParserNode::TableRow{children: columns}))
}

// attribute ::= identifier, kind_annotation?, space*, comma?, space* ;
pub fn attribute(input: ParseString) -> ParseResult<ParserNode> {
  let mut children = vec![];
  let (input, identifier) = identifier(input)?;
  children.push(identifier);
  let (input, kind) = opt(kind_annotation)(input)?;
  let (input, _) = tuple((many0(space), opt(comma), many0(space)))(input)?;
  if let Some(kind) = kind { children.push(kind); }
  Ok((input, ParserNode::Attribute{children}))
}

// table_header ::= bar, <attribute+>, <bar>, space*, newline? ;
pub fn table_header(input: ParseString) -> ParseResult<ParserNode> {
  let msg1 = "Expect at least one attribute for table header";
  let msg2 = "Expect vertical bar to terminate table header";
  let (input, (_, r)) = range(bar)(input)?;
  let (input, attributes) = label!(many1(attribute), msg1)(input)?;
  let (input, _) = tuple((label!(bar, msg2, r), many0(space), opt(newline)))(input)?;
  Ok((input, ParserNode::TableHeader{children: attributes}))
}

// anonymous_table ::= left_bracket, (space | newline | tab)*, table_header?,
// >>                  ((comment, newline) | table_row)*, (space | newline | tab)*, <right_bracket> ;
pub fn anonymous_table(input: ParseString) -> ParseResult<ParserNode> {
  let msg = "Expect right bracket ']' to finish the table";
  let (input, (_, r)) = range(left_bracket)(input)?;
  let (input, _) = many0(alt((space, newline, tab)))(input)?;
  let (mut input, table_header) = opt(table_header)(input)?;
  let mut table_rows = vec![];
  loop {
    let (i, mut rows) = many0(table_row)(input)?;
    let (i, comments) = many0(tuple((comment, newline)))(i)?;
    table_rows.append(&mut rows);
    input = i;
    if comments.is_empty() {
      break;
    }
  }
  let (input, _) = many0(alt((space, newline, tab)))(input)?;
  let (input, _) = label!(right_bracket, msg, r)(input)?;
  let mut table = vec![];
  match table_header {
    Some(table_header) => table.push(table_header),
    _ => (),
  };
  table.append(&mut table_rows);
  Ok((input, ParserNode::AnonymousTable{children: table}))
}

// empty_table ::= left_bracket, (space | newline | tab)*, table_header?, (space | newline | tab)*, right_bracket ;
pub fn empty_table(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = left_bracket(input)?;
  let (input, _) = many0(alt((space, newline, tab)))(input)?;
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

// inline_table ::= left_bracket, binding, <binding_strict*>, <right_bracket> ;
pub fn inline_table(input: ParseString) -> ParseResult<ParserNode> {
  let msg = "Expect right bracket ']' to terminate inline table";
  let (input, (_, r)) = range(left_bracket)(input)?;
  let (input, first_binding) = binding(input)?;
  let (input, mut other_bindings) = many0(binding_strict)(input)?;
  let (input, _) = label!(right_bracket, msg, r)(input)?;
  let mut bindings = vec![first_binding];
  bindings.append(&mut other_bindings);
  Ok((input, ParserNode::InlineTable{children: bindings}))
}

// #### Statements

// stmt_operator ::= split_operator | flatten_operator | set_operator | update_operator | add_row_operator | equal ;
pub fn stmt_operator(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = alt((split_operator, flatten_operator, set_operator, update_operator, add_row_operator, equal, async_assign_operator))(input)?;
  Ok((input, ParserNode::Null))
}

// comment_sigil ::= "--" ;
pub fn comment_sigil(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag("--")(input)?;
  Ok((input, ParserNode::Null))
}

// comment ::= (space | tab)*, comment_sigil, <text>, <!!newline> ;
pub fn comment(input: ParseString) -> ParseResult<ParserNode> {
  let msg1 = "Expect comment text";
  let msg2 = "Character not allowed in comment text";
  let (input, _) = many0(alt((space, tab)))(input)?;
  let (input, _) = comment_sigil(input)?;
  let (input, comment) = labelr!(text, skip_nil, msg1)(input)?;
  let (input, _) = labelr!(is(newline), skip_till_eol, msg2)(input)?;
  Ok((input, ParserNode::Comment{children: vec![comment]}))
}

// add_row_operator ::= "+=" ;
pub fn add_row_operator(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag("+=")(input)?;
  Ok((input, ParserNode::Null))
}

// async_assign_operator ::= "<~" ;
pub fn async_assign_operator(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag("<~")(input)?;
  Ok((input, ParserNode::Null))
}

// add_row ::= table, <!stmt_operator>, space*, add_row_operator, <space+>, <expression | inline_table | anonymous_table> ;
pub fn add_row(input: ParseString) -> ParseResult<ParserNode> {
  let msg1 = "Expect spaces around operator";
  let msg2 = "Expect expression, inline table, or anonymous table";
  let (input, table_id) = table(input)?;
  let (input, _) = labelr!(null(is_not(stmt_operator)), skip_nil, msg1)(input)?;
  let (input, _) = many0(space)(input)?;
  let (input, _) = add_row_operator(input)?;
  let (input, _) = labelr!(null(many1(space)), skip_nil, msg1)(input)?;
  let (input, table) = label!(alt((expression, inline_table, anonymous_table)), msg2)(input)?;
  Ok((input, ParserNode::AddRow{children: vec![table_id, table]}))
}

// add_update_operator ::= ":+=" ;
pub fn add_update_operator(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag(":+=")(input)?;
  Ok((input, ParserNode::AddUpdate))
}

// subtract_update_operator ::= ":-=" ;
pub fn subtract_update_operator(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag(":-=")(input)?;
  Ok((input, ParserNode::SubtractUpdate))
}

// multiply_update_operator ::= ":*=" ;
pub fn multiply_update_operator(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag(":*=")(input)?;
  Ok((input, ParserNode::MultiplyUpdate))
}

// divide_update_operator ::= ":/=" ;
pub fn divide_update_operator(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag(":/=")(input)?;
  Ok((input, ParserNode::DivideUpdate))
}

// pub fn update_exponent_operator(input: ParseString) -> ParseResult<ParserNode> {
//   let (input, _) = tag(":^=")(input)?;
//   Ok((input, ParserNode::ExponentUpdate))
// }

// pub fn update_matrix_multiply_operator(input: ParseString) -> ParseResult<ParserNode> {
//   let (input, _) = tag(":**=")(input)?;
//   Ok((input, ParserNode::Null))
// }

// update_operator ::= add_update_operator | subtract_update_operator | multiply_update_operator | divide_update_operator ;
pub fn update_operator(input: ParseString) -> ParseResult<ParserNode> {
  alt((add_update_operator,subtract_update_operator,multiply_update_operator,divide_update_operator))(input)
}

// update_data ::= data, <!stmt_operator>, space*, update_operator, <space+>, <expression> ;
pub fn update_data(input: ParseString) -> ParseResult<ParserNode> {
  let msg1 = "Expect spaces around operator";
  let msg2 = "Expect expression";
  let (input, table) = data(input)?;
  let (input, _) = labelr!(null(is_not(stmt_operator)), skip_nil, msg1)(input)?;
  let (input, _) = many0(space)(input)?;
  let (input, op) = update_operator(input)?;
  let (input, _) = labelr!(null(many1(space)), skip_nil, msg1)(input)?;
  let (input, expression) = label!(expression, msg2)(input)?;
  Ok((input, ParserNode::UpdateData{children: vec![op, table, expression]}))
}

// async_assign ::= (identifier | table), <!stmt_operator>, space*, async_assign_operator, <space+>, <expression> ;
pub fn async_assign(input: ParseString) -> ParseResult<ParserNode> {
  let msg1 = "Expect spaces around operator";
  let msg2 = "Expect expression";
  let (input, table) = alt((identifier, table))(input)?;
  let (input, _) = labelr!(null(is_not(stmt_operator)), skip_nil, msg1)(input)?;
  let (input, _) = many0(space)(input)?;
  let (input, _) = async_assign_operator(input)?;
  let (input, _) = labelr!(null(many1(space)), skip_nil, msg1)(input)?;
  let (input, expression) = label!(expression, msg2)(input)?;
  Ok((input, ParserNode::AsyncAssign{children: vec![table, expression]}))
}

// set_operator ::= ":=" ;
pub fn set_operator(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag(":=")(input)?;
  Ok((input, ParserNode::Null))
}

// set_data ::= data, <!stmt_operator>, space*, set_operator, <space+>, <expression> ;
pub fn set_data(input: ParseString) -> ParseResult<ParserNode> {
  let msg1 = "Expect spaces around operator";
  let msg2 = "Expect expression";
  let (input, table) = data(input)?;
  let (input, _) = labelr!(null(is_not(stmt_operator)), skip_nil, msg1)(input)?;
  let (input, _) = many0(space)(input)?;
  let (input, _) = set_operator(input)?;
  let (input, _) = labelr!(null(many1(space)), skip_nil, msg1)(input)?;
  let (input, expression) = label!(expression, msg2)(input)?;
  Ok((input, ParserNode::SetData{children: vec![table, expression]}))
}

// split_data ::= (identifier | table), <!stmt_operator>, space*, split_operator, <space+>, <expression> ;
pub fn split_data(input: ParseString) -> ParseResult<ParserNode> {
  let msg1 = "Expect spaces around operator";
  let msg2 = "Expect expression";
  let (input, table) = alt((identifier, table))(input)?;
  let (input, _) = labelr!(null(is_not(stmt_operator)), skip_nil, msg1)(input)?;
  let (input, _) = many0(space)(input)?;
  let (input, _) = split_operator(input)?;
  let (input, _) = labelr!(null(many1(space)), skip_nil, msg1)(input)?;
  let (input, expression) = label!(expression, msg2)(input)?;
  Ok((input, ParserNode::SplitData{children: vec![table, expression]}))
}

// flatten_data ::= identifier, <!stmt_operator>, space*, flatten_operator, <space+>, <expression> ;
pub fn flatten_data(input: ParseString) -> ParseResult<ParserNode> {
  let msg1 = "Expect spaces around operator";
  let msg2 = "Expect expression";
  let (input, table) = identifier(input)?;
  let (input, _) = labelr!(null(is_not(stmt_operator)), skip_nil, msg1)(input)?;
  let (input, _) = many0(space)(input)?;
  let (input, _) = flatten_operator(input)?;
  let (input, _) = labelr!(null(many1(space)), skip_nil, msg1)(input)?;
  let (input, expression) = label!(expression, msg2)(input)?;
  Ok((input, ParserNode::FlattenData{children: vec![table, expression]}))
}

// variable_define ::= identifier, <!stmt_operator>, space*, equal, <space+>, <expression> ;
pub fn variable_define(input: ParseString) -> ParseResult<ParserNode> {
  let msg1 = "Expect spaces around operator";
  let msg2 = "Expect expression";
  let (input, variable) = identifier(input)?;
  let (input, _) = labelr!(null(is_not(stmt_operator)), skip_nil, msg1)(input)?;
  let (input, _) = many0(space)(input)?;
  let (input, _) = equal(input)?;
  let (input, _) = labelr!(null(many1(space)), skip_nil, msg1)(input)?;
  let (input, expression) = label!(expression, msg2)(input)?;
  Ok((input, ParserNode::VariableDefine{children: vec![variable, expression]}))
}

pub fn table_define(input: ParseString) -> ParseResult<ParserNode> {
  alt((raw_table_define, formatted_table_define))(input)
}

pub fn raw_table_define(input: ParseString) -> ParseResult<ParserNode> {
  let msg1 = "Expect spaces around operator";
  let msg2 = "Expect expression";
  let mut children = vec![];
  let (input, table) = table(input)?;
  children.push(table);
  let (input, kind_id) = opt(kind_annotation)(input)?;
  if let Some(kind_id) = kind_id { children.push(kind_id); }
  let (input, _) = labelr!(null(is_not(stmt_operator)), skip_nil, msg1)(input)?;
  let (input, _) = many0(space)(input)?;
  let (input, _) = equal(input)?;
  let (input, _) = labelr!(null(many1(space)), skip_nil, msg1)(input)?;
  let (input, expression) = label!(expression, msg2)(input)?;
  children.push(expression);
  Ok((input, ParserNode::TableDefine{children}))
}
// parser for table in output format
pub fn formatted_table_define(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = table_line(input)?;
  let (input, name) = table_name(input)?;
  let (input, _) = table_line(input)?;
  let (input, table) = alt((table_with_column, table_no_column))(input)?;
  let mut children = vec![];
  children.push(name); 
  children.push(table);
  Ok((input, ParserNode::TableDefine{children}))
}
pub fn table_with_column(input: ParseString) -> ParseResult<ParserNode> {
  let (input, table_header) = formatted_table_columns(input)?;
  let (input, _) = table_line(input)?;
  let (input, _) = table_kinds(input)?;
  let (input, _) = table_line(input)?;
  let (input, mut items) = many1(table_items)(input)?;
  let (input, _) = table_line(input)?;
  let mut table = vec![];
  table.push(table_header);
  table.append(&mut items);
  Ok((input,ParserNode::AnonymousTable { children: table }))
}
pub fn table_no_column(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = table_kinds(input)?;
  let (input, _) = table_line(input)?;
  let (input, mut items) = many1(table_items)(input)?;
  let (input, _) = table_line(input)?;
  let mut table = vec![];
  table.append(&mut items);
  Ok((input,ParserNode::AnonymousTable { children: table }))
}
// parser for any line in the output table
pub fn table_line(input: ParseString) -> ParseResult<ParserNode> {
  let(input, _) = alt((tag("╭"), tag("├"), tag("╰")))(input)?;
  let(input, _) = many1(alt((tag("┼"),tag("─"),tag("┬"),tag("┴"))))(input)?;
  let(input, _) = alt((tag("╮"), tag("┤"), tag("╯")))(input)?;
  let(input, _) = newline(input)?;
  Ok((input, ParserNode::Null))
}
pub fn formatted_table_columns(input: ParseString) -> ParseResult<ParserNode> {
  let(input, _) = tag("│")(input)?;
  let (input, attr) = many1(formatted_table_column)(input)?;
  let(input, _) = newline(input)?;
  Ok((input, ParserNode::TableHeader { children: attr }))
}
pub fn formatted_table_column(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = many0(space)(input)?;
  let (input, item) = identifier(input)?;
  let (input, _) = many1(space)(input)?;
  let (input, _) = tag("│")(input)?;
  Ok((input, ParserNode::Attribute { children: vec![item] }))
}
// parser for the second line of the output table, generate the 
// var name if there is one.
pub fn table_name(input: ParseString) -> ParseResult<ParserNode> {
  let(input, _) = tag("│")(input)?;
  let(input, table_name) = table(input)?;
  let(input, s) = many0(alt((space, left_parenthesis, right_parenthesis, word, number)))(input)?;
  let(input, _) = tag("│")(input)?;
  let(input, _) = newline(input)?;
  Ok((input,table_name))
}
pub fn table_kinds(input: ParseString) -> ParseResult<ParserNode> {
  let(input, _) = tag("│")(input)?;
  let (input, _) = many1(table_kind)(input)?;
  let(input, _) = newline(input)?;
  Ok((input, ParserNode::Null))
}
pub fn table_kind(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = many0(space)(input)?;
  let (input, kind_id) = identifier(input)?;
  let (input, _) = many1(space)(input)?;
  let (input, _) = tag("│")(input)?;
  Ok((input, ParserNode::KindAnnotation { children: (vec![kind_id]) }))
}
pub fn table_items(input: ParseString) -> ParseResult<ParserNode> {
  let(input, _) = tag("│")(input)?;
  let (input, mut table_items) = many1(table_item)(input)?;
  let(input, _) = newline(input)?;
  Ok((input, ParserNode::TableRow{children:table_items}))
}
pub fn table_item(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = many0(space)(input)?;
  let (input, item) = expression(input)?;
  let (input, _) = many1(space)(input)?;
  let (input, _) = tag("│")(input)?;
  Ok((input, ParserNode::Column { children: vec![item] }))
}
pub fn table_select(input: ParseString) -> ParseResult<ParserNode> {
  let (input, expression) = expression(input)?;
  Ok((input, ParserNode::TableSelect{children: vec![expression]}))
}

// split_operator ::= ">-" ;
pub fn split_operator(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag(">-")(input)?;
  Ok((input, ParserNode::Null))
}

// flatten_operator ::= "-<" ;
pub fn flatten_operator(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag("-<")(input)?;
  Ok((input, ParserNode::Null))
}

// whenever_operator ::= "~" ;
pub fn whenever_operator(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag("~")(input)?;
  Ok((input, ParserNode::Null))
}

// until_operator ::= "~|" ;
pub fn until_operator(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag("~|")(input)?;
  Ok((input, ParserNode::Null))
}

// wait_operator ::= "|~" ;
pub fn wait_operator(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag("|~")(input)?;
  Ok((input, ParserNode::Null))
}

// followed_by_operator ::= "~>" ;
pub fn followed_by_operator(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag("~>")(input)?;
  Ok((input, ParserNode::Null))
}

// followed_by ::= table, kind_annotation?, <!stmt_operator>, space*, equal, <space+>, <expression>, space*, <followed_by_operator>, space*, <expression> ;
pub fn followed_by(input: ParseString) -> ParseResult<ParserNode> {
  let msg1 = "Expect spaces around operator";
  let msg2 = "Expect expression";
  let mut children = vec![];
  let (input, table) = table(input)?;
  children.push(table);
  let (input, kind_id) = opt(kind_annotation)(input)?;
  if let Some(kind_id) = kind_id { children.push(kind_id); }
  let (input, _) = labelr!(null(is_not(stmt_operator)), skip_nil, msg1)(input)?;
  let (input, _) = many0(space)(input)?;
  let (input, _) = equal(input)?;
  let (input, _) = labelr!(null(many1(space)), skip_nil, msg1)(input)?;
  let (input, nexpression) = label!(expression, msg2)(input)?;
  children.push(nexpression);
  let (input, _) = labelr!(null(many1(space)), skip_nil, msg1)(input)?;
  let (input, _) = followed_by_operator(input)?;
  let (input, _) = labelr!(null(many1(space)), skip_nil, msg1)(input)?;
  let (input, nexpression) = label!(expression, msg2)(input)?;
  children.push(nexpression);
  Ok((input, ParserNode::FollowedBy{children}))
}

// whenever_data ::= whenever_operator, <space>, <!space>, <variable_define | expression | data> ;
pub fn whenever_data(input: ParseString) -> ParseResult<ParserNode> {
  let msg1 = "Expect exactly 1 space after \"whenever\" operator";
  let msg2 = "Expect variable define, expression, or data";
  let (input, _) = whenever_operator(input)?;
  let (input, _) = labelr!(space, skip_nil, msg1)(input)?;
  let (input, _) = labelr!(is_not(space), skip_spaces, msg1)(input)?;
  let (input, watch) = label!(alt((variable_define, expression, data)), msg2)(input)?;
  Ok((input, ParserNode::Whenever{children: vec![watch]}))
}

// wait_data ::= wait_operator, <space>, <!space>, <variable_define | expression | data> ;
pub fn wait_data(input: ParseString) -> ParseResult<ParserNode> {
  let msg1 = "Expect exactly 1 space after \"wait\" operator";
  let msg2 = "Expect variable define, expression, or data";
  let (input, _) = wait_operator(input)?;
  let (input, _) = labelr!(space, skip_nil, msg1)(input)?;
  let (input, _) = labelr!(is_not(space), skip_spaces, msg1)(input)?;
  let (input, watch) = label!(alt((variable_define, expression, data)), msg2)(input)?;
  Ok((input, ParserNode::Wait{children: vec![watch]}))
}

// until_data ::= until_operator, <space>, <!space>, <variable_define | expression | data> ;
pub fn until_data(input: ParseString) -> ParseResult<ParserNode> {
  let msg1 = "Expect exactly 1 space after \"until\" operator";
  let msg2 = "Expect variable define, expression, or data";
  let (input, _) = until_operator(input)?;
  let (input, _) = labelr!(space, skip_nil, msg1)(input)?;
  let (input, _) = labelr!(is_not(space), skip_spaces, msg1)(input)?;
  let (input, watch) = label!(alt((variable_define, expression, data)), msg2)(input)?;
  Ok((input, ParserNode::Until{children: vec![watch]}))
}

// statement ::= (followed_by  | async_assign  | table_define | variable_define | split_data  | flatten_data | whenever_data | wait_data |
// >>             until_data   | set_data     | update_data     | add_row     | comment ), space*, <newline+> ;
pub fn statement(input: ParseString) -> ParseResult<ParserNode> {
  let msg = "Expect newline to terminate statement";
  let (input, (statement, src_range)) = range(alt((followed_by, async_assign, table_define, variable_define, split_data, flatten_data, whenever_data, wait_data, until_data, set_data, update_data, add_row, comment)))(input)?;
  let (input, _) = many0(space)(input)?;
  let (input, _) = label!(many1(newline), msg)(input)?;
  Ok((input, ParserNode::Statement{children: vec![statement], src_range}))
}

// #### Expressions

// ##### Math expressions

// parenthetical_expression ::= left_parenthesis, <l0>, <right_parenthesis> ;
pub fn parenthetical_expression(input: ParseString) -> ParseResult<ParserNode> {
  let msg1 = "Expect expression";
  let msg2 = "Expect right parenthesis ')'";
  let (input, (_, r)) = range(left_parenthesis)(input)?;
  let (input, l0) = label!(l0, msg1)(input)?;
  let (input, _) = label!(right_parenthesis, msg2, r)(input)?;
  Ok((input, l0))
}

// TODO: This won't parse -(5 - 3)
// negation ::= dash, !(dash | space), <data | value> ;
pub fn negation(input: ParseString) -> ParseResult<ParserNode> {
  let msg = "Expect a value to immediately follow the negation sign";
  let (input, (_, r)) = range(dash)(input)?;
  let (input, _) = is_not(alt((dash, space)))(input)?;  // so it's not comment sigil
  let (input, negated) = label!(alt((data, value)), msg, r)(input)?;
  Ok((input, ParserNode::Negation { children: vec![negated] }))
}

// function ::= identifier, left_parenthesis, <function_binding+>, <right_parenthesis> ;
pub fn function(input: ParseString) -> ParseResult<ParserNode> {
  let msg1 = "Expect function binding";
  let msg2 = "Expect right parenthesis ')'";
  let (input, identifier) = identifier(input)?;
  let (input, (_, r)) = range(left_parenthesis)(input)?;
  let (input, mut bindings) = label!(many1(function_binding), msg1)(input)?;
  let (input, _) = label!(right_parenthesis, msg2, r)(input)?;
  let mut function = vec![identifier];
  function.append(&mut bindings);
  Ok((input, ParserNode::Function { children: function }))
}

// user_function ::= left_bracket, function_output*, <right_bracket>, <space+>, <equal>, <space+>, <identifier>,
// >>                <left_parenthesis>, <function_input*>, <right_parenthesis>, <newline>, <function_body> ;
pub fn user_function(input: ParseString) -> ParseResult<ParserNode> {
  let msg1 = "Expect right bracket for user function definition";
  let msg2 = "Expect space after output declaration";
  let msg3 = "Expect equal sign '='";
  let msg4 = "Expect space after equal sign";
  let msg5 = "Expect identifier for function name";
  let msg6 = "Expect left parenthesis '('";
  let msg7 = "Expect right parenthesis ')'";
  let msg8 = "Expect newline after user function header";
  let msg9 = "Expect indented transformations for function body";
  let start = input.loc();
  let (input, (_, r1)) = range(left_bracket)(input)?;
  let (input, mut output_args) = many0(function_output)(input)?;
  let (input, _) = label!(right_bracket, msg1, r1)(input)?;
  let (input, _) = label!(many1(space), msg2)(input)?;
  let (input, _) = label!(equal, msg3)(input)?;
  let (input, _) = label!(many1(space), msg4)(input)?;
  let (input, function_name) = label!(identifier, msg5)(input)?;
  let (input, (_, r2)) = label!(range(left_parenthesis), msg6)(input)?;
  let (input, mut input_args) = many0(function_input)(input)?;
  let (input, _) = label!(right_parenthesis, msg7, r2)(input)?;
  let (input, _) = label!(newline, msg8)(input)?;
  let end = input.loc();
  let (input, function_body) = label!(function_body, msg9, SourceRange {start, end})(input)?;
  Ok((input, ParserNode::UserFunction { children: vec![ParserNode::FunctionArgs{children: output_args}, function_name, ParserNode::FunctionArgs{children: input_args}, function_body] }))
}

// function_output ::= identifier, <kind_annotation>, space*, comma?, space* ;
pub fn function_output(input: ParseString) -> ParseResult<ParserNode> {
  let msg = "Expect kind annotation";
  let (input, arg_id) = identifier(input)?;
  let (input, kind) = label!(kind_annotation, msg)(input)?;
  let (input, _) = tuple((many0(space), opt(comma), many0(space)))(input)?;
  Ok((input, ParserNode::FunctionOutput{children: vec![arg_id, kind]}))
}

// function_input ::= identifier, <kind_annotation>, space*, comma?, space* ;
pub fn function_input(input: ParseString) -> ParseResult<ParserNode> {
  let msg = "Expect kind annotation";
  let (input, arg_id) = identifier(input)?;
  let (input, kind) = label!(kind_annotation, msg)(input)?;
  let (input, _) = tuple((many0(space), opt(comma), many0(space)))(input)?;
  Ok((input, ParserNode::FunctionInput{children: vec![arg_id, kind]}))
}

// function_body ::= indented_tfm+, whitespace* ;
pub fn function_body(input: ParseString) -> ParseResult<ParserNode> {
  let (input, transformations) = many1(indented_tfm)(input)?;
  let (input, _) = many0(whitespace)(input)?;
  Ok((input, ParserNode::FunctionBody { children: transformations }))
}

// matrix_multiply ::= "**" ;
pub fn matrix_multiply(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag("**")(input)?;
  Ok((input, ParserNode::MatrixMultiply))
}

// add ::= "+" ;
pub fn add(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag("+")(input)?;
  Ok((input, ParserNode::Add))
}

// subtract ::= "-" ;
pub fn subtract(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag("-")(input)?;
  Ok((input, ParserNode::Subtract))
}

// multiply ::= "*" ;
pub fn multiply(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag("*")(input)?;
  Ok((input, ParserNode::Multiply))
}

// divide ::= "/" ;
pub fn divide(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag("/")(input)?;
  Ok((input, ParserNode::Divide))
}

// exponent ::= "^" ;
pub fn exponent(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag("^")(input)?;
  Ok((input, ParserNode::Exponent))
}

// range_op ::= colon ;
pub fn range_op(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = colon(input)?;
  Ok((input, ParserNode::Range))
}

// l0 ::= l1, l0_infix* ;
pub fn l0(input: ParseString) -> ParseResult<ParserNode> {
  let (input, l1) = l1(input)?;
  let (input, mut infix) = many0(l0_infix)(input)?;
  let mut math = vec![l1];
  math.append(&mut infix);
  Ok((input, ParserNode::L0 { children: math }))
}

// l0_infix ::= <!(space+, colon)>, range_op, <!space>, <l1> ;
pub fn l0_infix(input: ParseString) -> ParseResult<ParserNode> {
  let msg1 = "Unexpected space around range operator";
  let msg2 = "Expect expression after range operator";
  let (input, _) = labelr!(is_not(tuple((many1(space), colon))), skip_spaces, msg1)(input)?;
  let (input, (op, r)) = range(range_op)(input)?;
  let (input, _) = labelr!(is_not(space), skip_spaces, msg1)(input)?;
  let (input, l1) = label!(l1, msg2, r)(input)?;
  Ok((input, ParserNode::L0Infix { children: vec![op, l1] }))
}

// l1 ::= l2, l1_infix* ;
pub fn l1(input: ParseString) -> ParseResult<ParserNode> {
  let (input, l2) = l2(input)?;
  let (input, mut infix) = many0(l1_infix)(input)?;
  let mut math = vec![l2];
  math.append(&mut infix);
  Ok((input, ParserNode::L1 { children: math }))
}

// l1_op ::= add | subtract ;
pub fn l1_op(input: ParseString) -> ParseResult<ParserNode> {
  alt((add, subtract))(input)
}

// l1_infix ::= <!l1_op>, space*, !negation, !comment_sigil, l1_op, <space+>, <l2> ;
pub fn l1_infix(input: ParseString) -> ParseResult<ParserNode> {
  let msg1 = "Expect spaces around opeartor";
  let msg2 = "Expect expression after operator";
  let (input, _) = labelr!(null(is_not(l1_op)), skip_nil, msg1)(input)?;
  let (input, _) = many0(space)(input)?;
  let (input, _) = is_not(negation)(input)?;
  let (input, _) = is_not(comment_sigil)(input)?;
  let (input, op) = l1_op(input)?;
  let (input, _) = labelr!(null(many1(space)), skip_nil, msg1)(input)?;
  let (input, l2) = label!(l2, msg2)(input)?;
  Ok((input, ParserNode::L1Infix { children: vec![op, l2] }))
}

// l2 ::= l3, l2_infix* ;
pub fn l2(input: ParseString) -> ParseResult<ParserNode> {
  let (input, l3) = l3(input)?;
  let (input, mut infix) = many0(l2_infix)(input)?;
  let mut math = vec![l3];
  math.append(&mut infix);
  Ok((input, ParserNode::L2 { children: math }))
}

// l2_op ::= matrix_multiply | multiply | divide ;
pub fn l2_op(input: ParseString) -> ParseResult<ParserNode> {
  alt((matrix_multiply, multiply, divide))(input)
}

// l2_infix ::= <!l2_op>, space*, l2_op, <space+>, <l3> ;
pub fn l2_infix(input: ParseString) -> ParseResult<ParserNode> {
  let msg1 = "Expect spaces around opeartor";
  let msg2 = "Expect expression after operator";
  let (input, _) = labelr!(null(is_not(l2_op)), skip_nil, msg1)(input)?;
  let (input, _) = many0(space)(input)?;
  let (input, op) = l2_op(input)?;
  let (input, _) = labelr!(null(many1(space)), skip_nil, msg1)(input)?;
  let (input, l3) = label!(l3, msg2)(input)?;
  Ok((input, ParserNode::L2Infix { children: vec![op, l3] }))
}

// l3 ::= l4, l3_infix* ;
pub fn l3(input: ParseString) -> ParseResult<ParserNode> {
  let (input, l4) = l4(input)?;
  let (input, mut infix) = many0(l3_infix)(input)?;
  let mut math = vec![l4];
  math.append(&mut infix);
  Ok((input, ParserNode::L3 { children: math }))
}

// l3_op ::= exponent ;
pub fn l3_op(input: ParseString) -> ParseResult<ParserNode> {
  exponent(input)
}

// l3_infix ::= <!l3_op>, space*, l3_op, <space+>, <l4> ;
pub fn l3_infix(input: ParseString) -> ParseResult<ParserNode> {
  let msg1 = "Expect spaces around opeartor";
  let msg2 = "Expect expression after operator";
  let (input, _) = labelr!(null(is_not(l3_op)), skip_nil, msg1)(input)?;
  let (input, _) = many0(space)(input)?;
  let (input, op) = l3_op(input)?;
  let (input, _) = labelr!(null(many1(space)), skip_nil, msg1)(input)?;
  let (input, l4) = label!(l4, msg2)(input)?;
  Ok((input, ParserNode::L3Infix { children: vec![op, l4] }))
}

// l4 ::= l5, l4_infix* ;
pub fn l4(input: ParseString) -> ParseResult<ParserNode> {
  let (input, l5) = l5(input)?;
  let (input, mut infix) = many0(l4_infix)(input)?;
  let mut math = vec![l5];
  math.append(&mut infix);
  Ok((input, ParserNode::L4 { children: math }))
}

// l4_op ::= and | or | xor ;
pub fn l4_op(input: ParseString) -> ParseResult<ParserNode> {
  alt((and, or, xor))(input)
}

// l4_infix ::= <!l4_op>, space*, l4_op, <space+>, <l5> ;
pub fn l4_infix(input: ParseString) -> ParseResult<ParserNode> {
  let msg1 = "Expect spaces around opeartor";
  let msg2 = "Expect expression after operator";
  let (input, _) = labelr!(null(is_not(l4_op)), skip_nil, msg1)(input)?;
  let (input, _) = many0(space)(input)?;
  let (input, op) = l4_op(input)?;
  let (input, _) = labelr!(null(many1(space)), skip_nil, msg1)(input)?;
  let (input, l5) = label!(l5, msg2)(input)?;
  Ok((input, ParserNode::L4Infix { children: vec![op, l5] }))
}

// l5 ::= l6, l5_infix* ;
pub fn l5(input: ParseString) -> ParseResult<ParserNode> {
  let (input, l6) = l6(input)?;
  let (input, mut infix) = many0(l5_infix)(input)?;
  let mut math = vec![l6];
  math.append(&mut infix);
  Ok((input, ParserNode::L5 { children: math }))
}

// l5_op ::= not_equal | equal_to | greater_than_equal | greater_than | less_than_equal | less_than ;
pub fn l5_op(input: ParseString) -> ParseResult<ParserNode> {
  alt((not_equal, equal_to, greater_than_equal, greater_than, less_than_equal, less_than))(input)
}

// l5_infix ::= <!l5_op>, space*, l5_op, <space+>, <l6> ;
pub fn l5_infix(input: ParseString) -> ParseResult<ParserNode> {
  let msg1 = "Expect spaces around opeartor";
  let msg2 = "Expect expression after operator";
  let (input, _) = labelr!(null(is_not(l5_op)), skip_nil, msg1)(input)?;
  let (input, _) = many0(space)(input)?;
  let (input, op) = l5_op(input)?;
  let (input, _) = labelr!(null(many1(space)), skip_nil, msg1)(input)?;
  let (input, l6) = label!(l6, msg2)(input)?;
  Ok((input, ParserNode::L5Infix { children: vec![op, l6] }))
}

// l6 ::= empty_table | string | anonymous_table | function | value | not | data | negation | parenthetical_expression ;
pub fn l6(input: ParseString) -> ParseResult<ParserNode> {
  let (input, l6) = alt((empty_table, string, anonymous_table, function, value, not, data, negation, parenthetical_expression))(input)?;
  Ok((input, ParserNode::L6 { children: vec![l6] }))
}

// math_expression ::= l0 ;
pub fn math_expression(input: ParseString) -> ParseResult<ParserNode> {
  let (input, l0) = l0(input)?;
  Ok((input, ParserNode::MathExpression { children: vec![l0] }))
}

// ##### Filter expressions

// not_equal ::= "!=" | "¬=" | "≠" ;
pub fn not_equal(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = alt((tag("!="),tag("¬="),tag("≠")))(input)?;
  Ok((input, ParserNode::NotEqual))
}

// equal_to ::= "==" ;
pub fn equal_to(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag("==")(input)?;
  Ok((input, ParserNode::Equal))
}

// greater_than ::= ">" ;
pub fn greater_than(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag(">")(input)?;
  Ok((input, ParserNode::GreaterThan))
}

// less_than ::= "<" ;
pub fn less_than(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag("<")(input)?;
  Ok((input, ParserNode::LessThan))
}

// greater_than_equal ::= ">=" | "≥" ;
pub fn greater_than_equal(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = alt((tag(">="),tag("≥")))(input)?;
  Ok((input, ParserNode::GreaterThanEqual))
}

// less_than_equal ::= "<=" | "≤" ;
pub fn less_than_equal(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = alt((tag("<="),tag("≤")))(input)?;
  Ok((input, ParserNode::LessThanEqual))
}

// ##### Logic expressions

// or ::= "|" ;
pub fn or(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag("|")(input)?;
  Ok((input, ParserNode::Or))
}

// and ::= "&" ;
pub fn and(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag("&")(input)?;
  Ok((input, ParserNode::And))
}

// not ::= "!" | "¬" ;
pub fn not(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = alt((tag("!"), tag("¬")))(input)?;
  let (input, negated) = alt((data, true_literal, false_literal))(input)?;
  Ok((input, ParserNode::Not { children: vec![negated] }))
}

// xor ::= "xor" | "⊕" | "⊻" ;
pub fn xor(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = alt((tag("xor"), tag("⊕"), tag("⊻")))(input)?;
  Ok((input, ParserNode::Xor))
}

// ##### Other expressions

// pub fn string_interpolation(input: ParseString) -> IResult<ParseString, ParserNode> {
//   let (input, _) = tag("{{")(input)?;
//   let (input, expression) = expression(input)?;
//   let (input, _) = tag("}}")(input)?;
//   Ok((input, ParserNode::StringInterpolation { children: vec![expression] }))
// }

// string ::= quote, (!quote, <text>)*, quote ;
pub fn string(input: ParseString) -> ParseResult<ParserNode> {
  let msg = "Character not allowed in string";
  let (input, _) = quote(input)?;
  let (input, matched) = many0(tuple((is_not(quote), label!(text, msg))))(input)?;
  let (input, _) = quote(input)?;
  let (_, text): ((), Vec<_>) = matched.into_iter().unzip();
  Ok((input, ParserNode::String { children: text }))
}

// transpose ::= "'" ;
pub fn transpose(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag("'")(input)?;
  Ok((input, ParserNode::Transpose))
}

// expression ::= (empty_table | inline_table | math_expression | string | anonymous_table), transpose? ;
pub fn expression(input: ParseString) -> ParseResult<ParserNode> {
  let (input, expression) = alt((inline_table, math_expression, string, empty_table, anonymous_table))(input)?;
  let (input, transpose) = opt(transpose)(input)?;
  let mut children = vec![expression];
  match transpose {
    Some(transpose) => children.push(transpose),
    _ => (),
  }
  Ok((input, ParserNode::Expression { children }))
}

// #### Block basics

// transformation ::= statement;
pub fn transformation(input: ParseString) -> ParseResult<ParserNode> {
  let (input, statement) = statement(input)?;
  Ok((input, ParserNode::Transformation { children: vec![statement] }))
}

// empty_line ::= space*, newline ;
pub fn empty_line(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tuple((many0(space), newline))(input)?;
  Ok((input, ParserNode::Null))
}

// indented_tfm ::= !empty_line, space, <space>, <!space>, <transformation> ;
pub fn indented_tfm(input: ParseString) -> ParseResult<ParserNode> {
  let msg1 = "Indentation has to be exactly 2 spaces";
  let msg2 = "Expect transformation";
  let (input, _) = tuple((
    is_not(empty_line),
    space,
    labelr!(space, skip_nil, msg1),
    labelr!(is_not(space), skip_spaces, msg1),
  ))(input)?;
  label!(transformation, msg2)(input)
}

// block ::= indented_tfm+, whitespace* ;
pub fn block(input: ParseString) -> ParseResult<ParserNode> {
  let (input, (transformations, src_range)) = range(many1(indented_tfm))(input)?;
  let (input, _) = many0(whitespace)(input)?;
  Ok((input, ParserNode::Block { children: transformations, src_range }))
}

// ### Markdown

// ul_title ::= space*, text, space*, newline, equal+, space*, newline* ;
pub fn ul_title(input: ParseString) -> ParseResult<ParserNode> {
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
pub fn title(input: ParseString) -> ParseResult<ParserNode> {
  let (input,title) = ul_title(input)?;
  Ok((input, title))
}

// ul_subtitle ::= space*, text, space*, newline, dash+, space*, newline* ;
pub fn ul_subtitle(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = many0(space)(input)?;
  let (input, text) = text(input)?;
  let (input, _) = many0(space)(input)?;
  let (input, _) = newline(input)?;
  let (input, _) = dash(input)?;
  let (input, _) = dash(input)?;
  let (input, _) = dash(input)?;
  let (input, _) = many1(dash)(input)?;
  let (input, _) = many0(space)(input)?;
  let (input, _) = many0(newline)(input)?;
  Ok((input, ParserNode::Subtitle { level: 1, children: vec![text] }))
}

// number_subtitle ::= space*, number, period, space+, text, space*, newline* ;
pub fn number_subtitle(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = many0(space)(input)?;
  let (input, _) = left_parenthesis(input)?;
  let (input, _) = number(input)?;
  let (input, _) = right_parenthesis(input)?;
  let (input, _) = many1(space)(input)?;
  let (input, title) = text(input)?;
  let (input, _) = many0(space)(input)?;
  let (input, _) = many0(newline)(input)?;
  Ok((input, ParserNode::Subtitle {level: 2, children: vec![title] }))
}

// alpha_subtitle ::= space*, alpha, right_parenthesis, space+, text, space*, newline* ;
pub fn alpha_subtitle(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = many0(space)(input)?;
  let (input, _) = left_parenthesis(input)?;
  let (input, _) = alpha(input)?;
  let (input, _) = right_parenthesis(input)?;
  let (input, _) = many1(space)(input)?;
  let (input, title) = text(input)?;
  let (input, _) = many0(space)(input)?;
  let (input, _) = many0(newline)(input)?;
  Ok((input, ParserNode::Subtitle  { level: 3, children: vec![title] }))
}

// subtitle ::= ul_subtitle | number_subtitle | alpha_subtitle;
pub fn subtitle(input: ParseString) -> ParseResult<ParserNode> {
  let (input,title) = alt((ul_subtitle,alpha_subtitle,number_subtitle))(input)?;
  Ok((input, title))
}

// inline_code ::= grave, text, grave, space* ;
pub fn inline_code(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = grave(input)?;
  let (input, text) = text(input)?;
  let (input, _) = grave(input)?;
  let (input, _) = many0(space)(input)?;
  Ok((input, ParserNode::InlineCode { children: vec![text] }))
}

// paragraph_text ::= paragraph_starter, paragraph_rest? ;
pub fn paragraph_text(input: ParseString) -> ParseResult<ParserNode> {
  let (input, word) = paragraph_starter(input)?;
  let (input, text) = opt(paragraph_rest)(input)?;
  let mut paragraph = vec![word];
  match text {
    Some(text) => paragraph.push(text),
    _ => (),
  };
  Ok((input, ParserNode::ParagraphText { children: paragraph }))
}

// paragraph ::= (inline_code | paragraph_text)+, whitespace*, newline* ;
pub fn paragraph(input: ParseString) -> ParseResult<ParserNode> {
  let (input, paragraph_elements) = many1(
    alt((inline_code, paragraph_text))
  )(input)?;
  let (input, _) = many0(whitespace)(input)?;
  let (input, _) = many0(newline)(input)?;
  Ok((input, ParserNode::Paragraph { children: paragraph_elements }))
}

// unordered_list ::= list_item+, newline?, whitespace* ;
pub fn unordered_list(input: ParseString) -> ParseResult<ParserNode> {
  let (input, list_items) = many1(list_item)(input)?;
  let (input, _) = opt(newline)(input)?;
  let (input, _) = many0(whitespace)(input)?;
  Ok((input, ParserNode::UnorderedList { children: list_items }))
}

// list_item ::= dash, <space+>, <paragraph>, newline* ;
pub fn list_item(input: ParseString) -> ParseResult<ParserNode> {
  let msg1 = "Expect space after dash";
  let msg2 = "Expect paragraph as list item";
  let (input, _) = dash(input)?;
  let (input, _) = labelr!(null(many1(space)), skip_nil, msg1)(input)?;
  let (input, list_item) = label!(paragraph, msg2)(input)?;
  let (input, _) = many0(newline)(input)?;
  Ok((input, ParserNode::ListItem { children: vec![list_item] }))
}

// formatted_text ::= (!grave, !eof, <paragraph_rest | carriage_return | new_line_char>)* ;
pub fn formatted_text(input: ParseString) -> ParseResult<ParserNode> {
  let msg = "Character not permitted in formatted text";
  let (input, result) = many0(tuple((
    tuple((is_not(grave), is_not(eof))),
    label!(alt((paragraph_rest, newline)), msg)
  )))(input)?;
  let (_, formatted): (((), ()), Vec<_>) = result.into_iter().unzip();
  Ok((input, ParserNode::FormattedText { children: formatted }))
}

// code_block ::= grave, <grave>, <grave>, <newline>, formatted_text, <grave{3}, newline, whitespace*> ;
pub fn code_block(input: ParseString) -> ParseResult<ParserNode> {
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

// pub fn inline_mech_code(input: ParseString) -> ParseResult<ParserNode> {
//   let (input, _) = tuple((left_bracket,left_bracket))(input)?;
//   let (input, expression) = expression(input)?;
//   let (input, _) = tuple((right_bracket,right_bracket,opt(space)))(input)?;
//   Ok((input, ParserNode::InlineMechCode{ children: vec![expression] }))
// }

// mech_code_block ::= grave{3}, !!"mec", <"mech:">, text?, <newline>, <block>, <grave{3}, newline>, whitespace* ;
pub fn mech_code_block(input: ParseString) -> ParseResult<ParserNode> {
  let msg1 = "Expect newline";
  let msg2 = "Expect mech code block";
  let msg3 = "Expect the \"mech:\" tag";
  let msg4 = "Expect 3 graves followed by newline to terminate the mech code block";
  let (input, (_, r)) = range(tuple((grave, grave, grave)))(input)?;
  let (input, _) = tuple((is(tag("mec")), labelr!(tag("mech:"), skip_empty_mech_directive, msg3)))(input)?;
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

// section_element ::= user_function | block | mech_code_block | code_block | statement | paragraph | unordered_list;
pub fn section_element(input: ParseString) -> ParseResult<ParserNode> {
  let (input, element) = alt((
    section2, comment, user_function, block, mech_code_block, code_block, statement, paragraph, unordered_list, 
  ))(input)?;
  Ok((input, element))
}

// section ::= (!eof, <section_element>, whitespace?)+ ;
pub fn section(input: ParseString) -> ParseResult<ParserNode> {
  let msg = "Expect user function, block, mech code block, code block, statement, paragraph, or unordered list";
  let (input, title) = opt(ul_subtitle)(input)?;
  let (input, mut section_elements) = many1(
    tuple((
      is_not(eof),
      is_not(ul_subtitle),
      labelr!(section_element, skip_till_section_element, msg),
      opt(whitespace),
    ))
  )(input)?;
  let mut section = vec![];
  section.append(&mut section_elements.iter().map(|(_,_,x,_)|x).cloned().collect());
  let title = match title {
    Some(t) => Some(vec![t]),
    None => None,
  };
  Ok((input, ParserNode::Section{title, children: section }))
}

pub fn section_element2(input: ParseString) -> ParseResult<ParserNode> {
  let (input, element) = alt((
    section3, user_function, block, mech_code_block, code_block, statement, paragraph, unordered_list, 
  ))(input)?;
  Ok((input, element))
}

pub fn section2(input: ParseString) -> ParseResult<ParserNode> {
  let msg = "Expect user function, block, mech code block, code block, statement, paragraph, or unordered list";
  let (input, title) = number_subtitle(input)?;
  let (input, mut section_elements) = many1(
    tuple((
      is_not(eof),
      is_not(ul_subtitle),
      is_not(number_subtitle),
      labelr!(section_element2, skip_till_section_element2, msg),
      opt(whitespace),
    ))
  )(input)?;
  let mut section = vec![];
  section.append(&mut section_elements.iter().map(|(_,_,_,x,_)|x).cloned().collect());
  Ok((input, ParserNode::Section{title: Some(vec![title]), children: section }))
}

pub fn section_element3(input: ParseString) -> ParseResult<ParserNode> {
  let (input, element) = alt((
    user_function, block, mech_code_block, code_block, statement, paragraph, unordered_list, 
  ))(input)?;
  Ok((input, element))
}

pub fn section3(input: ParseString) -> ParseResult<ParserNode> {
  let msg = "Expect user function, block, mech code block, code block, statement, paragraph, or unordered list";
  let (input, title) = alpha_subtitle(input)?;
  let (input, mut section_elements) = many1(
    tuple((
      is_not(eof),
      is_not(ul_subtitle),
      is_not(alpha_subtitle),
      is_not(number_subtitle),
      labelr!(section_element3, skip_till_section_element3, msg),
      opt(whitespace),
    ))
  )(input)?;
  let mut section = vec![];
  section.append(&mut section_elements.iter().map(|(_,_,_,_,x,_)|x).cloned().collect());
  Ok((input, ParserNode::Section{title: Some(vec![title]), children: section }))
}


// body ::= whitespace*, section+ ;
pub fn body(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = many0(whitespace)(input)?;
  let (input, sections) = many1(section)(input)?;
  Ok((input, ParserNode::Body { children: sections }))
}

// pub fn fragment(input: ParseString) -> ParseResult<ParserNode> {
//   let (input, statement) = statement(input)?;
//   Ok((input, ParserNode::Fragment { children:  vec![statement] }))
// }

// program ::= whitespace?, title?, <body>, whitespace?, space* ;
pub fn program(input: ParseString) -> ParseResult<ParserNode> {
  let msg = "Expect program body";
  let mut program = vec![];
  let (input, _) = opt(whitespace)(input)?;
  let (input, title) = opt(title)(input)?;
  match title {
    Some(title) => program.push(title),
    None => (),
  };
  let (input, body) = labelr!(body, skip_nil, msg)(input)?;
  program.push(body);
  let (input, _) = opt(whitespace)(input)?;
  let (input, _) = many0(space)(input)?;
  Ok((input, ParserNode::Program { children: program }))
}

// pub fn raw_transformation(input: ParseString) -> ParseResult<ParserNode> {
//   let (input, statement) = statement(input)?;
//   let (input, _) = many0(alt((space,newline,tab)))(input)?;
//   Ok((input, ParserNode::Transformation { children:  vec![statement] }))
// }

// pub fn parse_block(input: ParseString) -> ParseResult<ParserNode> {
//   let (input, transformations) = many1(raw_transformation)(input)?;
//   let (input, _) = many0(whitespace)(input)?;
//   Ok((input, ParserNode::Block { children:  transformations }))
// }

// parse_mech_fragment ::= statement ;
pub fn parse_mech_fragment(input: ParseString) -> ParseResult<ParserNode> {
  let (input, statement) = statement(input)?;
  Ok((input, ParserNode::Root { children:  vec![
    ParserNode::Program { children:  vec![
      ParserNode::Body { children:  vec![
        ParserNode::Section { title: None, children:  vec![
          statement
        ]} 
      ]}
    ]}
  ]}))
}

// parse_mech ::= program | statement ;
pub fn parse_mech(input: ParseString) -> ParseResult<ParserNode> {
  let (input, mech) = alt((program, statement))(input)?;
  Ok((input, ParserNode::Root { children: vec![mech] }))
}

// ## Public interface

/// Print formatted error message.
pub fn print_err_report(text: &str, report: &ParserErrorReport) {
  let msg = TextFormatter::new(text).format_error(report);
  println!("{}", msg);
}

pub fn parse(text: &str) -> Result<ParserNode, MechError> {
  let graphemes = graphemes::init_source(text);
  let mut result_node = ParserNode::Error;
  let mut error_log: Vec<(SourceRange, ParseErrorDetail)> = vec![];
  let remaining: ParseString;

  // Do parse
  match parse_mech(ParseString::new(&graphemes)) {
    // Got a parse tree, however there may be errors
    Ok((mut remaining_input, parse_tree)) => {
      error_log.append(&mut remaining_input.error_log);
      result_node = parse_tree;
      remaining = remaining_input;
    },
    // Parsing failed and could not be recovered. No parse tree was created in this case
    Err(err) => match err {
      Err::Error(mut e) | Err::Failure(mut e) => {
        error_log.append(&mut e.remaining_input.error_log);
        error_log.push((e.cause_range, e.error_detail));
        remaining = e.remaining_input;
      },
      Err::Incomplete(_) => panic!("nom::Err::Incomplete is not supported!"),
    },
  }

  // Check if all inputs were parsed
  if remaining.len() != 0 {
    let e = ParseError::new(remaining, "Inputs since here are not parsed");
    error_log.push((e.cause_range, e.error_detail));
  }
  
  // Construct result
  if error_log.is_empty() {
    Ok(result_node)
  } else {
    let report: ParserErrorReport = error_log.into_iter().map(|e| ParserErrorContext {
      cause_rng: e.0,
      err_message: String::from(e.1.message),
      annotation_rngs: e.1.annotation_rngs,
    }).collect();
    let msg = TextFormatter::new(text).format_error(&report);
    Err(MechError{tokens: vec![], msg: "".to_string(), id: 3202, kind: MechErrorKind::ParserError(result_node, report, msg)})
  }
}

pub fn parse_fragment(text: &str) -> Result<ParserNode, MechError> {
  let graphemes = graphemes::init_source(text);
  let mut result_node = ParserNode::Error;
  let mut error_log: Vec<(SourceRange, ParseErrorDetail)> = vec![];
  let remaining: ParseString;

  // Do parse
  match parse_mech_fragment(ParseString::new(&graphemes)) {
    // Got a parse tree, however there may be errors
    Ok((mut remaining_input, parse_tree)) => {
      error_log.append(&mut remaining_input.error_log);
      result_node = parse_tree;
      remaining = remaining_input;
    },
    // Parsing failed and could not be recovered. No parse tree was created in this case
    Err(err) => match err {
      Err::Error(mut e) | Err::Failure(mut e) => {
        error_log.append(&mut e.remaining_input.error_log);
        error_log.push((e.cause_range, e.error_detail));
        remaining = e.remaining_input;
      },
      Err::Incomplete(_) => panic!("nom::Err::Incomplete is not supported!"),
    },
  }
  
  // Check if all inputs were parsed
  if remaining.len() != 0 {
    let e = ParseError::new(remaining, "Inputs since here are not parsed");
    error_log.push((e.cause_range, e.error_detail));
  }

  // Construct result
  if error_log.is_empty() {
    Ok(result_node)
  } else {
    println!("{:?}", error_log);
    let report = error_log.into_iter().map(|e| ParserErrorContext {
      cause_rng: e.0,
      err_message: String::from(e.1.message),
      annotation_rngs: e.1.annotation_rngs,
    }).collect();
    let msg = TextFormatter::new(text).format_error(&report);
    Err(MechError{tokens: vec![], msg: "".to_string(), id: 3202, kind: MechErrorKind::ParserError(result_node, report, msg)})
  }
}