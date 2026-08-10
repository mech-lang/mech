use mech_core::snapshot::SnapshotValidationContext;
use mech_core::{
    ConstantStoreBuilder, ReadSource, SchemaBody, SchemaDraft, SchemaId, SchemaTable,
    SchemaTableBuilder, SnapshotValueError, Value, ValueDataDraft, ValueDraft,
};

fn bool_schema_table() -> (SchemaTable, SchemaId) {
    let schema = SchemaDraft {
        dimension_parameters: Box::new([]),
        body: SchemaBody::Bool,
    }
    .finalize()
    .unwrap();
    let mut builder = SchemaTableBuilder::new();
    let handle = builder.insert(schema).unwrap();
    let build = builder.finish().unwrap();
    let schema = build.resolve(handle).unwrap();
    let (schemas, _) = build.into_parts();
    (schemas, schema)
}

fn bool_value(schemas: &SchemaTable, schema: SchemaId, value: bool) -> Value {
    ValueDraft {
        schema,
        shape_values: Box::new([]),
        data: ValueDataDraft::Bool(value),
    }
    .finalize(&SnapshotValidationContext::new(schemas))
    .unwrap()
}

#[test]
fn insertion_order_does_not_change_constant_ids() {
    let (schemas, schema) = bool_schema_table();
    let false_value = bool_value(&schemas, schema, false);
    let true_value = bool_value(&schemas, schema, true);

    let mut first = ConstantStoreBuilder::new(&schemas);
    let first_false = first.insert(false_value.clone()).unwrap();
    let first_true = first.insert(true_value.clone()).unwrap();
    let first = first.finish().unwrap();

    let mut second = ConstantStoreBuilder::new(&schemas);
    let second_true = second.insert(true_value).unwrap();
    let second_false = second.insert(false_value).unwrap();
    let second = second.finish().unwrap();

    assert_eq!(first.resolve(first_false), second.resolve(second_false));
    assert_eq!(first.resolve(first_true), second.resolve(second_true));
}

#[test]
fn duplicate_constants_deduplicate_with_checked_remapping() {
    let (schemas, schema) = bool_schema_table();
    let value = bool_value(&schemas, schema, true);
    let expected = value.clone();
    let mut builder = ConstantStoreBuilder::new(&schemas);
    let first = builder.insert(value.clone()).unwrap();
    let second = builder.insert(value).unwrap();
    let build = builder.finish().unwrap();

    assert_eq!(build.store.len(), 1);
    assert_eq!(build.resolve(first), build.resolve(second));
    let id = build.resolve(first).unwrap();
    let entry = build.store.entry(id).unwrap();
    assert_eq!(entry.hash(), entry.value().value_hash(&schemas).unwrap());
    assert!(
        entry
            .value()
            .snapshot_eq(&schemas, &expected, &schemas)
            .unwrap()
    );
}

#[test]
fn handles_are_content_compatible_but_not_authority_tokens() {
    let (schemas, schema) = bool_schema_table();

    let mut first = ConstantStoreBuilder::new(&schemas);
    let first_false = first.insert(bool_value(&schemas, schema, false)).unwrap();
    let first = first.finish().unwrap();

    let mut compatible = ConstantStoreBuilder::new(&schemas);
    let compatible_false = compatible
        .insert(bool_value(&schemas, schema, false))
        .unwrap();
    compatible.finish().unwrap();

    let mut foreign = ConstantStoreBuilder::new(&schemas);
    let foreign_true = foreign.insert(bool_value(&schemas, schema, true)).unwrap();
    let foreign_second = foreign.insert(bool_value(&schemas, schema, false)).unwrap();

    assert!(first.resolve(first_false).is_ok());
    assert_eq!(first.resolve(compatible_false), first.resolve(first_false));
    assert!(matches!(
        first.resolve(foreign_true),
        Err(SnapshotValueError::InvalidConstantHandleV1)
    ));
    assert!(matches!(
        first.resolve(foreign_second),
        Err(SnapshotValueError::InvalidConstantHandleV1)
    ));
}

#[test]
fn read_sources_distinguish_immutable_constants_from_cells() {
    let constant = ReadSource::Constant(mech_core::ConstantId::new(7));
    let cell = ReadSource::Cell(mech_core::CellId::new(
        mech_core::ReactiveInstanceId::new(1, 2),
        mech_core::CellSlotId::new(3),
    ));
    assert_ne!(constant, cell);
}
