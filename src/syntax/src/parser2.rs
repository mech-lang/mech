// Parser
// ========

/// Sections:
///   1. Prelude
///   2. Parser utilities
///   3. Parser combinators
///   4. Recovery functions
///   5. Primitive parsers
///   6. Parsers
///   7. Reporting errors
///   8. Public interface

// 1. Prelude
// ------------

use mech_core::{MechError, MechErrorKind, ParserErrorContext, ParserErrorReport};
use mech_core::nodes::*;
use mech_core::nodes::{SectionElement, MechString, Table};

#[cfg(not(feature = "no-std"))] use core::fmt;
#[cfg(feature = "no-std")] use alloc::fmt;
#[cfg(feature = "no-std")] use alloc::string::String;
#[cfg(feature = "no-std")] use alloc::vec::Vec;
use nom::{
  IResult,
  branch::alt,
  sequence::tuple as nom_tuple,
  combinator::{opt, eof},
  multi::{many1, many_till, many0, separated_list1,separated_list0},
  Err,
  Err::Failure
};

use std::collections::HashMap;
use colored::*;

use crate::*;

// 3. Parser combinators
// -----------------------

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
    if input.is_empty() {
      return Err(nom::Err::Error(ParseError::new(input, "Unexpected eof")));
    }
    if let Some(matched) = input.consume_tag(tag) {
      Ok((input, matched))
    } else {
      Err(nom::Err::Error(ParseError::new(input, "Unexpected char")))
    }
  }
}

// 4. Recovery functions
// -----------------------

pub fn skip_till_eol(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = many0(nom_tuple((
    is_not(new_line),
    any,
  )))(input)?;
  Ok((input, ParserNode::Null))
}

fn skip_past_eol(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = skip_till_eol(input)?;
  let (input, _) = new_line(input)?;
  Ok((input, ParserNode::Null))
}

fn skip_till_section_element(input: ParseString) -> ParseResult<ParserNode> {
  if input.is_empty() {
    return Ok((input, ParserNode::Error));
  }
  let (input, _) = skip_past_eol(input)?;
  let (input, _) = many0(nom_tuple((
    is_not(section_element),
    skip_past_eol,
  )))(input)?;
  Ok((input, ParserNode::Error))
}

/*
fn skip_till_section_element2(input: ParseString) -> ParseResult<ParserNode> {
  if input.len() == 0 {
    return Ok((input, ParserNode::Error));
  }
  let (input, _) = skip_past_eol(input)?;
  let (input, _) = many0(nom_tuple((
    is_not(section_element2),
    skip_past_eol,
  )))(input)?;
  Ok((input, ParserNode::Error))
}

fn skip_till_section_element3(input: ParseString) -> ParseResult<ParserNode> {
  if input.len() == 0 {
    return Ok((input, ParserNode::Error));
  }
  let (input, _) = skip_past_eol(input)?;
  let (input, _) = many0(nom_tuple((
    is_not(section_element3),
    skip_past_eol,
  )))(input)?;
  Ok((input, ParserNode::Error))
}*/

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

// 5. Primitive parsers
// -----------------------

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

// 6. Parsers
// -----------------------

// (a) The basics

macro_rules! leaf {
  ($name:ident, $byte:expr, $token:expr) => (
    fn $name(input: ParseString) -> ParseResult<Token> {
      if input.is_empty() {
        return Err(nom::Err::Error(ParseError::new(input, "Unexpected eof")))
      }
      let start = input.loc();
      let byte = input.graphemes[input.cursor];
      let (input, _) = tag($byte)(input)?;
      let end = input.loc();
      let src_range = SourceRange { start, end };
      Ok((input, Token{kind: $token, chars: $byte.chars().collect::<Vec<char>>(), src_range}))
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
leaf!{dollar, "$", TokenKind::Dollar}
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
leaf!{carriage_return_new_line, "\r\n", TokenKind::CarriageReturn}
leaf!{english_true_literal, "true", TokenKind::True}
leaf!{english_false_literal, "false", TokenKind::False}
leaf!{check_mark, "✓", TokenKind::True}
leaf!{cross, "✗", TokenKind::False}
leaf!{box_tl_round, "╭", TokenKind::BoxDrawing}
leaf!{box_tr_round, "╮", TokenKind::BoxDrawing}
leaf!{box_bl_round, "╰", TokenKind::BoxDrawing}
leaf!{box_br_round, "╯", TokenKind::BoxDrawing}
leaf!{box_cross, "┼", TokenKind::BoxDrawing}
leaf!{box_horz, "─", TokenKind::BoxDrawing}
leaf!{box_t_left, "├", TokenKind::BoxDrawing}
leaf!{box_t_right, "┤", TokenKind::BoxDrawing}
leaf!{box_t_top, "┬", TokenKind::BoxDrawing}
leaf!{box_t_bottom, "┴", TokenKind::BoxDrawing}
leaf!{box_vert, "│", TokenKind::BoxDrawing}

fn forbidden_emoji(input: ParseString) -> ParseResult<Token> {
  alt((box_t_left,box_tl_round,box_br_round, box_tr_round, box_bl_round, box_vert, box_cross, box_horz, box_t_right, box_t_top, box_t_bottom))(input)
}

// emoji := emoji_grapheme+ ;
fn emoji(input: ParseString) -> ParseResult<Token> {
  let msg1 = "Cannot be a box-drawing emoji";
  let start = input.loc();
  let (input, _) = is_not(forbidden_emoji)(input)?;
  let (input, g) = emoji_grapheme(input)?;
  let end = input.loc();
  let src_range = SourceRange { start, end };
  Ok((input, Token{kind: TokenKind::Emoji, chars: g.chars().collect::<Vec<char>>(), src_range}))
}

fn alpha_token(input: ParseString) -> ParseResult<Token> {
  let (input, (g, src_range)) = range(alpha)(input)?;
  Ok((input, Token{kind: TokenKind::Alpha, chars: g.chars().collect::<Vec<char>>(), src_range}))
}

fn digit_token(input: ParseString) -> ParseResult<Token> {
  let (input, (g, src_range)) = range(digit)(input)?;
  Ok((input, Token{kind: TokenKind::Digit, chars: g.chars().collect::<Vec<char>>(), src_range}))
}

// underscore_digit := underscore, digit ;
fn underscore_digit(input: ParseString) -> ParseResult<Token> {
  let (input, _) = underscore(input)?;
  let (input, digit) = digit_token(input)?;
  Ok((input,digit))
}

// digit_sequence := digit, (underscore_digit | digit)*
fn digit_sequence(input: ParseString) -> ParseResult<Vec<Token>> {
  let (input, mut start) = digit_token(input)?;
  let (input, mut tokens) = many0(alt((underscore_digit,digit_token)))(input)?;
  let mut all = vec![start];
  all.append(&mut tokens);
  Ok((input,all))
}

// grouping_symbol := left_parenthesis | right_parenthesis | left_angle | right_angle | left_brace | right_brace | left_bracket | right_bracket
pub fn grouping_symbol(input: ParseString) -> ParseResult<Token> {
  let (input, grouping) = alt((left_parenthesis, right_parenthesis, left_angle, right_angle, left_brace, right_brace, left_bracket, right_bracket))(input)?;
  Ok((input, grouping))
}

// punctuation := period | exclamation | question | comma | colon | semicolon | quote | apostrophe ;
pub fn punctuation(input: ParseString) -> ParseResult<Token> {
  let (input, punctuation) = alt((period, exclamation, question, comma, colon, semicolon, quote, apostrophe))(input)?;
  Ok((input, punctuation))
}

// escaped_char := "\" ,  symbol | punctuation ;
pub fn escaped_char(input: ParseString) -> ParseResult<Token> {
  let (input, _) = backslash(input)?;
  let (input, symbol) = alt((symbol, punctuation))(input)?;
  Ok((input, symbol))
}

// symbol := ampersand | bar | at | slash | hashtag | equal | backslash | tilde | plus | dash | asterisk | caret | underscore ;
pub fn symbol(input: ParseString) -> ParseResult<Token> {
  let (input, symbol) = alt((ampersand, bar, at, slash, hashtag, equal, backslash, tilde, plus, dash, asterisk, caret, underscore))(input)?;
  Ok((input, symbol))
}

// text := (alpha | digit | space | tabe | escaped_char | punctuation | grouping_symbol | symbol)+ ;
pub fn text(input: ParseString) -> ParseResult<Token> {
  let (input, text) = alt((alpha_token, digit_token, space, tab, escaped_char, punctuation, grouping_symbol, symbol))(input)?;
  Ok((input, text))
}

// identifier := (alpha | emoji), (alpha | digit | symbol | emoji)* ;
pub fn identifier(input: ParseString) -> ParseResult<Identifier> {
  let (input, (first, mut rest)) = nom_tuple((alt((alpha_token, emoji)), many0(alt((alpha_token, digit_token, symbol, emoji)))))(input)?;
  let mut tokens = vec![first];
  tokens.append(&mut rest);
  let mut merged = merge_tokens(&mut tokens).unwrap();
  merged.kind = TokenKind::Identifier; 
  Ok((input, Identifier{name: merged}))
}

// boolean_literal := true_literal | false_literal ;
pub fn boolean(input: ParseString) -> ParseResult<Token> {
  let (input, boolean) = alt((true_literal, false_literal))(input)?;
  Ok((input, boolean))
}

// true_literal := english_true_literal | check_mark ;
pub fn true_literal(input: ParseString) -> ParseResult<Token> {
  let (input, token) = alt((english_true_literal, check_mark))(input)?;
  Ok((input, token))
}

// false_literal := english_false_literal | cross ;
pub fn false_literal(input: ParseString) -> ParseResult<Token> {
  let (input, token) = alt((english_false_literal, cross))(input)?;
  Ok((input, token))
}

// new_line := new_line_char | carriage_new_line ;
pub fn new_line(input: ParseString) -> ParseResult<Token> {
  let (input, result) = alt((carriage_return_new_line,new_line_char,carriage_return, ))(input)?;
  Ok((input, result))
}

// whitespace := space | new_line | carriage_return | tabe ;
pub fn whitespace(input: ParseString) -> ParseResult<Token> {
  let (input, space) = alt((space,tab,new_line))(input)?;
  Ok((input, space))
}

// whitespace0 := 
pub fn whitespace0(input: ParseString) -> ParseResult<()> {
  let (input, _) = many0(whitespace)(input)?;
  Ok((input, ()))
}

pub fn whitespace1(input: ParseString) -> ParseResult<()> {
  let (input, _) = many1(whitespace)(input)?;
  Ok((input, ()))
}

pub fn space_tab(input: ParseString) -> ParseResult<Token> {
  let (input, space) = alt((space,tab))(input)?;
  Ok((input, space))
}

pub fn list_separator(input: ParseString) -> ParseResult<()> {
  let (input,_) = nom_tuple((whitespace0,tag(","),whitespace0))(input)?;
  Ok((input, ()))
}

pub fn enum_separator(input: ParseString) -> ParseResult<()> {
  let (input,_) = nom_tuple((whitespace0,tag("|"),whitespace0))(input)?;
  Ok((input, ()))
}


// number-literal := (integer | hexadecimal | octal | binary | decimal | float | rational | scientific) ;

pub fn number(input: ParseString) -> ParseResult<Number> {
  let (input, real_num) = real_number(input)?;
  match tag("i")(input.clone()) {
    Ok((input,_)) => {
      return Ok((input, Number::Imaginary(
        ComplexNumber{
          real: None, 
          imaginary: ImaginaryNumber{number: real_num}
        })));
      }
    _ => match nom_tuple((plus,real_number,tag("i")))(input.clone()) {
      Ok((input, (_,imaginary_num,_))) => {
        return Ok((input, Number::Imaginary(
          ComplexNumber{
            real: Some(real_num), 
            imaginary: ImaginaryNumber{number: imaginary_num},
          })));
        }
      _ => ()
    }
  }
  Ok((input, Number::Real(real_num)))
}

pub fn real_number(input: ParseString) -> ParseResult<RealNumber> {
  let (input, neg) = opt(dash)(input)?;
  let (input, result) = alt((hexadecimal_literal, decimal_literal, octal_literal, binary_literal, scientific_literal, rational_literal, float_literal, integer_literal))(input)?;
  let result = match neg {
    Some(_) => RealNumber::Negated(Box::new(result)),
    None => result,
  };
  Ok((input, result))
}

pub fn rational_literal(input: ParseString) -> ParseResult<RealNumber> {
  let (input, RealNumber::Integer(numerator)) = integer_literal(input)? else { unreachable!() };
  let (input, _) = slash(input)?;
  let (input, RealNumber::Integer(denominator)) = integer_literal(input)? else { unreachable!() };
  Ok((input, RealNumber::Rational((numerator,denominator))))
}

pub fn scientific_literal(input: ParseString) -> ParseResult<RealNumber> {
  let (input, base) = match float_literal(input.clone()) {
    Ok((input, RealNumber::Float(base))) => {
      (input, base)
    }
    _ => match integer_literal(input.clone()) {
      Ok((input, RealNumber::Integer(base))) => {
        (input, (base, Token::default()))
      }
      Err(err) => {return Err(err);}
      _ => unreachable!(),
    }
  };
  let (input, _) = alt((tag("e"), tag("E")))(input)?;
  let (input, _) = opt(plus)(input)?;
  let (input, neg) = opt(dash)(input)?;
  let (input, (ex_whole,ex_part)) = match float_literal(input.clone()) {
    Ok((input, RealNumber::Float(exponent))) => {
      (input, exponent)
    }
    _ => match integer_literal(input.clone()) {
      Ok((input, RealNumber::Integer(exponent))) => {
        (input, (exponent, Token::default()))
      }
      Err(err) => {return Err(err);}
      _ => unreachable!(),
    }
  };
  let ex_sign = match neg {
    Some(_) => true,
    None => false,
  };
  Ok((input, RealNumber::Scientific((base,(ex_sign,ex_whole,ex_part)))))
}

// float_decimal_start := ".", digit_sequence ;
fn float_decimal_start(input: ParseString) -> ParseResult<RealNumber> {
  let (input, _) = period(input)?;
  let (input, part) = digit_sequence(input)?;
  let mut tokens2 = part.clone();
  let mut merged = merge_tokens(&mut tokens2).unwrap();
  merged.kind = TokenKind::Number;
  Ok((input, RealNumber::Float((Token::default(),merged))))
}

// float_full := digit_sequence, ".", digit_sequnce ;
fn float_full(input: ParseString) -> ParseResult<RealNumber> {
  let (input, mut whole) = digit_sequence(input)?;
  let (input, _) = period(input)?;
  let (input, mut part) = digit_sequence(input)?;
  let mut whole = merge_tokens(&mut whole).unwrap();
  let mut part = merge_tokens(&mut part).unwrap();
  whole.kind = TokenKind::Number;
  part.kind = TokenKind::Number;
  Ok((input, RealNumber::Float((whole,part))))
}

// float_literal := "."?, digit1, "."?, digit0 ;
pub fn float_literal(input: ParseString) -> ParseResult<RealNumber> {
  let (input, result) = alt((float_decimal_start,float_full))(input)?;
  Ok((input, result))
}

// integer := digit1 ;
pub fn integer_literal(input: ParseString) -> ParseResult<RealNumber> {
  let (input, mut digits) = digit_sequence(input)?;
  let mut merged = merge_tokens(&mut digits).unwrap();
  merged.kind = TokenKind::Number; 
  Ok((input, RealNumber::Integer(merged)))
}

// decimal_literal := "0d", <digit1> ;
pub fn decimal_literal(input: ParseString) -> ParseResult<RealNumber> {
  let msg = "Expects decimal digits after \"0d\"";
  let input = tag("0d")(input);
  let (input, _) = input?;
  let (input, mut tokens) = label!(digit_sequence, msg)(input)?;
  let mut merged = merge_tokens(&mut tokens).unwrap();
  merged.kind = TokenKind::Number; 
  Ok((input, RealNumber::Decimal(merged)))
}

// hexadecimal_literal := "0x", <hex_digit+> ;
pub fn hexadecimal_literal(input: ParseString) -> ParseResult<RealNumber> {
  let msg = "Expects hexadecimal digits after \"0x\"";
  let input = tag("0x")(input);
  let (input, _) = input?;
  let (input, mut tokens) = label!(many1(alt((digit_token,underscore,alpha_token))), msg)(input)?;
  let mut merged = merge_tokens(&mut tokens).unwrap();
  merged.kind = TokenKind::Number; 
  Ok((input, RealNumber::Hexadecimal(merged)))
}

// octal_literal := "0o", <oct_digit+> ;
pub fn octal_literal(input: ParseString) -> ParseResult<RealNumber> {
  let msg = "Expects octal digits after \"0o\"";
  let input = tag("0o")(input);
  let (input, _) = input?;
  let (input, mut tokens) = label!(many1(alt((digit_token,underscore,alpha_token))), msg)(input)?;
  let mut merged = merge_tokens(&mut tokens).unwrap();
  merged.kind = TokenKind::Number; 
  Ok((input, RealNumber::Octal(merged)))
}

// binary_literal := "0b", <bin_digit+> ;
pub fn binary_literal(input: ParseString) -> ParseResult<RealNumber> {
  let msg = "Expects binary digits after \"0b\"";
  let input = tag("0b")(input);
  let (input, _) = input?;
  let (input, mut tokens) = label!(many1(alt((digit_token,underscore,alpha_token))), msg)(input)?;
  let mut merged = merge_tokens(&mut tokens).unwrap();
  merged.kind = TokenKind::Number; 
  Ok((input, RealNumber::Binary(merged)))
}

// empty := underscore+ ;
pub fn empty(input: ParseString) -> ParseResult<Token> {
  let (input, (g, src_range)) = range(many1(tag("_")))(input)?;
  Ok((input, Token{kind: TokenKind::Empty, chars: g.join("").chars().collect(), src_range}))
}

// #### Kind Annotations

// kind_annotation := left_angle, kind, right_angle ;
pub fn kind_annotation(input: ParseString) -> ParseResult<KindAnnotation> {
  let msg2 = "Expects at least one unit in kind annotation";
  let msg3 = "Expects right angle";
  let (input, (_, r)) = range(left_angle)(input)?;
  let (input, kind) = kind(input)?;
  let (input, _) = label!(right_angle, msg3, r)(input)?;
  Ok((input, KindAnnotation{ kind }))
}

// kind := empty | atom | tuple | scalar | bracket | map | brace
pub fn kind(input: ParseString) -> ParseResult<Kind> {
  let (input, kind) = alt((kind_fxn,kind_empty,kind_atom,kind_tuple, kind_scalar, kind_bracket, kind_map, kind_brace))(input)?;
  Ok((input, kind))
}

// kind_empty := underscore* ;
pub fn kind_empty(input: ParseString) -> ParseResult<Kind> {
  let (input, _) = many1(underscore)(input)?;
  Ok((input, Kind::Empty))
}

// kind_atom := "`", identifier ;
pub fn kind_atom(input: ParseString) -> ParseResult<Kind> {
  let (input, _) = grave(input)?;
  let (input, atm) = identifier(input)?;
  Ok((input, Kind::Atom(atm)))
}

// kind_map = "{", kind, ":", kind, "}" ;
pub fn kind_map(input: ParseString) -> ParseResult<Kind> {
  let (input, _) = left_brace(input)?;
  let (input, key_kind) = kind(input)?;
  let (input, _) = colon(input)?;
  let (input, value_kind) = kind(input)?;
  let (input, _) = right_brace(input)?;
  Ok((input, Kind::Map(Box::new(key_kind),Box::new(value_kind))))
}

pub fn kind_fxn(input: ParseString) -> ParseResult<Kind> {
  let (input, _) = left_parenthesis(input)?;
  let (input, input_kinds) = separated_list0(list_separator,kind)(input)?;
  let (input, _) = right_parenthesis(input)?;
  let (input, _) = equal(input)?;
  let (input, _) = left_parenthesis(input)?;
  let (input, output_kinds) = separated_list0(list_separator,kind)(input)?;
  let (input, _) = right_parenthesis(input)?;
  Ok((input, Kind::Function(input_kinds,output_kinds)))
}

// kind_brace = "{", list1(",",kind) "}", [":"], list0(",",literal) ;
pub fn kind_brace(input: ParseString) -> ParseResult<Kind> {
  let (input, _) = left_brace(input)?;
  let (input, kinds) = separated_list1(list_separator,kind)(input)?;
  let (input, _) = right_brace(input)?;
  let (input, _) = opt(colon)(input)?;
  let (input, size) = separated_list0(list_separator,literal)(input)?;
  Ok((input, Kind::Brace((kinds,size))))
}

// kind_bracket = "[", list1(",",kind) "]", [":"], list0(",",literal) ;
pub fn kind_bracket(input: ParseString) -> ParseResult<Kind> {
  let (input, _) = left_bracket(input)?;
  let (input, kinds) = separated_list1(list_separator,kind)(input)?;
  let (input, _) = right_bracket(input)?;
  let (input, _) = opt(colon)(input)?;
  let (input, size) = separated_list0(list_separator,literal)(input)?;
  Ok((input, Kind::Bracket((kinds,size))))
}

// kind_bracket = "(", list1(",",kind) ")" ;
pub fn kind_tuple(input: ParseString) -> ParseResult<Kind> {
  let (input, _) = left_parenthesis(input)?;
  let (input, kinds) = separated_list1(list_separator, kind)(input)?;
  let (input, _) = right_parenthesis(input)?;
  Ok((input, Kind::Tuple(kinds)))
}

// kind_scalar := identifier ;
pub fn kind_scalar(input: ParseString) -> ParseResult<Kind> {
  let (input, kind) = identifier(input)?;
  Ok((input, Kind::Scalar(kind)))
}

// #### Structures


fn max_err<'a>(x: Option<ParseError<'a>>, y: ParseError<'a>) -> ParseError<'a> {
  match (x,&y) {
    (None, y) => y.clone(),
    _ => y.clone(),
  }
}

// structure := empty_table | matrix | table | tuple | tuple_struct | record | map | set ;
pub fn structure(input: ParseString) -> ParseResult<Structure> {
  match empty_set(input.clone()) {
    Ok((input, set)) => {return Ok((input, Structure::Set(set)));},
    _ => (),
  }
  match empty_map(input.clone()) {
    Ok((input, map)) => {return Ok((input, Structure::Map(map)));},
    _ => (),
  }
  match table(input.clone()) {
    Ok((input, tbl)) => {return Ok((input, Structure::Table(tbl)));},
    //Err(Failure(err)) => { return Err(Failure(err)); }, 
    _ => (),
  }
  match matrix(input.clone()) {
    Ok((input, mtrx)) => {return Ok((input, Structure::Matrix(mtrx)));},
    //Err(Failure(err)) => { return Err(Failure(err)); }, 
    _ => (),
  }
  match tuple(input.clone()) {
    Ok((input, tpl)) => {return Ok((input, Structure::Tuple(tpl)));},
    _ => (),
  }
  match tuple_struct(input.clone()) {
    Ok((input, tpl)) => {return Ok((input, Structure::TupleStruct(tpl)));},
    _ => (),
  }
  match record(input.clone()) {
    Ok((input, table)) => {return Ok((input, Structure::Record(table)));},
    _ => (),
  }
  match map(input.clone()) {
    Ok((input, map)) => {return Ok((input, Structure::Map(map)));},
    _ => (),
  }
  match set(input.clone()) {
    Ok((input, set)) => {return Ok((input, Structure::Set(set)));},
    Err(err) => {return Err(err);}
  }
}

// atom := "`", identifier ;
pub fn atom(input: ParseString) -> ParseResult<Atom> {
  let (input, _) = grave(input)?;
  let (input, name) = identifier(input)?;
  Ok((input, Atom{name}))
}

// tuple_struct = atom, "(", expression, ")" ;
pub fn tuple_struct(input: ParseString) -> ParseResult<TupleStruct> {
  let (input, _) = grave(input)?;
  let (input, name) = identifier(input)?;
  let (input, _) = left_parenthesis(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, value) = expression(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, _) = right_parenthesis(input)?;
  Ok((input, TupleStruct{name, value: Box::new(value)}))
}

// binding := identifier, kind_annotation?, <!(space+, colon)>, colon, s+,
// >>          <empty | expression | identifier | value>, <!!right_bracket | (s*, comma, <s+>) | s+> ;
// >> where s := space | new_line | tab ;
pub fn binding(input: ParseString) -> ParseResult<Binding> {
  let msg1 = "Unexpected space before colon ':'";
  let msg2 = "Expects a value";
  let msg3 = "Expects whitespace or comma followed by whitespace";
  let msg4 = "Expects whitespace";
  let (input, _) = whitespace0(input)?;
  let (input, name) = identifier(input)?;
  let (input, kind) = opt(kind_annotation)(input)?;
  let (input, _) = label!(is_not(nom_tuple((many1(space), colon))), msg1)(input)?;
  let (input, _) = colon(input)?;
  let (input, _) = whitespace1(input)?;
  let (input, value) = label!(expression, msg2)(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, _) = opt(comma)(input)?;
  let (input, _) = whitespace0(input)?;
  Ok((input, Binding{name, kind, value}))
}

// table_column := (space | tab)*, (expression | value | data), comma?, (space | tab)* ;
pub fn table_column(input: ParseString) -> ParseResult<TableColumn> {
  let (input, _) = many0(space_tab)(input)?;
  let (input, element) = match expression(input) {
    Ok(result) => result,
    Err(err) => {
      return Err(err);
    }
  };
  let (input, _) = nom_tuple((many0(space_tab),opt(alt((comma,table_separator))), many0(space_tab)))(input)?;
  Ok((input, TableColumn{element}))
}

// matrix_column := (space | tab)*, (expression | value | data), comma?, (space | tab)* ;
pub fn matrix_column(input: ParseString) -> ParseResult<MatrixColumn> {
  let (input, _) = many0(space_tab)(input)?;
  let (input, element) = match expression(input) {
    Ok(result) => result,
    Err(err) => {
      return Err(err);
    }
  };
  let (input, _) = nom_tuple((many0(space_tab),opt(alt((comma,table_separator))), many0(space_tab)))(input)?;
  Ok((input, MatrixColumn{element}))
}


// table_row := (space | tab)*, table_column+, semicolon?, new_line? ;
pub fn table_row(input: ParseString) -> ParseResult<TableRow> {
  let (input, _) = opt(table_separator)(input)?;
  let (input, _) = many0(space_tab)(input)?;
  let (input, columns) = match many1(table_column)(input) {
    Ok(result) => result,
    Err(error) => {
      return Err(error);
    }
  };
  let (input, _) = nom_tuple((opt(semicolon), opt(new_line)))(input)?;
  let (input, _) = opt(nom_tuple((many1(box_drawing_char),new_line)))(input)?;
  Ok((input, TableRow{columns}))
}

// matrix_row := (space | tab)*, table_column+, semicolon?, new_line? ;
pub fn matrix_row(input: ParseString) -> ParseResult<MatrixRow> {
  let (input, _) = opt(table_separator)(input)?;
  let (input, _) = many0(space_tab)(input)?;
  let (input, columns) = match many1(matrix_column)(input) {
    Ok(result) => result,
    Err(error) => {
      return Err(error);
    }
  };
  let (input, _) = nom_tuple((opt(semicolon), opt(new_line)))(input)?;
  let (input, _) = opt(nom_tuple((many1(box_drawing_char),new_line)))(input)?;
  Ok((input, MatrixRow{columns}))
}

// table_header := bar, <attribute+>, <bar>, space*, new_line? ;
pub fn table_header(input: ParseString) -> ParseResult<Vec<Field>> {
  let (input, fields) = separated_list1(many1(space_tab),field)(input)?;
  let (input, _) = many0(space_tab)(input)?;
  let (input, _) = alt((bar,box_vert))(input)?;
  let (input, _) = whitespace0(input)?;
  Ok((input, fields))
}

// field := identifier, kind_annotation ;
pub fn field(input: ParseString) -> ParseResult<Field> {
  let (input, name) = identifier(input)?;
  let (input, kind) = kind_annotation(input)?;
  Ok((input, Field{name, kind}))
}

pub fn box_drawing_char(input: ParseString) -> ParseResult<Token> {
  alt((box_tr_round, box_bl_round, box_vert, box_cross, box_horz, box_t_left, box_t_right, box_t_top, box_t_bottom))(input)
}

pub fn box_drawing_emoji(input: ParseString) -> ParseResult<Token> {
  alt((box_tl_round, box_br_round, box_tr_round, box_bl_round, box_vert, box_cross, box_horz, box_t_left, box_t_right, box_t_top, box_t_bottom))(input)
}

pub fn matrix_start(input: ParseString) -> ParseResult<Token> {
  alt((box_tl_round, left_bracket))(input)
}

pub fn matrix_end(input: ParseString) -> ParseResult<Token> {
  let result = alt((box_br_round, right_bracket))(input);
  result
}

pub fn table_start(input: ParseString) -> ParseResult<Token> {
  alt((box_tl_round, left_brace))(input)
}

pub fn table_end(input: ParseString) -> ParseResult<Token> {
  let result = alt((box_br_round, right_brace))(input);
  result
}

pub fn table_separator(input: ParseString) -> ParseResult<Token> {
  let (input, token) = box_vert(input)?;
  Ok((input, token))
}

// matrix := matrix_start, box_drawing_char*, table_row, box_drawing_char*, matrix_end ;
pub fn matrix(input: ParseString) -> ParseResult<Matrix> {
  let msg = "Expects right bracket ']' to finish the matrix";
  let (input, (_, r)) = range(matrix_start)(input)?;
  let (input, _) = many0(alt((box_drawing_char,whitespace)))(input)?;
  let (input, rows) = many0(matrix_row)(input)?;
  let (input, _) = many0(box_drawing_char)(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, _) = match label!(matrix_end, msg, r)(input) {
    Ok(k) => k,
    Err(err) => {
      return Err(err);
    }
  };
  Ok((input, Matrix{rows}))
}

// table := table_start, box_drawing_char*, table_header, box_drawing_char*, table_row, box_drawing_char*, table_end ;
pub fn table(input: ParseString) -> ParseResult<Table> {
  let msg = "Expects right bracket '}' to finish the table";
  let (input, (_, r)) = range(table_start)(input)?;
  let (input, _) = many0(alt((box_drawing_char,whitespace)))(input)?;
  let (input, header) = table_header(input)?;
  let (input, _) = many0(alt((box_drawing_char,whitespace)))(input)?;
  let (input, rows) = many1(table_row)(input)?;
  let (input, _) = many0(box_drawing_char)(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, _) = match label!(table_end, msg, r)(input) {
    Ok(k) => k,
    Err(err) => {
      return Err(err);
    }
  };
  Ok((input, Table{header,rows}))
}

// empty_table := table_start, empty?, table_end ;
pub fn empty_map(input: ParseString) -> ParseResult<Map> {
  let (input, _) = table_start(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, _) = table_end(input)?;
  Ok((input, Map{elements: vec![]}))
}

pub fn empty_set(input: ParseString) -> ParseResult<Set> {
  let (input, _) = table_start(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, _) = empty(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, _) = table_end(input)?;
  Ok((input,  Set{elements: vec![]}))
}

// record := table_start, binding+, table_end ;
pub fn record(input: ParseString) -> ParseResult<Record> {
  let msg = "Expects right bracket ']' to terminate inline table";
  let (input, (_, r)) = range(table_start)(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, bindings) = many1(binding)(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, _) = label!(table_end, msg, r)(input)?;
  Ok((input, Record{bindings}))
}

// record := "{", mapping*, "}" ;
pub fn map(input: ParseString) -> ParseResult<Map> {
  let msg = "Expects right bracket '}' to terminate inline table";
  let (input, (_, r)) = range(left_brace)(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, elements) = many0(mapping)(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, _) = label!(right_brace, msg, r)(input)?;
  Ok((input, Map{elements}))
}

// mapping := expression, ":", expression
pub fn mapping(input: ParseString) -> ParseResult<Mapping> {
  let msg1 = "Unexpected space before colon ':'";
  let msg2 = "Expects a value";
  let msg3 = "Expects whitespace or comma followed by whitespace";
  let msg4 = "Expects whitespace";
  let (input, _) = whitespace0(input)?;
  let (input, key) = expression(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, _) = colon(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, value) = label!(expression, msg2)(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, _) = opt(comma)(input)?;
  let (input, _) = whitespace0(input)?;
  Ok((input, Mapping{key, value}))
}

// set := "{", list0(",",expression), "}" ;
pub fn set(input: ParseString) -> ParseResult<Set> {
  let msg = "Expects right bracket '}' to terminate inline table";
  let (input, (_, r)) = range(left_brace)(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, elements) = separated_list0(list_separator, expression)(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, _) = label!(right_brace, msg, r)(input)?;
  Ok((input, Set{elements}))
}

// #### State Machines

pub fn define_operator(input: ParseString) -> ParseResult<()> {
  let (input, _) = whitespace0(input)?;
  let (input, _) = tag(":=")(input)?;
  let (input, _) = whitespace0(input)?;
  Ok((input, ()))
}

pub fn output_operator(input: ParseString) -> ParseResult<()> {
  let (input, _) = whitespace0(input)?;
  let (input, _) = tag("=>")(input)?;
  let (input, _) = whitespace0(input)?;
  Ok((input, ()))
}

pub fn async_transition_operator(input: ParseString) -> ParseResult<()> {
  let (input, _) = whitespace0(input)?;
  let (input, _) = tag("~>")(input)?;
  let (input, _) = whitespace0(input)?;
  Ok((input, ()))
}

pub fn transition_operator(input: ParseString) -> ParseResult<()> {
  let (input, _) = whitespace0(input)?;
  let (input, _) = tag("->")(input)?;
  let (input, _) = whitespace0(input)?;
  Ok((input, ()))
}

pub fn guard_operator(input: ParseString) -> ParseResult<()> {
  let (input, _) = whitespace0(input)?;
  let (input, _) = alt((tag("|"),tag("│"),tag("├"),tag("└")))(input)?;
  let (input, _) = whitespace0(input)?;
  Ok((input, ()))
}

pub fn fsm_implementation(input: ParseString) -> ParseResult<FsmImplementation> {
  let ((input, _)) = hashtag(input)?;
  let ((input, name)) = identifier(input)?;
  let ((input, _)) = left_parenthesis(input)?;
  let ((input, input_vars)) = separated_list0(list_separator, identifier)(input)?;
  let ((input, _)) = right_parenthesis(input)?;
  let ((input, _)) = transition_operator(input)?;
  let ((input, start)) = fsm_pattern(input)?;
  let ((input, _)) = whitespace0(input)?;
  let ((input, arms)) = many1(fsm_arm)(input)?;
  let ((input, _)) = period(input)?;
  Ok((input, FsmImplementation{name,input: input_vars,start,arms}))
}

pub fn fsm_arm(input: ParseString) -> ParseResult<FsmArm> {
  let ((input, _)) = many0(comment)(input)?;
  let ((input, start)) = fsm_pattern(input)?;
  let ((input, trns)) = many1(alt((fsm_state_transition,fsm_output,fsm_guard)))(input)?;
  let ((input, _)) = whitespace0(input)?;
  Ok((input, FsmArm{start, transitions: trns}))
}

pub fn fsm_guard(input: ParseString) -> ParseResult<Transition> {
  let (input, _) = alt((transition_operator,guard_operator))(input)?;
  let (input, expr) = match wildcard(input.clone()) {
    Ok((input, _)) => (input, Guard::Wildcard),
    _ => match expression(input.clone()) {
      Ok((input, expr)) => (input, Guard::Expression(expr)),
      Err(err) => {return Err(err);}
    }
  };
  Ok((input, Transition::Guard(expr)))
}

pub fn wildcard(input: ParseString) -> ParseResult<Pattern> {
  let ((input, _)) = asterisk(input)?;
  Ok((input, Pattern::Wildcard))
}

pub fn fsm_state_transition(input: ParseString) -> ParseResult<Transition> {
  let (input, _) = transition_operator(input)?;
  let ((input, ptrn)) = fsm_pattern(input)?;
  Ok((input, Transition::Next(ptrn)))
}

pub fn fsm_async_transition(input: ParseString) -> ParseResult<Transition> {
  let (input, _) = async_transition_operator(input)?;
  let ((input, ptrn)) = fsm_pattern(input)?;
  Ok((input, Transition::Async(ptrn)))
}


pub fn fsm_output(input: ParseString) -> ParseResult<Transition> {
  let (input, _) = output_operator(input)?;
  let ((input, ptrn)) = fsm_pattern(input)?;
  Ok((input, Transition::Output(ptrn)))
}

pub fn fsm_specification(input: ParseString) -> ParseResult<FsmSpecification> {
  let ((input, _)) = hashtag(input)?;
  let ((input, name)) = identifier(input)?;
  let ((input, _)) = left_parenthesis(input)?;
  let ((input, input_vars)) = separated_list0(list_separator, identifier)(input)?;
  let ((input, _)) = right_parenthesis(input)?;
  let ((input, _)) = output_operator(input)?;
  let ((input, output)) = identifier(input)?;
  let ((input, _)) = define_operator(input)?;
  let ((input, states)) = many1(fsm_state_definition)(input)?;
  let ((input, _)) = period(input)?;
  Ok((input, FsmSpecification{name,input: input_vars,output,states}))
}

pub fn fsm_pattern(input: ParseString) -> ParseResult<Pattern> {
  match fsm_tuple_struct(input.clone()) {
    Ok((input, tpl)) => {return Ok((input, Pattern::TupleStruct(tpl)))},
    _ => ()
  }
  match wildcard(input.clone()) {
    Ok((input, _)) => {return Ok((input, Pattern::Wildcard))},
    _ => ()
  }
  match formula(input.clone()) {
    Ok((input, Factor::Expression(expr))) => {return Ok((input, Pattern::Expression(*expr)))},
    Ok((input, frmla)) => {return Ok((input, Pattern::Formula(frmla)))},
    Err(err) => {return Err(err)},
  }
}

pub fn fsm_tuple_struct(input: ParseString) -> ParseResult<PatternTupleStruct> {
  let (input, id) = identifier(input)?;
  let ((input, _)) = left_parenthesis(input)?;
  let ((input, patterns)) = separated_list1(list_separator, fsm_pattern)(input)?;
  let ((input, _)) = right_parenthesis(input)?;
  Ok((input, PatternTupleStruct{name: id, patterns}))
}

pub fn fsm_state_definition(input: ParseString) -> ParseResult<StateDefinition> {
  let ((input, _)) = guard_operator(input)?;
  let ((input, name)) = identifier(input)?;
  let ((input, vars)) = opt(fsm_state_definition_variables)(input)?;
  Ok((input, StateDefinition{name,state_variables: vars}))
}

pub fn fsm_state_definition_variables(input: ParseString) -> ParseResult<Vec<Identifier>> {
  let ((input, _)) = left_parenthesis(input)?;
  let ((input, names)) = separated_list1(list_separator, identifier)(input)?;
  let ((input, _)) = right_parenthesis(input)?;
  Ok((input, names))
}

pub fn fsm_pipe(input: ParseString) -> ParseResult<FsmPipe> {
  let ((input, start)) = fsm_instance(input)?;
  let ((input, trns)) = many0(alt((fsm_state_transition,fsm_async_transition,fsm_output,fsm_guard)))(input)?;
  Ok((input, FsmPipe{start, transitions: trns}))
}

fn fsm_declare(input: ParseString) -> ParseResult<FsmDeclare> {
  let (input, fsm) = fsm(input)?;
  let (input, _) = define_operator(input)?;
  let (input, pipe) = fsm_pipe(input)?;
  Ok((input, FsmDeclare{fsm,pipe}))
}
  
fn fsm(input: ParseString) -> ParseResult<Fsm> {
  let ((input, _)) = hashtag(input)?;
  let ((input, name)) = identifier(input)?;
  let ((input, args)) = opt(argument_list)(input)?;
  let ((input, kind)) = opt(kind_annotation)(input)?;
  Ok((input, Fsm{ name, args, kind }))
}

fn fsm_instance(input: ParseString) -> ParseResult<FsmInstance> {
  let ((input, _)) = hashtag(input)?;
  let (input, name) = identifier(input)?;
  let (input, args) = opt(fsm_args)(input)?;
  Ok((input, FsmInstance{name,args} ))
}

fn fsm_args(input: ParseString) -> ParseResult<Vec<(Option<Identifier>,Expression)>> {
  let (input, _) = left_parenthesis(input)?;
  let (input, args) = separated_list0(list_separator, alt((call_arg_with_binding,call_arg)))(input)?;
  let (input, _) = right_parenthesis(input)?;
  Ok((input, args))
}

// #### Statements

// comment_sigil := "--" | "//" | "/*" ;
pub fn comment_sigil(input: ParseString) -> ParseResult<()> {
  let (input, _) = alt((tag("--"),tag("//"),tag("/*")))(input)?;
  Ok((input, ()))
}

// comment := ws0, comment_sigil, text+ ;
pub fn comment(input: ParseString) -> ParseResult<Comment> {
  let msg2 = "Character not allowed in comment text";
  let (input, _) = whitespace0(input)?;
  let (input, _) = comment_sigil(input)?;
  let (input, text) = many1(text)(input)?;
  let (input, _) = whitespace0(input)?;
  Ok((input, Comment{text}))
}

// assign_operator := "=" ;
pub fn assign_operator(input: ParseString) -> ParseResult<()> {
  let (input, _) = whitespace0(input)?;
  let (input, _) = tag("=")(input)?;
  let (input, _) = whitespace0(input)?;
  Ok((input, ()))
}

// split_data := (identifier | table), <!stmt_operator>, space*, split_operator, <space+>, <expression> ;
pub fn split_data(input: ParseString) -> ParseResult<ParserNode> {
  /*let msg1 = "Expects spaces around operator";
  let msg2 = "Expects expression";
  let (input, table) = alt((identifier, table))(input)?;
  let (input, _) = labelr!(null(is_not(stmt_operator)), skip_nil, msg1)(input)?;
  let (input, _) = many0(space)(input)?;
  let (input, _) = split_operator(input)?;
  let (input, _) = labelr!(null(many1(space)), skip_nil, msg1)(input)?;
  let (input, expression) = label!(expression, msg2)(input)?;*/
  Ok((input, ParserNode::SplitData{children: vec![]}))
}

// flatten_data := identifier, <!stmt_operator>, space*, flatten_operator, <space+>, <expression> ;
pub fn flatten_data(input: ParseString) -> ParseResult<ParserNode> {
  /*let msg1 = "Expects spaces around operator";
  let msg2 = "Expects expression";
  let (input, table) = identifier(input)?;
  let (input, _) = labelr!(null(is_not(stmt_operator)), skip_nil, msg1)(input)?;
  let (input, _) = many0(space)(input)?;
  let (input, _) = flatten_operator(input)?;
  let (input, _) = labelr!(null(many1(space)), skip_nil, msg1)(input)?;
  let (input, expression) = label!(expression, msg2)(input)?;*/
  Ok((input, ParserNode::FlattenData{children: vec![]}))
}

// variable_define := identifier, define_operator, expression ;
pub fn variable_define(input: ParseString) -> ParseResult<VariableDefine> {
  let msg1 = "Expects spaces around operator";
  let msg2 = "Expects expression";
  let (input, var) = var(input)?;
  let (input, _) = labelr!(null(is_not(assign_operator)), skip_nil, msg1)(input)?;
  let (input, _) = define_operator(input)?;
  let (input, expression) = label!(expression, msg2)(input)?;
  Ok((input, VariableDefine{var,expression}))
}

// variable_define := identifier, assign_operator, expression ;
pub fn variable_assign(input: ParseString) -> ParseResult<VariableAssign> {
  let msg1 = "Expects spaces around operator";
  let msg2 = "Expects expression";
  let (input, target) = expression(input)?;
  let (input, _) = labelr!(null(is_not(define_operator)), skip_nil, msg1)(input)?;
  let (input, _) = assign_operator(input)?;
  let (input, expression) = label!(expression, msg2)(input)?;
  Ok((input, VariableAssign{target,expression}))
}

// parser for the second line of the output table, generate the 
// var name if there is one.

// split_operator := ">-" ;
pub fn split_operator(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag(">-")(input)?;
  Ok((input, ParserNode::Null))
}

// flatten_operator := "-<" ;
pub fn flatten_operator(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag("-<")(input)?;
  Ok((input, ParserNode::Null))
}

// statement := variable_define | variable_assign | enum_define | fm_declare | kind_define ;
pub fn statement(input: ParseString) -> ParseResult<Statement> {
  match variable_define(input.clone()) {
    Ok((input, var_def)) => { return Ok((input, Statement::VariableDefine(var_def))); },
    //Err(Failure(err)) => {return Err(Failure(err))},
    _ => (),
  }
  match variable_assign(input.clone()) {
    Ok((input, var_asgn)) => { return Ok((input, Statement::VariableAssign(var_asgn))); },
    //Err(Failure(err)) => {return Err(Failure(err))},
    _ => (),
  }
  match enum_define(input.clone()) {
    Ok((input, enm_def)) => { return Ok((input, Statement::EnumDefine(enm_def))); },
    //Err(Failure(err)) => {return Err(Failure(err))},
    _ => (),
  }
  match fsm_declare(input.clone()) {
    Ok((input, var_def)) => { return Ok((input, Statement::FsmDeclare(var_def))); },
    //Err(Failure(err)) => {return Err(Failure(err))},
    _ => (),
  }
  match kind_define(input.clone()) {
    Ok((input, knd_def)) => { return Ok((input, Statement::KindDefine(knd_def))); },
    Err(err) => { return Err(err); },
  }
}

// enum_define := "<", identifier, ">", define_operator, list1(enum_separator, enum_variant);
pub fn enum_define(input: ParseString) -> ParseResult<EnumDefine> {
  let (input, _) = left_angle(input)?;
  let (input, name) = identifier(input)?;
  let (input, _) = right_angle(input)?;
  let (input, _) = define_operator(input)?;
  let (input, variants) = separated_list1(enum_separator, enum_variant)(input)?;
  Ok((input, EnumDefine{name, variants}))
}

// enum_variant := atom | identifier, enum_variant_kind? ;
pub fn enum_variant(input: ParseString) -> ParseResult<EnumVariant> {
  let (input, _) = opt(grave)(input)?;
  let (input, name) = identifier(input)?;
  let (input, value) = opt(enum_variant_kind)(input)?;
  Ok((input, EnumVariant{name, value}))
}

// enum_variant_kind := "(", kind_annotation, ")" ;
pub fn enum_variant_kind(input: ParseString) -> ParseResult<KindAnnotation> {
  let (input, _) = left_parenthesis(input)?;
  let (input, annotation) = kind_annotation(input)?;
  let (input, _) = right_parenthesis(input)?;
  Ok((input, annotation))
}

// kind_define := "<", identifier, ">", define_operator, kind_annotation ;
pub fn kind_define(input: ParseString) -> ParseResult<KindDefine> {
  let (input, _) = left_angle(input)?;
  let (input, name) = identifier(input)?;
  let (input, _) = right_angle(input)?;
  let (input, _) = define_operator(input)?;
  let (input, knd) = kind_annotation(input)?;
  Ok((input, KindDefine{name,definition: knd}))
}

// #### Expressions

// ##### Math expressions

// parenthetical_expression := left_parenthesis, formula, right_parenthesis ;
pub fn parenthetical_term(input: ParseString) -> ParseResult<Factor> {
  let msg1 = "Expects expression";
  let msg2 = "Expects right parenthesis ')'";
  let (input, (_, r)) = range(left_parenthesis)(input)?;
  let (input, frmla) = label!(formula, msg1)(input)?;
  let (input, _) = label!(right_parenthesis, msg2, r)(input)?;
  Ok((input, frmla))
}

pub fn negated_factor(input: ParseString) -> ParseResult<Factor> {
  let (input, _) = dash(input)?;
  let (input, expr) = factor(input)?;
  Ok((input, Factor::Negated(Box::new(expr))))
}

// add := "+" ;
pub fn add(input: ParseString) -> ParseResult<AddSubOp> {
  let (input, _) = whitespace1(input)?;
  let (input, _) = tag("+")(input)?;
  let (input, _) = whitespace1(input)?;
  Ok((input, AddSubOp::Add))
}

// subtract := "-" ;
pub fn subtract(input: ParseString) -> ParseResult<AddSubOp> {
  let (input, _) = whitespace1(input)?;
  let (input, _) = tag("-")(input)?;
  let (input, _) = whitespace1(input)?;
  Ok((input, AddSubOp::Sub))
}

// multiply := "*" ;
pub fn multiply(input: ParseString) -> ParseResult<MulDivOp> {
  let (input, _) = whitespace1(input)?;
  let (input, _) = tag("*")(input)?;
  let (input, _) = whitespace1(input)?;
  Ok((input, MulDivOp::Mul))
}

// divide := "/" ;
pub fn divide(input: ParseString) -> ParseResult<MulDivOp> {
  let (input, _) = whitespace1(input)?;
  let (input, _) = tag("/")(input)?;
  let (input, _) = whitespace1(input)?;
  Ok((input, MulDivOp::Div))
}

// matrix_multiply := "**" ;
pub fn matrix_multiply(input: ParseString) -> ParseResult<VecOp> {
  let (input, _) = whitespace1(input)?;
  let (input, _) = tag("**")(input)?;
  let (input, _) = whitespace1(input)?;
  Ok((input, VecOp::MatMul))
}

// matrix_solve := "\" ;
pub fn matrix_solve(input: ParseString) -> ParseResult<VecOp> {
  let (input, _) = whitespace1(input)?;
  let (input, _) = tag("\\")(input)?;
  let (input, _) = whitespace1(input)?;
  Ok((input, VecOp::Solve))
}

// dot_product := "·" ;
pub fn dot_product(input: ParseString) -> ParseResult<VecOp> {
  let (input, _) = whitespace1(input)?;
  let (input, _) = tag("·")(input)?;
  let (input, _) = whitespace1(input)?;
  Ok((input, VecOp::Dot))
}

// cross_product := "⨯" ;
pub fn cross_product(input: ParseString) -> ParseResult<VecOp> {
  let (input, _) = whitespace1(input)?;
  let (input, _) = tag("⨯")(input)?;
  let (input, _) = whitespace1(input)?;
  Ok((input, VecOp::Cross))
}

// exponent := "^" ;
pub fn exponent(input: ParseString) -> ParseResult<ExponentOp> {
  let (input, _) = whitespace1(input)?;
  let (input, _) = tag("^")(input)?;
  let (input, _) = whitespace1(input)?;
  Ok((input, ExponentOp::Exp))
}

// range_inclusive := "..=" ;
pub fn range_inclusive(input: ParseString) -> ParseResult<RangeOp> {
  let (input, _) = tag("..=")(input)?;
  Ok((input, RangeOp::Inclusive))
}

// range_exclusive := ".." ;
pub fn range_exclusive(input: ParseString) -> ParseResult<RangeOp> {
  let (input, _) = tag("..")(input)?;
  Ok((input, RangeOp::Exclusive))
}

// range_operator := range_inclusive | range_exclusive ;
pub fn range_operator(input: ParseString) -> ParseResult<RangeOp> {
  let (input, op) = alt((range_inclusive,range_exclusive))(input)?;
  Ok((input, op))
}

// formula := l1, (range_operator, l1)* ;
pub fn formula(input: ParseString) -> ParseResult<Factor> {
  let (input, factor) = l1(input)?;
  Ok((input, factor))
}

// add_sub_operator := add | subtract ;
pub fn add_sub_operator(input: ParseString) -> ParseResult<FormulaOperator> {
  let (input, op) = alt((add, subtract))(input)?;
  Ok((input, FormulaOperator::AddSub(op)))
}

// l1 := l2, (add_sub_operator, l2)* ;
pub fn l1(input: ParseString) -> ParseResult<Factor> {
  let (input, lhs) = l2(input)?;
  let (input, rhs) = many0(nom_tuple((add_sub_operator,l2)))(input)?;
  let factor = if rhs.is_empty() { lhs } else { Factor::Term(Box::new(Term { lhs, rhs })) };
  Ok((input, factor))
}

// mul_div_operator := matrix_multiply | multiply | divide | matrix_solve ;
pub fn mul_div_operator(input: ParseString) -> ParseResult<FormulaOperator> {
  let (input, op) = alt((multiply, divide))(input)?;
  Ok((input, FormulaOperator::MulDiv(op)))
}

// mul_div_operator := matrix_multiply | multiply | divide | matrix_solve ;
pub fn vec_operator(input: ParseString) -> ParseResult<FormulaOperator> {
  let (input, op) = alt((matrix_multiply, matrix_solve, dot_product, cross_product))(input)?;
  Ok((input, FormulaOperator::Vec(op)))
}

// l2 := l3, (mul_div_operator, l3)* ;
pub fn l2(input: ParseString) -> ParseResult<Factor> {
  let (input, lhs) = l3(input)?;
  let (input, rhs) = many0(nom_tuple((alt((mul_div_operator, vec_operator)),l3)))(input)?;
  let factor = if rhs.is_empty() { lhs } else { Factor::Term(Box::new(Term { lhs, rhs })) };
  Ok((input, factor))
}

// exponent_operator := exponent ;
pub fn exponent_operator(input: ParseString) -> ParseResult<FormulaOperator> {
  let (input, op) = exponent(input)?;
  Ok((input, FormulaOperator::Exponent(op)))
}

// l3 := l4, (exponent_operator, l4)* ;
pub fn l3(input: ParseString) -> ParseResult<Factor> {
  let (input, lhs) = l4(input)?;
  let (input, rhs) = many0(nom_tuple((exponent_operator,l4)))(input)?;
  let factor = if rhs.is_empty() { lhs } else { Factor::Term(Box::new(Term { lhs, rhs })) };
  Ok((input, factor))
}

// logic_operator := and | or | xor ;
pub fn logic_operator(input: ParseString) -> ParseResult<FormulaOperator> {
  let (input, op) = alt((and, or, xor))(input)?;
  Ok((input, FormulaOperator::Logic(op)))
}

// l4 := l5, (logic_operator, l5)* ;
pub fn l4(input: ParseString) -> ParseResult<Factor> {
  let (input, lhs) = l5(input)?;
  let (input, rhs) = many0(nom_tuple((logic_operator,l5)))(input)?;
  let factor = if rhs.is_empty() { lhs } else { Factor::Term(Box::new(Term { lhs, rhs })) };
  Ok((input, factor))
}

// l5 := factor, (comparison_operator, factor)* ;
pub fn l5(input: ParseString) -> ParseResult<Factor> {
  let (input, lhs) = factor(input)?;
  let (input, rhs) = many0(nom_tuple((comparison_operator,factor)))(input)?;
  let factor = if rhs.is_empty() { lhs } else { Factor::Term(Box::new(Term { lhs, rhs })) };
  Ok((input, factor))
}

// comparison_operator := not_equal | equal_to | greater_than_equal | greater_than | less_than_equal | less_than ;
pub fn comparison_operator(input: ParseString) -> ParseResult<FormulaOperator> {
  let (input, op) = alt((not_equal, equal_to, greater_than_equal, greater_than, less_than_equal, less_than))(input)?;
  Ok((input, FormulaOperator::Comparison(op)))
}

// factor := parenthetical_term | structure | fsm_pipe | function_call | literal | slice | var ;
pub fn factor(input: ParseString) -> ParseResult<Factor> {
  let (input, fctr) = match parenthetical_term(input.clone()) {
    Ok((input, term)) => (input, term),
    Err(_) => match negated_factor(input.clone()) {
      Ok((input, neg)) => (input, neg),
      Err(_) => match structure(input.clone()) {
        Ok((input, strct)) => (input, Factor::Expression(Box::new(Expression::Structure(strct)))),
        Err(_) => match fsm_pipe(input.clone()) {
          Ok((input, pipe)) => (input, Factor::Expression(Box::new(Expression::FsmPipe(pipe)))),
          Err(_) => match function_call(input.clone()) {
            Ok((input, fxn)) => (input, Factor::Expression(Box::new(Expression::FunctionCall(fxn)))),
            Err(_) => match literal(input.clone()) {
              Ok((input, ltrl)) => (input, Factor::Expression(Box::new(Expression::Literal(ltrl)))),
              Err(_) => match slice(input.clone()) {
                Ok((input, slc)) => (input, Factor::Expression(Box::new(Expression::Slice(slc)))),
                Err(_) => match var(input.clone()) {
                  Ok((input, var)) => (input, Factor::Expression(Box::new(Expression::Var(var)))),
                  Err(err) => { return Err(err); },
                },
              },
            },
          },
        },
      },
    },
  };
  let (input, transpose) = opt(transpose)(input)?;
  let fctr = match transpose {
    Some(_) => Factor::Transpose(Box::new(fctr)),
    None => fctr,
  };
  Ok((input, fctr))
}
// statement_separator := ";" ;
pub fn statement_separator(input: ParseString) -> ParseResult<()> {
  let (input,_) = nom_tuple((whitespace0,semicolon,whitespace0))(input)?;
  Ok((input, ()))
}

pub fn function_define(input: ParseString) -> ParseResult<FunctionDefine> {
  let ((input, name)) = identifier(input)?;
  let ((input, _)) = left_parenthesis(input)?;
  let ((input, input_args)) = separated_list0(list_separator, function_arg)(input)?;
  let ((input, _)) = right_parenthesis(input)?;
  let ((input, _)) = whitespace0(input)?;
  let ((input, _)) = equal(input)?;
  let ((input, _)) = whitespace0(input)?;
  let ((input, output)) = alt((function_out_args,function_out_arg))(input)?;
  let ((input, _)) = define_operator(input)?;
  let ((input, statements)) = separated_list1(alt((whitespace1,statement_separator)), statement)(input)?;
  let ((input, _)) = period(input)?;
  Ok((input,FunctionDefine{name,input: input_args,output,statements}))
}

fn function_out_args(input: ParseString) -> ParseResult<Vec<FunctionArgument>> {
  let ((input, _)) = left_parenthesis(input)?;
  let ((input, args)) = separated_list1(list_separator,function_arg)(input)?;
  let ((input, _)) = right_parenthesis(input)?;
  Ok((input, args))
}

fn function_out_arg(input: ParseString) -> ParseResult<Vec<FunctionArgument>> {
  let ((input, arg)) = function_arg(input)?;
  Ok((input, vec![arg]))
}

// function_arg := identifier, kind_annotation ;
fn function_arg(input: ParseString) -> ParseResult<FunctionArgument> {
  let ((input, name)) = identifier(input)?;
  let ((input, kind)) = kind_annotation(input)?;
  Ok((input, FunctionArgument{ name, kind }))
}

// argument_list := "(", list0(",", call_arg_with_biding | call_arg)
fn argument_list(input: ParseString) -> ParseResult<ArgumentList> {
  let (input, _) = left_parenthesis(input)?;
  let (input, args) = separated_list0(list_separator, alt((call_arg_with_binding,call_arg)))(input)?;
  let (input, _) = right_parenthesis(input)?;
  Ok((input, args))
}

// function_call := identifier, argument_list
fn function_call(input: ParseString) -> ParseResult<FunctionCall> {
  let (input, name) = identifier(input)?;
  let (input, args) = argument_list(input)?;
  Ok((input, FunctionCall{name,args} ))
}

// call_arg_with_binding := identifier, colon, expression ;
fn call_arg_with_binding(input: ParseString) -> ParseResult<(Option<Identifier>,Expression)> {
  let (input, arg_name) = identifier(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, _) = colon(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, expr) = expression(input)?;
  Ok((input, (Some(arg_name), expr)))
}

// call_arg := expression ;
fn call_arg(input: ParseString) -> ParseResult<(Option<Identifier>,Expression)> {
  let (input, expr) = expression(input)?;
  Ok((input, (None, expr)))
}

// var := identifier, kind_annotation? ;
fn var(input: ParseString) -> ParseResult<Var> {
  let ((input, name)) = identifier(input)?;
  let ((input, kind)) = opt(kind_annotation)(input)?;
  Ok((input, Var{ name, kind }))
}

// ##### Filter expressions

// not_equal := "!=" | "¬=" | "≠" ;
pub fn not_equal(input: ParseString) -> ParseResult<ComparisonOp> {
  let (input, _) = whitespace1(input)?;
  let (input, _) = alt((tag("!="),tag("¬="),tag("≠")))(input)?;
  let (input, _) = whitespace1(input)?;
  Ok((input, ComparisonOp::NotEqual))
}

// equal_to := "==" ;
pub fn equal_to(input: ParseString) -> ParseResult<ComparisonOp> {
  let (input, _) = whitespace1(input)?;
  let (input, _) = tag("==")(input)?;
  let (input, _) = whitespace1(input)?;
  Ok((input, ComparisonOp::Equal))
}

// greater_than := ">" ;
pub fn greater_than(input: ParseString) -> ParseResult<ComparisonOp> {
  let (input, _) = whitespace1(input)?;
  let (input, _) = tag(">")(input)?;
  let (input, _) = whitespace1(input)?;
  Ok((input, ComparisonOp::GreaterThan))
}

// less_than := "<" ;
pub fn less_than(input: ParseString) -> ParseResult<ComparisonOp> {
  let (input, _) = whitespace1(input)?;
  let (input, _) = tag("<")(input)?;
  let (input, _) = whitespace1(input)?;
  Ok((input, ComparisonOp::LessThan))
}

// greater_than_equal := ">=" | "≥" ;
pub fn greater_than_equal(input: ParseString) -> ParseResult<ComparisonOp> {
  let (input, _) = whitespace1(input)?;
  let (input, _) = alt((tag(">="),tag("≥")))(input)?;
  let (input, _) = whitespace1(input)?;
  Ok((input, ComparisonOp::GreaterThanEqual))
}

// less_than_equal := "<=" | "≤" ;
pub fn less_than_equal(input: ParseString) -> ParseResult<ComparisonOp> {
  let (input, _) = whitespace1(input)?;
  let (input, _) = alt((tag("<="),tag("≤")))(input)?;
  let (input, _) = whitespace1(input)?;
  Ok((input, ComparisonOp::LessThanEqual))
}

// ##### Logic expressions

// or := "|" ;
pub fn or(input: ParseString) -> ParseResult<LogicOp> {
  let (input, _) = whitespace1(input)?;
  let (input, _) = tag("|")(input)?;
  let (input, _) = whitespace1(input)?;
  Ok((input, LogicOp::Or))
}

// and := "&" ;
pub fn and(input: ParseString) -> ParseResult<LogicOp> {
  let (input, _) = whitespace1(input)?;
  let (input, _) = tag("&")(input)?;
  let (input, _) = whitespace1(input)?;
  Ok((input, LogicOp::And))
}

// not := "!" | "¬" ;
pub fn not(input: ParseString) -> ParseResult<LogicOp> {
  let (input, _) = whitespace1(input)?;
  let (input, _) = alt((tag("!"), tag("¬")))(input)?;
  let (input, _) = whitespace1(input)?;
  Ok((input, LogicOp::Not))
}

// xor := "xor" | "⊕" | "⊻" ;
pub fn xor(input: ParseString) -> ParseResult<LogicOp> {
  let (input, _) = whitespace1(input)?;
  let (input, _) = alt((tag("xor"), tag("⊕"), tag("⊻")))(input)?;
  let (input, _) = whitespace1(input)?;
  Ok((input, LogicOp::Xor))
}

// ##### Other expressions

// string := quote, (!quote, <text>)*, quote ;
pub fn string(input: ParseString) -> ParseResult<MechString> {
  let msg = "Character not allowed in string";
  let (input, _) = quote(input)?;
  let (input, matched) = many0(nom_tuple((is_not(quote), label!(text, msg))))(input)?;
  let (input, _) = quote(input)?;
  let (_, mut text): ((), Vec<_>) = matched.into_iter().unzip();
  let mut merged = merge_tokens(&mut text).unwrap();
  merged.kind = TokenKind::String;
  Ok((input, MechString { text: merged }))
}

// transpose := "'" ;
pub fn transpose(input: ParseString) -> ParseResult<()> {
  let (input, _) = tag("'")(input)?;
  Ok((input, ()))
}


// literal := number | string | atom | boolean | empty, kind_annotation? ;
pub fn literal(input: ParseString) -> ParseResult<Literal> {
  let (input, result) = match number(input.clone()) {
    Ok((input, num)) => (input, Literal::Number(num)),
    _ => match string(input.clone()) {
      Ok((input, s)) => (input, Literal::String(s)),
      _ => match atom(input.clone()) {
        Ok((input, atm)) => (input, Literal::Atom(atm)),
        _ => match boolean(input.clone()) {
          Ok((input, boolean)) => (input, Literal::Boolean(boolean)),
          _ => match empty(input.clone()) {
            Ok((input, empty)) => (input, Literal::Empty(empty)), 
            Err(err) => {return Err(err);}
          }
        }
      }
    }
  };
  let (input, result) = match opt(kind_annotation)(input.clone()) {
    Ok((input, Some(knd))) => ((input, Literal::TypedLiteral((Box::new(result),knd)))),
    Ok((input, None)) => (input,result),
    Err(err) => {return Err(err);}
  };
  Ok((input, result))
}

// slice := identifier, subscript ;
fn slice(input: ParseString) -> ParseResult<Slice> {
  let (input, name) = identifier(input)?;
  let (input, ixes) = subscript(input)?;
  Ok((input, Slice{name, subscript: ixes}))
}

// subscript := (swizzle_subscript | dot_subscript | bracket_subscript | brace_subscript)+ ; 
fn subscript(input: ParseString) -> ParseResult<Vec<Subscript>> {
  let (input, subscripts) = many1(alt((swizzle_subscript,dot_subscript,bracket_subscript,brace_subscript)))(input)?;
  Ok((input, subscripts))
}

// swizzle_subscript := ".", identifier, "," , list1(",", identifier) ;
fn swizzle_subscript(input: ParseString) -> ParseResult<Subscript> {
  let (input, _) = period(input)?;
  let (input, first) = identifier(input)?;
  let (input, _) = comma(input)?;
  let (input, mut name) = separated_list1(tag(","),identifier)(input)?;
  let mut subscripts = vec![first];
  subscripts.append(&mut name);
  Ok((input, Subscript::Swizzle(subscripts)))
}

// dot_subscript := ".", identifier
fn dot_subscript(input: ParseString) -> ParseResult<Subscript> {
  let (input, _) = period(input)?;
  let (input, name) = identifier(input)?;
  Ok((input, Subscript::Dot(name)))
}

// bracket_subscript := "[", list1(",", select_all | formula_subscript) "]" ;
fn bracket_subscript(input: ParseString) -> ParseResult<Subscript> {
  let (input, _) = left_bracket(input)?;
  let (input, subscripts) = separated_list1(list_separator,alt((select_all,range_subscript,formula_subscript)))(input)?;
  let (input, _) = right_bracket(input)?;
  Ok((input, Subscript::Bracket(subscripts)))
}

// brace_subscript := "{", list1(",", select_all | formula_subscript) "}" ;
fn brace_subscript(input: ParseString) -> ParseResult<Subscript> {
  let (input, _) = left_brace(input)?;
  let (input, subscripts) = separated_list1(list_separator,alt((select_all,formula_subscript)))(input)?;
  let (input, _) = right_brace(input)?;
  Ok((input, Subscript::Brace(subscripts)))
}

// select_all := ":" ;
pub fn select_all(input: ParseString) -> ParseResult<Subscript> {
  let (input, lhs) = colon(input)?;
  Ok((input, Subscript::All))
}

// formula_subscript := formula ;
pub fn formula_subscript(input: ParseString) -> ParseResult<Subscript> {
  let (input, factor) = l1(input)?;
  Ok((input, Subscript::Formula(factor)))
}

// formula_subscript := formula ;
pub fn range_subscript(input: ParseString) -> ParseResult<Subscript> {
  let (input, rng) = range_expression(input)?;
  Ok((input, Subscript::Range(rng)))
}

// range
pub fn range_expression(input: ParseString) -> ParseResult<RangeExpression> {
  let (input, start) = formula(input)?;
  let (input, op) = range_operator(input)?;
  let (input, x) = formula(input)?;
  let (input, y) = opt(nom_tuple((range_operator,formula)))(input)?;
  let range = match y {
    Some((op2,terminal)) => RangeExpression{start, increment: Some((op,x)), operator: op2, terminal},
    None => RangeExpression{start, increment: None, operator: op, terminal: x},
  };
  Ok((input, range))
}

// tuple := "(", list0(",", expression), ")" ;
pub fn tuple(input: ParseString) -> ParseResult<Tuple> {
  let (input, _) = left_parenthesis(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, exprs) = separated_list0(list_separator, expression)(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, _) = right_parenthesis(input)?;
  Ok((input, Tuple{elements: exprs}))
}

// expression := formula, transpose? ;
pub fn expression(input: ParseString) -> ParseResult<Expression> {
  let (input, expr) = match range_expression(input.clone()) {
    Ok((input, rng)) => (input, Expression::Range(Box::new(rng))),
    Err(_) => match formula(input.clone()) {
      Ok((input, Factor::Expression(expr))) => (input, *expr),
      Ok((input, fctr)) => (input, Expression::Formula(fctr)),
      Err(err) => {return Err(err);},
    } 
  };
  Ok((input, expr))
}

// ### Mechdown

// title := text+, new_line, equal+, (space|tab)*, whitespace* ;
pub fn title(input: ParseString) -> ParseResult<Title> {
  let (input, mut text) = many1(text)(input)?;
  let (input, _) = new_line(input)?;
  let (input, _) = many1(equal)(input)?;
  let (input, _) = many0(space_tab)(input)?;
  let (input, _) = new_line(input)?;
  let (input, _) = many0(space_tab)(input)?;
  let (input, _) = whitespace0(input)?;
  let mut title = merge_tokens(&mut text).unwrap();
  title.kind = TokenKind::Title;
  Ok((input, Title{text: title}))
}

// subtitle := text+, new_line, dash+, (space|tab)*, whitespace* ;
pub fn ul_subtitle(input: ParseString) -> ParseResult<Subtitle> {
  let (input, _) = many1(digit_token)(input)?;
  let (input, _) = period(input)?;
  let (input, _) = many0(space)(input)?;
  let (input, mut text) = many1(text)(input)?;
  let (input, _) = new_line(input)?;
  let (input, _) = many1(dash)(input)?;
  let (input, _) = many0(space_tab)(input)?;
  let (input, _) = new_line(input)?;
  let (input, _) = many0(space_tab)(input)?;
  let (input, _) = whitespace0(input)?;
  let mut title = merge_tokens(&mut text).unwrap();
  title.kind = TokenKind::Title;
  Ok((input, Subtitle{text: title}))
}

// number_subtitle := space*, number, period, space+, text, space*, new_line* ;
pub fn number_subtitle(input: ParseString) -> ParseResult<Subtitle> {
  let (input, _) = many0(space_tab)(input)?;
  let (input, _) = left_parenthesis(input)?;
  let (input, _) = integer_literal(input)?;
  let (input, _) = right_parenthesis(input)?;
  let (input, _) = many1(space_tab)(input)?;
  let (input, mut text) = many1(text)(input)?;
  let (input, _) = many0(space_tab)(input)?;
  let (input, _) = whitespace0(input)?;
  let mut title = merge_tokens(&mut text).unwrap();
  title.kind = TokenKind::Title;
  Ok((input, Subtitle{text: title}))
}

// alpha_subtitle := space*, alpha, right_parenthesis, space+, text, space*, new_line* ;
pub fn alpha_subtitle(input: ParseString) -> ParseResult<Subtitle> {
  let (input, _) = many0(space_tab)(input)?;
  let (input, _) = left_parenthesis(input)?;
  let (input, _) = alpha(input)?;
  let (input, _) = right_parenthesis(input)?;
  let (input, _) = many0(space_tab)(input)?;
  let (input, mut text) = many1(text)(input)?;
  let (input, _) = many0(space_tab)(input)?;
  let (input, _) = whitespace0(input)?;
  let mut title = merge_tokens(&mut text).unwrap();
  title.kind = TokenKind::Title;
  Ok((input, Subtitle{text: title}))
}

// paragraph_symbol := ampersand | at | slash | backslash | asterisk | caret | hashtag | underscore ;
pub fn paragraph_symbol(input: ParseString) -> ParseResult<Token> {
  let (input, symbol) = alt((ampersand, at, slash, backslash, asterisk, caret, hashtag, underscore, equal, tilde, plus, percent))(input)?;
  Ok((input, symbol))
}

// paragraph_starter := (word | number | quote | left_angle | right_angle | left_bracket | right_bracket | period | exclamation | question | comma | colon | semicolon | left_parenthesis | right_parenthesis | emoji)+ ;
pub fn paragraph_starter(input: ParseString) -> ParseResult<ParagraphElement> {
  let (input, text) = alt((alpha_token, quote))(input)?;
  Ok((input, ParagraphElement::Start(text)))
}

// paragraph_element := text+ ;
pub fn paragraph_element(input: ParseString) -> ParseResult<ParagraphElement> {
  let (input, elements) = match many1(text)(input) {
    Ok((input, mut text)) => {
      let mut text = merge_tokens(&mut text).unwrap();
      text.kind = TokenKind::Text;
      (input, ParagraphElement::Text(text))
    }, 
    Err(err) => {return Err(err);},
  };
  Ok((input, elements))
}

// paragraph := (inline_code | paragraph_text)+, whitespace*, new_line* ;
pub fn paragraph(input: ParseString) -> ParseResult<Paragraph> {
  let (input, first) = paragraph_starter(input)?;
  let (input, mut rest) = many0(paragraph_element)(input)?;
  let mut elements = vec![first];
  elements.append(&mut rest);
  Ok((input, Paragraph{elements}))
}

// unordered_list := list_item+, new_line?, whitespace* ;
pub fn unordered_list(input: ParseString) -> ParseResult<UnorderedList> {
  let (input, items) = many1(list_item)(input)?;
  let (input, _) = whitespace0(input)?;
  Ok((input,  UnorderedList{items}))
}

// list_item := dash, <space+>, <paragraph>, new_line* ;
pub fn list_item(input: ParseString) -> ParseResult<Paragraph> {
  let msg1 = "Expects space after dash";
  let msg2 = "Expects paragraph as list item";
  let (input, _) = dash(input)?;
  let (input, _) = labelr!(null(many1(space)), skip_nil, msg1)(input)?;
  let (input, list_item) = label!(paragraph, msg2)(input)?;
  let (input, _) = many0(new_line)(input)?;
  Ok((input,  list_item))
}


// code_block := grave, <grave>, <grave>, <new_line>, formatted_text, <grave{3}, new_line, whitespace*> ;
pub fn code_block(input: ParseString) -> ParseResult<SectionElement> {
  let msg1 = "Expects 3 graves to start a code block";
  let msg2 = "Expects new_line";
  let msg3 = "Expects 3 graves followed by new_line to terminate a code block";
  let (input, (_, r)) = range(nom_tuple((
    grave,
    label!(grave, msg1),
    label!(grave, msg1),
  )))(input)?;
  let (input, _) = label!(new_line, msg2)(input)?;
  //let (input, text) = formatted_text(input)?;
  let (input, _) = label!(nom_tuple((grave, grave, grave, new_line, whitespace0)), msg3, r)(input)?;
  Ok((input, SectionElement::CodeBlock))
}

// mech_code_alt := fsm_specification | fsm_implementation | function_define | statement | expression ;
pub fn mech_code_alt(input: ParseString) -> ParseResult<MechCode> {
  match fsm_specification(input.clone()) {
    Ok((input, fsm_spec)) => {return Ok((input, MechCode::FsmSpecification(fsm_spec)));},
    //Err(Failure(err)) => { return Err(Failure(err)); }
    _ => () 
  }
  match fsm_implementation(input.clone()) {
    Ok((input, fsm_impl)) => {return Ok((input, MechCode::FsmImplementation(fsm_impl)));},
    //Err(Failure(err)) => { return Err(Failure(err)); }
    _ => ()
  }
  match function_define(input.clone()) {
    Ok((input, fxn_def)) => {return Ok((input, MechCode::FunctionDefine(fxn_def)));},
    //Err(Failure(err)) => { return Err(Failure(err)); }
    _ => () 
  }
  match statement(input.clone()) {
    Ok((input, stmt)) => { return Ok((input, MechCode::Statement(stmt)));},
    //Err(Failure(err)) => { return Err(Failure(err)); }
    _ => ()
  }
  match expression(input.clone()) {
    Ok((input, expr)) => {return Ok((input, MechCode::Expression(expr)));},
    Err(err) => {return Err(err);}
  }
}

// mech_code := mech_code_alt, "\n" | ";" ;
pub fn mech_code(input: ParseString) -> ParseResult<MechCode> {
  let (input, code) = mech_code_alt(input.clone())?;
  let (input, _) = many0(space_tab)(input)?;
  let (input, _) = alt((new_line, semicolon))(input)?;
  Ok((input, code))
}

// ### Start here

// section_element := mech_code | unordered_list | comment | paragraph | code_block | sub_section;
pub fn section_element(input: ParseString) -> ParseResult<SectionElement> {
  let (input, section_element) = match mech_code(input.clone()) {
    Ok((input, code)) => (input, SectionElement::MechCode(code)),
    //Err(Failure(err)) => {return Err(Failure(err));}
    _ => match unordered_list(input.clone()) {
      Ok((input, list)) => (input, SectionElement::UnorderedList(list)),
      //Err(Failure(err)) => {return Err(Failure(err));}
      _ => match comment(input.clone()) {
        Ok((input, comment)) => (input, SectionElement::Comment(comment)),
        //Err(Failure(err)) => {return Err(Failure(err));}
        _ => match paragraph(input.clone()) {
          Ok((input, p)) => (input, SectionElement::Paragraph(p)),
          //Err(Failure(err)) => {return Err(Failure(err));}
          _ => match code_block(input.clone()) {
            Ok((input, m)) => (input,SectionElement::CodeBlock),
            //Err(Failure(err)) => {return Err(Failure(err));}
            _ => match sub_section(input) {
              Ok((input, s)) => (input, SectionElement::Section(Box::new(s))),
              Err(err) => { return Err(err); }
            }
          }
        }
      }
    }
  };
  let (input, _) = whitespace0(input)?;
  Ok((input, section_element))
}

// section_element := comment | unordered_list | mech_code | paragraph | code_block;
pub fn sub_section_element(input: ParseString) -> ParseResult<SectionElement> {
  let (input, section_element) = match comment(input.clone()) {
    Ok((input, comment)) => (input, SectionElement::Comment(comment)),
    _ => match unordered_list(input.clone()) {
      Ok((input, list)) => (input, SectionElement::UnorderedList(list)),
      _ => match mech_code(input.clone()) {
        Ok((input, m)) => (input, SectionElement::MechCode(m)),
        _ => match paragraph(input.clone()) {
          Ok((input, p)) => (input, SectionElement::Paragraph(p)),
          _ => match code_block(input.clone()) {
            Ok((input, m)) => (input,SectionElement::CodeBlock),
            Err(err) => { return Err(err); }
          }
        }
      }
    }
  };
  let (input, _) = whitespace0(input)?;
  Ok((input, section_element))
}

// section := ul_subtitle?, section_element+ ;
pub fn section(input: ParseString) -> ParseResult<Section> {
  let msg = "Expects user function, block, mech code block, code block, statement, paragraph, or unordered list";
  let (input, subtitle) = opt(ul_subtitle)(input)?;
  let (input, elements) = many1(section_element)(input)?;
  Ok((input, Section{subtitle, elements}))
}

// sub_section := alpha_subtitle, sub_section_element* ;
pub fn sub_section(input: ParseString) -> ParseResult<Section> {
  let msg = "Expects user function, block, mech code block, code block, statement, paragraph, or unordered list";
  let (input, subtitle) = alpha_subtitle(input)?;
  let (input, elements) = many0(sub_section_element)(input)?;
  Ok((input, Section{subtitle: Some(subtitle), elements}))
}


// body := section+ ;
pub fn body(input: ParseString) -> ParseResult<Body> {
  let (input, _) = whitespace0(input)?;
  let (input, sections) = many1(section)(input)?;
  let (input, _) = whitespace0(input)?;
  Ok((input, Body{sections}))
}

// program := title?, body ;
pub fn program(input: ParseString) -> ParseResult<Program> {
  let msg = "Expects program body";
  let (input, _) = whitespace0(input)?;
  let (input, title) = opt(title)(input)?;
  //let (input, body) = labelr!(body, skip_nil, msg)(input)?;
  let (input, body) = body(input)?;
  let (input, _) = whitespace0(input)?;
  Ok((input, Program{title, body}))
}

// parse_mech := program | statement ;
pub fn parse_mech(input: ParseString) -> ParseResult<Program> {
  //let (input, mech) = alt((program, statement))(input)?;
  //Ok((input, ParserNode::Root { children: vec![mech] }))
  let (input, mech) = program(input)?;
  Ok((input, mech))
}


// 8. Public interface
// ---------------------

/// Print formatted error message.
pub fn print_err_report(text: &str, report: &ParserErrorReport) {
  let msg = TextFormatter::new(text).format_error(report);
  println!("{}", msg);
}

pub fn parse(text: &str) -> Result<Program, MechError> {
  let graphemes = graphemes::init_source(text);
  let mut result_node = None;
  let mut error_log: Vec<(SourceRange, ParseErrorDetail)> = vec![];

  // Do parse
  let remaining: ParseString = match parse_mech(ParseString::new(&graphemes)) {
    // Got a parse tree, however there may be errors
    Ok((mut remaining_input, parse_tree)) => {
      error_log.append(&mut remaining_input.error_log);
      result_node = Some(parse_tree);
      remaining_input
    },
    // Parsing failed and could not be recovered. No parse tree was created in this case
    Err(err) => {
      match err {
        Err::Error(mut e) | Err::Failure(mut e) => {
          error_log.append(&mut e.remaining_input.error_log);
          error_log.push((e.cause_range, e.error_detail));
          e.remaining_input
        },
        Err::Incomplete(_) => panic!("nom::Err::Incomplete is not supported!"),
      }
    },
  };

  // Check if all inputs were parsed
  if remaining.len() != 0 {
    let e = ParseError::new(remaining, "Inputs since here are not parsed");
    error_log.push((e.cause_range, e.error_detail));
  }

  // Construct result
  if error_log.is_empty() {
    Ok(result_node.unwrap())
  } else {
    let report: ParserErrorReport = error_log.into_iter().map(|e| ParserErrorContext {
      cause_rng: e.0,
      err_message: String::from(e.1.message),
      annotation_rngs: e.1.annotation_rngs,
    }).collect();
    let msg = TextFormatter::new(text).format_error(&report);
    Err(MechError{tokens: vec![], msg: "".to_string(), id: 3202, kind: MechErrorKind::ParserError(ParserNode::Error, report, msg)})
  }
}