#[macro_use]
use crate::*;
use crate::base::*;
use crate::label;
use crate::labelr;
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

// structure := empty_set | empty_table | matrix | table | tuple | tuple_struct | record | map | set ;
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

// binding := identifier, kind_annotation?, <!(space+, colon)>, colon, s+,
// >>          <empty | expression | identifier | value>, <!!right_bracket | (s*, comma, <s+>) | s+> ;
// >> where s := space | new_line | tab ;

// binding ::= identifier, kind_annotation?, whitespace?, ":", whitespace?, (expression | empty | identifier | value), whitespace?, (comma, whitespace?)?
/* Gemini AI Explanation:

Optional Whitespace: The whitespace? elements explicitly allow for optional whitespace before the kind_annotation and after the colon.
Specific Value Non-Terminal: The value non-terminal symbol is used for the value part, which could be more specific than expression depending on the language's context.
Direct Use of space, new_line, and tab: The s non-terminal is replaced with its definition, making the grammar more concise. */

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

// table_column := (space | tab)*, expression ;
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

// matrix_column := (space | tab)*, expression ;
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

// field := identifier, [kind_annotation] ;
pub fn field(input: ParseString) -> ParseResult<Field> {
  let (input, name) = identifier(input)?;
  let (input, kind) = opt(kind_annotation)(input)?;
  Ok((input, Field{name, kind}))
}

// box_drawing_char := box_tr_round | box_bl_round | box_vert | box_cross | box_horz | box_t_left | box_t_right | box_t_top | box_t_bottom ;
pub fn box_drawing_char(input: ParseString) -> ParseResult<Token> {
  alt((box_tl, box_br, box_bl, box_tr, box_tr_round, box_bl_round, box_vert, box_cross, box_horz, box_t_left, box_t_right, box_t_top, box_t_bottom))(input)
}

// box_drawing_emoji := box_tl_round | box_br_round | box_tr_round | box_bl_round | box_vert | box_cross | box_horz | box_t_left | box_t_right | box_t_top | box_t_bottom ;
pub fn box_drawing_emoji(input: ParseString) -> ParseResult<Token> {
  alt((box_tl, box_br, box_bl, box_tr, box_tl_round, box_br_round, box_tr_round, box_bl_round, box_vert, box_cross, box_horz, box_t_left, box_t_right, box_t_top, box_t_bottom))(input)
}

// matrix_start := box_tl_round | left_bracket ;
pub fn matrix_start(input: ParseString) -> ParseResult<Token> {
  alt((box_tl_round, box_tl, left_bracket))(input)
}

// matrix_end := box_br_round | right_bracket ;
pub fn matrix_end(input: ParseString) -> ParseResult<Token> {
  let result = alt((box_br_round, box_br, right_bracket))(input);
  result
}

// table_start := box_tl_round | left_brace ;
pub fn table_start(input: ParseString) -> ParseResult<Token> {
  alt((box_tl_round, box_tl, left_brace))(input)
}

// table_end := box_br_round | right_brace ;
pub fn table_end(input: ParseString) -> ParseResult<Token> {
  let result = alt((box_br_round, box_br, right_brace))(input);
  result
}

// table_separator := box_vert ;
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

// empty_set ::= table_start, whitespace?, empty, whitespace?, table_end
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

// map := "{", mapping*, "}" ;
pub fn map(input: ParseString) -> ParseResult<Map> {
  let msg = "Expects right bracket '}' to terminate inline table";
  let (input, (_, r)) = range(left_brace)(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, elements) = many0(mapping)(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, _) = label!(right_brace, msg, r)(input)?;
  Ok((input, Map{elements}))
}

// mapping := expression, ":", expression ;
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

//define_operator ::= ":" "="
pub fn define_operator(input: ParseString) -> ParseResult<()> {
  let (input, _) = whitespace0(input)?;
  let (input, _) = tag(":=")(input)?;
  let (input, _) = whitespace0(input)?;
  Ok((input, ()))
}

// output_operator ::= "=>"
pub fn output_operator(input: ParseString) -> ParseResult<()> {
  let (input, _) = whitespace0(input)?;
  let (input, _) = tag("=>")(input)?;
  let (input, _) = whitespace0(input)?;
  Ok((input, ()))
}

// async_transition_operator ::= "~>"
pub fn async_transition_operator(input: ParseString) -> ParseResult<()> {
  let (input, _) = whitespace0(input)?;
  let (input, _) = tag("~>")(input)?;
  let (input, _) = whitespace0(input)?;
  Ok((input, ()))
}

// transition_operator ::= "->"
pub fn transition_operator(input: ParseString) -> ParseResult<()> {
  let (input, _) = whitespace0(input)?;
  let (input, _) = tag("->")(input)?;
  let (input, _) = whitespace0(input)?;
  Ok((input, ()))
}

// guard_operator ::= "|" | "│" | "├" | "└"
pub fn guard_operator(input: ParseString) -> ParseResult<()> {
  let (input, _) = whitespace0(input)?;
  let (input, _) = alt((tag("|"),tag("│"),tag("├"),tag("└")))(input)?;
  let (input, _) = whitespace0(input)?;
  Ok((input, ()))
}

// fsm_implementation ::= "#" identifier, "(", identifier*, ")", "->", fsm_pattern, whitespace?, fsm_arm+, "."
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

// fsm_arm ::= comment*, fsm_pattern, (fsm_state_transition | fsm_output | fsm_guard)+, whitespace?
pub fn fsm_arm(input: ParseString) -> ParseResult<FsmArm> {
  let ((input, _)) = many0(comment)(input)?;
  let ((input, arm)) = alt((fsm_guard_arm,fsm_transition))(input)?;
  let ((input, _)) = whitespace0(input)?;
  Ok((input, arm))
}

// fsm_guard ::= guard_operator, fsm_arm
pub fn fsm_guard_arm(input: ParseString) -> ParseResult<FsmArm> {
  let ((input, _)) = many0(comment)(input)?;
  let ((input, start)) = fsm_pattern(input)?;
  let (input, grds) = many1(fsm_guard)(input)?;
  Ok((input, FsmArm::Guard(start, grds)))
}

pub fn fsm_guard(input: ParseString) -> ParseResult<Guard> {
  let (input, _) = guard_operator(input)?;
  let (input, cnd) = fsm_pattern(input)?;
  let (input, trns) = many1(alt((fsm_state_transition,fsm_output,fsm_async_transition)))(input)?;
  Ok((input, Guard{condition: cnd, transitions: trns}))
}

pub fn fsm_transition(input: ParseString) -> ParseResult<FsmArm> {
  let ((input, _)) = many0(comment)(input)?;
  let ((input, start)) = fsm_pattern(input)?;
  let ((input, trns)) = many1(alt((fsm_state_transition,fsm_output,fsm_async_transition)))(input)?;
  Ok((input, FsmArm::Transition(start, trns)))
}

// fsm_state_transition ::= transition_operator, fsm_pattern
pub fn fsm_state_transition(input: ParseString) -> ParseResult<Transition> {
  let (input, _) = transition_operator(input)?;
  let ((input, ptrn)) = fsm_pattern(input)?;
  Ok((input, Transition::Next(ptrn)))
}

// fsm_async_transition ::= async_transition_operator, fsm_pattern
pub fn fsm_async_transition(input: ParseString) -> ParseResult<Transition> {
  let (input, _) = async_transition_operator(input)?;
  let ((input, ptrn)) = fsm_pattern(input)?;
  Ok((input, Transition::Async(ptrn)))
}

// fsm_output ::= output_operator, fsm_pattern
pub fn fsm_output(input: ParseString) -> ParseResult<Transition> {
  let (input, _) = output_operator(input)?;
  let ((input, ptrn)) = fsm_pattern(input)?;
  Ok((input, Transition::Output(ptrn)))
}

// fsm_specification ::= "#" identifier, "(", var*, ")", output_operator, identifier, define_operator, fsm_state_definition+, "."
pub fn fsm_specification(input: ParseString) -> ParseResult<FsmSpecification> {
  let ((input, _)) = hashtag(input)?;
  let ((input, name)) = identifier(input)?;
  let ((input, _)) = left_parenthesis(input)?;
  let ((input, input_vars)) = separated_list0(list_separator, var)(input)?;
  let ((input, _)) = right_parenthesis(input)?;
  let ((input, _)) = opt(output_operator)(input)?;
  let ((input, output)) = opt(kind_annotation)(input)?;
  let ((input, _)) = define_operator(input)?;
  let ((input, states)) = many1(fsm_state_definition)(input)?;
  let ((input, _)) = period(input)?;
  Ok((input, FsmSpecification{name,input: input_vars, output, states}))
}

// fsm_pattern ::= fsm_tuple_struct | wildcard | formula
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

// wildcard ::= "*"
pub fn wildcard(input: ParseString) -> ParseResult<Pattern> {
  let ((input, _)) = asterisk(input)?;
  Ok((input, Pattern::Wildcard))
}

// fsm_tuple_struct ::= identifier, "(", fsm_pattern+, ")"
pub fn fsm_tuple_struct(input: ParseString) -> ParseResult<PatternTupleStruct> {
  let (input, _) = grave(input)?;
  let (input, id) = identifier(input)?;
  let ((input, _)) = left_parenthesis(input)?;
  let ((input, patterns)) = separated_list1(list_separator, fsm_pattern)(input)?;
  let ((input, _)) = right_parenthesis(input)?;
  Ok((input, PatternTupleStruct{name: id, patterns}))
}

// fsm_state_definition ::= guard_operator?, identifier, fsm_state_definition_variables?
pub fn fsm_state_definition(input: ParseString) -> ParseResult<StateDefinition> {
  let ((input, _)) = guard_operator(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, _) = grave(input)?;
  let ((input, name)) = identifier(input)?;
  let ((input, vars)) = opt(fsm_state_definition_variables)(input)?;
  Ok((input, StateDefinition{name,state_variables: vars}))
}

// fsm_state_definition_variables ::= "(", identifier+, ")"
pub fn fsm_state_definition_variables(input: ParseString) -> ParseResult<Vec<Var>> {
  let ((input, _)) = left_parenthesis(input)?;
  let ((input, names)) = separated_list1(list_separator, var)(input)?;
  let ((input, _)) = right_parenthesis(input)?;
  Ok((input, names))
}

// fsm_pipe ::= fsm_instance, (fsm_state_transition | fsm_async_transition | fsm_output | fsm_guard)*
pub fn fsm_pipe(input: ParseString) -> ParseResult<FsmPipe> {
  let ((input, start)) = fsm_instance(input)?;
  let ((input, trns)) = many0(alt((fsm_state_transition,fsm_async_transition,fsm_output)))(input)?;
  Ok((input, FsmPipe{start, transitions: trns}))
}

// fsm_declare ::= fsm, define_operator, fsm_pipe
pub fn fsm_declare(input: ParseString) -> ParseResult<FsmDeclare> {
  let (input, fsm) = fsm(input)?;
  let (input, _) = define_operator(input)?;
  let (input, pipe) = fsm_pipe(input)?;
  Ok((input, FsmDeclare{fsm,pipe}))
}
  
// fsm ::= "#" identifier, argument_list?, kind_annotation?
pub fn fsm(input: ParseString) -> ParseResult<Fsm> {
  let ((input, _)) = hashtag(input)?;
  let ((input, name)) = identifier(input)?;
  let ((input, args)) = opt(argument_list)(input)?;
  let ((input, kind)) = opt(kind_annotation)(input)?;
  Ok((input, Fsm{ name, args, kind }))
}

// fsm_instance ::= "#" identifier, argument_list?
pub fn fsm_instance(input: ParseString) -> ParseResult<FsmInstance> {
  let ((input, _)) = hashtag(input)?;
  let (input, name) = identifier(input)?;
  let (input, args) = opt(fsm_args)(input)?;
  Ok((input, FsmInstance{name,args} ))
}

// fsm_args ::= "(", (call_arg_with_binding | call_arg)*, ")"
pub fn fsm_args(input: ParseString) -> ParseResult<Vec<(Option<Identifier>,Expression)>> {
  let (input, _) = left_parenthesis(input)?;
  let (input, args) = separated_list0(list_separator, alt((call_arg_with_binding,call_arg)))(input)?;
  let (input, _) = right_parenthesis(input)?;
  Ok((input, args))
}
