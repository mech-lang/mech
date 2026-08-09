use super::support::{
    LegacyValue, ReactiveDependencyKind, Ref, arm_pulse_generation, arm_register_nodes,
    committed_capture_value, f64_symbol, interpret, plan_snapshot, registration, root_cell,
    selected_arm_index, set_f64_symbol,
};

#[test]
fn activation_selected_arm_commits_register_batch_and_other_arms_schedule_nothing() {
    let mut interpreter = interpret(
        r#"
event := 0.0
~x := 1.0
~y := 10.0

~> event
  | first, first > 10.0 => {
      x = first
      y += first
    }
  | second, second > 0.0 => {
      x = second * 2.0
      y = second + 1.0
    }
  | * => {
      x = -1.0
      y = -1.0
    }
"#,
    );
    let trigger = root_cell(&interpreter, "event");
    let activation = registration(&interpreter);
    let topology = plan_snapshot(&interpreter);
    let arm_registers = (0..activation.arms.len())
        .map(|arm| arm_register_nodes(&interpreter, &activation, arm))
        .collect::<Vec<_>>();
    assert!(arm_registers.iter().all(|registers| registers.len() == 2));
    {
        let plan = interpreter.plan();
        let plan = plan.borrow();
        for (arm_index, registers) in arm_registers.iter().enumerate() {
            for register in registers {
                let node = plan.node(*register).unwrap();
                assert!(node.inputs.iter().any(|dependency| {
                    dependency.cell == activation.arms[arm_index].pulse_cell
                        && dependency.kind == ReactiveDependencyKind::Reactive
                }));
                for (other_arm, registration) in activation.arms.iter().enumerate() {
                    if other_arm != arm_index {
                        assert!(!node.inputs.iter().any(|dependency| {
                            dependency.cell == registration.pulse_cell
                                && dependency.kind == ReactiveDependencyKind::Reactive
                        }));
                    }
                }
                for capture in &activation.arms[arm_index].captures {
                    assert!(node.inputs.iter().any(|dependency| {
                        dependency.cell == capture.cell
                            && dependency.kind == ReactiveDependencyKind::Sampled
                    }));
                }
            }
        }
    }

    // Initial dispatch selects the fallback only to seed captures. No arm
    // body is pulsed while the static graph is being registered.
    assert_eq!(
        (f64_symbol(&interpreter, "x"), f64_symbol(&interpreter, "y")),
        (1.0, 10.0)
    );
    assert!(!interpreter.has_pending_reactive_registers());
    for arm in 0..activation.arms.len() {
        assert_eq!(arm_pulse_generation(&interpreter, arm), 0);
    }

    // Both guarded arms are eligible, so source order selects arm zero.
    // The matching-but-losing arm must not schedule either register.
    set_f64_symbol(&interpreter, "event", 20.0);
    let first = interpreter.advance_reactive_turn(&[trigger]).unwrap();
    assert_eq!(selected_arm_index(&activation, &first), 0);
    assert_eq!(first.before_commit.pending_register_nodes, arm_registers[0]);
    assert_eq!(first.register_commit.staged_nodes, arm_registers[0]);
    assert_eq!(first.register_commit.committed_nodes, arm_registers[0]);
    assert_eq!(
        (f64_symbol(&interpreter, "x"), f64_symbol(&interpreter, "y")),
        (20.0, 30.0)
    );
    for register in arm_registers[1].iter().chain(&arm_registers[2]) {
        assert!(
            !first
                .before_commit
                .pending_register_nodes
                .contains(register)
        );
        assert!(!first.register_commit.staged_nodes.contains(register));
        assert!(!first.register_commit.committed_nodes.contains(register));
        assert!(!first.after_commit.pending_register_nodes.contains(register));
    }

    // Arm zero still matches, but its false guard must leave its register
    // nodes dormant. Arm one's capture is consumed by both writes from
    // this same trigger.
    set_f64_symbol(&interpreter, "event", 5.0);
    let second = interpreter.advance_reactive_turn(&[trigger]).unwrap();
    assert_eq!(selected_arm_index(&activation, &second), 1);
    assert_eq!(
        second.before_commit.pending_register_nodes,
        arm_registers[1]
    );
    assert_eq!(second.register_commit.staged_nodes, arm_registers[1]);
    assert_eq!(second.register_commit.committed_nodes, arm_registers[1]);
    assert_eq!(
        (f64_symbol(&interpreter, "x"), f64_symbol(&interpreter, "y")),
        (10.0, 6.0)
    );
    assert_eq!(
        committed_capture_value(&interpreter, 1, 0),
        LegacyValue::F64(Ref::new(5.0))
    );
    for register in arm_registers[0].iter().chain(&arm_registers[2]) {
        assert!(
            !second
                .before_commit
                .pending_register_nodes
                .contains(register)
        );
        assert!(!second.register_commit.staged_nodes.contains(register));
        assert!(!second.register_commit.committed_nodes.contains(register));
        assert!(
            !second
                .after_commit
                .pending_register_nodes
                .contains(register)
        );
    }
    assert!(!interpreter.has_pending_reactive_registers());
    assert_eq!(plan_snapshot(&interpreter), topology);
}

#[test]
fn activation_equal_trigger_packets_produce_distinct_register_transitions_without_plan_growth() {
    let mut interpreter = interpret(
        r#"
event := 2.0
~count := 0.0

~> event
  | amount => {
      count += amount
    }
  | * => {
      fallback := 0.0
    }
"#,
    );
    let trigger = root_cell(&interpreter, "event");
    let activation = registration(&interpreter);
    let registers = arm_register_nodes(&interpreter, &activation, 0);
    let topology = plan_snapshot(&interpreter);
    assert_eq!(registers.len(), 1);
    assert_eq!(f64_symbol(&interpreter, "count"), 0.0);

    for (expected_count, expected_pulse) in [(2.0, 1usize), (4.0, 2usize)] {
        let outcome = interpreter.advance_reactive_turn(&[trigger]).unwrap();
        assert_eq!(selected_arm_index(&activation, &outcome), 0);
        assert_eq!(outcome.before_commit.pending_register_nodes, registers);
        assert_eq!(outcome.register_commit.staged_nodes, registers);
        assert_eq!(outcome.register_commit.committed_nodes, registers);
        assert_eq!(f64_symbol(&interpreter, "count"), expected_count);
        assert_eq!(arm_pulse_generation(&interpreter, 0), expected_pulse);
        assert_eq!(plan_snapshot(&interpreter), topology);
    }
}
