use super::support::{
    ActivationPatternArmsNonExhaustive, ActivationPatternWildcardMustBeLast, Ref, Value,
    arm_register_nodes, assert_dispatch_turn, committed_capture_value, f64_symbol, interpret,
    interpret_more, plan_snapshot, registration, root_cell, selected_arm_index,
    set_f64_matrix_event, set_f64_symbol, set_tuple_event,
};

#[test]
fn activation_final_binding_is_exhaustive_and_guarded_binding_falls_through_to_it() {
    let mut interpreter = interpret(
        r#"
physics-tick := 0.25
~position-y := -1.0
~velocity-y := 0.0
floor := 0.0
restitution := 0.5
gravity := 4.0

~> physics-tick
  | dt, position-y >= floor => {
      velocity-y = -velocity-y * restitution
      position-y = floor
    }
  | dt => {
      velocity-y = velocity-y + gravity * dt
      position-y = position-y + velocity-y * dt
    }
"#,
    );
    let trigger = root_cell(&interpreter, "physics-tick");
    let activation = registration(&interpreter);
    let topology = plan_snapshot(&interpreter);
    assert_eq!(activation.arms.len(), 2);
    assert!(activation.arms[0].guard.is_some());
    assert!(activation.arms[1].guard.is_none());
    assert_eq!(activation.arms[0].captures.len(), 1);
    assert_eq!(activation.arms[1].captures.len(), 1);
    let arm_registers = (0..activation.arms.len())
        .map(|arm| arm_register_nodes(&interpreter, &activation, arm))
        .collect::<Vec<_>>();
    assert!(arm_registers.iter().all(|registers| registers.len() == 2));
    assert_eq!(
        (
            f64_symbol(&interpreter, "position-y"),
            f64_symbol(&interpreter, "velocity-y"),
        ),
        (-1.0, 0.0),
    );

    let fallback = interpreter.advance_reactive_turn(&[trigger]).unwrap();
    assert_eq!(selected_arm_index(&activation, &fallback), 1);
    assert_eq!(fallback.register_commit.committed_nodes, arm_registers[1]);
    assert_eq!(
        (
            f64_symbol(&interpreter, "position-y"),
            f64_symbol(&interpreter, "velocity-y"),
        ),
        (-1.0, 1.0),
    );
    assert_eq!(
        committed_capture_value(&interpreter, 1, 0),
        Value::F64(Ref::new(0.25)),
    );
    assert_eq!(plan_snapshot(&interpreter), topology);

    set_f64_symbol(&interpreter, "position-y", 1.0);
    let guarded = interpreter.advance_reactive_turn(&[trigger]).unwrap();
    assert_eq!(selected_arm_index(&activation, &guarded), 0);
    assert_eq!(guarded.register_commit.committed_nodes, arm_registers[0]);
    assert_eq!(
        (
            f64_symbol(&interpreter, "position-y"),
            f64_symbol(&interpreter, "velocity-y"),
        ),
        (0.0, -0.5),
    );
    assert_eq!(
        committed_capture_value(&interpreter, 0, 0),
        Value::F64(Ref::new(0.25)),
    );
    assert_eq!(plan_snapshot(&interpreter), topology);
}

#[test]
fn activation_guarded_wildcard_may_precede_an_exhaustive_binding() {
    let mut interpreter = interpret(
        r#"
event := 2.0
ready := false

~> event
  | *, ready == true => {
      selected := -1.0
    }
  | value => {
      selected := value
    }
"#,
    );
    let trigger = root_cell(&interpreter, "event");
    let topology = plan_snapshot(&interpreter);
    let outcome = interpreter.advance_reactive_turn(&[trigger]).unwrap();
    assert_dispatch_turn(&interpreter, &topology, &outcome, 1, 2.0);
}

#[test]
fn activation_final_irrefutable_tuple_is_exhaustive() {
    let mut interpreter = interpret(
        r#"
event := (1.0, 2.0)
~> event
  | (tick, dt) => {
      selected := tick * 10.0 + dt
    }
"#,
    );
    let trigger = root_cell(&interpreter, "event");
    let topology = plan_snapshot(&interpreter);
    set_tuple_event(
        &interpreter,
        vec![Value::F64(Ref::new(3.0)), Value::F64(Ref::new(4.0))],
    );
    let outcome = interpreter.advance_reactive_turn(&[trigger]).unwrap();
    assert_dispatch_turn(&interpreter, &topology, &outcome, 0, 34.0);
}

#[cfg(all(feature = "matrix", feature = "f64"))]
#[test]
fn activation_final_irrefutable_fixed_matrix_pattern_is_exhaustive() {
    let mut interpreter = interpret(
        r#"
event := [1.0 2.0]
~> event
  | [left, right] => {
      selected := left * 10.0 + right
    }
"#,
    );
    let trigger = root_cell(&interpreter, "event");
    let topology = plan_snapshot(&interpreter);
    set_f64_matrix_event(&interpreter, vec![3.0, 4.0]);
    let outcome = interpreter.advance_reactive_turn(&[trigger]).unwrap();
    assert_dispatch_turn(&interpreter, &topology, &outcome, 0, 34.0);
}

#[test]
fn activation_final_refutable_pattern_is_non_exhaustive() {
    for pattern in ["1.0", "(x, x)"] {
        let mut interpreter = if pattern == "1.0" {
            interpret("event := 1.0")
        } else {
            interpret("event := (1.0, 1.0)")
        };
        let topology = plan_snapshot(&interpreter);
        let error = interpret_more(
            &mut interpreter,
            &format!("~> event\n  | {pattern} => {{\n      selected := 1.0\n    }}"),
        )
        .unwrap_err();
        assert_eq!(error.kind_name(), "ActivationPatternArmsNonExhaustive");
        assert_eq!(plan_snapshot(&interpreter), topology);
    }

    let mut interpreter = interpret("event := 1.0");
    let topology = plan_snapshot(&interpreter);
    let error = interpret_more(
        &mut interpreter,
        r#"
~> event
  | value, value > 0.0 => {
      selected := value
    }
"#,
    )
    .unwrap_err();
    assert_eq!(error.kind_name(), "ActivationPatternArmsNonExhaustive");
    assert_eq!(plan_snapshot(&interpreter), topology);
}

#[test]
fn activation_unguarded_wildcard_before_the_final_arm_is_rejected() {
    let mut interpreter = interpret("event := 1.0");
    let topology = plan_snapshot(&interpreter);
    let error = interpret_more(
        &mut interpreter,
        r#"
~> event
  | * => {
      selected := 1.0
    }
  | value => {
      selected := value
    }
"#,
    )
    .unwrap_err();
    assert_eq!(error.kind_name(), "ActivationPatternWildcardMustBeLast");
    assert_eq!(plan_snapshot(&interpreter), topology);
}
