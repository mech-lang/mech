pub(super) use super::super::{
    ActivationPatternArmsNonExhaustive, ActivationPatternCapture,
    ActivationPatternCaptureKindUnsupported, ActivationPatternDefinitionUnsupported,
    ActivationPatternGuardMustBePure, ActivationPatternRegisterWriteUnsupported,
    ActivationPatternWildcardMustBeLast, ActivationScopeTriggerWriteUnsupported, Finalize, Gate,
    GuardFinalize, Matcher, ReactiveBindingSink, Select, UnmatchedFinalize, commit_capture_slot,
    create_capture_slot_for_kind, detached,
};
pub(super) use crate::patterns::PatternBindingSink;
#[cfg(feature = "compiler")]
pub(super) use crate::{BytecodeCompilerContext, MechFunctionCompiler, Register};
pub(super) use crate::{
    C64, CompiledPattern, Dictionary, FunctionExtensionEntry, FunctionSpecializer, GenericError,
    Interpreter, LegacyValue, MResult, Matrix, MechAtom, MechEnum, MechError, MechErrorKind,
    MechFunction, MechFunctionImpl, MechMap, MechRecord, MechSet, MechTable, MechTuple, Pattern,
    PatternActivationRegistration, PatternBinding, PatternMatch, R64, ReactiveCellId,
    ReactiveDependencyKind, ReactiveNodeId, ReactiveNodeKind, ReactiveRegisterCommit,
    ReactiveTurnOutcome, Ref, SymbolTableSnapshot, ValRef, ValueKind, hash_str,
};
pub(super) use std::collections::HashMap;
pub(super) use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

pub(super) struct EagerGuardTestSpecializer {
    pub(super) compile_calls: Arc<AtomicUsize>,
}

impl FunctionSpecializer for EagerGuardTestSpecializer {
    fn specialize(&self, _arguments: &[LegacyValue]) -> MResult<Box<dyn MechFunction>> {
        self.compile_calls.fetch_add(1, Ordering::SeqCst);
        panic!("unsupported guard specializer must not run during preflight")
    }
}

#[derive(Debug, Clone)]
pub(super) struct PatternRegisterStageFailure;
impl MechErrorKind for PatternRegisterStageFailure {
    fn name(&self) -> &str {
        "PatternRegisterStageFailure"
    }
    fn message(&self) -> String {
        "intentional patterned register staging failure".to_string()
    }
}

pub(super) struct FailingPatternRegister {
    sink: Ref<f64>,
    solve_calls: Arc<AtomicUsize>,
    stage_calls: Arc<AtomicUsize>,
}
impl MechFunctionImpl for FailingPatternRegister {
    fn solve_result(&self) -> MResult<()> {
        self.solve_calls.fetch_add(1, Ordering::SeqCst);
        *self.sink.borrow_mut() = -999.0;
        Ok(())
    }
    fn stage_register(&self) -> MResult<Box<dyn ReactiveRegisterCommit>> {
        self.stage_calls.fetch_add(1, Ordering::SeqCst);
        Err(MechError::new(PatternRegisterStageFailure, None))
    }
    fn out(&self) -> LegacyValue {
        LegacyValue::F64(self.sink.clone())
    }
    fn reactive_node_kind(&self) -> ReactiveNodeKind {
        ReactiveNodeKind::Register
    }
    fn to_string(&self) -> String {
        "FailingPatternRegister".to_string()
    }

    fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
        Ok(self.reactive_output_values())
    }
}
#[cfg(feature = "compiler")]
impl MechFunctionCompiler for FailingPatternRegister {
    fn compile(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        Err(MechError::new(PatternRegisterStageFailure, None))
    }
}

pub(super) struct FailingPatternRegisterSpecializer {
    pub(super) solve_calls: Arc<AtomicUsize>,
    pub(super) stage_calls: Arc<AtomicUsize>,
}
impl FunctionSpecializer for FailingPatternRegisterSpecializer {
    fn specialize(&self, arguments: &[LegacyValue]) -> MResult<Box<dyn MechFunction>> {
        let argument = arguments.first().ok_or_else(|| {
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

pub(super) fn install_function_extension(
    interpreter: &Interpreter,
    name: &str,
    specializer: Arc<dyn FunctionSpecializer>,
) {
    let entry = FunctionExtensionEntry::new(name, specializer);
    let extension = entry.id;
    let mut state = interpreter.state.borrow_mut();
    state.function_extensions.insert_or_replace(entry).unwrap();
    state
        .function_environment
        .bind_extension(name, name, extension)
        .unwrap();
}

pub(super) fn scalar_capture_cases() -> Vec<(ValueKind, LegacyValue)> {
    let mut cases = Vec::new();
    #[cfg(feature = "u8")]
    cases.push((ValueKind::U8, LegacyValue::U8(Ref::new(8))));
    #[cfg(feature = "u16")]
    cases.push((ValueKind::U16, LegacyValue::U16(Ref::new(16))));
    #[cfg(feature = "u32")]
    cases.push((ValueKind::U32, LegacyValue::U32(Ref::new(32))));
    #[cfg(feature = "u64")]
    cases.push((ValueKind::U64, LegacyValue::U64(Ref::new(64))));
    #[cfg(feature = "u128")]
    cases.push((ValueKind::U128, LegacyValue::U128(Ref::new(128))));
    #[cfg(feature = "i8")]
    cases.push((ValueKind::I8, LegacyValue::I8(Ref::new(-8))));
    #[cfg(feature = "i16")]
    cases.push((ValueKind::I16, LegacyValue::I16(Ref::new(-16))));
    #[cfg(feature = "i32")]
    cases.push((ValueKind::I32, LegacyValue::I32(Ref::new(-32))));
    #[cfg(feature = "i64")]
    cases.push((ValueKind::I64, LegacyValue::I64(Ref::new(-64))));
    #[cfg(feature = "i128")]
    cases.push((ValueKind::I128, LegacyValue::I128(Ref::new(-128))));
    #[cfg(feature = "f32")]
    cases.push((ValueKind::F32, LegacyValue::F32(Ref::new(3.25))));
    #[cfg(feature = "f64")]
    cases.push((ValueKind::F64, LegacyValue::F64(Ref::new(6.5))));
    #[cfg(feature = "complex")]
    cases.push((
        ValueKind::C64,
        LegacyValue::C64(Ref::new(C64::new(3.0, 4.0))),
    ));
    #[cfg(feature = "rational")]
    cases.push((ValueKind::R64, LegacyValue::R64(Ref::new(R64::new(3, 4)))));
    #[cfg(any(feature = "bool", feature = "variable_define"))]
    cases.push((ValueKind::Bool, LegacyValue::Bool(Ref::new(true))));
    #[cfg(any(feature = "string", feature = "variable_define"))]
    cases.push((
        ValueKind::String,
        LegacyValue::String(Ref::new("captured".to_string())),
    ));
    cases.push((ValueKind::Index, LegacyValue::Index(Ref::new(42))));
    #[cfg(feature = "atom")]
    {
        let atom = MechAtom::from_name("captured");
        cases.push((
            ValueKind::Atom(atom.id(), atom.name()),
            LegacyValue::Atom(Ref::new(atom)),
        ));
    }
    cases
}

pub(super) type PlanSnapshot = (
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

pub(super) fn interpret(source: &str) -> Interpreter {
    let tree = mech_syntax::parser::parse(source.trim_start()).unwrap();
    let mut interpreter = Interpreter::with_function_catalog(
        0,
        10_000,
        crate::test_support::catalog::function_catalog(),
    );
    interpreter.interpret(&tree).unwrap();
    interpreter
}

pub(super) fn interpret_more(interpreter: &mut Interpreter, source: &str) -> MResult<LegacyValue> {
    let tree = mech_syntax::parser::parse(source.trim_start()).unwrap();
    interpreter.interpret(&tree)
}

pub(super) fn symbol_ref(interpreter: &Interpreter, name: &str) -> ValRef {
    interpreter
        .symbols()
        .borrow()
        .get(hash_str(name))
        .unwrap_or_else(|| panic!("missing symbol `{name}`"))
}
pub(super) fn symbol(interpreter: &Interpreter, name: &str) -> LegacyValue {
    symbol_ref(interpreter, name).borrow().clone()
}
pub(super) fn root_cell(interpreter: &Interpreter, name: &str) -> ReactiveCellId {
    symbol(interpreter, name).reactive_root_cell_ids()[0]
}
pub(super) fn f64_symbol(interpreter: &Interpreter, name: &str) -> f64 {
    *symbol(interpreter, name)
        .as_f64()
        .unwrap_or_else(|_| panic!("symbol `{name}` is not f64"))
        .borrow()
}
pub(super) fn set_f64_symbol(interpreter: &Interpreter, name: &str, value: f64) {
    *symbol(interpreter, name)
        .as_f64()
        .unwrap_or_else(|_| panic!("symbol `{name}` is not f64"))
        .borrow_mut() = value;
}
pub(super) fn registration(interpreter: &Interpreter) -> PatternActivationRegistration {
    let plan = interpreter.plan();
    let registrations = plan.pattern_activation_registrations();
    assert_eq!(registrations.len(), 1);
    registrations[0].clone()
}
pub(super) fn node_output_for_cell(
    interpreter: &Interpreter,
    node: ReactiveNodeId,
    cell: ReactiveCellId,
) -> LegacyValue {
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
pub(super) fn committed_capture_value(
    interpreter: &Interpreter,
    arm: usize,
    capture: usize,
) -> LegacyValue {
    let registration = registration(interpreter);
    let arm = &registration.arms[arm];
    node_output_for_cell(interpreter, arm.gate_node, arm.captures[capture].cell)
}
pub(super) fn proposed_capture_value(
    interpreter: &Interpreter,
    arm: usize,
    capture: usize,
) -> LegacyValue {
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
pub(super) fn arm_pulse_generation(interpreter: &Interpreter, arm: usize) -> usize {
    let registration = registration(interpreter);
    let arm = &registration.arms[arm];
    let LegacyValue::Index(generation) =
        node_output_for_cell(interpreter, arm.gate_node, arm.pulse_cell)
    else {
        panic!("activation arm pulse is not an index")
    };
    let value = *generation.borrow();
    value
}
pub(super) fn arm_register_nodes(
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
pub(super) fn plan_snapshot(interpreter: &Interpreter) -> PlanSnapshot {
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
pub(super) fn turn_executed_nodes(outcome: &ReactiveTurnOutcome) -> Vec<ReactiveNodeId> {
    outcome
        .before_commit
        .executed_nodes
        .iter()
        .chain(outcome.after_commit.executed_nodes.iter())
        .copied()
        .collect()
}
pub(super) fn turn_changed_nodes(outcome: &ReactiveTurnOutcome) -> Vec<ReactiveNodeId> {
    outcome
        .before_commit
        .changed_nodes
        .iter()
        .chain(outcome.after_commit.changed_nodes.iter())
        .copied()
        .collect()
}
pub(super) fn turn_unchanged_nodes(outcome: &ReactiveTurnOutcome) -> Vec<ReactiveNodeId> {
    outcome
        .before_commit
        .unchanged_nodes
        .iter()
        .chain(outcome.after_commit.unchanged_nodes.iter())
        .copied()
        .collect()
}
pub(super) fn body_output_f64(interpreter: &Interpreter, arm_index: usize) -> f64 {
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
pub(super) fn body_output(interpreter: &Interpreter, arm_index: usize) -> LegacyValue {
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
pub(super) fn set_enum_event(interpreter: &Interpreter, variant: &str, payload: f64) {
    let LegacyValue::Enum(event) = symbol(interpreter, "event") else {
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
        variants: vec![(hash_str(variant), Some(LegacyValue::F64(Ref::new(payload))))],
        names,
    };
}
pub(super) fn set_unit_enum_event(interpreter: &Interpreter, variant: &str) {
    let event_value = symbol(interpreter, "event");
    if let LegacyValue::Atom(event) = &event_value {
        *event.borrow_mut() = MechAtom::from_name(variant);
        return;
    }
    let LegacyValue::Enum(event) = event_value else {
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
pub(super) fn set_atom_tuple_event(interpreter: &Interpreter, tag: &str, payload: f64) {
    let LegacyValue::Tuple(event) = symbol(interpreter, "event") else {
        panic!("event is not tuple")
    };
    *event.borrow_mut() = MechTuple::from_vec(vec![
        LegacyValue::Atom(Ref::new(MechAtom::from_name(tag))),
        LegacyValue::F64(Ref::new(payload)),
    ]);
}
pub(super) fn set_tuple_event(interpreter: &Interpreter, values: Vec<LegacyValue>) {
    let LegacyValue::Tuple(event) = symbol(interpreter, "event") else {
        panic!("event is not tuple")
    };
    *event.borrow_mut() = MechTuple::from_vec(values);
}
#[cfg(all(feature = "matrix", feature = "f64"))]
pub(super) fn set_f64_matrix_event(interpreter: &Interpreter, values: Vec<f64>) {
    let LegacyValue::MatrixF64(event) = symbol(interpreter, "event") else {
        panic!("event is not an f64 matrix")
    };
    event.set(values);
}
pub(super) fn selected_arm_index(
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
pub(super) fn assert_dispatch_turn(
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

pub(super) const ENUM_ACTIVATION: &str = r#"
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

pub(super) fn load_enum_activation() -> (
    Interpreter,
    ReactiveCellId,
    PatternActivationRegistration,
    PlanSnapshot,
) {
    let interpreter = interpret(ENUM_ACTIVATION);
    assert!(matches!(
        symbol(&interpreter, "event"),
        LegacyValue::Enum(_)
    ));
    let enum_id = match symbol(&interpreter, "event") {
        LegacyValue::Enum(event) => event.borrow().id,
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

pub(super) const ATOM_TUPLE_ACTIVATION: &str = r#"
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
pub(super) fn load_atom_tuple_activation() -> (
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
pub(super) const FLAT_TUPLE_ACTIVATION: &str = r#"
event := (1.0, 2.0)
~> event
  | (x, y) => {
      selected := x * 10.0 + y
    }
  | * => {
      selected := -1.0
    }
"#;
pub(super) const NESTED_TUPLE_ACTIVATION: &str = r#"
event := ((1.0, 2.0), 3.0)
~> event
  | ((x, y), z) => {
      selected := x * 100.0 + y * 10.0 + z
    }
  | * => {
      selected := -1.0
    }
"#;
pub(super) const REPEATED_CAPTURE_ACTIVATION: &str = r#"
event := (1.0, 1.0)
~> event
  | (x, x) => {
      selected := x
    }
  | * => {
      selected := -1.0
    }
"#;
pub(super) fn tuple_fixture(source: &str) -> (Interpreter, ReactiveCellId, PlanSnapshot) {
    let i = interpret(source);
    let trigger = root_cell(&i, "event");
    let topology = plan_snapshot(&i);
    (i, trigger, topology)
}
pub(super) fn failed_elaboration_fixture() -> (
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
pub(super) fn assert_failed_elaboration_restored() -> (
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
