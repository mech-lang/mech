    use super::*;
    mod registration;
    use std::collections::HashMap;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    struct EagerGuardTestCompiler {
        compile_calls: Arc<AtomicUsize>,
    }

    impl NativeFunctionCompiler for EagerGuardTestCompiler {
        fn compile(&self, _arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
            self.compile_calls.fetch_add(1, Ordering::SeqCst);
            panic!("unsupported guard compiler must not run during preflight")
        }
    }

    #[derive(Debug, Clone)]
    struct PatternRegisterStageFailure;
    impl MechErrorKind for PatternRegisterStageFailure {
        fn name(&self) -> &str {
            "PatternRegisterStageFailure"
        }
        fn message(&self) -> String {
            "intentional patterned register staging failure".to_string()
        }
    }

    struct FailingPatternRegister {
        sink: Ref<f64>,
        solve_calls: Arc<AtomicUsize>,
        stage_calls: Arc<AtomicUsize>,
    }
    impl MechFunctionImpl for FailingPatternRegister {
        fn solve(&self) {
            self.solve_calls.fetch_add(1, Ordering::SeqCst);
            *self.sink.borrow_mut() = -999.0;
        }
        fn stage_register(&self) -> MResult<Box<dyn ReactiveRegisterCommit>> {
            self.stage_calls.fetch_add(1, Ordering::SeqCst);
            Err(MechError::new(PatternRegisterStageFailure, None))
        }
        fn out(&self) -> Value {
            Value::F64(self.sink.clone())
        }
        fn reactive_node_kind(&self) -> ReactiveNodeKind {
            ReactiveNodeKind::Register
        }
        fn to_string(&self) -> String {
            "FailingPatternRegister".to_string()
        }

      fn transaction_state_values(&self) -> MResult<Vec<Value>> {
        Ok(self.reactive_output_values())
      }
    }
    #[cfg(feature = "compiler")]
    impl MechFunctionCompiler for FailingPatternRegister {
        fn compile(&self, _ctx: &mut CompileCtx) -> MResult<Register> {
            Err(MechError::new(PatternRegisterStageFailure, None))
        }
    }

    struct FailingPatternRegisterCompiler {
        solve_calls: Arc<AtomicUsize>,
        stage_calls: Arc<AtomicUsize>,
    }
    impl NativeFunctionCompiler for FailingPatternRegisterCompiler {
        fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
            let argument = arguments
                .first()
                .ok_or_else(|| {
                    MechError::new(
                        GenericError {
                            msg: "failing pattern register expects one f64 sink".to_string(),
                        },
                        None,
                    )
                })?;
            let sink = argument.as_f64()?;
            Ok(Box::new(FailingPatternRegister {
                sink,
                solve_calls: self.solve_calls.clone(),
                stage_calls: self.stage_calls.clone(),
            }))
        }
    }

    fn scalar_capture_cases() -> Vec<(ValueKind, Value)> {
        let mut cases = Vec::new();
        #[cfg(feature = "u8")]
        cases.push((ValueKind::U8, Value::U8(Ref::new(8))));
        #[cfg(feature = "u16")]
        cases.push((ValueKind::U16, Value::U16(Ref::new(16))));
        #[cfg(feature = "u32")]
        cases.push((ValueKind::U32, Value::U32(Ref::new(32))));
        #[cfg(feature = "u64")]
        cases.push((ValueKind::U64, Value::U64(Ref::new(64))));
        #[cfg(feature = "u128")]
        cases.push((ValueKind::U128, Value::U128(Ref::new(128))));
        #[cfg(feature = "i8")]
        cases.push((ValueKind::I8, Value::I8(Ref::new(-8))));
        #[cfg(feature = "i16")]
        cases.push((ValueKind::I16, Value::I16(Ref::new(-16))));
        #[cfg(feature = "i32")]
        cases.push((ValueKind::I32, Value::I32(Ref::new(-32))));
        #[cfg(feature = "i64")]
        cases.push((ValueKind::I64, Value::I64(Ref::new(-64))));
        #[cfg(feature = "i128")]
        cases.push((ValueKind::I128, Value::I128(Ref::new(-128))));
        #[cfg(feature = "f32")]
        cases.push((ValueKind::F32, Value::F32(Ref::new(3.25))));
        #[cfg(feature = "f64")]
        cases.push((ValueKind::F64, Value::F64(Ref::new(6.5))));
        #[cfg(feature = "complex")]
        cases.push((ValueKind::C64, Value::C64(Ref::new(C64::new(3.0, 4.0)))));
        #[cfg(feature = "rational")]
        cases.push((ValueKind::R64, Value::R64(Ref::new(R64::new(3, 4)))));
        #[cfg(any(feature = "bool", feature = "variable_define"))]
        cases.push((ValueKind::Bool, Value::Bool(Ref::new(true))));
        #[cfg(any(feature = "string", feature = "variable_define"))]
        cases.push((
            ValueKind::String,
            Value::String(Ref::new("captured".to_string())),
        ));
        cases.push((ValueKind::Index, Value::Index(Ref::new(42))));
        #[cfg(feature = "atom")]
        {
            let atom = MechAtom::from_name("captured");
            cases.push((
                ValueKind::Atom(atom.id(), atom.name()),
                Value::Atom(Ref::new(atom)),
            ));
        }
        cases
    }

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

    type PlanSnapshot = (
        usize,
        Vec<(
            ReactiveNodeId,
            usize,
            ReactiveNodeKind,
            Vec<u64>,
            Vec<(u64, ReactiveDependencyKind)>,
        )>,
        Vec<(u64, Vec<ReactiveNodeId>)>,
        Vec<(u64, Vec<ReactiveNodeId>)>,
        Vec<PatternActivationRegistration>,
        usize,
    );

    fn interpret(source: &str) -> Interpreter {
        let tree = mech_syntax::parser::parse(source.trim_start()).unwrap();
        let mut interpreter = Interpreter::new_with_full_stdlib(0);
        interpreter.interpret(&tree).unwrap();
        interpreter
    }

    fn interpret_more(interpreter: &mut Interpreter, source: &str) -> MResult<Value> {
        let tree = mech_syntax::parser::parse(source.trim_start()).unwrap();
        interpreter.interpret(&tree)
    }

    fn symbol_ref(interpreter: &Interpreter, name: &str) -> ValRef {
        interpreter
            .symbols()
            .borrow()
            .get(hash_str(name))
            .unwrap_or_else(|| panic!("missing symbol `{name}`"))
    }
    fn symbol(interpreter: &Interpreter, name: &str) -> Value {
        symbol_ref(interpreter, name).borrow().clone()
    }
    fn root_cell(interpreter: &Interpreter, name: &str) -> ReactiveCellId {
        symbol(interpreter, name).reactive_root_cell_ids()[0]
    }
    fn f64_symbol(interpreter: &Interpreter, name: &str) -> f64 {
        *symbol(interpreter, name)
            .as_f64()
            .unwrap_or_else(|_| panic!("symbol `{name}` is not f64"))
            .borrow()
    }
    fn set_f64_symbol(interpreter: &Interpreter, name: &str, value: f64) {
        *symbol(interpreter, name)
            .as_f64()
            .unwrap_or_else(|_| panic!("symbol `{name}` is not f64"))
            .borrow_mut() = value;
    }
    fn registration(interpreter: &Interpreter) -> PatternActivationRegistration {
        let plan = interpreter.plan();
        let registrations = plan.pattern_activation_registrations();
        assert_eq!(registrations.len(), 1);
        registrations[0].clone()
    }
    fn node_output_for_cell(
        interpreter: &Interpreter,
        node: ReactiveNodeId,
        cell: ReactiveCellId,
    ) -> Value {
        let plan = interpreter.plan();
        let plan = plan.borrow();
        plan.node(node)
            .expect("missing activation dispatch node")
            .function
            .reactive_output_values()
            .into_iter()
            .find(|value| value.reactive_root_cell_ids().contains(&cell))
            .unwrap_or_else(|| panic!("node {node} does not expose cell {cell:?}"))
    }
    fn committed_capture_value(interpreter: &Interpreter, arm: usize, capture: usize) -> Value {
        let registration = registration(interpreter);
        let arm = &registration.arms[arm];
        node_output_for_cell(interpreter, arm.gate_node, arm.captures[capture].cell)
    }
    fn proposed_capture_value(interpreter: &Interpreter, arm: usize, capture: usize) -> Value {
        let registration = registration(interpreter);
        let arm = &registration.arms[arm];
        arm.captures
            .get(capture)
            .expect("missing capture registration");
        let plan = interpreter.plan();
        let plan = plan.borrow();
        plan.node(arm.matcher_node)
            .expect("missing activation matcher")
            .function
            .reactive_output_values()
            .into_iter()
            .skip(1)
            .nth(capture)
            .expect("missing proposed capture output")
    }
    fn arm_pulse_generation(interpreter: &Interpreter, arm: usize) -> usize {
        let registration = registration(interpreter);
        let arm = &registration.arms[arm];
        let Value::Index(generation) =
            node_output_for_cell(interpreter, arm.gate_node, arm.pulse_cell)
        else {
            panic!("activation arm pulse is not an index")
        };
        let value = *generation.borrow();
        value
    }
    fn arm_register_nodes(
        interpreter: &Interpreter,
        registration: &PatternActivationRegistration,
        arm: usize,
    ) -> Vec<ReactiveNodeId> {
        let arm = &registration.arms[arm];
        let plan = interpreter.plan();
        let plan = plan.borrow();
        plan.nodes[arm.body_node_start..arm.body_node_end]
            .iter()
            .filter(|node| node.kind == ReactiveNodeKind::Register)
            .map(|node| node.id)
            .collect()
    }
    fn plan_snapshot(interpreter: &Interpreter) -> PlanSnapshot {
        let plan = interpreter.plan();
        let depth = plan.activation_registration_depth();
        let plan = plan.borrow();
        let nodes = plan
            .nodes
            .iter()
            .map(|node| {
                (
                    node.id,
                    node.plan_index,
                    node.kind,
                    node.outputs.iter().map(|cell| cell.get()).collect(),
                    node.inputs
                        .iter()
                        .map(|dependency| (dependency.cell.get(), dependency.kind))
                        .collect(),
                )
            })
            .collect();
        let mut reactive = plan
            .reactive_consumers
            .iter()
            .map(|(cell, nodes)| (cell.get(), nodes.clone()))
            .collect::<Vec<_>>();
        reactive.sort_by_key(|(cell, _)| *cell);
        let mut sampled = plan
            .sampled_consumers
            .iter()
            .map(|(cell, nodes)| (cell.get(), nodes.clone()))
            .collect::<Vec<_>>();
        sampled.sort_by_key(|(cell, _)| *cell);
        (
            plan.len(),
            nodes,
            reactive,
            sampled,
            plan.pattern_activation_registrations().to_vec(),
            depth,
        )
    }
    fn turn_executed_nodes(outcome: &ReactiveTurnOutcome) -> Vec<ReactiveNodeId> {
        outcome
            .before_commit
            .executed_nodes
            .iter()
            .chain(outcome.after_commit.executed_nodes.iter())
            .copied()
            .collect()
    }
    fn turn_changed_nodes(outcome: &ReactiveTurnOutcome) -> Vec<ReactiveNodeId> {
        outcome
            .before_commit
            .changed_nodes
            .iter()
            .chain(outcome.after_commit.changed_nodes.iter())
            .copied()
            .collect()
    }
    fn turn_unchanged_nodes(outcome: &ReactiveTurnOutcome) -> Vec<ReactiveNodeId> {
        outcome
            .before_commit
            .unchanged_nodes
            .iter()
            .chain(outcome.after_commit.unchanged_nodes.iter())
            .copied()
            .collect()
    }
    fn body_output_f64(interpreter: &Interpreter, arm_index: usize) -> f64 {
        let registration = registration(interpreter);
        let arm = &registration.arms[arm_index];
        let plan = interpreter.plan();
        let plan = plan.borrow();
        for id in (arm.body_node_start..arm.body_node_end).rev() {
            if let Ok(value) = plan.node(id).unwrap().function.out().as_f64() {
                return *value.borrow();
            }
        }
        panic!("no f64 output")
    }
    fn body_output(interpreter: &Interpreter, arm_index: usize) -> Value {
        let registration = registration(interpreter);
        let arm = &registration.arms[arm_index];
        let plan = interpreter.plan();
        let plan = plan.borrow();
        detached(
            &plan
                .node(arm.body_node_end - 1)
                .expect("missing activation body node")
                .function
                .out(),
        )
    }
    fn set_enum_event(interpreter: &Interpreter, variant: &str, payload: f64) {
        let Value::Enum(event) = symbol(interpreter, "event") else {
            panic!("event is not an enum");
        };
        let enum_id = event.borrow().id;
        let names = interpreter
            .state
            .borrow()
            .enums
            .get(&enum_id)
            .expect("event enum definition is missing")
            .names
            .clone();
        *event.borrow_mut() = MechEnum {
            id: enum_id,
            variants: vec![(hash_str(variant), Some(Value::F64(Ref::new(payload))))],
            names,
        };
    }
    fn set_unit_enum_event(interpreter: &Interpreter, variant: &str) {
        let event_value = symbol(interpreter, "event");
        if let Value::Atom(event) = &event_value {
            *event.borrow_mut() = MechAtom::from_name(variant);
            return;
        }
        let Value::Enum(event) = event_value else {
            panic!("event is neither an atom nor an enum");
        };
        let enum_id = event.borrow().id;
        let names = interpreter
            .state
            .borrow()
            .enums
            .get(&enum_id)
            .expect("event enum definition is missing")
            .names
            .clone();
        *event.borrow_mut() = MechEnum {
            id: enum_id,
            variants: vec![(hash_str(variant), None)],
            names,
        };
    }
    fn set_atom_tuple_event(interpreter: &Interpreter, tag: &str, payload: f64) {
        let Value::Tuple(event) = symbol(interpreter, "event") else {
            panic!("event is not tuple")
        };
        *event.borrow_mut() = MechTuple::from_vec(vec![
            Value::Atom(Ref::new(MechAtom::from_name(tag))),
            Value::F64(Ref::new(payload)),
        ]);
    }
    fn set_tuple_event(interpreter: &Interpreter, values: Vec<Value>) {
        let Value::Tuple(event) = symbol(interpreter, "event") else {
            panic!("event is not tuple")
        };
        *event.borrow_mut() = MechTuple::from_vec(values);
    }
    #[cfg(all(feature = "matrix", feature = "f64"))]
    fn set_f64_matrix_event(interpreter: &Interpreter, values: Vec<f64>) {
        let Value::MatrixF64(event) = symbol(interpreter, "event") else {
            panic!("event is not an f64 matrix")
        };
        event.set(values);
    }
    fn selected_arm_index(
        registration: &PatternActivationRegistration,
        outcome: &ReactiveTurnOutcome,
    ) -> usize {
        let changed = turn_changed_nodes(outcome);
        registration
            .arms
            .iter()
            .position(|arm| changed.contains(&arm.gate_node))
            .expect("no selected gate")
    }
    fn assert_dispatch_turn(
        interpreter: &Interpreter,
        topology: &PlanSnapshot,
        outcome: &ReactiveTurnOutcome,
        expected_arm: usize,
        output: f64,
    ) {
        let registration = registration(interpreter);
        let executed = turn_executed_nodes(outcome);
        let changed = turn_changed_nodes(outcome);
        let unchanged = turn_unchanged_nodes(outcome);
        assert_eq!(
            executed
                .iter()
                .filter(|id| **id == registration.scope_pulse_node)
                .count(),
            1
        );
        assert_eq!(
            executed
                .iter()
                .filter(|id| **id == registration.selector_node)
                .count(),
            1
        );
        assert_eq!(selected_arm_index(&registration, outcome), expected_arm);
        for (index, arm) in registration.arms.iter().enumerate() {
            for node in [arm.matcher_node, arm.finalizer_node, arm.gate_node] {
                assert_eq!(executed.iter().filter(|id| **id == node).count(), 1);
            }
            if index == expected_arm {
                assert!(changed.contains(&arm.gate_node));
                assert!(!unchanged.contains(&arm.gate_node));
                for node in arm.body_node_start..arm.body_node_end {
                    assert_eq!(executed.iter().filter(|id| **id == node).count(), 1);
                }
            } else {
                assert!(unchanged.contains(&arm.gate_node));
                assert!(!changed.contains(&arm.gate_node));
                for node in arm.body_node_start..arm.body_node_end {
                    assert!(!executed.contains(&node));
                }
            }
        }
        assert_eq!(body_output_f64(interpreter, expected_arm), output);
        assert_eq!(&plan_snapshot(interpreter), topology);
    }

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
                &format!(
                    "~> event\n  | {pattern} => {{\n      selected := 1.0\n    }}"
                ),
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
            assert!(!first.before_commit.pending_register_nodes.contains(register));
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
        assert_eq!(second.before_commit.pending_register_nodes, arm_registers[1]);
        assert_eq!(second.register_commit.staged_nodes, arm_registers[1]);
        assert_eq!(second.register_commit.committed_nodes, arm_registers[1]);
        assert_eq!(
            (f64_symbol(&interpreter, "x"), f64_symbol(&interpreter, "y")),
            (10.0, 6.0)
        );
        assert_eq!(
            committed_capture_value(&interpreter, 1, 0),
            Value::F64(Ref::new(5.0))
        );
        for register in arm_registers[0].iter().chain(&arm_registers[2]) {
            assert!(!second.before_commit.pending_register_nodes.contains(register));
            assert!(!second.register_commit.staged_nodes.contains(register));
            assert!(!second.register_commit.committed_nodes.contains(register));
            assert!(!second.after_commit.pending_register_nodes.contains(register));
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

    #[test]
    fn activation_arm_alias_of_live_input_remains_sampled_until_trigger() {
        let mut interpreter = Interpreter::new_with_full_stdlib(0);
        let outer_id = hash_str("outer");
        {
            let symbols = interpreter.symbols();
            let mut symbols = symbols.borrow_mut();
            symbols.insert(outer_id, Value::F64(Ref::new(1.0)), true);
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
                dependency.cell == outer
                    && dependency.kind == ReactiveDependencyKind::Sampled
            }));
            assert!(register.inputs.iter().any(|dependency| {
                dependency.cell == activation.arms[0].pulse_cell
                    && dependency.kind == ReactiveDependencyKind::Reactive
            }));
        }

        set_f64_symbol(&interpreter, "outer", 5.0);
        let sampled_only = interpreter.advance_reactive_turn(&[outer]).unwrap();
        assert!(!sampled_only
            .before_commit
            .pending_register_nodes
            .contains(&registers[0]));
        assert!(!sampled_only
            .register_commit
            .committed_nodes
            .contains(&registers[0]));
        assert_eq!(f64_symbol(&interpreter, "state"), 0.0);

        let tick = interpreter.advance_reactive_turn(&[trigger]).unwrap();
        assert_eq!(selected_arm_index(&activation, &tick), 0);
        assert_eq!(tick.register_commit.committed_nodes, registers);
        assert_eq!(f64_symbol(&interpreter, "state"), 5.0);
        assert_eq!(plan_snapshot(&interpreter), topology);
    }

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
        interpreter
            .functions()
            .borrow_mut()
            .insert_function_compiler(
                "test/failing-pattern-register",
                Arc::new(FailingPatternRegisterCompiler {
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
            (f64_symbol(&interpreter, "first"), f64_symbol(&interpreter, "second")),
            (1.0, 2.0)
        );
        assert_eq!(solve_calls.load(Ordering::SeqCst), 0);

        set_f64_symbol(&interpreter, "event", 9.0);
        let error = interpreter.advance_reactive_turn(&[trigger]).unwrap_err();
        assert_eq!(error.kind_name(), "PatternRegisterStageFailure");
        assert_eq!(stage_calls.load(Ordering::SeqCst), 1);
        assert_eq!(solve_calls.load(Ordering::SeqCst), 0);
        assert_eq!(
            (f64_symbol(&interpreter, "first"), f64_symbol(&interpreter, "second")),
            (1.0, 2.0)
        );
        assert_eq!(plan_snapshot(&interpreter), topology);
    }

    const ENUM_ACTIVATION: &str = r#"
<event-kind> := :pressed<f64>
  | :released<f64>
  | :other<f64>

event<event-kind> := :pressed(0.0)

~> event
  | :pressed(x) => {
      selected := x + 0.0
    }
  | :released(x) => {
      selected := x + 1000.0
    }
  | * => {
      selected := -1.0
    }
"#;

    fn load_enum_activation() -> (
        Interpreter,
        ReactiveCellId,
        PatternActivationRegistration,
        PlanSnapshot,
    ) {
        let interpreter = interpret(ENUM_ACTIVATION);
        assert!(matches!(symbol(&interpreter, "event"), Value::Enum(_)));
        let enum_id = match symbol(&interpreter, "event") {
            Value::Enum(event) => event.borrow().id,
            value => panic!("expected enum event, found {:?}", value.kind()),
        };
        let enum_definition = interpreter
            .state
            .borrow()
            .enums
            .get(&enum_id)
            .cloned()
            .expect("event enum definition is missing");
        for variant in ["pressed", "released", "other"] {
            assert!(
                enum_definition
                    .variants
                    .iter()
                    .any(|(variant_id, _)| *variant_id == hash_str(variant)),
                "missing enum variant `{variant}`"
            );
        }
        let trigger = root_cell(&interpreter, "event");
        let registration = registration(&interpreter);
        assert_eq!(registration.arms.len(), 3);
        assert_eq!(registration.arms[0].captures.len(), 1);
        assert_eq!(registration.arms[1].captures.len(), 1);
        assert_eq!(registration.arms[0].captures[0].kind, ValueKind::F64);
        assert_eq!(registration.arms[1].captures[0].kind, ValueKind::F64);
        assert!(registration.arms[2].captures.is_empty());
        assert!(!interpreter.symbols().borrow().contains(hash_str("x")));
        assert!(
            !interpreter
                .symbols()
                .borrow()
                .contains(hash_str("selected"))
        );
        let topology = plan_snapshot(&interpreter);
        (interpreter, trigger, registration, topology)
    }

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

    const ATOM_TUPLE_ACTIVATION: &str = r#"
event := (:pressed, 0.0)
~> event
  | :pressed(x) => {
      selected := x + 0.0
    }
  | :released(x) => {
      selected := x + 1000.0
    }
  | * => {
      selected := -1.0
    }
"#;
    fn load_atom_tuple_activation() -> (
        Interpreter,
        ReactiveCellId,
        PatternActivationRegistration,
        PlanSnapshot,
    ) {
        let i = interpret(ATOM_TUPLE_ACTIVATION);
        let trigger = root_cell(&i, "event");
        let r = registration(&i);
        let topology = plan_snapshot(&i);
        (i, trigger, r, topology)
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
    fn activation_pattern_atom_tuple_arms_compile_independent_of_initial_tag() {
        let (mut i, trigger, r, topology) = load_atom_tuple_activation();
        assert_eq!(r.arms[0].captures[0].kind, ValueKind::F64);
        assert_eq!(r.arms[1].captures[0].kind, ValueKind::F64);
        set_atom_tuple_event(&i, "released", 20.);
        let o = i.advance_reactive_turn(&[trigger]).unwrap();
        assert_dispatch_turn(&i, &topology, &o, 1, 1020.);
    }

    const FLAT_TUPLE_ACTIVATION: &str = r#"
event := (1.0, 2.0)
~> event
  | (x, y) => {
      selected := x * 10.0 + y
    }
  | * => {
      selected := -1.0
    }
"#;
    const NESTED_TUPLE_ACTIVATION: &str = r#"
event := ((1.0, 2.0), 3.0)
~> event
  | ((x, y), z) => {
      selected := x * 100.0 + y * 10.0 + z
    }
  | * => {
      selected := -1.0
    }
"#;
    const REPEATED_CAPTURE_ACTIVATION: &str = r#"
event := (1.0, 1.0)
~> event
  | (x, x) => {
      selected := x
    }
  | * => {
      selected := -1.0
    }
"#;
    fn tuple_fixture(source: &str) -> (Interpreter, ReactiveCellId, PlanSnapshot) {
        let i = interpret(source);
        let trigger = root_cell(&i, "event");
        let topology = plan_snapshot(&i);
        (i, trigger, topology)
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

    #[test]
    fn activation_unmatched_guard_skips_runtime_error_and_guard_error_commits_nothing() {
        let mut i = interpret(
            r#"
event := (:pressed, 1.0)
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

        let Value::F64(index) = symbol(&i, "index") else {
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
        assert_eq!(committed_capture_value(&i, 0, 0), Value::F64(Ref::new(3.0)));
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
    fn failed_elaboration_fixture() -> (
        Interpreter,
        SymbolTableSnapshot,
        Dictionary,
        PlanSnapshot,
        ValRef,
        usize,
    ) {
        let i = interpret("event := (1.0, 2.0)\nouter := 99.0");
        let symbols = i.symbols().borrow().snapshot();
        let dictionary = i.dictionary().borrow().clone();
        let topology = plan_snapshot(&i);
        let outer = symbol_ref(&i, "outer");
        let address = outer.addr();
        (i, symbols, dictionary, topology, outer, address)
    }
    fn assert_failed_elaboration_restored() -> (
        Interpreter,
        SymbolTableSnapshot,
        Dictionary,
        PlanSnapshot,
        usize,
    ) {
        let (mut i, symbols, dictionary, topology, outer, address) = failed_elaboration_fixture();
        let error=interpret_more(&mut i,"~> event\n  | (x, y) => {\n      local-atom := :temporary\n      local-first := x + y\n      local-failure := function-that-does-not-exist(local-first)\n    }\n  | * => {
      fallback := 0.0
    }").unwrap_err();
        assert!(error.kind_name().contains("Function"));
        assert!(!i.dictionary().borrow().contains_key(&hash_str("temporary")));
        for name in [
            "local-atom",
            "local-first",
            "local-failure",
            "fallback",
            "x",
            "y",
        ] {
            assert!(!i.symbols().borrow().contains(hash_str(name)));
        }
        assert_eq!(*symbol(&i, "outer").as_f64().unwrap().borrow(), 99.);
        assert_eq!(symbol_ref(&i, "outer").addr(), address);
        drop(outer);
        (i, symbols, dictionary, topology, address)
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
