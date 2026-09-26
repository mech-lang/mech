use super::*;
use mech_core::snapshot::{OptionDraft, SnapshotValidationContext};
use mech_core::{
    DimensionExpr, FloatWidth, SchemaBody, SchemaDraft, SchemaTableBuilder, ValueDataDraft,
    ValueDraft,
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

fn mode_schema() -> SchemaBody {
    use mech_core::{CanonicalNominalPath, EnumVariantSchema, NominalKey, NominalKind};
    SchemaBody::Enum {
        key: NominalKey::from_path(
            NominalKind::Enum,
            &CanonicalNominalPath::new(vec!["mode".to_owned()]).unwrap(),
        ),
        variants: ["paused", "patrol", "fault"]
            .into_iter()
            .map(|name| EnumVariantSchema {
                name: name.to_owned(),
                payload: None,
            })
            .collect::<Vec<_>>()
            .into_boxed_slice(),
    }
}

#[test]
fn runtime_value_snapshot_uses_declared_enum_names_not_ordinals() {
    use mech_core::snapshot::EnumDraft;
    for (ordinal, name) in ["paused", "patrol", "fault"].into_iter().enumerate() {
        let snapshot = RuntimeValueSnapshot::from_value(canonical(
            mode_schema(),
            ValueDataDraft::Enum(EnumDraft {
                ordinal: ordinal as u32,
                payload: None,
            }),
        ))
        .unwrap();
        assert_eq!(snapshot.format_canonical_inline(), format!(":{name}"));
        assert_eq!(snapshot.format_repl_inline(0), "…");
        #[cfg(feature = "pretty_print")]
        assert_eq!(
            snapshot.format_html(),
            format!("<span class='mech-value'>:{name}</span>")
        );
    }
}

#[test]
fn runtime_value_snapshot_preserves_nested_enum_names_and_preview_budget() {
    use mech_core::snapshot::EnumDraft;
    let snapshot = RuntimeValueSnapshot::from_value(canonical(
        SchemaBody::Tuple(
            vec![mode_schema(), SchemaBody::Option(Box::new(mode_schema()))].into_boxed_slice(),
        ),
        ValueDataDraft::Tuple(
            vec![
                ValueDataDraft::Enum(EnumDraft {
                    ordinal: 1,
                    payload: None,
                }),
                ValueDataDraft::Option(OptionDraft {
                    present: true,
                    value: Some(Box::new(ValueDataDraft::Enum(EnumDraft {
                        ordinal: 2,
                        payload: None,
                    }))),
                }),
            ]
            .into_boxed_slice(),
        ),
    ))
    .unwrap();
    assert_eq!(
        snapshot.format_canonical_inline(),
        "(:patrol, some(:fault))"
    );
    assert_eq!(snapshot.format_repl_inline(1), "(:patrol, …)");
}

#[test]
fn runtime_value_snapshot_formats_static_matrix_orientation_from_its_schema() {
    let matrix = |rows, columns| {
        RuntimeValueSnapshot::from_value(canonical(
            SchemaBody::Matrix {
                element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
                dimensions: vec![
                    DimensionExpr::Constant(rows),
                    DimensionExpr::Constant(columns),
                ]
                .into_boxed_slice(),
            },
            ValueDataDraft::Matrix(
                [3.0, 7.0]
                    .into_iter()
                    .map(|value| ValueDataDraft::F64(F64Bits::from_f64(value)))
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            ),
        ))
        .unwrap()
    };

    assert_eq!(matrix(1, 2).format_canonical_inline(), "[3 7]");
    assert_eq!(matrix(2, 1).format_canonical_inline(), "[3 7]'");
}

#[test]
fn runtime_value_snapshot_column_preview_keeps_orientation_and_budget() {
    let snapshot = RuntimeValueSnapshot::from_value(canonical(
        SchemaBody::Matrix {
            element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W32)),
            dimensions: vec![DimensionExpr::Constant(3), DimensionExpr::Constant(1)]
                .into_boxed_slice(),
        },
        ValueDataDraft::Matrix(
            [0.0, 1.0, 1.0]
                .into_iter()
                .map(|value| ValueDataDraft::F32(F32Bits::from_f32(value)))
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        ),
    ))
    .unwrap();
    let original = snapshot.clone();

    assert_eq!(snapshot.format_canonical_inline(), "[0 1 1]'");
    assert_eq!(snapshot.format_repl_inline(2), "[0 1 …]'");
    assert_eq!(snapshot.format_repl_inline(0), "[…]'");
    assert_eq!(
        snapshot, original,
        "display never changes schema or payload"
    );
    #[cfg(feature = "pretty_print")]
    {
        assert_eq!(
            snapshot.format_html(),
            "<span class='mech-value'>[0 1 1]'</span>"
        );
        assert_eq!(
            snapshot.format_repl_html(2),
            "<pre class='mech-value-preview mech-value-elided'>[0 1 …]'</pre>"
        );
    }
}

#[test]
fn runtime_value_snapshot_omits_repeated_scalar_types_without_changing_them() {
    for value in [0.0_f32, -0.0_f32, 0.125_f32] {
        let snapshot = RuntimeValueSnapshot::from_value(canonical(
            SchemaBody::FloatingPoint(FloatWidth::W32),
            ValueDataDraft::F32(F32Bits::from_f32(value)),
        ))
        .unwrap();
        assert_eq!(snapshot.format_canonical_inline(), value.to_string());
        assert_eq!(snapshot.kind(), ValueDataKind::F32);
        assert!(
            matches!(snapshot.value().data(), ValueData::F32(bits) if bits.bits() == value.to_bits())
        );
    }
}

#[test]
fn runtime_value_snapshot_matrix_preview_preserves_rows_and_degenerate_shapes() {
    for (rows, columns, values, expected) in [
        (2, 2, vec![0.0, 1.0, 2.0, 3.0], "[0 1; 2 3]"),
        (1, 1, vec![7.0], "[7]"),
        (0, 1, vec![], "[]"),
        (1, 0, vec![], "[]"),
    ] {
        let snapshot = RuntimeValueSnapshot::from_value(canonical(
            SchemaBody::Matrix {
                element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
                dimensions: vec![
                    DimensionExpr::Constant(rows),
                    DimensionExpr::Constant(columns),
                ]
                .into_boxed_slice(),
            },
            ValueDataDraft::Matrix(
                values
                    .into_iter()
                    .map(|value| ValueDataDraft::F64(F64Bits::from_f64(value)))
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            ),
        ))
        .unwrap();
        assert_eq!(snapshot.format_canonical_inline(), expected);
    }
}
