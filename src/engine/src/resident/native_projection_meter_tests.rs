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
    assert!(escaped.charge_compute_work(1).is_err());
    foreign
        .with_call_limits(1000, 1000, || {
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
