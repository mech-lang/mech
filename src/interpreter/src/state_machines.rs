use crate::tracing::{
    format_fsm_trace, summarize_guard_condition, summarize_pattern, summarize_value,
};
use crate::*;
use crate::patterns::*;
use std::collections::HashSet;

// Finite State Machines
// ----------------------------------------------------------------------------

/// Registers an FSM definition into the interpreter's runtime lookup tables.
/// Hashes the FSM name to produce a stable ID used as the key in both the
/// state machine store and the human-readable dictionary (for error messages).
pub fn register_fsm_implementation(fsm: &FsmImplementation, p: &Interpreter) -> MResult<()> {
  let fsm_id = fsm.name.hash();

  let mut state_machines = p.user_state_machines.borrow_mut();
  if state_machines.contains_key(&fsm_id) {
    return Err(MechError::new(
      DuplicateFsmError {
        fsm_name: fsm.name.to_string(),
        fsm_id,
      },
      None,
    )
    .with_compiler_loc());
  }
  state_machines.insert(fsm_id, fsm.clone());
  drop(state_machines); // release before second borrow_mut

  p.state
    .borrow()
    .dictionary
    .borrow_mut()
    .insert(fsm_id, fsm.name.to_string());

  Ok(())
}

/// Entry point for executing an FSM invocation written as a pipe expression,
/// e.g. `MyFsm(arg1, arg2) |> ...`.
/// Handles argument evaluation, arity checking, optional kind validation,
/// start-state construction, state-coverage validation, then delegates to the
/// step loop.
pub fn execute_fsm_pipe(fsm_pipe: &FsmPipe, env: Option<&Environment>, p: &Interpreter) -> MResult<Value> {
  // Look up the FSM by its hashed name; fail early with a descriptive error
  // that includes the source tokens so the user gets a good error location.
  let fsm_id = fsm_pipe.start.name.hash();
  let fsm = {
    let fsms = p.user_state_machines.borrow();
    fsms.get(&fsm_id).cloned()
  }
  .ok_or_else(|| {
    MechError::new(MissingFsmError { fsm_id }, None)
      .with_compiler_loc()
      .with_tokens(fsm_pipe.start.tokens())
  })?;

  // Evaluate every argument expression in the call site against the current env.
  let mut call_env = Environment::new();
  let mut args = Vec::new();
  if let Some(start_args) = &fsm_pipe.start.args {
    for (_, arg_expr) in start_args {
      args.push(expression(arg_expr, env, p)?);
    }
  }

  // Arity check — the number of supplied args must match the FSM's declared inputs.
  if fsm.input.len() != args.len() {
    return Err(MechError::new(
      IncorrectNumberOfArguments {
        expected: fsm.input.len(),
        found: args.len(),
      },
      None,
    )
    .with_compiler_loc()
    .with_tokens(fsm_pipe.start.tokens()));
  }

  // Bind each argument into the FSM's local environment.
  // When the `kind_annotation` feature is enabled, attempt an implicit type
  // conversion; reject the call if the kinds are incompatible and no conversion
  // exists.
  for (arg_decl, arg_value) in fsm.input.iter().zip(args.iter()) {
    let detached_arg = detach_value(arg_value); // snapshot: breaks any shared references
    #[cfg(feature = "kind_annotation")]
    if let Some(kind_annotation_node) = &arg_decl.kind {
      let expected_kind = kind_annotation(&kind_annotation_node.kind, p)?
          .to_value_kind(&p.state.borrow().kinds)?;
      let actual_kind = arg_value.kind();
      let converted_arg = detached_arg.convert_to(&expected_kind);
      if actual_kind != expected_kind && converted_arg.is_none() {
        return Err(MechError::new(
          FsmArgumentKindMismatchError {
            argument: arg_decl.name.to_string(),
            expected_kind,
            actual_kind,
          },
          None,
        )
        .with_compiler_loc()
        .with_tokens(fsm_pipe.start.tokens()));
      }
      // Prefer the converted value; fall back to the original if conversion
      // was a no-op (i.e. kinds already matched).
      call_env.insert(arg_decl.name.hash(), converted_arg.unwrap_or(detached_arg));
      continue;
    }
    call_env.insert(arg_decl.name.hash(), detached_arg);
  }

  // Build the initial state value from the FSM's declared start pattern.
  let mut state = pattern_to_value(&fsm.start, &call_env, p)?;

  // Static check: every state referenced by transitions must be declared as
  // an arm pattern. Catches typos and missing arms before we start running.
  validate_fsm_state_coverage(&fsm, fsm_pipe)?;

  execute_fsm_pipe_impl(&fsm, &mut state, &mut call_env, p)
}

/// Core FSM step loop. Iterates up to `p.max_steps` times; on each iteration
/// it scans every arm in order and applies the first one whose pattern matches
/// the current state.
///
/// Returns when:
///   • an `Output` transition is reached  → returns the output value, or
///   • no arm matches the current state   → returns the current state as-is
///     (terminal / halted), or
///   • the step limit is exceeded         → returns an error.
fn execute_fsm_pipe_impl(fsm: &FsmImplementation, state: &mut Value, call_env: &mut Environment, p: &Interpreter) -> MResult<Value> {
  trace_println!(
    p, "{}",
    format_fsm_trace("start", format!("name={} state={}", fsm.name.to_string(), summarize_value(state)))
  );

  for step in 0..p.max_steps {
    trace_println!(
      p, "{}",
      format_fsm_trace("step", format!("{step:>4} state={}", summarize_value(state)))
    );

    let mut transitioned = false; // becomes true if any arm fires this step

    for (arm_idx, arm) in fsm.arms.iter().enumerate() {
      match arm {

        // ── Transition arm ──────────────────────────────────────────────────
        // Simple: if the pattern matches, run the transition list immediately.
        FsmArm::Transition(pattern, transitions) => {
          let mut arm_env = call_env.clone();
          clear_pattern_bindings(pattern, &mut arm_env); // reset bindings before matching
          let matched = pattern_matches_value(pattern, state, &mut arm_env, p)?;
          trace_println!(
            p, "{}",
            format_fsm_trace("arm", format!(
              "[{arm_idx}] check transition pattern={} {}",
              summarize_pattern(pattern),
              if matched { "✓" } else { "✗" }
            ))
          );
          if matched {
            let previous_state = summarize_value(state);
            let out = apply_transitions(transitions, state, &mut arm_env, p)?;
            *call_env = arm_env; // commit bindings mutated during the arm
            if let Some(value) = out {
              // Output transition — we're done.
              trace_println!(p, "{}", format_fsm_trace("output", format!("value={}", summarize_value(&value))));
              return Ok(value);
            }
            trace_println!(
              p, "{}",
              format_fsm_trace("transition", format!("arm[{arm_idx}] {} -> {}", previous_state, summarize_value(state)))
            );
            transitioned = true;
            break; // restart the arm scan with the new state
          }
        }

        // ── Guard arm ───────────────────────────────────────────────────────
        // Two-level match: the outer pattern must match the state first, then
        // each guard clause is evaluated in order and the first truthy one wins.
        // A wildcard guard (`*`) is unconditionally true, acting as an else branch.
        FsmArm::Guard(pattern, guards) => {
          let mut arm_env = call_env.clone();
          clear_pattern_bindings(pattern, &mut arm_env);
          let pattern_matched = pattern_matches_value(pattern, state, &mut arm_env, p)?;
          trace_println!(
            p, "{}",
            format_fsm_trace("arm", format!(
              "[{arm_idx}] check guard pattern={} {}",
              summarize_pattern(pattern),
              if pattern_matched { "✓" } else { "✗" }
            ))
          );
          if !pattern_matched {
            continue; // outer pattern didn't match — skip the whole guard arm
          }

          for (guard_idx, guard) in guards.iter().enumerate() {
            // Evaluate the guard condition; wildcard always passes.
            let guard_passes = match &guard.condition {
              Pattern::Wildcard => true,
              _ => {
                let cond = pattern_to_value(&guard.condition, &arm_env, p)?;
                match cond {
                  Value::Bool(x) => *x.borrow(),
                  other => {
                    // Guard conditions must evaluate to Bool — anything else is a type error.
                    return Err(MechError::new(
                      FsmGuardConditionKindMismatchError {
                        arm_index: arm_idx,
                        guard_index: guard_idx,
                        actual_kind: other.kind(),
                      },
                      None,
                    )
                    .with_compiler_loc());
                  }
                }
              }
            };
            trace_println!(
              p, "{}",
              format_fsm_trace("guard", format!(
                "arm[{arm_idx}] check guard[{guard_idx}] condition={} {}",
                summarize_guard_condition(&guard.condition),
                if guard_passes { "✓" } else { "✗" }
              ))
            );
            if !guard_passes {
              continue; // try next guard clause
            }

            let previous_state = summarize_value(state);
            let out = apply_transitions(&guard.transitions, state, &mut arm_env, p)?;
            *call_env = arm_env;
            if let Some(value) = out {
              trace_println!(p, "{}", format_fsm_trace("output", format!("value={}", summarize_value(&value))));
              return Ok(value);
            }
            trace_println!(
              p, "{}",
              format_fsm_trace("transition", format!("arm[{arm_idx}] {} -> {}", previous_state, summarize_value(state)))
            );
            transitioned = true;
            break; // one guard fired — stop checking other clauses
          }

          if transitioned {
            break; // restart the outer arm scan with the new state
          }
        }
      }
    }

    // No arm matched the current state → the machine has halted naturally.
    if !transitioned {
      trace_println!(p, "{}", format_fsm_trace("halt", format!("state={}", summarize_value(state))));
      return Ok(state.clone());
    }
  }

  // Fell through the step limit — probably an infinite loop in the FSM definition.
  Err(MechError::new(
    FsmExceededTransitionLimitError { max_transitions: p.max_steps },
    None,
  )
  .with_compiler_loc())
}

/// Static validation pass run before execution begins.
///
/// Collects the set of named states declared across all arms, then checks:
///   1. The start state is in that set (if the set is non-empty).
///   2. Every `Next` / `Async` target state in every transition is also in the set.
///
/// If the FSM uses only anonymous / structural patterns (no named states),
/// the set will be empty and we skip the check entirely — it's not meaningful
/// to validate coverage for purely data-driven machines.
fn validate_fsm_state_coverage(fsm: &FsmImplementation, fsm_pipe: &FsmPipe) -> MResult<()> {
  // Gather state names declared as arm patterns (TupleStruct or Atom literals).
  let state_names: HashSet<String> = fsm
    .arms
    .iter()
    .filter_map(|arm| {
      let pattern = match arm {
        FsmArm::Guard(pattern, _) | FsmArm::Transition(pattern, _) => pattern,
      };
      state_name_from_pattern(pattern)
    })
    .collect();

  if state_names.is_empty() {
    return Ok(()); // nothing to validate
  }

  // Check the declared start state exists.
  let start_state = state_name_from_pattern(&fsm.start).ok_or_else(|| {
    MechError::new(
      FsmUndefinedStateError {
        fsm_name: fsm.name.to_string(),
        state_name: summarize_pattern(&fsm.start),
      },
      None,
    )
    .with_compiler_loc()
    .with_tokens(fsm_pipe.start.tokens())
  })?;
  if !state_names.contains(&start_state) {
    return Err(MechError::new(
      FsmUndefinedStateError {
        fsm_name: fsm.name.to_string(),
        state_name: start_state,
      },
      None,
    )
    .with_compiler_loc()
    .with_tokens(fsm_pipe.start.tokens()));
  }

  // Check every transition target.
  for arm in &fsm.arms {
    let transitions = match arm {
      FsmArm::Transition(_, transitions) => transitions.as_slice(),
      FsmArm::Guard(_, guards) => {
        // Validate targets inside every guard clause.
        for guard in guards {
          for transition in &guard.transitions {
            validate_transition_target_state(transition, fsm, &state_names, fsm_pipe)?;
          }
        }
        &[] // guard arm's top-level slice is empty; transitions are nested
      }
    };
    for transition in transitions {
      validate_transition_target_state(transition, fsm, &state_names, fsm_pipe)?;
    }
  }
  Ok(())
}

/// Checks that a single transition's target state (if it has one) is in the
/// known state set. Only `Next` and `Async` transitions carry target states;
/// `Output`, `Statement`, and `CodeBlock` do not name a next state.
fn validate_transition_target_state(
  transition: &Transition,
  fsm: &FsmImplementation,
  state_names: &HashSet<String>,
  fsm_pipe: &FsmPipe,
) -> MResult<()> {
  let target = match transition {
    Transition::Next(pattern) | Transition::Async(pattern) => state_name_from_pattern(pattern),
    _ => None,
  };
  if let Some(state_name) = target {
    if !state_names.contains(&state_name) {
      return Err(MechError::new(
        FsmUndefinedStateError {
          fsm_name: fsm.name.to_string(),
          state_name,
        },
        None,
      )
      .with_compiler_loc()
      .with_tokens(fsm_pipe.start.tokens()));
    }
  }
  Ok(())
}

/// Extracts the state name from a pattern if it represents a named state.
/// Returns `Some(name)` for `TupleStruct` variants (e.g. `Running(x)`) and
/// bare `Atom` literals (e.g. `:idle`); returns `None` for all other patterns.
fn state_name_from_pattern(pattern: &Pattern) -> Option<String> {
  match pattern {
    Pattern::TupleStruct(tuple_struct) => Some(tuple_struct.name.to_string()),
    Pattern::Expression(Expression::Literal(Literal::Atom(atom))) => Some(atom.name.to_string()),
    _ => None,
  }
}

/// Applies a list of transition directives in order against the current state.
///
/// | Directive    | Effect                                                        |
/// |-------------|---------------------------------------------------------------|
/// | `Next`      | Replaces `*state` with the evaluated next-state pattern.      |
/// | `Async`     | Same as `Next` for now (async scheduling not yet wired up).   |
/// | `Output`    | Returns a value immediately, ending the FSM execution.        |
/// | `Statement` | Executes a side-effecting statement; mutates `env` in place.  |
/// | `CodeBlock` | Executes a sequence of top-level Mech code lines.             |
///
/// Returns `Ok(Some(value))` when an `Output` is reached, `Ok(None)` otherwise.
fn apply_transitions(
  transitions: &[Transition],
  state: &mut Value,
  env: &mut Environment,
  p: &Interpreter,
) -> MResult<Option<Value>> {
  for transition in transitions {
    match transition {
      Transition::Next(next_pattern) | Transition::Async(next_pattern) => {
        *state = pattern_to_value(next_pattern, env, p)?;
      }
      Transition::Output(output_pattern) => {
        return Ok(Some(pattern_to_value(output_pattern, env, p)?));
      }
      Transition::Statement(stmt) => {
        statement(stmt, Some(env), p)?;
      }
      Transition::CodeBlock(code) => {
        for (line, _) in code {
          mech_code(line, p)?;
        }
      }
    }
  }
  Ok(None) // no Output reached; caller should check `transitioned` flag
}

// FSM Errors
// ----------------------------------------------------------------------------
// Each error type is a plain data struct (Clone + Debug) carrying the fields
// needed to produce a useful message. The `MechErrorKind` trait provides a
// stable `name()` (used for matching / logging) and a human-readable `message()`.

/// Returned when an FSM pipe references an FSM id that was never registered.
#[derive(Debug, Clone)]
pub struct MissingFsmError {
  pub fsm_id: u64,
}
impl MechErrorKind for MissingFsmError {
  fn name(&self) -> &str { "MissingFsm" }
  fn message(&self) -> String { format!("FSM with id {} not found", self.fsm_id) }
}

/// Returned when the start state or a transition target names a state that
/// has no corresponding arm pattern in the FSM definition.
#[derive(Debug, Clone)]
pub struct FsmUndefinedStateError {
  pub fsm_name: String,
  pub state_name: String,
}
impl MechErrorKind for FsmUndefinedStateError {
  fn name(&self) -> &str { "FsmUndefinedState" }
  fn message(&self) -> String {
    format!("FSM '{}' references undefined state '{}'", self.fsm_name, self.state_name)
  }
}

/// Returned when a guard condition evaluates to a non-Bool value.
#[derive(Debug, Clone)]
pub struct FsmGuardConditionKindMismatchError {
  pub arm_index: usize,
  pub guard_index: usize,
  pub actual_kind: ValueKind,
}
impl MechErrorKind for FsmGuardConditionKindMismatchError {
  fn name(&self) -> &str { "FsmGuardConditionKindMismatch" }
  fn message(&self) -> String {
    format!(
      "FSM guard condition arm[{}] guard[{}] must evaluate to Bool, got '{}'",
      self.arm_index, self.guard_index, self.actual_kind.to_string()
    )
  }
}

/// Returned when the FSM step loop reaches `p.max_steps` without halting,
/// indicating a likely infinite loop in the FSM's transition graph.
#[derive(Debug, Clone)]
pub struct FsmExceededTransitionLimitError {
  pub max_transitions: usize,
}
impl MechErrorKind for FsmExceededTransitionLimitError {
  fn name(&self) -> &str { "FsmExceededTransitionLimit" }
  fn message(&self) -> String {
    format!("FSM exceeded maximum transition limit of {} steps", self.max_transitions)
  }
}

/// Returned when a call-site argument's kind doesn't match the FSM's declared
/// input kind and no implicit conversion is available.
#[derive(Debug, Clone)]
pub struct FsmArgumentKindMismatchError {
  pub argument: String,
  pub expected_kind: ValueKind,
  pub actual_kind: ValueKind,
}
impl MechErrorKind for FsmArgumentKindMismatchError {
  fn name(&self) -> &str { "FsmArgumentKindMismatch" }
  fn message(&self) -> String {
    format!(
      "FSM argument '{}' expected kind '{}' but received '{}'",
      self.argument, self.expected_kind.to_string(), self.actual_kind.to_string()
    )
  }
}

/// Returned when an FSM definition is registered with a name that hashes to an ID
/// already in use by another FSM, indicating a naming collision. This is unlikely
/// but possible if two FSMs have different names that hash to the same value, or if
/// the same FSM is registered twice.
#[derive(Debug, Clone)]
pub struct DuplicateFsmError {
  pub fsm_name: String,
  pub fsm_id: u64,
}

impl MechErrorKind for DuplicateFsmError {
  fn name(&self) -> &str { "DuplicateFsm" }
  fn message(&self) -> String {
    format!(
      "FSM '{}' (id {}) has already been registered",
      self.fsm_name, self.fsm_id
    )
  }
}