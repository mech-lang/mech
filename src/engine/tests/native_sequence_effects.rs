//! Scheduling must preserve observable IO and runtime regex error order.
#![cfg(feature = "native-sequence")]

use mech_engine::native_sequence::{Constant, Node, Program, SequenceKind};
use std::collections::BTreeMap;

fn graph(operations: &[(u64, Vec<usize>, SequenceKind)]) -> Program {
    // Duplicate and unsorted constants exercise the scheduler's value renaming
    // independently of the ordered effects. "[" is a runtime-invalid pattern.
    let constants = vec![
        Constant::Text("subject".into()),
        Constant::Text("[".into()),
        Constant::Text("replacement".into()),
        Constant::Text("before".into()),
        Constant::Text("before".into()),
    ];
    let mut kinds = vec![SequenceKind::Text; constants.len()];
    kinds.extend(operations.iter().map(|(_, _, kind)| *kind));
    Program {
        constants,
        nodes: operations
            .iter()
            .map(|(op, args, _)| Node {
                op: *op,
                args: args.clone(),
                kernel: None,
            })
            .collect(),
        kinds,
        windows: BTreeMap::new(),
        functions: BTreeMap::new(),
    }
}

#[test]
fn regex_errors_stay_between_preceding_and_following_writes() {
    let program = graph(&[
        (8, vec![3], SequenceKind::Unit),
        (53, vec![0, 1], SequenceKind::Integer),
        (8, vec![4], SequenceKind::Unit),
        (54, vec![0, 1, 2], SequenceKind::Text),
        (8, vec![3], SequenceKind::Unit),
    ]);
    let scheduled = program.scheduled().unwrap();
    assert_eq!(
        scheduled
            .nodes
            .iter()
            .map(|node| node.op)
            .collect::<Vec<_>>(),
        [8, 53, 8, 54, 8]
    );
}

#[test]
fn higher_priority_count_does_not_precede_an_earlier_fallible_replace() {
    let program = graph(&[
        (54, vec![0, 1, 2], SequenceKind::Text),
        (53, vec![0, 1], SequenceKind::Integer),
        (8, vec![3], SequenceKind::Unit),
    ]);
    let scheduled = program.scheduled().unwrap();
    assert_eq!(
        scheduled
            .nodes
            .iter()
            .map(|node| node.op)
            .collect::<Vec<_>>(),
        [54, 53, 8]
    );
}

#[test]
fn ordered_regex_dependencies_survive_constant_and_node_renumbering() {
    // Replacement input is computed by concat; its result feeds a later write.
    // Neither the concat dependency nor the error may move the first write.
    let program = graph(&[
        (8, vec![4], SequenceKind::Unit),
        (27, vec![0, 0], SequenceKind::Text),
        (54, vec![6, 1, 2], SequenceKind::Text),
        (8, vec![7], SequenceKind::Unit),
    ]);
    let scheduled = program.scheduled().unwrap();
    assert_eq!(scheduled.constants.len(), 4);
    assert_eq!(
        scheduled
            .nodes
            .iter()
            .map(|node| node.op)
            .collect::<Vec<_>>(),
        [8, 27, 54, 8]
    );
    let base = scheduled.constants.len();
    assert_eq!(scheduled.nodes[2].args[0], base + 1);
    assert_eq!(scheduled.nodes[3].args, [base + 2]);
    assert_eq!(
        scheduled.constants[scheduled.nodes[0].args[0]],
        Constant::Text("before".into())
    );
    assert_eq!(
        scheduled.constants[scheduled.nodes[2].args[1]],
        Constant::Text("[".into())
    );
}
