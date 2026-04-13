use crate::*;
use crate::tracing::{
  format_trace,
  format_trace_args,
  summarize_function_pattern,
  summarize_function_value,
  summarize_values_with_kinds,
};
#[cfg(all(feature = "kind_annotation", feature = "enum"))]
use std::collections::HashSet;
use crate::*;

// Functions
// ============================================================================


// Frames
// ----------------------------------------------------------------------------

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum FrameState {
  Running,
  Suspended,
  Completed,
}

// One activation record on the call stack. Every user-function invocation gets
// its own Frame so locals and the instruction pointer don't bleed across calls.
#[derive(Clone)]
pub struct Frame {
  plan: Plan,
  ip: usize,              // index of the next instruction to execute
  locals: SymbolTableRef, // variables local to this invocation
  out: Option<Value>,     // value yielded by a coroutine, if any
  state: FrameState,      // Running / Suspended / Completed
}

// The call stack is a simple growable list of frames; the last entry is current.
#[derive(Clone)]
pub struct Stack {
  frames: Vec<Frame>,
}

// Registers a user-written function so it can be called by name later.
// Hashes the name to a u64 id used as the lookup key throughout the runtime.
pub fn function_define(fxn_def: &FunctionDefine, p: &Interpreter) -> MResult<FunctionDefinition> {
  let fxn_name_id = fxn_def.name.hash();
  let mut new_fxn = FunctionDefinition::new(fxn_name_id, fxn_def.name.to_string(), fxn_def.clone());

  // Record declared input arguments and their kind annotations.
  for input_arg in &fxn_def.input {
    new_fxn
      .input
      .insert(input_arg.name.hash(), input_arg.kind.clone());
  }

  // Record declared output arguments and their kind annotations.
  for output_arg in &fxn_def.output {
    new_fxn
      .output
      .insert(output_arg.name.hash(), output_arg.kind.clone());
  }

  // Store the definition and register the human-readable name string in both
  // dictionaries so error messages and debug output can print it.
  let functions = p.functions();
  let mut functions_brrw = functions.borrow_mut();
  functions_brrw
    .user_functions
    .insert(fxn_name_id, new_fxn.clone());
  functions_brrw
    .dictionary
    .borrow_mut()
    .insert(fxn_name_id, fxn_def.name.to_string());
  p.state
    .borrow()
    .dictionary
    .borrow_mut()
    .insert(fxn_name_id, fxn_def.name.to_string());

  Ok(new_fxn)
}

// Calls
// ----------------------------------------------------------------------------

// Dispatches a function call to whichever implementation is available:
// user-defined functions first, then built-in functions, then native compiled
// functions. Returns an error if the name is not found in any registry.
pub fn function_call(fxn_call: &FunctionCall, env: Option<&Environment>, p: &Interpreter) -> MResult<Value> {
  let functions = p.functions();
  let fxn_name_id = fxn_call.name.hash();

  // User-defined function: evaluate arguments then run the interpreted body.
  if let Some(user_fxn) = { functions.borrow().user_functions.get(&fxn_name_id).cloned() } {
    let mut input_arg_values = vec![];
    for (_, arg_expr) in fxn_call.args.iter() {
      input_arg_values.push(expression(arg_expr, env, p)?);
    }
    return execute_user_function(&user_fxn, &input_arg_values, p);
  }

  // Pre-compiled built-in functions.
  if { functions.borrow().functions.contains_key(&fxn_name_id) } {
    todo!();
  }

  // Native function compiler: the compiler picks a concrete implementation
  // based on the runtime argument types, then we execute it immediately.
  let fxn_compiler = {
    functions
      .borrow()
      .function_compilers
      .get(&fxn_name_id)
      .copied()
  };
  match fxn_compiler {
    Some(fxn_compiler) => {
      let mut input_arg_values = vec![];
      for (_, arg_expr) in fxn_call.args.iter() {
        input_arg_values.push(expression(arg_expr, env, p)?);
      }
      trace_println!(
        p,
        "{}",
        format_trace(
          "fn",
          format!(
            "native {}({})",
            fxn_call.name.to_string(),
            format_trace_args(&input_arg_values)
          ),
        )
      );
      execute_native_function_compiler(fxn_compiler, &input_arg_values, p)
    }
    // No implementation found under this name at all.
    None => Err(MechError::new(
      MissingFunctionError {
        function_id: fxn_name_id,
      },
      None,
    )
    .with_compiler_loc()
    .with_tokens(fxn_call.name.tokens())),
  }
}

// Asks a native function compiler to select the right concrete implementation
// for the given argument types, runs it once to produce an initial value, then
// pushes it onto the reactive plan so it re-runs when its inputs change.
pub fn execute_native_function_compiler(
  fxn_compiler: &'static dyn NativeFunctionCompiler,
  input_arg_values: &Vec<Value>,
  p: &Interpreter,
) -> MResult<Value> {
  let plan = p.plan();
  match fxn_compiler.compile(input_arg_values) {
    Ok(mut new_fxn) => {
      trace_println!(
        p,
        "{}",
        format_trace(
          "arm",
          format!(
            "selected {} args=[{}]",
            new_fxn
              .to_string()
              .lines()
              .next()
              .unwrap_or("<unknown-arm>"),
            format_trace_args(input_arg_values)
          ),
        )
      );
      let mut plan_brrw = plan.borrow_mut();
      new_fxn.solve();                   // run the function once to initialise its output
      let result = new_fxn.out();
      trace_println!(
        p,
        "{}",
        format_trace("arm", format!("result {}", summarize_function_value(&result)))
      );
      plan_brrw.push(new_fxn);          // keep it in the plan for reactive re-evaluation
      Ok(result)
    }
    Err(err) => Err(err),
  }
}

// Executes a user-defined function. Handles argument count validation,
// optional matrix broadcasting, match-arm dispatch, and plain statement bodies.
// Logs entry/exit (or failure) via the trace machinery.
fn execute_user_function(
  fxn_def: &FunctionDefinition,
  input_arg_values: &Vec<Value>,
  p: &Interpreter,
) -> MResult<Value> {
  // Reject calls with the wrong number of arguments before doing anything else.
  if input_arg_values.len() != fxn_def.input.len() {
    return Err(MechError::new(
      IncorrectNumberOfArguments {
        expected: fxn_def.input.len(),
        found: input_arg_values.len(),
      },
      None,
    )
    .with_compiler_loc()
    .with_tokens(fxn_def.code.name.tokens()));
  }

  // If the function takes a single matrix argument and the element kind matches
  // the output kind, broadcast element-wise instead of running the body once.
  #[cfg(feature = "matrix")]
  if let Some(result) = try_broadcast_user_function(fxn_def, input_arg_values, p)? {
    return Ok(result);
  }

  trace_println!(
    p,
    "{}",
    format_trace(
      "fn",
      format!(
        "enter {}({})",
        fxn_def.name,
        format_trace_args(input_arg_values)
      ),
    )
  );

  // Choose execution strategy: match-arm body vs. plain statement body.
  let output = if !fxn_def.code.match_arms.is_empty() {
    // Match-arm body: loop to support tail-call optimisation. Each iteration
    // opens a fresh scope, binds the current arguments, runs the arms, then
    // either returns the result or loops with a new argument set.
    let mut current_args: Vec<Value> = input_arg_values.clone();
    loop {
      let scope = FunctionScope::enter(p);
      bind_function_inputs(fxn_def, &current_args, p)?;
      let step: FunctionCallStep = execute_function_match_arms(fxn_def, &current_args, p)?;
      drop(scope);
      match step {
        FunctionCallStep::Return(value) => break Ok(value),
        // Tail call: swap in the new args and go around again without growing
        // the Rust call stack.
        FunctionCallStep::TailCall(next_args) => {
          current_args = next_args;
        }
      }
    }
  } else {
    // Plain statement body: run statements in order, then collect named outputs.
    let scope = FunctionScope::enter(p);
    bind_function_inputs(fxn_def, input_arg_values, p)?;
    for statement_node in &fxn_def.code.statements {
      statement(statement_node, None, p)?;
    }
    let result = collect_function_output(p, fxn_def);
    drop(scope);
    result
  };

  match output {
    Ok(value) => {
      trace_println!(
        p,
        "{}",
        format_trace(
          "fn",
          format!("exit  {} => {}", fxn_def.name, summarize_function_value(&value))
        )
      );
      Ok(value)
    }
    Err(err) => {
      trace_println!(
        p,
        "{}",
        format_trace("fn", format!("fail  {} => {:?}", fxn_def.name, err))
      );
      Err(err)
    }
  }
}

// The outcome of executing one match arm. Either we have a final value, or
// we identified a tail call and carry its new arguments for the next iteration.
enum FunctionCallStep {
  Return(Value),
  TailCall(Vec<Value>),
}

// If the function is single-input / single-output with matching scalar kinds,
// and the actual argument is a matrix, run the function on each element and
// reassemble the result into a matrix of the same shape.
// Returns None if any condition for broadcasting isn't met, so the caller can
// fall through to normal execution.
#[cfg(feature = "matrix")]
fn try_broadcast_user_function(
  fxn_def: &FunctionDefinition,
  input_arg_values: &Vec<Value>,
  p: &Interpreter,
) -> MResult<Option<Value>> {
  if input_arg_values.len() != 1
    || fxn_def.code.output.len() != 1
    || fxn_def.code.input.len() != 1
  {
    return Ok(None);
  }

  let source = detach_value(&input_arg_values[0]);
  if !source.is_matrix() {
    return Ok(None);
  }

  // Resolve the declared input and output kinds from their annotations.
  // Without kind_annotation feature we can't know the element type, so bail.
  #[cfg(feature = "kind_annotation")]
  let (input_kind, output_kind) = {
    let input_kind = kind_annotation(&fxn_def.code.input[0].kind.kind, p)?
      .to_value_kind(&p.state.borrow().kinds)?;
    let output_kind = kind_annotation(&fxn_def.code.output[0].kind.kind, p)?
      .to_value_kind(&p.state.borrow().kinds)?;
    (input_kind, output_kind)
  };

  #[cfg(not(feature = "kind_annotation"))]
  let (input_kind, output_kind) = {
    return Ok(None);
  };

  // Only broadcast when input and output kinds are the same scalar kind.
  // If the input is already a matrix kind, don't recurse.
  if input_kind != output_kind || matches!(input_kind, ValueKind::Matrix(_, _)) {
    return Ok(None);
  }

  let Some(elements) = crate::patterns::matrix_like_values(&source) else {
    return Ok(None);
  };

  // Apply the function element-wise, then reassemble into the original shape.
  let mut outputs = Vec::with_capacity(elements.len());
  for element in elements {
    outputs.push(execute_user_function(fxn_def, &vec![element], p)?);
  }

  let shape = source.shape();
  Ok(Some(build_typed_matrix_from_values(
    &output_kind,
    outputs,
    shape[0],
    shape[1],
  )))
}

// Assembles a list of scalar Values into a typed matrix.
// TODO add more types
#[cfg(feature = "matrix")]
fn build_typed_matrix_from_values(
  output_kind: &ValueKind,
  outputs: Vec<Value>,
  rows: usize,
  cols: usize,
) -> Value {
  match output_kind {
    #[cfg(feature = "f64")]
    ValueKind::F64 => Value::MatrixF64(f64::to_matrix(
      outputs
        .into_iter()
        .map(|value| {
          value
            .as_f64()
            .expect("Expected f64 output")
            .borrow()
            .clone()
        })
        .collect::<Vec<f64>>(),
      rows,
      cols,
    )),
    _ => Value::MatrixValue(Value::to_matrix(outputs, rows, cols)),
  }
}

// Tries each match arm in order against the current arguments. Handles:
//   - enum exhaustiveness checking (kind_annotation + enum features)
//   - tail-call detection (arm body is a recursive call with same arity)
//   - output kind coercion
// Returns an error if no arm matched.
fn execute_function_match_arms(
  fxn_def: &FunctionDefinition,
  input_arg_values: &Vec<Value>,
  p: &Interpreter,
) -> MResult<FunctionCallStep> {

  // Exhaustiveness check: when the single input is an enum type and there is
  // no wildcard arm, every variant must be covered or we report which ones
  // are missing before even attempting to run.
  #[cfg(all(feature = "kind_annotation", feature = "enum"))]
  {
    let has_wildcard = fxn_def
      .code
      .match_arms
      .iter()
      .any(|arm| matches!(arm.pattern, Pattern::Wildcard));
    if !has_wildcard && fxn_def.input.len() == 1 {
      if let Some((_, kind_annotation_node)) = fxn_def.input.iter().next() {
        let input_kind = kind_annotation(&kind_annotation_node.kind, p)?
          .to_value_kind(&p.state.borrow().kinds)?;
        if let ValueKind::Enum(enum_id, _) = input_kind {
          let state_brrw = p.state.borrow();
          if let Some(enum_def) = state_brrw.enums.get(&enum_id) {
            // Collect every variant name that appears in the written arms.
            let mut covered_variants: HashSet<u64> = HashSet::new();
            for arm in &fxn_def.code.match_arms {
              match &arm.pattern {
                #[cfg(feature = "atom")]
                Pattern::TupleStruct(tuple_struct) => {
                  covered_variants.insert(tuple_struct.name.hash());
                }
                Pattern::Expression(expr) => {
                  if let Expression::Literal(Literal::Atom(atom)) = expr {
                    covered_variants.insert(atom.name.hash());
                  }
                }
                _ => {}
              }
            }
            let all_covered = enum_def
              .variants
              .iter()
              .all(|(variant_id, _)| covered_variants.contains(variant_id));
            if !all_covered {
              // Build a readable list of the missing variant patterns.
              let missing_patterns = enum_def
                .variants
                .iter()
                .filter(|(variant_id, _)| !covered_variants.contains(variant_id))
                .map(|(variant_id, payload_kind)| {
                  let variant_name = enum_def
                    .names
                    .borrow()
                    .get(variant_id)
                    .cloned()
                    .unwrap_or_else(|| variant_id.to_string());
                  if payload_kind.is_some() {
                    format!(":{}(...)", variant_name)
                  } else {
                    format!(":{}", variant_name)
                  }
                })
                .collect::<Vec<String>>();
              return Err(MechError::new(
                FunctionMatchNonExhaustiveError {
                  function_name: fxn_def.name.clone(),
                  missing_patterns,
                },
                None,
              )
              .with_compiler_loc()
              .with_tokens(fxn_def.code.name.tokens()));
            }
          }
        }
      }
    }
  }

  // Try each arm in source order; the first one whose pattern matches wins.
  for (arm_idx, arm) in fxn_def.code.match_arms.iter().enumerate() {
    let mut env = Environment::new();
    let matched = crate::patterns::pattern_matches_arguments(
      &arm.pattern,
      input_arg_values,
      &mut env,
      p,
    )?;
    trace_println!(p, "{}", {
      let args_summary = summarize_values_with_kinds(input_arg_values);
      let pattern_summary = summarize_function_pattern(&arm.pattern);
      let marker = if matched { "✓" } else { "X" };
      format_trace(
        "match",
        format!(
          "arm[{arm_idx}] test pattern={pattern_summary} args=[{args_summary}] {marker}"
        ),
      )
    });
    if matched {
      // Tail-call optimisation: if the arm body is a direct recursive call
      // with the same arity, return new arguments instead of recursing.
      if let Expression::FunctionCall(fxn_call) = &arm.expression {
        if fxn_call.name.hash() == fxn_def.code.name.hash() {
          let mut tail_args = Vec::with_capacity(fxn_call.args.len());
          for (_, arg_expr) in fxn_call.args.iter() {
            tail_args.push(expression(arg_expr, Some(&env), p)?);
          }
          if tail_args.len() == fxn_def.input.len() {
            trace_println!(
              p,
              "{}",
              format_trace(
                "match",
                format!("arm[{arm_idx}] tail-call {}", fxn_def.name)
              )
            );
            return Ok(FunctionCallStep::TailCall(tail_args));
          }
        }
      }
      // Normal arm: evaluate the expression and coerce to the declared output kind.
      let out = expression(&arm.expression, Some(&env), p)?;
      let coerced = coerce_function_output_kind(detach_value(&out), fxn_def, p)?;
      trace_println!(
        p,
        "{}",
        format_trace(
          "match",
          format!(
            "arm[{arm_idx}] out  value={} kind={}",
            summarize_function_value(&coerced),
            coerced.kind().to_string()
          )
        )
      );
      return Ok(FunctionCallStep::Return(coerced));
    }
  }
  // No arm matched — this is a runtime error; the function has no defined output.
  Err(MechError::new(
    FunctionOutputUndefinedError {
      output_id: fxn_def.id,
    },
    None,
  )
  .with_compiler_loc()
  .with_tokens(fxn_def.code.name.tokens()))
}

// Coerces a match-arm result to the function's declared output kind.
// If no output annotation exists, or conversion fails, the value is returned as-is.
#[cfg(feature = "kind_annotation")]
fn coerce_function_output_kind(
  value: Value,
  fxn_def: &FunctionDefinition,
  p: &Interpreter,
) -> MResult<Value> {
  if fxn_def.output.is_empty() {
    return Ok(value);
  }
  let Some((_, output_kind_annotation)) = fxn_def.output.get_index(0) else {
    return Ok(value);
  };
  let target_kind =
    kind_annotation(&output_kind_annotation.kind, p)?.to_value_kind(&p.state.borrow().kinds)?;
  return Ok(value.convert_to(&target_kind).unwrap_or(value));
}

// RAII guard that swaps in a fresh symbol table and plan for the duration of a
// function call, then restores the previous ones on drop. This is what gives
// each function its own local variable namespace.
struct FunctionScope {
  state: Ref<ProgramState>,
  previous_symbols: SymbolTableRef,
  previous_plan: Plan,
  previous_environment: Option<SymbolTableRef>,
}

impl FunctionScope {
  fn enter(p: &Interpreter) -> Self {
    let state = p.state.clone();
    let mut state_brrw = state.borrow_mut();
    // A new symbol table that shares the global name dictionary so that
    // lookups by hash still resolve to human-readable names.
    let mut local_symbols = SymbolTable::new();
    local_symbols.dictionary = state_brrw.dictionary.clone();
    let local_symbols = Ref::new(local_symbols);
    let previous_symbols = std::mem::replace(&mut state_brrw.symbol_table, local_symbols);
    let previous_plan = std::mem::replace(&mut state_brrw.plan, Plan::new());
    let previous_environment = state_brrw.environment.take();
    drop(state_brrw);

    Self {
      state,
      previous_symbols,
      previous_plan,
      previous_environment,
    }
  }
}

// Restore the caller's symbol table, plan, and environment when the scope ends.
impl Drop for FunctionScope {
  fn drop(&mut self) {
    let mut state_brrw = self.state.borrow_mut();
    state_brrw.symbol_table = self.previous_symbols.clone();
    state_brrw.plan = self.previous_plan.clone();
    state_brrw.environment = self.previous_environment.clone();
  }
}

// Function Definitions
// ----------------------------------------------------------------------------

// Binds each argument value to the corresponding local variable name.
// With kind_annotation: validates and coerces argument types, including
// special handling for enum types where coercion rules differ.
fn bind_function_inputs(
  fxn_def: &FunctionDefinition,
  input_arg_values: &Vec<Value>,
  p: &Interpreter,
) -> MResult<()> {
  let scoped_state = p.state.borrow();
  for ((arg_id, input_kind_annotation), input_value) in
    fxn_def.input.iter().zip(input_arg_values.iter())
  {
    // Look up the human-readable argument name for error messages.
    let arg_name = fxn_def
      .code
      .input
      .iter()
      .find(|arg| arg.name.hash() == *arg_id)
      .map(|arg| arg.name.to_string())
      .unwrap_or_else(|| arg_id.to_string());

    let bound_value = {
      #[cfg(feature = "kind_annotation")]
      {
        let target_kind = kind_annotation(&input_kind_annotation.kind, p)?
          .to_value_kind(&p.state.borrow().kinds)?;
        let detached_input = detach_value(input_value);

        // Enum arguments are checked for membership rather than converted,
        // because coercion semantics don't apply across enum variants.
        #[cfg(all(feature = "enum", feature = "atom"))]
        if let ValueKind::Enum(enum_id, _) = &target_kind {
          let state_brrw = p.state.borrow();
          if enum_value_matches(detached_input.clone(), *enum_id, &state_brrw) {
            detached_input.clone()
          } else {
            return Err(MechError::new(
              FunctionInputTypeMismatchError {
                function_name: fxn_def.name.clone(),
                argument_name: arg_name.clone(),
                expected: target_kind.clone(),
                found: detached_input.kind(),
              },
              None,
            )
            .with_compiler_loc()
            .with_tokens(input_kind_annotation.tokens()));
          }
        } else {
          // Non-enum: attempt type conversion; error if it can't be done.
          detached_input
            .clone()
            .convert_to(&target_kind)
            .ok_or_else(|| {
              MechError::new(
                FunctionInputTypeMismatchError {
                  function_name: fxn_def.name.clone(),
                  argument_name: arg_name.clone(),
                  expected: target_kind.clone(),
                  found: detached_input.kind(),
                },
                None,
              )
              .with_compiler_loc()
              .with_tokens(input_kind_annotation.tokens())
            })?
        }
        #[cfg(not(all(feature = "enum", feature = "atom")))]
        detached_input
          .clone()
          .convert_to(&target_kind)
          .ok_or_else(|| {
            MechError::new(
              FunctionInputTypeMismatchError {
                function_name: fxn_def.name.clone(),
                argument_name: arg_name.clone(),
                expected: target_kind.clone(),
                found: detached_input.kind(),
              },
              None,
            )
            .with_compiler_loc()
            .with_tokens(input_kind_annotation.tokens())
          })?
      }
      // Without kind_annotation: accept the value as-is, just detach any reference.
      #[cfg(not(feature = "kind_annotation"))]
      {
        detach_value(input_value)
      }
    };
    scoped_state.save_symbol(*arg_id, arg_name, bound_value, false);
  }
  Ok(())
}

// Returns true if `value` is a valid member of the enum identified by `enum_id`.
// Handles bare atom variants and tuple-struct variants (atom tag + payload).
#[cfg(all(feature = "enum", feature = "atom"))]
fn enum_value_matches(value: Value, enum_id: u64, state: &ProgramState) -> bool {
  let enum_def = match state.enums.get(&enum_id) {
    Some(enm) => enm,
    None => return false,
  };
  match value {
    // Bare atom: check that the atom's id is a known payload-less variant.
    Value::Atom(atom) => {
      let variant_id = atom.borrow().id();
      enum_def
        .variants
        .iter()
        .any(|(known_variant, payload_kind)| {
          *known_variant == variant_id && payload_kind.is_none()
        })
    }
    // Tuple-struct variant: a 2-element tuple of (atom-tag, payload).
    // The tag must match a known variant and the payload must satisfy the
    // declared payload kind, recursing for nested enums.
    #[cfg(feature = "tuple")]
    Value::Tuple(tuple_val) => {
      let tuple_brrw = tuple_val.borrow();
      if tuple_brrw.elements.len() != 2 {
        return false;
      }
      let tag = match tuple_brrw.elements[0].as_ref() {
        Value::Atom(atom) => atom.borrow().id(),
        _ => return false,
      };
      let payload = tuple_brrw.elements[1].as_ref().clone();
      let (_, declared_payload_kind) = match enum_def
        .variants
        .iter()
        .find(|(known_variant, _)| *known_variant == tag)
      {
        Some(entry) => entry,
        None => return false,
      };
      match declared_payload_kind {
        Some(Value::Kind(expected_kind)) => match expected_kind {
          // Nested enum payload: recurse.
          ValueKind::Enum(inner_enum_id, _) => {
            enum_value_matches(payload, *inner_enum_id, state)
          }
          // Scalar payload: accept exact match or a convertible value.
          _ => {
            payload.kind() == expected_kind.clone()
              || payload.convert_to(expected_kind).is_some()
          }
        },
        _ => false,
      }
    }
    _ => false,
  }
}

// Reads each declared output variable out of the local symbol table and
// returns them as a single Value. Multiple outputs are wrapped in a Tuple;
// a single output is returned directly; zero outputs return Empty.
fn collect_function_output(p: &Interpreter, fxn_def: &FunctionDefinition) -> MResult<Value> {
  let symbols = p.symbols();
  let symbols_brrw = symbols.borrow();
  let mut outputs = vec![];

  for output_arg in &fxn_def.code.output {
    let output_id = output_arg.name.hash();
    match symbols_brrw.get(output_id) {
      Some(cell) => outputs.push(detach_value(&cell.borrow())),
      None => {
        return Err(
          MechError::new(FunctionOutputUndefinedError { output_id }, None)
            .with_compiler_loc()
            .with_tokens(output_arg.tokens()),
        );
      }
    }
  }

  Ok(match outputs.len() {
    0 => Value::Empty,
    1 => outputs.remove(0),
    _ => Value::Tuple(Ref::new(MechTuple::from_vec(outputs))),
  })
}

// Peels off any MutableReference wrappers to get to the underlying value.
// Used before storing arguments or returning results so callers always see
// plain owned values, not live references into other cells.
pub(crate) fn detach_value(value: &Value) -> Value {
  match value {
    Value::MutableReference(reference) => detach_value(&reference.borrow()),
    _ => value.clone(),
  }
}

// Function Errors
// ----------------------------------------------------------------------------

// The called function name doesn't exist in any registry.
#[derive(Debug, Clone)]
pub struct MissingFunctionError {
  pub function_id: u64,
}

impl MechErrorKind for MissingFunctionError {
  fn name(&self) -> &str {
    "MissingFunction"
  }
  fn message(&self) -> String {
    format!("Function with id {} not found", self.function_id)
  }
}

// A function's output variable was declared but never assigned during execution.
#[derive(Debug, Clone)]
pub struct FunctionOutputUndefinedError {
  pub output_id: u64,
}

impl MechErrorKind for FunctionOutputUndefinedError {
  fn name(&self) -> &str {
    "FunctionOutputUndefined"
  }
  fn message(&self) -> String {
    format!(
      "Function output {} was declared but never defined",
      self.output_id
    )
  }
}

// A match-arm function doesn't cover every variant of its enum input type.
#[derive(Debug, Clone)]
pub struct FunctionMatchNonExhaustiveError {
  pub function_name: String,
  pub missing_patterns: Vec<String>,
}

impl MechErrorKind for FunctionMatchNonExhaustiveError {
  fn name(&self) -> &str {
    "FunctionMatchNonExhaustive"
  }

  fn message(&self) -> String {
    format!(
      "Function '{}' has non-exhaustive match arms. Missing patterns: {}. Add the missing patterns or add a wildcard (`*`) arm.",
      self.function_name,
      self.missing_patterns.join(", ")
    )
  }
}

// A value passed to a function argument didn't match the declared kind and
// couldn't be coerced to it.
#[derive(Debug, Clone)]
pub struct FunctionInputTypeMismatchError {
  pub function_name: String,
  pub argument_name: String,
  pub expected: ValueKind,
  pub found: ValueKind,
}

impl MechErrorKind for FunctionInputTypeMismatchError {
  fn name(&self) -> &str {
    "FunctionInputTypeMismatch"
  }

  fn message(&self) -> String {
    format!(
      "Function '{}' argument '{}' expected {}, found {}",
      self.function_name, self.argument_name, self.expected, self.found
    )
  }
}