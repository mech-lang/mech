use super::*;
use crate::snapshot::MapEntryDraft;
use crate::{CardinalitySpec, SchemaDraft, SchemaTableBuilder};
use core::cell::{Cell, RefCell};

fn canonical_result(
    body: &SchemaBody,
    result: Result<ValueData, SnapshotValueError>,
) -> Result<Box<[u8]>, SnapshotValueError> {
    result.map(|data| super::super::encoding::canonical_material(body, &data))
}

fn fixture() -> (Arc<SchemaTable>, SchemaId) {
    let mut builder = SchemaTableBuilder::new();
    let handle = builder
        .insert(
            SchemaDraft {
                dimension_parameters: Box::new([]),
                body: SchemaBody::Map {
                    key: Box::new(SchemaBody::String),
                    value: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W64)),
                    cardinality: CardinalitySpec::Dynamic { upper_bound: None },
                },
            }
            .finalize()
            .unwrap(),
        )
        .unwrap();
    let built = builder.finish().unwrap();
    {
        let schema = built.resolve(handle).unwrap();
        (Arc::new(built.table), schema)
    }
}

fn entries(keys: &[(&str, u64)]) -> Box<[MapEntryDraft]> {
    keys.iter()
        .map(|(key, value)| MapEntryDraft {
            items: vec![
                ValueDataDraft::String((*key).into()),
                ValueDataDraft::U64(*value),
            ]
            .into_boxed_slice(),
        })
        .collect::<Vec<_>>()
        .into_boxed_slice()
}

// Exact eager reference for the changed Map arm. All schema, recursive leaf,
// comparison, normalization and failure machinery is the maintained machinery.
fn eager_map(
    body: &SchemaBody,
    entries: Box<[MapEntryDraft]>,
    shape: &ShapeInstance,
    context: &SnapshotValidationContext<'_>,
    path: &SnapshotPath,
) -> Result<ValueData, SnapshotValueError> {
    let SchemaBody::Map {
        key,
        value,
        cardinality,
    } = body
    else {
        unreachable!()
    };
    ensure_collection_cardinality(path, cardinality, shape, entries.len())?;
    let mut finalized = context.try_vec_with_capacity(entries.len())?;
    for (index, entry) in entries.into_vec().into_iter().enumerate() {
        if entry.items.len() != 2 {
            return Err(SnapshotValueError::MapEntryArityMismatchV1 {
                path: path.clone(),
                actual: entry.items.len() as u64,
            });
        }
        let mut items = entry.items.into_vec().into_iter();
        let key_data = finalize_data(
            key,
            items.next().unwrap(),
            shape,
            context,
            &path.child(SnapshotPathSegment::MapKey(index as u64)),
        )?;
        let value_data = finalize_data(
            value,
            items.next().unwrap(),
            shape,
            context,
            &path.child(SnapshotPathSegment::MapValue(index as u64)),
        )?;
        super::super::relations::insert_map_entry(
            key,
            &mut finalized,
            key_data,
            value_data,
            &path.child(SnapshotPathSegment::MapKey(index as u64)),
            context.canonicalization_budget,
        )?;
    }
    Ok(ValueData::Map(MapValue {
        entries: finalized.into_boxed_slice(),
    }))
}

#[derive(Default)]
struct Authority {
    calls: RefCell<Vec<(u64, u32)>>,
    fail_at: Option<usize>,
}
impl SnapshotConstructionAuthority for Authority {
    fn admit_snapshot_allocation(
        &self,
        bytes: u64,
        alignment: u32,
    ) -> Result<(), crate::MemoryRuntimeError> {
        let mut calls = self.calls.borrow_mut();
        let n = calls.len();
        calls.push((bytes, alignment));
        if self.fail_at == Some(n) {
            Err(crate::MemoryRuntimeError::BudgetExceeded {
                operation: None,
                requested: bytes,
                limit: 0,
            })
        } else {
            Ok(())
        }
    }
    fn allocation_object(&self) -> Option<crate::MemoryObjectId> {
        None
    }
}

#[test]
fn lazy_map_paths_keep_canonical_results_ordered_budgets_and_allocation_authority() {
    let (schemas, schema) = fixture();
    let definition = schemas.get(schema).unwrap();
    let shape = definition.instantiate_shape(Box::new([])).unwrap();
    let cases = [
        vec![],
        vec![("z", u64::MAX), ("", 0), ("é\0", 17), ("a", 1)],
        vec![("duplicate", 2), ("duplicate", 3)],
    ];
    for keys in cases {
        for limit in [0, 1, 2, 5, 16, 1024] {
            for fail_at in [None, Some(0), Some(1), Some(2), Some(3)] {
                let old_budget = SnapshotCanonicalizationBudget::new(limit);
                let new_budget = SnapshotCanonicalizationBudget::new(limit);
                let old_authority = Authority {
                    fail_at,
                    ..Authority::default()
                };
                let new_authority = Authority {
                    fail_at,
                    ..Authority::default()
                };
                let old_context = SnapshotValidationContext::with_shared_schemas(&schemas)
                    .with_canonicalization_budget(&old_budget)
                    .with_construction_authority(&old_authority);
                let new_context = SnapshotValidationContext::with_shared_schemas(&schemas)
                    .with_canonicalization_budget(&new_budget)
                    .with_construction_authority(&new_authority);
                let path = SnapshotPath::root().child(SnapshotPathSegment::RecordField(7));
                let old = eager_map(
                    definition.body(),
                    entries(&keys),
                    &shape,
                    &old_context,
                    &path,
                );
                let new = finalize_data(
                    definition.body(),
                    ValueDataDraft::Map(entries(&keys)),
                    &shape,
                    &new_context,
                    &path,
                );
                assert_eq!(
                    canonical_result(definition.body(), new),
                    canonical_result(definition.body(), old),
                    "{keys:?} limit{limit} fail{fail_at:?}"
                );
                assert_eq!(
                    (new_budget.consumed(), new_budget.rejected_total_work()),
                    (old_budget.consumed(), old_budget.rejected_total_work())
                );
                assert_eq!(*new_authority.calls.borrow(), *old_authority.calls.borrow());
            }
        }
    }
}

#[test]
fn lazy_map_paths_preserve_malformed_and_unsupported_error_paths() {
    let (schemas, schema) = fixture();
    let definition = schemas.get(schema).unwrap();
    let shape = definition.instantiate_shape(Box::new([])).unwrap();
    let context = SnapshotValidationContext::with_shared_schemas(&schemas);
    let path = SnapshotPath::root().child(SnapshotPathSegment::TupleElement(4));
    for items in [
        vec![],
        vec![ValueDataDraft::String("a".into())],
        vec![ValueDataDraft::U64(1), ValueDataDraft::U64(2)],
        vec![
            ValueDataDraft::String("a".into()),
            ValueDataDraft::String("wrong".into()),
        ],
    ] {
        let draft = || {
            vec![MapEntryDraft {
                items: items.clone().into_boxed_slice(),
            }]
            .into_boxed_slice()
        };
        assert_eq!(
            canonical_result(
                definition.body(),
                finalize_data(
                    definition.body(),
                    ValueDataDraft::Map(draft()),
                    &shape,
                    &context,
                    &path
                )
            ),
            canonical_result(
                definition.body(),
                eager_map(definition.body(), draft(), &shape, &context, &path)
            )
        );
    }
    let body = SchemaBody::Map {
        key: Box::new(SchemaBody::Bool),
        value: Box::new(SchemaBody::String),
        cardinality: CardinalitySpec::Dynamic {
            upper_bound: Some(crate::DimensionExpr::Constant(1)),
        },
    };
    let draft = || {
        vec![MapEntryDraft {
            items: vec![
                ValueDataDraft::Bool(true),
                ValueDataDraft::String("ok".into()),
            ]
            .into_boxed_slice(),
        }]
        .into_boxed_slice()
    };
    assert_eq!(
        canonical_result(
            &body,
            finalize_data(&body, ValueDataDraft::Map(draft()), &shape, &context, &path)
        ),
        canonical_result(&body, eager_map(&body, draft(), &shape, &context, &path))
    );
    let two = || {
        vec![
            MapEntryDraft {
                items: vec![
                    ValueDataDraft::Bool(true),
                    ValueDataDraft::String("ok".into()),
                ]
                .into_boxed_slice(),
            },
            MapEntryDraft {
                items: vec![
                    ValueDataDraft::Bool(false),
                    ValueDataDraft::String("no".into()),
                ]
                .into_boxed_slice(),
            },
        ]
        .into_boxed_slice()
    };
    assert_eq!(
        canonical_result(
            &body,
            finalize_data(&body, ValueDataDraft::Map(two()), &shape, &context, &path)
        ),
        canonical_result(&body, eager_map(&body, two(), &shape, &context, &path))
    );
}

#[test]
fn lazy_map_paths_construct_diagnostic_children_only_when_they_are_needed() {
    let (schemas, schema) = fixture();
    let shape = schemas
        .get(schema)
        .unwrap()
        .instantiate_shape(Box::new([]))
        .unwrap();
    let context = SnapshotValidationContext::with_shared_schemas(&schemas);
    let calls = Cell::new(0);
    let path = || {
        calls.set(calls.get() + 1);
        SnapshotPath::root().child(SnapshotPathSegment::MapKey(9))
    };
    let key = finalize_map_item(
        &SchemaBody::String,
        ValueDataDraft::String("key".into()),
        &shape,
        &context,
        path,
    )
    .unwrap();
    let value = finalize_map_item(
        &SchemaBody::UnsignedInteger(IntegerWidth::W64),
        ValueDataDraft::U64(u64::MAX),
        &shape,
        &context,
        path,
    )
    .unwrap();
    let mut output = Vec::new();
    super::super::relations::insert_map_entry_with_path(
        &SchemaBody::String,
        &mut output,
        key,
        value,
        path,
        None,
    )
    .unwrap();
    assert_eq!(
        calls.get(),
        0,
        "successful closed leaves and insertion allocate no diagnostic path"
    );
    let duplicate = super::super::relations::insert_map_entry_with_path(
        &SchemaBody::String,
        &mut output,
        ValueData::String("key".into()),
        ValueData::U64(7),
        path,
        None,
    )
    .unwrap_err();
    assert_eq!(calls.get(), 1);
    assert_eq!(
        duplicate,
        SnapshotValueError::DuplicateCanonicalKeyV1 {
            path: SnapshotPath::root().child(SnapshotPathSegment::MapKey(9))
        }
    );
    assert!(
        finalize_map_item(
            &SchemaBody::String,
            ValueDataDraft::U64(1),
            &shape,
            &context,
            path
        )
        .is_err()
    );
    assert_eq!(calls.get(), 2);
    let no_work = SnapshotCanonicalizationBudget::new(0);
    assert!(
        super::super::relations::insert_map_entry_with_path(
            &SchemaBody::String,
            &mut output,
            ValueData::String("other".into()),
            ValueData::U64(9),
            path,
            Some(&no_work)
        )
        .is_err()
    );
    assert_eq!(
        calls.get(),
        2,
        "comparison denial precedes duplicate-path construction"
    );
    let original = SnapshotPath::root().child(SnapshotPathSegment::RecordField(5));
    assert_eq!(
        super::super::relations::insert_map_entry(
            &SchemaBody::String,
            &mut output,
            ValueData::String("key".into()),
            ValueData::U64(0),
            &original,
            None
        )
        .unwrap_err(),
        SnapshotValueError::DuplicateCanonicalKeyV1 { path: original }
    );
}

#[test]
fn lazy_map_paths_keep_snapshot_ownership_import_failures_and_retry() {
    let (schemas, schema) = fixture();
    let draft = || ValueDraft {
        schema,
        shape_values: Box::new([]),
        data: ValueDataDraft::Map(entries(&[("z", 0), ("a", u64::MAX)])),
    };
    let raw = draft()
        .finalize(&SnapshotValidationContext::with_shared_schemas(&schemas))
        .unwrap();
    let budget = crate::ManagedMemoryBudget::new(1024 * 1024);
    let bytes = raw.memory_budget_admission_bytes(&budget).unwrap();
    for step in [0, 1] {
        let mut claim = budget.reserve_capacity(bytes).unwrap();
        let before = claim.capacity_bytes();
        budget.inject_snapshot_import_failure_after(step);
        assert!(matches!(
            raw.clone().into_independent_memory_budget(&mut claim),
            Err(crate::MemoryRuntimeError::AllocationFailed { .. })
        ));
        assert_eq!(claim.capacity_bytes(), before);
        assert_eq!(budget.retained_snapshot_nodes(), 0);
        drop(claim);
        assert_eq!(budget.used_bytes(), 0);
    }
    let mut claim = budget.reserve_capacity(bytes).unwrap();
    let imported = raw
        .clone()
        .into_independent_memory_budget(&mut claim)
        .unwrap();
    assert!(imported.shares_frozen_storage(&raw));
    assert!(budget.retained_snapshot_nodes() > 0);
    drop((imported, claim));
    assert_eq!(budget.used_bytes(), 0);
    assert_eq!(budget.retained_snapshot_nodes(), 0);
}
