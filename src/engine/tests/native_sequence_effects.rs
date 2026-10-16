//! Scheduling must preserve observable IO and runtime failure order.
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

#[test]
fn byte_encoding_errors_stay_between_preceding_and_following_writes() {
    for (input, alphabet, mode) in [
        ("A", "", "exact"),
        ("A", "A", "unknown"),
        ("A", "AA", "exact"),
        ("A", "Aa", "ignore-ascii-case"),
        ("B", "A", "exact"),
    ] {
        let mut program = graph(&[
            (8, vec![3], SequenceKind::Unit),
            (3, vec![0, 1, 2], SequenceKind::Bytes),
            (8, vec![4], SequenceKind::Unit),
        ]);
        program.constants[0] = Constant::Text(input.into());
        program.constants[1] = Constant::Text(alphabet.into());
        program.constants[2] = Constant::Text(mode.into());
        let scheduled = program.scheduled().unwrap();
        assert_eq!(
            scheduled
                .nodes
                .iter()
                .map(|node| node.op)
                .collect::<Vec<_>>(),
            [8, 3, 8],
            "{input:?}, {alphabet:?}, {mode:?}"
        );
    }
}

#[test]
fn byte_encoding_and_regex_failures_keep_their_source_order() {
    for (first, second) in [(3, 53), (53, 3), (3, 54), (54, 3)] {
        let operation = |op| match op {
            3 => (3, vec![0, 1, 2], SequenceKind::Bytes),
            53 => (53, vec![0, 1], SequenceKind::Integer),
            54 => (54, vec![0, 1, 2], SequenceKind::Text),
            _ => unreachable!(),
        };
        let scheduled = graph(&[
            (8, vec![3], SequenceKind::Unit),
            operation(first),
            operation(second),
            (8, vec![4], SequenceKind::Unit),
        ])
        .scheduled()
        .unwrap();
        assert_eq!(
            scheduled
                .nodes
                .iter()
                .map(|node| node.op)
                .collect::<Vec<_>>(),
            [8, first, second, 8]
        );
    }
}

#[test]
fn an_earlier_formatting_failure_precedes_a_higher_priority_regex() {
    let mut program = graph(&[
        (8, vec![3], SequenceKind::Unit),
        (24, vec![0, 1], SequenceKind::Text),
        (53, vec![2, 4], SequenceKind::Integer),
        (8, vec![3], SequenceKind::Unit),
    ]);
    program.constants[0] = Constant::Integer(1);
    program.constants[1] = Constant::Integer(13);
    program.constants[4] = Constant::Text("[".into());
    program.kinds[0] = SequenceKind::Integer;
    program.kinds[1] = SequenceKind::Integer;
    let scheduled = program.scheduled().unwrap();
    assert_eq!(
        scheduled
            .nodes
            .iter()
            .map(|node| node.op)
            .collect::<Vec<_>>(),
        [8, 24, 53, 8]
    );
}
