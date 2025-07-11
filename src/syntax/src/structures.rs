#[macro_use]
use crate::*;
use nom::{
  multi::separated_list0,
  sequence::tuple as nom_tuple,
};
use crate::nodes::Matrix;

// #### Structures

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
  match markdown_table(input.clone()) {
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

// tuple := "(", list0(",", expression), ")" ;
pub fn tuple(input: ParseString) -> ParseResult<Tuple> {
  let (input, _) = left_parenthesis(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, exprs) = separated_list0(list_separator, expression)(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, _) = right_parenthesis(input)?;
  Ok((input, Tuple{elements: exprs}))
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


// table_row := table_separator?, (space | tab)*, table_column+, semicolon?, new_line?, (box_drawing_char+, new_line)? ;
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

// matrix_row := table_separator?, (space | tab)*, matrix_column+, semicolon?, new_line?, (box_drawing_char+, new_line)? ;
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

// table_header := list1(space_tab+, field), (space | tab)*, (bar| box_vert), whitespace* ;
pub fn table_header(input: ParseString) -> ParseResult<Vec<Field>> {
  let (input, fields) = separated_list1(many1(space_tab),field)(input)?;
  let (input, _) = many0(space_tab)(input)?;
  let (input, _) = alt((bar,box_vert,box_vert_bold))(input)?;
  let (input, _) = whitespace0(input)?;
  Ok((input, fields))
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

// matrix_start := box_tl_round | box_tl | left_bracket ;
pub fn matrix_start(input: ParseString) -> ParseResult<Token> {
  alt((box_tl_round, box_tl, box_tl_bold, left_bracket))(input)
}

// matrix_end := box_br_round | box_br | right_bracket ;
pub fn matrix_end(input: ParseString) -> ParseResult<Token> {
  let result = alt((box_br_round, box_br, box_br_bold, right_bracket))(input);
  result
}

// table_start := box_tl_round | box_tl | left_brace ;
pub fn table_start(input: ParseString) -> ParseResult<Token> {
  alt((box_tl_round, box_tl, box_tl_bold, left_brace))(input)
}

// table_end := box_br_round | box_br | right_brace ;
pub fn table_end(input: ParseString) -> ParseResult<Token> {
  let result = alt((box_br_round, box_br, box_br_bold, right_brace))(input);
  result
}

// table_separator := box_vert ;
pub fn table_separator(input: ParseString) -> ParseResult<Token> {
  let (input, token) = alt((box_vert,box_vert_bold))(input)?;
  Ok((input, token))
}

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

// table := table_start, (box_drawing_char | whitespace)*, table_header, (box_drawing_char | whitespace)*, table_row+, box_drawing_char*, whitespace*, table_end ;
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

// empty_table := table_start, whitespace*, table_end ;
pub fn empty_map(input: ParseString) -> ParseResult<Map> {
  let (input, _) = table_start(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, _) = table_end(input)?;
  Ok((input, Map{elements: vec![]}))
}

// empty_set := table_start, whitespace*, empty, whitespace*, table_end ;
pub fn empty_set(input: ParseString) -> ParseResult<Set> {
  let (input, _) = table_start(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, _) = empty(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, _) = table_end(input)?;
  Ok((input,  Set{elements: vec![]}))
}

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