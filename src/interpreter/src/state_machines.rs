use crate::tracing::{
    format_fsm_trace, summarize_guard_condition, summarize_pattern, summarize_value,
};
use crate::*;
#[cfg(feature = "state_machines")]
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

#[cfg(feature = "state_machines")]
pub fn execute_fsm_pipe(
    fsm_pipe: &FsmPipe,
    env: Option<&Environment>,
    p: &Interpreter,
) -> MResult<Value> {
    let fsm_id = fsm_pipe.start.name.hash();
    let fsm = {
        let fsms = p.user_state_machines.borrow();
        fsms.get(&fsm_id).cloned()
    }
    .ok_or_else(|| {
        MechError::new(
            MissingFunctionError {
                function_id: fsm_id,
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
        #[cfg(feature = "kind_annotation")]
        if let Some(kind_annotation_node) = &arg_decl.kind {
            let expected_kind = kind_annotation(&kind_annotation_node.kind, p)?
                .to_value_kind(&p.state.borrow().kinds)?;
            let actual_kind = arg_value.kind();
            if actual_kind != expected_kind {
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
        }
        call_env.insert(arg_decl.name.hash(), detach_value(arg_value));
    }
    let mut state = pattern_to_value(&fsm.start, &call_env, p)?;
    execute_fsm_pipe_impl(&fsm, &mut state, &mut call_env, p)
}

#[cfg(feature = "state_machines")]
fn execute_fsm_pipe_impl(
    fsm: &FsmImplementation,
    state: &mut Value,
    call_env: &mut Environment,
    p: &Interpreter,
) -> MResult<Value> {
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
    let max_steps = 10_000usize; // TODO This must be a parameter
    for step in 0..max_steps {
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
                                let cond = pattern_to_value(&guard.condition, &arm_env, p)?;
                                matches!(cond, Value::Bool(x) if *x.borrow())
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
        FeatureNotEnabledError,
        Some("FSM exceeded maximum transition limit".to_string()),
    )
    .with_compiler_loc())
}

#[cfg(feature = "state_machines")]
#[derive(Debug, Clone)]
pub struct FsmArgumentKindMismatchError {
    pub argument: String,
    pub expected_kind: ValueKind,
    pub actual_kind: ValueKind,
}

#[cfg(feature = "state_machines")]
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

#[cfg(feature = "state_machines")]
fn clear_pattern_bindings(pattern: &Pattern, env: &mut Environment) {
    let mut ids = Vec::new();
    collect_pattern_variable_ids(pattern, &mut ids);
    for var_id in ids {
        env.remove(&var_id);
    }
}

#[cfg(feature = "state_machines")]
fn collect_pattern_variable_ids(pattern: &Pattern, ids: &mut Vec<u64>) {
    match pattern {
        Pattern::Expression(Expression::Var(var)) => ids.push(var.name.hash()),
        Pattern::Tuple(tuple) => {
            for item in &tuple.0 {
                collect_pattern_variable_ids(item, ids);
            }
        }
        Pattern::TupleStruct(tuple_struct) => {
            for item in &tuple_struct.patterns {
                collect_pattern_variable_ids(item, ids);
            }
        }
        _ => {}
    }
}

#[cfg(feature = "state_machines")]
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
    Ok(None)
}

#[cfg(feature = "state_machines")]
fn pattern_to_value(pattern: &Pattern, env: &Environment, p: &Interpreter) -> MResult<Value> {
    match pattern {
        Pattern::Wildcard => Ok(Value::Empty),
        Pattern::Expression(expr) => expression(expr, Some(env), p),
        Pattern::Tuple(pattern_tuple) => {
            let mut values = Vec::with_capacity(pattern_tuple.0.len());
            for inner in &pattern_tuple.0 {
                values.push(pattern_to_value(inner, env, p)?);
            }
            Ok(Value::Tuple(Ref::new(MechTuple::from_vec(values))))
        }
        Pattern::TupleStruct(pattern_tuple_struct) => {
            let mut values = Vec::with_capacity(pattern_tuple_struct.patterns.len() + 1);
            values.push(atom(
                &Atom {
                    name: pattern_tuple_struct.name.clone(),
                },
                p,
            ));
            for inner in &pattern_tuple_struct.patterns {
                values.push(pattern_to_value(inner, env, p)?);
            }
            Ok(Value::Tuple(Ref::new(MechTuple::from_vec(values))))
        }
    }
}
