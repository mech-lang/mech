#[macro_use]
use crate::*;

#[cfg(not(feature = "no-std"))] use core::fmt;
#[cfg(feature = "no-std")] use alloc::fmt;
#[cfg(feature = "no-std")] use alloc::string::String;
#[cfg(feature = "no-std")] use alloc::vec::Vec;
use nom::{
  IResult,
  branch::alt,
  sequence::tuple as nom_tuple,
  combinator::{opt, eof, peek},
  multi::{many1, many_till, many0, separated_list1,separated_list0},
  bytes::complete::{take_until, take_while},
  Err,
  Err::Failure
};
use std::collections::HashMap;
use colored::*;

use crate::*;
use crate::nodes::Matrix;

// Structures
// =============================================================================

pub fn max_err<'a>(x: Option<ParseError<'a>>, y: ParseError<'a>) -> ParseError<'a> {
  match (x,&y) {
    (None, y) => y.clone(),
    _ => y.clone(),
  }
}

// structure := empty_set | empty_table | table | matrix | tuple | tuple_struct | record | map | set ;
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
    Ok((input, tbl)) => {
      return Ok((input, Structure::Table(tbl)));
    },
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

// Matrix
// ----------------------------------------------------------------------------

// matrix := matrix_start, (box_drawing_char | whitespace)*, matrix_row*, box_drawing_char*, matrix_end ;
pub fn matrix(input: ParseString) -> ParseResult<Matrix> {
  let msg = "Expects right bracket ']' to finish the matrix";
  let (input, (_, r)) = range(matrix_start)(input)?;
  let (input, _) = many0(alt((box_drawing_char,whitespace)))(input)?;
  let (input, rows) = many0(matrix_row)(input)?;
  let (input, _) = many0(alt((box_drawing_char,whitespace)))(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, _) = match label!(matrix_end, msg, r)(input) {
    Ok(k) => k,
    Err(err) => {
      return Err(err);
    }
  };
  Ok((input, Matrix{rows}))
}

// matrix_column := (space | tab)*, expression, ((space | tab)*, ("," | table_separator)?, (space | tab)*) ;
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

// matrix_row := table_separator?, (space | tab)*, matrix_column+, semicolon?, new_line?, (box_drawing_char+, new_line)? ;
pub fn matrix_row(input: ParseString) -> ParseResult<MatrixRow> {
  let (input, _) = many0(space_tab)(input)?;
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

// matrix_start := box_tl_round | box_tl | left_bracket ;
pub fn matrix_start(input: ParseString) -> ParseResult<Token> {
  alt((box_tl_round, box_tl, box_tl_bold, left_bracket))(input)
}

// matrix_end := box_br_round | box_br | right_bracket ;
pub fn matrix_end(input: ParseString) -> ParseResult<Token> {
  let result = alt((box_br_round, box_br, box_br_bold, right_bracket))(input);
  result
}

// Table
// ----------------------------------------------------------------------------

// table := inline-table | regular-table | fancy-table ;
fn table(input: ParseString) -> ParseResult<Table> { 
  alt((inline_table, regular_table, fancy_table))(input)
}

// fancy-table := table-top, fancy-header, +fancy-row, table-bottom ;
pub fn fancy_table(input: ParseString) -> ParseResult<Table> {
  let (input, _) = table_top(input)?;
  let (input, _) = many0(space_tab)(input)?;
  let (input, _) = table_separator(input)?;
  let (input, _) = many0(space_tab)(input)?;
  let (input, header) = table_header(input)?;
  let (input, rows) = separated_list1(new_line,alt((table_row,row_separator)))(input)?;
  let rows: Vec<TableRow> = rows.into_iter().filter(|row| !row.columns.is_empty()).collect();
  Ok((input, Table{header, rows}))
}

// row-separator := *whitespace, *box-drawing-char, *(space | tab), *whitespace ;
pub fn row_separator(input: ParseString) -> ParseResult<TableRow> {
  let (input, _) = whitespace0(input)?;
  let (input, _) = many1(alt((box_drawing_char,table_end,space_tab)))(input)?;
  let (input, _) = many0(space_tab)(input)?;
  Ok((input, TableRow{columns: vec![]}))
}

// table-top := table-start, *box-drawing-char, new-line ;
fn table_top(input: ParseString) -> ParseResult<()> {
  let (input, _) = table_start(input)?;
  let (input, _) = many0(box_drawing_char)(input)?;
  let (input, _) = new_line(input)?;
  Ok((input, ()))
}

// table-bottom := *box-drawing-char, table-end;
fn table_bottom(input: ParseString) -> ParseResult<()> {
  let (input, _) = many0(box_drawing_char)(input)?;
  let (input, _) = table_end(input)?;
  Ok((input, ()))
}

// inline-table := table-separator, *whitespace, table-header, *whitespace, +table-row;
pub fn inline_table(input: ParseString) -> ParseResult<Table> {
  let (input, _) = table_separator(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, header) = table_header(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, rows) = many1(inline_table_row)(input)?;
  Ok((input, Table{header, rows}))
}

// table-row := bar, list1((space | tab)*, expression), (space | tab)*, bar, new-line ;
pub fn inline_table_row(input: ParseString) -> ParseResult<TableRow> {
  let (input, _) = many0(space_tab)(input)?;
  let (input, row) = many1(nom_tuple((many0(space_tab), expression)))(input)?;
  let (input, _) = many0(space_tab)(input)?;
  let (input, _) = table_separator(input)?;
  let row = row.into_iter().map(|(_,tkn)| TableColumn{element:tkn}).collect();
  Ok((input, TableRow{columns: row}))
}

// fancy-table := table_start, (box_drawing_char | whitespace)*, table_header, (box_drawing_char | whitespace)*, table_row+, box_drawing_char*, whitespace*, table_end ;
pub fn regular_table(input: ParseString) -> ParseResult<Table> {
  let (input, _) = table_separator(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, header) = table_header(input)?;
  let (input, rows) = separated_list1(new_line,table_row)(input)?;
  Ok((input, Table{header,rows}))
}

// table_header := list1(space_tab+, field), (space | tab)*, (bar| box_vert), whitespace* ;
pub fn table_header(input: ParseString) -> ParseResult<Vec<Field>> {
  let (input, fields) = separated_list1(many1(alt((space_tab, table_separator))),field)(input)?;
  let (input, _) = many0(space_tab)(input)?;
  let (input, _) = table_separator(input)?;
  let (input, _) = whitespace0(input)?;
  Ok((input, fields))
}

// table-row := bar, list1((space | tab)*, expression), (space | tab)*, bar, new-line ;
pub fn table_row(input: ParseString) -> ParseResult<TableRow> {
  let (input, _) = whitespace0(input)?;
  let (input, _) = table_separator(input)?;
  let (input, row) = many1(nom_tuple((many0(alt((space_tab, table_separator))), expression)))(input)?;
  let (input, _) = many0(space_tab)(input)?;
  let (input, _) = table_separator(input)?;
  let (input, _) = many0(space_tab)(input)?;
  let row = row.into_iter().map(|(_,tkn)| TableColumn{element:tkn}).collect();
  Ok((input, TableRow{columns: row}))
}

// table_column := (space | tab)*, expression, ((space | tab)*, ("," | table_separator)?, (space | tab)*) ;
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

// field := identifier, kind_annotation? ;
pub fn field(input: ParseString) -> ParseResult<Field> {
  let (input, name) = identifier(input)?;
  let (input, kind) = opt(kind_annotation)(input)?;
  Ok((input, Field{name, kind}))
}

// box_drawing_char := box_tl | box_br | box_bl | box_tr | box_tr_round | box_bl_round | box_vert | box_cross | box_horz | box_t_left | box_t_right | box_t_top | box_t_bottom ;
pub fn box_drawing_char(input: ParseString) -> ParseResult<Token> {
  alt((box_tl, box_bl, box_tr, box_tl_bold, box_bl_bold, box_tr_bold, box_tr_round, box_bl_round, box_vert, box_cross, box_horz, box_t_left, box_t_right, box_t_top, box_t_bottom))(input)
}

// box_drawing_emoji := box_tl | box_br | box_bl | box_tr | box_tl_round | box_br_round | box_tr_round | box_bl_round | box_vert | box_cross | box_horz | box_t_left | box_t_right | box_t_top | box_t_bottom ;
pub fn box_drawing_emoji(input: ParseString) -> ParseResult<Token> {
  alt((box_tl, box_bl, box_tr, box_tl_bold, box_bl_bold, box_tr_bold, box_tl_round, box_br_round, box_tr_round, box_bl_round, box_vert, box_cross, box_horz, box_t_left, box_t_right, box_t_top, box_t_bottom))(input)
}

// table_start := box_tl_round | box_tl | left_brace ;
pub fn table_start(input: ParseString) -> ParseResult<Token> {
  alt((box_tl_round, box_tl, box_tl_bold, left_brace, table_separator))(input)
}

// table_end := box_br_round | box_br | right_brace ;
pub fn table_end(input: ParseString) -> ParseResult<Token> {
  let result = alt((box_br_round, box_br, box_br_bold, right_brace, table_separator))(input);
  result
}

// table_separator := box_vert ;
pub fn table_separator(input: ParseString) -> ParseResult<Token> {
  let (input, token) = alt((box_vert,box_vert_bold,bar))(input)?;
  Ok((input, token))
}

pub fn table_horz(input: ParseString) -> ParseResult<Token> {
  let (input, token) = alt((dash,box_horz))(input)?;
  Ok((input, token))
}

// Map
// ----------------------------------------------------------------------------

// empty_table := table_start, whitespace*, table_end ;
pub fn empty_map(input: ParseString) -> ParseResult<Map> {
  let (input, _) = left_brace(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, _) = colon(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, _) = right_brace(input)?;
  Ok((input, Map{elements: vec![]}))
}

// map := "{", whitespace*, mapping*, whitespace*, "}" ;
pub fn map(input: ParseString) -> ParseResult<Map> {
  let msg = "Expects right bracket '}' to terminate inline table";
  let (input, (_, r)) = range(left_brace)(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, elements) = many0(mapping)(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, _) = label!(right_brace, msg, r)(input)?;
  Ok((input, Map{elements}))
}

// mapping :=  whitespace*, expression, whitespace*, ":", whitespace*, expression, comma?, whitespace* ;
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


// Record
// ----------------------------------------------------------------------------

// record := table_start, whitespace*, binding+, whitespace*, table_end ;
pub fn record(input: ParseString) -> ParseResult<Record> {
  let msg = "Expects right bracket ']' to terminate inline table";
  let (input, (_, r)) = range(table_start)(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, bindings) = many1(binding)(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, _) = label!(table_end, msg, r)(input)?;
  Ok((input, Record{bindings}))
}

// binding := identifier, kind_annotation?, colon, expression, ","? ;
pub fn binding(input: ParseString) -> ParseResult<Binding> {
  let msg1 = "Unexpected space before colon ':'";
  let msg2 = "Expects a value";
  let msg3 = "Expects whitespace or comma followed by whitespace";
  let msg4 = "Expects whitespace";
  let (input, _) = whitespace0(input)?;
  let (input, name) = identifier(input)?;
  let (input, kind) = opt(kind_annotation)(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, _) = colon(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, value) = label!(expression, msg2)(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, _) = opt(comma)(input)?;
  let (input, _) = whitespace0(input)?;
  Ok((input, Binding{name, kind, value}))
}

// Set
// ----------------------------------------------------------------------------

// empty_set := table_start, whitespace*, empty, whitespace*, table_end ;
pub fn empty_set(input: ParseString) -> ParseResult<Set> {
  let (input, _) = left_brace(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, _) = empty(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, _) = right_brace(input)?;
  Ok((input,  Set{elements: vec![]}))
}

// set := "{", whitespace*, list0(("," | whitespace+), expression), whitespace*, "}" ;
pub fn set(input: ParseString) -> ParseResult<Set> {
  let msg = "Expects right bracket '}' to terminate inline table";
  let (input, (_, r)) = range(left_brace)(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, elements) = separated_list0(alt((list_separator,whitespace1)), expression)(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, _) = label!(right_brace, msg, r)(input)?;
  Ok((input, Set{elements}))
}

// Tuple
// ----------------------------------------------------------------------------

// tuple := "(", list0(",", expression), ")" ;
pub fn tuple(input: ParseString) -> ParseResult<Tuple> {
  let (input, _) = left_parenthesis(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, exprs) = separated_list0(list_separator, expression)(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, _) = right_parenthesis(input)?;
  Ok((input, Tuple{elements: exprs}))
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