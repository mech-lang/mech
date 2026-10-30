//! Whole-scan admission must preserve the original incremental accounting.
use super::*;
use mech_core::snapshot::{SnapshotValidationContext, StringMatrixSnapshotConstructor};
use mech_core::{DimensionExpr, SchemaDraft, SchemaTableBuilder, ValueDataDraft, ValueDraft};

fn string_matrix(strings: &[&str], compact: bool) -> (Value, Arc<SchemaTable>) {
    let mut builder = SchemaTableBuilder::new();
    let handle = builder
        .insert(
            SchemaDraft {
                body: SchemaBody::Matrix {
                    element: Box::new(SchemaBody::String),
                    dimensions: vec![
                        DimensionExpr::Constant(1),
                        DimensionExpr::Constant(strings.len() as u64),
                    ]
                    .into_boxed_slice(),
                },
                dimension_parameters: Box::new([]),
            }
            .finalize()
            .unwrap(),
        )
        .unwrap();
    let build = builder.finish().unwrap();
    let schema = build.resolve(handle).unwrap();
    let schemas = Arc::new(build.table);
    let value = ValueDraft {
        schema,
        shape_values: Box::new([]),
        data: ValueDataDraft::Matrix(
            strings
                .iter()
                .map(|text| ValueDataDraft::String((*text).into()))
                .collect(),
        ),
    }
    .finalize(&SnapshotValidationContext::with_shared_schemas(&schemas))
    .unwrap();
    let value = if compact {
        StringMatrixSnapshotConstructor::bind(schema, value.shape().clone(), schemas.clone())
            .unwrap()
            .construct(strings.iter().map(|text| Box::<str>::from(*text)).collect())
            .unwrap()
    } else {
        value
    };
    (value, schemas)
}

// The pre-optimization visitor is an independent accounting oracle. In
// particular, a failed retained-node charge may already have spent compute.
fn incremental(
    meter: &mut ResidentBudgetMeter,
    schema: &SchemaBody,
    data: &ValueData,
    retain: bool,
) -> Result<ValueFootprint, ResidentKernelError> {
    let mut footprint = ValueFootprint::zero();
    mech_core::snapshot::visit_canonical_data_work(schema, data, |chunk| {
        meter.charge_compute_work(checked_cost_product(&[
            BORROWED_FOOTPRINT_METADATA_WORK,
            chunk.node_count.max(1),
        ])?)?;
        if retain {
            meter.charge_retained_nodes(chunk.node_count)?;
        }
        footprint = footprint
            .checked_add(ValueFootprint {
                encoded_bytes: chunk.encoded_bytes,
                retained_bytes: chunk.retained_bytes,
                node_count: chunk.node_count,
            })
            .map_err(|_| ResidentKernelError::InvalidShape)?;
        Ok(())
    })
    .map_err(|error| match error {
        mech_core::snapshot::CanonicalDataWorkError::Visitor(error) => error,
        _ => ResidentKernelError::InvalidInput,
    })?;
    Ok(footprint)
}

#[test]
fn whole_metadata_scan_matches_incremental_closed_analysis_at_every_boundary() {
    let long = "é\0\r\n🧑‍🚀".repeat(2_048);
    for strings in [
        vec![],
        vec![""],
        vec!["é", "a\0b\r\n", &long],
        vec!["x"; 129],
    ] {
        for compact in [false, true] {
            let (value, schemas) = string_matrix(&strings, compact);
            let body = schemas.get(value.schema()).unwrap().body();
            for retain in [false, true] {
                let mut baseline = ResidentBudgetMeter::for_closed_analysis(1_000_000, 0);
                let expected = incremental(&mut baseline, body, value.data(), retain).unwrap();
                let work = baseline.estimate().compute_work();
                for limit in [0, 1, 63, 64, 65, 127, 128, 129, work - 1, work, work + 1] {
                    let mut old = ResidentBudgetMeter::for_closed_analysis(limit, 0);
                    let mut new = old;
                    assert_eq!(
                        measure_proven_borrowed_canonical_data_footprint(
                            &mut new,
                            body,
                            value.data(),
                            retain
                        ),
                        incremental(&mut old, body, value.data(), retain),
                        "compact={compact}, retain={retain}, limit={limit}"
                    );
                    assert_eq!(new, old, "denial must retain the identical paid prefix");
                }
                assert_eq!(
                    expected.encoded_bytes,
                    strings.iter().map(|s| s.len() as u64 + 8).sum::<u64>()
                );
                if compact {
                    assert_eq!(expected.node_count, strings.len() as u64 + 2);
                }
            }
        }
    }
}

#[cfg(feature = "native-artifact")]
#[test]
fn whole_metadata_scan_keeps_real_turn_account_call_and_node_denials_identical() {
    let (value, schemas) = string_matrix(&["", "é\0", "a\r\nb", "🧑‍🚀"], true);
    let body = schemas.get(value.schema()).unwrap().body();
    let execute = |legacy, retain, compute, nodes, account, call| {
        let memory = mech_core::ManagedMemoryBudget::new(4_096);
        let budget = NativeLibraryWorkBudget::new(account, 0, &memory).unwrap();
        budget.charge(11, 0).unwrap();
        let mut plan = detached_turn_plan(ResourceDemand::default()).unwrap();
        plan.facts.additional_demand.work.compute = 17;
        plan.budget_limits.max_compute_work = Some(compute);
        plan.budget_limits.max_retained_nodes = Some(nodes);
        with_resident_turn_plan(plan, || {
            budget
                .with_call_limits(call, 0, || {
                    let mut meter = ResidentBudgetMeter::default();
                    meter.charge_compute_work(31).unwrap();
                    let result = if legacy {
                        incremental(&mut meter, body, value.data(), retain)
                    } else {
                        measure_proven_borrowed_canonical_data_footprint(
                            &mut meter,
                            body,
                            value.data(),
                            retain,
                        )
                    };
                    (result, meter.estimate(), budget.used())
                })
                .unwrap()
        })
    };
    for retain in [false, true] {
        for compute in [111, 431, 432, 1_000] {
            for nodes in [0, 1, 3, 5, 6, 7] {
                for account in [137, 425, 426, 1_000] {
                    for call in [95, 414, 415, 1_000] {
                        assert_eq!(
                            execute(false, retain, compute, nodes, account, call),
                            execute(true, retain, compute, nodes, account, call),
                            "retain={retain}, compute={compute}, nodes={nodes}, account={account}, call={call}"
                        );
                    }
                }
            }
        }
    }
}

#[cfg(feature = "native-artifact")]
#[test]
fn whole_metadata_preview_is_pure_and_repeated_scans_share_the_cumulative_account() {
    let (value, schemas) = string_matrix(&["one", "two", "three"], true);
    let body = schemas.get(value.schema()).unwrap().body();
    let execute = |legacy| {
        let memory = mech_core::ManagedMemoryBudget::new(4_096);
        let budget = NativeLibraryWorkBudget::new(750, 0, &memory).unwrap();
        with_resident_turn_plan(
            detached_turn_plan(ResourceDemand::default()).unwrap(),
            || {
                budget
                    .with_scope(|| {
                        let mut meter = ResidentBudgetMeter::default();
                        assert!(
                            !meter
                                .try_charge_borrowed_metadata_scan(u64::MAX, true)
                                .unwrap()
                        );
                        assert!(!meter.try_charge_borrowed_metadata_scan(12, true).unwrap());
                        assert_eq!(budget.used(), (0, 0));
                        assert_eq!(meter, ResidentBudgetMeter::default());
                        assert_eq!(
                            CONTROL_WORK_SCOPE.with(|active| active
                                .borrow()
                                .as_ref()
                                .unwrap()
                                .observed
                                .compute),
                            0
                        );
                        let mut results = Vec::new();
                        for _ in 0..3 {
                            results.push(if legacy {
                                incremental(&mut meter, body, value.data(), true)
                            } else {
                                measure_proven_borrowed_canonical_data_footprint(
                                    &mut meter,
                                    body,
                                    value.data(),
                                    true,
                                )
                            });
                        }
                        (results, meter.estimate(), budget.used())
                    })
                    .unwrap()
            },
        )
    };
    let old = execute(true);
    assert!(old.0[0].is_ok() && old.0[1].is_ok() && old.0[2].is_err());
    assert_eq!(execute(false), old);
}

#[test]
fn metadata_scan_preview_cannot_create_turn_authority_or_overflow_a_meter() {
    let mut detached = ResidentBudgetMeter::default();
    assert!(!detached.try_charge_borrowed_metadata_scan(2, true).unwrap());
    assert_eq!(detached, ResidentBudgetMeter::default());
    for (compute, nodes) in [(u64::MAX - 1, 0), (0, u64::MAX)] {
        let mut meter = ResidentBudgetMeter::for_closed_analysis(1_000, 0);
        meter.accumulated.demand.work.compute = compute;
        meter.accumulated.demand.retained_nodes = nodes;
        let before = meter;
        assert!(!meter.try_charge_borrowed_metadata_scan(2, true).unwrap());
        assert_eq!(meter, before);
    }
}

#[cfg(feature = "native-artifact")]
#[test]
fn native_projection_metadata_scan_matches_incremental_accounting_and_denials() {
    let (value, schemas) = string_matrix(&["", "é\0", "a\r\nb", "🧑‍🚀"], true);
    let body = schemas.get(value.schema()).unwrap().body();
    let execute = |legacy, retain, compute, nodes, account, call| {
        let memory = mech_core::ManagedMemoryBudget::new(4096);
        let work = NativeLibraryWorkBudget::new(account, 0, &memory).unwrap();
        work.charge(11, 0).unwrap();
        work.with_call_limits(call, 0, || {
            let mut policy = mech_core::TargetMemoryProfile::current_resident_cpu()
                .unwrap()
                .limits;
            policy.max_compute_work = Some(compute);
            policy.max_comparison_work = Some(0);
            policy.max_retained_nodes = Some(nodes);
            let mut meter = ResidentBudgetMeter::for_native_projection(&work, policy).unwrap();
            meter.charge_compute_work(17).unwrap();
            let result = if legacy {
                incremental(&mut meter, body, value.data(), retain)
            } else {
                measure_proven_borrowed_canonical_data_footprint(
                    &mut meter,
                    body,
                    value.data(),
                    retain,
                )
            };
            (result, meter.estimate(), work.used())
        })
        .unwrap()
    };
    for retain in [false, true] {
        for compute in [31, 384, 400, 401, 1000] {
            for nodes in [0, 1, 3, 5, 6, 7] {
                for account in [137, 411, 412, 1000] {
                    for call in [95, 400, 401, 1000] {
                        assert_eq!(
                            execute(false, retain, compute, nodes, account, call),
                            execute(true, retain, compute, nodes, account, call),
                            "retain={retain}, compute={compute}, nodes={nodes}, account={account}, call={call}"
                        );
                    }
                }
            }
        }
    }
}
