use crate::patterns::PatternBindingSink;
use crate::{
    ActivationArm, ActivationScope, CanonicalCellId, Interpreter, InterpreterExecution, MResult,
    MechError, PatternActivationArmRegistration, PatternActivationCaptureRegistration,
    PatternActivationGuardRegistration, PatternActivationRegistration, ValueCell,
    match_compiled_pattern_with_values,
};

use super::{
    ActivationPatternArmsNonExhaustive, Finalize, MatchGate, Matcher, ReactiveBindingSink,
    ScopePulse, Select, UnmatchedFinalize,
    arms::PreflightPatternedActivation,
    captures::{bool_state, read_selected_arm, register_node, selected_arm_state},
    commit_proposed_captures, elaborate_patterned_arm_guard, generation,
    registers::{Gate, elaborate_patterned_arm_body},
    validation::preflight_patterned_activation,
};

pub(crate) fn activation_scope_entry_cells(interpreter: &Interpreter) -> Vec<CanonicalCellId> {
    let symbols = interpreter.symbols();
    let symbols = symbols.borrow();
    let mut cells = Vec::new();
    for symbol in symbols.symbols.values() {
        let cell = symbol.reactive_cell_id();
        if !cells.contains(&cell) {
            cells.push(cell);
        }
    }
    cells
}

fn elaborate_patterned_activation_inner(
    arms: &[ActivationArm],
    trigger: ValueCell,
    preflight: PreflightPatternedActivation,
    i: &InterpreterExecution<'_>,
) -> MResult<ValueCell> {
    let compiled = preflight.arms;
    let plan = i.plan();
    let _persistent_user_function_plan = crate::function::PersistentUserFunctionPlanScope::enter(i);
    let pattern_expression_values = compiled
        .iter()
        .map(|arm| {
            arm.pattern
                .expressions()
                .iter()
                .map(|expression| crate::expression_cell(expression, None, i))
                .collect::<MResult<Vec<_>>>()
        })
        .collect::<MResult<Vec<_>>>()?;
    drop(_persistent_user_function_plan);
    // Seed proposal storage before guard graphs are elaborated. Composite
    // guard expressions may need the current proposal shape to compile, but
    // eligibility and selection are still determined by the runtime graph
    // initialization below.
    for (arm, expression_values) in compiled.iter().zip(&pattern_expression_values) {
        let pattern_match =
            match_compiled_pattern_with_values(&arm.pattern, &trigger, expression_values)?;
        ReactiveBindingSink {
            captures: &arm.captures,
        }
        .commit(&pattern_match)?;
    }
    let (scope_gen, scope_v) = generation();
    let scope_node = register_node(
        &plan,
        Box::new(ScopePulse {
            out: scope_gen.clone(),
        }),
        scope_gen,
        vec![trigger.clone()],
    )?;
    let (mut matcher_nodes, mut completions, mut matched) = (Vec::new(), Vec::new(), Vec::new());
    for (arm, expression_values) in compiled.iter().zip(&pattern_expression_values) {
        let (o, v) = generation();
        let f = bool_state(false);
        let mut inputs = Vec::with_capacity(2 + expression_values.len());
        inputs.push(scope_v.clone());
        inputs.push(trigger.clone());
        inputs.extend(expression_values.iter().cloned());
        let n = register_node(
            &plan,
            Box::new(Matcher {
                pattern: arm.pattern.clone(),
                trigger: trigger.clone(),
                expression_values: expression_values.clone(),
                captures: arm.captures.clone(),
                matched: f.clone(),
                out: o.clone(),
            }),
            o,
            inputs,
        )?;
        matcher_nodes.push(n);
        completions.push(v);
        matched.push(f);
    }
    let (mut finalizers, mut guards, mut eligible, mut done) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for n in 0..arms.len() {
        let e = bool_state(false);
        if let Some(guard) = &arms[n].guard {
            let (match_gate_out, match_gate_pulse) = generation();
            let match_gate_node = register_node(
                &plan,
                Box::new(MatchGate {
                    matched: matched[n].clone(),
                    out: match_gate_out.clone(),
                }),
                match_gate_out,
                vec![completions[n].clone()],
            )?;
            let (unmatched_out, unmatched_done) = generation();
            let unmatched_finalizer = register_node(
                &plan,
                Box::new(UnmatchedFinalize {
                    matched: matched[n].clone(),
                    eligible: e.clone(),
                    out: unmatched_out.clone(),
                }),
                unmatched_out,
                vec![completions[n].clone()],
            )?;
            let (guard_out, guard_done) = generation();
            let elaborated = elaborate_patterned_arm_guard(
                guard,
                &compiled[n].captures,
                &match_gate_pulse,
                &e,
                guard_out,
                i,
            )?;
            finalizers.push(unmatched_finalizer);
            guards.push(Some(PatternActivationGuardRegistration {
                match_gate_node,
                guard_finalizer_node: elaborated.finalizer_node,
                guard_node_start: elaborated.node_start,
                guard_node_end: elaborated.node_end,
            }));
            done.push(unmatched_done);
            done.push(guard_done);
        } else {
            let (out, arm_done) = generation();
            finalizers.push(register_node(
                &plan,
                Box::new(Finalize {
                    matched: matched[n].clone(),
                    eligible: e.clone(),
                    out: out.clone(),
                }),
                out,
                vec![completions[n].clone()],
            )?);
            guards.push(None);
            done.push(arm_done);
        }
        eligible.push(e);
    }
    let (o, selection) = generation();
    let selected = selected_arm_state(usize::MAX);
    let selector = register_node(
        &plan,
        Box::new(Select {
            eligible: eligible.clone(),
            selected: selected.clone(),
            out: o.clone(),
        }),
        o,
        done.clone(),
    )?;
    let private_scope_cell = scope_v.reactive_cell_id();
    plan.solve_dirty_cells(&[private_scope_cell])?;
    let initially_selected = read_selected_arm(&selected)?;
    if initially_selected >= compiled.len() {
        return Err(MechError::new(ActivationPatternArmsNonExhaustive, None));
    }
    commit_proposed_captures(&compiled[initially_selected].captures)?;
    let (mut gates, mut pulses) = (Vec::new(), Vec::new());
    for arm in 0..arms.len() {
        let (o, v) = generation();
        gates.push(register_node(
            &plan,
            Box::new(Gate {
                arm,
                selected: selected.clone(),
                captures: compiled[arm].captures.clone(),
                out: o.clone(),
            }),
            o,
            vec![selection.clone()],
        )?);
        pulses.push(v);
    }
    let mut ranges = Vec::new();
    for (arm, compiled_arm) in arms.iter().zip(&compiled) {
        ranges.push(elaborate_patterned_arm_body(
            arm,
            &compiled_arm.captures,
            &pulses[ranges.len()],
            i,
        )?);
    }
    let registration = PatternActivationRegistration {
        scope_pulse_node: scope_node,
        selector_node: selector,
        arms: (0..arms.len())
            .map(|n| PatternActivationArmRegistration {
                matcher_node: matcher_nodes[n],
                finalizer_node: finalizers[n],
                guard: guards[n].clone(),
                gate_node: gates[n],
                pulse_cell: pulses[n].reactive_cell_id(),
                body_node_start: ranges[n].0,
                body_node_end: ranges[n].1,
                captures: compiled[n]
                    .captures
                    .iter()
                    .map(|c| PatternActivationCaptureRegistration {
                        id: c.id,
                        schema: c.schema.clone(),
                        cell: c.committed.reactive_cell_id(),
                    })
                    .collect(),
            })
            .collect(),
    };
    plan.borrow_mut().register_pattern_activation(registration);
    Ok(ValueCell::unit())
}

pub(crate) fn elaborate_patterned_activation(
    scope: &ActivationScope,
    arms: &[ActivationArm],
    trigger: ValueCell,
    trigger_cells: Vec<CanonicalCellId>,
    interpreter: &InterpreterExecution<'_>,
) -> MResult<ValueCell> {
    let preflight =
        preflight_patterned_activation(scope, arms, &trigger, &trigger_cells, interpreter)?;
    let plan = interpreter.plan();
    let checkpoint = plan.checkpoint();
    let program_dictionary = interpreter.state.borrow().dictionary.clone();
    let dictionary_snapshot = program_dictionary.borrow().clone();
    match elaborate_patterned_activation_inner(arms, trigger, preflight, interpreter) {
        Ok(value) => Ok(value),
        Err(error) => {
            *program_dictionary.borrow_mut() = dictionary_snapshot;
            match plan.rollback(checkpoint) {
                Ok(()) => Err(error),
                Err(rollback_error) => Err(rollback_error),
            }
        }
    }
}
