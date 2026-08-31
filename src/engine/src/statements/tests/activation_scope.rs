use crate::{
    CanonicalCellId, CanonicalFunctionSpecializer, FunctionExtensionEntry, GuardFunctionSafety,
    Interpreter, MResult, ReactiveDependencyKind, ReactiveNodeId, ReactiveNodeKind,
    SpecializationContext, SpecializationInvocation, SpecializedFunction, hash_str,
};
use std::collections::HashSet;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

const PURE: &str = "tick := 0.0\nx := 10.0\nradius := 2.0\n~> tick {\n left := x - radius\n doubled := left * 2.0\n}";
const REGISTER: &str = "tick := 0.0\n~x := 10.0\n\n~> tick {\n  next-x := x + 1.0\n  x = next-x\n}";
const TWO_REGISTERS: &str = "tick := 0.0\n\n~x := 0.0\n~y := 0.0\n\n~> tick {\n  next-x := x + 1.0\n  next-y := y + 2.0\n\n  x = next-x\n  y = next-y\n}";
fn interpret(source: &str) -> Interpreter {
    let t = mech_syntax::parser::parse(source).unwrap();
    let mut i = Interpreter::with_function_catalog(
        0,
        10_000,
        crate::test_support::catalog::function_catalog(),
    );
    i.interpret(&t).unwrap();
    i
}
fn load() -> Interpreter {
    interpret(PURE)
}
fn cell(i: &Interpreter, n: &str) -> CanonicalCellId {
    i.symbols()
        .borrow()
        .get(hash_str(n))
        .unwrap()
        .reactive_cell_id()
}
fn value(i: &Interpreter, n: &str) -> f64 {
    let value = i
        .symbols()
        .borrow()
        .get(hash_str(n))
        .unwrap()
        .snapshot()
        .unwrap();
    let crate::ValueData::F64(value) = value.data() else {
        panic!("symbol {n} is not f64")
    };
    value.to_f64()
}
fn set_value(i: &Interpreter, n: &str, replacement: f64) {
    let replacement = crate::ValueCell::from_exact(replacement)
        .unwrap()
        .snapshot()
        .unwrap();
    i.symbols()
        .borrow()
        .get(hash_str(n))
        .unwrap_or_else(|| panic!("missing symbol {n}"))
        .replace(&replacement)
        .unwrap();
}
fn nodes_for_output(i: &Interpreter, name: &str, kind: ReactiveNodeKind) -> Vec<ReactiveNodeId> {
    let output = cell(i, name);
    let p = i.plan();
    p.borrow()
        .nodes
        .iter()
        .filter(|n| n.kind == kind && n.outputs.contains(&output))
        .map(|n| n.id)
        .collect()
}
fn unique_register_for(i: &Interpreter, name: &str) -> ReactiveNodeId {
    let found = nodes_for_output(i, name, ReactiveNodeKind::Register);
    assert_eq!(found.len(), 1, "expected one register node for {name}");
    found[0]
}
fn activation_nodes(
    i: &Interpreter,
    trigger_name: &str,
    kind: ReactiveNodeKind,
) -> Vec<ReactiveNodeId> {
    let trigger = cell(i, trigger_name);
    let p = i.plan();
    p.borrow()
        .nodes
        .iter()
        .filter(|node| {
            node.kind == kind
                && node.inputs.iter().any(|dependency| {
                    dependency.cell == trigger
                        && dependency.kind == ReactiveDependencyKind::Reactive
                })
        })
        .map(|node| node.id)
        .collect()
}
fn combinational_node_for_output_and_dependency(
    i: &Interpreter,
    output_name: &str,
    input_name: &str,
    input_kind: ReactiveDependencyKind,
) -> ReactiveNodeId {
    let output = cell(i, output_name);
    let input = cell(i, input_name);
    let p = i.plan();
    let found = p
        .borrow()
        .nodes
        .iter()
        .filter(|node| {
            node.kind == ReactiveNodeKind::Combinational
                && node.outputs.contains(&output)
                && node
                    .inputs
                    .iter()
                    .any(|dependency| dependency.cell == input && dependency.kind == input_kind)
        })
        .map(|node| node.id)
        .collect::<Vec<_>>();
    assert_eq!(
        found.len(),
        1,
        "expected one combinational node for {output_name} consuming {input_name}"
    );
    found[0]
}
fn nodes(i: &Interpreter) -> Vec<ReactiveNodeId> {
    activation_nodes(i, "tick", ReactiveNodeKind::Combinational)
}
fn two_register_nodes(i: &Interpreter) -> Vec<ReactiveNodeId> {
    let registers = activation_nodes(i, "tick", ReactiveNodeKind::Register);
    assert_eq!(registers.len(), 2, "exactly two activation registers");
    let p = i.plan();
    let p = p.borrow();
    let outputs = registers
        .iter()
        .flat_map(|id| p.node(*id).unwrap().outputs.iter())
        .copied()
        .collect::<HashSet<_>>();
    assert_eq!(outputs, [cell(i, "x"), cell(i, "y")].into_iter().collect());
    registers
}
fn snapshot(
    i: &Interpreter,
) -> (
    usize,
    Vec<(
        usize,
        usize,
        ReactiveNodeKind,
        Vec<u64>,
        Vec<(u64, ReactiveDependencyKind)>,
    )>,
    Vec<(u64, Vec<usize>)>,
    Vec<(u64, Vec<usize>)>,
) {
    let p = i.plan();
    let p = p.borrow();
    let nodes = p
        .nodes
        .iter()
        .map(|n| {
            (
                n.id,
                n.plan_index,
                n.kind,
                n.outputs.iter().map(|c| c.get()).collect(),
                n.inputs.iter().map(|d| (d.cell.get(), d.kind)).collect(),
            )
        })
        .collect();
    let mut reactive = p
        .reactive_consumers
        .iter()
        .map(|(c, n)| (c.get(), n.clone()))
        .collect::<Vec<_>>();
    let mut sampled = p
        .sampled_consumers
        .iter()
        .map(|(c, n)| (c.get(), n.clone()))
        .collect::<Vec<_>>();
    reactive.sort_by_key(|(c, _)| *c);
    sampled.sort_by_key(|(c, _)| *c);
    (p.len(), nodes, reactive, sampled)
}
#[test]
fn activation_scope_does_not_execute_during_load() {
    let i = interpret(REGISTER);
    let (next_x, register) = (
        nodes_for_output(&i, "next-x", ReactiveNodeKind::Combinational),
        unique_register_for(&i, "x"),
    );
    assert_eq!(value(&i, "x"), 10.);
    assert!(!next_x.is_empty());
    assert_eq!(
        i.plan()
            .borrow()
            .nodes
            .iter()
            .filter(|n| n.kind == ReactiveNodeKind::Register && n.outputs.contains(&cell(&i, "x")))
            .map(|n| n.id)
            .collect::<Vec<_>>(),
        vec![register]
    );
    assert!(!i.has_pending_reactive_registers());
    assert!(!i.plan().activation_registration_active());
}
#[test]
fn activation_scope_trigger_is_reactive() {
    let i = load();
    let t = cell(&i, "tick");
    let p = i.plan();
    assert!(nodes(&i).iter().all(|n| {
        p.borrow()
            .node(*n)
            .unwrap()
            .inputs
            .iter()
            .any(|d| d.cell == t && d.kind == ReactiveDependencyKind::Reactive)
    }));
}
#[test]
fn activation_scope_external_inputs_are_sampled() {
    let i = load();
    let left = combinational_node_for_output_and_dependency(
        &i,
        "left",
        "x",
        ReactiveDependencyKind::Sampled,
    );
    let p = i.plan();
    let p = p.borrow();
    for input in ["x", "radius"] {
        let c = cell(&i, input);
        assert!(
            p.node(left)
                .unwrap()
                .inputs
                .iter()
                .any(|d| d.cell == c && d.kind == ReactiveDependencyKind::Sampled)
        );
        assert!(p.sampled_consumers_for(c).contains(&left));
        assert!(!p.reactive_consumers_for(c).contains(&left));
    }
}
#[test]
fn activation_scope_local_outputs_are_reactive() {
    let i = load();
    let p = i.plan();
    assert!(
        p.borrow()
            .node(combinational_node_for_output_and_dependency(
                &i,
                "doubled",
                "left",
                ReactiveDependencyKind::Reactive
            ))
            .unwrap()
            .inputs
            .iter()
            .any(|d| d.cell == cell(&i, "left") && d.kind == ReactiveDependencyKind::Reactive)
    );
}
#[test]
fn activation_scope_runs_once_on_trigger() {
    let mut i = load();
    let body = nodes(&i);
    let t = cell(&i, "tick");
    let o = i.advance_reactive_turn(&[t]).unwrap();
    let executed = o
        .before_commit
        .executed_nodes
        .iter()
        .chain(o.after_commit.executed_nodes.iter())
        .copied()
        .collect::<Vec<_>>();
    let unique = executed.iter().copied().collect::<HashSet<_>>();
    assert_eq!(unique.len(), executed.len(), "no node runs twice");
    for node in body {
        assert_eq!(
            executed.iter().filter(|id| **id == node).count(),
            1,
            "body node {node} runs exactly once"
        );
    }
    assert_eq!((value(&i, "left"), value(&i, "doubled")), (8., 16.));
}
#[test]
fn activation_scope_ignores_external_value_change() {
    let mut i = load();
    let x = cell(&i, "x");
    let o = i.advance_reactive_turn(&[x]).unwrap();
    assert!(
        nodes(&i)
            .iter()
            .all(|n| !o.before_commit.executed_nodes.contains(n))
    );
}
#[test]
fn activation_scope_samples_latest_external_value() {
    let mut i = load();
    let x = i.symbols().borrow().get(hash_str("x")).unwrap();
    x.replace(
        &crate::ValueCell::from_exact(20.0)
            .unwrap()
            .snapshot()
            .unwrap(),
    )
    .unwrap();
    let t = cell(&i, "tick");
    i.advance_reactive_turn(&[t]).unwrap();
    assert_eq!(
        {
            let value = i
                .symbols()
                .borrow()
                .get(hash_str("left"))
                .unwrap()
                .snapshot()
                .unwrap();
            let crate::ValueData::F64(value) = value.data() else {
                panic!("left is not f64")
            };
            value.to_f64()
        },
        18.
    );
}
#[test]
fn activation_scope_registers_commit_atomically() {
    let mut i = interpret(TWO_REGISTERS);
    let registers = two_register_nodes(&i);
    assert_eq!((value(&i, "x"), value(&i, "y")), (0., 0.));
    let o = i.advance_reactive_turn(&[cell(&i, "tick")]).unwrap();
    assert_eq!(o.before_commit.pending_register_nodes, registers);
    assert_eq!(o.register_commit.staged_nodes, registers);
    assert_eq!(o.register_commit.committed_nodes, registers);
    assert_eq!(
        o.register_commit.dirty_cells,
        vec![cell(&i, "x"), cell(&i, "y")]
    );
    assert_eq!((value(&i, "x"), value(&i, "y")), (1., 2.));
}
#[test]
fn activation_scope_register_commit_does_not_reactivate_body() {
    let mut i = interpret(TWO_REGISTERS);
    let combinational = activation_nodes(&i, "tick", ReactiveNodeKind::Combinational);
    let registers = two_register_nodes(&i);
    assert!(!combinational.is_empty());
    let o = i.advance_reactive_turn(&[cell(&i, "tick")]).unwrap();
    assert!(combinational.iter().all(|id| {
        o.before_commit
            .executed_nodes
            .iter()
            .filter(|node| **node == *id)
            .count()
            == 1
    }));
    assert_eq!(
        o.before_commit
            .pending_register_nodes
            .iter()
            .copied()
            .collect::<HashSet<_>>(),
        registers.iter().copied().collect()
    );
    assert_eq!(
        o.before_commit.pending_register_nodes.len(),
        registers.len()
    );
    assert_eq!(
        o.register_commit
            .staged_nodes
            .iter()
            .copied()
            .collect::<HashSet<_>>(),
        registers.iter().copied().collect()
    );
    assert_eq!(o.register_commit.staged_nodes.len(), registers.len());
    assert_eq!(
        o.register_commit
            .committed_nodes
            .iter()
            .copied()
            .collect::<HashSet<_>>(),
        registers.iter().copied().collect()
    );
    assert_eq!(o.register_commit.committed_nodes.len(), registers.len());
    assert_eq!((value(&i, "x"), value(&i, "y")), (1., 2.));
    assert!(
        combinational
            .iter()
            .all(|id| !o.after_commit.executed_nodes.contains(id))
    );
}
#[test]
fn activation_scope_failed_elaboration_clears_registration_state() {
    let mut i = interpret("tick := 0.0\nx := 1.0");
    let before = snapshot(&i);
    let failing=mech_syntax::parser::parse("~> tick {\n  registered-first := x + 1.0\n  fails-later := function-that-does-not-exist(registered-first)\n}").unwrap();
    let error = i.interpret(&failing).unwrap_err();
    assert!(format!("{error:?}").contains("Function"));
    assert_eq!(snapshot(&i), before);
    assert!(!i.symbols().borrow().contains(hash_str("registered-first")));
    assert!(!i.symbols().borrow().contains(hash_str("fails-later")));
    assert!(!i.plan().activation_registration_active());
    let ordinary = mech_syntax::parser::parse("ordinary := x + 2.0").unwrap();
    i.interpret(&ordinary).unwrap();
    let ordinary_nodes = nodes_for_output(&i, "ordinary", ReactiveNodeKind::Combinational);
    assert!(!ordinary_nodes.is_empty());
    let p = i.plan();
    let p = p.borrow();
    assert!(ordinary_nodes.iter().all(|node| {
        !p.node(*node)
            .unwrap()
            .inputs
            .iter()
            .any(|d| d.cell == cell(&i, "tick"))
    }));
    assert!(ordinary_nodes.iter().any(|node| {
        p.node(*node)
            .unwrap()
            .inputs
            .iter()
            .any(|d| d.cell == cell(&i, "x") && d.kind == ReactiveDependencyKind::Reactive)
    }));
    assert!(!i.plan().activation_registration_active());
}
#[test]
fn activation_scope_rejects_whole_assignment_to_trigger() {
    let mut i = Interpreter::with_function_catalog(
        0,
        10_000,
        crate::test_support::catalog::function_catalog(),
    );
    let setup = mech_syntax::parser::parse("~tick := 0.0").unwrap();
    i.interpret(&setup).unwrap();
    let before = snapshot(&i);
    let t = mech_syntax::parser::parse("~> tick {\n tick = tick + 1.0\n}").unwrap();
    assert!(
        format!("{:?}", i.interpret(&t).unwrap_err())
            .contains("ActivationScopeTriggerWriteUnsupported")
    );
    assert_eq!(snapshot(&i), before);
    assert!(!i.plan().activation_registration_active());
}
#[test]
fn activation_scope_rejects_operator_assignment_to_trigger() {
    let mut i = Interpreter::with_function_catalog(
        0,
        10_000,
        crate::test_support::catalog::function_catalog(),
    );
    let setup = mech_syntax::parser::parse("~tick := 0.0").unwrap();
    i.interpret(&setup).unwrap();
    let before = snapshot(&i);
    let t = mech_syntax::parser::parse("~> tick {\n tick += 1.0\n}").unwrap();
    assert!(
        format!("{:?}", i.interpret(&t).unwrap_err())
            .contains("ActivationScopeTriggerWriteUnsupported")
    );
    assert_eq!(snapshot(&i), before);
    assert!(!i.plan().activation_registration_active());
}

#[test]
fn patterned_activation_rejects_alias_and_indexed_writes_atomically() {
    let mut alias = interpret("~event := 1.0\nalias := event");
    let before = snapshot(&alias);
    let tree =
        mech_syntax::parser::parse("~> alias\n  | * => {\n      event += 1.0\n    }").unwrap();
    let error = alias.interpret(&tree).unwrap_err();
    assert_eq!(error.kind_name(), "ActivationScopeTriggerWriteUnsupported");
    assert_eq!(value(&alias, "event"), 1.0);
    assert_eq!(snapshot(&alias), before);
    assert!(!alias.plan().activation_registration_active());

    for assignment in ["values[1] = 3.0", "values[1] += 3.0"] {
        let mut indexed = interpret("event := 0.0\n~values := [1.0 2.0]");
        let before = snapshot(&indexed);
        let values = indexed
            .symbols()
            .borrow()
            .get(hash_str("values"))
            .unwrap()
            .snapshot()
            .unwrap()
            .canonical_data_draft()
            .unwrap();
        let tree = mech_syntax::parser::parse(&format!(
            "~> event\n  | * => {{\n      {assignment}\n    }}"
        ))
        .unwrap();
        let error = indexed.interpret(&tree).unwrap_err();
        assert_eq!(
            error.kind_name(),
            "ActivationPatternRegisterWriteUnsupported"
        );
        assert_eq!(
            indexed
                .symbols()
                .borrow()
                .get(hash_str("values"))
                .unwrap()
                .snapshot()
                .unwrap()
                .canonical_data_draft()
                .unwrap(),
            values
        );
        assert_eq!(snapshot(&indexed), before);
        assert!(!indexed.plan().activation_registration_active());
    }
}

#[test]
fn patterned_activation_arm_definitions_do_not_leak_between_arms() {
    let mut interpreter = interpret("event := (1.0, 2.0)");
    let symbols = interpreter.symbols().borrow().snapshot();
    let dictionary = interpreter.dictionary().borrow().clone();
    let topology = snapshot(&interpreter);
    let tree = mech_syntax::parser::parse(
        r#"
~> event
  | (x, y) => { first-local := x + y }
  | (a, b) => { second-local := first-local + a + b }
  | * => { fallback := 0.0 }
"#,
    )
    .unwrap();
    let error = interpreter.interpret(&tree).unwrap_err();
    assert_eq!(error.kind_name(), "UndefinedVariable");
    assert_eq!(interpreter.symbols().borrow().snapshot(), symbols);
    assert_eq!(*interpreter.dictionary().borrow(), dictionary);
    assert_eq!(snapshot(&interpreter), topology);
    for name in [
        "first-local",
        "second-local",
        "fallback",
        "x",
        "y",
        "a",
        "b",
    ] {
        assert!(!interpreter.symbols().borrow().contains(hash_str(name)));
    }
}

#[test]
fn activation_scope_plan_is_stable_across_triggers() {
    let mut i = load();
    let before = snapshot(&i);
    let t = cell(&i, "tick");
    for _ in 0..3 {
        i.advance_reactive_turn(&[t]).unwrap();
    }
    assert_eq!(snapshot(&i), before);
}

#[test]
fn patterned_activation_captures_tuple_values_without_growing_the_plan() {
    let mut i = interpret(
        r#"
event := (3.0, 4.0)
~selected := 0.0
~> event
  | (x, y) => {
      selected = x * 10.0 + y
    }
  | * => {
      selected = -1.0
    }
"#,
    );
    let trigger = cell(&i, "event");
    let before = snapshot(&i);
    assert_eq!(value(&i, "selected"), 0.0);

    for _ in 0..2 {
        i.advance_reactive_turn(&[trigger]).unwrap();
        assert_eq!(value(&i, "selected"), 34.0);
        assert_eq!(snapshot(&i), before);
    }
}

#[test]
fn patterned_activation_guards_dispatch_in_order_without_growing_the_plan() {
    let mut i = interpret(
        r#"
event := 0.0
~selected := 0.0
~> event
  | first, first > 10.0 => {
      selected = first + 100.0
    }
  | second, second > 0.0 => {
      selected = second + 200.0
    }
  | * => {
      selected = -1.0
    }
"#,
    );
    let trigger = cell(&i, "event");
    let topology = snapshot(&i);

    for (event, selected) in [(20.0, 120.0), (5.0, 205.0), (-5.0, -1.0)] {
        for _ in 0..2 {
            set_value(&i, "event", event);
            i.advance_reactive_turn(&[trigger]).unwrap();
            assert_eq!(value(&i, "selected"), selected);
            assert_eq!(snapshot(&i), topology);
        }
    }
}

#[test]
fn patterned_activation_unselected_guard_skips_body_errors_atomically() {
    let mut interpreter = interpret(
        r#"
event := -1.0
values := [1.0]
ix := 1.0
~selected := 0.0
~> event
  | x, x > 0.0 => { selected = values[ix] }
  | * => { selected = -1.0 }
"#,
    );
    let trigger = cell(&interpreter, "event");
    let ix = cell(&interpreter, "ix");
    let topology = snapshot(&interpreter);

    set_value(&interpreter, "ix", 2.0);
    interpreter.advance_reactive_turn(&[ix]).unwrap();
    interpreter.advance_reactive_turn(&[trigger]).unwrap();
    assert_eq!(value(&interpreter, "selected"), -1.0);
    assert_eq!(snapshot(&interpreter), topology);

    set_value(&interpreter, "event", 1.0);
    let error = interpreter.advance_reactive_turn(&[trigger]).unwrap_err();
    let kind = error.kind_name();
    assert!(
        matches!(
            kind.as_str(),
            "FunctionShapeContractViolation" | "MatrixIndexOutOfBounds" | "GenericError"
        ),
        "{error:?}"
    );
    assert_eq!(value(&interpreter, "selected"), -1.0);
    assert_eq!(snapshot(&interpreter), topology);
}

#[test]
fn patterned_activation_preflight_and_elaboration_fail_atomically() {
    let mut i = interpret("event := (1.0, 2.0)\nouter := 99.0");
    let symbols = i.symbols().borrow().snapshot();
    let dictionary = i.dictionary().borrow().clone();
    let topology = snapshot(&i);

    let refutable = mech_syntax::parser::parse(
        r#"
~> event
  | (x, x) => {
      selected := x
    }
"#,
    )
    .unwrap();
    let error = i.interpret(&refutable).unwrap_err();
    assert_eq!(error.kind_name(), "ActivationPatternArmsNonExhaustive");
    assert_eq!(i.symbols().borrow().snapshot(), symbols);
    assert_eq!(*i.dictionary().borrow(), dictionary);
    assert_eq!(snapshot(&i), topology);

    let invalid_body = mech_syntax::parser::parse(
        r#"
~> event
  | (x, y) => {
      local := x + y
      failure := function-that-does-not-exist(local)
    }
  | * => {
      fallback := 0.0
    }
"#,
    )
    .unwrap();
    assert!(i.interpret(&invalid_body).is_err());
    assert_eq!(i.symbols().borrow().snapshot(), symbols);
    assert_eq!(*i.dictionary().borrow(), dictionary);
    assert_eq!(snapshot(&i), topology);
    for name in ["x", "y", "local", "failure", "fallback"] {
        assert!(!i.symbols().borrow().contains(hash_str(name)));
    }

    let mut mismatch = interpret("event := (1.0, \"one\")");
    let before = snapshot(&mismatch);
    let tree = mech_syntax::parser::parse(
        "~> event\n  | (x, x) => { selected := x }\n  | * => { selected := 0.0 }",
    )
    .unwrap();
    let error = mismatch.interpret(&tree).unwrap_err();
    assert_eq!(error.kind_name(), "PatternCompileError");
    assert_eq!(snapshot(&mismatch), before);

    let mut nested = interpret("event := 1.0\ntick := 0.0");
    let symbols = nested.symbols().borrow().snapshot();
    let dictionary = nested.dictionary().borrow().clone();
    let topology = snapshot(&nested);
    let tree = mech_syntax::parser::parse(
        r#"
~> event
  | 1.0 => {
      ~> tick { nested := 1.0 }
    }
  | * => { fallback := 0.0 }
"#,
    )
    .unwrap();
    let error = nested.interpret(&tree).unwrap_err();
    assert_eq!(error.kind_name(), "ActivationPatternDefinitionUnsupported");
    assert_eq!(nested.symbols().borrow().snapshot(), symbols);
    assert_eq!(*nested.dictionary().borrow(), dictionary);
    assert_eq!(snapshot(&nested), topology);
    assert!(!nested.symbols().borrow().contains(hash_str("nested")));
    assert!(!nested.symbols().borrow().contains(hash_str("fallback")));

    let mut context = interpret("event := 1.0");
    let symbols = context.symbols().borrow().snapshot();
    let dictionary = context.dictionary().borrow().clone();
    let topology = snapshot(&context);
    let context_bindings = context.context_bindings.borrow().clone();
    let tree = mech_syntax::parser::parse(
        r#"
~> event
  | 1.0 => { @temporary := test://resource }
  | * => { fallback := 0.0 }
"#,
    )
    .unwrap();
    let error = context.interpret(&tree).unwrap_err();
    assert_eq!(error.kind_name(), "ActivationPatternDefinitionUnsupported");
    assert_eq!(context.symbols().borrow().snapshot(), symbols);
    assert_eq!(*context.dictionary().borrow(), dictionary);
    assert_eq!(snapshot(&context), topology);
    assert_eq!(*context.context_bindings.borrow(), context_bindings);
    assert!(
        !context
            .context_bindings
            .borrow()
            .contains_key(&hash_str("temporary"))
    );
    assert!(context.plan().pattern_activation_registrations().is_empty());
    assert!(!context.symbols().borrow().contains(hash_str("fallback")));
}

#[test]
fn patterned_activation_exhaustiveness_rules_are_preserved_canonically() {
    for (setup, arms, expected) in [
        (
            "event := 1.0",
            "| 1.0 => { selected := 1.0 }",
            "ActivationPatternArmsNonExhaustive",
        ),
        (
            "event := (1.0, 1.0)",
            "| (x, x) => { selected := x }",
            "ActivationPatternArmsNonExhaustive",
        ),
        (
            "event := 1.0",
            "| value, value > 0.0 => { selected := value }",
            "ActivationPatternArmsNonExhaustive",
        ),
        (
            "event := 1.0",
            "| * => { selected := 1.0 }\n  | value => { selected := value }",
            "ActivationPatternWildcardMustBeLast",
        ),
    ] {
        let mut interpreter = interpret(setup);
        let before = snapshot(&interpreter);
        let tree = mech_syntax::parser::parse(&format!("~> event\n  {arms}")).unwrap();
        let error = interpreter.interpret(&tree).unwrap_err();
        assert_eq!(error.kind_name(), expected, "{arms}");
        assert_eq!(snapshot(&interpreter), before, "{arms}");
    }

    for source in [
        r#"
event := (1.0, 2.0)
~> event
  | (x, y) => {
      selected := x * 10.0 + y
    }
"#,
        r#"
event := [1.0 2.0]
~> event
  | [left, right] => {
      selected := left * 10.0 + right
    }
"#,
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
    ] {
        let interpreter = interpret(source);
        assert_eq!(
            interpreter.plan().pattern_activation_registrations().len(),
            1
        );
    }
}

#[test]
fn patterned_activation_rejects_eager_guard_control_flow_atomically() {
    for guard in ["(x? | 0.0 => false | * => true.)", "passes(x)"] {
        let setup = if guard == "passes(x)" {
            r#"
passes(value<f64>) => <bool>
  | 0.0 => false
  | * => true.
event := 1.0
"#
        } else {
            "event := 1.0"
        };
        let mut interpreter = interpret(setup);
        let before = snapshot(&interpreter);
        let tree = mech_syntax::parser::parse(&format!(
            "~> event\n  | x, {guard} => {{ selected := x }}\n  | * => {{ selected := -1.0 }}"
        ))
        .unwrap();
        let error = interpreter.interpret(&tree).unwrap_err();
        assert_eq!(error.kind_name(), "ActivationPatternGuardMustBePure");
        assert_eq!(snapshot(&interpreter), before);
        assert!(!interpreter.plan().activation_registration_active());
    }

    let mut interpreter = interpret("event := 1.0");
    let symbols = interpreter.symbols().borrow().snapshot();
    let dictionary = interpreter.dictionary().borrow().clone();
    let before = snapshot(&interpreter);
    let tree = mech_syntax::parser::parse(
        "~> event\n  | x, x + 1.0 => { selected := x }\n  | * => { selected := -1.0 }",
    )
    .unwrap();
    assert!(interpreter.interpret(&tree).is_err());
    assert_eq!(interpreter.symbols().borrow().snapshot(), symbols);
    assert_eq!(*interpreter.dictionary().borrow(), dictionary);
    assert_eq!(snapshot(&interpreter), before);
    assert!(!interpreter.plan().activation_registration_active());
}

struct UnsupportedGuardSpecializer {
    calls: Arc<AtomicUsize>,
}

impl CanonicalFunctionSpecializer for UnsupportedGuardSpecializer {
    fn specialize_invocation(
        &self,
        _: &SpecializationInvocation,
        _: &mut SpecializationContext<'_>,
    ) -> MResult<SpecializedFunction> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        panic!("an unsupported guard extension must be rejected before specialization")
    }

    fn guard_safety(&self) -> GuardFunctionSafety {
        GuardFunctionSafety::Unsupported
    }
}

#[test]
fn patterned_activation_rejects_unsafe_extensions_before_specialization() {
    let mut interpreter = interpret("event := 1.0");
    let calls = Arc::new(AtomicUsize::new(0));
    let entry = FunctionExtensionEntry::new(
        "unsafe-guard-test",
        Arc::new(UnsupportedGuardSpecializer {
            calls: calls.clone(),
        }),
    );
    let extension = entry.id;
    {
        let mut state = interpreter.state.borrow_mut();
        state.function_extensions.insert_or_replace(entry).unwrap();
        state
            .function_environment
            .bind_extension("unsafe-guard-test", "unsafe-guard-test", extension)
            .unwrap();
    }
    let before = snapshot(&interpreter);
    let tree = mech_syntax::parser::parse(
        "~> event\n  | x, unsafe-guard-test(x) => { selected := x }\n  | * => { selected := -1.0 }",
    )
    .unwrap();
    let error = interpreter.interpret(&tree).unwrap_err();
    assert_eq!(error.kind_name(), "ActivationPatternGuardMustBePure");
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(snapshot(&interpreter), before);
    assert!(!interpreter.plan().activation_registration_active());
}

#[test]
fn patterned_activation_permanent_pattern_forms_dispatch_without_topology_growth() {
    for (source, expected) in [
        (
            r#"
<event-kind> := :pressed<f64> | :released<f64> | :other<f64>
event<event-kind> := :pressed(3.0)
~selected := 0.0
~> event
  | :pressed(x) => { selected = x }
  | :released(x) => { selected = x + 100.0 }
  | * => { selected = -1.0 }
"#,
            3.0,
        ),
        (
            r#"
<event-kind> := :pressed<f64> | :released<f64> | :other<f64>
event<event-kind> := :released(2.0)
~selected := 0.0
~> event
  | :pressed(x) => { selected = x }
  | :released(x) => { selected = x + 100.0 }
  | * => { selected = -1.0 }
"#,
            102.0,
        ),
        (
            r#"
<signal> := :ready | :other
event<signal> := :ready
~selected := 0.0
~> event
  | :ready => { selected = 1.0 }
  | * => { selected = -1.0 }
"#,
            1.0,
        ),
        (
            r#"
event := ((1.0, 2.0), 3.0)
~selected := 0.0
~> event
  | ((x, y), z) => { selected = x * 100.0 + y * 10.0 + z }
  | * => { selected = -1.0 }
"#,
            123.0,
        ),
        (
            r#"
event := (4.0, 4.0)
~selected := 0.0
~> event
  | (x, x) => { selected = x }
  | * => { selected = -1.0 }
"#,
            4.0,
        ),
        (
            r#"
event := (4.0, 5.0)
~selected := 0.0
~> event
  | (x, x) => { selected = x }
  | * => { selected = -1.0 }
"#,
            -1.0,
        ),
        (
            r#"
event := 1u64
~selected := 0.0
~> event
  | 1u64 => { selected = 1.0 }
  | * => { selected = -1.0 }
"#,
            1.0,
        ),
        (
            r#"
x := 9.0
event := [1.0 10.0]
~selected := 0.0
~> event
  | [captured, x + 1.0] => { selected = captured }
  | * => { selected = -1.0 }
"#,
            1.0,
        ),
        (
            r#"
event := [1.0 2.0 3.0 4.0]
~selected := 0.0
~> event
  | [head | rest] => { selected = head + rest[1] + rest[3] }
  | * => { selected = -1.0 }
"#,
            7.0,
        ),
        (
            r#"
event := [1.0 2.0 3.0 1.0]
~selected := 0.0
~> event
  | [x, ..., x] => { selected = x + 10.0 }
  | * => { selected = -1.0 }
"#,
            11.0,
        ),
        (
            r#"
event := [1.0 2.0 3.0 4.0]
~selected := 0.0
~> event
  | [head | [second, ..., last]] => {
      selected = head * 100.0 + second * 10.0 + last
    }
  | * => { selected = -1.0 }
"#,
            124.0,
        ),
    ] {
        let mut interpreter = interpret(source);
        let trigger = cell(&interpreter, "event");
        let topology = snapshot(&interpreter);
        let activation = interpreter.plan().pattern_activation_registrations()[0].clone();
        for _ in 0..2 {
            let outcome = interpreter.advance_reactive_turn(&[trigger]).unwrap();
            let executed = outcome
                .before_commit
                .executed_nodes
                .iter()
                .chain(&outcome.after_commit.executed_nodes)
                .copied()
                .collect::<HashSet<_>>();
            let changed = outcome
                .before_commit
                .changed_nodes
                .iter()
                .chain(&outcome.after_commit.changed_nodes)
                .copied()
                .collect::<HashSet<_>>();
            let selected = activation
                .arms
                .iter()
                .position(|arm| changed.contains(&arm.gate_node))
                .expect("one activation arm must be selected");
            for (index, arm) in activation.arms.iter().enumerate() {
                for node in arm.body_node_start..arm.body_node_end {
                    if index != selected {
                        assert!(!executed.contains(&node));
                    }
                }
            }
            assert_eq!(value(&interpreter, "selected"), expected);
            assert_eq!(snapshot(&interpreter), topology);
        }
    }
}

#[test]
fn patterned_activation_captures_do_not_leak_and_restore_outer_bindings() {
    let interpreter = interpret(
        r#"
x := 9.0
event := (1.0, 2.0)
~selected := 0.0
~> event
  | (x, local) => { selected = x + local }
  | * => { selected = -1.0 }
"#,
    );
    assert_eq!(value(&interpreter, "x"), 9.0);
    assert!(!interpreter.symbols().borrow().contains(hash_str("local")));

    let mut interpreter = interpreter;
    let topology = snapshot(&interpreter);
    interpreter
        .advance_reactive_turn(&[cell(&interpreter, "event")])
        .unwrap();
    assert_eq!(value(&interpreter, "selected"), 3.0);
    assert_eq!(value(&interpreter, "x"), 9.0);
    assert!(!interpreter.symbols().borrow().contains(hash_str("local")));
    assert_eq!(snapshot(&interpreter), topology);
}

#[test]
fn patterned_activation_samples_pattern_expressions_only_on_trigger() {
    let mut interpreter = interpret(
        r#"
expected := 2.0
event := [1.0 2.0]
~selected := 0.0
~> event
  | [head, expected + 0.0] => { selected = head }
  | * => { selected = -1.0 }
"#,
    );
    let expected = cell(&interpreter, "expected");
    let event = cell(&interpreter, "event");
    let topology = snapshot(&interpreter);
    let activation = interpreter.plan().pattern_activation_registrations()[0].clone();

    set_value(&interpreter, "expected", 3.0);
    let outcome = interpreter.advance_reactive_turn(&[expected]).unwrap();
    let executed = outcome
        .before_commit
        .executed_nodes
        .iter()
        .chain(&outcome.after_commit.executed_nodes)
        .copied()
        .collect::<HashSet<_>>();
    assert!(!executed.contains(&activation.scope_pulse_node));
    assert!(!executed.contains(&activation.selector_node));
    for arm in &activation.arms {
        assert!(!executed.contains(&arm.matcher_node));
        assert!(!executed.contains(&arm.finalizer_node));
        assert!(!executed.contains(&arm.gate_node));
    }
    assert_eq!(value(&interpreter, "selected"), 0.0);

    interpreter.advance_reactive_turn(&[event]).unwrap();
    assert_eq!(value(&interpreter, "selected"), -1.0);
    assert_eq!(snapshot(&interpreter), topology);
}

#[test]
fn patterned_activation_samples_current_user_function_output_on_trigger() {
    let mut interpreter = interpret(
        r#"
sample(value<f64>) => <f64>
  | value + 0.0.
expected := 2.0
event := [1.0 2.0]
~selected := 0.0
~> event
  | [head, sample(expected)] => { selected = head }
  | * => { selected = -1.0 }
"#,
    );
    let expected = cell(&interpreter, "expected");
    let event = cell(&interpreter, "event");
    let topology = snapshot(&interpreter);
    let activation = interpreter.plan().pattern_activation_registrations()[0].clone();

    set_value(&interpreter, "expected", 3.0);
    let outcome = interpreter.advance_reactive_turn(&[expected]).unwrap();
    let executed = outcome
        .before_commit
        .executed_nodes
        .iter()
        .chain(&outcome.after_commit.executed_nodes)
        .copied()
        .collect::<HashSet<_>>();
    assert!(!executed.contains(&activation.scope_pulse_node));
    assert!(!executed.contains(&activation.selector_node));
    for arm in &activation.arms {
        assert!(!executed.contains(&arm.matcher_node));
        assert!(!executed.contains(&arm.finalizer_node));
        assert!(!executed.contains(&arm.gate_node));
    }
    assert_eq!(value(&interpreter, "selected"), 0.0);

    interpreter.advance_reactive_turn(&[event]).unwrap();
    assert_eq!(value(&interpreter, "selected"), -1.0);
    assert_eq!(snapshot(&interpreter), topology);
}
