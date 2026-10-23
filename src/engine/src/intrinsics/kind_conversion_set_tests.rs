//! Actual legacy source preparation and managed publication share the declared
//! schema/current-shape executor with resident conversion.
use super::*;
use mech_core::snapshot::EnumDraft;

fn matrix(element: SchemaBody, columns: u64) -> SchemaBody {
    SchemaBody::Matrix {
        element: Box::new(element),
        dimensions: vec![DimensionExpr::Constant(1), DimensionExpr::Constant(columns)]
            .into_boxed_slice(),
    }
}
fn set(element: SchemaBody) -> SchemaBody {
    SchemaBody::Set {
        element: Box::new(element),
        cardinality: CardinalitySpec::Dynamic { upper_bound: None },
    }
}
fn plan(source: &ValueCell, target: &SchemaBody) -> ConversionPlan {
    plan_explicit_cast(
        &source.resolved_type().unwrap(),
        &ResolvedType::from_schema_body(target, &[]).unwrap(),
    )
    .unwrap()
}
fn enum_value(ordinal: u32, text: Option<&str>) -> ValueDataDraft {
    ValueDataDraft::Enum(EnumDraft {
        ordinal,
        payload: text.map(|value| Box::new(ValueDataDraft::String(value.into()))),
    })
}

#[test]
fn declared_enum_matrix_to_set_runs_in_source_preparation_and_managed_update() {
    let event = SchemaBody::Enum {
        key: NominalKey::from_bytes([41; 32]),
        variants: vec![
            EnumVariantSchema {
                name: "none".into(),
                payload: None,
            },
            EnumVariantSchema {
                name: "word".into(),
                payload: Some(SchemaBody::String),
            },
        ]
        .into_boxed_slice(),
    };
    let schema = matrix(event.clone(), 3);
    let cell = |values: Vec<ValueDataDraft>| {
        ValueCell::from_schema_data(
            schema.clone(),
            ValueDataDraft::Matrix(values.into_boxed_slice()),
        )
        .unwrap()
    };
    let source = cell(vec![
        enum_value(1, Some("é")),
        enum_value(0, None),
        enum_value(1, Some("é")),
    ]);
    let target = set(event.clone());
    let selected = plan(&source, &target);
    assert!(selected.step.contains_matrix_to_set());
    let output = execute_conversion_plan(&source, &target, &selected).unwrap();
    assert_eq!(
        output.snapshot().unwrap().canonical_data_draft().unwrap(),
        ValueDataDraft::Set(vec![enum_value(0, None), enum_value(1, Some("é"))].into_boxed_slice())
    );
    let conversion =
        planned_type_conversion_specialized(source.clone(), output.clone(), selected).unwrap();
    source
        .replace(
            &cell(vec![
                enum_value(1, Some("é")),
                enum_value(1, Some("é")),
                enum_value(1, Some("é")),
            ])
            .snapshot()
            .unwrap(),
        )
        .unwrap();
    conversion.instance().solve_result().unwrap();
    assert_eq!(
        output.snapshot().unwrap().canonical_data_draft().unwrap(),
        ValueDataDraft::Set(
            vec![enum_value(1, Some("é")), enum_value(1, Some("é"))].into_boxed_slice()
        )
    );
    assert!(matches!(output.closed_schema_body().unwrap(),
        SchemaBody::Set { element, .. } if *element == event));
}

fn key(values: &[u16]) -> ValueDataDraft {
    ValueDataDraft::Matrix(values.iter().copied().map(ValueDataDraft::U16).collect())
}
fn byte_key(values: &[u8]) -> ValueDataDraft {
    ValueDataDraft::Matrix(values.iter().copied().map(ValueDataDraft::U8).collect())
}

#[test]
fn matrix_valued_set_keys_keep_declared_geometry_and_atomic_managed_updates() {
    let dimensions =
        vec![DimensionExpr::Constant(2), DimensionExpr::Constant(2)].into_boxed_slice();
    let source_element = SchemaBody::Matrix {
        element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W16)),
        dimensions: dimensions.clone(),
    };
    let target_element = SchemaBody::Matrix {
        element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
        dimensions,
    };
    let schema = matrix(source_element, 2);
    let cell = |left: &[u16], right: &[u16]| {
        ValueCell::from_schema_data(
            schema.clone(),
            ValueDataDraft::Matrix(vec![key(left), key(right)].into_boxed_slice()),
        )
        .unwrap()
    };
    let source = cell(&[1, 2, 3, 4], &[4, 3, 2, 1]);
    let target = set(target_element);
    let selected = plan(&source, &target);
    let output = execute_conversion_plan(&source, &target, &selected).unwrap();
    let expected = ValueDataDraft::Set(
        vec![byte_key(&[1, 2, 3, 4]), byte_key(&[4, 3, 2, 1])].into_boxed_slice(),
    );
    assert_eq!(
        output.snapshot().unwrap().canonical_data_draft().unwrap(),
        expected
    );
    let conversion =
        planned_type_conversion_specialized(source.clone(), output.clone(), selected).unwrap();
    let version = output.published_version();
    source
        .replace(&cell(&[1, 2, 3, 4], &[4, 3, 2, 300]).snapshot().unwrap())
        .unwrap();
    assert_eq!(
        conversion
            .instance()
            .solve_result()
            .unwrap_err()
            .kind_name(),
        "ConversionOutOfRange"
    );
    assert_eq!(output.published_version(), version);
    assert_eq!(
        output.snapshot().unwrap().canonical_data_draft().unwrap(),
        expected
    );
    source
        .replace(&cell(&[5, 6, 7, 8], &[5, 6, 7, 8]).snapshot().unwrap())
        .unwrap();
    conversion.instance().solve_result().unwrap();
    assert_eq!(
        output.snapshot().unwrap().canonical_data_draft().unwrap(),
        ValueDataDraft::Set(vec![byte_key(&[5, 6, 7, 8])].into_boxed_slice())
    );
    assert!(output.published_version() > version);
}
