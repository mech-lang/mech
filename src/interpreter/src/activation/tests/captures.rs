use super::support::{
    ActivationPatternCapture,
    ActivationPatternCaptureKindUnsupported,
    FLAT_TUPLE_ACTIVATION,
    Gate,
    HashMap,
    Interpreter,
    Matrix,
    MechAtom,
    MechEnum,
    MechFunctionImpl,
    MechMap,
    MechRecord,
    MechSet,
    MechTable,
    MechTuple,
    PatternBinding,
    PatternBindingSink,
    PatternMatch,
    ReactiveBindingSink,
    ReactiveDependencyKind,
    Ref,
    Value,
    ValueKind,
    arm_register_nodes,
    assert_dispatch_turn,
    body_output,
    commit_capture_slot,
    committed_capture_value,
    create_capture_slot_for_kind,
    detached,
    f64_symbol,
    hash_str,
    interpret,
    interpret_more,
    load_enum_activation,
    plan_snapshot,
    proposed_capture_value,
    registration,
    root_cell,
    scalar_capture_cases,
    selected_arm_index,
    set_enum_event,
    set_f64_matrix_event,
    set_f64_symbol,
    set_tuple_event,
    symbol,
    symbol_ref,
    tuple_fixture,
    turn_executed_nodes,
};

    #[test]
    fn activation_capture_slot_supports_all_enabled_scalar_kinds() {
        let interpreter = Interpreter::new_with_full_stdlib(0);
        for (kind, source) in scalar_capture_cases() {
            let slot = create_capture_slot_for_kind(&kind, &interpreter).unwrap();
            let cells_before = slot.reactive_root_cell_ids();
            assert_eq!(cells_before.len(), 1);
            commit_capture_slot(&slot, &source).unwrap();
            assert_eq!(slot, source);
            assert_eq!(slot.reactive_root_cell_ids(), cells_before);
        }
    }

    #[cfg(any(feature = "string", feature = "variable_define"))]
    #[test]
    fn activation_capture_slot_preserves_identity_across_updates() {
        let interpreter = Interpreter::new_with_full_stdlib(0);
        let slot = create_capture_slot_for_kind(&ValueKind::String, &interpreter).unwrap();
        let cells = slot.reactive_root_cell_ids();
        commit_capture_slot(&slot, &Value::String(Ref::new("first".to_string()))).unwrap();
        assert_eq!(slot, Value::String(Ref::new("first".to_string())));
        assert_eq!(slot.reactive_root_cell_ids(), cells);
        commit_capture_slot(&slot, &Value::String(Ref::new("second".to_string()))).unwrap();
        assert_eq!(slot, Value::String(Ref::new("second".to_string())));
        assert_eq!(slot.reactive_root_cell_ids(), cells);
    }

    #[cfg(all(
        feature = "tuple",
        feature = "enum",
        feature = "record",
        feature = "map",
        feature = "set",
        feature = "table",
        feature = "string",
        feature = "f64"
    ))]
    #[test]
    fn activation_capture_slots_support_enabled_composite_value_kinds() {
        let interpreter = Interpreter::new_with_full_stdlib(0);
        let enum_id = hash_str("capture-enum");
        let variant_id = hash_str("payload");
        let names = Ref::new(HashMap::from([
            (enum_id, "capture-enum".to_string()),
            (variant_id, "payload".to_string()),
        ]));
        let cases = vec![
            Value::Tuple(Ref::new(MechTuple::from_vec(vec![
                Value::F64(Ref::new(1.0)),
                Value::String(Ref::new("tuple".to_string())),
            ]))),
            Value::Enum(Ref::new(MechEnum {
                id: enum_id,
                variants: vec![(variant_id, Some(Value::F64(Ref::new(2.0))))],
                names,
            })),
            Value::Record(Ref::new(MechRecord::new(vec![
                ("field", Value::F64(Ref::new(3.0))),
            ]))),
            Value::Map(Ref::new(MechMap::from_vec(vec![(
                Value::String(Ref::new("key".to_string())),
                Value::F64(Ref::new(4.0)),
            )]))),
            Value::Set(Ref::new(MechSet::from_vec(vec![Value::String(Ref::new(
                "member".to_string(),
            ))]))),
            Value::Table(Ref::new(MechTable::new_table(
                vec!["column".to_string()],
                vec![ValueKind::F64],
                vec![vec![Value::F64(Ref::new(5.0)), Value::F64(Ref::new(6.0))]],
            ))),
        ];

        for source in cases {
            let kind = source.kind();
            let slot = create_capture_slot_for_kind(&kind, &interpreter).unwrap();
            let cells = slot.reactive_root_cell_ids();
            assert_eq!(cells.len(), 1, "missing stable root for {kind}");
            commit_capture_slot(&slot, &source).unwrap();
            assert_eq!(slot, source);
            assert_eq!(slot.reactive_root_cell_ids(), cells);
        }
    }

    #[cfg(all(feature = "f64", feature = "string"))]
    #[test]
    fn activation_capture_commit_validates_every_binding_before_mutation() {
        let interpreter = Interpreter::new_with_full_stdlib(0);
        let number = ActivationPatternCapture {
            id: hash_str("number"),
            name: "number".to_string(),
            kind: ValueKind::F64,
            proposed: create_capture_slot_for_kind(&ValueKind::F64, &interpreter).unwrap(),
            committed: create_capture_slot_for_kind(&ValueKind::F64, &interpreter).unwrap(),
        };
        let text = ActivationPatternCapture {
            id: hash_str("text"),
            name: "text".to_string(),
            kind: ValueKind::String,
            proposed: create_capture_slot_for_kind(&ValueKind::String, &interpreter).unwrap(),
            committed: create_capture_slot_for_kind(&ValueKind::String, &interpreter).unwrap(),
        };
        let captures = vec![number, text];
        let attempted = PatternMatch {
            matched: true,
            bindings: vec![
                PatternBinding {
                    index: 0,
                    id: hash_str("number"),
                    name: "number".to_string(),
                    kind: ValueKind::F64,
                    value: Value::F64(Ref::new(9.0)),
                },
                PatternBinding {
                    index: 1,
                    id: hash_str("text"),
                    name: "text".to_string(),
                    kind: ValueKind::F64,
                    value: Value::F64(Ref::new(10.0)),
                },
            ],
        };

        let error = ReactiveBindingSink {
            captures: &captures,
        }
        .commit(&attempted)
        .unwrap_err();
        assert_eq!(error.kind_name(), "ActivationPatternCaptureKindUnsupported");
        assert_eq!(captures[0].proposed, Value::F64(Ref::new(0.0)));
        assert_eq!(captures[1].proposed, Value::String(Ref::new(String::new())));
        assert_eq!(captures[0].committed, Value::F64(Ref::new(0.0)));
        assert_eq!(
            captures[1].committed,
            Value::String(Ref::new(String::new()))
        );
    }

    #[cfg(all(feature = "f64", feature = "string"))]
    #[test]
    fn activation_capture_gate_validates_entire_commit_before_mutation_or_pulse() {
        let captures = vec![
            ActivationPatternCapture {
                id: hash_str("number"),
                name: "number".to_string(),
                kind: ValueKind::F64,
                proposed: Value::F64(Ref::new(9.0)),
                committed: Value::F64(Ref::new(1.0)),
            },
            ActivationPatternCapture {
                id: hash_str("text"),
                name: "text".to_string(),
                kind: ValueKind::String,
                proposed: Value::F64(Ref::new(10.0)),
                committed: Value::String(Ref::new("before".to_string())),
            },
        ];
        let selected = Ref::new(0);
        let pulse = Ref::new(0);
        let gate = Gate {
            arm: 0,
            selected,
            captures: captures.clone(),
            out: pulse.clone(),
        };

        let error = gate.solve_reactive().unwrap_err();
        assert_eq!(error.kind_name(), "ActivationPatternCaptureKindUnsupported");
        assert_eq!(captures[0].committed, Value::F64(Ref::new(1.0)));
        assert_eq!(
            captures[1].committed,
            Value::String(Ref::new("before".to_string()))
        );
        assert_eq!(
            *pulse.borrow(),
            0,
            "body pulse must follow a successful commit"
        );
    }

    #[cfg(feature = "atom")]
    #[test]
    fn activation_atom_capture_accepts_a_new_atom_value() {
        let mut interpreter = interpret(
            r#"
event := :first
~> event
  | captured => {
      selected := captured
    }
  | * => {
      selected := :fallback
    }
"#,
        );
        let trigger = root_cell(&interpreter, "event");
        let topology = plan_snapshot(&interpreter);
        let registration = registration(&interpreter);
        let Value::Atom(event) = symbol(&interpreter, "event") else {
            panic!("event is not an atom")
        };
        *event.borrow_mut() = MechAtom::from_name("second");

        let outcome = interpreter.advance_reactive_turn(&[trigger]).unwrap();
        assert_eq!(selected_arm_index(&registration, &outcome), 0);
        let selected_atom = {
            let plan = interpreter.plan();
            let plan = plan.borrow();
            (registration.arms[0].body_node_start..registration.arms[0].body_node_end)
                .rev()
                .find_map(|node| match detached(&plan.node(node).unwrap().function.out()) {
                    Value::Atom(atom) => Some(atom.borrow().id()),
                    _ => None,
                })
                .expect("no atom output in selected arm body")
        };
        assert_eq!(selected_atom, hash_str("second"));
        assert_eq!(plan_snapshot(&interpreter), topology);
    }

    #[cfg(all(feature = "f64", any(feature = "string", feature = "variable_define")))]
    #[test]
    fn activation_capture_slot_rejects_kind_mismatch() {
        let interpreter = Interpreter::new_with_full_stdlib(0);
        let slot = create_capture_slot_for_kind(&ValueKind::F64, &interpreter).unwrap();
        let error =
            commit_capture_slot(&slot, &Value::String(Ref::new("wrong".to_string()))).unwrap_err();
        assert_eq!(error.kind_name(), "ActivationPatternCaptureKindUnsupported");
    }

    #[test]
    fn activation_pattern_enum_payload_capture_is_available() {
        let (mut i, trigger, r, topology) = load_enum_activation();
        let cell = r.arms[0].captures[0].cell;
        assert!(
            i.plan().borrow().nodes[r.arms[0].body_node_start..r.arms[0].body_node_end]
                .iter()
                .any(|n| n.inputs.iter().any(|d| d.cell == cell))
        );
        set_enum_event(&i, "pressed", 10.);
        let o = i.advance_reactive_turn(&[trigger]).unwrap();
        assert_dispatch_turn(&i, &topology, &o, 0, 10.);
    }

    #[test]
    fn activation_only_selected_arm_commits_matching_captures() {
        let mut i = interpret(
            r#"
event := 1.0
~> event
  | first => {
      selected := first
    }
  | later => {
      rejected := later
    }
  | * => {
      fallback := -1.0
    }
"#,
        );
        let trigger = root_cell(&i, "event");
        let activation = registration(&i);
        let later_before = committed_capture_value(&i, 1, 0);
        let committed_cell = activation.arms[0].captures[0].cell;
        let proposed_cell = proposed_capture_value(&i, 0, 0).reactive_root_cell_ids()[0];
        assert_ne!(proposed_cell, committed_cell);
        {
            let plan = i.plan();
            let plan = plan.borrow();
            assert!(
                plan.nodes[activation.arms[0].matcher_node]
                    .outputs
                    .contains(&proposed_cell)
            );
            assert!(
                !plan.nodes[activation.arms[0].matcher_node]
                    .outputs
                    .contains(&committed_cell)
            );
            assert!(
                plan.nodes[activation.arms[0].gate_node]
                    .outputs
                    .contains(&committed_cell)
            );
            let body_inputs = plan.nodes
                [activation.arms[0].body_node_start..activation.arms[0].body_node_end]
                .iter()
                .flat_map(|node| node.inputs.iter().map(|dependency| dependency.cell))
                .collect::<Vec<_>>();
            assert!(body_inputs.contains(&committed_cell));
            assert!(!body_inputs.contains(&proposed_cell));
        }
        let Value::F64(event) = symbol(&i, "event") else {
            panic!("event is not f64")
        };
        *event.borrow_mut() = 5.0;

        let outcome = i.advance_reactive_turn(&[trigger]).unwrap();

        assert_eq!(selected_arm_index(&activation, &outcome), 0);
        assert_eq!(committed_capture_value(&i, 0, 0), Value::F64(Ref::new(5.0)));
        assert_eq!(proposed_capture_value(&i, 1, 0), Value::F64(Ref::new(5.0)));
        assert_eq!(committed_capture_value(&i, 1, 0), later_before);
        let executed = turn_executed_nodes(&outcome);
        for node in activation.arms[1].body_node_start..activation.arms[1].body_node_end {
            assert!(!executed.contains(&node));
        }
    }

    #[test]
    fn activation_failed_repeated_binding_leaves_proposed_and_committed_unchanged() {
        let mut i = interpret(
            r#"
event := (1.0, 1.0)
~> event
  | (x, x) => {
      selected := x
    }
  | * => {
      selected := -1.0
    }
"#,
        );
        let trigger = root_cell(&i, "event");
        let activation = registration(&i);
        let proposed_before = proposed_capture_value(&i, 0, 0);
        let committed_before = committed_capture_value(&i, 0, 0);
        set_tuple_event(
            &i,
            vec![Value::F64(Ref::new(2.0)), Value::F64(Ref::new(3.0))],
        );

        let outcome = i.advance_reactive_turn(&[trigger]).unwrap();

        assert_eq!(selected_arm_index(&activation, &outcome), 1);
        assert_eq!(proposed_capture_value(&i, 0, 0), proposed_before);
        assert_eq!(committed_capture_value(&i, 0, 0), committed_before);
        let executed = turn_executed_nodes(&outcome);
        for node in activation.arms[0].body_node_start..activation.arms[0].body_node_end {
            assert!(!executed.contains(&node));
        }
    }

    #[test]
    fn activation_non_selected_composite_capture_keeps_last_committed_value() {
        let mut i = interpret(
            r#"
event := (0.0, 1.0)
~> event
  | (1.0, x) => {
      selected := x
    }
  | later => {
      rejected := later.2
    }
  | * => {
      fallback := -1.0
    }
"#,
        );
        let trigger = root_cell(&i, "event");
        let activation = registration(&i);
        let committed_before = committed_capture_value(&i, 1, 0);
        set_tuple_event(
            &i,
            vec![Value::F64(Ref::new(1.0)), Value::F64(Ref::new(10.0))],
        );

        let outcome = i.advance_reactive_turn(&[trigger]).unwrap();

        assert_eq!(selected_arm_index(&activation, &outcome), 0);
        assert_eq!(committed_capture_value(&i, 1, 0), committed_before);
        assert_eq!(
            proposed_capture_value(&i, 1, 0),
            Value::Tuple(Ref::new(MechTuple::from_vec(vec![
                Value::F64(Ref::new(1.0)),
                Value::F64(Ref::new(10.0)),
            ])))
        );
        let executed = turn_executed_nodes(&outcome);
        for node in activation.arms[1].body_node_start..activation.arms[1].body_node_end {
            assert!(!executed.contains(&node));
        }
    }

    #[test]
    fn activation_pattern_capture_storage_identity_is_stable() {
        let (mut i, trigger, r, topology) = load_enum_activation();
        let captures = r
            .arms
            .iter()
            .flat_map(|arm| arm.captures.iter())
            .map(|capture| (capture.id, capture.kind.clone(), capture.cell))
            .collect::<Vec<_>>();
        for (name, payload) in [("pressed", 10.), ("released", 20.), ("other", 30.)] {
            set_enum_event(&i, name, payload);
            i.advance_reactive_turn(&[trigger]).unwrap();
            let current = registration(&i)
                .arms
                .iter()
                .flat_map(|arm| arm.captures.iter())
                .map(|capture| (capture.id, capture.kind.clone(), capture.cell))
                .collect::<Vec<_>>();
            assert_eq!(current, captures);
            assert_eq!(plan_snapshot(&i), topology);
        }
    }

    #[cfg(all(feature = "matrix", feature = "f64"))]
    #[test]
    fn activation_pattern_expression_uses_outer_symbol_when_capture_name_collides() {
        let mut i = interpret(
            r#"
x := 9.0
event := [1.0 10.0]

~> event
  | [x, x + 1.0] => {
      selected := x
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
        assert_ne!(proposed_cell, committed_cell);
        assert_ne!(outer_cell, proposed_cell);
        assert_ne!(outer_cell, committed_cell);
        let topology = plan_snapshot(&i);

        let outcome = i.advance_reactive_turn(&[trigger]).unwrap();

        assert_dispatch_turn(&i, &topology, &outcome, 0, 1.0);
        assert_eq!(symbol(&i, "x"), Value::F64(Ref::new(9.0)));
        assert_eq!(proposed_capture_value(&i, 0, 0), Value::F64(Ref::new(1.0)));
        assert_eq!(committed_capture_value(&i, 0, 0), Value::F64(Ref::new(1.0)));
        assert_eq!(
            proposed_capture_value(&i, 0, 0).reactive_root_cell_ids()[0],
            proposed_cell
        );
        assert_eq!(registration(&i).arms[0].captures[0].cell, committed_cell);
    }

    #[cfg(all(feature = "matrix", feature = "f64"))]
    #[test]
    fn activation_array_pattern_samples_expression_only_on_trigger() {
        let mut i = interpret(
            r#"
event := [1.0 2.0 1.0]
threshold := 2.0
~> event
  | [x, threshold + 0.0, x] => {
      selected := x + 100.0
    }
  | * => {
      selected := -1.0
    }
"#,
        );
        let trigger = root_cell(&i, "event");
        let threshold_cell = root_cell(&i, "threshold");
        let topology = plan_snapshot(&i);
        let registration = registration(&i);

        let Value::F64(threshold) = symbol(&i, "threshold") else {
            panic!("threshold is not f64")
        };
        *threshold.borrow_mut() = 3.0;
        let dependency_turn = i.advance_reactive_turn(&[threshold_cell]).unwrap();
        let dependency_nodes = turn_executed_nodes(&dependency_turn);
        assert!(!dependency_nodes.contains(&registration.scope_pulse_node));
        assert!(!dependency_nodes.contains(&registration.selector_node));
        for arm in &registration.arms {
            assert!(!dependency_nodes.contains(&arm.matcher_node));
            assert!(!dependency_nodes.contains(&arm.finalizer_node));
            assert!(!dependency_nodes.contains(&arm.gate_node));
        }

        set_f64_matrix_event(&i, vec![4.0, 3.0, 4.0]);
        let outcome = i.advance_reactive_turn(&[trigger]).unwrap();
        assert_dispatch_turn(&i, &topology, &outcome, 0, 104.0);

        set_f64_matrix_event(&i, vec![4.0, 3.0, 5.0]);
        let outcome = i.advance_reactive_turn(&[trigger]).unwrap();
        assert_dispatch_turn(&i, &topology, &outcome, 1, -1.0);
    }

    #[cfg(all(feature = "matrix", feature = "f64"))]
    #[test]
    fn activation_pattern_samples_current_user_function_output_on_trigger() {
        let mut i = interpret(
            r#"
sample(value<f64>) => <f64>
  | value + 0.0.

event := [1.0 2.0 1.0]
threshold := 2.0
~> event
  | [x, sample(threshold), x] => {
      selected := x + 100.0
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

        let Value::F64(threshold) = symbol(&i, "threshold") else {
            panic!("threshold is not f64")
        };
        *threshold.borrow_mut() = 3.0;
        let dependency_turn = i.advance_reactive_turn(&[threshold_cell]).unwrap();
        let dependency_nodes = turn_executed_nodes(&dependency_turn);
        assert!(!dependency_nodes.contains(&activation.scope_pulse_node));
        assert!(!dependency_nodes.contains(&activation.selector_node));
        for arm in &activation.arms {
            assert!(!dependency_nodes.contains(&arm.matcher_node));
            assert!(!dependency_nodes.contains(&arm.finalizer_node));
            assert!(!dependency_nodes.contains(&arm.gate_node));
        }

        set_f64_matrix_event(&i, vec![4.0, 3.0, 4.0]);
        let outcome = i.advance_reactive_turn(&[trigger]).unwrap();
        assert_dispatch_turn(&i, &topology, &outcome, 0, 104.0);
    }

    #[test]
    fn activation_whole_composite_capture_is_stable_and_visible_to_the_body() {
        let mut i = interpret(
            r#"
event := (1.0, 2.0)
~> event
  | whole => {
      selected := whole
    }
  | * => {
      selected := (-1.0, -1.0)
    }
"#,
        );
        let trigger = root_cell(&i, "event");
        let activation = registration(&i);
        let capture = &activation.arms[0].captures[0];
        assert_eq!(capture.kind, ValueKind::Tuple(vec![ValueKind::F64, ValueKind::F64]));
        let body_inputs = i.plan().borrow().nodes
            [activation.arms[0].body_node_start..activation.arms[0].body_node_end]
            .iter()
            .flat_map(|node| node.inputs.iter().map(|dependency| dependency.cell))
            .collect::<Vec<_>>();
        assert!(
            body_inputs.contains(&capture.cell),
            "capture cell {:?} is absent from body inputs {:?}",
            capture.cell,
            body_inputs
        );
        let topology = plan_snapshot(&i);
        for values in [[3.0, 4.0], [5.0, 6.0]] {
            set_tuple_event(
                &i,
                values
                    .into_iter()
                    .map(|value| Value::F64(Ref::new(value)))
                    .collect(),
            );
            let outcome = i.advance_reactive_turn(&[trigger]).unwrap();
            assert_eq!(selected_arm_index(&activation, &outcome), 0);
            assert_eq!(
                body_output(&i, 0),
                Value::Tuple(Ref::new(MechTuple::from_vec(
                    values
                        .into_iter()
                        .map(|value| Value::F64(Ref::new(value)))
                        .collect(),
                )))
            );
            assert_eq!(registration(&i).arms[0].captures[0].cell, capture.cell);
            assert_eq!(plan_snapshot(&i), topology);
        }
    }

    #[test]
    fn activation_whole_tuple_capture_keeps_element_access_attached() {
        let mut i = interpret(
            r#"
event := (1.0, 2.0)
~> event
  | whole => {
      selected := whole.1 * 10.0 + whole.2
    }
  | * => {
      selected := -1.0
    }
"#,
        );
        let trigger = root_cell(&i, "event");
        let topology = plan_snapshot(&i);
        for (values, expected) in [([3.0, 4.0], 34.0), ([5.0, 6.0], 56.0)] {
            set_tuple_event(
                &i,
                values
                    .into_iter()
                    .map(|value| Value::F64(Ref::new(value)))
                    .collect(),
            );
            let outcome = i.advance_reactive_turn(&[trigger]).unwrap();
            assert_dispatch_turn(&i, &topology, &outcome, 0, expected);
        }
    }

    #[cfg(all(feature = "matrix", feature = "f64"))]
    #[test]
    fn activation_array_rest_capture_preserves_kind_payload_and_identity() {
        let mut i = interpret(
            r#"
event := [1.0 2.0 3.0 4.0 5.0]
~> event
  | [head | rest] => {
      selected := rest
    }
  | * => {
      selected := [-1.0]
    }
"#,
        );
        let trigger = root_cell(&i, "event");
        let activation = registration(&i);
        let rest_capture = &activation.arms[0].captures[1];
        assert_eq!(
            rest_capture.kind,
            ValueKind::Matrix(Box::new(ValueKind::F64), Vec::new())
        );
        assert!(
            i.plan().borrow().nodes
                [activation.arms[0].body_node_start..activation.arms[0].body_node_end]
                .iter()
                .any(|node| node
                    .inputs
                    .iter()
                    .any(|dependency| dependency.cell == rest_capture.cell))
        );
        let topology = plan_snapshot(&i);
        let Value::MatrixF64(event) = symbol(&i, "event") else {
            panic!("event is not an f64 matrix")
        };
        for values in [
            vec![10.0, 20.0, 30.0, 40.0, 50.0],
            vec![11.0, 21.0, 31.0, 41.0, 51.0, 61.0],
        ] {
            let source = Matrix::from_vec(values.clone(), 1, values.len());
            assert!(event.replace_payload_from(&source));
            let outcome = i.advance_reactive_turn(&[trigger]).unwrap();
            assert_eq!(selected_arm_index(&activation, &outcome), 0);
            let Value::MatrixF64(rest) = body_output(&i, 0) else {
                panic!("rest output is not an f64 matrix")
            };
            assert_eq!(rest.shape(), vec![1, values.len() - 1]);
            assert_eq!(rest.as_vec(), values[1..]);
            assert_eq!(
                registration(&i).arms[0].captures[1].cell,
                rest_capture.cell
            );
            assert_eq!(plan_snapshot(&i), topology);
        }
    }

    #[test]
    fn activation_pattern_capture_does_not_leak() {
        let (mut i, trigger, topology) = tuple_fixture(FLAT_TUPLE_ACTIVATION);
        for name in ["x", "y", "selected"] {
            assert!(!i.symbols().borrow().contains(hash_str(name)));
        }
        set_tuple_event(&i, vec![Value::F64(Ref::new(3.)), Value::F64(Ref::new(4.))]);
        let o = i.advance_reactive_turn(&[trigger]).unwrap();
        assert_dispatch_turn(&i, &topology, &o, 0, 34.);
    }

    #[test]
    fn activation_pattern_capture_shadows_and_restores_outer_symbol() {
        let mut i = interpret("event := (1.0, 2.0)\nx := 99.0");
        let outer = symbol_ref(&i, "x");
        let address = outer.addr();
        interpret_more(
            &mut i,
            "~> event\n  | (x, y) => {
      selected := x + y
    }\n  | * => {
      selected := -1.0
    }",
        )
        .unwrap();
        assert_eq!(*symbol(&i, "x").as_f64().unwrap().borrow(), 99.);
        assert_eq!(symbol_ref(&i, "x").addr(), address);
        assert!(!i.symbols().borrow().contains(hash_str("y")));
        assert!(!i.symbols().borrow().contains(hash_str("selected")));
        let topology = plan_snapshot(&i);
        let trigger = root_cell(&i, "event");
        set_tuple_event(&i, vec![Value::F64(Ref::new(3.)), Value::F64(Ref::new(4.))]);
        let o = i.advance_reactive_turn(&[trigger]).unwrap();
        assert_dispatch_turn(&i, &topology, &o, 0, 7.);
    }
