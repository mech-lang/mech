#[macro_use]
use crate::*;
use nom::{
  multi::separated_list0,
  sequence::tuple as nom_tuple,
};

// #### State Machines

// guard_operator := "|" | "│" | "├" | "└" ;
pub fn guard_operator(input: ParseString) -> ParseResult<()> {
  let (input, _) = whitespace0(input)?;
  let (input, _) = alt((tag("|"),tag("│"),tag("├"),tag("└")))(input)?;
  let (input, _) = whitespace0(input)?;
  Ok((input, ()))
}

// fsm_implementation := "#", identifier, "(", list0(",", identifier), ")", transition_operator, pattern, whitespace*, fsm_arm+, "." ;
pub fn fsm_implementation(input: ParseString) -> ParseResult<FsmImplementation> {
  let (input, _) = hashtag(input)?;
  let (input, name) = identifier(input)?;
  let (input, _) = left_parenthesis(input)?;
  let (input, input_vars) = separated_list0(list_separator, identifier)(input)?;
  let (input, _) = right_parenthesis(input)?;
  let (input, _) = transition_operator(input)?;
  let (input, start) = pattern(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, arms) = many1(fsm_arm)(input)?;
  let (input, _) = period(input)?;
  Ok((input, FsmImplementation{name,input: input_vars,start,arms}))
}

// fsm_arm := comment*, (fsm_transition | fsm_guard_arm), whitespace* ;
pub fn fsm_arm(input: ParseString) -> ParseResult<FsmArm> {
  let (input, _) = many0(comment)(input)?;
  let (input, arm) = alt((fsm_guard_arm,fsm_transition))(input)?;
  let (input, _) = whitespace0(input)?;
  Ok((input, arm))
}

// fsm_guard_arm := comment*, pattern, fsm_guard+ ;
pub fn fsm_guard_arm(input: ParseString) -> ParseResult<FsmArm> {
  let (input, _) = many0(comment)(input)?;
  let (input, start) = pattern(input)?;
  let (input, grds) = many1(fsm_guard)(input)?;
  Ok((input, FsmArm::Guard(start, grds)))
}

// fsm_guard := guard_operator, pattern, (fsm_statement_transition | fsm_state_transition | fsm_output | fsm_async_transition | fsm_block_transition)+ ;
pub fn fsm_guard(input: ParseString) -> ParseResult<Guard> {
  let (input, _) = guard_operator(input)?;
  let (input, cnd) = pattern(input)?;
  let (input, trns) = many1(alt((
    fsm_statement_transition,
    fsm_state_transition,
    fsm_output,
    fsm_async_transition,
    fsm_block_transition)))(input)?;
  Ok((input, Guard{condition: cnd, transitions: trns}))
}

// fsm_transition := comment*, pattern, (fsm_statement_transition | fsm_state_transition | fsm_output | fsm_async_transition | fsm_block_transition)+ ;
pub fn fsm_transition(input: ParseString) -> ParseResult<FsmArm> {
  let (input, _) = many0(comment)(input)?;
  let (input, start) = pattern(input)?;
  let (input, trns) = many1(alt((
    fsm_state_transition,
    fsm_output,
    fsm_async_transition,
    fsm_statement_transition,
    fsm_block_transition)))(input)?;
  Ok((input, FsmArm::Transition(start, trns)))
}

// fsm_state_transition := transition_operator, pattern ;
pub fn fsm_state_transition(input: ParseString) -> ParseResult<Transition> {
  let (input, _) = transition_operator(input)?;
  let (input, ptrn) = pattern(input)?;
  Ok((input, Transition::Next(ptrn)))
}

// fsm_async_transition := async_transition_operator, pattern ;
pub fn fsm_async_transition(input: ParseString) -> ParseResult<Transition> {
  let (input, _) = async_transition_operator(input)?;
  let (input, ptrn) = pattern(input)?;
  Ok((input, Transition::Async(ptrn)))
}

// fsm_statement_transition := transition_operator, statement ;
pub fn fsm_statement_transition(input: ParseString) -> ParseResult<Transition> {
  let (input, _) = transition_operator(input)?;
  let (input, stmnt) = statement(input)?;
  Ok((input, Transition::Statement(stmnt)))
}

// fsm_block_transition := transition_operator, left_brace, mech_code+, right_brace ;
pub fn fsm_block_transition(input: ParseString) -> ParseResult<Transition> {
  let (input, _) = transition_operator(input)?;
  let (input, _) = left_brace(input)?;
  let (input, code) = mech_code(input)?;
  let (input, _) = right_brace(input)?;
  Ok((input, Transition::CodeBlock(code)))
}


// fsm_output := output_operator, pattern ;
pub fn fsm_output(input: ParseString) -> ParseResult<Transition> {
  let (input, _) = output_operator(input)?;
  let ((input, ptrn)) = pattern(input)?;
  Ok((input, Transition::Output(ptrn)))
}

// fsm_specification := "#", identifier, "(", list0(",", var), ")", output_operator?, kind_annotation?, define_operator, fsm_state_definition+, "." ;
pub fn fsm_specification(input: ParseString) -> ParseResult<FsmSpecification> {
  let (input, _) = hashtag(input)?;
  let (input, name) = identifier(input)?;
  let (input, _) = left_parenthesis(input)?;
  let (input, input_vars) = separated_list0(list_separator, var)(input)?;
  let (input, _) = right_parenthesis(input)?;
  let (input, _) = opt(output_operator)(input)?;
  let (input, output) = opt(kind_annotation)(input)?;
  let (input, _) = define_operator(input)?;
  let (input, states) = many1(fsm_state_definition)(input)?;
  let (input, _) = period(input)?;
  Ok((input, FsmSpecification{name,input: input_vars, output, states}))
}

// pattern := pattern_tuple_struct | wildcard | formula ;
pub fn pattern(input: ParseString) -> ParseResult<Pattern> {
  match pattern_tuple_struct(input.clone()) {
    Ok((input, tpl)) => {return Ok((input, Pattern::TupleStruct(tpl)))},
    _ => ()
  }
  match wildcard(input.clone()) {
    Ok((input, _)) => {return Ok((input, Pattern::Wildcard))},
    _ => ()
  }
  match pattern_tuple(input.clone()) {
    Ok((input, tpl)) => {return Ok((input, Pattern::Tuple(tpl)))},
    _ => ()
  }
  match expression(input.clone()) {
    Ok((input, expr)) => {return Ok((input, Pattern::Expression(expr)))},
    Err(err) => {return Err(err)},
  }
}

// wildcard := "*" ;
pub fn wildcard(input: ParseString) -> ParseResult<Pattern> {
  let ((input, _)) = asterisk(input)?;
  Ok((input, Pattern::Wildcard))
}

// pattern_tuple_struct := grave, identifier, "(", list1(",", pattern), ")" ;
pub fn pattern_tuple_struct(input: ParseString) -> ParseResult<PatternTupleStruct> {
  let (input, _) = grave(input)?;
  let (input, id) = identifier(input)?;
  let (input, _) = left_parenthesis(input)?;
  let (input, patterns) = separated_list1(list_separator, pattern)(input)?;
  let (input, _) = right_parenthesis(input)?;
  Ok((input, PatternTupleStruct{name: id, patterns}))
}

// pattern-tuple := "(", [pattern, ","], ")" ;
pub fn pattern_tuple(input: ParseString) -> ParseResult<PatternTuple> {
  let (input, _) = left_parenthesis(input)?;
  let (input, patterns) = separated_list1(list_separator, pattern)(input)?;
  let (input, _) = right_parenthesis(input)?;
  Ok((input, PatternTuple(patterns)))
}

// fsm_state_definition := guard_operator, grave, identifier, fsm_state_definition_variables? ;
pub fn fsm_state_definition(input: ParseString) -> ParseResult<StateDefinition> {
  let (input, _) = guard_operator(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, _) = grave(input)?;
  let (input, name) = identifier(input)?;
  let (input, vars) = opt(fsm_state_definition_variables)(input)?;
  Ok((input, StateDefinition{name,state_variables: vars}))
}

// fsm_state_definition_variables := "(", list0(list_separator, var), ")" ;
pub fn fsm_state_definition_variables(input: ParseString) -> ParseResult<Vec<Var>> {
  let (input, _) = left_parenthesis(input)?;
  let (input, names) = separated_list1(list_separator, var)(input)?;
  let (input, _) = right_parenthesis(input)?;
  Ok((input, names))
}

// fsm_pipe := fsm_instance, (fsm_state_transition | fsm_async_transition | fsm_output)* ;
pub fn fsm_pipe(input: ParseString) -> ParseResult<FsmPipe> {
  let (input, start) = fsm_instance(input)?;
  let (input, trns) = many0(alt((fsm_state_transition,fsm_async_transition,fsm_output)))(input)?;
  Ok((input, FsmPipe{start, transitions: trns}))
}

// fsm_declare := fsm, define_operator, fsm_pipe ;
pub fn fsm_declare(input: ParseString) -> ParseResult<FsmDeclare> {
  let (input, fsm) = fsm(input)?;
  let (input, _) = define_operator(input)?;
  let (input, pipe) = fsm_pipe(input)?;
  Ok((input, FsmDeclare{fsm,pipe}))
}
  
// fsm := "#", identifier, argument_list?, kind_annotation? ;
pub fn fsm(input: ParseString) -> ParseResult<Fsm> {
  let ((input, _)) = hashtag(input)?;
  let ((input, name)) = identifier(input)?;
  let ((input, args)) = opt(argument_list)(input)?;
  let ((input, kind)) = opt(kind_annotation)(input)?;
  Ok((input, Fsm{ name, args, kind }))
}

// fsm_instance := "#", identifier, fsm_args? ;
pub fn fsm_instance(input: ParseString) -> ParseResult<FsmInstance> {
  let ((input, _)) = hashtag(input)?;
  let (input, name) = identifier(input)?;
  let (input, args) = opt(fsm_args)(input)?;
  Ok((input, FsmInstance{name,args} ))
}

// fsm_args := "(", list0(list_separator, (call_arg_with_binding | call_arg)), ")" ;
pub fn fsm_args(input: ParseString) -> ParseResult<Vec<(Option<Identifier>,Expression)>> {
  let (input, _) = left_parenthesis(input)?;
  let (input, args) = separated_list0(list_separator, alt((call_arg_with_binding,call_arg)))(input)?;
  let (input, _) = right_parenthesis(input)?;
  Ok((input, args))
}
