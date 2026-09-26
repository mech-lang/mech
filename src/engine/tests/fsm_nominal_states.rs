#![cfg(all(
    feature = "semantic-compiler",
    feature = "state_machines",
    feature = "kind_annotation",
    feature = "kind_define",
    feature = "enum"
))]

use mech_core::{
    CanonicalNominalPath, MResult, NoMechExecutionServices, NominalKey, NominalKind, SchemaBody,
    ValueCell,
};
use mech_engine::{CompilerPlanningConfig, CompilerPlanningProgram};

fn run(source: &str) -> MResult<Option<ValueCell>> {
    let program = mech_syntax::parser::parse(source).expect("FSM fixture must parse");
    CompilerPlanningProgram::new(CompilerPlanningConfig::default())
        .plan_tree_with_services(&program, &mut NoMechExecutionServices)
}

fn assert_atom(source: &str, name: &str) {
    let value = run(source)
        .expect("FSM must execute")
        .expect("FSM must return a value");
    let path = CanonicalNominalPath::new(vec![name.to_owned()]).unwrap();
    assert_eq!(
        value.closed_schema_body().unwrap(),
        SchemaBody::Atom(NominalKey::from_path(NominalKind::Atom, &path),)
    );
}

#[test]
fn fsm_initial_state_converts_known_atom_to_declared_enum() {
    assert_atom(
        r#"
<mode> := :ready | :busy
#Check() := | :State(mode<mode>).
#Check() -> :State(:ready)
  :State(:ready) => :matched
.
#Check()
"#,
        "matched",
    );
}

#[test]
fn fsm_next_and_async_states_convert_known_atoms_to_declared_enums() {
    for transition in ["->", "~>"] {
        assert_atom(
            &format!(
                r#"
<mode> := :ready | :busy
#Check() := | :Start | :State(mode<mode>).
#Check() -> :Start
  :Start {transition} :State(:ready)
  :State(:ready) => :matched
.
#Check()
"#
            ),
            "matched",
        );
    }
}

#[test]
fn fsm_mixed_payload_retains_enum_hint_and_untyped_nested_pattern() {
    assert_atom(
        r#"
<mode> := :ready | :busy
#Check() := | :State(mode<mode>, payload).
#Check() -> :State(:ready, (:left, :right))
  :State(:ready, (left, right)) => right
.
#Check()
"#,
        "right",
    );
}

#[test]
fn fsm_mixed_payload_uses_one_binding_scope() {
    assert_atom(
        r#"
<mode> := :ready | :busy
#Check(mode<mode>) := | :State(mode<mode>, echo).
#Check(mode) -> :State(mode, mode)
  :State(value, value) => :matched
.
#Check(:ready)
"#,
        "matched",
    );
}

#[test]
fn fsm_state_rejects_unknown_atom_and_different_enum_identity() {
    assert!(
        run(r#"
<mode> := :ready | :busy
#Check() := | :State(mode<mode>).
#Check() -> :State(:unknown)
  :State(*) => :matched
.
#Check()
"#)
        .is_err()
    );

    assert!(
        run(r#"
<mode> := :ready | :busy
<other> := :ready | :busy
#Check(value<other>) := | :State(mode<mode>).
#Check(value) -> :State(value)
  :State(*) => :matched
.
#Check(:ready)
"#)
        .is_err()
    );
}

#[test]
fn fsm_state_rejects_declared_payload_arity_mismatch() {
    assert!(
        run(r#"
<mode> := :ready | :busy
#Check() := | :State(mode<mode>).
#Check() -> :State(:ready, :extra)
  :State(*) => :matched
.
#Check()
"#)
        .is_err()
    );
}
