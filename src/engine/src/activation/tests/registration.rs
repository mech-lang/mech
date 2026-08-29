use super::support::{
    CompiledPattern, Finalize, GuardFinalize, Interpreter, Matcher, MechFunctionImpl,
    ReactiveDependencyKind, Select, UnmatchedFinalize, ValueCell, arm_register_nodes, f64_symbol,
    hash_str, interpret, interpret_more, plan_snapshot, registration, root_cell,
    selected_arm_index, set_f64_symbol, symbol,
};

#[cfg(any(feature = "bool", feature = "variable_define"))]
#[test]
fn activation_transaction_state_exposes_hidden_mutable_cells() {
    fn contains_representation(
        ports: &[mech_core::FunctionStatePort<'_>],
        expected: mech_core::FunctionValueRepresentation,
    ) -> bool {
        ports.iter().any(|port| port.representation() == expected)
    }

    let matched = ValueCell::from_exact(false).unwrap();
    let matcher = Matcher {
        pattern: CompiledPattern::Wildcard,
        trigger: ValueCell::unit(),
        expression_values: Vec::new(),
        captures: Vec::new(),
        matched: matched.clone(),
        out: ValueCell::from_exact(1_usize).unwrap(),
    };
    let matcher_ports = matcher.transaction_state_ports().unwrap().unwrap();
    assert_eq!(matcher_ports.len(), 2);
    assert!(contains_representation(
        &matcher_ports,
        mech_core::FunctionValueRepresentation::Bool
    ));

    let eligible = ValueCell::from_exact(false).unwrap();
    let finalize = Finalize {
        matched: matched.clone(),
        eligible: eligible.clone(),
        out: ValueCell::from_exact(1_usize).unwrap(),
    };
    let finalize_ports = finalize.transaction_state_ports().unwrap().unwrap();
    assert_eq!(finalize_ports.len(), 2);
    assert!(contains_representation(
        &finalize_ports,
        mech_core::FunctionValueRepresentation::Bool
    ));

    let unmatched_eligible = ValueCell::from_exact(false).unwrap();
    let unmatched = UnmatchedFinalize {
        matched: matched.clone(),
        eligible: unmatched_eligible.clone(),
        out: ValueCell::from_exact(1_usize).unwrap(),
    };
    let unmatched_ports = unmatched.transaction_state_ports().unwrap().unwrap();
    assert_eq!(unmatched_ports.len(), 2);
    assert!(contains_representation(
        &unmatched_ports,
        mech_core::FunctionValueRepresentation::Bool
    ));

    let guard_eligible = ValueCell::from_exact(false).unwrap();
    let guard = GuardFinalize {
        guard: ValueCell::from_exact(false).unwrap(),
        eligible: guard_eligible.clone(),
        out: ValueCell::from_exact(1_usize).unwrap(),
    };
    let guard_ports = guard.transaction_state_ports().unwrap().unwrap();
    assert_eq!(guard_ports.len(), 2);
    assert!(contains_representation(
        &guard_ports,
        mech_core::FunctionValueRepresentation::Bool
    ));

    let selected = ValueCell::from_exact(usize::MAX).unwrap();
    let select = Select {
        eligible: vec![eligible],
        selected: selected.clone(),
        out: ValueCell::from_exact(1_usize).unwrap(),
    };
    let select_ports = select.transaction_state_ports().unwrap().unwrap();
    assert_eq!(select_ports.len(), 2);
    assert!(contains_representation(
        &select_ports,
        mech_core::FunctionValueRepresentation::Index
    ));
}

#[test]
fn activation_patterned_body_rejects_writes_to_its_trigger() {
    for assignment in ["event = event + 1.0", "event += 1.0"] {
        let mut interpreter = interpret("~event := 1.0");
        let topology = plan_snapshot(&interpreter);
        let error = interpret_more(
            &mut interpreter,
            &format!(
                r#"
~> event
  | * => {{
      {assignment}
    }}
"#
            ),
        )
        .unwrap_err();
        assert_eq!(error.kind_name(), "ActivationScopeTriggerWriteUnsupported");
        assert_eq!(f64_symbol(&interpreter, "event"), 1.0);
        assert_eq!(plan_snapshot(&interpreter), topology);
        assert_eq!(interpreter.plan().activation_registration_depth(), 0);
    }
}

#[test]
fn activation_patterned_body_rejects_writes_through_a_trigger_alias() {
    let mut interpreter = interpret(
        r#"
~event := 1.0
alias := event
"#,
    );
    let topology = plan_snapshot(&interpreter);
    let error = interpret_more(
        &mut interpreter,
        r#"
~> alias
  | * => {
      event += 1.0
    }
"#,
    )
    .unwrap_err();
    assert_eq!(error.kind_name(), "ActivationScopeTriggerWriteUnsupported");
    assert_eq!(f64_symbol(&interpreter, "event"), 1.0);
    assert_eq!(plan_snapshot(&interpreter), topology);
    assert_eq!(interpreter.plan().activation_registration_depth(), 0);
}

#[cfg(all(feature = "matrix", feature = "f64"))]
#[test]
fn activation_patterned_body_rejects_eager_subscript_writes() {
    for assignment in ["values[1] = 3.0", "values[1] += 3.0"] {
        let mut interpreter = interpret(
            r#"
event := 0.0
~values := [1.0 2.0]
"#,
        );
        let topology = plan_snapshot(&interpreter);
        let values_before = symbol(&interpreter, "values");
        let error = interpret_more(
            &mut interpreter,
            &format!(
                r#"
~> event
  | * => {{
      {assignment}
    }}
"#
            ),
        )
        .unwrap_err();
        assert_eq!(
            error.kind_name(),
            "ActivationPatternRegisterWriteUnsupported"
        );
        assert_eq!(symbol(&interpreter, "values"), values_before);
        assert_eq!(plan_snapshot(&interpreter), topology);
        assert_eq!(interpreter.plan().activation_registration_depth(), 0);
    }
}

#[test]
fn activation_pattern_arm_definitions_do_not_leak_between_arms() {
    let mut i = interpret("event := (1.0, 2.0)");
    let symbols = i.symbols().borrow().snapshot();
    let dictionary = i.dictionary().borrow().clone();
    let topology = plan_snapshot(&i);
    let error = interpret_more(
        &mut i,
        "~> event\n  | (x, y) => {
      first-local := x + y
    }\n  | (a, b) => {
      second-local := first-local + a + b
    }\n  | * => {
      fallback := 0.0
    }",
    )
    .unwrap_err();
    assert_eq!(error.kind_name(), "UndefinedVariable");
    assert_eq!(i.symbols().borrow().snapshot(), symbols);
    assert_eq!(*i.dictionary().borrow(), dictionary);
    assert_eq!(plan_snapshot(&i), topology);
    for name in [
        "first-local",
        "second-local",
        "fallback",
        "x",
        "y",
        "a",
        "b",
    ] {
        assert!(!i.symbols().borrow().contains(hash_str(name)));
    }
}

#[test]
fn activation_arm_alias_of_live_input_remains_sampled_until_trigger() {
    let mut interpreter = Interpreter::with_function_catalog(
        0,
        10_000,
        crate::test_support::catalog::function_catalog(),
    );
    let outer_id = hash_str("outer");
    {
        let symbols = interpreter.symbols();
        let mut symbols = symbols.borrow_mut();
        symbols.insert_cell(outer_id, ValueCell::from_exact(1.0_f64).unwrap(), true);
        symbols
            .dictionary
            .borrow_mut()
            .insert(outer_id, "outer".to_string());
    }
    interpreter
        .dictionary()
        .borrow_mut()
        .insert(outer_id, "outer".to_string());
    interpret_more(
        &mut interpreter,
        r#"
event := 0.0
~state := 0.0

~> event
  | tick => {
      sampled := outer
      state = sampled
    }
"#,
    )
    .unwrap();

    let trigger = root_cell(&interpreter, "event");
    let outer = root_cell(&interpreter, "outer");
    let activation = registration(&interpreter);
    let registers = arm_register_nodes(&interpreter, &activation, 0);
    let topology = plan_snapshot(&interpreter);
    assert_eq!(registers.len(), 1);
    assert_eq!(f64_symbol(&interpreter, "state"), 0.0);
    {
        let plan = interpreter.plan();
        let plan = plan.borrow();
        let register = plan.node(registers[0]).unwrap();
        assert!(register.inputs.iter().any(|dependency| {
            dependency.cell == outer && dependency.kind == ReactiveDependencyKind::Sampled
        }));
        assert!(register.inputs.iter().any(|dependency| {
            dependency.cell == activation.arms[0].pulse_cell
                && dependency.kind == ReactiveDependencyKind::Reactive
        }));
    }

    set_f64_symbol(&interpreter, "outer", 5.0);
    let sampled_only = interpreter.advance_reactive_turn(&[outer]).unwrap();
    assert!(
        !sampled_only
            .before_commit
            .pending_register_nodes
            .contains(&registers[0])
    );
    assert!(
        !sampled_only
            .register_commit
            .committed_nodes
            .contains(&registers[0])
    );
    assert_eq!(f64_symbol(&interpreter, "state"), 0.0);

    let tick = interpreter.advance_reactive_turn(&[trigger]).unwrap();
    assert_eq!(selected_arm_index(&activation, &tick), 0);
    assert_eq!(tick.register_commit.committed_nodes, registers);
    assert_eq!(f64_symbol(&interpreter, "state"), 5.0);
    assert_eq!(plan_snapshot(&interpreter), topology);
}
