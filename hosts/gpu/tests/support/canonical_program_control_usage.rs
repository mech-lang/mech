//! Read-only measurements of the locally checked compressor source. These
//! observations grant no artifact or execution authority; finalization follows.
use mech_engine::{
    CollectionPattern, ComprehensionDeclaration, ComprehensionStep, ControlOperationBody,
    IterationDeclaration, MatchDeclaration, MatchPattern, SourceNodeBody, SourceProgram,
};

enum Control<'a, C> {
    Match(&'a MatchDeclaration<C>),
    Collection(&'a ComprehensionDeclaration<C>),
    Iteration(&'a IterationDeclaration<C>),
}

fn nested<'a, C>(
    body: &'a ControlOperationBody<C>,
    depth: usize,
    pending: &mut Vec<(Control<'a, C>, usize)>,
) {
    let control = match body {
        ControlOperationBody::Match(value) => Control::Match(value),
        ControlOperationBody::Comprehension(value) => Control::Collection(value),
        ControlOperationBody::Iteration(value) => Control::Iteration(value),
        ControlOperationBody::Operation { .. }
        | ControlOperationBody::Recur(_)
        | ControlOperationBody::Suspend
        | ControlOperationBody::Assert
        | ControlOperationBody::Publish => return,
    };
    pending.push((control, depth));
}

fn pattern<S, V>(root: &CollectionPattern<S, V>) -> (usize, usize) {
    let mut pending = vec![root];
    let (mut nodes, mut bindings) = (0usize, 0usize);
    while let Some(value) = pending.pop() {
        nodes += 1;
        match value {
            CollectionPattern::Bind { .. } => bindings += 1,
            CollectionPattern::Enum { payload, .. } => pending.extend(payload.as_deref()),
            CollectionPattern::Tuple(items) => pending.extend(items.iter()),
            CollectionPattern::Array {
                prefix,
                rest,
                suffix,
            } => {
                pending.extend(prefix.iter());
                pending.extend(rest.as_deref());
                pending.extend(suffix.iter());
            }
            CollectionPattern::Wildcard | CollectionPattern::Equal(_) => {}
        }
    }
    (nodes, bindings)
}

fn usage<C>(root: Control<'_, C>) -> ([usize; 5], usize) {
    let mut counts = [0usize; 5];
    let mut maximum_depth = 0;
    let mut pending = vec![(root, 1)];
    while let Some((control, depth)) = pending.pop() {
        maximum_depth = maximum_depth.max(depth);
        match control {
            Control::Match(value) => {
                counts[0] += value.arms.len();
                counts[3] += value.captures.len();
                for arm in &value.arms {
                    if let MatchPattern::Structural(value) = &arm.pattern {
                        let (nodes, bindings) = pattern(value);
                        counts[3] += nodes;
                        counts[4] += bindings;
                    }
                    for block in arm.guard.iter().chain(core::iter::once(&arm.body)) {
                        counts[1] += 1;
                        counts[2] += block.operations.len();
                        counts[3] += block.parameters.len();
                        counts[4] += block.operations.len();
                        for operation in &block.operations {
                            counts[3] += operation.inputs.len();
                            nested(&operation.body, depth + 1, &mut pending);
                        }
                    }
                }
            }
            Control::Collection(value) => {
                counts[1] += 1;
                counts[2] += value.steps.len();
                for step in &value.steps {
                    match step {
                        ComprehensionStep::Generator { pattern: value, .. } => {
                            let (nodes, bindings) = pattern(value);
                            counts[3] += nodes;
                            counts[4] += bindings;
                        }
                        ComprehensionStep::Filter(_) => counts[3] += 1,
                        ComprehensionStep::Operation(operation) => {
                            counts[3] += operation.inputs.len();
                            counts[4] += 1;
                            nested(&operation.body, depth + 1, &mut pending);
                        }
                    }
                }
            }
            Control::Iteration(value) => {
                counts[1] += 1;
                counts[2] += value.body.operations.len();
                counts[3] += value.captures.len() + value.body.parameters.len();
                counts[4] += 1 + value.body.operations.len();
                for operation in &value.body.operations {
                    counts[3] += operation.inputs.len();
                    nested(&operation.body, depth + 1, &mut pending);
                }
            }
        }
    }
    (counts, maximum_depth)
}

pub(super) fn report(program: &SourceProgram) {
    let mut totals = [0usize; 4];
    for (node, value) in program.nodes.iter().enumerate() {
        let control = match &value.body {
            SourceNodeBody::Match(value) | SourceNodeBody::Activation(value) => {
                Control::Match(value)
            }
            SourceNodeBody::Comprehension(value) => Control::Collection(value),
            SourceNodeBody::Iteration(value) => Control::Iteration(value),
            SourceNodeBody::Operation { .. } => continue,
            SourceNodeBody::Fsm(_) => {
                panic!("compressor qualification does not use FSM declarations")
            }
        };
        let (counts, depth) = usage(control);
        for (total, count) in totals.iter_mut().zip(counts) {
            *total += count;
        }
        println!(
            "{}",
            serde_json::json!({"witness":"source-control-usage", "source_node":node,
            "arms":counts[0],"blocks":counts[1],"operations":counts[2],"operands":counts[3],"local_slots":counts[4],"depth":depth})
        );
    }
    println!(
        "{}",
        serde_json::json!({"witness":"source-control-totals","arms":totals[0],"blocks":totals[1],"operations":totals[2],"operands":totals[3]})
    );
}
