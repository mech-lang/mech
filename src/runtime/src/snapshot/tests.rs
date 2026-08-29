use super::*;
use mech_core::snapshot::{OptionDraft, SnapshotValidationContext};
use mech_core::{
    FloatWidth, SchemaBody, SchemaDraft, SchemaTableBuilder, ValueDataDraft, ValueDraft,
};

fn canonical(schema: SchemaBody, data: ValueDataDraft) -> Value {
    let mut builder = SchemaTableBuilder::new();
    let handle = builder
        .insert(
            SchemaDraft {
                dimension_parameters: Box::new([]),
                body: schema,
            }
            .finalize()
            .unwrap(),
        )
        .unwrap();
    let build = builder.finish().unwrap();
    let schema = build.resolve(handle).unwrap();
    let (schemas, _) = build.into_parts();
    ValueDraft {
        schema,
        shape_values: Box::new([]),
        data,
    }
    .finalize(&SnapshotValidationContext::new(&schemas))
    .unwrap()
}

#[test]
fn runtime_value_snapshot_empty_is_canonical_unit() {
    let snapshot = RuntimeValueSnapshot::empty();

    assert!(snapshot.is_empty());
    assert!(matches!(snapshot.value().data(), ValueData::Tuple(values) if values.is_empty()));
    assert_eq!(snapshot.format_canonical_inline(), "()");
}

#[test]
fn runtime_value_snapshot_retains_scalar_schema_and_payload() {
    let value = canonical(
        SchemaBody::FloatingPoint(FloatWidth::W64),
        ValueDataDraft::F64(F64Bits::from_f64(-0.0)),
    );
    let schema_key = value.schema_key();

    let snapshot = RuntimeValueSnapshot::from_value(value).unwrap();

    assert_eq!(snapshot.schema_key(), schema_key);
    assert!(
        matches!(snapshot.value().data(), ValueData::F64(value) if value.bits() == (-0.0_f64).to_bits())
    );
}

#[test]
fn runtime_value_snapshot_clone_shares_only_immutable_data() {
    let value = canonical(
        SchemaBody::String,
        ValueDataDraft::String("detached".into()),
    );
    let snapshot = RuntimeValueSnapshot::from_value(value).unwrap();
    let cloned = snapshot.clone();

    assert_eq!(snapshot, cloned);
    assert_eq!(snapshot.format_canonical_inline(), "\"detached\"");
}

#[test]
fn runtime_value_snapshot_formats_option_without_legacy_materialization() {
    let value = canonical(
        SchemaBody::Option(Box::new(SchemaBody::Bool)),
        ValueDataDraft::Option(OptionDraft {
            present: true,
            value: Some(Box::new(ValueDataDraft::Bool(true))),
        }),
    );
    let snapshot = RuntimeValueSnapshot::from_value(value).unwrap();

    assert_eq!(snapshot.format_canonical_inline(), "some(true)");
}
