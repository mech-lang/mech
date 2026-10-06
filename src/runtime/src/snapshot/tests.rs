use super::*;
use mech_core::snapshot::{OptionDraft, SnapshotValidationContext};
use mech_core::{
    DimensionExpr, DimensionLifetime, DimensionParameterDeclaration, DimensionParameterId,
    DimensionParameterOrigin, FloatWidth, IntegerInterval, IntegerWidth, SchemaBody, SchemaDraft,
    SchemaTableBuilder, ValueDataDraft, ValueDraft,
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
    assert_eq!(snapshot.format_repl_kind(), "f64");
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
    assert_eq!(matrix(2, 1).format_canonical_inline(), "[3; 7]");
    assert_eq!(matrix(1, 2).format_repl_kind(), "[f64]:1,2");
    assert_eq!(matrix(2, 1).format_repl_kind(), "[f64]:2,1");
}

#[test]
fn runtime_value_snapshot_repl_kind_preserves_nonsquare_and_empty_extents() {
    for (rows, columns) in [(2, 3), (1, 3), (3, 1), (0, 3), (3, 0)] {
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
                (0..rows * columns)
                    .map(|value| ValueDataDraft::F64(F64Bits::from_f64(value as f64)))
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            ),
        ))
        .unwrap();
        assert_eq!(
            snapshot.format_repl_kind(),
            format!("[f64]:{rows},{columns}")
        );
        assert!(snapshot.value().shape().parameter_values().is_empty());
    }
}

#[test]
fn runtime_value_snapshot_repl_kind_uses_resolved_dimension_expressions() {
    let mut builder = SchemaTableBuilder::new();
    let handle = builder
        .insert(
            SchemaDraft {
                dimension_parameters: (0..2)
                    .map(|id| DimensionParameterDeclaration {
                        id: DimensionParameterId::new(id),
                        origin: DimensionParameterOrigin::Explicit,
                        lifetime: DimensionLifetime::Turn,
                        lower_bound: DimensionExpr::Constant(1),
                        upper_bound: Some(DimensionExpr::Constant(8)),
                    })
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
                body: SchemaBody::Matrix {
                    element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
                    dimensions: vec![
                        DimensionExpr::Parameter(DimensionParameterId::new(0)),
                        DimensionExpr::Add(
                            vec![
                                DimensionExpr::Parameter(DimensionParameterId::new(1)),
                                DimensionExpr::Constant(1),
                            ]
                            .into_boxed_slice(),
                        ),
                    ]
                    .into_boxed_slice(),
                },
            }
            .finalize()
            .unwrap(),
        )
        .unwrap();
    let build = builder.finish().unwrap();
    let schema = build.resolve(handle).unwrap();
    let (schemas, _) = build.into_parts();
    let value = ValueDraft {
        schema,
        shape_values: vec![2, 2].into_boxed_slice(),
        data: ValueDataDraft::Matrix(
            (0..6)
                .map(|value| ValueDataDraft::F64(F64Bits::from_f64(value as f64)))
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        ),
    }
    .finalize(&SnapshotValidationContext::new(&schemas))
    .unwrap();
    let snapshot = RuntimeValueSnapshot::from_value(value).unwrap();
    let schema_key = snapshot.schema_key();
    assert_eq!(snapshot.format_repl_kind(), "[f64]:2,3");
    assert_eq!(snapshot.schema_key(), schema_key);
    assert_eq!(snapshot.value().shape().parameter_values(), [2, 2]);
}

#[test]
fn runtime_value_snapshot_repl_kind_preserves_exact_interval_identity() {
    let lower = u128::MAX - 1;
    for upper_inclusive in [false, true] {
        let interval = SchemaBody::IntegerInterval(IntegerInterval::Unsigned {
            width: IntegerWidth::W128,
            lower,
            upper: u128::MAX,
            upper_inclusive,
        });
        let scalar = RuntimeValueSnapshot::from_value(canonical(
            interval.clone(),
            ValueDataDraft::U128(lower),
        ))
        .unwrap();
        let expected = format!(
            "u128:{lower}..{}{}",
            if upper_inclusive { "=" } else { "" },
            u128::MAX
        );
        assert_eq!(scalar.format_repl_kind(), expected);
        let matrix = RuntimeValueSnapshot::from_value(canonical(
            SchemaBody::Matrix {
                element: Box::new(interval),
                dimensions: vec![DimensionExpr::Constant(1), DimensionExpr::Constant(2)]
                    .into_boxed_slice(),
            },
            ValueDataDraft::Matrix(vec![ValueDataDraft::U128(lower); 2].into_boxed_slice()),
        ))
        .unwrap();
        assert_eq!(matrix.format_repl_kind(), format!("[{expected}]:1,2"));
    }
    let signed = RuntimeValueSnapshot::from_value(canonical(
        SchemaBody::IntegerInterval(IntegerInterval::Signed {
            width: IntegerWidth::W128,
            lower: i128::MIN,
            upper: -1,
            upper_inclusive: false,
        }),
        ValueDataDraft::I128(i128::MIN),
    ))
    .unwrap();
    assert_eq!(signed.format_repl_kind(), format!("i128:{}..-1", i128::MIN));
    let base = RuntimeValueSnapshot::from_value(canonical(
        SchemaBody::UnsignedInteger(IntegerWidth::W128),
        ValueDataDraft::U128(lower),
    ))
    .unwrap();
    assert_eq!(base.format_repl_kind(), "u128");
}

#[cfg(feature = "pretty_print")]
mod html_projection {
    use super::*;
    use mech_core::{
        SchemaField,
        snapshot::{NamedValueDraft, TableColumnDraft},
    };

    fn float(value: f64) -> ValueDataDraft {
        ValueDataDraft::F64(F64Bits::from_f64(value))
    }

    fn field(name: &str, schema: SchemaBody) -> SchemaField {
        SchemaField {
            name: name.into(),
            schema,
        }
    }

    fn record_schema() -> SchemaBody {
        SchemaBody::Record(
            vec![
                field("x", SchemaBody::FloatingPoint(FloatWidth::W64)),
                field("y", SchemaBody::FloatingPoint(FloatWidth::W64)),
            ]
            .into_boxed_slice(),
        )
    }

    fn record(x: f64, y: f64) -> ValueDataDraft {
        // Supply fields in reverse order to exercise certified schema ordering.
        ValueDataDraft::Record(
            vec![
                NamedValueDraft {
                    name: "y".into(),
                    value: float(y),
                },
                NamedValueDraft {
                    name: "x".into(),
                    value: float(x),
                },
            ]
            .into_boxed_slice(),
        )
    }

    #[test]
    fn records_preserve_names_and_numeric_values() {
        let value =
            RuntimeValueSnapshot::from_value(canonical(record_schema(), record(1.0, 2.0))).unwrap();
        assert_eq!(
            value.format_html(),
            "<table class='mech-record'><tbody><tr><th scope='row'>x</th><td><span class='mech-value'>1</span></td></tr><tr><th scope='row'>y</th><td><span class='mech-value'>2</span></td></tr></tbody></table>"
        );
        assert_eq!(value.format_repl_html(2), value.format_html());
        assert_eq!(
            value.format_repl_html(1),
            "<pre class='mech-value-preview mech-value-elided'>{1, …}</pre>"
        );
    }

    #[test]
    fn tables_preserve_row_column_order_nested_records_and_escape_text() {
        let schema = SchemaBody::Table {
            columns: vec![
                field("reading", SchemaBody::FloatingPoint(FloatWidth::W64)),
                field("label<&>", SchemaBody::String),
                field("point", record_schema()),
            ]
            .into_boxed_slice(),
            rows: DimensionExpr::Constant(2).into(),
        };
        let draft = ValueDataDraft::Table(
            vec![
                TableColumnDraft {
                    name: "reading".into(),
                    values: vec![float(1.25), float(-2.5)].into_boxed_slice(),
                },
                TableColumnDraft {
                    name: "label<&>".into(),
                    values: vec![
                        ValueDataDraft::String("<img src=x onerror=alert(1)>&\"".into()),
                        ValueDataDraft::String("second".into()),
                    ]
                    .into_boxed_slice(),
                },
                TableColumnDraft {
                    name: "point".into(),
                    values: vec![record(1.0, 2.0), record(3.0, 4.0)].into_boxed_slice(),
                },
            ]
            .into_boxed_slice(),
        );
        let snapshot = RuntimeValueSnapshot::from_value(canonical(schema, draft)).unwrap();
        let html = snapshot.format_repl_html(500);
        assert!(html.starts_with("<table class='mech-table'><thead><tr><th scope='col'>reading</th><th scope='col'>label&lt;&amp;&gt;</th><th scope='col'>point</th></tr></thead><tbody>"), "{html}");
        assert!(
            html.contains("<tbody><tr><td><span class='mech-value'>1.25</span></td>"),
            "{html}"
        );
        assert!(
            html.contains("</tr><tr><td><span class='mech-value'>-2.5</span></td>"),
            "{html}"
        );
        assert_eq!(html.matches("<table class='mech-record'>").count(), 2);
        assert!(
            html.contains("&lt;img src=x onerror=alert(1)&gt;&amp;"),
            "{html}"
        );
        assert!(html.contains("&quot;"), "{html}");
        assert!(!html.contains("<img"));
        assert!(!html.contains("F64Bits"));
        assert!(
            snapshot
                .format_repl_html(2)
                .starts_with("<pre class='mech-value-preview mech-value-elided'>")
        );
    }

    #[test]
    fn matrices_preserve_rows_columns_and_exact_scalar_html() {
        let value = RuntimeValueSnapshot::from_value(canonical(
            SchemaBody::Matrix {
                element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
                dimensions: vec![DimensionExpr::Constant(2), DimensionExpr::Constant(3)]
                    .into_boxed_slice(),
            },
            ValueDataDraft::Matrix(
                [1.0, 2.0, 3.0, 4.0, 5.0, 6.0]
                    .into_iter()
                    .map(float)
                    .collect(),
            ),
        ))
        .unwrap();
        let html = value.format_html();
        assert_eq!(html.matches("<tr>").count(), 2);
        assert_eq!(html.matches("<td>").count(), 6);
        assert!(
            html.contains("3</span></td></tr><tr><td><span class='mech-value'>4"),
            "{html}"
        );
        let exact = RuntimeValueSnapshot::from_value(canonical(
            SchemaBody::UnsignedInteger(IntegerWidth::W128),
            ValueDataDraft::U128(u128::MAX),
        ))
        .unwrap();
        assert_eq!(
            exact.format_html(),
            format!("<span class='mech-value'>{}</span>", u128::MAX)
        );
    }
}
