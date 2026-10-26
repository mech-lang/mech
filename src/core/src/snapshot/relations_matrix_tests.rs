//! Validated packed/nested Matrix keys use the shared canonical relation.
use super::super::{
    F32Bits, F64Bits, SnapshotValidationContext, ValueDataDraft as Draft, ValueDraft,
};
use super::*;
use crate::{CardinalitySpec, IntegerWidth, NominalKey, SchemaDraft, SchemaTableBuilder};

fn matrix_body(element: SchemaBody, count: u64) -> SchemaBody {
    SchemaBody::Matrix {
        element: Box::new(element),
        dimensions: vec![
            crate::DimensionExpr::Constant(1),
            crate::DimensionExpr::Constant(count),
        ]
        .into_boxed_slice(),
    }
}
fn finalized(body: SchemaBody, data: Draft) -> (SchemaTable, Value) {
    let mut builder = SchemaTableBuilder::new();
    let handle = builder
        .insert(
            SchemaDraft {
                body,
                dimension_parameters: Box::new([]),
            }
            .finalize()
            .unwrap(),
        )
        .unwrap();
    let build = builder.finish().unwrap();
    let schema = build.resolve(handle).unwrap();
    let table = build.into_parts().0;
    let value = ValueDraft {
        schema,
        shape_values: Box::new([]),
        data,
    }
    .finalize(&SnapshotValidationContext::new(&table))
    .unwrap();
    (table, value)
}
fn check_packed(element: SchemaBody, left: Vec<Draft>, right: Vec<Draft>, expected: Ordering) {
    let body = matrix_body(element, left.len() as u64);
    let (table, left) = finalized(body.clone(), Draft::Matrix(left.into_boxed_slice()));
    let (_, right) = finalized(body.clone(), Draft::Matrix(right.into_boxed_slice()));
    assert!(table.get(left.schema()).unwrap().is_keyable());
    let exact =
        SnapshotCanonicalizationBudget::new(if expected == Ordering::Equal { 3 } else { 2 });
    assert_eq!(
        compare_key_data_with_budget(&body, left.data(), right.data(), Some(&exact)).unwrap(),
        expected
    );
    assert_eq!(
        exact.consumed(),
        if expected == Ordering::Equal { 3 } else { 2 }
    );
    assert_eq!(
        compare_key_data(&body, right.data(), left.data()).unwrap(),
        expected.reverse()
    );
}

#[test]
fn every_keyable_packed_scalar_matrix_uses_the_ordinary_scalar_order() {
    macro_rules! numeric {
        ($body:expr,$variant:ident,$left:expr,$right:expr) => {
            check_packed(
                $body,
                vec![Draft::$variant($left), Draft::$variant($left)],
                vec![Draft::$variant($right), Draft::$variant($left)],
                Ordering::Less,
            );
        };
    }
    numeric!(SchemaBody::UnsignedInteger(IntegerWidth::W8), U8, 1, 2);
    numeric!(SchemaBody::UnsignedInteger(IntegerWidth::W16), U16, 1, 2);
    numeric!(SchemaBody::UnsignedInteger(IntegerWidth::W32), U32, 1, 2);
    numeric!(SchemaBody::UnsignedInteger(IntegerWidth::W64), U64, 1, 2);
    numeric!(SchemaBody::UnsignedInteger(IntegerWidth::W128), U128, 1, 2);
    numeric!(SchemaBody::SignedInteger(IntegerWidth::W8), I8, -2, -1);
    numeric!(SchemaBody::SignedInteger(IntegerWidth::W16), I16, -2, -1);
    numeric!(SchemaBody::SignedInteger(IntegerWidth::W32), I32, -2, -1);
    numeric!(SchemaBody::SignedInteger(IntegerWidth::W64), I64, -2, -1);
    numeric!(SchemaBody::SignedInteger(IntegerWidth::W128), I128, -2, -1);
    numeric!(
        SchemaBody::FloatingPoint(FloatWidth::W32),
        F32,
        F32Bits::from_f32(-2.0),
        F32Bits::from_f32(-1.0)
    );
    numeric!(
        SchemaBody::FloatingPoint(FloatWidth::W64),
        F64,
        F64Bits::from_f64(-2.0),
        F64Bits::from_f64(-1.0)
    );
    numeric!(SchemaBody::Bool, Bool, false, true);
    numeric!(SchemaBody::Id, Id, 1, 2);
    numeric!(SchemaBody::Index, Index, 1, 2);
    check_packed(
        SchemaBody::Rational64,
        vec![
            Draft::Rational64 {
                numerator: 1,
                denominator: 2
            };
            2
        ],
        vec![
            Draft::Rational64 {
                numerator: 2,
                denominator: 3
            };
            2
        ],
        Ordering::Less,
    );
    check_packed(
        SchemaBody::Atom(NominalKey::from_bytes([87; 32])),
        vec![Draft::Atom; 2],
        vec![Draft::Atom; 2],
        Ordering::Equal,
    );
}

#[test]
fn string_matrix_comparisons_charge_raw_bytes_and_keep_partial_work_on_denial() {
    let body = matrix_body(SchemaBody::String, 2);
    let make = |last: &str| {
        finalized(
            body.clone(),
            Draft::Matrix(
                vec![Draft::String("a\0é".into()), Draft::String(last.into())].into_boxed_slice(),
            ),
        )
        .1
    };
    let left = make("x");
    let right = make("y");
    // Matrix header1 +first raw UTF-8 key4 +second key1.
    let exact = SnapshotCanonicalizationBudget::new(6);
    assert_eq!(
        compare_key_data_with_budget(&body, left.data(), right.data(), Some(&exact)).unwrap(),
        Ordering::Less
    );
    assert_eq!(exact.consumed(), 6);
    let short = SnapshotCanonicalizationBudget::new(5);
    assert_eq!(
        compare_key_data_with_budget(&body, left.data(), right.data(), Some(&short)),
        Err(SnapshotValueError::CanonicalizationWorkLimitExceededV1 { limit: 5 })
    );
    assert_eq!(short.consumed(), 5);
    assert_eq!(
        compare_key_data(&body, left.data(), left.data()).unwrap(),
        Ordering::Equal
    );
}

#[test]
fn matrix_float_keys_normalize_zero_and_nan_bits_before_storage_and_deduplication() {
    for (element, left, right) in [
        (
            SchemaBody::FloatingPoint(FloatWidth::W32),
            vec![
                Draft::F32(F32Bits::from_bits(0x80000000)),
                Draft::F32(F32Bits::from_bits(0xffa00001)),
            ],
            vec![
                Draft::F32(F32Bits::from_bits(0)),
                Draft::F32(F32Bits::from_bits(0x7fc00002)),
            ],
        ),
        (
            SchemaBody::FloatingPoint(FloatWidth::W64),
            vec![
                Draft::F64(F64Bits::from_bits(0x8000000000000000)),
                Draft::F64(F64Bits::from_bits(0xfff0000000000001)),
            ],
            vec![
                Draft::F64(F64Bits::from_bits(0)),
                Draft::F64(F64Bits::from_bits(0x7ff8000000000002)),
            ],
        ),
    ] {
        let body = matrix_body(element, 2);
        let (element_table, left_value) =
            finalized(body.clone(), Draft::Matrix(left.clone().into_boxed_slice()));
        let (_, right_value) = finalized(
            body.clone(),
            Draft::Matrix(right.clone().into_boxed_slice()),
        );
        let left_normalized = normalized_key_data(&body, left_value.data().clone()).unwrap();
        let right_normalized = normalized_key_data(&body, right_value.data().clone()).unwrap();
        assert!(exact_data_eq(&body, &left_normalized, &right_normalized));
        // Ordinary Set construction rejects duplicate keys. Exercise the SAME
        // conversion normalizer to sort/deduplicate before that finalization.
        let normalize = |first: Vec<Draft>, second: Vec<Draft>| {
            crate::snapshot::normalize_set_conversion_drafts_with_context(
                &body,
                vec![
                    Draft::Matrix(first.into_boxed_slice()),
                    Draft::Matrix(second.into_boxed_slice()),
                ]
                .into_boxed_slice(),
                left_value.shape(),
                &SnapshotValidationContext::new(&element_table),
            )
            .unwrap()
        };
        let forward_keys = normalize(left.clone(), right.clone());
        let reverse_keys = normalize(right, left);
        let set = SchemaBody::Set {
            element: Box::new(body),
            cardinality: CardinalitySpec::Dynamic { upper_bound: None },
        };
        let (table, forward) = finalized(set.clone(), Draft::Set(forward_keys));
        let (reverse_table, reverse) = finalized(set, Draft::Set(reverse_keys));
        let ValueData::Set(values) = forward.data() else {
            panic!("Set output")
        };
        assert_eq!(values.elements().len(), 1);
        assert!(
            forward
                .snapshot_eq(&table, &reverse, &reverse_table)
                .unwrap()
        );
    }
}

#[test]
fn nested_matrix_keys_recurse_without_materializing_borrowed_payloads() {
    let inner = matrix_body(SchemaBody::String, 1);
    let body = matrix_body(inner, 2);
    let make = |last: &str| {
        finalized(
            body.clone(),
            Draft::Matrix(
                ["a\0é", last]
                    .map(|text| Draft::Matrix(vec![Draft::String(text.into())].into_boxed_slice()))
                    .into(),
            ),
        )
        .1
    };
    let left = make("x");
    let right = make("y");
    let exact = SnapshotCanonicalizationBudget::new(8);
    assert_eq!(
        compare_key_data_with_budget(&body, left.data(), right.data(), Some(&exact)).unwrap(),
        Ordering::Less
    );
    assert_eq!(exact.consumed(), 8); // outer1 +inner1+4 +inner1+1.
    let empty_body = matrix_body(SchemaBody::String, 0);
    let (_, empty) = finalized(empty_body.clone(), Draft::Matrix(Box::new([])));
    let budget = SnapshotCanonicalizationBudget::new(1);
    assert_eq!(
        compare_key_data_with_budget(&empty_body, empty.data(), empty.data(), Some(&budget))
            .unwrap(),
        Ordering::Equal
    );
    assert_eq!(budget.consumed(), 1);
}
