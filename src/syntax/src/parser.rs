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
use crate::functions::function_define;

use mech_core::nodes::*;
use mech_core::nodes::{SectionElement, MechString, Table};

#[cfg(not(feature = "no-std"))] use core::fmt;
#[cfg(feature = "no-std")] use alloc::fmt;
#[cfg(feature = "no-std")] use alloc::string::String;
#[cfg(feature = "no-std")] use alloc::vec::Vec;
use nom::{
  IResult,
  branch::alt,
  sequence::{tuple as nom_tuple, preceded},
  combinator::{opt, eof, cut, peek},
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
        e.cause_range = SourceRange { start, end: e.cause_range.end };
        //e.error_detail = error_detail.clone();
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
      Err(nom::Err::Error(ParseError::new(input, "Unexpected character")))
    }
  }
}

// 3. Recovery functions
// -----------------------

// skip_till_eol := (!new_line, any)* ;
pub fn skip_till_eol(input: ParseString) -> ParseResult<Token> {
  let (input, matched) = many0(nom_tuple((
    is_not(new_line),
    any_token,
  )))(input)?;
  let mut matched: Vec<Token> = matched.into_iter().map(|(_, t)| t).collect(); 
  let tkn = Token::merge_tokens(&mut matched).unwrap_or(Token::default()); 
  Ok((input, tkn))
}

// skip_past_eol := skip_till_eol, new_line ;
pub fn skip_past_eol(input: ParseString) -> ParseResult<Token> {
  let (input, matched) = skip_till_eol(input)?;
  let (input, nl) = new_line(input)?;
  let matched = Token::merge_tokens(&mut vec![matched, nl]).unwrap_or(Token::default());
  Ok((input, matched))
}

// skip-till-end-of-statement := *((!new-line, !";"), any) ;
pub fn skip_till_end_of_statement(input: ParseString) -> ParseResult<Token> {
  // If empty, return
  if input.is_empty() {
      return Ok((input, Token::default()));
  }

  // Consume until either newline or ;
  let (input, matched) = many0(nom_tuple((
      // is_not matches any char NOT in the set
      is_not(alt((
          new_line,
          semicolon,
      ))),
      any_token,
  )))(input)?;

  let mut matched: Vec<Token> = matched.into_iter().map(|(_, t)| t).collect();
  let tkn = Token::merge_tokens(&mut matched).unwrap_or(Token::default());

  Ok((input, tkn))
}

// skip_till_section_element := skip_past_eol, (!section_element, skip_past_eol)* ;
pub fn skip_till_section_element(input: ParseString) -> ParseResult<Token> {
  if input.is_empty() {
    return Ok((input, Token::default()));
  }
  let (input, matched) = skip_past_eol(input)?;
  let (input, matched2) = many0(nom_tuple((
    is_not(section_element),
    skip_past_eol,
  )))(input)?;
  let mut matched: Vec<Token> = vec![matched];
  matched.extend(matched2.into_iter().map(|(_, t)| t));
  let tkn = Token::merge_tokens(&mut matched).unwrap_or(Token::default());
  Ok((input, tkn))
}

pub fn skip_till_paragraph_element(input: ParseString) -> ParseResult<Token> {
  // if it's empty, return
  if input.is_empty() {
    return Ok((input, Token::default()));
  }
  // Otherwise, consume tokens until we reach a paragraph element
  let (input, matched) = many0(nom_tuple((
    is_not(paragraph_element),
    any_token,
  )))(input)?;
  let mut matched: Vec<Token> = matched.into_iter().map(|(_, t)| t).collect(); 
  let tkn = Token::merge_tokens(&mut matched).unwrap_or(Token::default());
  Ok((input, tkn))
}

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

// recovery function for Recoverable nodes with customizable skip function
pub fn recover<T: Recoverable, F>(input: ParseString, skip_fn: F) -> ParseResult<T>
where
  F: Fn(ParseString) -> ParseResult<Token>,
{
  let start = input.loc();
  let (input, matched) = skip_fn(input)?;
  let end = input.loc();
  Ok((input, T::error_placeholder(matched, SourceRange { start, end })))
}

// 4. Public interface
// ---------------------

// mech_code_alt := fsm_specification | fsm_implementation | function_define | statement | expression | comment ;
pub fn mech_code_alt(input: ParseString) -> ParseResult<MechCode> {
  let (input, _) = whitespace0(input)?;
  let parsers: Vec<(&str, Box<dyn Fn(ParseString) -> ParseResult<MechCode>>)> = vec![
    // ("fsm_specification", Box::new(|i| fsm_specification(i).map(|(i, v)| (i, MechCode::FsmSpecification(v))))),
    // ("fsm_implementation", Box::new(|i| fsm_implementation(i).map(|(i, v)| (i, MechCode::FsmImplementation(v))))),
    // ("function_define", Box::new(|i| function_define(i).map(|(i, v)| (i, MechCode::FunctionDefine(v))))),
    ("statement",   Box::new(|i| statement(i).map(|(i, v)| (i, MechCode::Statement(v))))),
    ("expression",  Box::new(|i| expression(i).map(|(i, v)| (i, MechCode::Expression(v))))),
    ("comment",     Box::new(|i| comment(i).map(|(i, v)| (i, MechCode::Comment(v))))),
  ];
  match alt_best(input, &parsers) {
    Ok((input, code)) => {
      return Ok((input, code));
    }
    Err(e) => {
      return Err(e);
    }
  };

}

/// code-terminal := *space-tab, ?(?semicolon, *space-tab, comment), (new-line | ";" | eof), *whitespace ;
pub fn code_terminal(input: ParseString) -> ParseResult<Option<Comment>> {
  let (input, _) = many0(space_tab)(input)?;
  let (input, cmmnt) = opt(tuple((opt(semicolon), many0(space_tab), comment)))(input)?;
  let (input, _) = alt((null(new_line), null(semicolon), null(eof)))(input)?;
  let (input, _) = whitespace0(input)?;
  let cmmt = match cmmnt {
    Some((_, _, cmnt)) => Some(cmnt),
    None => None,
  };
  Ok((input, cmmt))
}

// mech-code-block := +(mech-code, code-terminal) ;
pub fn mech_code(input: ParseString) -> ParseResult<Vec<(MechCode,Option<Comment>)>> {
  let mut output = vec![];
  let mut new_input = input.clone();
  loop {

    if peek(not_mech_code)(new_input.clone()).is_ok() {
      if output.len() > 0 {
        return Ok((new_input, output));
      } else {
        let e = ParseError::new(new_input, "Unexpected character");
        return Err(Err::Error(e));
      }
    }

    let start = new_input.loc();
    let start_cursor = new_input.cursor;
    let (input, code) = match mech_code_alt(new_input.clone()) {
      Err(Err::Error(mut e)) => {
        // if the error is just "Unexpected character", we will just fail.
        if e.error_detail.message == "Unexpected character" {
          if output.len() > 0 {
            return Ok((new_input, output));
          } else {
            return Err(Err::Error(e));
          }
        } else {
          e.cause_range = SourceRange { start, end: e.cause_range.end };
          e.log();
          // skip till the end of the statement
          let (input, skipped) = skip_till_end_of_statement(e.remaining_input)?;
          // get tokens from start_cursor to input.cursor
          let skipped_input = input.slice(start_cursor, input.cursor);
          let skipped_token = Token {
            kind: TokenKind::Error,
            chars: skipped_input.chars().collect(),
            src_range: SourceRange { start, end: input.loc() },
          };
          let mech_error = MechCode::Error(skipped_token, e.cause_range);
          (input, mech_error)
        }
      }
      Err(Err::Failure(mut e)) => {
        // Check if this thing matches a section element:
        match subtitle(new_input.clone()) {
          Ok((_, _)) => {
            // if it does, and we have already parsed something, return what we have.
            if output.len() > 0 {
              return Ok((new_input, output));
            } else {
              return Err(Err::Failure(e));
            }
          }
          Err(_) => { /* continue with error recovery */ }
        }
        e.cause_range = SourceRange { start, end: e.cause_range.end };
        e.log();
        // skip till the end of the statement
        let (input, skipped) = skip_till_end_of_statement(e.remaining_input)?;
        // get tokens from start_cursor to input.cursor
        let skipped_input = input.slice(start_cursor, input.cursor);
        let skipped_token = Token {
          kind: TokenKind::Error,
          chars: skipped_input.chars().collect(),
          src_range: SourceRange { start, end: input.loc() },
        };
        let mech_error = MechCode::Error(skipped_token, e.cause_range);
        (input, mech_error)
      },
      Ok(x) => x,
      _ => unreachable!(),
    };
    let (input, cmmt) = match code_terminal(input) {
      Ok((input, cmmt)) => (input, cmmt),
      Err(e) => {
        // if we didn't parse a terminal, just return what we've got so far.
        if output.len() > 0 {
          return Ok((new_input, output));
        }
        // otherwise, return the error.
        return Err(e);
      }
    };
    output.push((code, cmmt));
    new_input = input;
    if new_input.is_empty() {
      break;
    }
  }
  Ok((new_input, output))
}

// program := ws0, ?title, body, ws0 ;
pub fn program(input: ParseString) -> ParseResult<Program> {
  let msg = "Expects program body";
  let (input, _) = whitespace0(input)?;
  let (input, title) = opt(title)(input)?;
  //let (input, body) = labelr!(body, skip_nil, msg)(input)?;
  let (input, body) = body(input)?;
  //println!("Parsed program body: {:#?}", body);
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
    let report: Vec<ParserErrorContext> = error_log.into_iter().map(|e| ParserErrorContext {
      cause_rng: e.0,
      err_message: String::from(e.1.message),
      annotation_rngs: e.1.annotation_rngs,
    }).collect();
    Err(MechError2::new(
      ParserErrorReport(text.to_string(), report),
      None
    ))
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
    let report: Vec<ParserErrorContext> = error_log.into_iter().map(|e| ParserErrorContext {
      cause_rng: e.0,
      err_message: String::from(e.1.message),
      annotation_rngs: e.1.annotation_rngs,
    }).collect();
    Err(MechError2::new(
      ParserErrorReport(text.to_string(), report),
      None
    ).with_compiler_loc())
  }
}