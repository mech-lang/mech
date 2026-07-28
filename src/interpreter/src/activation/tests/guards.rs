use super::*;

    #[test]
    fn activation_guards_fall_through_in_source_order_and_commit_only_the_selected_arm() {
        let mut i = interpret(
            r#"
event := 0.0
~> event
  | first, first > 10.0 => {
      selected := first + 100.0
    }
  | second, second > 0.0 => {
      selected := second + 200.0
    }
  | * => {
      selected := -1.0
    }
"#,
        );
        let trigger = root_cell(&i, "event");
        let topology = plan_snapshot(&i);
        let activation = registration(&i);
        assert!(activation.arms[0].guard.is_some());
        assert!(activation.arms[1].guard.is_some());
        assert!(activation.arms[2].guard.is_none());

        let Value::F64(event) = symbol(&i, "event") else {
            panic!("event is not f64")
        };
        *event.borrow_mut() = 20.0;
        let outcome = i.advance_reactive_turn(&[trigger]).unwrap();
        assert_dispatch_turn(&i, &topology, &outcome, 0, 120.0);
        let changed = turn_changed_nodes(&outcome);
        let unchanged = turn_unchanged_nodes(&outcome);
        for arm in &activation.arms[..2] {
            let guard = arm.guard.as_ref().unwrap();
            assert!(changed.contains(&guard.match_gate_node));
            assert!(changed.contains(&guard.guard_finalizer_node));
            assert!(unchanged.contains(&arm.finalizer_node));
        }
        assert_eq!(committed_capture_value(&i, 0, 0), Value::F64(Ref::new(20.0)));
        assert_eq!(committed_capture_value(&i, 1, 0), Value::F64(Ref::new(0.0)));

        *event.borrow_mut() = 5.0;
        let outcome = i.advance_reactive_turn(&[trigger]).unwrap();
        assert_dispatch_turn(&i, &topology, &outcome, 1, 205.0);
        assert_eq!(committed_capture_value(&i, 0, 0), Value::F64(Ref::new(20.0)));
        assert_eq!(committed_capture_value(&i, 1, 0), Value::F64(Ref::new(5.0)));

        *event.borrow_mut() = -5.0;
        let outcome = i.advance_reactive_turn(&[trigger]).unwrap();
        assert_dispatch_turn(&i, &topology, &outcome, 2, -1.0);
        assert_eq!(proposed_capture_value(&i, 0, 0), Value::F64(Ref::new(-5.0)));
        assert_eq!(proposed_capture_value(&i, 1, 0), Value::F64(Ref::new(-5.0)));
        assert_eq!(committed_capture_value(&i, 0, 0), Value::F64(Ref::new(20.0)));
        assert_eq!(committed_capture_value(&i, 1, 0), Value::F64(Ref::new(5.0)));
    }


    #[test]
    fn activation_guard_outer_dependencies_are_sampled_until_the_next_trigger() {
        let mut i = interpret(
            r#"
event := (:pressed, 0.0)
threshold := 10.0
~> event
  | :pressed(x), x > threshold => {
      selected := x + 0.0
    }
  | * => {
      selected := -1.0
    }
"#,
        );
        let trigger = root_cell(&i, "event");
        let threshold_cell = root_cell(&i, "threshold");
        let topology = plan_snapshot(&i);
        let activation = registration(&i);
        let guard = activation.arms[0].guard.as_ref().unwrap();
        let Value::F64(threshold) = symbol(&i, "threshold") else {
            panic!("threshold is not f64")
        };
        *threshold.borrow_mut() = 3.0;

        let dependency_turn = i.advance_reactive_turn(&[threshold_cell]).unwrap();
        let executed = turn_executed_nodes(&dependency_turn);
        assert!(!executed.contains(&activation.scope_pulse_node));
        assert!(!executed.contains(&guard.match_gate_node));
        assert!(!executed.contains(&guard.guard_finalizer_node));
        for node in guard.guard_node_start..guard.guard_node_end {
            assert!(!executed.contains(&node));
        }
        assert_eq!(plan_snapshot(&i), topology);

        set_atom_tuple_event(&i, "pressed", 5.0);
        let outcome = i.advance_reactive_turn(&[trigger]).unwrap();
        assert_dispatch_turn(&i, &topology, &outcome, 0, 5.0);
    }


    #[test]
    fn activation_guard_user_function_refreshes_on_each_matching_trigger() {
        let mut i = interpret(
            r#"
passes(value<f64>, limit<f64>) => <bool>
  | value > limit.

event := (:pressed, 0.0)
threshold := 5.0
~> event
  | :pressed(x), passes(x, threshold) => {
      selected := x + 0.0
    }
  | * => {
      selected := -1.0
    }
"#,
        );
        let trigger = root_cell(&i, "event");
        let threshold_cell = root_cell(&i, "threshold");
        let topology = plan_snapshot(&i);
        {
            let functions = i.functions();
            let functions = functions.borrow();
            let passes = functions.user_functions.get(&hash_str("passes")).unwrap();
            assert_eq!(passes.code.match_arms.len(), 1);
            assert!(matches!(passes.code.match_arms[0].pattern, Pattern::Wildcard));
        }

        set_atom_tuple_event(&i, "pressed", 6.0);
        let outcome = i.advance_reactive_turn(&[trigger]).unwrap();
        assert_dispatch_turn(&i, &topology, &outcome, 0, 6.0);

        set_atom_tuple_event(&i, "pressed", 4.0);
        let outcome = i.advance_reactive_turn(&[trigger]).unwrap();
        assert_dispatch_turn(&i, &topology, &outcome, 1, -1.0);

        let Value::F64(threshold) = symbol(&i, "threshold") else {
            panic!("threshold is not f64")
        };
        *threshold.borrow_mut() = 3.0;
        let dependency_turn = i.advance_reactive_turn(&[threshold_cell]).unwrap();
        assert!(turn_executed_nodes(&dependency_turn).is_empty());

        let outcome = i.advance_reactive_turn(&[trigger]).unwrap();
        assert_dispatch_turn(&i, &topology, &outcome, 0, 4.0);
    }


    #[test]
    fn activation_guard_initialization_commits_the_first_eligible_arm_without_pulsing_a_body() {
        let i = interpret(
            r#"
event := 5.0
~> event
  | first, first > 10.0 => {
      selected := first + 100.0
    }
  | second, second > 0.0 => {
      selected := second + 200.0
    }
  | * => {
      selected := -1.0
    }
"#,
        );

        assert_eq!(proposed_capture_value(&i, 0, 0), Value::F64(Ref::new(5.0)));
        assert_eq!(proposed_capture_value(&i, 1, 0), Value::F64(Ref::new(5.0)));
        assert_eq!(committed_capture_value(&i, 0, 0), Value::F64(Ref::new(0.0)));
        assert_eq!(committed_capture_value(&i, 1, 0), Value::F64(Ref::new(5.0)));
        assert_eq!(body_output_f64(&i, 1), 205.0);
        for arm in 0..3 {
            assert_eq!(arm_pulse_generation(&i, arm), 0);
        }
    }


    #[test]
    fn activation_guard_equal_packets_dispatch_again_without_changing_topology() {
        let mut i = interpret(
            r#"
event := 20.0
~> event
  | value, value > 10.0 => {
      selected := value + 1.0
    }
  | * => {
      selected := -1.0
    }
"#,
        );
        let trigger = root_cell(&i, "event");
        let topology = plan_snapshot(&i);
        for expected_pulse in [1usize, 2] {
            let outcome = i.advance_reactive_turn(&[trigger]).unwrap();
            assert_dispatch_turn(&i, &topology, &outcome, 0, 21.0);
            let guard = registration(&i).arms[0].guard.clone().unwrap();
            let executed = turn_executed_nodes(&outcome);
            for guard_node in guard.guard_node_start..guard.guard_node_end {
                assert_eq!(
                    executed.iter().filter(|node| **node == guard_node).count(),
                    1
                );
            }
            assert_eq!(arm_pulse_generation(&i, 0), expected_pulse);
        }
    }


    #[test]
    fn activation_guard_rejects_unclassified_native_compiler_before_compile() {
        let mut i = interpret("event := (:released, 1.0)");
        let compile_calls = Arc::new(AtomicUsize::new(0));
        i.functions().borrow_mut().insert_function_compiler(
            "test/eager-guard",
            Arc::new(EagerGuardTestCompiler {
                compile_calls: compile_calls.clone(),
            }),
        );
        let symbols = i.symbols().borrow().snapshot();
        let topology = plan_snapshot(&i);

        let error = interpret_more(
            &mut i,
            r#"
~> event
  | :pressed(x), test/eager-guard(x) => {
      selected := x
    }
  | * => {
      selected := -1.0
    }
"#,
        )
        .unwrap_err();

        assert_eq!(error.kind_name(), "ActivationPatternGuardMustBePure");
        assert_eq!(compile_calls.load(Ordering::SeqCst), 0);
        assert_eq!(i.symbols().borrow().snapshot(), symbols);
        assert_eq!(plan_snapshot(&i), topology);
        assert!(i.plan().pattern_activation_registrations().is_empty());
        assert_eq!(i.plan().activation_registration_depth(), 0);
    }


    #[test]
    fn activation_guard_rejects_eager_nested_match_control_flow() {
        let mut i = interpret("event := 1.0");
        let symbols = i.symbols().borrow().snapshot();
        let topology = plan_snapshot(&i);

        let error = interpret_more(
            &mut i,
            r#"
~> event
  | x, (x? | 0.0 => false | * => true.) => {
      selected := x
    }
  | * => {
      selected := -1.0
    }
"#,
        )
        .unwrap_err();

        assert_eq!(error.kind_name(), "ActivationPatternGuardMustBePure");
        assert_eq!(i.symbols().borrow().snapshot(), symbols);
        assert_eq!(plan_snapshot(&i), topology);
    }


    #[test]
    fn activation_guard_rejects_user_function_pattern_dispatch_that_cannot_refresh_statically() {
        let mut i = interpret(
            r#"
passes(value<f64>) => <bool>
  | 0.0 => false
  | * => true.

event := 1.0
"#,
        );
        let topology = plan_snapshot(&i);

        let error = interpret_more(
            &mut i,
            r#"
~> event
  | x, passes(x) => {
      selected := x
    }
  | * => {
      selected := -1.0
    }
"#,
        )
        .unwrap_err();

        assert_eq!(error.kind_name(), "ActivationPatternGuardMustBePure");
        assert_eq!(plan_snapshot(&i), topology);
    }


    #[cfg(all(feature = "matrix", feature = "f64"))]
    #[test]
    fn activation_guard_capture_shadows_outer_name_while_pattern_expression_keeps_outer_name() {
        let mut i = interpret(
            r#"
x := 9.0
event := [1.0 10.0]

~> event
  | [x, x + 1.0], x < 2.0 => {
      selected := x + 0.0
    }
  | * => {
      selected := -1.0
    }
"#,
        );
        let trigger = root_cell(&i, "event");
        let outer_cell = root_cell(&i, "x");
        let activation = registration(&i);
        let proposed_cell = proposed_capture_value(&i, 0, 0).reactive_root_cell_ids()[0];
        let committed_cell = activation.arms[0].captures[0].cell;
        assert_ne!(outer_cell, proposed_cell);
        assert_ne!(outer_cell, committed_cell);
        assert_ne!(proposed_cell, committed_cell);
        let guard = activation.arms[0].guard.as_ref().unwrap();
        {
            let plan = i.plan();
            let plan = plan.borrow();
            let guard_pulse_cell = plan.node(guard.match_gate_node).unwrap().outputs[0];
            for node in &plan.nodes[guard.guard_node_start..guard.guard_node_end] {
                assert!(node.inputs.iter().any(|dependency| {
                    dependency.cell == guard_pulse_cell
                        && dependency.kind == ReactiveDependencyKind::Reactive
                }));
                assert!(node.inputs.iter().any(|dependency| {
                    dependency.cell == proposed_cell
                        && dependency.kind == ReactiveDependencyKind::Sampled
                }));
                assert!(!node
                    .inputs
                    .iter()
                    .any(|dependency| dependency.cell == committed_cell));
            }
        }
        let topology = plan_snapshot(&i);

        let outcome = i.advance_reactive_turn(&[trigger]).unwrap();

        assert_dispatch_turn(&i, &topology, &outcome, 0, 1.0);
        assert_eq!(symbol(&i, "x"), Value::F64(Ref::new(9.0)));
        assert_eq!(proposed_capture_value(&i, 0, 0), Value::F64(Ref::new(1.0)));
        assert_eq!(committed_capture_value(&i, 0, 0), Value::F64(Ref::new(1.0)));
    }


    #[cfg(all(feature = "matrix", feature = "f64"))]
    #[test]
    fn activation_guard_composite_rest_proposal_commits_only_when_the_guard_passes() {
        let mut i = interpret(
            r#"
event := [1.0 2.0 3.0]
~> event
  | [head | rest], head < rest[1] && rest[2] > rest[1] => {
      selected := head + 0.0
    }
  | * => {
      selected := -1.0
    }
"#,
        );
        let trigger = root_cell(&i, "event");
        let topology = plan_snapshot(&i);

        set_f64_matrix_event(&i, vec![4.0, 5.0, 6.0]);
        let outcome = i.advance_reactive_turn(&[trigger]).unwrap();
        assert_eq!(selected_arm_index(&registration(&i), &outcome), 0);
        assert_eq!(body_output_f64(&i, 0), 4.0);
        assert_eq!(plan_snapshot(&i), topology);
        let committed_before = committed_capture_value(&i, 0, 1);
        let pulse_before = arm_pulse_generation(&i, 0);

        set_f64_matrix_event(&i, vec![7.0, 6.0, 5.0]);
        let outcome = i.advance_reactive_turn(&[trigger]).unwrap();
        assert_dispatch_turn(&i, &topology, &outcome, 1, -1.0);
        let Value::MatrixF64(proposed_rest) = proposed_capture_value(&i, 0, 1) else {
            panic!("proposed rest is not an f64 matrix")
        };
        assert_eq!(proposed_rest.as_vec(), vec![6.0, 5.0]);
        assert_eq!(committed_capture_value(&i, 0, 1), committed_before);
        assert_eq!(arm_pulse_generation(&i, 0), pulse_before);
        assert_eq!(body_output_f64(&i, 0), 4.0);
        assert_eq!(plan_snapshot(&i), topology);
    }
