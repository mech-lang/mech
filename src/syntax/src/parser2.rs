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
};

use std::collections::HashMap;
use colored::*;

// 2. Parser utilities
// ---------------------

/// Unicode grapheme group utilities.
/// Current implementation does not guarantee correct behavior for
/// all possible unicode characters.
pub mod graphemes {
  use unicode_segmentation::UnicodeSegmentation;

  /// Obtain unicode grapheme groups from input source, then make sure
  /// it ends with new_line.  Many functions in the parser assume input
  /// ends with new_line.
  pub fn init_source(text: &str) -> Vec<&str> {
    let mut graphemes = UnicodeSegmentation::graphemes(text, true).collect::<Vec<&str>>();
    graphemes.push("\n");
    graphemes
  }

  pub fn init_tag(tag: &str) -> Vec<&str> {
    UnicodeSegmentation::graphemes(tag, true).collect::<Vec<&str>>()
  }

  pub fn is_new_line(grapheme: &str) -> bool {
    match grapheme {
      "\r" | "\n" | "\r\n" => true,
      _ => false,
    }
  }

  pub fn is_numeric(grapheme: &str) -> bool {
    grapheme.chars().next().unwrap().is_numeric()
  }

  pub fn is_alpha(grapheme: &str) -> bool {
    grapheme.chars().next().unwrap().is_alphabetic()
  }

  pub fn is_emoji(grapheme: &str) -> bool {
    let ch = grapheme.chars().next().unwrap();
    !(ch.is_alphanumeric() || ch.is_ascii())
  }

  pub fn width(grapheme: &str) -> usize {
    // TODO: uniode width?
    let ch = grapheme.chars().next().unwrap();
    if ch == '\t' {
      1
    } else if ch.is_control() {
      0
    } else {
      1
    }
  }
}

/// Just alias
pub type ParseResult<'a, O> = IResult<ParseString<'a>, O, ParseError<'a>>;

/// The input type for nom parsers. Instead of holding the actual input
/// string, this struct only holds a reference to that string so that it
/// can be cloned at much lower cost.
#[derive(Clone, Debug)]
pub struct ParseString<'a> {
  /// Source code
  pub graphemes: &'a Vec<&'a str>,
  /// Error report, a list of (error_location, error_context)
  pub error_log: Vec<(SourceRange, ParseErrorDetail)>,
  /// Point at the next grapheme to consume
  pub cursor: usize,
  /// Location of the grapheme pointed by cursor
  pub location: SourceLocation,
}

impl<'a> ParseString<'a> {
  /// Must always point a an actual string
  pub fn new(graphemes: &'a Vec<&'a str>) -> Self {
    ParseString {
      graphemes,
      error_log: vec![],
      cursor: 0,
      location: SourceLocation { row: 1, col: 1 },
    }
  }

  pub fn current(&self) -> Option<&str> {
    self.graphemes.get(self.cursor).copied()
  }

  /// If current location matches the tag, consume the matched string.
  fn consume_tag(&mut self, tag: &str) -> Option<String> {
    if self.is_empty() {
      return None;
    }
    let current = self.graphemes[self.cursor];

    let gs = graphemes::init_tag(tag); 
    let gs_len = gs.len();

    // Must have enough remaining characters
    if self.len() < gs_len {
      return None;
    }

    // Try to match the tag
    let mut tmp_location = self.location;
    for i in 0..gs_len {
      let c = self.cursor + i;
      let g = self.graphemes[c];
      if g != gs[i] {
        return None;
      }
      if graphemes::is_new_line(g) {
        if !self.is_last_grapheme(c) {
          tmp_location.row += 1;
          tmp_location.col = 1;
        }
      } else {
        tmp_location.col += graphemes::width(g);
      }
    }
    // Tag matched, commit change
    self.cursor += gs_len;
    self.location = tmp_location;
    Some(tag.to_string())
  }

  /// Mutate self by consuming one grapheme
  fn consume_one(&mut self) -> Option<String> {
    if self.is_empty() {
      return None;
    }
    let g = self.graphemes[self.cursor];
    if graphemes::is_new_line(g) {
      if !self.is_last_grapheme(self.cursor) {
        self.location.row += 1;
        self.location.col = 1;
      }
    } else {
      self.location.col += graphemes::width(g);
    }
    self.cursor += 1;
    Some(g.to_string())
  }


  /// If current location matches any emoji, consume the matched string.
  fn consume_emoji(&mut self) -> Option<String> {
    if self.is_empty() {
      return None;
    }
    let g = self.graphemes[self.cursor];
    
    if graphemes::is_emoji(g) {
      self.cursor += 1;
      self.location.col += graphemes::width(g);
      Some(g.to_string())
    } else {
      None
    }
  }

  /// If current location matches any alpha char, consume the matched string.
  fn consume_alpha(&mut self) -> Option<String> {
    if self.is_empty() {
      return None;
    }
    let g = self.graphemes[self.cursor];
    if graphemes::is_alpha(g) {
      self.cursor += 1;
      self.location.col += graphemes::width(g);
      Some(g.to_string())
    } else {
      None
    }
  }

  /// If current location matches any digit, consume the matched string.
  fn consume_digit(&mut self) -> Option<String> {
    if self.is_empty() {
      return None;
    }
    let g = self.graphemes[self.cursor];
    if graphemes::is_numeric(g) {
      self.cursor += 1;
      self.location.col += graphemes::width(g);
      Some(g.to_string())
    } else {
      None
    }
  }

  /// Get cursor's location in source code
  fn loc(&self) -> SourceLocation {
    self.location
  }

  /// Test whether the grapheme pointed by cursor is the last grapheme
  fn is_last_grapheme(&self, c: usize) -> bool {
    (self.graphemes.len() - 1 - c) == 0
  }

  /// Get remaining (unparsed) length
  pub fn len(&self) -> usize {
    self.graphemes.len() - self.cursor
  }
  
  pub fn is_empty(&self) -> bool {
    self.len() == 0
  }

  /// For debug purpose
  fn output(&self) {
              
    println!("───────────────────{}", self.len());
    for i in self.cursor..self.graphemes.len() {
      print!("{}", self.graphemes[i]);
    }
    println!();
    println!("───────────────────");
  }
}

/// Required by nom
impl<'a> nom::InputLength for ParseString<'a> {
  fn input_len(&self) -> usize {
    self.len()
  }
}

/// The part of error context that's independent to its cause location.
#[derive(Clone, Debug)]
pub struct ParseErrorDetail {
  pub message: &'static str,
  pub annotation_rngs: Vec<SourceRange>,
}

/// The error type for the nom parser, which handles full error context
/// (location + detail) and ownership of the input ParseString.
///
/// Eventually error context will be logged and ownership will be moved out.
#[derive(Clone, Debug)]
pub struct ParseError<'a> {
  /// Cause range is defined as [start, end), where `start` points at the first
  /// character that's catched by a label, and `end` points at the next 
  /// character of the character that didn't match.
  ///
  /// Example:
  ///   index:  1234567
  ///   input:  abcdefg
  ///   error:   ~~~^
  ///   range:   |   |
  ///           [2,  5)
  ///
  pub cause_range: SourceRange,
  /// Hold ownership to the input ParseString
  pub remaining_input: ParseString<'a>,
  /// Detailed information about this error
  pub error_detail: ParseErrorDetail,
}

impl<'a> ParseError<'a> {
  /// Create a new error at current location of the input, with given message
  /// and empty annotations.  Ownership of the input is also passed into this
  /// error object.
  pub fn new(input: ParseString<'a>, msg: &'static str) -> Self {
    let start = input.loc();
    let mut end = start;
    end.col += 1;
    ParseError {
      cause_range: SourceRange { start, end },
      remaining_input: input,
      error_detail: ParseErrorDetail {
        message: msg,
        annotation_rngs: vec![],
      }
    }
  }

  /// Add self to the error log of input string.
  fn log(&mut self) {
    self.remaining_input.error_log.push((self.cause_range, self.error_detail.clone()));
  }
}

/// Required by nom
impl<'a> nom::error::ParseError<ParseString<'a>> for ParseError<'a> {
  /// Not used, unless we have logical error
  fn from_error_kind(input: ParseString<'a>,
                     _kind: nom::error::ErrorKind) -> Self {
    ParseError::new(input, "Unexpected error")
  }

  /// Probably not used
  fn append(_input: ParseString<'a>,
            _kind: nom::error::ErrorKind,
            other: Self) -> Self {
    other
  }

  /// Barely used, but we do want to keep the error with larger depth.
  fn or(self, other: Self) -> Self {
    let self_start = self.cause_range.start;
    let other_start = other.cause_range.start;
    if self_start > other_start {
      self
    } else {
      other
    }
  }
}

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

fn skip_pass_eol(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = skip_till_eol(input)?;
  let (input, _) = new_line(input)?;
  Ok((input, ParserNode::Null))
}

fn skip_till_section_element(input: ParseString) -> ParseResult<ParserNode> {
  if input.is_empty() {
    return Ok((input, ParserNode::Error));
  }
  let (input, _) = skip_pass_eol(input)?;
  let (input, _) = many0(nom_tuple((
    is_not(section_element),
    skip_pass_eol,
  )))(input)?;
  Ok((input, ParserNode::Error))
}

/*
fn skip_till_section_element2(input: ParseString) -> ParseResult<ParserNode> {
  if input.len() == 0 {
    return Ok((input, ParserNode::Error));
  }
  let (input, _) = skip_pass_eol(input)?;
  let (input, _) = many0(nom_tuple((
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
  let (input, _) = many0(nom_tuple((
    is_not(section_element3),
    skip_pass_eol,
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

/*pub fn emoji_grapheme(mut input: ParseString) -> ParseResult<String> {
  if let Some(matched) = input.consume_emoji() {
    Ok((input, matched))
  } else {
    Err(nom::Err::Error(ParseError::new(input, "Unexpected character")))
  }
}*/

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

// emoji ::= emoji_grapheme+ ;
/*fn emoji(input: ParseString) -> ParseResult<Token> {
  let start = input.loc();
  let (input, g) = emoji_grapheme(input)?;
  let end = input.loc();
  let src_range = SourceRange { start, end };
  Ok((input, Token{kind: TokenKind::Emoji, chars: g.chars().collect::<Vec<char>>(), src_range}))
}*/

fn alpha_token(input: ParseString) -> ParseResult<Token> {
  let (input, (g, src_range)) = range(alpha)(input)?;
  Ok((input, Token{kind: TokenKind::Alpha, chars: g.chars().collect::<Vec<char>>(), src_range}))
}

fn digit_token(input: ParseString) -> ParseResult<Token> {
  let (input, (g, src_range)) = range(digit)(input)?;
  Ok((input, Token{kind: TokenKind::Digit, chars: g.chars().collect::<Vec<char>>(), src_range}))
}

fn underscore_digit(input: ParseString) -> ParseResult<Token> {
  let (input, _) = underscore(input)?;
  let (input, digit) = digit_token(input)?;
  Ok((input,digit))
}

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

// punctuation ::= period | exclamation | question | comma | colon | semicolon | dash | apostrophe ;
pub fn punctuation(input: ParseString) -> ParseResult<Token> {
  let (input, punctuation) = alt((period, exclamation, question, comma, colon, semicolon, quote, apostrophe))(input)?;
  Ok((input, punctuation))
}

// escaped_char ::= "\" ,  symbol | punctuation ;
pub fn escaped_char(input: ParseString) -> ParseResult<Token> {
  let (input, _) = backslash(input)?;
  let (input, symbol) = alt((symbol, punctuation))(input)?;
  Ok((input, symbol))
}

// symbol ::= ampersand | bar | at | slash | hashtag | equal | tilde | plus | asterisk | asterisk | caret | underscore ;
pub fn symbol(input: ParseString) -> ParseResult<Token> {
  let (input, symbol) = alt((ampersand, bar, at, slash, hashtag, equal, backslash, tilde, plus, dash, asterisk, caret, underscore))(input)?;
  Ok((input, symbol))
}

// text ::= (alpha | digit_token | space | punctuation | grouping_symbol | symbol | emoji | escaped_char)+ ;
pub fn text(input: ParseString) -> ParseResult<Token> {
  let (input, text) = alt((alpha_token, digit_token, space, tab, escaped_char, punctuation, grouping_symbol, symbol))(input)?;
  Ok((input, text))
}

// identifier ::= (word | emoji), (word | number | symbol | emoji)* ;
pub fn identifier(input: ParseString) -> ParseResult<Identifier> {
  let (input, (first, mut rest)) = nom_tuple((alpha_token, many0(alt((alpha_token, digit_token, symbol)))))(input)?;
  let mut tokens = vec![first];
  tokens.append(&mut rest);
  let mut merged = merge_tokens(&mut tokens).unwrap();
  merged.kind = TokenKind::Identifier; 
  Ok((input, Identifier{name: merged}))
}

// boolean_literal ::= true_literal | false_literal ;
pub fn boolean(input: ParseString) -> ParseResult<Token> {
  let (input, boolean) = alt((true_literal, false_literal))(input)?;
  Ok((input, boolean))
}

// true_literal ::= english_true_literal | check_mark ;
pub fn true_literal(input: ParseString) -> ParseResult<Token> {
  let (input, token) = alt((english_true_literal, check_mark))(input)?;
  Ok((input, token))
}

// false_literal ::= english_false_literal | cross ;
pub fn false_literal(input: ParseString) -> ParseResult<Token> {
  let (input, token) = alt((english_false_literal, cross))(input)?;
  Ok((input, token))
}

// new_line ::= new_line_char | carriage_new_line ;
pub fn new_line(input: ParseString) -> ParseResult<Token> {
  let (input, result) = alt((carriage_return_new_line,new_line_char,carriage_return, ))(input)?;
  Ok((input, result))
}

// whitespace ::= space | new_line | carriage_return | tabe ;
pub fn whitespace(input: ParseString) -> ParseResult<Token> {
  let (input, space) = alt((space,tab,new_line))(input)?;
  Ok((input, space))
}

pub fn list_separator(input: ParseString) -> ParseResult<()> {
  let (input,_) = nom_tuple((many0(whitespace),tag(","),many0(whitespace)))(input)?;
  Ok((input, ()))
}

// number-literal := (integer | hexadecimal | octal | binary | decimal | float | rational | scientific) ;
pub fn number(input: ParseString) -> ParseResult<Number> {
  let (input, neg) = opt(dash)(input)?;
  let (input, result) = alt((hexadecimal_literal, decimal_literal, octal_literal, binary_literal, scientific_literal, rational_literal, float_literal, integer_literal))(input)?;
  let result = match neg {
    Some(_) => Number::Negated(Box::new(result)),
    None => result,
  };
  Ok((input, result))
}

pub fn rational_literal(input: ParseString) -> ParseResult<Number> {
  let (input, Number::Integer(numerator)) = integer_literal(input)? else { unreachable!() };
  let (input, _) = slash(input)?;
  let (input, Number::Integer(denominator)) = integer_literal(input)? else { unreachable!() };
  Ok((input, Number::Rational((numerator,denominator))))
}

pub fn scientific_literal(input: ParseString) -> ParseResult<Number> {
  let (input, base) = match float_literal(input.clone()) {
    Ok((input, Number::Float(base))) => {
      (input, base)
    }
    _ => match integer_literal(input.clone()) {
      Ok((input, Number::Integer(base))) => {
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
    Ok((input, Number::Float(exponent))) => {
      (input, exponent)
    }
    _ => match integer_literal(input.clone()) {
      Ok((input, Number::Integer(exponent))) => {
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
  Ok((input, Number::Scientific((base,(ex_sign,ex_whole,ex_part)))))
}

fn float_decimal_start(input: ParseString) -> ParseResult<Number> {
  let (input, _) = period(input)?;
  let (input, part) = many1(digit_token)(input)?;
  let mut tokens2 = part.clone();
  let mut merged = merge_tokens(&mut tokens2).unwrap();
  merged.kind = TokenKind::Number;
  Ok((input, Number::Float((Token::default(),merged))))
}

fn float_full(input: ParseString) -> ParseResult<Number> {
  let (input, mut whole) = many1(digit_token)(input)?;
  let (input, _) = period(input)?;
  let (input, mut part) = many1(digit_token)(input)?;
  let mut whole = merge_tokens(&mut whole).unwrap();
  let mut part = merge_tokens(&mut part).unwrap();
  whole.kind = TokenKind::Number;
  part.kind = TokenKind::Number;
  Ok((input, Number::Float((whole,part))))
}

// float_literal ::= "."?, digit1, "."?, digit0 ;
pub fn float_literal(input: ParseString) -> ParseResult<Number> {
  let (input, result) = alt((float_decimal_start,float_full))(input)?;
  Ok((input, result))
}

// integer ::= digit1 ;
pub fn integer_literal(input: ParseString) -> ParseResult<Number> {
  let (input, mut digits) = digit_sequence(input)?;
  let mut merged = merge_tokens(&mut digits).unwrap();
  merged.kind = TokenKind::Number; 
  Ok((input, Number::Integer(merged)))
}

// decimal_literal ::= "0d", <digit1> ;
pub fn decimal_literal(input: ParseString) -> ParseResult<Number> {
  let msg = "Expects decimal digits after \"0d\"";
  let input = tag("0d")(input);
  let (input, _) = input?;
  let (input, mut tokens) = label!(many1(digit_token), msg)(input)?;
  let mut merged = merge_tokens(&mut tokens).unwrap();
  merged.kind = TokenKind::Number; 
  Ok((input, Number::Decimal(merged)))
}

// hexadecimal_literal ::= "0x", <hex_digit+> ;
pub fn hexadecimal_literal(input: ParseString) -> ParseResult<Number> {
  let msg = "Expects hexadecimal digits after \"0x\"";
  let input = tag("0x")(input);
  let (input, _) = input?;
  let (input, mut tokens) = label!(many1(alt((digit_token,alpha_token))), msg)(input)?;
  let mut merged = merge_tokens(&mut tokens).unwrap();
  merged.kind = TokenKind::Number; 
  Ok((input, Number::Hexadecimal(merged)))
}

// octal_literal ::= "0o", <oct_digit+> ;
pub fn octal_literal(input: ParseString) -> ParseResult<Number> {
  let msg = "Expects octal digits after \"0o\"";
  let input = tag("0o")(input);
  let (input, _) = input?;
  let (input, mut tokens) = label!(many1(alt((digit_token,alpha_token))), msg)(input)?;
  let mut merged = merge_tokens(&mut tokens).unwrap();
  merged.kind = TokenKind::Number; 
  Ok((input, Number::Octal(merged)))
}

// binary_literal ::= "0b", <bin_digit+> ;
pub fn binary_literal(input: ParseString) -> ParseResult<Number> {
  let msg = "Expects binary digits after \"0b\"";
  let input = tag("0b")(input);
  let (input, _) = input?;
  let (input, mut tokens) = label!(many1(alt((digit_token,alpha_token))), msg)(input)?;
  let mut merged = merge_tokens(&mut tokens).unwrap();
  merged.kind = TokenKind::Number; 
  Ok((input, Number::Binary(merged)))
}

// empty ::= underscore+ ;
pub fn empty(input: ParseString) -> ParseResult<Token> {
  let (input, (g, src_range)) = range(many1(tag("_")))(input)?;
  Ok((input, Token{kind: TokenKind::Empty, chars: g.join("").chars().collect(), src_range}))
}

// #### Enums

// enum_define ::= "<", identifier, ">", space*, "=", space*, enum_list;
pub fn enum_define(input: ParseString) -> ParseResult<ParserNode> {
  /*let msg2 = "Expects expression";
  let (input, _) = left_angle(input)?;
  let (input, variable) = identifier(input)?;
  let (input, _) = right_angle(input)?;
  let (input, _) = many1(space)(input)?;
  let (input, _) = equal(input)?;
  let (input, _) = many1(space)(input)?;
  let (input, expression) = label!(expression, msg2)(input)?;*/
  Ok((input,ParserNode::Error))
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
  /*let (input, subscript) = alt((select_all, expression, tilde))(input)?;
  let (input, _) = nom_tuple((many0(space), opt(comma), many0(space)))(input)?;*/
  Ok((input, ParserNode::Error))
}

// subscript_index ::= left_brace, <subscript+>, <right_brace> ;
pub fn subscript_index(input: ParseString) -> ParseResult<ParserNode> {
  let msg1 = "Expects subscript";
  let msg2 = "Expects right brace '}'";
  let (input, (_, r)) = range(left_brace)(input)?;
  let (input, subscripts) = label!(many1(subscript), msg1)(input)?;
  let (input, _) = label!(right_brace, msg2, r)(input)?;
  Ok((input, ParserNode::SubscriptIndex{children: subscripts}))
}

// single_subscript_index ::= left_brace, <subscript>, <right_brace> ;
pub fn single_subscript_index(input: ParseString) -> ParseResult<ParserNode> {
  let msg1 = "Expects subscript";
  let msg2 = "Expects right brace '}'";
  let (input, _) = left_brace(input)?;
  let (input, subscript) = label!(subscript, msg1)(input)?;
  let (input, _) = label!(right_brace, msg2)(input)?;
  Ok((input, ParserNode::SubscriptIndex{children: vec![subscript]}))
}

// dot_index ::= period, <identifier>, single_subscript_index? ;
pub fn dot_index(input: ParseString) -> ParseResult<ParserNode> {
  /*let msg = "Expects identifier";
  let (input, _) = period(input)?;
  let (input, identifier) = label!(identifier, msg)(input)?;
  let (input, subscript) = opt(single_subscript_index)(input)?;
  let index = match subscript {
    Some(subscript) => vec![subscript, identifier],
    None => vec![ParserNode::Null, identifier],
  };*/
  Ok((input, ParserNode::Error))
}

// swizzle ::= period, identifier, comma, !space, <identifier, (",", identifier)*> ;
pub fn swizzle(input: ParseString) -> ParseResult<ParserNode> {
  /*let msg = "Expects identifier for swizzling";
  let (input, _) = period(input)?;
  let (input, first) = identifier(input)?;
  let (input, _) = comma(input)?;
  let (input, _) = is_not(space)(input)?;  // so that it's not a binding
  let (input, mut rest) = label!(separated_list1(tag(","), identifier), msg)(input)?;
  let mut cols = vec![first];
  cols.append(&mut rest);*/
  Ok((input, ParserNode::Error))
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
  /*let (input, source) = alt((table, identifier))(input)?;
  let (input, mut indices) = many0(index)(input)?;
  let (input, transpose) = opt(transpose)(input)?;
  let mut data = vec![source];
  data.append(&mut indices);
  match transpose {
    Some(transpose) => {
      data.push(transpose);
    }
    _ => (),
  }*/
  Ok((input, ParserNode::Error))
}


// #### Kind Annotations

// kind_annotation ::= left_angle, <(identifier | underscore), (",", (identifier | underscore))*>, <right_angle> ;
pub fn kind_annotation(input: ParseString) -> ParseResult<KindAnnotation> {
  let msg2 = "Expects at least one unit in kind annotation";
  let msg3 = "Expects right angle";
  let (input, (_, r)) = range(left_angle)(input)?;
  let (input, kinds) = separated_list1(list_separator, kind)(input)?;
  let (input, _) = label!(right_angle, msg3, r)(input)?;
  Ok((input, KindAnnotation{ kinds }))
}

pub fn kind(input: ParseString) -> ParseResult<Kind> {
  let (input, kind) = alt((kind_tuple, kind_scalar))(input)?;
  Ok((input, kind))
}

pub fn kind_tuple(input: ParseString) -> ParseResult<Kind> {
  let (input, _) = left_parenthesis(input)?;
  let (input, kinds) = separated_list1(list_separator, kind)(input)?;
  let (input, _) = right_parenthesis(input)?;
  Ok((input, Kind::Tuple(kinds)))
}

pub fn kind_scalar(input: ParseString) -> ParseResult<Kind> {
  let (input, kind) = kind_label(input)?;
  Ok((input, Kind::Scalar(kind)))
}

pub fn kind_label(input: ParseString) -> ParseResult<KindLabel> {
  let (input, name) = identifier(input)?;
  let (input, _) = opt(colon)(input)?;
  let (input, size) = separated_list0(list_separator,number)(input)?;
  Ok((input, KindLabel{ name, size }))
}

// #### Structures

// structure ::= hashtag, <identifier> ;
pub fn structure(input: ParseString) -> ParseResult<Structure> {
  let (input, table) = match empty_table(input.clone()) {
    Ok((input, table)) => (input, Structure::Empty),
    Err(err) => match table(input.clone()) {
      Ok((input, tbl)) => (input, Structure::Table(tbl)),
      Err(err) => {
        match tuple(input.clone()) {
          Ok((input, tpl)) => (input, Structure::Tuple(tpl)),
          _ => match record(input.clone()) {
            Ok((input, table)) => (input, Structure::Record(table)),
            Err(e3) => {
              return Err(err);
            }
          }
        }
      },
    }
  };
  Ok((input, table))
}

// binding ::= s*, identifier, kind_annotation?, <!(space+, colon)>, colon, s+,
// >>          <empty | expression | identifier | value>, <!!right_bracket | (s*, comma, <s+>) | s+> ;
// >> where s ::= space | new_line | tab ;
pub fn binding(input: ParseString) -> ParseResult<Binding> {
  let msg1 = "Unexpected space before colon ':'";
  let msg2 = "Expects a value";
  let msg3 = "Expects whitespace or comma followed by whitespace";
  let msg4 = "Expects whitespace";
  let (input, _) = many0(whitespace)(input)?;
  let (input, name) = identifier(input)?;
  let (input, kind) = opt(kind_annotation)(input)?;
  let (input, _) = label!(is_not(nom_tuple((many1(space), colon))), msg1)(input)?;
  let (input, _) = colon(input)?;
  let (input, _) = many1(whitespace)(input)?;
  let (input, value) = label!(expression, msg2)(input)?;
  let (input, _) = many0(whitespace)(input)?;
  let (input, _) = opt(comma)(input)?;
  let (input, _) = many0(whitespace)(input)?;
  Ok((input, Binding{name, kind, value}))
}

// function_binding ::= identifier, <colon>, <space+>, <expression | identifier | value>, space*, comma?, space* ;
pub fn function_binding(input: ParseString) -> ParseResult<ParserNode> {
  /*let msg1 = "Expects colon ':' for function binding";
  let msg2 = "Expects space after colon for function binding";
  let msg3 = "Expects expression, identifier, or value to bind";
  let (input, (binding_id, r)) = range(identifier)(input)?;
  let (input, _) = label!(colon, msg1)(input)?;
  let (input, _) = label!(many1(space), msg2)(input)?;
  let (input, bound) = label!(alt((expression, identifier, value)), msg3, r)(input)?;
  let (input, _) = nom_tuple((many0(space), opt(comma), many0(space)))(input)?;*/
  Ok((input, ParserNode::Error))
}

// table_column ::= (space | tab)*, (expression | value | data), comma?, (space | tab)* ;
pub fn table_column(input: ParseString) -> ParseResult<TableColumn> {
  let (input, _) = many0(alt((space, tab)))(input)?;
  let (input, element) = expression(input)?;
  let (input, _) = nom_tuple((many0(alt((space, tab))),opt(alt((comma,table_separator))), many0(alt((space, tab)))))(input)?;
  Ok((input, TableColumn{element}))
}

// table_row ::= (space | tab)*, table_column+, semicolon?, new_line? ;
pub fn table_row(input: ParseString) -> ParseResult<TableRow> {
  let (input, _) = opt(table_separator)(input)?;
  let (input, _) = many0(alt((space, tab)))(input)?;
  let (input, columns) = many1(table_column)(input)?;
  let (input, _) = nom_tuple((opt(semicolon), opt(new_line)))(input)?;
  let (input, _) = opt(nom_tuple((many1(box_drawing_char),new_line)))(input)?;
  Ok((input, TableRow{columns}))
}

// table_header ::= bar, <attribute+>, <bar>, space*, new_line? ;
/*pub fn table_header(input: ParseString) -> ParseResult<ParserNode> {
  let msg1 = "Expects at least one attribute for table header";
  let msg2 = "Expects vertical bar to terminate table header";
  let (input, (_, r)) = range(bar)(input)?;
  let (input, attributes) = label!(many1(attribute), msg1)(input)?;
  let (input, _) = nom_tuple((label!(bar, msg2, r), many0(space), opt(new_line)))(input)?;
  Ok((input, ParserNode::TableHeader{children: attributes}))
}*/


pub fn box_drawing_char(input: ParseString) -> ParseResult<Token> {
  alt((box_tr_round, box_bl_round, box_vert, box_cross, box_horz, box_t_left, box_t_right, box_t_top, box_t_bottom))(input)
}

pub fn table_start(input: ParseString) -> ParseResult<Token> {
  alt((box_tl_round, left_bracket))(input)
}

pub fn table_end(input: ParseString) -> ParseResult<Token> {
  let result = alt((box_br_round, right_bracket))(input);
  result
}

pub fn table_separator(input: ParseString) -> ParseResult<Token> {
  let (input, token) = alt((dollar, at, box_vert))(input)?;
  Ok((input, token))
}

// anonymous_table ::= left_bracket, (space | new_line | tab)*, table_header?,
// >>                  ((comment, new_line) | table_row)*, (space | new_line | tab)*, <right_bracket> ;
pub fn table(input: ParseString) -> ParseResult<Table> {
  let msg = "Expects right bracket ']' to finish the table";
  let (input, (_, r)) = range(table_start)(input)?;
  let (input, _) = many0(alt((box_drawing_char,whitespace)))(input)?;
  let (input, rows) = many1(table_row)(input)?;
  let (input, _) = many0(box_drawing_char)(input)?;
  let (input, _) = many0(whitespace)(input)?;
  let (input, _) = match label!(table_end, msg, r)(input) {
    Ok((input, matches)) => {
      (input, matches)
    }
    Err(err) => {
      //println!("!!!!!!!!!!!!!!{:?}", err);
      return Err(err);
    }
  };
  Ok((input, Table{rows}))
}

// empty_table ::= left_bracket, (space | new_line | tab)*, table_header?, (space | new_line | tab)*, right_bracket ;
pub fn empty_table(input: ParseString) -> ParseResult<Structure> {
  let (input, _) = table_start(input)?;
  let (input, _) = many0(whitespace)(input)?;
  let (input, _) = opt(empty)(input)?;
  let (input, _) = many0(whitespace)(input)?;
  let (input, _) = table_end(input)?;
  Ok((input, Structure::Empty))
}

// record ::= left_bracket, binding, <binding_strict*>, <right_bracket> ;
pub fn record(input: ParseString) -> ParseResult<Record> {
  let msg = "Expects right bracket ']' to terminate inline table";
  let (input, (_, r)) = range(left_bracket)(input)?;
  let (input, bindings) = many1(binding)(input)?;
  let (input, _) = label!(right_bracket, msg, r)(input)?;
  Ok((input, Record{bindings}))
}

// #### State Machines

pub fn fsm_define_operator(input: ParseString) -> ParseResult<()> {
  let (input, _) = many1(whitespace)(input)?;
  let (input, _) = tag(":=")(input)?;
  Ok((input, ()))
}

pub fn fsm_output_operator(input: ParseString) -> ParseResult<()> {
  let (input, _) = many1(whitespace)(input)?;
  let (input, _) = tag("->")(input)?;
  let (input, _) = many1(whitespace)(input)?;
  Ok((input, ()))
}

pub fn fsm_transition_operator(input: ParseString) -> ParseResult<()> {
  let (input, _) = many1(whitespace)(input)?;
  let (input, _) = tag("=>")(input)?;
  let (input, _) = many1(whitespace)(input)?;
  Ok((input, ()))
}

pub fn fsm_guard_operator(input: ParseString) -> ParseResult<()> {
  let (input, _) = many1(whitespace)(input)?;
  let (input, _) = alt((tag("|"),tag("│"),tag("├"),tag("└")))(input)?;
  let (input, _) = many1(whitespace)(input)?;
  Ok((input, ()))
}

pub fn fsm_implementation(input: ParseString) -> ParseResult<FsmImplementation> {
  let ((input, _)) = hashtag(input)?;
  let ((input, name)) = identifier(input)?;
  let ((input, _)) = left_parenthesis(input)?;
  let ((input, input_vars)) = separated_list0(list_separator, identifier)(input)?;
  let ((input, _)) = right_parenthesis(input)?;
  let ((input, _)) = fsm_transition_operator(input)?;
  let ((input, start)) = fsm_pattern(input)?;
  let ((input, _)) = many0(whitespace)(input)?;
  let ((input, arms)) = many0(fsm_arm)(input)?;
  let ((input, _)) = period(input)?;
  Ok((input, FsmImplementation{name,input: input_vars,start,arms}))
}

pub fn fsm_arm(input: ParseString) -> ParseResult<FsmArm> {
  let ((input, _)) = many0(comment)(input)?;
  let ((input, start)) = fsm_pattern(input)?;
  let ((input, trns)) = many1(alt((fsm_state_transition,fsm_output,fsm_guard)))(input)?;
  let ((input, _)) = many0(whitespace)(input)?;
  Ok((input, FsmArm{start, transitions: trns}))
}

pub fn fsm_guard(input: ParseString) -> ParseResult<Transition> {
  let (input, _) = alt((fsm_transition_operator,fsm_guard_operator))(input)?;
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
  let (input, _) = fsm_transition_operator(input)?;
  let ((input, ptrn)) = fsm_pattern(input)?;
  Ok((input, Transition::Next(ptrn)))
}

pub fn fsm_output(input: ParseString) -> ParseResult<Transition> {
  let (input, _) = fsm_output_operator(input)?;
  let ((input, ptrn)) = fsm_pattern(input)?;
  Ok((input, Transition::Output(ptrn)))
}

pub fn fsm_specification(input: ParseString) -> ParseResult<FsmSpecification> {
  let ((input, _)) = hashtag(input)?;
  let ((input, name)) = identifier(input)?;
  let ((input, _)) = left_parenthesis(input)?;
  let ((input, input_vars)) = separated_list0(list_separator, identifier)(input)?;
  let ((input, _)) = right_parenthesis(input)?;
  let ((input, _)) = fsm_output_operator(input)?;
  let ((input, output)) = identifier(input)?;
  let ((input, _)) = fsm_define_operator(input)?;
  let ((input, states)) = many1(fsm_state_definition)(input)?;
  let ((input, _)) = period(input)?;
  Ok((input, FsmSpecification{name,input: input_vars,output,states}))
}

pub fn fsm_pattern(input: ParseString) -> ParseResult<Pattern> {
  let ((input, ptrn)) = match tuple_struct(input.clone()) {
    Ok((input, tpl)) => (input, Pattern::TupleStruct(tpl)),
    _ => match wildcard(input.clone()) {
      Ok((input, _)) => (input, Pattern::Wildcard),
      _ => match formula(input.clone()) {
        Ok((input, Factor::Expression(expr))) => (input, Pattern::Expression(*expr)),
        Ok((input, frmla)) => (input, Pattern::Formula(frmla)),
        Err(err) => {return Err(err)},
      },
    },
  };
  Ok((input, ptrn))
}

pub fn tuple_struct(input: ParseString) -> ParseResult<PatternTupleStruct> {
  let (input, id) = identifier(input)?;
  let ((input, _)) = left_parenthesis(input)?;
  let ((input, patterns)) = separated_list1(list_separator, fsm_pattern)(input)?;
  let ((input, _)) = right_parenthesis(input)?;
  Ok((input, PatternTupleStruct{name: id, patterns}))
}

pub fn fsm_state_definition(input: ParseString) -> ParseResult<StateDefinition> {
  let ((input, _)) = fsm_guard_operator(input)?;
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

// #### Statements

// comment_sigil ::= "--" ;
pub fn comment_sigil(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag("--")(input)?;
  Ok((input, ParserNode::Null))
}

// comment ::= (space | tab)*, comment_sigil, <text>, <!!new_line> ;
pub fn comment(input: ParseString) -> ParseResult<Comment> {
  let msg2 = "Character not allowed in comment text";
  let (input, _) = many0(whitespace)(input)?;
  let (input, _) = comment_sigil(input)?;
  let (input, text) = many1(text)(input)?;
  let (input, _) = many0(whitespace)(input)?;
  Ok((input, Comment{text}))
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
  /*let msg1 = "Expects spaces around operator";
  let msg2 = "Expects expression, inline table, or anonymous table";
  let (input, table_id) = table(input)?;
  let (input, _) = labelr!(null(is_not(stmt_operator)), skip_nil, msg1)(input)?;
  let (input, _) = many0(space)(input)?;
  let (input, _) = add_row_operator(input)?;
  let (input, _) = labelr!(null(many1(space)), skip_nil, msg1)(input)?;
  let (input, table) = label!(alt((expression, inline_table, anonymous_table)), msg2)(input)?;*/
  Ok((input, ParserNode::Error))
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
//   let (input, _) = tag(":Expects=")(input)?;
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
  /*let msg1 = "Expects spaces around operator";
  let msg2 = "Expects expression";
  let (input, table) = data(input)?;
  let (input, _) = labelr!(null(is_not(stmt_operator)), skip_nil, msg1)(input)?;
  let (input, _) = many0(space)(input)?;
  let (input, op) = update_operator(input)?;
  let (input, _) = labelr!(null(many1(space)), skip_nil, msg1)(input)?;
  let (input, expression) = label!(expression, msg2)(input)?;*/
  Ok((input, ParserNode::UpdateData{children: vec![]}))
}

// async_assign ::= (identifier | table), <!stmt_operator>, space*, async_assign_operator, <space+>, <expression> ;
pub fn async_assign(input: ParseString) -> ParseResult<ParserNode> {
  /*let msg1 = "Expects spaces around operator";
  let msg2 = "Expects expression";
  let (input, table) = alt((identifier, table))(input)?;
  let (input, _) = labelr!(null(is_not(stmt_operator)), skip_nil, msg1)(input)?;
  let (input, _) = many0(space)(input)?;
  let (input, _) = async_assign_operator(input)?;
  let (input, _) = labelr!(null(many1(space)), skip_nil, msg1)(input)?;
  let (input, expression) = label!(expression, msg2)(input)?;*/
  Ok((input, ParserNode::AsyncAssign{children: vec![]}))
}

// set_operator ::= ":=" ;
pub fn set_operator(input: ParseString) -> ParseResult<()> {
  let (input, _) = tag(":=")(input)?;
  Ok((input, ()))
}

// define_operator ::= ":=" ;
pub fn define_operator(input: ParseString) -> ParseResult<()> {
  let (input, _) = tag(":=")(input)?;
  Ok((input, ()))
}

// assign_operator ::= "=" ;
pub fn assign_operator(input: ParseString) -> ParseResult<()> {
  let (input, _) = tag("=")(input)?;
  Ok((input, ()))
}


// set_data ::= data, <!stmt_operator>, space*, set_operator, <space+>, <expression> ;
pub fn set_data(input: ParseString) -> ParseResult<ParserNode> {
  /*let msg1 = "Expects spaces around operator";
  let msg2 = "Expects expression";
  let (input, table) = data(input)?;
  let (input, _) = labelr!(null(is_not(stmt_operator)), skip_nil, msg1)(input)?;
  let (input, _) = many0(space)(input)?;
  let (input, _) = set_operator(input)?;
  let (input, _) = labelr!(null(many1(space)), skip_nil, msg1)(input)?;
  let (input, expression) = label!(expression, msg2)(input)?;*/
  Ok((input, ParserNode::SetData{children: vec![]}))
}

// split_data ::= (identifier | table), <!stmt_operator>, space*, split_operator, <space+>, <expression> ;
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

// flatten_data ::= identifier, <!stmt_operator>, space*, flatten_operator, <space+>, <expression> ;
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

// variable_define ::= identifier, <!stmt_operator>, space*, equal, <space+>, <expression> ;
pub fn variable_define(input: ParseString) -> ParseResult<VariableDefine> {
  let msg1 = "Expects spaces around operator";
  let msg2 = "Expects expression";
  let (input, var) = var(input)?;
  let (input, _) = labelr!(null(is_not(assign_operator)), skip_nil, msg1)(input)?;
  let (input, _) = many0(space)(input)?;
  let (input, _) = define_operator(input)?;
  let (input, _) = labelr!(null(many1(space)), skip_nil, msg1)(input)?;
  let (input, expression) = label!(expression, msg2)(input)?;
  Ok((input, VariableDefine{var,expression}))
}

// variable_define ::= identifier, <!stmt_operator>, space*, equal, <space+>, <expression> ;
pub fn variable_assign(input: ParseString) -> ParseResult<VariableAssign> {
  let msg1 = "Expects spaces around operator";
  let msg2 = "Expects expression";
  let (input, target) = expression(input)?;
  let (input, _) = labelr!(null(is_not(define_operator)), skip_nil, msg1)(input)?;
  let (input, _) = many0(space)(input)?;
  let (input, _) = assign_operator(input)?;
  let (input, _) = labelr!(null(many1(space)), skip_nil, msg1)(input)?;
  let (input, expression) = label!(expression, msg2)(input)?;
  Ok((input, VariableAssign{target,expression}))
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
  let(input, _) = new_line(input)?;
  Ok((input, ParserNode::Null))
}
pub fn formatted_table_columns(input: ParseString) -> ParseResult<ParserNode> {
  let(input, _) = tag("│")(input)?;
  let (input, attr) = many1(formatted_table_column)(input)?;
  let(input, _) = new_line(input)?;
  Ok((input, ParserNode::TableHeader { children: attr }))
}
pub fn formatted_table_column(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = many0(space)(input)?;
  let (input, item) = identifier(input)?;
  let (input, _) = many1(space)(input)?;
  let (input, _) = tag("│")(input)?;
  Ok((input, ParserNode::Attribute { children: vec![] }))
}
// parser for the second line of the output table, generate the 
// var name if there is one.
pub fn table_name(input: ParseString) -> ParseResult<ParserNode> {
  /*let(input, _) = tag("│")(input)?;
  let(input, table_name) = table(input)?;
  let(input, s) = many0(alt((space, left_parenthesis, right_parenthesis, word, number)))(input)?;
  let(input, _) = tag("│")(input)?;
  let(input, _) = new_line(input)?;*/
  Ok((input,ParserNode::Error))
}
pub fn table_kinds(input: ParseString) -> ParseResult<ParserNode> {
  let(input, _) = tag("│")(input)?;
  let (input, _) = many1(table_kind)(input)?;
  let(input, _) = new_line(input)?;
  Ok((input, ParserNode::Error))
}
pub fn table_kind(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = many0(space)(input)?;
  let (input, kind_id) = identifier(input)?;
  let (input, _) = many1(space)(input)?;
  let (input, _) = tag("│")(input)?;
  Ok((input, ParserNode::Error))
}
pub fn table_items(input: ParseString) -> ParseResult<ParserNode> {
  let(input, _) = tag("│")(input)?;
  let (input, mut table_items) = many1(table_item)(input)?;
  let(input, _) = new_line(input)?;
  Ok((input, ParserNode::Error))
}
pub fn table_item(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = many0(space)(input)?;
  let (input, item) = expression(input)?;
  let (input, _) = many1(space)(input)?;
  let (input, _) = tag("│")(input)?;
  Ok((input, ParserNode::Error))
}
pub fn table_select(input: ParseString) -> ParseResult<ParserNode> {
  let (input, expression) = expression(input)?;
  Ok((input, ParserNode::Error))
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

// statement ::= (followed_by  | async_assign  | table_define | variable_define | split_data  | flatten_data | whenever_data | wait_data |
// >>             until_data   | set_data     | update_data     | add_row     | comment ), space*, <new_line+> ;
pub fn statement(input: ParseString) -> ParseResult<Statement> {
  let msg = "Expects new_line or semicolon to terminate statement";
  //let (input, (statement, src_range)) = range(alt((followed_by, async_assign, table_define, variable_define, split_data, flatten_data, whenever_data, wait_data, until_data, set_data, update_data, add_row, comment)))(input)?;
  let (input, statement) = match variable_define(input.clone()) {
    Ok((input, var_def)) => (input, Statement::VariableDefine(var_def)),
    _ => match variable_assign(input.clone()) {
      Ok((input, var_asgn)) => (input, Statement::VariableAssign(var_asgn)),
      _ => match kind_define(input.clone()) {
        Ok((input, knd_def)) => ((input, Statement::KindDefine(knd_def))),
        Err(err) => {return Err(err);}   
      },
    },
  };
  let (input, _) = many0(space)(input)?;
  Ok((input, statement))
}

pub fn kind_define(input: ParseString) -> ParseResult<KindDefine> {
  let (input, _) = left_angle(input)?;
  let (input, name) = identifier(input)?;
  let (input, _) = right_angle(input)?;
  let (input, _) = many1(whitespace)(input)?;
  let (input, _) = define_operator(input)?;
  let (input, _) = many1(whitespace)(input)?;
  let (input, knd) = kind_annotation(input)?;
  Ok((input, KindDefine{name,definition: knd}))
}

// #### Expressions

// ##### Math expressions

// parenthetical_expression ::= left_parenthesis, <l0>, <right_parenthesis> ;
pub fn parenthetical_term(input: ParseString) -> ParseResult<Factor> {
  let msg1 = "Expects expression";
  let msg2 = "Expects right parenthesis ')'";
  let (input, (_, r)) = range(left_parenthesis)(input)?;
  let (input, frmla) = label!(formula, msg1)(input)?;
  let (input, _) = label!(right_parenthesis, msg2, r)(input)?;
  Ok((input, frmla))
}

// TODO: This won't parse -(5 - 3)
// negation ::= dash, !(dash | space), <data | value> ;
pub fn negation(input: ParseString) -> ParseResult<ParserNode> {
  let msg = "Expects a value to immediately follow the negation sign";
  let (input, (_, r)) = range(dash)(input)?;
  let (input, _) = is_not(alt((dash, space)))(input)?;  // so it's not comment sigil
  let (input, negated) = label!(data, msg, r)(input)?;
  Ok((input, ParserNode::Negation { children: vec![negated] }))
}

// function ::= identifier, left_parenthesis, <function_binding+>, <right_parenthesis> ;
pub fn function(input: ParseString) -> ParseResult<ParserNode> {
  /*let msg1 = "Expects function binding";
  let msg2 = "Expects right parenthesis ')'";
  let (input, identifier) = identifier(input)?;
  let (input, (_, r)) = range(left_parenthesis)(input)?;
  let (input, mut bindings) = label!(many1(function_binding), msg1)(input)?;
  let (input, _) = label!(right_parenthesis, msg2, r)(input)?;
  let mut function = vec![identifier];
  function.append(&mut bindings);*/
  Ok((input, ParserNode::Function { children: vec![] }))
}

// user_function ::= left_bracket, function_output*, <right_bracket>, <space+>, <equal>, <space+>, <identifier>,
// >>                <left_parenthesis>, <function_input*>, <right_parenthesis>, <new_line>, <function_body> ;
pub fn user_function(input: ParseString) -> ParseResult<ParserNode> {
  let msg1 = "Expects right bracket for user function definition";
  let msg2 = "Expects space after output declaration";
  let msg3 = "Expects equal sign '='";
  let msg4 = "Expects space after equal sign";
  let msg5 = "Expects identifier for function name";
  let msg6 = "Expects left parenthesis '('";
  let msg7 = "Expects right parenthesis ')'";
  let msg8 = "Expects new_line after user function header";
  let msg9 = "Expects indented transformations for function body";
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
  let (input, _) = label!(new_line, msg8)(input)?;
  let end = input.loc();
  let (input, function_body) = label!(function_body, msg9, SourceRange {start, end})(input)?;
  Ok((input, ParserNode::UserFunction { children: vec![] }))
}

// function_output ::= identifier, <kind_annotation>, space*, comma?, space* ;
pub fn function_output(input: ParseString) -> ParseResult<ParserNode> {
  let msg = "Expects kind annotation";
  let (input, arg_id) = identifier(input)?;
  let (input, kind) = label!(kind_annotation, msg)(input)?;
  let (input, _) = nom_tuple((many0(space), opt(comma), many0(space)))(input)?;
  Ok((input, ParserNode::FunctionOutput{children: vec![]}))
}

// function_input ::= identifier, <kind_annotation>, space*, comma?, space* ;
pub fn function_input(input: ParseString) -> ParseResult<ParserNode> {
  let msg = "Expects kind annotation";
  let (input, arg_id) = identifier(input)?;
  let (input, kind) = label!(kind_annotation, msg)(input)?;
  let (input, _) = nom_tuple((many0(space), opt(comma), many0(space)))(input)?;
  Ok((input, ParserNode::FunctionInput{children: vec![]}))
}

// function_body ::= indented_tfm+, whitespace* ;
pub fn function_body(input: ParseString) -> ParseResult<ParserNode> {
  //let (input, transformations) = many1(indented_tfm)(input)?;
  //let (input, _) = many0(whitespace)(input)?;
  Ok((input, ParserNode::Error))
}

// add ::= "+" ;
pub fn add(input: ParseString) -> ParseResult<AddSubOp> {
  let (input, _) = many1(whitespace)(input)?;
  let (input, _) = tag("+")(input)?;
  let (input, _) = many1(whitespace)(input)?;
  Ok((input, AddSubOp::Add))
}

// subtract ::= "-" ;
pub fn subtract(input: ParseString) -> ParseResult<AddSubOp> {
  let (input, _) = many1(whitespace)(input)?;
  let (input, _) = tag("-")(input)?;
  let (input, _) = many1(whitespace)(input)?;
  Ok((input, AddSubOp::Sub))
}

// multiply ::= "*" ;
pub fn multiply(input: ParseString) -> ParseResult<MulDivOp> {
  let (input, _) = many1(whitespace)(input)?;
  let (input, _) = tag("*")(input)?;
  let (input, _) = many1(whitespace)(input)?;
  Ok((input, MulDivOp::Mul))
}

// divide ::= "/" ;
pub fn divide(input: ParseString) -> ParseResult<MulDivOp> {
  let (input, _) = many1(whitespace)(input)?;
  let (input, _) = tag("/")(input)?;
  let (input, _) = many1(whitespace)(input)?;
  Ok((input, MulDivOp::Div))
}


// matrix_multiply ::= "**" ;
pub fn matrix_multiply(input: ParseString) -> ParseResult<MulDivOp> {
  let (input, _) = many1(whitespace)(input)?;
  let (input, _) = tag("**")(input)?;
  let (input, _) = many1(whitespace)(input)?;
  Ok((input, MulDivOp::MatMul))
}

// matrix_solve ::= "\" ;
pub fn matrix_solve(input: ParseString) -> ParseResult<MulDivOp> {
  let (input, _) = many1(whitespace)(input)?;
  let (input, _) = tag("\\")(input)?;
  let (input, _) = many1(whitespace)(input)?;
  Ok((input, MulDivOp::Solve))
}

// exponent ::= "^" ;
pub fn exponent(input: ParseString) -> ParseResult<ExponentOp> {
  let (input, _) = many1(whitespace)(input)?;
  let (input, _) = tag("^")(input)?;
  let (input, _) = many1(whitespace)(input)?;
  Ok((input, ExponentOp::Exp))
}

// range_op ::= colon ;
pub fn range_op(input: ParseString) -> ParseResult<RangeOp> {
  let (input, _) = colon(input)?;
  Ok((input, RangeOp::Inclusive))
}

// l0_op ::= range_op ;
pub fn range_operator(input: ParseString) -> ParseResult<FormulaOperator> {
  let (input, op) = range_op(input)?;
  Ok((input, FormulaOperator::Range(op)))
}

// l0 ::= l1, l0_infix* ;
pub fn formula(input: ParseString) -> ParseResult<Factor> {
  let (input, lhs) = l1(input)?;
  let (input, rhs) = many0(nom_tuple((range_operator,l1)))(input)?;
  let factor = if rhs.is_empty() { lhs } else { Factor::Term(Box::new(Term { lhs, rhs })) };
  Ok((input, factor))
}

// l1_op ::= add | subtract ;
pub fn add_sub_operator(input: ParseString) -> ParseResult<FormulaOperator> {
  let (input, op) = alt((add, subtract))(input)?;
  Ok((input, FormulaOperator::AddSub(op)))
}

// l1 ::= l2, l1_infix* ;
pub fn l1(input: ParseString) -> ParseResult<Factor> {
  let (input, lhs) = l2(input)?;
  let (input, rhs) = many0(nom_tuple((add_sub_operator,l2)))(input)?;
  let factor = if rhs.is_empty() { lhs } else { Factor::Term(Box::new(Term { lhs, rhs })) };
  Ok((input, factor))
}

// l2_op ::= matrix_multiply | multiply | divide | matrix_solve ;
pub fn mul_div_operator(input: ParseString) -> ParseResult<FormulaOperator> {
  let (input, op) = alt((matrix_multiply, multiply, divide, matrix_solve))(input)?;
  Ok((input, FormulaOperator::MulDiv(op)))
}

// l2 ::= l3, l2_infix* ;
pub fn l2(input: ParseString) -> ParseResult<Factor> {
  let (input, lhs) = l3(input)?;
  let (input, rhs) = many0(nom_tuple((mul_div_operator,l3)))(input)?;
  let factor = if rhs.is_empty() { lhs } else { Factor::Term(Box::new(Term { lhs, rhs })) };
  Ok((input, factor))
}

// l3_op ::= exponent ;
pub fn exponent_operator(input: ParseString) -> ParseResult<FormulaOperator> {
  let (input, op) = exponent(input)?;
  Ok((input, FormulaOperator::Exponent(op)))
}

// l3 ::= l4, l3_infix* ;
pub fn l3(input: ParseString) -> ParseResult<Factor> {
  let (input, lhs) = l4(input)?;
  let (input, rhs) = many0(nom_tuple((exponent_operator,l4)))(input)?;
  let factor = if rhs.is_empty() { lhs } else { Factor::Term(Box::new(Term { lhs, rhs })) };
  Ok((input, factor))
}

// l4_op ::= and | or | xor ;
pub fn logic_operator(input: ParseString) -> ParseResult<FormulaOperator> {
  let (input, op) = alt((and, or, xor))(input)?;
  Ok((input, FormulaOperator::Logic(op)))
}

// l4 ::= l5, l4_infix* ;
pub fn l4(input: ParseString) -> ParseResult<Factor> {
  let (input, lhs) = l5(input)?;
  let (input, rhs) = many0(nom_tuple((logic_operator,l5)))(input)?;
  let factor = if rhs.is_empty() { lhs } else { Factor::Term(Box::new(Term { lhs, rhs })) };
  Ok((input, factor))
}

// l5 ::= l6, l5_infix* ;
pub fn l5(input: ParseString) -> ParseResult<Factor> {
  let (input, lhs) = factor(input)?;
  let (input, rhs) = many0(nom_tuple((comparison_operator,factor)))(input)?;
  let factor = if rhs.is_empty() { lhs } else { Factor::Term(Box::new(Term { lhs, rhs })) };
  Ok((input, factor))
}

// l5_op ::= not_equal | equal_to | greater_than_equal | greater_than | less_than_equal | less_than ;
pub fn comparison_operator(input: ParseString) -> ParseResult<FormulaOperator> {
  let (input, op) = alt((not_equal, equal_to, greater_than_equal, greater_than, less_than_equal, less_than))(input)?;
  Ok((input, FormulaOperator::Comparison(op)))
}

// l6 ::= literal | data | slice | table | parenthetical_expression ;
pub fn factor(input: ParseString) -> ParseResult<Factor> {
  let (input, fctr) = match parenthetical_term(input.clone()) {
    Ok((input, term)) => (input, term),
    _ => match structure(input.clone()) {
      Ok((input, strct)) => (input, Factor::Expression(Box::new(Expression::Structure(strct)))),
      _ => match literal(input.clone()) {
        Ok((input, ltrl)) => (input, Factor::Expression(Box::new(Expression::Literal(ltrl)))),
        _ => match slice(input.clone()) {
          Ok((input, slc)) => (input, Factor::Expression(Box::new(Expression::Slice(slc)))),
          _ => match var(input.clone()) {
            Ok((input, var)) => (input, Factor::Expression(Box::new(Expression::Var(var)))),
            Err(err) => {return Err(err);}
          },
        },
      },
    },
  };
  /*let (input, transpose) = opt(transpose)(input)?;
  let fctr = match transpose {
    Some(_) => Factor::Transpose(Box::new(fctr)),
    None => fctr,
  };*/
  Ok((input, fctr))
}

fn var(input: ParseString) -> ParseResult<Var> {
  let ((input, name)) = identifier(input)?;
  let ((input, kind)) = opt(kind_annotation)(input)?;
  Ok((input, Var{ name, kind }))
}

// ##### Filter expressions

// not_equal ::= "!=" | "¬=" | "≠" ;
pub fn not_equal(input: ParseString) -> ParseResult<ComparisonOp> {
  let (input, _) = many1(whitespace)(input)?;
  let (input, _) = alt((tag("!="),tag("¬="),tag("≠")))(input)?;
  let (input, _) = many1(whitespace)(input)?;
  Ok((input, ComparisonOp::NotEqual))
}

// equal_to ::= "==" ;
pub fn equal_to(input: ParseString) -> ParseResult<ComparisonOp> {
  let (input, _) = many1(whitespace)(input)?;
  let (input, _) = tag("==")(input)?;
  let (input, _) = many1(whitespace)(input)?;
  Ok((input, ComparisonOp::Equal))
}

// greater_than ::= ">" ;
pub fn greater_than(input: ParseString) -> ParseResult<ComparisonOp> {
  let (input, _) = many1(whitespace)(input)?;
  let (input, _) = tag(">")(input)?;
  let (input, _) = many1(whitespace)(input)?;
  Ok((input, ComparisonOp::GreaterThan))
}

// less_than ::= "<" ;
pub fn less_than(input: ParseString) -> ParseResult<ComparisonOp> {
  let (input, _) = many1(whitespace)(input)?;
  let (input, _) = tag("<")(input)?;
  let (input, _) = many1(whitespace)(input)?;
  Ok((input, ComparisonOp::LessThan))
}

// greater_than_equal ::= ">=" | "≥" ;
pub fn greater_than_equal(input: ParseString) -> ParseResult<ComparisonOp> {
  let (input, _) = many1(whitespace)(input)?;
  let (input, _) = alt((tag(">="),tag("≥")))(input)?;
  let (input, _) = many1(whitespace)(input)?;
  Ok((input, ComparisonOp::GreaterThanEqual))
}

// less_than_equal ::= "<=" | "≤" ;
pub fn less_than_equal(input: ParseString) -> ParseResult<ComparisonOp> {
  let (input, _) = many1(whitespace)(input)?;
  let (input, _) = alt((tag("<="),tag("≤")))(input)?;
  let (input, _) = many1(whitespace)(input)?;
  Ok((input, ComparisonOp::LessThanEqual))
}

// ##### Logic expressions

// or ::= "|" ;
pub fn or(input: ParseString) -> ParseResult<LogicOp> {
  let (input, _) = many1(whitespace)(input)?;
  let (input, _) = tag("|")(input)?;
  let (input, _) = many1(whitespace)(input)?;
  Ok((input, LogicOp::Or))
}

// and ::= "&" ;
pub fn and(input: ParseString) -> ParseResult<LogicOp> {
  let (input, _) = many1(whitespace)(input)?;
  let (input, _) = tag("&")(input)?;
  let (input, _) = many1(whitespace)(input)?;
  Ok((input, LogicOp::And))
}

// not ::= "!" | "¬" ;
pub fn not(input: ParseString) -> ParseResult<LogicOp> {
  let (input, _) = many1(whitespace)(input)?;
  let (input, _) = alt((tag("!"), tag("¬")))(input)?;
  let (input, _) = many1(whitespace)(input)?;
  Ok((input, LogicOp::Not))
}

// xor ::= "xor" | "⊕" | "⊻" ;
pub fn xor(input: ParseString) -> ParseResult<LogicOp> {
  let (input, _) = many1(whitespace)(input)?;
  let (input, _) = alt((tag("xor"), tag("⊕"), tag("⊻")))(input)?;
  let (input, _) = many1(whitespace)(input)?;
  Ok((input, LogicOp::Xor))
}

// ##### Other expressions

// pub fn string_interpolation(input: ParseString) -> IResult<ParseString, ParserNode> {
//   let (input, _) = tag("{{")(input)?;
//   let (input, expression) = expression(input)?;
//   let (input, _) = tag("}}")(input)?;
//   Ok((input, ParserNode::StringInterpolation { children: vec![expression] }))
// }

// string ::= quote, (!quote, <text>)*, quote ;
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

// transpose ::= "'" ;
pub fn transpose(input: ParseString) -> ParseResult<()> {
  let (input, _) = tag("'")(input)?;
  Ok((input, ()))
}

pub fn literal(input: ParseString) -> ParseResult<Literal> {
  let (input, result) = match number(input.clone()) {
    Ok((input, number)) => (input, Literal::Number(number)),
    _ => match string(input.clone()) {
      Ok((input, string)) => (input, Literal::String(string)),
      _ => match boolean(input.clone()) {
        Ok((input, boolean)) => (input, Literal::Boolean(boolean)),
        _ => match empty(input.clone()) {
          Ok((input, empty)) => (input, Literal::Empty(empty)), 
          Err(err) => {return Err(err);}
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

fn slice(input: ParseString) -> ParseResult<(Identifier,Vec<Expression>)> {
  let (input, name) = identifier(input)?;
  let (input, _) = left_bracket(input)?;
  let (input, ixes) = separated_list1(list_separator,expression)(input)?;
  let (input, _) = right_bracket(input)?;
  Ok((input, (name, ixes)))
}

pub fn tuple(input: ParseString) -> ParseResult<Tuple> {
  let (input, _) = left_parenthesis(input)?;
  let (input, _) = many0(whitespace)(input)?;
  let (input, exprs) = separated_list0(list_separator, expression)(input)?;
  let (input, _) = many0(whitespace)(input)?;
  let (input, _) = right_parenthesis(input)?;
  Ok((input, Tuple{elements: exprs}))
}

// expression ::= (empty_table | inline_table | math_expression | string | anonymous_table), transpose? ;
pub fn expression(input: ParseString) -> ParseResult<Expression> {
  let (input, expression) = match formula(input.clone()) {
    Ok((input, Factor::Expression(expr))) => (input, *expr),
    Ok((input, fctr)) => (input, Expression::Formula(fctr)),
    Err(err) => {return Err(err);}
  };
  let (input, transpose) = opt(transpose)(input)?;
  let expr = match transpose {
    Some(_) => Expression::Transpose(Box::new(expression)),
    None => expression,
  };
  Ok((input, expr))
}

// #### Block basics

// transformation ::= statement;
pub fn transformation(input: ParseString) -> ParseResult<ParserNode> {
  let (input, statement) = statement(input)?;
  Ok((input, ParserNode::Transformation { children: vec![] }))
}

// empty_line ::= space*, new_line ;
pub fn empty_line(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = nom_tuple((many0(space), new_line))(input)?;
  Ok((input, ParserNode::Null))
}

// indented_tfm ::= !empty_line, space, <space>, <!space>, <transformation> ;
/*pub fn indented_tfm(input: ParseString) -> ParseResult<ParserNode> {
  let msg1 = "Indentation has to be exactly 2 spaces";
  let msg2 = "Expects transformation";
  let (input, _) = nom_tuple((
    is_not(empty_line),
    space,
    labelr!(space, skip_nil, msg1),
    labelr!(is_not(space), skip_spaces, msg1),
  ))(input)?;
  label!(transformation, msg2)(input)
}*/

// block ::= indented_tfm+, whitespace* ;
pub fn block(input: ParseString) -> ParseResult<ParserNode> {
  //let (input, (transformations, src_range)) = range(many1(indented_tfm))(input)?;
  //let (input, _) = many0(whitespace)(input)?;
  Ok((input, ParserNode::Error))
}

// ### Mechdown

// title ::= text+, new_line, equal+, (space|tab)*, whitespace* ;
pub fn title(input: ParseString) -> ParseResult<Title> {
  let (input, mut text) = many1(text)(input)?;
  let (input, _) = new_line(input)?;
  let (input, _) = many1(equal)(input)?;
  let (input, _) = many0(alt((space,tab)))(input)?;
  let (input, _) = new_line(input)?;
  let (input, _) = many0(alt((space,tab)))(input)?;
  let (input, _) = many0(whitespace)(input)?;
  let mut title = merge_tokens(&mut text).unwrap();
  title.kind = TokenKind::Title;
  Ok((input, Title{text: title}))
}

// subtitle ::= text+, new_line, dash+, (space|tab)*, whitespace* ;
pub fn ul_subtitle(input: ParseString) -> ParseResult<Subtitle> {
  let (input, _) = many1(digit_token)(input)?;
  let (input, _) = period(input)?;
  let (input, _) = many0(space)(input)?;
  let (input, mut text) = many1(text)(input)?;
  let (input, _) = new_line(input)?;
  let (input, _) = many1(dash)(input)?;
  let (input, _) = many0(alt((space,tab)))(input)?;
  let (input, _) = new_line(input)?;
  let (input, _) = many0(alt((space,tab)))(input)?;
  let (input, _) = many0(whitespace)(input)?;
  let mut title = merge_tokens(&mut text).unwrap();
  title.kind = TokenKind::Title;
  Ok((input, Subtitle{text: title}))
}

// number_subtitle ::= space*, number, period, space+, text, space*, new_line* ;
pub fn number_subtitle(input: ParseString) -> ParseResult<Subtitle> {
  let (input, _) = many0(alt((space,tab)))(input)?;
  let (input, _) = left_parenthesis(input)?;
  let (input, _) = integer_literal(input)?;
  let (input, _) = right_parenthesis(input)?;
  let (input, _) = many1(alt((space,tab)))(input)?;
  let (input, mut text) = many1(text)(input)?;
  let (input, _) = many0(alt((space,tab)))(input)?;
  let (input, _) = many0(whitespace)(input)?;
  let mut title = merge_tokens(&mut text).unwrap();
  title.kind = TokenKind::Title;
  Ok((input, Subtitle{text: title}))
}

// alpha_subtitle ::= space*, alpha, right_parenthesis, space+, text, space*, new_line* ;
pub fn alpha_subtitle(input: ParseString) -> ParseResult<Subtitle> {
  let (input, _) = many0(alt((space,tab)))(input)?;
  let (input, _) = left_parenthesis(input)?;
  let (input, _) = alpha(input)?;
  let (input, _) = right_parenthesis(input)?;
  let (input, _) = many0(alt((space,tab)))(input)?;
  let (input, mut text) = many1(text)(input)?;
  let (input, _) = many0(alt((space,tab)))(input)?;
  let (input, _) = many0(whitespace)(input)?;
  let mut title = merge_tokens(&mut text).unwrap();
  title.kind = TokenKind::Title;
  Ok((input, Subtitle{text: title}))
}

// inline_code ::= grave, text, grave, space* ;
pub fn inline_code(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = grave(input)?;
  let (input, text) = text(input)?;
  let (input, _) = grave(input)?;
  let (input, _) = many0(space)(input)?;
  Ok((input, ParserNode::Error))
}

// paragraph_symbol ::= ampersand | at | slash | backslash | asterisk | caret | hashtag | underscore ;
pub fn paragraph_symbol(input: ParseString) -> ParseResult<Token> {
  let (input, symbol) = alt((ampersand, at, slash, backslash, asterisk, caret, hashtag, underscore, equal, tilde, plus, percent))(input)?;
  Ok((input, symbol))
}

// paragraph_rest ::= (word | space | number | punctuation | paragraph_symbol | quote | emoij)+ ;
pub fn paragraph_rest(input: ParseString) -> ParseResult<ParserNode> {
  //let (input, word) = many1(alt((word, space, number, punctuation, paragraph_symbol, quote, emoji)))(input)?;
  Ok((input, ParserNode::Error))
}

// paragraph_starter ::= (word | number | quote | left_angle | right_angle | left_bracket | right_bracket | period | exclamation | question | comma | colon | semicolon | left_parenthesis | right_parenthesis | emoji)+ ;
pub fn paragraph_starter(input: ParseString) -> ParseResult<ParagraphElement> {
  let (input, text) = alt((alpha_token, quote))(input)?;
  Ok((input, ParagraphElement::Start(text)))
}

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

// paragraph ::= (inline_code | paragraph_text)+, whitespace*, new_line* ;
pub fn paragraph(input: ParseString) -> ParseResult<Paragraph> {
  let (input, first) = paragraph_starter(input)?;
  let (input, mut rest) = many0(paragraph_element)(input)?;
  let mut elements = vec![first];
  elements.append(&mut rest);
  Ok((input, Paragraph{elements}))
}

// unordered_list ::= list_item+, new_line?, whitespace* ;
pub fn unordered_list(input: ParseString) -> ParseResult<UnorderedList> {
  let (input, items) = many1(list_item)(input)?;
  let (input, _) = many0(whitespace)(input)?;
  Ok((input,  UnorderedList{items}))
}

// list_item ::= dash, <space+>, <paragraph>, new_line* ;
pub fn list_item(input: ParseString) -> ParseResult<Paragraph> {
  let msg1 = "Expects space after dash";
  let msg2 = "Expects paragraph as list item";
  let (input, _) = dash(input)?;
  let (input, _) = labelr!(null(many1(space)), skip_nil, msg1)(input)?;
  let (input, list_item) = label!(paragraph, msg2)(input)?;
  let (input, _) = many0(new_line)(input)?;
  Ok((input,  list_item))
}

// formatted_text ::= (!grave, !eof, <paragraph_rest | carriage_return | new_line_char>)* ;
pub fn formatted_text(input: ParseString) -> ParseResult<ParserNode> {
  /*let msg = "Character not permitted in formatted text";
  let (input, result) = many0(nom_tuple((
    nom_tuple((is_not(grave), is_not(eof))),
    label!(alt((paragraph_rest, new_line)), msg)
  )))(input)?;
  let (_, formatted): (((), ()), Vec<_>) = result.into_iter().unzip();*/
  Ok((input,  ParserNode::Error))
}

// code_block ::= grave, <grave>, <grave>, <new_line>, formatted_text, <grave{3}, new_line, whitespace*> ;
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
  let (input, text) = formatted_text(input)?;
  let (input, _) = label!(nom_tuple((grave, grave, grave, new_line, many0(whitespace))), msg3, r)(input)?;
  Ok((input, SectionElement::CodeBlock))
}


// pub fn inline_mech_code(input: ParseString) -> ParseResult<ParserNode> {
//   let (input, _) = nom_tuple((left_bracket,left_bracket))(input)?;
//   let (input, expression) = expression(input)?;
//   let (input, _) = nom_tuple((right_bracket,right_bracket,opt(space)))(input)?;
//   Ok((input, ParserNode::InlineMechCode{ children: vec![expression] }))
// }

// mech_code_block ::= grave{3}, !!"mec", <"mech:">, text?, <new_line>, <block>, <grave{3}, new_line>, whitespace* ;
/*pub fn mech_code_block(input: ParseString) -> ParseResult<SectionElement> {
  let msg1 = "Expects new_line";
  let msg2 = "Expects mech code block";
  let msg3 = "Expects the \"mech:\" tag";
  let msg4 = "Expects 3 graves followed by new_line to terminate the mech code block";
  let (input, (_, r)) = range(nom_tuple((grave, grave, grave)))(input)?;
  let (input, _) = nom_tuple((is(tag("mec")), labelr!(tag("mech:"), skip_empty_mech_directive, msg3)))(input)?;
  let (input, directive) = opt(text)(input)?;
  let (input, _) = label!(new_line, msg1)(input)?;
  let (input, mech_block) = label!(block, msg2)(input)?;
  let (input, _) = label!(nom_tuple((grave, grave, grave, new_line)), msg4, r)(input)?;
  let (input, _) = many0(whitespace)(input)?;
  let mut elements = vec![];
  match directive {
    Some(directive) => elements.push(directive),
    _ => (),
  }
  elements.push(mech_block);
  Ok((input, SectionElement::MechCode))
}*/

pub fn mech_code(input: ParseString) -> ParseResult<MechCode> {
  let (input, mech_code) = match fsm_specification(input.clone()) {
    Ok((input, fsm_spec)) => ((input, MechCode::FsmSpecification(fsm_spec))),
    _ => match fsm_implementation(input.clone()) {
      Ok((input, fsm_impl)) => ((input, MechCode::FsmImplementation(fsm_impl))),
      _ => match statement(input.clone()) {
        Ok((input, stmt)) => ((input, MechCode::Statement(stmt))),
        _ => match expression(input.clone()) {
          Ok((input, expr)) => ((input, MechCode::Expression(expr))),
          Err(err) => {return Err(err);}
        }
      }
    }
  };
  let (input, _) = alt((new_line, semicolon))(input)?;
  Ok((input, mech_code))
}

// ### Start here

// section_element ::= user_function | block | mech_code_block | code_block | statement | paragraph | unordered_list;
pub fn section_element(input: ParseString) -> ParseResult<SectionElement> {
  let (input, section_element) = match mech_code(input.clone()) {
    Ok((input, code)) => (input, SectionElement::MechCode(code)),
    _ => match unordered_list(input.clone()) {
      Ok((input, list)) => (input, SectionElement::UnorderedList(list)),
      _ => match comment(input.clone()) {
        Ok((input, comment)) => (input, SectionElement::Comment(comment)),
        _ => match paragraph(input.clone()) {
          Ok((input, p)) => (input, SectionElement::Paragraph(p)),
          _ => match code_block(input.clone()) {
            Ok((input, m)) => (input,SectionElement::CodeBlock),
            _ => match sub_section(input) {
              Ok((input, s)) => (input, SectionElement::Section(Box::new(s))),
              Err(err) => { return Err(err); }
            }
          }
        }
      }
    }
  };
  let (input, _) = many0(whitespace)(input)?;
  Ok((input, section_element))
}

// section_element ::= user_function | block | mech_code_block | code_block | statement | paragraph | unordered_list;
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
  let (input, _) = many0(whitespace)(input)?;
  Ok((input, section_element))
}

// section ::= (!eof, <section_element>, whitespace?)+ ;
pub fn section(input: ParseString) -> ParseResult<Section> {
  let msg = "Expects user function, block, mech code block, code block, statement, paragraph, or unordered list";
  let (input, subtitle) = opt(ul_subtitle)(input)?;
  let (input, elements) = many1(section_element)(input)?;
  Ok((input, Section{subtitle, elements}))
}

// section ::= (!eof, <section_element>, whitespace?)+ ;
pub fn sub_section(input: ParseString) -> ParseResult<Section> {
  let msg = "Expects user function, block, mech code block, code block, statement, paragraph, or unordered list";
  let (input, subtitle) = alpha_subtitle(input)?;
  let (input, elements) = many0(sub_section_element)(input)?;
  Ok((input, Section{subtitle: Some(subtitle), elements}))
}


// body ::= whitespace*, section+ ;
pub fn body(input: ParseString) -> ParseResult<Body> {
  let (input, _) = many0(whitespace)(input)?;
  let (input, sections) = many1(section)(input)?;
  let (input, _) = many0(whitespace)(input)?;
  Ok((input, Body{sections}))
}

// program ::= whitespace?, title?, <body>, whitespace?, space* ;
pub fn program(input: ParseString) -> ParseResult<Program> {
  let msg = "Expects program body";
  let (input, _) = many0(whitespace)(input)?;
  let (input, title) = opt(title)(input)?;
  //let (input, body) = labelr!(body, skip_nil, msg)(input)?;
  let (input, body) = body(input)?;
  let (input, _) = many0(whitespace)(input)?;
  Ok((input, Program{title, body}))
}

// pub fn raw_transformation(input: ParseString) -> ParseResult<ParserNode> {
//   let (input, statement) = statement(input)?;
//   let (input, _) = many0(alt((space,new_line,tab)))(input)?;
//   Ok((input, ParserNode::Transformation { children:  vec![statement] }))
// }

// pub fn parse_block(input: ParseString) -> ParseResult<ParserNode> {
//   let (input, transformations) = many1(raw_transformation)(input)?;
//   let (input, _) = many0(whitespace)(input)?;
//   Ok((input, ParserNode::Block { children:  transformations }))
// }

// parse_mech ::= program | statement ;
pub fn parse_mech(input: ParseString) -> ParseResult<Program> {
  //let (input, mech) = alt((program, statement))(input)?;
  //Ok((input, ParserNode::Root { children: vec![mech] }))
  let (input, mech) = program(input)?;
  Ok((input, mech))
}

// 7. Reporting errors
// -----------------------

/// This struct is responsible for analysing text, interpreting indices
/// and ranges, and producing formatted messages.
pub struct TextFormatter<'a> {
  graphemes: Vec<&'a str>,
  line_beginnings: Vec<usize>,
  end_index: usize,
}

impl<'a> TextFormatter<'a> {
  pub fn new(text: &'a str) -> Self {
    let graphemes = graphemes::init_source(text);
    let mut line_beginnings = vec![0];
    for i in 0..graphemes.len() {
      if graphemes::is_new_line(graphemes[i]) {
        line_beginnings.push(i + 1);
      }
    }
    line_beginnings.pop();
    TextFormatter {
      end_index: graphemes.len(),
      graphemes,
      line_beginnings,
    }
  }

  // Index interpreter

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
    let (start, end) = match self.get_line_range(linenum) {
      Some(v) => v,
      None => return "\n".to_string(),
    };
    let mut s = self.graphemes[start..end].iter().map(|s| *s).collect::<String>();
    if !s.ends_with("\n") {
      s.push('\n');
    }
    s
  }

  fn get_textlen_by_linenum(&self, linenum: usize) -> usize {
    let (start, end) = match self.get_line_range(linenum) {
      Some(v) => v,
      None => return 1,
    };
    let mut len = 0;
    for i in start..end {
      len += graphemes::width(self.graphemes[i]);
    }
    len + 1
  }

  // FormattedString printer

  fn heading_color(s: &str) -> String {
    s.truecolor(246, 192, 78).bold().to_string()
  }

  fn location_color(s: &str) -> String {
    s.truecolor(0,187,204).bold().to_string()
  }

  fn linenum_color(s: &str) -> String {
    s.truecolor(0,187,204).bold().to_string()
  }

  fn text_color(s: &str) -> String {
    s.to_string()
  }

  fn annotation_color(s: &str) -> String {
    s.truecolor(102,51,153).bold().to_string()
  }

  fn error_color(s: &str) -> String {
    s.truecolor(170,51,85).bold().to_string()
  }

  fn ending_color(s: &str) -> String {
    s.truecolor(246, 192, 78).bold().to_string()
  }

  fn err_heading(index: usize) -> String {
    let n = index + 1;
    let d = "────────────────────────";
    let s = format!("{} syntax error #{} {}\n", d, n, d);
    Self::heading_color(&s)
  }

  fn err_location(&self, ctx: &ParserErrorContext) -> String {
    let err_end = ctx.cause_rng.end;
    // error range will not ends at first column, so `minus 1` here is safe
    let (row, col) = (err_end.row, err_end.col - 1);
    let s = format!("@location:{}:{}\n", row, col);
    Self::location_color(&s)
  }

  fn err_context(&self, ctx: &ParserErrorContext) -> String {
    let mut result = String::new();

    let mut annotation_rngs = ctx.annotation_rngs.clone();
    annotation_rngs.push(ctx.cause_rng);

    // the lines to print (1-indexed)
    let mut lines_to_print: Vec<usize> = vec![];
    for rng in &annotation_rngs {
      let r1 = rng.start.row;
      // if range ends at first column, it doesn't reach that row
      let r2 = if rng.end.col == 1 {
        usize::max(rng.start.row, rng.end.row - 1)
      } else {
        rng.end.row
      };
      for i in r1..=r2 {
        lines_to_print.push(i);
      }
    }
    lines_to_print.sort();
    lines_to_print.dedup();

    // the annotations on each line
    // <linenum, Vec<(start_col, rng_len, is_major, is_cause)>>
    let mut range_table: HashMap<usize, Vec<(usize, usize, bool, bool)>> = HashMap::new();
    for linenum in &lines_to_print {
      range_table.insert(*linenum, vec![]);
    }
    let n = annotation_rngs.len() - 1;  // if i == n, it's the last rng, i.e. the cause rng
    for (i, rng) in annotation_rngs.iter().enumerate() {
      // c2 might be 0
      let (r1, c1) = (rng.start.row, rng.start.col);
      let (r2, c2) = (rng.end.row, rng.end.col - 1);
      if r1 == r2 {  // the entire range is on one line
        if c2 >= c1 {  // and the range has non-zero length
          range_table.get_mut(&r1).unwrap().push((c1, c2 - c1 + 1, true, i == n));
        }
      } else {  // the range spans over multiple lines
        range_table.get_mut(&r1).unwrap().push((c1, usize::MAX, i != n, i == n));
        for r in r1+1..r2 {
          range_table.get_mut(&r).unwrap().push((1, usize::MAX, false, i == n));
        }
        if c2 != 0 {  // only add the last line if it hfnas non-zero length
          range_table.get_mut(&r2).unwrap().push((1, c2, i == n, i == n));
        }
      }
    }

    // other data for printing
    let dots = "...";
    let indentation = " ";
    let vert_split1 = " │";
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
      let text = self.get_text_by_linenum(lines_to_print[i]);
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
      let line_len = self.get_textlen_by_linenum(lines_to_print[i]);
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

    // print error message;
    // error range never ends at first column, so it's safe to `minus 1` here
    let cause_col = ctx.cause_rng.end.col - 1;
    result.push_str(indentation);
    for _ in 0..row_str_len { result.push(' '); }
    result.push_str(vert_split2);
    for _ in 0..cause_col-1 { result.push(' '); }
    result.push_str(&Self::error_color(&ctx.err_message));
    result.push('\n');

    result
  }

  fn err_ending(d: usize) -> String {
    let s = format!("... and {} other error{} not shown\n", d, if d == 1 {""} else {"s"});
    Self::heading_color(&s)
  }

  /// Get formatted error message.
  pub fn format_error(&self, errors: &ParserErrorReport) -> String {
    let n = usize::min(errors.len(), 10);
    let mut result = String::new();
    result.push('\n');
    for i in 0..n {
      let ctx = &errors[i];
      result.push_str(&Self::err_heading(i));
      result.push_str(&self.err_location(ctx));
      result.push_str(&self.err_context(ctx));
      result.push_str("\n\n");
    }
    let d = errors.len() - n;
    if d != 0 {
      result.push_str(&Self::err_ending(d));
    }
    result
  }
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
    Err(MechError{msg: "".to_string(), id: 3202, kind: MechErrorKind::ParserError(ParserNode::Error, report, msg)})
  }
}