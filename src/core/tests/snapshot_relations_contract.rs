use core::cmp::Ordering;
use mech_core::{snapshot::*, *};

fn fixture(body: SchemaBody) -> (SchemaTable, SchemaId) {
    let schema = SchemaDraft {
        dimension_parameters: Box::new([]),
        body,
    }
    .finalize()
    .unwrap();
    let mut builder = SchemaTableBuilder::new();
    let handle = builder.insert(schema).unwrap();
    let build = builder.finish().unwrap();
    let id = build.resolve(handle).unwrap();
    let (schemas, _) = build.into_parts();
    (schemas, id)
}

fn value(schemas: &SchemaTable, schema: SchemaId, data: ValueDataDraft) -> Value {
    ValueDraft {
        schema,
        shape_values: Box::new([]),
        data,
    }
    .finalize(&SnapshotValidationContext::new(schemas))
    .unwrap()
}

fn bytes(hex: &str) -> Vec<u8> {
    hex.as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let digit = |byte| match byte {
                b'0'..=b'9' => byte - b'0',
                b'a'..=b'f' => byte - b'a' + 10,
                _ => panic!("invalid hex"),
            };
            digit(pair[0]) << 4 | digit(pair[1])
        })
        .collect()
}

#[test]
fn frozen_scalar_payloads_and_value_hashes_match() {
    let (bool_schemas, bool_id) = fixture(SchemaBody::Bool);
    let bool_value = value(&bool_schemas, bool_id, ValueDataDraft::Bool(true));
    assert_eq!(
        bool_value
            .canonical_payload_bytes(&bool_schemas)
            .unwrap()
            .as_ref(),
        &[1]
    );
    assert_eq!(
        bool_value.value_hash(&bool_schemas).unwrap().as_bytes(),
        bytes("ee9e1269904567ee1e7d1e8331e7311825a0451078d43d5cebe8281dd0e956ad").as_slice()
    );

    let (f64_schemas, f64_id) = fixture(SchemaBody::FloatingPoint(FloatWidth::W64));
    let f64_value = value(
        &f64_schemas,
        f64_id,
        ValueDataDraft::F64(F64Bits::from_f64(1.5)),
    );
    assert_eq!(
        f64_value
            .canonical_payload_bytes(&f64_schemas)
            .unwrap()
            .as_ref(),
        &1.5_f64.to_bits().to_le_bytes()
    );
    assert_eq!(
        f64_value.value_hash(&f64_schemas).unwrap().as_bytes(),
        bytes("67dabee9a12a3509a8dc273d273e2369ed4439a01800ff679d87ab40706ca0d9").as_slice()
    );
}

#[test]
fn exact_language_and_key_float_relations_are_distinct() {
    let (schemas, schema) = fixture(SchemaBody::FloatingPoint(FloatWidth::W64));
    let positive_zero = value(&schemas, schema, ValueDataDraft::F64(F64Bits::from_bits(0)));
    let negative_zero = value(
        &schemas,
        schema,
        ValueDataDraft::F64(F64Bits::from_bits(1 << 63)),
    );
    assert!(
        !positive_zero
            .snapshot_eq(&schemas, &negative_zero, &schemas)
            .unwrap()
    );
    assert!(
        positive_zero
            .language_eq(&schemas, &negative_zero, &schemas)
            .unwrap()
    );
    assert_eq!(
        positive_zero
            .key_cmp(&schemas, &negative_zero, &schemas)
            .unwrap(),
        Ordering::Equal
    );
    assert_eq!(
        positive_zero.key_hash(&schemas).unwrap(),
        negative_zero.key_hash(&schemas).unwrap()
    );
    assert_eq!(
        positive_zero.key_hash(&schemas).unwrap().as_bytes(),
        bytes("4a16b2e1c1f429c50b64d45d3cf1bdea9af42c857dec939562bf696a1da690c1").as_slice()
    );

    let first_nan = value(
        &schemas,
        schema,
        ValueDataDraft::F64(F64Bits::from_bits(0x7ff0_0000_0000_0001)),
    );
    let second_nan = value(
        &schemas,
        schema,
        ValueDataDraft::F64(F64Bits::from_bits(0xfff8_0000_0000_0042)),
    );
    assert!(
        !first_nan
            .snapshot_eq(&schemas, &second_nan, &schemas)
            .unwrap()
    );
    assert!(
        !first_nan
            .language_eq(&schemas, &first_nan, &schemas)
            .unwrap()
    );
    assert_eq!(
        first_nan.key_cmp(&schemas, &second_nan, &schemas).unwrap(),
        Ordering::Equal
    );
    assert_eq!(
        first_nan.key_hash(&schemas).unwrap().as_bytes(),
        bytes("06b26a0a523b8c91b68d4d1535e92f597833c542856f74ffa6624556165ab0fb").as_slice()
    );
}

#[test]
fn sets_and_maps_are_sorted_deduplicated_and_hash_stable() {
    let (set_schemas, set_id) = fixture(SchemaBody::Set {
        element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
        cardinality: DimensionExpr::Constant(3),
    });
    let make_set = |values: [f64; 3]| {
        value(
            &set_schemas,
            set_id,
            ValueDataDraft::Set(
                values
                    .into_iter()
                    .map(|value| ValueDataDraft::F64(F64Bits::from_f64(value)))
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            ),
        )
    };
    let left = make_set([2.0, -1.0, 0.0]);
    let right = make_set([0.0, 2.0, -1.0]);
    assert!(
        left.snapshot_eq(&set_schemas, &right, &set_schemas)
            .unwrap()
    );
    assert_eq!(
        left.value_hash(&set_schemas).unwrap(),
        right.value_hash(&set_schemas).unwrap()
    );
    assert_eq!(
        left.value_hash(&set_schemas).unwrap().as_bytes(),
        bytes("e59b6570ecf303b78bf48ec4ecbc316ac6945a9959ae08ba110cfbd40937a300").as_slice()
    );

    let (map_schemas, map_id) = fixture(SchemaBody::Map {
        key: Box::new(SchemaBody::String),
        value: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W64)),
        cardinality: DimensionExpr::Constant(2),
    });
    let map = value(
        &map_schemas,
        map_id,
        ValueDataDraft::Map(
            vec![
                MapEntryDraft {
                    items: vec![ValueDataDraft::String("b".into()), ValueDataDraft::U64(2)]
                        .into_boxed_slice(),
                },
                MapEntryDraft {
                    items: vec![ValueDataDraft::String("a".into()), ValueDataDraft::U64(1)]
                        .into_boxed_slice(),
                },
            ]
            .into_boxed_slice(),
        ),
    );
    assert_eq!(
        map.value_hash(&map_schemas).unwrap().as_bytes(),
        bytes("2ed845c40652b81762a6c534c37f28749e7666305e1c0a9d27bafa943c5df4ff").as_slice()
    );
}

#[test]
fn duplicate_canonical_float_keys_are_rejected() {
    let (schemas, schema) = fixture(SchemaBody::Set {
        element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
        cardinality: DimensionExpr::Constant(2),
    });
    let error = ValueDraft {
        schema,
        shape_values: Box::new([]),
        data: ValueDataDraft::Set(
            vec![
                ValueDataDraft::F64(F64Bits::from_bits(0)),
                ValueDataDraft::F64(F64Bits::from_bits(1 << 63)),
            ]
            .into_boxed_slice(),
        ),
    }
    .finalize(&SnapshotValidationContext::new(&schemas))
    .unwrap_err();
    assert!(matches!(
        error,
        SnapshotValueError::DuplicateCanonicalKeyV1 { .. }
    ));
}

#[test]
fn rational_tuple_and_record_keys_use_semantic_order() {
    let (rational_schemas, rational_id) = fixture(SchemaBody::Rational64);
    let rational = |numerator, denominator| {
        value(
            &rational_schemas,
            rational_id,
            ValueDataDraft::Rational64 {
                numerator,
                denominator,
            },
        )
    };
    assert_eq!(
        rational(1, 2)
            .key_cmp(&rational_schemas, &rational(2, 3), &rational_schemas)
            .unwrap(),
        Ordering::Less
    );

    let (tuple_schemas, tuple_id) = fixture(SchemaBody::Tuple(
        vec![SchemaBody::Bool, SchemaBody::String].into_boxed_slice(),
    ));
    let tuple = |flag, text: &str| {
        value(
            &tuple_schemas,
            tuple_id,
            ValueDataDraft::Tuple(
                vec![
                    ValueDataDraft::Bool(flag),
                    ValueDataDraft::String(text.into()),
                ]
                .into_boxed_slice(),
            ),
        )
    };
    assert_eq!(
        tuple(false, "z")
            .key_cmp(&tuple_schemas, &tuple(true, "a"), &tuple_schemas)
            .unwrap(),
        Ordering::Less
    );
}

#[test]
fn complex_values_are_not_keys() {
    let (schemas, schema) = fixture(SchemaBody::Complex(FloatWidth::W64));
    let complex = value(
        &schemas,
        schema,
        ValueDataDraft::Complex64(Complex64Bits::new(
            F64Bits::from_f64(1.0),
            F64Bits::from_f64(2.0),
        )),
    );
    assert!(matches!(
        complex.key_cmp(&schemas, &complex, &schemas),
        Err(SnapshotValueError::SchemaNotKeyableV1)
    ));
}

#[test]
fn distinct_nominal_schemas_order_by_schema_key() {
    let (atom_a_schemas, atom_a_id) = fixture(SchemaBody::Atom(NominalKey::from_bytes([1; 32])));
    let atom_a = value(&atom_a_schemas, atom_a_id, ValueDataDraft::Atom);
    let (atom_b_schemas, atom_b_id) = fixture(SchemaBody::Atom(NominalKey::from_bytes([2; 32])));
    let atom_b = value(&atom_b_schemas, atom_b_id, ValueDataDraft::Atom);
    assert_eq!(
        atom_a
            .key_cmp(&atom_a_schemas, &atom_b, &atom_b_schemas)
            .unwrap(),
        atom_a.schema_key().cmp(&atom_b.schema_key())
    );

    let enum_body = |key| SchemaBody::Enum {
        key,
        variants: vec![EnumVariantSchema {
            name: "Only".to_owned(),
            payload: None,
        }]
        .into_boxed_slice(),
    };
    let enum_data = || {
        ValueDataDraft::Enum(EnumDraft {
            ordinal: 0,
            payload: None,
        })
    };
    let (enum_a_schemas, enum_a_id) = fixture(enum_body(NominalKey::from_bytes([3; 32])));
    let enum_a = value(&enum_a_schemas, enum_a_id, enum_data());
    let (enum_b_schemas, enum_b_id) = fixture(enum_body(NominalKey::from_bytes([4; 32])));
    let enum_b = value(&enum_b_schemas, enum_b_id, enum_data());
    assert_eq!(
        enum_a
            .key_cmp(&enum_a_schemas, &enum_b, &enum_b_schemas)
            .unwrap(),
        enum_a.schema_key().cmp(&enum_b.schema_key())
    );
    assert_eq!(
        atom_a
            .key_cmp(&atom_a_schemas, &enum_a, &enum_a_schemas)
            .unwrap(),
        atom_a.schema_key().cmp(&enum_a.schema_key())
    );

    let (bool_schemas, bool_id) = fixture(SchemaBody::Bool);
    let bool_value = value(&bool_schemas, bool_id, ValueDataDraft::Bool(false));
    let (u8_schemas, u8_id) = fixture(SchemaBody::UnsignedInteger(IntegerWidth::W8));
    let u8_value = value(&u8_schemas, u8_id, ValueDataDraft::U8(0));
    assert!(matches!(
        bool_value.key_cmp(&bool_schemas, &u8_value, &u8_schemas),
        Err(SnapshotValueError::SnapshotSchemaDefinitionMismatch { .. })
    ));
}

#[test]
fn collection_language_equality_uses_key_equality_only_for_keys() {
    let (set_schemas, set_id) = fixture(SchemaBody::Set {
        element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
        cardinality: DimensionExpr::Constant(1),
    });
    let nan_set = |bits| {
        value(
            &set_schemas,
            set_id,
            ValueDataDraft::Set(
                vec![ValueDataDraft::F64(F64Bits::from_bits(bits))].into_boxed_slice(),
            ),
        )
    };
    let canonical_nan = nan_set(0x7ff8_0000_0000_0000);
    assert!(
        canonical_nan
            .language_eq(&set_schemas, &canonical_nan, &set_schemas)
            .unwrap()
    );
    let other_nan = nan_set(0xfff0_0000_0000_0042);
    assert!(
        canonical_nan
            .language_eq(&set_schemas, &other_nan, &set_schemas)
            .unwrap()
    );

    let (map_schemas, map_id) = fixture(SchemaBody::Map {
        key: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
        value: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
        cardinality: DimensionExpr::Constant(1),
    });
    let singleton_map = |key_bits, value_bits| {
        value(
            &map_schemas,
            map_id,
            ValueDataDraft::Map(
                vec![MapEntryDraft {
                    items: vec![
                        ValueDataDraft::F64(F64Bits::from_bits(key_bits)),
                        ValueDataDraft::F64(F64Bits::from_bits(value_bits)),
                    ]
                    .into_boxed_slice(),
                }]
                .into_boxed_slice(),
            ),
        )
    };
    let positive_zero = singleton_map(0, 1.0_f64.to_bits());
    let negative_zero = singleton_map(1 << 63, 1.0_f64.to_bits());
    assert!(
        positive_zero
            .language_eq(&map_schemas, &negative_zero, &map_schemas)
            .unwrap()
    );

    let mapped_nan = singleton_map(0, 0x7ff8_0000_0000_0000);
    assert!(
        !mapped_nan
            .language_eq(&map_schemas, &mapped_nan, &map_schemas)
            .unwrap()
    );
}
