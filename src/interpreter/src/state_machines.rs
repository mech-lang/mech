use crate::tracing::{
    format_fsm_trace, summarize_guard_condition, summarize_pattern, summarize_value,
};
use crate::*;
use crate::patterns::*;
use std::collections::HashSet;

// Finite State Machines
// ----------------------------------------------------------------------------

// Review: how does this fail?
pub fn register_fsm_implementation(fsm: &FsmImplementation, p: &Interpreter) -> MResult<()> {
  let fsm_id = fsm.name.hash();
  p.user_state_machines
    .borrow_mut()
    .insert(fsm_id, fsm.clone());
  p.state
    .borrow()
    .dictionary
    .borrow_mut()
    .insert(fsm_id, fsm.name.to_string());
  Ok(())
}

pub fn execute_fsm_pipe(fsm_pipe: &FsmPipe, env: Option<&Environment>, p: &Interpreter) -> MResult<Value> {
  let fsm_id = fsm_pipe.start.name.hash();
  let fsm = {
    let fsms = p.user_state_machines.borrow();
    fsms.get(&fsm_id).cloned()
  }
  .ok_or_else(|| {
    MechError::new(
      MissingFsmError {
        fsm_id,
      },
      None,
    )
    .with_compiler_loc()
    .with_tokens(fsm_pipe.start.tokens())
  })?;

  let mut call_env = Environment::new();
  let mut args = Vec::new();
  if let Some(start_args) = &fsm_pipe.start.args {
    for (_, arg_expr) in start_args {
      args.push(expression(arg_expr, env, p)?);
    }
  }
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
  for (arg_decl, arg_value) in fsm.input.iter().zip(args.iter()) {
    let detached_arg = detach_value(arg_value);
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
      call_env.insert(arg_decl.name.hash(), converted_arg.unwrap_or(detached_arg));
      continue;
    }
    call_env.insert(arg_decl.name.hash(), detached_arg);
  }
  let mut state = pattern_to_value(&fsm.start, &call_env, p)?;
  validate_fsm_state_coverage(&fsm, fsm_pipe)?;
  execute_fsm_pipe_impl(&fsm, &mut state, &mut call_env, p)
}

fn execute_fsm_pipe_impl(fsm: &FsmImplementation, state: &mut Value, call_env: &mut Environment, p: &Interpreter) -> MResult<Value> {
  trace_println!(
    p,
    "{}",
    format_fsm_trace(
      "start",
      format!(
        "name={} state={}",
        fsm.name.to_string(),
        summarize_value(state)
      )
    )
  );
  // Step through the FSM, applying transitions until we hit a terminal state (no applicable transitions) or exceed the step limit.
  for step in 0..p.max_steps {
    trace_println!(
      p,
      "{}",
      format_fsm_trace(
        "step",
        format!("{step:>4} state={}", summarize_value(state))
      )
    );
    let mut transitioned = false;
    for (arm_idx, arm) in fsm.arms.iter().enumerate() {
      match arm {
        FsmArm::Transition(pattern, transitions) => {
          let mut arm_env = call_env.clone();
          clear_pattern_bindings(pattern, &mut arm_env);
          let matched = pattern_matches_value(pattern, state, &mut arm_env, p)?;
          trace_println!(
            p,
            "{}",
            format_fsm_trace(
              "arm",
              format!(
                "[{arm_idx}] check transition pattern={} {}",
                summarize_pattern(pattern),
                if matched { "✓" } else { "✗" }
              )
            )
          );
          if matched {
            let previous_state = summarize_value(state);
            let out = apply_transitions(transitions, state, &mut arm_env, p)?;
            *call_env = arm_env;
            if let Some(value) = out {
              trace_println!(
                p,
                "{}",
                format_fsm_trace(
                  "output",
                  format!("value={}", summarize_value(&value))
                )
              );
              return Ok(value);
            }
            trace_println!(
              p,
              "{}",
              format_fsm_trace(
                "transition",
                format!(
                  "arm[{arm_idx}] {} -> {}",
                  previous_state,
                  summarize_value(state)
                )
              )
            );
            transitioned = true;
            break;
          }
        }
        FsmArm::Guard(pattern, guards) => {
          let mut arm_env = call_env.clone();
          clear_pattern_bindings(pattern, &mut arm_env);
          let pattern_matched = pattern_matches_value(pattern, state, &mut arm_env, p)?;
          trace_println!(
            p,
            "{}",
            format_fsm_trace(
              "arm",
              format!(
                "[{arm_idx}] check guard pattern={} {}",
                summarize_pattern(pattern),
                if pattern_matched { "✓" } else { "✗" }
              )
            )
          );
          if !pattern_matched {
            continue;
          }
          for (guard_idx, guard) in guards.iter().enumerate() {
            let guard_passes = match &guard.condition {
              Pattern::Wildcard => true,
              _ => {
                let cond = pattern_to_value(&guard.condition,&arm_env,p)?;
                match cond {
                  Value::Bool(x) => *x.borrow(),
                  other => {
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
              p,
              "{}",
              format_fsm_trace(
                "guard",
                format!(
                  "arm[{arm_idx}] check guard[{guard_idx}] condition={} {}",
                  summarize_guard_condition(&guard.condition),
                  if guard_passes { "✓" } else { "✗" }
                )
              )
            );
            if !guard_passes {
              continue;
            }
            let previous_state = summarize_value(state);
            let out = apply_transitions(&guard.transitions, state, &mut arm_env, p)?;
            *call_env = arm_env;
            if let Some(value) = out {
              trace_println!(
                p,
                "{}",
                format_fsm_trace(
                  "output",
                  format!("value={}", summarize_value(&value))
                )
              );
              return Ok(value);
            }
            trace_println!(
              p,
              "{}",
              format_fsm_trace(
                "transition",
                format!(
                  "arm[{arm_idx}] {} -> {}",
                  previous_state,
                  summarize_value(state)
                )
              )
            );
            transitioned = true;
            break;
          }
          if transitioned {
            break;
          }
        }
      }
    }
    if !transitioned {
      trace_println!(
        p,
        "{}",
        format_fsm_trace("halt", format!("state={}", summarize_value(state)))
      );
      return Ok(state.clone());
    }
  }
  Err(MechError::new(
    FsmExceededTransitionLimitError {
      max_transitions: p.max_steps,
    },
    None,
  )
  .with_compiler_loc())
}

fn validate_fsm_state_coverage(fsm: &FsmImplementation, fsm_pipe: &FsmPipe) -> MResult<()> {
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
    return Ok(());
  }

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

  for arm in &fsm.arms {
    let transitions = match arm {
      FsmArm::Transition(_, transitions) => transitions.as_slice(),
      FsmArm::Guard(_, guards) => {
        for guard in guards {
          for transition in &guard.transitions {
              validate_transition_target_state(transition, fsm, &state_names, fsm_pipe)?;
          }
        }
        &[]
      }
    };
    for transition in transitions {
      validate_transition_target_state(transition, fsm, &state_names, fsm_pipe)?;
    }
  }
  Ok(())
}

fn validate_transition_target_state(transition: &Transition, fsm: &FsmImplementation, state_names: &HashSet<String>, fsm_pipe: &FsmPipe) -> MResult<()> {
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

fn state_name_from_pattern(pattern: &Pattern) -> Option<String> {
  match pattern {
    Pattern::TupleStruct(tuple_struct) => Some(tuple_struct.name.to_string()),
    Pattern::Expression(Expression::Literal(Literal::Atom(atom))) => {
      Some(atom.name.to_string())
    }
    _ => None,
  }
}

fn apply_transitions(transitions: &[Transition], state: &mut Value, env: &mut Environment, p: &Interpreter) -> MResult<Option<Value>> {
  for transition in transitions {
    match transition {
      Transition::Next(next_pattern) | Transition::Async(next_pattern) => {
        *state = pattern_to_value(next_pattern, env, p)?;
      }
      Transition::Output(output_pattern) => {
        return Ok(Some(pattern_to_value(
          output_pattern,
          env,
          p,
        )?));
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
  Ok(None)
}

// FSM Errors
// ----------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct MissingFsmError {
  pub fsm_id: u64,
}

impl MechErrorKind for MissingFsmError {
  fn name(&self) -> &str {
    "MissingFsm"
  }
  fn message(&self) -> String {
    format!("FSM with id {} not found", self.fsm_id)
  }
}

#[derive(Debug, Clone)]
pub struct FsmUndefinedStateError {
  pub fsm_name: String,
  pub state_name: String,
}

impl MechErrorKind for FsmUndefinedStateError {
  fn name(&self) -> &str {
    "FsmUndefinedState"
  }
  fn message(&self) -> String {
    format!(
      "FSM '{}' references undefined state '{}'",
      self.fsm_name, self.state_name
    )
  }
}

#[derive(Debug, Clone)]
pub struct FsmGuardConditionKindMismatchError {
  pub arm_index: usize,
  pub guard_index: usize,
  pub actual_kind: ValueKind,
}

impl MechErrorKind for FsmGuardConditionKindMismatchError {
  fn name(&self) -> &str {
    "FsmGuardConditionKindMismatch"
  }

  fn message(&self) -> String {
    format!(
      "FSM guard condition arm[{}] guard[{}] must evaluate to Bool, got '{}'",
      self.arm_index,
      self.guard_index,
      self.actual_kind.to_string(),
    )
  }
}

#[derive(Debug, Clone)]
pub struct FsmExceededTransitionLimitError {
  pub max_transitions: usize,
}

impl MechErrorKind for FsmExceededTransitionLimitError {
  fn name(&self) -> &str {
    "FsmExceededTransitionLimit"
  }

  fn message(&self) -> String {
    format!(
      "FSM exceeded maximum transition limit of {} steps",
      self.max_transitions
    )
  }
}

#[derive(Debug, Clone)]
pub struct FsmArgumentKindMismatchError {
  pub argument: String,
  pub expected_kind: ValueKind,
  pub actual_kind: ValueKind,
}

impl MechErrorKind for FsmArgumentKindMismatchError {
  fn name(&self) -> &str {
    "FsmArgumentKindMismatch"
  }
  fn message(&self) -> String {
    format!(
      "FSM argument '{}' expected kind '{}' but received '{}'",
      self.argument,
      self.expected_kind.to_string(),
      self.actual_kind.to_string()
    )
  }
}
