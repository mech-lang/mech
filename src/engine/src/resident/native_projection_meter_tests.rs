//! Borrowed native measurement never creates a resident execution permit.
use super::*;

fn limits() -> mech_core::MemoryBudgetLimits {
    let mut limits = mech_core::TargetMemoryProfile::current_resident_cpu()
        .unwrap()
        .limits;
    limits.max_compute_work = Some(1000);
    limits.max_comparison_work = Some(1000);
    limits
}

#[test]
fn native_projection_meter_requires_its_live_work_scope() {
    let memory = mech_core::ManagedMemoryBudget::new(1024 * 1024);
    let work = NativeLibraryWorkBudget::new(1000, 1000, &memory).unwrap();
    let foreign = NativeLibraryWorkBudget::new(1000, 1000, &memory).unwrap();
    assert!(ResidentBudgetMeter::for_native_projection(&work, limits()).is_err());
    let mut escaped = work
        .with_call_limits(1000, 1000, || {
            assert!(ResidentBudgetMeter::for_native_projection(&foreign, limits()).is_err());
            ResidentBudgetMeter::for_native_projection(&work, limits()).unwrap()
        })
        .unwrap();
    assert!(!escaped.can_charge_compute_and_nodes(1, 0));
    assert!(escaped.charge_compute_work(1).is_err());
    foreign
        .with_call_limits(1000, 1000, || {
            assert!(!escaped.can_charge_compute_and_nodes(1, 0));
            assert!(escaped.charge_compute_work(1).is_err());
        })
        .unwrap();
    assert_eq!(work.used(), (0, 0));
    assert_eq!(foreign.used(), (0, 0));
}

#[test]
fn native_projection_meter_debits_the_existing_ledger_and_tighter_call_caps() {
    let memory = mech_core::ManagedMemoryBudget::new(1024 * 1024);
    let work = NativeLibraryWorkBudget::new(20, 10, &memory).unwrap();
    work.with_call_limits(20, 10, || {
        let mut meter = ResidentBudgetMeter::for_native_projection(&work, limits()).unwrap();
        meter.charge_compute_work(4).unwrap();
        meter.charge_comparison_work(3).unwrap();
        assert_eq!(work.used(), (7, 3));
        assert_eq!(meter.remaining_incremental_work().unwrap(), 7);
        work.with_call_limits(2, 2, || {
            let mut inner = ResidentBudgetMeter::for_native_projection(&work, limits()).unwrap();
            inner.charge_comparison_work(2).unwrap();
            assert!(inner.charge_compute_work(1).is_err());
        })
        .unwrap();
        assert_eq!(work.used(), (9, 5));
    })
    .unwrap();
    work.with_call_limits(20, 10, || {
        let mut meter = ResidentBudgetMeter::for_native_projection(&work, limits()).unwrap();
        meter.charge_comparison_work(5).unwrap();
        assert!(meter.charge_comparison_work(1).is_err());
    })
    .unwrap();
    assert_eq!(work.used(), (14, 10));
}

#[test]
fn native_projection_meter_checks_current_node_and_byte_limits() {
    let memory = mech_core::ManagedMemoryBudget::new(1024 * 1024);
    let work = NativeLibraryWorkBudget::new(1000, 1000, &memory).unwrap();
    work.with_call_limits(1000, 1000, || {
        let mut tight = limits();
        tight.max_retained_nodes = Some(2);
        let mut nodes = ResidentBudgetMeter::for_native_projection(&work, tight).unwrap();
        nodes.charge_retained_nodes(2).unwrap();
        assert!(nodes.charge_retained_nodes(1).is_err());
        tight.max_temporary_bytes = Some(3);
        let mut bytes = ResidentBudgetMeter::for_native_projection(&work, tight).unwrap();
        bytes.charge_temporary_bytes(3).unwrap();
        assert!(bytes.charge_temporary_bytes(1).is_err());
        let mut missing = limits();
        missing.max_compute_work = None;
        assert!(ResidentBudgetMeter::for_native_projection(&work, missing).is_err());
    })
    .unwrap();
}

#[test]
fn native_projection_whole_metadata_preview_uses_its_scope_without_spending() {
    let memory = mech_core::ManagedMemoryBudget::new(1024 * 1024);
    let work = NativeLibraryWorkBudget::new(1000, 1000, &memory).unwrap();
    work.with_call_limits(200, 100, || {
        let mut policy = limits();
        policy.max_retained_nodes = Some(3);
        let mut meter = ResidentBudgetMeter::for_native_projection(&work, policy).unwrap();
        meter.charge_compute_work(17).unwrap();
        let before = (meter, work.used());
        assert!(meter.can_charge_compute_and_nodes(128, 2));
        assert!(!meter.can_charge_compute_and_nodes(184, 2));
        assert!(!meter.can_charge_compute_and_nodes(128, 4));
        assert!(!meter.can_charge_compute_and_nodes(u64::MAX, 0));
        assert_eq!((meter, work.used()), before);
        assert!(meter.try_charge_borrowed_metadata_scan(2, true).unwrap());
        assert_eq!(work.used(), (145, 0));
        assert_eq!(meter.estimate().retained_nodes(), 2);
        assert!(!meter.try_charge_borrowed_metadata_scan(1, true).unwrap());
        assert_eq!(work.used(), (145, 0));
    })
    .unwrap();
    drop(work);
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn denied_native_projection_preflight_keeps_spent_comparisons_on_retry() {
    let body = SchemaBody::Map {
        key: Box::new(SchemaBody::String),
        value: Box::new(SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64)),
        cardinality: mech_core::CardinalitySpec::Dynamic { upper_bound: None },
    };
    let mut builder = mech_core::SchemaTableBuilder::new();
    let handle = builder
        .insert(
            mech_core::SchemaDraft {
                dimension_parameters: Box::new([]),
                body: body.clone(),
            }
            .finalize()
            .unwrap(),
        )
        .unwrap();
    let build = builder.finish().unwrap();
    let source = mech_core::ValueDraft {
        schema: build.resolve(handle).unwrap(),
        shape_values: Box::new([]),
        data: mech_core::ValueDataDraft::Map(
            (0..32)
                .map(|key| mech_core::snapshot::MapEntryDraft {
                    items: vec![
                        mech_core::ValueDataDraft::String(format!("key-{key:02}")),
                        mech_core::ValueDataDraft::U64(1),
                    ]
                    .into_boxed_slice(),
                })
                .collect(),
        ),
    }
    .finalize(&mech_core::snapshot::SnapshotValidationContext::new(
        &build.table,
    ))
    .unwrap();
    let memory = mech_core::ManagedMemoryBudget::new(1024 * 1024);
    let work = NativeLibraryWorkBudget::new(1_000_000, 100, &memory).unwrap();
    let mut policy = limits();
    policy.max_compute_work = Some(1_000_000);
    policy.max_comparison_work = Some(100);
    let mut previous = 0;
    for _ in 0..3 {
        work.with_call_limits(1_000_000, 100, || {
            let mut meter = ResidentBudgetMeter::for_native_projection(&work, policy).unwrap();
            assert!(
                preflight_canonical_data_finalization(&mut meter, &body, source.data()).is_err()
            );
        })
        .unwrap();
        let spent = work.used().1;
        assert!(spent > 0 && spent >= previous && spent <= 100);
        previous = spent;
    }
}

fn layout_fixture() -> SchemaTable {
    let mut builder = mech_core::SchemaTableBuilder::new();
    for body in [
        SchemaBody::String,
        SchemaBody::Tuple(
            vec![
                SchemaBody::Record(
                    vec![mech_core::SchemaField {
                        name: "unrelated-λ".into(),
                        schema: SchemaBody::Option(Box::new(SchemaBody::Tuple(
                            vec![SchemaBody::Bool; 32].into_boxed_slice(),
                        ))),
                    }]
                    .into_boxed_slice(),
                ),
                SchemaBody::Matrix {
                    element: Box::new(SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W16)),
                    dimensions: vec![mech_core::DimensionExpr::Constant(262_144)]
                        .into_boxed_slice(),
                },
            ]
            .into_boxed_slice(),
        ),
    ] {
        builder
            .insert(
                mech_core::SchemaDraft {
                    dimension_parameters: Box::new([]),
                    body,
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
    }
    builder.finish().unwrap().table
}

#[test]
fn native_schema_precharge_matches_ordered_visitor_at_every_denial_boundary() {
    let schemas = layout_fixture();
    let mut contributions = Vec::new();
    let full = schemas
        .clone_allocation_bound_bytes_with_precharge(|bytes| {
            contributions.push(bytes);
            Ok::<_, ResidentKernelError>(())
        })
        .unwrap();
    assert_eq!(Some(full), schemas.clone_allocation_bound_bytes());
    let execute = |legacy, cap, account, call, base| {
        let memory = mech_core::ManagedMemoryBudget::new(1 << 20);
        let work = NativeLibraryWorkBudget::new(account, 1000, &memory).unwrap();
        let answer = work
            .with_call_limits(call, 1000, || {
                let mut policy = limits();
                policy.max_compute_work = Some(cap);
                let mut meter = ResidentBudgetMeter::for_native_projection(&work, policy).unwrap();
                meter.charge_compute_work(base).unwrap();
                let result = if legacy {
                    schemas
                        .clone_allocation_bound_bytes_with_precharge(|bytes| {
                            meter.charge_compute_work(bytes)
                        })
                        .map_err(|error| match error {
                            mech_core::SchemaAllocationBoundError::Precharge(error) => error,
                            mech_core::SchemaAllocationBoundError::ArithmeticOverflow => {
                                ResidentKernelError::InvalidShape
                            }
                        })
                } else {
                    meter.precharge_schema_layout(&schemas)
                };
                (result, meter.estimate(), work.used())
            })
            .unwrap();
        drop(work);
        assert_eq!(memory.used_bytes(), 0);
        answer
    };
    let base = 17;
    let mut caps = vec![base, base + full - 1, base + full, base + full + 1];
    let mut prefix = base;
    for part in contributions {
        if part > 0 {
            caps.extend([prefix + part - 1, prefix + part]);
        }
        prefix += part;
    }
    for cap in caps {
        assert_eq!(
            execute(false, cap, base + full + 1, base + full + 1, base),
            execute(true, cap, base + full + 1, base + full + 1, base),
            "policy cap {cap}"
        );
        assert_eq!(
            execute(false, base + full + 1, cap, base + full + 1, base),
            execute(true, base + full + 1, cap, base + full + 1, base),
            "account cap {cap}"
        );
        assert_eq!(
            execute(false, base + full + 1, base + full + 1, cap, base),
            execute(true, base + full + 1, base + full + 1, cap, base),
            "call cap {cap}"
        );
    }
    assert_eq!(
        execute(false, u64::MAX, u64::MAX, u64::MAX, u64::MAX - 1),
        execute(true, u64::MAX, u64::MAX, u64::MAX, u64::MAX - 1)
    );
}

#[test]
fn native_schema_precharge_keeps_shared_workers_incremental() {
    let schemas = layout_fixture();
    let full = schemas.clone_allocation_bound_bytes().unwrap();
    let memory = mech_core::ManagedMemoryBudget::new(1 << 20);
    let parent = NativeLibraryWorkBudget::new(full, 1000, &memory).unwrap();
    let group = parent.reserve_worker_group(full, 1000).unwrap();
    let worker = NativeLibraryWorkBudget::from_worker_loan(group.worker_loan().unwrap()).unwrap();
    worker
        .with_call_limits(full, 1000, || {
            let mut policy = limits();
            policy.max_compute_work = Some(full);
            let mut meter = ResidentBudgetMeter::for_native_projection(&worker, policy).unwrap();
            assert!(!meter.can_charge_compute_and_nodes(full, 0));
            assert_eq!(meter.precharge_schema_layout(&schemas), Ok(full));
        })
        .unwrap();
    assert_eq!(worker.used(), (full, 0));
    drop(worker);
    assert_eq!(group.finish().unwrap(), (full, 0));
    assert_eq!(parent.used(), (full, 0));
    drop(parent);
    assert_eq!(memory.used_bytes(), 0);
}
