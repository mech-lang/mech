use super::support::{
    FLAT_TUPLE_ACTIVATION, MechTuple, NESTED_TUPLE_ACTIVATION, REPEATED_CAPTURE_ACTIVATION, Ref,
    Value, ValueKind, assert_dispatch_turn, hash_str, interpret, interpret_more,
    load_atom_tuple_activation, load_enum_activation, plan_snapshot, root_cell,
    set_atom_tuple_event, set_enum_event, set_f64_matrix_event, set_tuple_event,
    set_unit_enum_event, tuple_fixture, turn_executed_nodes,
};

#[test]
fn activation_pattern_selects_pressed_released_and_wildcard() {
    let (mut i, trigger, _, topology) = load_enum_activation();
    for (name, payload, arm, output) in [
        ("pressed", 10., 0, 10.),
        ("released", 20., 1, 1020.),
        ("other", 30., 2, -1.),
    ] {
        set_enum_event(&i, name, payload);
        let outcome = i.advance_reactive_turn(&[trigger]).unwrap();
        assert_dispatch_turn(&i, &topology, &outcome, arm, output);
    }
}

#[test]
fn activation_pattern_enum_arms_compile_independent_of_initial_variant() {
    let (mut i, trigger, r, topology) = load_enum_activation();
    assert_eq!(r.arms[1].captures[0].kind, ValueKind::F64);
    set_enum_event(&i, "released", 20.);
    let o = i.advance_reactive_turn(&[trigger]).unwrap();
    assert_dispatch_turn(&i, &topology, &o, 1, 1020.);
}

#[test]
fn activation_pattern_equal_packets_dispatch_repeatedly() {
    let (mut i, trigger, _, topology) = load_enum_activation();
    set_enum_event(&i, "pressed", 30.);
    for _ in 0..2 {
        let o = i.advance_reactive_turn(&[trigger]).unwrap();
        assert_dispatch_turn(&i, &topology, &o, 0, 30.);
    }
}

#[test]
fn activation_pattern_unselected_arm_nodes_do_not_execute() {
    let (mut i, trigger, r, topology) = load_enum_activation();
    set_enum_event(&i, "released", 20.);
    let o = i.advance_reactive_turn(&[trigger]).unwrap();
    assert_dispatch_turn(&i, &topology, &o, 1, 1020.);
    let executed = turn_executed_nodes(&o);
    for arm in [&r.arms[0], &r.arms[2]] {
        for node in arm.body_node_start..arm.body_node_end {
            assert!(!executed.contains(&node));
        }
    }
}

#[test]
fn activation_pattern_switching_arms_does_not_grow_plan() {
    let (mut i, trigger, _, topology) = load_enum_activation();
    for (name, payload) in [
        ("pressed", 10.),
        ("released", 20.),
        ("other", 30.),
        ("pressed", 30.),
        ("pressed", 30.),
    ] {
        set_enum_event(&i, name, payload);
        i.advance_reactive_turn(&[trigger]).unwrap();
        assert_eq!(plan_snapshot(&i), topology);
    }
}

#[test]
fn activation_pattern_matches_payload_free_enum_variant() {
    let mut i = interpret(
        r#"
<signal> := :ready | :other
event<signal> := :other
~> event
  | :ready => {
      selected := 1.0
    }
  | * => {
      selected := -1.0
    }
"#,
    );
    let trigger = root_cell(&i, "event");
    let topology = plan_snapshot(&i);
    set_unit_enum_event(&i, "ready");
    let outcome = i.advance_reactive_turn(&[trigger]).unwrap();
    assert_dispatch_turn(&i, &topology, &outcome, 0, 1.0);
}

#[test]
fn activation_pattern_atom_tagged_tuple_selects_arm() {
    let (mut i, trigger, _, topology) = load_atom_tuple_activation();
    for (tag, payload, arm, output) in [
        ("pressed", 10., 0, 10.),
        ("released", 20., 1, 1020.),
        ("other", 30., 2, -1.),
    ] {
        set_atom_tuple_event(&i, tag, payload);
        let o = i.advance_reactive_turn(&[trigger]).unwrap();
        assert_dispatch_turn(&i, &topology, &o, arm, output);
    }
}

#[test]
fn activation_pattern_atom_tuple_arms_compile_independent_of_initial_tag() {
    let (mut i, trigger, r, topology) = load_atom_tuple_activation();
    assert_eq!(r.arms[0].captures[0].kind, ValueKind::F64);
    assert_eq!(r.arms[1].captures[0].kind, ValueKind::F64);
    set_atom_tuple_event(&i, "released", 20.);
    let o = i.advance_reactive_turn(&[trigger]).unwrap();
    assert_dispatch_turn(&i, &topology, &o, 1, 1020.);
}

#[cfg(feature = "u64")]
#[test]
fn activation_typed_literal_pattern_uses_shared_value_matching() {
    let mut i = interpret(
        r#"
event := 1u64
~> event
  | 1u64 => {
      selected := 1.0
    }
  | * => {
      selected := -1.0
    }
"#,
    );
    let trigger = root_cell(&i, "event");
    let topology = plan_snapshot(&i);
    let outcome = i.advance_reactive_turn(&[trigger]).unwrap();
    assert_dispatch_turn(&i, &topology, &outcome, 0, 1.0);
}

#[test]
fn activation_pattern_atom_tagged_tuple_captures_payload() {
    let (mut i, trigger, r, topology) = load_atom_tuple_activation();
    assert_eq!(r.arms[0].captures[0].kind, ValueKind::F64);
    let cell = r.arms[0].captures[0].cell;
    assert!(
        i.plan().borrow().nodes[r.arms[0].body_node_start..r.arms[0].body_node_end]
            .iter()
            .any(|n| n.inputs.iter().any(|d| d.cell == cell))
    );
    set_atom_tuple_event(&i, "pressed", 10.);
    let o = i.advance_reactive_turn(&[trigger]).unwrap();
    assert_dispatch_turn(&i, &topology, &o, 0, 10.);
}

#[test]
fn activation_pattern_tuple_captures_elements() {
    let (mut i, trigger, topology) = tuple_fixture(FLAT_TUPLE_ACTIVATION);
    set_tuple_event(&i, vec![Value::F64(Ref::new(3.)), Value::F64(Ref::new(4.))]);
    let o = i.advance_reactive_turn(&[trigger]).unwrap();
    assert_dispatch_turn(&i, &topology, &o, 0, 34.);
}

#[test]
fn activation_pattern_nested_tuple_captures_elements() {
    let (mut i, trigger, topology) = tuple_fixture(NESTED_TUPLE_ACTIVATION);
    set_tuple_event(
        &i,
        vec![
            Value::Tuple(Ref::new(MechTuple::from_vec(vec![
                Value::F64(Ref::new(4.)),
                Value::F64(Ref::new(5.)),
            ]))),
            Value::F64(Ref::new(6.)),
        ],
    );
    let o = i.advance_reactive_turn(&[trigger]).unwrap();
    assert_dispatch_turn(&i, &topology, &o, 0, 456.);
}

#[test]
fn activation_pattern_repeated_capture_requires_equal_values() {
    let (mut i, trigger, topology) = tuple_fixture(REPEATED_CAPTURE_ACTIVATION);
    set_tuple_event(&i, vec![Value::F64(Ref::new(2.)), Value::F64(Ref::new(2.))]);
    let o = i.advance_reactive_turn(&[trigger]).unwrap();
    assert_dispatch_turn(&i, &topology, &o, 0, 2.);
    set_tuple_event(&i, vec![Value::F64(Ref::new(2.)), Value::F64(Ref::new(3.))]);
    let o = i.advance_reactive_turn(&[trigger]).unwrap();
    assert_dispatch_turn(&i, &topology, &o, 1, -1.);
}

#[cfg(all(feature = "matrix", feature = "f64"))]
#[test]
fn activation_array_pattern_supports_prefix_suffix_and_anonymous_spread() {
    let mut i = interpret(
        r#"
event := [1.0 2.0 3.0 1.0]
~> event
  | [x, ..., x] => {
      selected := x + 10.0
    }
  | * => {
      selected := -1.0
    }
"#,
    );
    let trigger = root_cell(&i, "event");
    let topology = plan_snapshot(&i);

    let outcome = i.advance_reactive_turn(&[trigger]).unwrap();
    assert_dispatch_turn(&i, &topology, &outcome, 0, 11.0);

    set_f64_matrix_event(&i, vec![1.0, 2.0, 3.0, 4.0]);
    let outcome = i.advance_reactive_turn(&[trigger]).unwrap();
    assert_dispatch_turn(&i, &topology, &outcome, 1, -1.0);
}

#[cfg(all(feature = "matrix", feature = "f64"))]
#[test]
fn activation_array_rest_segment_accepts_nested_array_pattern() {
    let mut i = interpret(
        r#"
event := [1.0 2.0 3.0 4.0]
~> event
  | [head | [second, ..., last]] => {
      selected := head * 100.0 + second * 10.0 + last
    }
  | * => {
      selected := -1.0
    }
"#,
    );
    let trigger = root_cell(&i, "event");
    let topology = plan_snapshot(&i);
    let outcome = i.advance_reactive_turn(&[trigger]).unwrap();
    assert_dispatch_turn(&i, &topology, &outcome, 0, 124.0);
}

#[test]
fn activation_pattern_repeated_capture_kind_mismatch_uses_canonical_error() {
    let mut i = interpret("event := (1.0, \"one\")");
    let topology = plan_snapshot(&i);
    let error = interpret_more(
        &mut i,
        "~> event\n  | (x, x) => {
      selected := x
    }\n  | * => {
      selected := 0.0
    }",
    )
    .unwrap_err();
    assert_eq!(error.kind_name(), "PatternCompileError");
    assert_eq!(plan_snapshot(&i), topology);
    assert!(!i.symbols().borrow().contains(hash_str("x")));
    assert!(!i.symbols().borrow().contains(hash_str("selected")));
}
