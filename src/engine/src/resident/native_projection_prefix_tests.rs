//! Exact prepared metadata prefixes stay on the original scoped account.
use super::*;
use std::sync::{Barrier, mpsc};
use std::time::Duration;

fn policy() -> mech_core::MemoryBudgetLimits {
    let mut limits = mech_core::TargetMemoryProfile::current_resident_cpu()
        .unwrap()
        .limits;
    limits.max_compute_work = Some(1_000_000_000);
    limits.max_comparison_work = Some(1_000_000_000);
    limits
}

fn observed() -> (u64, u64) {
    with_control_work_scope(|scope| {
        let scope = scope.borrow();
        let scope = scope.as_ref().unwrap();
        (scope.observed.compute, scope.observed.comparison)
    })
}

fn with_account<T>(
    worker: bool,
    compute: u64,
    comparison: u64,
    run: impl FnOnce(&NativeLibraryWorkBudget, &mech_core::ManagedMemoryBudget) -> T,
) -> T {
    let memory = mech_core::ManagedMemoryBudget::new(8 * 1024 * 1024);
    let root = NativeLibraryWorkBudget::new(compute, comparison, &memory).unwrap();
    let group = worker.then(|| root.reserve_worker_group(compute, comparison).unwrap());
    let budget = if let Some(group) = &group {
        NativeLibraryWorkBudget::from_worker_loan(group.worker_loan().unwrap()).unwrap()
    } else {
        root.clone()
    };
    let result = run(&budget, &memory);
    drop(budget);
    if let Some(group) = group {
        let used = group.finish().unwrap();
        assert_eq!(root.used(), used);
    }
    drop(root);
    assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
    result
}

#[test]
fn prepared_prefix_commits_only_observer_delta_on_root_and_worker_aliases() {
    for worker in [false, true] {
        with_account(worker, 100, 20, |budget, _| {
            let alias = budget.clone();
            budget
                .with_call_limits(100, 20, || {
                    // Direct setup spending does not belong to this observer.
                    budget.charge(11, 2).unwrap();
                    let mut meter =
                        ResidentBudgetMeter::for_native_projection(budget, policy()).unwrap();
                    meter.charge_compute_work(3).unwrap();
                    meter.charge_comparison_work(2).unwrap();
                    assert_eq!((alias.used(), observed()), ((16, 4), (5, 2)));
                    if worker {
                        assert!(
                            !meter.can_charge_compute_and_nodes(7, 3),
                            "broad worker previews stay unchanged"
                        );
                    }
                    assert!(meter.try_charge_borrowed_footprint(7, 3).unwrap());
                    assert_eq!((alias.used(), observed()), ((23, 4), (12, 2)));
                    assert_eq!(
                        (
                            meter.estimate().compute_work(),
                            meter.estimate().retained_nodes()
                        ),
                        (12, 3)
                    );
                    // A second meter below the current high-water mark must not
                    // charge that already observed work for a second time.
                    let mut covered =
                        ResidentBudgetMeter::for_native_projection(budget, policy()).unwrap();
                    assert!(covered.try_charge_borrowed_footprint(4, 1).unwrap());
                    assert_eq!((alias.used(), observed()), ((23, 4), (12, 2)));
                    assert!(meter.try_charge_borrowed_footprint(1, 0).unwrap());
                    assert_eq!((alias.used(), observed()), ((24, 4), (13, 2)));
                })
                .unwrap();
        });
    }
}

#[test]
fn prepared_prefix_cap_overflow_and_foreign_scope_misses_are_nonspending() {
    for worker in [false, true] {
        with_account(worker, 100, 20, |budget, memory| {
            let mut escaped = budget
                .with_call_limits(9, 2, || {
                    let mut limits = policy();
                    limits.max_compute_work = Some(9);
                    limits.max_retained_nodes = Some(2);
                    let mut meter =
                        ResidentBudgetMeter::for_native_projection(budget, limits).unwrap();
                    meter.charge_compute_work(3).unwrap();
                    meter.charge_retained_nodes(1).unwrap();
                    let before = (meter, budget.used(), observed());
                    for (work, nodes) in [(7, 0), (0, 2), (u64::MAX, 0), (0, u64::MAX)] {
                        assert_eq!(meter.try_charge_borrowed_footprint(work, nodes), Ok(false));
                        assert_eq!((meter, budget.used(), observed()), before);
                    }
                    assert!(meter.try_charge_borrowed_footprint(6, 1).unwrap());
                    assert_eq!((budget.used(), observed()), ((9, 0), (9, 0)));
                    meter
                })
                .unwrap();
            let before = (escaped, budget.used());
            assert_eq!(escaped.try_charge_borrowed_footprint(0, 0), Ok(false));
            assert_eq!((escaped, budget.used()), before);
            let foreign = NativeLibraryWorkBudget::new(100, 20, memory).unwrap();
            foreign
                .with_call_limits(100, 20, || {
                    assert_eq!(escaped.try_charge_borrowed_footprint(0, 0), Ok(false));
                    assert_eq!((escaped, budget.used()), before);
                    assert!(escaped.charge_compute_work(1).is_err());
                })
                .unwrap();
            assert_eq!(foreign.used(), (0, 0));
            budget
                .with_call_limits(1, 0, || {
                    let mut fresh =
                        ResidentBudgetMeter::for_native_projection(budget, policy()).unwrap();
                    assert_eq!(fresh.try_charge_borrowed_footprint(2, 1), Ok(false));
                    assert_eq!((fresh.estimate().compute_work(), observed()), (0, (0, 0)));
                    assert!(fresh.try_charge_borrowed_footprint(1, 1).unwrap());
                })
                .unwrap();
            assert_eq!(budget.used(), (10, 0));
        });
    }
}

#[test]
fn prepared_prefix_shared_race_keeps_original_denial_prefix_and_same_account_retry() {
    fn trial(prepared: bool) -> (String, u64, (u64, u64), (u64, u64)) {
        let memory = mech_core::ManagedMemoryBudget::new(32 * 1024);
        let parent = NativeLibraryWorkBudget::new(40, 10, &memory).unwrap();
        let group = parent.reserve_worker_group(40, 10).unwrap();
        let first = group.worker_loan().unwrap();
        let sibling = group.worker_loan().unwrap();
        let previewed = Arc::new(Barrier::new(2));
        let spent = Arc::new(Barrier::new(2));
        let peer = {
            let previewed = previewed.clone();
            let spent = spent.clone();
            std::thread::spawn(move || {
                let budget = NativeLibraryWorkBudget::from_worker_loan(sibling).unwrap();
                previewed.wait();
                budget.charge(30, 0).unwrap();
                spent.wait();
            })
        };
        let result = std::thread::spawn(move || {
            let budget = NativeLibraryWorkBudget::from_worker_loan(first).unwrap();
            budget
                .with_call_limits(40, 10, || {
                    let mut meter =
                        ResidentBudgetMeter::for_native_projection(&budget, policy()).unwrap();
                    meter.charge_compute_work(3).unwrap();
                    let before = (meter, budget.used(), observed());
                    if prepared {
                        BEFORE_NATIVE_PROJECTION_PREFIX_COMMIT.with(|hook| {
                            *hook.borrow_mut() = Some(Box::new(move || {
                                previewed.wait();
                                spent.wait();
                            }));
                        });
                        assert_eq!(meter.try_charge_borrowed_footprint(9, 2), Ok(false));
                    } else {
                        previewed.wait();
                        spent.wait();
                    }
                    assert_eq!((meter, budget.used(), observed()), before);
                    meter.charge_compute_work(4).unwrap();
                    meter.charge_retained_nodes(1).unwrap();
                    let error = meter.charge_compute_work(5).unwrap_err();
                    assert_eq!(
                        (
                            meter.estimate().compute_work(),
                            meter.estimate().retained_nodes()
                        ),
                        (7, 1)
                    );
                    let denied_observed = observed();
                    if prepared {
                        assert!(meter.try_charge_borrowed_footprint(3, 1).unwrap());
                    } else {
                        meter.charge_compute_work(3).unwrap();
                        meter.charge_retained_nodes(1).unwrap();
                    }
                    assert_eq!((budget.used(), observed()), ((10, 0), (10, 0)));
                    (format!("{error:?}"), 7, denied_observed, observed())
                })
                .unwrap()
        })
        .join()
        .unwrap();
        peer.join().unwrap();
        assert_eq!(group.finish().unwrap(), (40, 0));
        assert_eq!(parent.used(), (40, 0));
        drop(parent);
        assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
        result
    }
    let ordinary = trial(false);
    assert!(ordinary.0.contains("required: 42"), "{ordinary:?}");
    assert!(ordinary.0.contains("limit: 40"), "{ordinary:?}");
    assert_eq!(trial(true), ordinary);
}

#[test]
fn prepared_prefix_pending_peer_native_fuel_declines_before_peer_release() {
    let memory = mech_core::ManagedMemoryBudget::new(32 * 1024);
    let parent = NativeLibraryWorkBudget::new(100, 100, &memory).unwrap();
    let group = parent.reserve_worker_group(100, 100).unwrap();
    let native_loan = group.worker_loan().unwrap();
    let prefix_loan = group.worker_loan().unwrap();
    let (ready_tx, ready_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let native = std::thread::spawn(move || {
        let budget = NativeLibraryWorkBudget::from_worker_loan(native_loan).unwrap();
        let fuel = budget.reserve_native_worker_fuel().unwrap().unwrap();
        assert_eq!(fuel.allowance(), (50, 50));
        ready_tx.send(()).unwrap();
        release_rx.recv().unwrap();
        fuel.finish(0, 0).unwrap();
    });
    ready_rx.recv().unwrap();
    let (result_tx, result_rx) = mpsc::channel();
    let (retry_tx, retry_rx) = mpsc::channel();
    let prefix = std::thread::spawn(move || {
        let budget = NativeLibraryWorkBudget::from_worker_loan(prefix_loan).unwrap();
        budget
            .with_call_limits(100, 100, || {
                let mut meter =
                    ResidentBudgetMeter::for_native_projection(&budget, policy()).unwrap();
                let before = (meter, budget.used(), observed());
                let result = meter.try_charge_borrowed_footprint(60, 1);
                let missed = result == Ok(false);
                result_tx
                    .send((result, (meter, budget.used(), observed()) == before))
                    .unwrap();
                retry_rx.recv().unwrap();
                if missed {
                    assert!(meter.try_charge_borrowed_footprint(60, 1).unwrap());
                }
                assert_eq!(budget.used(), (60, 0));
            })
            .unwrap();
    });
    // Always release/join both children, even if a regressed optional debit
    // waited for native fuel. The assertion then reports that blocking edge.
    let result = result_rx.recv_timeout(Duration::from_secs(2));
    release_tx.send(()).unwrap();
    native.join().unwrap();
    retry_tx.send(()).unwrap();
    prefix.join().unwrap();
    assert_eq!(result.unwrap(), (Ok(false), true));
    assert_eq!(group.finish().unwrap(), (60, 0));
    drop(parent);
    assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
}

#[test]
fn prepared_prefix_zero_delta_checks_live_and_sealed_worker_groups() {
    with_account(true, 0, 0, |budget, _| {
        budget
            .with_call_limits(0, 0, || {
                let mut meter =
                    ResidentBudgetMeter::for_native_projection(budget, policy()).unwrap();
                assert!(meter.try_charge_borrowed_footprint(0, 1).unwrap());
                assert_eq!((budget.used(), observed()), ((0, 0), (0, 0)));
                assert_eq!(meter.estimate().retained_nodes(), 1);
            })
            .unwrap();
    });
    let memory = mech_core::ManagedMemoryBudget::new(32 * 1024);
    let parent = NativeLibraryWorkBudget::new(100, 100, &memory).unwrap();
    let group = parent.reserve_worker_group(100, 100).unwrap();
    let budget = NativeLibraryWorkBudget::from_worker_loan(group.worker_loan().unwrap()).unwrap();
    drop(group);
    budget
        .with_call_limits(100, 100, || {
            let mut meter = ResidentBudgetMeter::for_native_projection(&budget, policy()).unwrap();
            let before = (meter, budget.used(), observed());
            assert_eq!(meter.try_charge_borrowed_footprint(0, 1), Ok(false));
            assert_eq!((meter, budget.used(), observed()), before);
            assert_eq!(
                meter.charge_retained_nodes(1),
                Err(ResidentKernelError::InvalidInput)
            );
            assert_eq!((meter, budget.used(), observed()), before);
        })
        .unwrap();
    drop((budget, parent));
    assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
}

#[cfg(feature = "sequence_buffers")]
#[test]
fn prepared_frame_recipe_selects_real_worker_prefix_for_current_matrix_and_scalar_fields() {
    use crate::native_artifact::{
        NativeCollectionPrimitive, NativeIterationAction, lower_native_program_artifact,
        prepare_native_program_library_call,
    };
    use mech_core::snapshot::SnapshotValidationContext;
    use mech_core::{ValueDataDraft, ValueDraft};
    use mech_syntax::document::{
        AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
        parse_canonical_document,
    };

    let parsed = parse_canonical_document(
        TextSnapshot::new(
            DocumentId(0x50524658),
            Revision(0),
            "(left<[u32]>, right<[u32]>, count<u64>)\n",
        )
        .unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
    let original = crate::CanonicalSourceFrontend
        .compile_document(&DocumentSyntax::cast(parsed.syntax()).unwrap())
        .unwrap()
        .compile_artifact()
        .unwrap();
    let decoded = crate::decode_program_artifact_bytecode_v1(
        &crate::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    for artifact in [original, decoded] {
        let mut catalog = mech_core::FunctionCatalogBuilder::new();
        crate::install_intrinsic_resident(&mut catalog).unwrap();
        let catalog = catalog.build().unwrap();
        let plan = lower_native_program_artifact(&artifact, &catalog).unwrap();
        let template = plan
            .blocks()
            .iter()
            .find_map(|block| match block.action() {
                NativeIterationAction::LibraryCall { template, .. }
                    if template.primitive() == NativeCollectionPrimitive::CompositePack(3) =>
                {
                    Some(template)
                }
                _ => None,
            })
            .unwrap();
        let call = prepare_native_program_library_call(&artifact, template, &catalog).unwrap();
        let mech_core::ResolvedOperationContract::Declared(contract) = template.contract() else {
            panic!("declared frame constructor");
        };
        for worker in [false, true] {
            let run = |allowance| {
                with_account(worker, 1_000_000_000, 1_000_000_000, |budget, memory| {
                    let values: [Value; 2] = std::array::from_fn(|index| {
                        let length = [0usize, 3][index];
                        // Both prepared matrix fields share their closed numeric
                        // element layout but retain fresh current zero/nonzero size.
                        let schema = contract.inputs[index].schema;
                        let definition = artifact.schemas().get(schema).unwrap();
                        let raw = ValueDraft {
                            schema,
                            shape_values: mech_core::shape_for_resolved_extents(
                                definition,
                                &[1, length as u64],
                            )
                            .unwrap()
                            .parameter_values()
                            .into(),
                            data: ValueDataDraft::Matrix(
                                vec![ValueDataDraft::U32(7); length].into(),
                            ),
                        }
                        .finalize(&SnapshotValidationContext::new(artifact.schemas()))
                        .unwrap();
                        let mut claim = memory
                            .reserve_capacity(raw.memory_budget_admission_bytes(memory).unwrap())
                            .unwrap();
                        raw.into_memory_budget(&mut claim).unwrap()
                    });
                    let aliases = values.clone();
                    let inputs = [Some(values[0].clone()), Some(values[1].clone()), None];
                    let held = (
                        memory.used_bytes(),
                        memory.retained_nodes(),
                        memory.retained_snapshot_nodes(),
                    );
                    let mut limits = policy();
                    limits.max_compute_work = Some(allowance);
                    NATIVE_PROJECTION_PREFIX_COMMITS.with(|count| count.set(0));
                    let result = call.admit_buffer_frame_words(
                        &inputs,
                        &[0, 0, 17],
                        limits,
                        budget,
                        ResourceDemand::default(),
                    );
                    let commits = NATIVE_PROJECTION_PREFIX_COMMITS.with(std::cell::Cell::get);
                    assert_eq!(
                        (
                            memory.used_bytes(),
                            memory.retained_nodes(),
                            memory.retained_snapshot_nodes()
                        ),
                        held
                    );
                    assert!(
                        values
                            .iter()
                            .zip(&aliases)
                            .all(|(value, alias)| value.shares_frozen_storage(alias))
                    );
                    (
                        result.err().map(|error| format!("{error:?}")),
                        budget.used().0,
                        commits,
                    )
                })
            };
            let full = run(1_000_000_000);
            assert!(full.0.is_none(), "{full:?}");
            assert!(
                full.2 > 0,
                "real prepared frame must select the exact prefix"
            );
            let exact = run(full.1);
            assert!(exact.0.is_none(), "{exact:?}");
            assert_eq!(exact.1, full.1);
            let denied = run(full.1 - 1);
            assert!(denied.0.is_some(), "{denied:?}");
            assert!(denied.1 < full.1);
            let retry = run(full.1);
            assert!(retry.0.is_none());
            assert_eq!(retry.1, full.1);
        }
    }
}

#[test]
fn prepared_prefix_denial_preserves_observe_before_policy_error_and_comparison_caps() {
    for worker in [false, true] {
        for cap in [100, 4] {
            with_account(worker, 100, 100, |budget, _| {
                budget
                    .with_call_limits(cap, 100, || {
                        let mut limits = policy();
                        limits.max_compute_work = Some(2);
                        limits.max_retained_nodes = Some(3);
                        let mut meter =
                            ResidentBudgetMeter::for_native_projection(budget, limits).unwrap();
                        let before = (meter, budget.used(), observed());
                        assert_eq!(meter.try_charge_borrowed_footprint(5, 4), Ok(false));
                        assert_eq!((meter, budget.used(), observed()), before);
                        let denied = meter.charge(|cost| {
                            cost.set_compute_work(5)?;
                            cost.set_retained_nodes(4);
                            Ok(())
                        });
                        let (dimension, required, limit, spent) = if cap == 100 {
                            (MemoryBudgetDimension::RetainedNodes, 4, 3, 5)
                        } else {
                            (MemoryBudgetDimension::ComputeWork, 5, 4, 0)
                        };
                        assert_eq!(
                            denied,
                            Err(ResidentKernelError::ResourceLimit {
                                phase: mech_core::ResidentResourcePhase::Admission,
                                dimension,
                                required,
                                limit,
                            })
                        );
                        assert_eq!(meter, before.0);
                        assert_eq!(budget.used(), (spent, 0));
                        assert!(meter.try_charge_borrowed_footprint(2, 3).unwrap());
                        assert_eq!(budget.used(), (spent.max(2), 0));
                    })
                    .unwrap();
            });
        }
        with_account(worker, 100, 3, |budget, _| {
            let mut meter = budget
                .with_call_limits(100, 3, || {
                    let mut meter =
                        ResidentBudgetMeter::for_native_projection(budget, policy()).unwrap();
                    meter.charge_comparison_work(3).unwrap();
                    meter
                })
                .unwrap();
            budget
                .with_call_limits(100, 0, || {
                    let before = (meter, budget.used(), observed());
                    assert_eq!(meter.try_charge_borrowed_footprint(0, 1), Ok(false));
                    assert_eq!((meter, budget.used(), observed()), before);
                    assert_eq!(
                        meter.charge_retained_nodes(1),
                        Err(ResidentKernelError::ResourceLimit {
                            phase: mech_core::ResidentResourcePhase::Admission,
                            dimension: MemoryBudgetDimension::ComparisonWork,
                            required: 6,
                            limit: 3,
                        })
                    );
                    assert_eq!((meter, budget.used(), observed()), before);
                })
                .unwrap();
        });
    }
}
