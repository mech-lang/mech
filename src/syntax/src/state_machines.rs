#[macro_use]
use crate::*;
use nom::{
  multi::separated_list0,
  sequence::tuple as nom_tuple,
};

#[derive(Debug, Clone)]
struct InvalidFsmValuePatternError {
  message: String,
}

impl MechErrorKind for InvalidFsmValuePatternError {
  fn name(&self) -> &str {
    "InvalidFsmValuePattern"
  }

  fn message(&self) -> String {
    self.message.clone()
  }
}

fn invalid_fsm_value_pattern(msg: &str, ptrn: &Pattern) -> MechError {
  MechError::new(
    InvalidFsmValuePatternError {
      message: msg.to_string(),
    },
    None,
  )
  .with_compiler_loc()
  .with_tokens(ptrn.tokens())
}

fn validate_fsm_value_pattern(ptrn: &Pattern) -> MResult<()> {
  match ptrn {
    Pattern::Wildcard => Err(invalid_fsm_value_pattern(
      "Wildcard `*` is only valid in pattern-matching positions",
      ptrn,
    )),
    Pattern::Array(arr) => {
      if arr.spread.is_some() {
        return Err(invalid_fsm_value_pattern(
          "Array spread/rest syntax (`...` or `|`) is only valid in pattern-matching positions",
          ptrn,
        ));
      }
      for item in arr.prefix.iter().chain(arr.suffix.iter()) {
        validate_fsm_value_pattern(item)?;
      }
      Ok(())
    }
    Pattern::Tuple(tpl) => {
      for item in tpl.0.iter() {
        validate_fsm_value_pattern(item)?;
      }
      Ok(())
    }
    Pattern::TupleStruct(tpl) => {
      for item in tpl.patterns.iter() {
        validate_fsm_value_pattern(item)?;
      }
      Ok(())
    }
    Pattern::Expression(_) => Ok(()),
  }
}

fn fsm_value(input: ParseString) -> ParseResult<Pattern> {
  let (input, ptrn) = pattern(input)?;
  if validate_fsm_value_pattern(&ptrn).is_err() {
    return Err(nom::Err::Error(ParseError::new(
      input,
      "FSM RHS expects value syntax; wildcard and array spread/rest are only allowed in match patterns",
    )));
  }
  Ok((input, ptrn))
}

// State Machines
// ----------------------------------------------------------------------------

// Grammar: docs/design/specification.mec, `guard-operator`.
pub fn guard_operator(input: ParseString) -> ParseResult<()> {
  let (input, _) = whitespace0(input)?;
  let (input, _) = alt((tag("|"),tag("│"),tag("├"),tag("└")))(input)?;
  let (input, _) = whitespace0(input)?;
  Ok((input, ()))
}

// Grammar: docs/design/specification.mec, `fsm-implementation`.
pub fn fsm_implementation(input: ParseString) -> ParseResult<FsmImplementation> {
  let (input, _) = hashtag(input)?;
  let (input, name) = identifier(input)?;
  let (input, _) = left_parenthesis(input)?;
  let (input, input_vars) = separated_list0(list_separator, var)(input)?;
  let (input, _) = right_parenthesis(input)?;
  let (input, _) = transition_operator(input)?;
  let (input, start) = fsm_value(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, arms) = many1(fsm_arm)(input)?;
  let (input, _) = period(input)?;
  Ok((input, FsmImplementation{name,input: input_vars,start,arms}))
}

// Grammar: docs/design/specification.mec, `fsm-arm`.
pub fn fsm_arm(input: ParseString) -> ParseResult<FsmArm> {
  let (input, arm) = alt((fsm_guard_arm,fsm_transition,fsm_comment_arm))(input)?;
  let (input, _) = whitespace0(input)?;
  Ok((input, arm))
}

// Grammar: docs/design/specification.mec, `fsm-guard-arm`.
pub fn fsm_guard_arm(input: ParseString) -> ParseResult<FsmArm> {
  let (input, start) = pattern(input)?;
  let (input, grds) = many1(fsm_guard)(input)?;
  Ok((input, FsmArm::Guard(start, grds)))
}

pub fn fsm_comment_arm(input: ParseString) -> ParseResult<FsmArm> {
  let (input, comment) = comment(input)?;
  Ok((input, FsmArm::Comment(comment)))
}

// Grammar: docs/design/specification.mec, `fsm-guard`.
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

// Grammar: docs/design/specification.mec, `fsm-transition`.
pub fn fsm_transition(input: ParseString) -> ParseResult<FsmArm> {
  let (input, start) = pattern(input)?;
  let (input, trns) = many1(alt((
    fsm_block_transition,
    fsm_statement_transition,
    fsm_state_transition,
    fsm_output,
    fsm_async_transition)))(input)?;
  Ok((input, FsmArm::Transition(start, trns)))
}

// Grammar: docs/design/specification.mec, `fsm-state-transition`.
pub fn fsm_state_transition(input: ParseString) -> ParseResult<Transition> {
  let (input, _) = transition_operator(input)?;
  let (input, ptrn) = fsm_value(input)?;
  Ok((input, Transition::Next(ptrn)))
}

// Grammar: docs/design/specification.mec, `fsm-async-transition`.
pub fn fsm_async_transition(input: ParseString) -> ParseResult<Transition> {
  let (input, _) = async_transition_operator(input)?;
  let (input, ptrn) = fsm_value(input)?;
  Ok((input, Transition::Async(ptrn)))
}

// Grammar: docs/design/specification.mec, `fsm-statement-transition`.
pub fn fsm_statement_transition(input: ParseString) -> ParseResult<Transition> {
  let (input, _) = transition_operator(input)?;
  let (input, stmnt) = statement(input)?;
  Ok((input, Transition::Statement(stmnt)))
}

// Grammar: docs/design/specification.mec, `fsm-block-transition`.
pub fn fsm_block_transition(input: ParseString) -> ParseResult<Transition> {
  let (mut input, _) = transition_operator(input)?;
  let (next_input, _) = left_brace(input)?;
  input = next_input;

  let mut code = Vec::new();
  loop {
    let (next_input, _) = whitespace0(input.clone())?;
    input = next_input;
    if right_brace(input.clone()).is_ok() {
      break;
    }
    let (next_input, item) = mech_code_alt(input)?;
    input = next_input;
    let (next_input, comment) = match code_terminal(input.clone()) {
      Ok((next_input, comment)) => (next_input, comment),
      Err(_) => (input, None),
    };
    input = next_input;
    code.push((item, comment));
  }

  let (input, _) = right_brace(input)?;
  Ok((input, Transition::CodeBlock(code)))
}

// Grammar: docs/design/specification.mec, `fsm-output`.
pub fn fsm_output(input: ParseString) -> ParseResult<Transition> {
  let (input, _) = output_operator(input)?;
  let ((input, ptrn)) = fsm_value(input)?;
  Ok((input, Transition::Output(ptrn)))
}

// Grammar: docs/design/specification.mec, `fsm-specification`.
pub fn fsm_specification(input: ParseString) -> ParseResult<FsmSpecification> {
  let (input, _) = hashtag(input)?;
  let (input, name) = identifier(input)?;
  let (input, _) = left_parenthesis(input)?;
  let (input, input_vars) = separated_list0(list_separator, var)(input)?;
  let (input, _) = right_parenthesis(input)?;
  let (input, _) = opt(output_operator)(input)?;
  let (input, output) = opt(kind_annotation)(input)?;
  let (input, _) = opt(define_operator)(input)?;
  let (input, states) = many1(fsm_state_definition)(input)?;
  let (input, _) = period(input)?;
  Ok((input, FsmSpecification{name,input: input_vars, output, states}))
}

// Grammar: docs/design/specification.mec, `fsm-state-definition`.
pub fn fsm_state_definition(input: ParseString) -> ParseResult<StateDefinition> {
  let (input, _) = guard_operator(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, state_atom) = atom(input)?;
  let (input, vars) = opt(fsm_state_definition_variables)(input)?;
  Ok((input, StateDefinition{name: state_atom.name,state_variables: vars}))
}

// Grammar: docs/design/specification.mec, `fsm-state-definition-variables`.
pub fn fsm_state_definition_variables(input: ParseString) -> ParseResult<Vec<Var>> {
  let (input, _) = left_parenthesis(input)?;
  let (input, names) = separated_list1(list_separator, var)(input)?;
  let (input, _) = right_parenthesis(input)?;
  Ok((input, names))
}

// Grammar: docs/design/specification.mec, `fsm-pipe`.
pub fn fsm_pipe(input: ParseString) -> ParseResult<FsmPipe> {
  let (input, start) = fsm_instance(input)?;
  let (input, trns) = many0(alt((fsm_state_transition,fsm_async_transition,fsm_output)))(input)?;
  Ok((input, FsmPipe{start, transitions: trns}))
}

// Grammar: docs/design/specification.mec, `fsm-declare`.
pub fn fsm_declare(input: ParseString) -> ParseResult<FsmDeclare> {
  let (input, fsm) = fsm(input)?;
  let (input, _) = define_operator(input)?;
  let (input, pipe) = fsm_pipe(input)?;
  Ok((input, FsmDeclare{fsm,pipe}))
}
  
// Grammar: docs/design/specification.mec, `fsm`.
pub fn fsm(input: ParseString) -> ParseResult<Fsm> {
  let ((input, _)) = hashtag(input)?;
  let ((input, name)) = identifier(input)?;
  let ((input, args)) = opt(argument_list)(input)?;
  let ((input, kind)) = opt(kind_annotation)(input)?;
  Ok((input, Fsm{ name, args, kind }))
}

// Grammar: docs/design/specification.mec, `fsm-instance`.
pub fn fsm_instance(input: ParseString) -> ParseResult<FsmInstance> {
  let ((input, _)) = hashtag(input)?;
  let (input, name) = identifier(input)?;
  let (input, args) = opt(fsm_args)(input)?;
  Ok((input, FsmInstance{name,args} ))
}

// Grammar: docs/design/specification.mec, `fsm-args`.
pub fn fsm_args(input: ParseString) -> ParseResult<Vec<(Option<Identifier>,Expression)>> {
  let (input, _) = left_parenthesis(input)?;
  let (input, args) = separated_list0(list_separator, alt((call_arg_with_binding,call_arg)))(input)?;
  let (input, _) = right_parenthesis(input)?;
  Ok((input, args))
}
