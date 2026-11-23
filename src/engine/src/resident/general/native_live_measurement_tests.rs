//! Differential metadata admission; these fixtures grant no execution permit.
use super::data_footprint_tests::{admitted, fixture_with_len, variants};
use super::measurement::NativeMeasurementCheck;
use super::*;
use live::MeasurementCheck;
use mech_core::snapshot::StringMatrixSnapshotConstructor;
use mech_core::{ManagedMemoryBudget, SchemaBody, ValueData, snapshot::SequenceView};

struct Recording<C> {
    inner: C,
    checks: usize,
    scans: usize,
}
impl<C: MeasurementCheck> MeasurementCheck for Recording<C> {
    fn check(
        &mut self,
        footprint: CurrentMemoryFootprint,
        work: ResourceDemand,
    ) -> Result<(), MemoryPlanError> {
        self.checks += 1;
        self.inner.check(footprint, work)
    }
    fn try_metadata_scan(
        &mut self,
        value: &Value,
        body: &SchemaBody,
        footprint: CurrentMemoryFootprint,
        work: ResourceDemand,
    ) -> Result<bool, MemoryPlanError> {
        let result = self.inner.try_metadata_scan(value, body, footprint, work)?;
        self.scans += usize::from(result);
        Ok(result)
    }
}

fn fixture(
    artifact: &crate::ProgramArtifact,
    memory: &ManagedMemoryBudget,
    length: u64,
    compact: bool,
) -> (NativeCollectionCall, [Option<Value>; 2]) {
    let (call, mut inputs) = fixture_with_len(artifact, memory, length);
    if compact {
        let old = inputs[0].take().unwrap();
        let value = StringMatrixSnapshotConstructor::bind(
            old.schema(),
            old.shape().clone(),
            old.schemas().unwrap(),
        )
        .unwrap()
        .construct(
            (0..length)
                .map(|i| Box::<str>::from(["", "é\0\r\n", "🧑‍🚀"][i as usize % 3]))
                .collect(),
        )
        .unwrap();
        inputs[0] = Some(admitted(value, memory));
    }
    let ValueData::Matrix(matrix) = inputs[0].as_ref().unwrap().data() else {
        panic!("actual matrix input")
    };
    // Both ordinary draft finalization and the explicit constructor produce
    // canonical packed String storage; they differ in construction workspace.
    assert!(matches!(matrix.elements(), SequenceView::String(_)));
    (call, inputs)
}

#[derive(Debug)]
struct Outcome {
    result: Result<CurrentMemoryFootprint, MemoryPlanError>,
    observed: ResourceDemand,
    spent: (u64, u64),
    checks: usize,
    scans: usize,
}

fn measure(
    call: &NativeCollectionCall,
    input: &Option<Value>,
    memory: &ManagedMemoryBudget,
    optimized: bool,
    account: u64,
    call_limit: u64,
) -> Outcome {
    let budget = NativeLibraryWorkBudget::new(account, 0, memory).unwrap();
    budget.charge(11, 0).unwrap();
    let (result, observed, checks, scans) = budget
        .with_call_limits(call_limit, 0, || {
            let run = |inner| {
                let mut check = Recording {
                    inner,
                    checks: 0,
                    scans: 0,
                };
                let mut observed = ResourceDemand::default();
                observed.work.compute = 17;
                let result = live::footprint_with_check(
                    ResidentValueRef::Snapshot(core::slice::from_ref(input)),
                    &call.memory.inputs[0].descriptor,
                    &call.schemas,
                    &mut observed,
                    &mut check,
                );
                (result, observed, check.checks, check.scans)
            };
            let mut native = NativeMeasurementCheck::new(call, None, memory);
            if optimized {
                run(native)
            } else {
                // The old per-chunk checks use identical live authorities.
                // A plain closure has no whole-scan admission method.
                let mut observed = ResourceDemand::default();
                observed.work.compute = 17;
                let mut checks = 0;
                let result = live::footprint_with_check(
                    ResidentValueRef::Snapshot(core::slice::from_ref(input)),
                    &call.memory.inputs[0].descriptor,
                    &call.schemas,
                    &mut observed,
                    &mut |footprint: CurrentMemoryFootprint, work: ResourceDemand| {
                        checks += 1;
                        native.check(footprint, work)
                    },
                );
                (result, observed, checks, 0)
            }
        })
        .unwrap();
    Outcome {
        result,
        observed,
        spent: budget.used(),
        checks,
        scans,
    }
}

fn same(new: &Outcome, old: &Outcome) {
    assert_eq!(new.result, old.result);
    assert_eq!(new.observed, old.observed);
    assert_eq!(new.spent, old.spent);
}

fn measure_prepared_header(
    call: &NativeCollectionCall,
    input: &Option<Value>,
    memory: &ManagedMemoryBudget,
    account: u64,
    call_limit: u64,
) -> Outcome {
    let budget = NativeLibraryWorkBudget::new(account, 0, memory).unwrap();
    budget.charge(11, 0).unwrap();
    let (result, observed, checks, scans) = budget
        .with_call_limits(call_limit, 0, || {
            let mut check = Recording {
                inner: NativeMeasurementCheck::new(call, None, memory),
                checks: 0,
                scans: 0,
            };
            let mut observed = ResourceDemand::default();
            observed.work.compute = 17;
            let result = live::footprint_with_schema_check(
                ResidentValueRef::Snapshot(core::slice::from_ref(input)),
                &call.memory.inputs[0].descriptor,
                call.schemas.entry(call.inputs[0].schema_id).unwrap(),
                &mut observed,
                &mut check,
            );
            (result, observed, check.checks, check.scans)
        })
        .unwrap();
    Outcome {
        result,
        observed,
        spent: budget.used(),
        checks,
        scans,
    }
}

#[test]
fn prepared_schema_header_keeps_current_payloads_and_exact_denial_prefixes() {
    for artifact in variants() {
        for length in [0, 4, 64] {
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let (mut call, inputs) = fixture(&artifact, &memory, length, true);
            let baseline = measure(&call, &inputs[0], &memory, true, 1_000_000, 1_000_000);
            let total = baseline.observed.work.compute;
            for compute in [0, total - 1, total, total + 1] {
                call.memory.target.limits.max_compute_work = Some(compute);
                for nodes in [0, length + 2, length + 3, 1_024] {
                    call.memory.target.limits.max_retained_nodes = Some(nodes);
                    for account in [11, baseline.spent.0 - 1, baseline.spent.0 + 1] {
                        for limit in [0, total - 1, total, total + 1] {
                            let old = measure(&call, &inputs[0], &memory, true, account, limit);
                            let new =
                                measure_prepared_header(&call, &inputs[0], &memory, account, limit);
                            same(&new, &old);
                            assert_eq!((new.checks, new.scans), (old.checks, old.scans));
                        }
                    }
                }
            }
            drop(inputs);
            drop(call);
            assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
        }
    }
}

#[test]
fn prepared_schema_header_rejects_wrong_contract_and_preserves_overflow() {
    for artifact in variants() {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let (call, inputs) = fixture(&artifact, &memory, 4, true);
        let descriptor = &call.memory.inputs[0].descriptor;
        let entry = call.schemas.entry(call.inputs[0].schema_id).unwrap();
        let wrong_entry = call.schemas.entry(call.inputs[1].schema_id).unwrap();
        let mut observed = ResourceDemand::default();
        let mut checks = 0;
        assert_eq!(
            live::footprint_with_schema_check(
                ResidentValueRef::Snapshot(&inputs[..1]),
                descriptor,
                wrong_entry,
                &mut observed,
                &mut |_, _| {
                    checks += 1;
                    Ok(())
                },
            ),
            Err(MemoryPlanError::DescriptorMismatch)
        );
        assert_eq!((observed, checks), (ResourceDemand::default(), 0));
        // The prepared descriptor cannot authorize an actual value from a
        // different contract, even if both are valid canonical snapshots.
        for input in [&inputs[0], &inputs[1], &None] {
            for initial in [0, u64::MAX] {
                let run = |prepared| {
                    let mut observed = ResourceDemand::default();
                    observed.work.compute = initial;
                    let mut prefixes = Vec::new();
                    let mut check = |footprint, work| {
                        prefixes.push((footprint, work));
                        Ok(())
                    };
                    let value = ResidentValueRef::Snapshot(core::slice::from_ref(input));
                    let result = if prepared {
                        live::footprint_with_schema_check(
                            value,
                            descriptor,
                            entry,
                            &mut observed,
                            &mut check,
                        )
                    } else {
                        live::footprint_with_check(
                            value,
                            descriptor,
                            &call.schemas,
                            &mut observed,
                            &mut check,
                        )
                    };
                    (result, observed, prefixes)
                };
                assert_eq!(run(true), run(false));
            }
        }
    }
}

#[test]
fn native_live_metadata_scan_keeps_actual_sizes_and_reduces_only_checks() {
    for artifact in variants() {
        for length in [0, 1, 4, 64] {
            for compact in [false, true] {
                let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
                let (call, inputs) = fixture(&artifact, &memory, length, compact);
                let old = measure(&call, &inputs[0], &memory, false, 1_000_000, 1_000_000);
                let new = measure(&call, &inputs[0], &memory, true, 1_000_000, 1_000_000);
                assert!(old.result.is_ok(), "{old:?}");
                same(&new, &old);
                assert_eq!(new.scans, 1);
                assert_eq!(new.checks, 2, "header and final actual-facts check remain");
                assert_eq!(old.checks, length as usize + 4);
                assert_eq!(new.result.unwrap().retained_nodes, length + 3);
            }
        }
    }
}

#[test]
fn native_live_metadata_scan_preserves_exact_denial_and_spent_prefix() {
    for artifact in variants() {
        for compact in [false, true] {
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let (mut call, inputs) = fixture(&artifact, &memory, 64, compact);
            let baseline = measure(&call, &inputs[0], &memory, false, 1_000_000, 1_000_000);
            let total = baseline.observed.work.compute;
            let spent = baseline.spent.0;
            assert!(total > 2);
            for compute in [0, 1, total - 1, total, total + 1] {
                call.memory.target.limits.max_compute_work = Some(compute);
                for nodes in [0, 1, 2, 3, 65, 66, 67, 68, 1_024] {
                    call.memory.target.limits.max_retained_nodes = Some(nodes);
                    for account in [11, spent - 1, spent, spent + 1] {
                        for call_limit in [0, total - 1, total, total + 1] {
                            let old =
                                measure(&call, &inputs[0], &memory, false, account, call_limit);
                            let new =
                                measure(&call, &inputs[0], &memory, true, account, call_limit);
                            same(&new, &old);
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn native_live_metadata_preview_is_pure_and_requires_the_actual_admitted_owner() {
    let artifact = variants().into_iter().next().unwrap();
    let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
    let (mut call, inputs) = fixture(&artifact, &memory, 4, true);
    let value = inputs[0].as_ref().unwrap();
    let body = call.memory.inputs[0].descriptor.schema().body();
    let mut limited = call.clone();
    limited.memory.target.limits.max_retained_nodes = Some(0);
    let budget = NativeLibraryWorkBudget::new(1_000_000, 0, &memory).unwrap();
    budget.charge(11, 0).unwrap();
    budget
        .with_call_limits(1_000_000, 0, || {
            let mut check = NativeMeasurementCheck::new(&limited, None, &memory);
            for _ in 0..3 {
                assert!(
                    !check
                        .try_metadata_scan(
                            value,
                            body,
                            CurrentMemoryFootprint::default(),
                            ResourceDemand::default()
                        )
                        .unwrap()
                );
            }
            assert_eq!(budget.used(), (11, 0));
            let mut probe = ResourceDemand::default();
            probe.work.compute = 5;
            budget::observe_control_work(probe).unwrap();
            assert_eq!(
                budget.used(),
                (16, 0),
                "failed previews cannot advance high-water"
            );
        })
        .unwrap();
    let foreign = ManagedMemoryBudget::new(256 * 1024 * 1024);
    let mut check = NativeMeasurementCheck::new(&call, None, &foreign);
    assert!(
        !check
            .try_metadata_scan(
                value,
                body,
                CurrentMemoryFootprint::default(),
                ResourceDemand::default()
            )
            .unwrap()
    );
    for footprint in [
        CurrentMemoryFootprint {
            payload_bytes: u64::MAX,
            ..Default::default()
        },
        CurrentMemoryFootprint {
            encoded_bytes: u64::MAX,
            ..Default::default()
        },
        CurrentMemoryFootprint {
            retained_nodes: u64::MAX,
            ..Default::default()
        },
    ] {
        let mut check = NativeMeasurementCheck::new(&call, None, &memory);
        assert!(
            !check
                .try_metadata_scan(value, body, footprint, ResourceDemand::default())
                .unwrap()
        );
    }
    call.memory.target.limits.max_compute_work = Some(1_000_000);
    let body = call.memory.inputs[0].descriptor.schema().body();
    let mut check = NativeMeasurementCheck::new(&call, None, &memory);
    let mut overflow = ResourceDemand::default();
    overflow.work.compute = u64::MAX;
    assert!(
        !check
            .try_metadata_scan(value, body, CurrentMemoryFootprint::default(), overflow)
            .unwrap()
    );
    assert!(
        !check
            .try_metadata_scan(
                value,
                &SchemaBody::String,
                CurrentMemoryFootprint::default(),
                ResourceDemand::default()
            )
            .unwrap()
    );
}
