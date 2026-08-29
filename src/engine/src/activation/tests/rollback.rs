use super::support::{
    Arc, AtomicUsize, FailingPatternRegisterSpecializer, LegacyValue, Ordering, Ref,
    arm_pulse_generation, arm_register_nodes, assert_dispatch_turn,
    assert_failed_elaboration_restored, body_output_f64, committed_capture_value, f64_symbol,
    hash_str, install_function_extension, interpret, interpret_more, plan_snapshot,
    proposed_capture_value, registration, root_cell, set_atom_tuple_event, set_f64_symbol, symbol,
    turn_changed_nodes, turn_executed_nodes, turn_unchanged_nodes,
};

#[test]
fn activation_failed_multi_register_staging_mutates_nothing() {
    let mut interpreter = interpret(
        r#"
event := 0.0
~first := 1.0
~second := 2.0
"#,
    );
    let solve_calls = Arc::new(AtomicUsize::new(0));
    let stage_calls = Arc::new(AtomicUsize::new(0));
    install_function_extension(
        &interpreter,
        "test/failing-pattern-register",
        Arc::new(FailingPatternRegisterSpecializer {
            solve_calls: solve_calls.clone(),
            stage_calls: stage_calls.clone(),
        }),
    );
    interpret_more(
        &mut interpreter,
        r#"
~> event
  | value => {
      first = value
      test/failing-pattern-register(second)
    }
  | * => {
      fallback := 0.0
    }
"#,
    )
    .unwrap();

    let trigger = root_cell(&interpreter, "event");
    let activation = registration(&interpreter);
    let registers = arm_register_nodes(&interpreter, &activation, 0);
    let topology = plan_snapshot(&interpreter);
    assert_eq!(registers.len(), 2);
    assert_eq!(
        (
            f64_symbol(&interpreter, "first"),
            f64_symbol(&interpreter, "second")
        ),
        (1.0, 2.0)
    );
    assert_eq!(solve_calls.load(Ordering::SeqCst), 0);

    set_f64_symbol(&interpreter, "event", 9.0);
    let error = interpreter.advance_reactive_turn(&[trigger]).unwrap_err();
    assert_eq!(error.kind_name(), "PatternRegisterStageFailure");
    assert_eq!(stage_calls.load(Ordering::SeqCst), 1);
    assert_eq!(solve_calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        (
            f64_symbol(&interpreter, "first"),
            f64_symbol(&interpreter, "second")
        ),
        (1.0, 2.0)
    );
    assert_eq!(plan_snapshot(&interpreter), topology);
}

#[test]
fn activation_guard_non_boolean_result_rolls_back_plan_and_symbols() {
    let mut i = interpret("event := 1.0\nouter := 9.0");
    let symbols = i.symbols().borrow().snapshot();
    let dictionary = i.dictionary().borrow().clone();
    let topology = plan_snapshot(&i);

    let error = interpret_more(
        &mut i,
        r#"
~> event
  | x, x + 1.0 => {
      selected := x
    }
  | * => {
      selected := -1.0
    }
"#,
    )
    .unwrap_err();

    assert_eq!(error.kind_name(), "InvalidGuardExpression");
    assert_eq!(i.symbols().borrow().snapshot(), symbols);
    assert_eq!(*i.dictionary().borrow(), dictionary);
    assert_eq!(plan_snapshot(&i), topology);
    assert_eq!(i.plan().activation_registration_depth(), 0);
}

#[test]
fn activation_unmatched_guard_skips_runtime_error_and_guard_error_commits_nothing() {
    let mut i = interpret(
        r#"
<guard-event> := :pressed<f64> | :other<f64>
event<guard-event> := :pressed(1.0)
text := "abc"
~index := 1.0
~> event
  | :pressed(x), text[index] == "a" => {
      selected := x + 0.0
    }
  | * => {
      selected := -1.0
    }
"#,
    );
    let trigger = root_cell(&i, "event");
    let topology = plan_snapshot(&i);

    set_atom_tuple_event(&i, "pressed", 2.0);
    let outcome = i.advance_reactive_turn(&[trigger]).unwrap();
    assert_dispatch_turn(&i, &topology, &outcome, 0, 2.0);
    let committed_before = committed_capture_value(&i, 0, 0);
    let pulse_before = arm_pulse_generation(&i, 0);
    let body_before = body_output_f64(&i, 0);

    let LegacyValue::F64(index) = symbol(&i, "index") else {
        panic!("index is not f64")
    };
    *index.borrow_mut() = 4.0;
    set_atom_tuple_event(&i, "other", 3.0);
    let outcome = i.advance_reactive_turn(&[trigger]).unwrap();
    assert_dispatch_turn(&i, &topology, &outcome, 1, -1.0);
    let guard = registration(&i).arms[0].guard.clone().unwrap();
    let executed = turn_executed_nodes(&outcome);
    let changed = turn_changed_nodes(&outcome);
    let unchanged = turn_unchanged_nodes(&outcome);
    assert!(executed.contains(&guard.match_gate_node));
    assert!(unchanged.contains(&guard.match_gate_node));
    assert!(changed.contains(&registration(&i).arms[0].finalizer_node));
    for node in guard.guard_node_start..guard.guard_node_end {
        assert!(!executed.contains(&node));
    }

    let pulses_before_error = (0..2)
        .map(|arm| arm_pulse_generation(&i, arm))
        .collect::<Vec<_>>();
    let bodies_before_error = (0..2)
        .map(|arm| body_output_f64(&i, arm))
        .collect::<Vec<_>>();
    let proposed_before_error = proposed_capture_value(&i, 0, 0);
    set_atom_tuple_event(&i, "pressed", 3.0);
    let error = i.advance_reactive_turn(&[trigger]).unwrap_err();
    assert_eq!(error.kind_name(), "IndexOutOfBounds");
    assert_eq!(proposed_capture_value(&i, 0, 0), proposed_before_error);
    assert_eq!(committed_capture_value(&i, 0, 0), committed_before);
    assert_eq!(arm_pulse_generation(&i, 0), pulse_before);
    assert_eq!(body_output_f64(&i, 0), body_before);
    assert_eq!(
        (0..2)
            .map(|arm| arm_pulse_generation(&i, arm))
            .collect::<Vec<_>>(),
        pulses_before_error
    );
    assert_eq!(
        (0..2)
            .map(|arm| body_output_f64(&i, arm))
            .collect::<Vec<_>>(),
        bodies_before_error
    );
    assert_eq!(plan_snapshot(&i), topology);

    *index.borrow_mut() = 1.0;
    let outcome = i.advance_reactive_turn(&[trigger]).unwrap();
    assert_dispatch_turn(&i, &topology, &outcome, 0, 3.0);
    assert_eq!(
        committed_capture_value(&i, 0, 0),
        LegacyValue::F64(Ref::new(3.0))
    );
}

#[test]
fn activation_pattern_elaboration_error_restores_symbol_table() {
    let (i, symbols, _, _, _) = assert_failed_elaboration_restored();
    assert_eq!(i.symbols().borrow().snapshot(), symbols);
}

#[test]
fn activation_pattern_elaboration_error_restores_program_dictionary() {
    let (i, _, dictionary, _, _) = assert_failed_elaboration_restored();
    assert_eq!(*i.dictionary().borrow(), dictionary);
}

#[test]
fn activation_pattern_elaboration_error_restores_plan() {
    let (i, _, _, topology, _) = assert_failed_elaboration_restored();
    assert_eq!(plan_snapshot(&i), topology);
}

#[test]
fn activation_pattern_preflight_error_does_not_modify_plan() {
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
}

#[test]
fn activation_pattern_recursive_preflight_rejects_nested_activation() {
    let mut i = interpret("event := 1.0\ntick := 0.0");
    let symbols = i.symbols().borrow().snapshot();
    let dictionary = i.dictionary().borrow().clone();
    let topology = plan_snapshot(&i);
    let error = interpret_more(
            &mut i,
            "~> event\n  | 1.0 => {\n      ~> tick {\n        nested := 1.0\n      }\n    }\n  | * => {\n      fallback := 0.0\n    }",
        )
        .unwrap_err();
    assert_eq!(error.kind_name(), "ActivationPatternDefinitionUnsupported");
    assert_eq!(i.symbols().borrow().snapshot(), symbols);
    assert_eq!(*i.dictionary().borrow(), dictionary);
    assert_eq!(plan_snapshot(&i), topology);
    assert!(!i.symbols().borrow().contains(hash_str("nested")));
    assert!(!i.symbols().borrow().contains(hash_str("fallback")));
}

#[test]
fn activation_pattern_recursive_preflight_rejects_context_declaration() {
    let mut i = interpret("event := 1.0");
    let symbols = i.symbols().borrow().snapshot();
    let dictionary = i.dictionary().borrow().clone();
    let topology = plan_snapshot(&i);
    let context_bindings = i.context_bindings.borrow().clone();
    let error = interpret_more(
        &mut i,
        "~> event\n  | 1.0 => {
      @temporary := test://resource
    }\n  | * => {
      fallback := 0.0
    }",
    )
    .unwrap_err();
    assert_eq!(error.kind_name(), "ActivationPatternDefinitionUnsupported");
    assert_eq!(i.symbols().borrow().snapshot(), symbols);
    assert_eq!(*i.dictionary().borrow(), dictionary);
    assert_eq!(plan_snapshot(&i), topology);
    assert_eq!(*i.context_bindings.borrow(), context_bindings);
    assert!(
        !i.context_bindings
            .borrow()
            .contains_key(&hash_str("temporary"))
    );
    assert!(i.plan().pattern_activation_registrations().is_empty());
    assert!(!i.symbols().borrow().contains(hash_str("fallback")));
}
