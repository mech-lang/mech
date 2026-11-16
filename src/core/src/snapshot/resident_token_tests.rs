//! On-demand receipt fingerprints keep the exact former eager algorithm.
use super::*;
use crate::{SchemaDraft, SchemaTableBuilder};

fn value(body: SchemaBody, data: ValueDataDraft, unrelated: bool) -> Value {
    let mut builder = SchemaTableBuilder::new();
    if unrelated {
        builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Bool,
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
    }
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
    let schemas = Arc::new(build.table);
    ValueDraft {
        schema,
        shape_values: Box::new([]),
        data,
    }
    .finalize(&SnapshotValidationContext::with_shared_schemas(&schemas))
    .unwrap()
}

// The old constructor's eager sequence, deliberately independent of the cache.
fn eager(value: &Value) -> u64 {
    let mut token = token_bytes(RESIDENT_TOKEN_SEED, value.schema_key.as_bytes());
    token = token_word(token, value.shape.parameter_values().len() as u64);
    for item in value.shape.parameter_values() {
        token = token_word(token, *item);
    }
    token_data(token, value.data())
}

#[test]
fn demand_token_is_exact_and_no_payload_walk_occurs_at_construction() {
    for text in ["", "a\0b", "A\r\nB", "é🧑‍🚀"] {
        let scalar = value(
            SchemaBody::String,
            ValueDataDraft::String(text.into()),
            false,
        );
        let matrix = value(
            SchemaBody::Matrix {
                element: Box::new(SchemaBody::String),
                dimensions: vec![
                    crate::DimensionExpr::Constant(3),
                    crate::DimensionExpr::Constant(1),
                ]
                .into_boxed_slice(),
            },
            ValueDataDraft::Matrix(vec![ValueDataDraft::String(text.into()); 3].into_boxed_slice()),
            false,
        );
        let compact = StringMatrixSnapshotConstructor::bind(
            matrix.schema(),
            matrix.shape().clone(),
            matrix.schemas().unwrap(),
        )
        .unwrap()
        .construct(vec![Box::<str>::from(text); 3].into_boxed_slice())
        .unwrap();
        assert_eq!(eager(&matrix), eager(&compact));
        for value in [scalar, matrix, compact] {
            assert!(value.root.data.resident_token.get().is_none());
            let expected = eager(&value);
            assert!(value.root.data.resident_token.get().is_none());
            assert_eq!(value.resident_token(), expected);
            assert_eq!(
                value.root.data.resident_token.get().map(|entry| entry.1),
                Some(expected)
            );
            assert_eq!(value.resident_token(), expected);
        }
    }
}

#[test]
fn demand_token_cache_is_shared_by_clone_rebind_and_independent_budget_wrappers() {
    let raw = value(
        SchemaBody::String,
        ValueDataDraft::String("raw\0é\r\n".into()),
        false,
    );
    let target = value(SchemaBody::String, ValueDataDraft::String("".into()), true);
    let schemas = target.schemas().unwrap();
    let rebound = raw
        .rebind_with_context(
            target.schema(),
            raw.shape(),
            &SnapshotValidationContext::with_shared_schemas(&schemas),
        )
        .unwrap();
    assert!(raw.shares_frozen_storage(&rebound));
    assert!(raw.root.data.resident_token.get().is_none());
    let memory = crate::ManagedMemoryBudget::new(1_000_000);
    let required = rebound.memory_budget_required_bytes().unwrap()
        + Value::memory_budget_claim_metadata_bytes();
    let mut reservation = memory.reserve_capacity(required).unwrap();
    let imported = rebound
        .into_independent_memory_budget(&mut reservation)
        .unwrap();
    drop(reservation);
    let before = memory.used_bytes();
    assert!(before > 0);
    assert!(imported.root.data.resident_token.get().is_none());
    let expected = eager(&raw);
    assert_eq!(imported.resident_token(), expected);
    assert_eq!(
        raw.root.data.resident_token.get().map(|entry| entry.1),
        Some(expected)
    );
    assert_eq!(raw.clone().resident_token(), expected);
    assert_eq!(
        memory.used_bytes(),
        before,
        "cache initialization has no unadmitted allocation"
    );
    drop(imported);
    assert_eq!(memory.used_bytes(), 0);
    assert_eq!(raw.resident_token(), expected);
    let equal = value(
        SchemaBody::String,
        ValueDataDraft::String("raw\0é\r\n".into()),
        false,
    );
    assert!(!raw.shares_frozen_storage(&equal));
    assert!(equal.root.data.resident_token.get().is_none());
    assert_eq!(equal.resident_token(), expected);
}

#[test]
fn changed_shape_cannot_reuse_an_old_receipt_token() {
    use crate::{
        DimensionExpr, DimensionLifetime, DimensionParameterDeclaration, DimensionParameterId,
        DimensionParameterOrigin,
    };
    let mut builder = SchemaTableBuilder::new();
    let handle = builder
        .insert(
            SchemaDraft {
                body: SchemaBody::Matrix {
                    element: Box::new(SchemaBody::Bool),
                    dimensions: vec![
                        DimensionExpr::Parameter(DimensionParameterId::new(0)),
                        DimensionExpr::Parameter(DimensionParameterId::new(1)),
                    ]
                    .into_boxed_slice(),
                },
                dimension_parameters: (0..2)
                    .map(|id| DimensionParameterDeclaration {
                        id: DimensionParameterId::new(id),
                        origin: DimensionParameterOrigin::Explicit,
                        lifetime: DimensionLifetime::Activation,
                        lower_bound: DimensionExpr::Constant(1),
                        upper_bound: Some(DimensionExpr::Constant(3)),
                    })
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            }
            .finalize()
            .unwrap(),
        )
        .unwrap();
    let build = builder.finish().unwrap();
    let schema = build.resolve(handle).unwrap();
    let schemas = Arc::new(build.table);
    let raw = ValueDraft {
        schema,
        shape_values: vec![2, 3].into_boxed_slice(),
        data: ValueDataDraft::Matrix(vec![ValueDataDraft::Bool(true); 6].into_boxed_slice()),
    }
    .finalize(&SnapshotValidationContext::with_shared_schemas(&schemas))
    .unwrap();
    let before = raw.resident_token();
    let shape = schemas
        .get(schema)
        .unwrap()
        .instantiate_shape(vec![3, 2].into_boxed_slice())
        .unwrap();
    let rebound = raw
        .rebind_with_context(
            schema,
            &shape,
            &SnapshotValidationContext::with_shared_schemas(&schemas),
        )
        .unwrap();
    assert!(!raw.shares_frozen_storage(&rebound));
    assert!(rebound.root.data.resident_token.get().is_none());
    assert_eq!(rebound.resident_token(), eager(&rebound));
    assert_ne!(rebound.resident_token(), before);
    assert_eq!(raw.resident_token(), before);
}

#[cfg(any(not(feature = "no_std"), feature = "std"))]
#[test]
fn concurrent_receipt_requests_share_one_complete_immutable_cache() {
    let raw = value(
        SchemaBody::String,
        ValueDataDraft::String("é\0".repeat(16384)),
        false,
    );
    let expected = eager(&raw);
    assert!(raw.root.data.resident_token.get().is_none());
    std::thread::scope(|scope| {
        for _ in 0..8 {
            let clone = raw.clone();
            scope.spawn(move || assert_eq!(clone.resident_token(), expected));
        }
    });
    assert_eq!(
        raw.root.data.resident_token.get().map(|entry| entry.1),
        Some(expected)
    );
}
#[test]
fn equivalent_computed_matrix_rebind_shares_payload_and_keeps_receipts() {
    use crate::{
        DimensionExpr as D, DimensionLifetime, DimensionParameterDeclaration, DimensionParameterId,
        DimensionParameterOrigin, IntegerWidth,
    };
    let mut builder = SchemaTableBuilder::new();
    let parameter = DimensionParameterId::new(0);
    let schema = |computed| {
        SchemaDraft {
            dimension_parameters: vec![DimensionParameterDeclaration {
                id: parameter,
                origin: DimensionParameterOrigin::Explicit,
                lifetime: DimensionLifetime::Turn,
                lower_bound: D::Constant(0),
                upper_bound: None,
            }]
            .into_boxed_slice(),
            body: SchemaBody::Matrix {
                element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W64)),
                dimensions: vec![
                    D::Constant(1),
                    if computed {
                        D::Add(vec![D::Constant(1), D::Parameter(parameter)].into_boxed_slice())
                    } else {
                        D::Parameter(parameter)
                    },
                ]
                .into_boxed_slice(),
            },
        }
        .finalize()
        .unwrap()
    };
    let from = builder.insert(schema(true)).unwrap();
    let to = builder.insert(schema(false)).unwrap();
    let build = builder.finish().unwrap();
    let from = build.resolve(from).unwrap();
    let to = build.resolve(to).unwrap();
    let schemas = Arc::new(build.table);
    let context = SnapshotValidationContext::with_shared_schemas(&schemas);
    let value = ValueDraft {
        schema: from,
        shape_values: vec![2].into_boxed_slice(),
        data: ValueDataDraft::Matrix(vec![ValueDataDraft::U64(7); 3].into_boxed_slice()),
    }
    .finalize(&context)
    .unwrap();
    let before = value.resident_token();
    let shape = schemas
        .get(to)
        .unwrap()
        .instantiate_shape(vec![3].into_boxed_slice())
        .unwrap();
    let rebound = value.rebind_with_context(to, &shape, &context).unwrap();
    assert!(
        value.shares_frozen_storage(&rebound),
        "reifying an equivalent extent must not copy matrix elements"
    );
    assert_eq!(rebound.resident_token(), eager(&rebound));
    assert_ne!(before, rebound.resident_token());
    assert_eq!(before, value.resident_token());
    assert_eq!(
        value.canonical_data_draft().unwrap(),
        rebound.canonical_data_draft().unwrap()
    );
    let wrong = schemas
        .get(to)
        .unwrap()
        .instantiate_shape(vec![4].into_boxed_slice())
        .unwrap();
    assert!(value.rebind_with_context(to, &wrong, &context).is_err());
    let memory = crate::ManagedMemoryBudget::new(1_000_000);
    let import = |value: Value| {
        let needed = value.memory_budget_admission_bytes(&memory).unwrap();
        let mut reservation = memory.reserve_capacity(needed).unwrap();
        let imported = value.into_memory_budget(&mut reservation).unwrap();
        drop(reservation);
        imported
    };
    let original = import(value.clone());
    // Resident inputs already carry a budget wrapper. Rebinding must not
    // mistake that old wrapper for admission of the new shape owner.
    let rebound = original.rebind_with_context(to, &shape, &context).unwrap();
    let before = memory.used_bytes();
    assert!(before > 0);
    // Sharing the data does not mean the new shape metadata is admitted.
    assert!(rebound.memory_budget_retained_bytes(&memory).is_none());
    let needed = rebound.memory_budget_admission_bytes(&memory).unwrap();
    assert!(needed > 0);
    let mut denied = memory.reserve_capacity(needed - 1).unwrap();
    assert!(rebound.clone().into_memory_budget(&mut denied).is_err());
    drop(denied);
    assert_eq!(memory.used_bytes(), before);
    let imported = import(rebound.clone());
    assert!(original.shares_frozen_storage(&imported));
    assert_eq!(imported.shape().parameter_values(), &[3]);
    assert_eq!(original.shape().parameter_values(), &[2]);
    let admitted = memory.used_bytes();
    let repeated = import(imported.clone());
    assert_eq!(memory.used_bytes(), admitted);
    assert_eq!(repeated.resident_token(), eager(&repeated));
    drop((original, imported, repeated, rebound));
    assert_eq!(memory.used_bytes(), 0);
}
