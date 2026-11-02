use mech_core::snapshot::{F64Bits, ValueDataDraft as Data};
use mech_core::*;

fn plan(source_element: SchemaBody, target_element: SchemaBody) -> ConversionPlan {
    let source = ResolvedType::from_schema_body(
        &SchemaBody::Matrix {
            element: Box::new(source_element),
            dimensions: vec![DimensionExpr::Constant(2), DimensionExpr::Constant(2)]
                .into_boxed_slice(),
        },
        &[],
    )
    .unwrap();
    let target = ResolvedType::from_schema_body(
        &SchemaBody::Set {
            element: Box::new(target_element),
            cardinality: CardinalitySpec::Dynamic { upper_bound: None },
        },
        &[],
    )
    .unwrap();
    assert!(plan_implicit_conversion(&source, &target).is_err());
    plan_explicit_cast(&source, &target).unwrap()
}

fn convert(plan: &ConversionPlan, values: Vec<Data>) -> Vec<Data> {
    let Data::Set(values) =
        execute_conversion_draft(Data::Matrix(values.into_boxed_slice()), &plan.step).unwrap()
    else {
        panic!("matrix conversion must produce a Set draft")
    };
    values.into_vec()
}

#[test]
fn documented_matrix_to_set_annotation_preserves_all_elements_and_removes_duplicates() {
    let schema = SchemaBody::FloatingPoint(FloatWidth::W64);
    let plan = plan(schema.clone(), schema);
    let values = [1.1, 2.2, 3.3, 4.4]
        .map(|n| Data::F64(F64Bits::from_f64(n)))
        .to_vec();
    assert_eq!(convert(&plan, values.clone()), values);
    let values = [4.4, 1.1, 4.4, 2.2]
        .map(|n| Data::F64(F64Bits::from_f64(n)))
        .to_vec();
    assert_eq!(
        convert(&plan, values),
        [1.1, 2.2, 4.4]
            .map(|n| Data::F64(F64Bits::from_f64(n)))
            .to_vec()
    );
    assert_eq!(convert(&plan, vec![]), vec![]);
}

#[test]
fn set_conversion_uses_canonical_float_key_identity() {
    let schema = SchemaBody::FloatingPoint(FloatWidth::W64);
    let plan = plan(schema.clone(), schema);
    let values = convert(
        &plan,
        vec![
            Data::F64(F64Bits::from_f64(-0.0)),
            Data::F64(F64Bits::from_f64(0.0)),
            Data::F64(F64Bits::from_bits(0x7ff8_0000_0000_0001)),
            Data::F64(F64Bits::from_bits(0x7ff8_0000_0000_0002)),
        ],
    );
    assert_eq!(values.len(), 2);
    assert!(
        values
            .iter()
            .any(|v| matches!(v, Data::F64(bits) if bits.bits() == 0))
    );
    assert!(
        values
            .iter()
            .any(|v| matches!(v, Data::F64(bits) if bits.to_f64().is_nan()))
    );
}

#[test]
fn set_conversion_keeps_unicode_wide_integers_and_tuple_keys() {
    let text = plan(SchemaBody::String, SchemaBody::String);
    let values = convert(
        &text,
        vec![
            Data::String("🧑‍🚀".into()),
            Data::String("é".into()),
            Data::String("é".into()),
            Data::String("é".into()),
        ],
    );
    assert_eq!(values.len(), 3);
    assert!(values.contains(&Data::String("é".into())));
    assert!(values.contains(&Data::String("é".into())));
    let wide = SchemaBody::UnsignedInteger(IntegerWidth::W128);
    assert_eq!(
        convert(
            &plan(wide.clone(), wide),
            vec![
                Data::U128(u128::MAX),
                Data::U128(1 << 100),
                Data::U128(u128::MAX)
            ]
        ),
        vec![Data::U128(1 << 100), Data::U128(u128::MAX)]
    );
    let tuple = SchemaBody::Tuple(vec![SchemaBody::String, SchemaBody::Bool].into_boxed_slice());
    let key = Data::Tuple(vec![Data::String("x".into()), Data::Bool(true)].into_boxed_slice());
    assert_eq!(
        convert(&plan(tuple.clone(), tuple), vec![key.clone(), key.clone()]),
        vec![key]
    );
}

#[test]
fn element_cast_happens_before_set_key_deduplication_and_checks_errors() {
    let plan = plan(
        SchemaBody::FloatingPoint(FloatWidth::W64),
        SchemaBody::SignedInteger(IntegerWidth::W64),
    );
    assert_eq!(
        convert(
            &plan,
            [1.1, 1.9, 2.1, 2.9]
                .map(|n| Data::F64(F64Bits::from_f64(n)))
                .to_vec()
        ),
        vec![Data::I64(1), Data::I64(2)]
    );
    assert_eq!(
        execute_conversion_draft(
            Data::Matrix(vec![Data::F64(F64Bits::from_f64(f64::INFINITY))].into_boxed_slice()),
            &plan.step
        ),
        Err(ConversionExecutionError::ConversionNonFinite)
    );
    assert_eq!(
        execute_conversion_draft(Data::Bool(false), &plan.step),
        Err(ConversionExecutionError::ConversionPlanSourceMismatch)
    );
}
