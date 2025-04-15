// Parser
// ========

/// Sections:
///   1. Prelude
///   2. Parser combinators
///   3. Recovery functions
///   4. Public interface
///   5. Error reporting

// 1. Prelude
// ------------

use crate::*;
use crate::expressions::function_define;

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
  multi::{many1, many_till, many0, separated_list1, separated_list0},
  Err,
  Err::Failure
};

use std::collections::HashMap;
use colored::*;

//use crate::*;
use crate::{
  TextFormatter,
  ParseError,
  ParseString,
  ParseErrorDetail,
  graphemes,
  ParseResult,
};

// 2. Parser combinators
// -----------------------

/// Convert output of any parser into ParserNode::Null.
/// Useful for working with `alt` combinator and error recovery functions.
pub fn null<'a, F, O>(mut parser: F) ->
  impl FnMut(ParseString<'a>) -> ParseResult<()>
where
  F: FnMut(ParseString<'a>) -> ParseResult<O>
{
  move |input: ParseString| match parser(input) {
    Ok((remaining, _)) => Ok((remaining, ())),
    Err(Err::Error(e)) => Err(Err::Error(e)),
    Err(Err::Failure(e)) => Err(Err::Failure(e)),
    x => panic!("Err::Incomplete is not supported"),
  }
}

/// For parser p, run p and also output the range that p has matched
/// upon success.
pub fn range<'a, F, O>(mut parser: F) ->
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

#[macro_export]
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

#[macro_export]
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
pub fn label_without_recovery<'a, F, O>(
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
pub fn label_with_recovery<'a, F, O>(
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
pub fn is<'a, F, O>(mut parser: F) ->
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
pub fn is_not<'a, F, E>(mut parser: F) ->
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

// 3. Recovery functions
// -----------------------

// skip_till_eol := (!new_line, any)* ;
pub fn skip_till_eol(input: ParseString) -> ParseResult<()> {
  let (input, _) = many0(nom_tuple((
    is_not(new_line),
    any,
  )))(input)?;
  Ok((input, ()))
}

// skip_past_eol := skip_till_eol, new_line ;
fn skip_past_eol(input: ParseString) -> ParseResult<()> {
  let (input, _) = skip_till_eol(input)?;
  let (input, _) = new_line(input)?;
  Ok((input, ()))
}

// skip_till_section_element := skip_past_eol, (!section_element, skip_past_eol)* ;
fn skip_till_section_element(input: ParseString) -> ParseResult<()> {
  if input.is_empty() {
    return Ok((input, ()));
  }
  let (input, _) = skip_past_eol(input)?;
  let (input, _) = many0(nom_tuple((
    is_not(section_element),
    skip_past_eol,
  )))(input)?;
  Ok((input, ()))
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

// skip_spaces := space* ;
pub fn skip_spaces(input: ParseString) -> ParseResult<()> {
  let (input, _) = many0(space)(input)?;
  Ok((input, ()))
}

// skip_nil := ;
pub fn skip_nil(input: ParseString) -> ParseResult<()> {
  Ok((input, ()))
}

// skip_empty_mech_directive := ;
pub fn skip_empty_mech_directive(input: ParseString) -> ParseResult<String> {
  Ok((input, String::from("mech:")))
}

// 4. Public interface
// ---------------------

// mech_code_alt := fsm_specification | fsm_implementation | function_define | statement | expression | comment ;
pub fn mech_code_alt(input: ParseString) -> ParseResult<MechCode> {
  let (input, _) = whitespace0(input)?;
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
    _ => ()
  }
  match comment(input.clone()) {
    Ok((input, cmnt)) => {return Ok((input, MechCode::Comment(cmnt)));},
    Err(err) => {return Err(err);}
  }
}

// This is here to satisfy the type checker for this: alt((new_line, semicolon, comment_token))(input)?;
pub fn comment_token(input: ParseString) -> ParseResult<Token> {
  let (input, c) = comment(input)?;
  Ok((input, c.text))
}

// mech_code := mech_code_alt, ("\n" | ";" | comment) ;
pub fn mech_code(input: ParseString) -> ParseResult<MechCode> {
  let (input, code) = mech_code_alt(input)?;
  let (input, _) = many0(space_tab)(input)?;
  let (input, _) = alt((new_line, semicolon, comment_token))(input)?;
  let (input, _) = whitespace0(input)?;
  Ok((input, code))
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

// 5. Error Reporting
// --------------------

/// Print formatted error message.
pub fn print_err_report(text: &str, report: &ParserErrorReport) {
  let msg = TextFormatter::new(text).format_error(report);
  println!("{}", msg);
}

pub fn parse_grammar(text: &str) -> MResult<Grammar> {
  // remove all whitespace from the input string
  let text_no_Ws = &text.replace(" ", "").replace("\n", "").replace("\r", "").replace("\t", "");
  let graphemes = graphemes::init_source(text_no_Ws);
  let mut result_node = None;
  let mut error_log: Vec<(SourceRange, ParseErrorDetail)> = vec![];

  // Do parse
  let remaining: ParseString = match grammar(ParseString::new(&graphemes)) {
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
    Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: 3202, kind: MechErrorKind::ParserError(report, msg)})
  }
}


pub fn parse(text: &str) -> MResult<Program> {
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
    Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: 3202, kind: MechErrorKind::ParserError(report, msg)})
  }
}