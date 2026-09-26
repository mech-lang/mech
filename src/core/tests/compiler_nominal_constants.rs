#![cfg(feature = "semantic-compiler")]

use mech_core::{snapshot::EnumDraft, *};

fn atom(name: &str) -> ValueCell {
    let path = CanonicalNominalPath::new(vec![name.to_owned()]).unwrap();
    ValueCell::from_schema_data(
        SchemaBody::Atom(NominalKey::from_path(NominalKind::Atom, &path)),
        ValueDataDraft::Atom,
    )
    .unwrap()
}

fn mode(ordinal: u32) -> ValueCell {
    let path = CanonicalNominalPath::new(vec!["mode".to_owned()]).unwrap();
    ValueCell::from_schema_data(
        SchemaBody::Enum {
            key: NominalKey::from_path(NominalKind::Enum, &path),
            variants: ["paused", "patrol", "fault"]
                .into_iter()
                .map(|name| EnumVariantSchema {
                    name: name.to_owned(),
                    payload: None,
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        },
        ValueDataDraft::Enum(EnumDraft {
            ordinal,
            payload: None,
        }),
    )
    .unwrap()
}

fn compile(cell: &ValueCell) -> CompiledBytecode {
    let mut context = CompileCtx::new();
    let register = compile_value_cell_register(cell, &mut context).unwrap();
    context.finish_program(register).unwrap()
}

#[test]
fn semantic_atom_and_complete_enum_constants_preserve_canonical_values() {
    for cell in [atom("paused"), mode(0), mode(2)] {
        let compiled = compile(&cell);
        let values = compiled.artifact_constant_values().unwrap();
        assert_eq!(values.len(), 1);
        let restored = ValueCell::from_snapshot(values[0].clone()).unwrap();
        assert_eq!(
            restored.closed_schema_body().unwrap(),
            cell.closed_schema_body().unwrap()
        );
        assert!(restored.snapshot_eq(&cell).unwrap());
        assert_eq!(compiled.canonical_constants.len(), 1);
        assert!(
            write_bytecode(&compiled.program).is_err(),
            "legacy wire export must not discard nominal authority"
        );
    }
}

#[test]
fn semantic_constant_sidecar_cannot_be_missing_replaced_or_out_of_range() {
    let compiled = compile(&mode(0));
    let mut missing = compiled.clone();
    missing.canonical_constants.clear();
    assert!(missing.artifact_constant_values().is_err());
    let mut replaced = compiled.clone();
    replaced
        .canonical_constants
        .insert(0, mode(2).snapshot().unwrap());
    assert!(replaced.artifact_constant_values().is_err());
    let mut changed_marker = compiled.clone();
    changed_marker.program.constants[0].bytes.push(0);
    assert!(changed_marker.artifact_constant_values().is_err());
    let mut extra = compiled;
    extra
        .canonical_constants
        .insert(1, mode(0).snapshot().unwrap());
    assert!(extra.artifact_constant_values().is_err());
}

#[test]
fn semantic_constant_remapping_tracks_removed_initializers() {
    let mut context = CompileCtx::new();
    let first = compile_value_cell_register(&mode(0), &mut context).unwrap();
    let second_value = mode(2);
    let second = compile_value_cell_register(&second_value, &mut context).unwrap();
    context.record_runtime_produced_register(first).unwrap();
    let compiled = context.finish_program(second).unwrap();
    let values = compiled.artifact_constant_values().unwrap();
    assert_eq!(values.len(), 1);
    assert!(
        ValueCell::from_snapshot(values[0].clone())
            .unwrap()
            .snapshot_eq(&second_value)
            .unwrap()
    );
    assert!(compiled.canonical_constants.contains_key(&0));
}

#[test]
fn ordinary_constants_still_use_the_unchanged_bytecode_route() {
    let compiled = compile(&ValueCell::unit());
    assert!(compiled.canonical_constants.is_empty());
    assert!(write_bytecode(&compiled.program).is_ok());
    assert_eq!(compiled.artifact_constant_values().unwrap().len(), 1);
}
