use super::*;
use std::collections::{BTreeMap, BTreeSet};

fn candidate() -> Candidate {
    Candidate {
        tag: 100,
        active: 0,
        terminal: 1,
        induction: 101,
        count: 9,
        next_tag: 104,
        next_induction: 105,
    }
}
fn graph(nodes: Vec<Node>) -> Graph {
    Graph {
        nodes,
        advances: vec![BTreeMap::new()],
        written: BTreeSet::from([100, 101, 102, 103, 150]),
        terminal_inputs: Some(BTreeSet::new()),
    }
}

#[test]
fn ordered_reduction_keeps_reverse_length_bound_to_its_current_array() {
    let c = candidate();
    let mut g = graph(vec![
        Node::Initial(101),
        Node::Constant(3),
        Node::Scalar(Op::U64WrappingAdd, vec![0, 1]),
        Node::Length(0),
        Node::Scalar(Op::U64WrappingSub, vec![3, 2]),
        Node::Constant(1),
        Node::Scalar(Op::U64WrappingAdd, vec![4, 5]),
        Node::Read { array: 1, index: 6 },
    ]);
    let windows = [Window {
        induction: 101,
        count: 9,
        array: 1,
        delta: 3,
        reverse: true,
    }];
    assert!(
        Normalize {
            graph: &g,
            candidate: &c,
            windows: &windows,
            fuel: 128
        }
        .cost(7)
        .is_none()
    );
    g.nodes[3] = Node::Length(1);
    assert_eq!(
        Normalize {
            graph: &g,
            candidate: &c,
            windows: &windows,
            fuel: 128
        }
        .cost(7),
        Some(Cost::Read(windows[0]))
    );
}

#[test]
fn ordered_reduction_refuses_overwritten_captures_and_recursive_expressions() {
    let c = candidate();
    let g = graph(vec![
        Node::Initial(150),
        Node::Initial(500),
        Node::Scalar(Op::U64WrappingAdd, vec![2, 1]),
    ]);
    assert!(
        Normalize {
            graph: &g,
            candidate: &c,
            windows: &[],
            fuel: 128
        }
        .cost(0)
        .is_none()
    );
    assert_eq!(
        Normalize {
            graph: &g,
            candidate: &c,
            windows: &[],
            fuel: 128
        }
        .cost(1),
        Some(Cost::Word(500))
    );
    assert!(
        Normalize {
            graph: &g,
            candidate: &c,
            windows: &[],
            fuel: 128
        }
        .cost(2)
        .is_none()
    );
    assert!(
        Normalize {
            graph: &g,
            candidate: &c,
            windows: &[],
            fuel: 128
        }
        .item(0)
        .is_none()
    );
}

#[test]
fn ordered_reduction_requires_every_terminal_dependency_to_survive_elision() {
    let mut g = graph(vec![
        Node::Initial(100),
        Node::Initial(101),
        Node::Initial(102),
        Node::Initial(103),
        Node::Initial(150),
        Node::Initial(500),
        Node::Scalar(Op::U64Eq, vec![0, 0]),
    ]);
    g.terminal_inputs = Some(BTreeSet::from([0, 1, 2, 3, 5, 6]));
    assert!(terminal_local(&g, 100, 4));
    g.terminal_inputs.as_mut().unwrap().insert(4);
    assert!(!terminal_local(&g, 100, 4));
    g.terminal_inputs = None;
    assert!(!terminal_local(&g, 100, 4));
}
