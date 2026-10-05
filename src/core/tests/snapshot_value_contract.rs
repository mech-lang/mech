use mech_core::{snapshot::*, *};

struct OneNamedKind {
    path: CanonicalNominalPath,
}

impl NamedKindPathResolver for OneNamedKind {
    fn canonical_path(&self, id: KindId) -> Option<&CanonicalNominalPath> {
        (id == KindId::new(7)).then_some(&self.path)
    }
}

fn table_with(
    body: SchemaBody,
    dimensions: Box<[DimensionParameterDeclaration]>,
) -> (SchemaTable, SchemaId) {
    let schema = SchemaDraft {
        dimension_parameters: dimensions,
        body,
    }
    .finalize()
    .unwrap();
    let mut builder = SchemaTableBuilder::new();
    let handle = builder.insert(schema).unwrap();
    let build = builder.finish().unwrap();
    let id = build.resolve(handle).unwrap();
    let (table, _) = build.into_parts();
    (table, id)
}

fn finalize(body: SchemaBody, data: ValueDataDraft) -> Value {
    let (schemas, schema) = table_with(body, Box::new([]));
    ValueDraft {
        schema,
        shape_values: Box::new([]),
        data,
    }
    .finalize(&SnapshotValidationContext::new(&schemas))
    .unwrap()
}

fn unsigned_interval_payload(width: IntegerWidth, value: u128) -> ValueDataDraft {
    match width {
        IntegerWidth::W8 => ValueDataDraft::U8(value.try_into().unwrap()),
        IntegerWidth::W16 => ValueDataDraft::U16(value.try_into().unwrap()),
        IntegerWidth::W32 => ValueDataDraft::U32(value.try_into().unwrap()),
        IntegerWidth::W64 => ValueDataDraft::U64(value.try_into().unwrap()),
        IntegerWidth::W128 => ValueDataDraft::U128(value),
    }
}

fn signed_interval_payload(width: IntegerWidth, value: i128) -> ValueDataDraft {
    match width {
        IntegerWidth::W8 => ValueDataDraft::I8(value.try_into().unwrap()),
        IntegerWidth::W16 => ValueDataDraft::I16(value.try_into().unwrap()),
        IntegerWidth::W32 => ValueDataDraft::I32(value.try_into().unwrap()),
        IntegerWidth::W64 => ValueDataDraft::I64(value.try_into().unwrap()),
        IntegerWidth::W128 => ValueDataDraft::I128(value),
    }
}

fn assert_interval_membership(
    interval: IntegerInterval,
    members: &[ValueDataDraft],
    nonmembers: &[ValueDataDraft],
) {
    let body = SchemaBody::IntegerInterval(interval);
    let mut builder = SchemaTableBuilder::new();
    let insert = |builder: &mut SchemaTableBuilder, body| {
        builder
            .insert(
                SchemaDraft {
                    dimension_parameters: Box::new([]),
                    body,
                }
                .finalize()
                .unwrap(),
            )
            .unwrap()
    };
    let constrained = insert(&mut builder, body.clone());
    let base = insert(&mut builder, interval.base_body());
    let build = builder.finish().unwrap();
    let constrained = build.resolve(constrained).unwrap();
    let base = build.resolve(base).unwrap();
    let schemas = build.into_parts().0;
    let context = SnapshotValidationContext::new(&schemas);
    let finalize = |schema, data: &ValueDataDraft| {
        ValueDraft {
            schema,
            shape_values: Box::new([]),
            data: data.clone(),
        }
        .finalize(&context)
    };
    for data in members {
        let accepted = finalize(constrained, data).unwrap();
        assert_eq!(accepted.schema(), constrained);
        assert!(accepted.shape().parameter_values().is_empty());
        assert_eq!(schemas.get(constrained).unwrap().body(), &body);
        assert_eq!(
            canonical_snapshot_data_draft(&body, accepted.data()).unwrap(),
            *data
        );
        let base_value = finalize(base, data).unwrap();
        let narrowed = base_value
            .rebind(constrained, base_value.shape(), &schemas)
            .unwrap();
        assert!(accepted.snapshot_eq(&schemas, &narrowed, &schemas).unwrap());
    }
    for data in nonmembers {
        assert!(matches!(
            finalize(constrained, data),
            Err(SnapshotValueError::IntegerIntervalViolationV1 { .. })
        ));
        let base_value = finalize(base, data).unwrap();
        assert!(matches!(
            base_value.rebind(constrained, base_value.shape(), &schemas),
            Err(SnapshotValueError::IntegerIntervalViolationV1 { .. })
        ));
    }
}

#[test]
fn integer_interval_membership_is_exact_at_all_integer_width_extremes() {
    for width in [
        IntegerWidth::W8,
        IntegerWidth::W16,
        IntegerWidth::W32,
        IntegerWidth::W64,
        IntegerWidth::W128,
    ] {
        let unsigned_max = if width == IntegerWidth::W128 {
            u128::MAX
        } else {
            (1_u128 << (width as u16)) - 1
        };
        let unsigned = |value| unsigned_interval_payload(width, value);
        assert_interval_membership(
            IntegerInterval::Unsigned {
                width,
                lower: unsigned_max,
                upper: unsigned_max,
                upper_inclusive: true,
            },
            &[unsigned(unsigned_max)],
            &[unsigned(unsigned_max - 1)],
        );
        assert_interval_membership(
            IntegerInterval::Unsigned {
                width,
                lower: unsigned_max - 1,
                upper: unsigned_max,
                upper_inclusive: false,
            },
            &[unsigned(unsigned_max - 1)],
            &[unsigned(unsigned_max)],
        );
        let (signed_min, signed_max) = if width == IntegerWidth::W128 {
            (i128::MIN, i128::MAX)
        } else {
            let half = 1_i128 << ((width as u16) - 1);
            (-half, half - 1)
        };
        let signed = |value| signed_interval_payload(width, value);
        for (endpoint, excluded_neighbor) in
            [(signed_min, signed_min + 1), (signed_max, signed_max - 1)]
        {
            assert_interval_membership(
                IntegerInterval::Signed {
                    width,
                    lower: endpoint,
                    upper: endpoint,
                    upper_inclusive: true,
                },
                &[signed(endpoint)],
                &[signed(excluded_neighbor)],
            );
        }
        assert_interval_membership(
            IntegerInterval::Signed {
                width,
                lower: signed_max - 1,
                upper: signed_max,
                upper_inclusive: false,
            },
            &[signed(signed_max - 1)],
            &[signed(signed_max)],
        );
        assert_interval_membership(
            IntegerInterval::Signed {
                width,
                lower: signed_min,
                upper: -1,
                upper_inclusive: true,
            },
            &[signed(signed_min), signed(-1)],
            &[signed(0)],
        );
    }
}

#[test]
fn integer_interval_membership_preserves_large_exact_payloads_and_adjacent_bounds() {
    for (width, member) in [
        (IntegerWidth::W64, (1_u128 << 53) + 1),
        (IntegerWidth::W128, (1_u128 << 100) + 1),
    ] {
        assert_interval_membership(
            IntegerInterval::Unsigned {
                width,
                lower: member,
                upper: member + 1,
                upper_inclusive: false,
            },
            &[unsigned_interval_payload(width, member)],
            &[
                unsigned_interval_payload(width, member - 1),
                unsigned_interval_payload(width, member + 1),
            ],
        );
        for member in [member as i128, -(member as i128)] {
            assert_interval_membership(
                IntegerInterval::Signed {
                    width,
                    lower: member,
                    upper: member + 1,
                    upper_inclusive: false,
                },
                &[signed_interval_payload(width, member)],
                &[
                    signed_interval_payload(width, member - 1),
                    signed_interval_payload(width, member + 1),
                ],
            );
        }
    }
}

fn finalize_reified_kind(
    kind: KindExpr,
    context: &SnapshotValidationContext<'_>,
    schema: SchemaId,
) -> Result<Value, SnapshotValueError> {
    ValueDraft {
        schema,
        shape_values: Box::new([]),
        data: ValueDataDraft::Type(ReifiedTypeDraft::Kind {
            kind,
            dimension_parameters: Box::new([]),
        }),
    }
    .finalize(context)
}

fn nominal(byte: u8) -> NominalKey {
    NominalKey::from_bytes([byte; 32])
}

#[test]
fn every_schema_body_family_finalizes_to_immutable_data() {
    let cases = vec![
        (SchemaBody::Bool, ValueDataDraft::Bool(true)),
        (
            SchemaBody::UnsignedInteger(IntegerWidth::W8),
            ValueDataDraft::U8(7),
        ),
        (
            SchemaBody::SignedInteger(IntegerWidth::W64),
            ValueDataDraft::I64(-7),
        ),
        (
            SchemaBody::FloatingPoint(FloatWidth::W32),
            ValueDataDraft::F32(F32Bits::from_bits(1)),
        ),
        (
            SchemaBody::Complex(FloatWidth::W64),
            ValueDataDraft::Complex64(Complex64Bits::new(
                F64Bits::from_f64(1.0),
                F64Bits::from_f64(-2.0),
            )),
        ),
        (
            SchemaBody::Rational64,
            ValueDataDraft::Rational64 {
                numerator: 1,
                denominator: 2,
            },
        ),
        (
            SchemaBody::String,
            ValueDataDraft::String("snapshot".to_owned()),
        ),
        (SchemaBody::Id, ValueDataDraft::Id(3)),
        (SchemaBody::Index, ValueDataDraft::Index(u64::MAX)),
        (SchemaBody::Atom(nominal(1)), ValueDataDraft::Atom),
        (
            SchemaBody::Enum {
                key: nominal(2),
                variants: vec![EnumVariantSchema {
                    name: "Some".to_owned(),
                    payload: Some(SchemaBody::Bool),
                }]
                .into_boxed_slice(),
            },
            ValueDataDraft::Enum(EnumDraft {
                ordinal: 0,
                payload: Some(Box::new(ValueDataDraft::Bool(true))),
            }),
        ),
        (
            SchemaBody::Option(Box::new(SchemaBody::Bool)),
            ValueDataDraft::Option(OptionDraft {
                present: false,
                value: None,
            }),
        ),
        (
            SchemaBody::Tuple(vec![SchemaBody::Bool].into_boxed_slice()),
            ValueDataDraft::Tuple(vec![ValueDataDraft::Bool(true)].into_boxed_slice()),
        ),
        (
            SchemaBody::Record(
                vec![SchemaField {
                    name: "x".to_owned(),
                    schema: SchemaBody::Bool,
                }]
                .into_boxed_slice(),
            ),
            ValueDataDraft::Record(
                vec![NamedValueDraft {
                    name: "x".to_owned(),
                    value: ValueDataDraft::Bool(true),
                }]
                .into_boxed_slice(),
            ),
        ),
        (
            SchemaBody::Matrix {
                element: Box::new(SchemaBody::Bool),
                dimensions: vec![DimensionExpr::Constant(1)].into_boxed_slice(),
            },
            ValueDataDraft::Matrix(vec![ValueDataDraft::Bool(true)].into_boxed_slice()),
        ),
        (
            SchemaBody::Table {
                columns: vec![SchemaField {
                    name: "x".to_owned(),
                    schema: SchemaBody::Bool,
                }]
                .into_boxed_slice(),
                rows: DimensionExpr::Constant(1).into(),
            },
            ValueDataDraft::Table(
                vec![TableColumnDraft {
                    name: "x".to_owned(),
                    values: vec![ValueDataDraft::Bool(true)].into_boxed_slice(),
                }]
                .into_boxed_slice(),
            ),
        ),
        (
            SchemaBody::Set {
                element: Box::new(SchemaBody::Bool),
                cardinality: DimensionExpr::Constant(1).into(),
            },
            ValueDataDraft::Set(vec![ValueDataDraft::Bool(true)].into_boxed_slice()),
        ),
        (
            SchemaBody::Map {
                key: Box::new(SchemaBody::Bool),
                value: Box::new(SchemaBody::String),
                cardinality: DimensionExpr::Constant(1).into(),
            },
            ValueDataDraft::Map(
                vec![MapEntryDraft {
                    items: vec![
                        ValueDataDraft::Bool(true),
                        ValueDataDraft::String("value".to_owned()),
                    ]
                    .into_boxed_slice(),
                }]
                .into_boxed_slice(),
            ),
        ),
        (
            SchemaBody::ReifiedType,
            ValueDataDraft::Type(ReifiedTypeDraft::Schema(SchemaKey::from_bytes([9; 32]))),
        ),
    ];

    for (body, data) in cases {
        finalize(body, data);
    }
}

#[test]
fn closed_reified_kinds_require_a_resolver_only_for_named_kinds() {
    let (schemas, schema) = table_with(SchemaBody::ReifiedType, Box::new([]));
    let context = SnapshotValidationContext::new(&schemas);
    for kind in [
        KindExpr::Wildcard,
        KindExpr::Never,
        KindExpr::Id,
        KindExpr::TypeOf(Box::new(KindExpr::Id)),
        KindExpr::Tuple(vec![KindExpr::Id, KindExpr::Never].into_boxed_slice()),
    ] {
        assert!(finalize_reified_kind(kind, &context, schema).is_ok());
    }

    let named = KindExpr::Named(KindId::new(7));
    assert!(matches!(
        finalize_reified_kind(named.clone(), &context, schema),
        Err(SnapshotValueError::MissingNamedKindResolver)
    ));

    let resolver = OneNamedKind {
        path: CanonicalNominalPath::new(vec!["fixture".to_owned(), "Named".to_owned()]).unwrap(),
    };
    let context = SnapshotValidationContext::with_named_kinds(&schemas, &resolver);
    assert!(finalize_reified_kind(named, &context, schema).is_ok());
}

#[test]
fn shape_and_payload_cardinality_are_checked_without_panics() {
    let dimensions = vec![DimensionParameterDeclaration {
        id: DimensionParameterId::new(0),
        origin: DimensionParameterOrigin::Explicit,
        lifetime: DimensionLifetime::Activation,
        lower_bound: DimensionExpr::Constant(1),
        upper_bound: Some(DimensionExpr::Constant(3)),
    }]
    .into_boxed_slice();
    let body = SchemaBody::Matrix {
        element: Box::new(SchemaBody::Bool),
        dimensions: vec![DimensionExpr::Parameter(DimensionParameterId::new(0))].into_boxed_slice(),
    };
    let (schemas, schema) = table_with(body, dimensions);

    let error = ValueDraft {
        schema,
        shape_values: vec![2].into_boxed_slice(),
        data: ValueDataDraft::Matrix(vec![ValueDataDraft::Bool(true)].into_boxed_slice()),
    }
    .finalize(&SnapshotValidationContext::new(&schemas))
    .unwrap_err();
    assert!(matches!(
        error,
        SnapshotValueError::PayloadCardinalityMismatchV1 {
            expected: 2,
            actual: 1,
            ..
        }
    ));
}

#[test]
fn records_and_tables_are_reordered_into_schema_order() {
    let value = finalize(
        SchemaBody::Record(
            vec![
                SchemaField {
                    name: "a".to_owned(),
                    schema: SchemaBody::Bool,
                },
                SchemaField {
                    name: "b".to_owned(),
                    schema: SchemaBody::UnsignedInteger(IntegerWidth::W8),
                },
            ]
            .into_boxed_slice(),
        ),
        ValueDataDraft::Record(
            vec![
                NamedValueDraft {
                    name: "b".to_owned(),
                    value: ValueDataDraft::U8(8),
                },
                NamedValueDraft {
                    name: "a".to_owned(),
                    value: ValueDataDraft::Bool(true),
                },
            ]
            .into_boxed_slice(),
        ),
    );
    let ValueData::Record(record) = value.data() else {
        panic!("record expected")
    };
    assert!(matches!(
        record.fields(),
        [ValueData::Bool(true), ValueData::U8(8)]
    ));
}

#[test]
fn malformed_aggregate_option_enum_and_map_forms_are_structured() {
    let tuple = finalize_error(
        SchemaBody::Tuple(vec![SchemaBody::Bool].into_boxed_slice()),
        ValueDataDraft::Tuple(Box::new([])),
    );
    assert!(matches!(
        tuple,
        SnapshotValueError::AggregateArityMismatchV1 { .. }
    ));

    let option = finalize_error(
        SchemaBody::Option(Box::new(SchemaBody::Bool)),
        ValueDataDraft::Option(OptionDraft {
            present: false,
            value: Some(Box::new(ValueDataDraft::Bool(true))),
        }),
    );
    assert!(matches!(
        option,
        SnapshotValueError::PayloadCardinalityMismatchV1 { .. }
    ));

    let map = finalize_error(
        SchemaBody::Map {
            key: Box::new(SchemaBody::Bool),
            value: Box::new(SchemaBody::Bool),
            cardinality: DimensionExpr::Constant(1).into(),
        },
        ValueDataDraft::Map(
            vec![MapEntryDraft {
                items: vec![ValueDataDraft::Bool(true)].into_boxed_slice(),
            }]
            .into_boxed_slice(),
        ),
    );
    assert!(matches!(
        map,
        SnapshotValueError::MapEntryArityMismatchV1 { actual: 1, .. }
    ));
}

fn finalize_error(body: SchemaBody, data: ValueDataDraft) -> SnapshotValueError {
    let (schemas, schema) = table_with(body, Box::new([]));
    ValueDraft {
        schema,
        shape_values: Box::new([]),
        data,
    }
    .finalize(&SnapshotValidationContext::new(&schemas))
    .unwrap_err()
}

#[test]
fn rational_input_must_already_be_canonical() {
    for (numerator, denominator) in [(2, 4), (0, 2), (1, 0)] {
        assert!(matches!(
            finalize_error(
                SchemaBody::Rational64,
                ValueDataDraft::Rational64 {
                    numerator,
                    denominator,
                },
            ),
            SnapshotValueError::NonCanonicalRationalV1
        ));
    }
}

#[test]
fn values_refuse_foreign_schema_tables_before_payload_interpretation() {
    let (first, schema) = table_with(SchemaBody::Bool, Box::new([]));
    let value = ValueDraft {
        schema,
        shape_values: Box::new([]),
        data: ValueDataDraft::Bool(true),
    }
    .finalize(&SnapshotValidationContext::new(&first))
    .unwrap();
    let (foreign, _) = table_with(SchemaBody::String, Box::new([]));
    assert!(matches!(
        value.validate_against(&foreign),
        Err(SnapshotValueError::SnapshotSchemaTableMismatch { .. })
    ));
}

#[test]
fn resident_tokens_are_canonical_and_payload_sensitive() {
    let schema = SchemaBody::Set {
        element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
        cardinality: DimensionExpr::Constant(2).into(),
    };
    let set = |values: [f64; 2]| {
        finalize(
            schema.clone(),
            ValueDataDraft::Set(
                values
                    .into_iter()
                    .map(|value| ValueDataDraft::F64(F64Bits::from_f64(value)))
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            ),
        )
    };
    let first = set([2.0, 1.0]);
    let reordered = set([1.0, 2.0]);
    let changed = set([1.0, 3.0]);
    assert_eq!(first.resident_token(), first.clone().resident_token());
    assert_eq!(first.resident_token(), reordered.resident_token());
    assert_ne!(first.resident_token(), changed.resident_token());
}
