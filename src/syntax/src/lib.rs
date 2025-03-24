// # Syntax

#![cfg_attr(feature = "no-std", no_std)]
#![cfg_attr(feature = "no-std", alloc)]
#![feature(extract_if)]
#![feature(get_mut_unchecked)]
#![allow(dead_code)]
#![allow(warnings)]
#![feature(step_trait)]

extern crate mech_core;
#[cfg(feature="no-std")] #[macro_use] extern crate alloc;
#[cfg(not(feature = "no-std"))] extern crate core;
extern crate hashbrown;
extern crate nom;
extern crate nom_unicode;
#[macro_use]
extern crate lazy_static;
extern crate nalgebra as na;
extern crate tabled;
extern crate libm;

use mech_core::*;
use mech_core::nodes::*;
use std::cell::RefCell;
use std::rc::Rc;

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

use std::collections::HashMap;
use colored::*;

pub mod mechdown;
pub mod expressions;
pub mod statements;
pub mod structures;
pub mod base;
pub mod parser;
pub mod formatter;
pub mod grammar;

pub use crate::parser::*;
pub use crate::mechdown::*;
pub use crate::expressions::*;
pub use crate::statements::*;
pub use crate::structures::*;
pub use crate::base::*;
pub use crate::formatter::*;
pub use crate::grammar::*;


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