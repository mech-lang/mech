//! Compact Range geometry uses the same selection, owner and finite accounts.
use super::*;
use crate::native_artifact::{NativeSelectedStringMatrix, NativeSelectedStringSpanSelector};
use std::rc::Rc;

fn programs(inclusive: bool) -> [crate::ProgramArtifact; 2] {
    variants_for(if inclusive {
        "lines := text/lines(input<string>)\nselected := lines[first<u64>..=last<u64>]\ntext/join(selected, separator<string>)\n"
    } else {
        "lines := text/lines(input<string>)\nselected := lines[first<u64>..last<u64>]\ntext/join(selected, separator<string>)\n"
    })
}

fn admitted(
    artifact: &crate::ProgramArtifact,
    memory: &ManagedMemoryBudget,
    schema: SchemaId,
    data: ValueDataDraft,
) -> Value {
    let value = ValueDraft {
        schema,
        shape_values: Box::new([]),
        data,
    }
    .finalize(&mech_core::snapshot::SnapshotValidationContext::new(
        artifact.schemas(),
    ))
    .unwrap();
    let mut reservation = memory
        .reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap())
        .unwrap();
    value.into_memory_budget(&mut reservation).unwrap()
}

fn compact(
    range: &NativePreparedLibraryCall,
    start: u64,
    end: u64,
    work: &NativeLibraryWorkBudget,
) -> Result<NativeSelectedStringSpanSelector, InvokeError> {
    let limits = limits();
    work.with_call_limits(
        limits.max_compute_work.unwrap(),
        limits.max_comparison_work.unwrap(),
        || {
            work.charge(range.selected_u64_string_span_selector_work(), 0)?;
            let _scratch = work
                .memory_budget()
                .reserve_capacity(range.selected_u64_string_span_selector_metadata_bytes())?;
            range.selected_u64_string_span_selector(start, end)
        },
    )?
}

fn materialized(
    artifact: &crate::ProgramArtifact,
    range: &NativePreparedLibraryCall,
    start: u64,
    end: u64,
    work: &NativeLibraryWorkBudget,
) -> Result<Value, InvokeError> {
    let mech_core::ResolvedOperationContract::Declared(contract) = range.template.contract() else {
        panic!("declared Range")
    };
    let memory = work.memory_budget();
    let inputs = [
        Some(admitted(
            artifact,
            &memory,
            contract.inputs[0].schema,
            ValueDataDraft::U64(start),
        )),
        Some(admitted(
            artifact,
            &memory,
            contract.inputs[1].schema,
            ValueDataDraft::U64(end),
        )),
    ];
    let mut output = None;
    range.invoke_current(&inputs, &mut output, limits(), work)?;
    Ok(output.expect("actual Range output"))
}

fn lines(
    producer: &NativePreparedLibraryCall,
    work: &NativeLibraryWorkBudget,
) -> Rc<NativeTextLineMatrix> {
    let owner = capture(producer, "discard\r\n猫\ne\u{301}\nlast\n", limits(), work);
    Rc::new(
        producer
            .text_line_matrix_from_capture(owner, limits(), work, ResourceDemand::default())
            .unwrap(),
    )
}

fn failure<T>(result: Result<T, InvokeError>) -> InvokeError {
    match result {
        Err(error) => error,
        Ok(_) => panic!("expected rejection"),
    }
}

fn append(
    join: &NativePreparedLibraryCall,
    view: NativeSelectedStringMatrix,
    separator: &Value,
    work: &NativeLibraryWorkBudget,
) -> String {
    let expected_bytes = view.payload_bytes() + view.count().saturating_sub(1) as u64;
    let mut output = String::new();
    output.try_reserve_exact(expected_bytes as usize).unwrap();
    let _reservation = work
        .memory_budget()
        .reserve_capacity(expected_bytes)
        .unwrap();
    work.charge((expected_bytes + view.count() as u64 + 1) * 128, 0)
        .unwrap();
    join.selected_matrix_join_append_consuming(view, separator, &mut output, &work.memory_budget())
        .unwrap();
    output
}

#[test]
fn compact_range_matches_materialized_range_for_unicode_and_both_owner_lifetimes() {
    for inclusive in [false, true] {
        for artifact in programs(inclusive) {
            let producer = prepared(&artifact, NativeCollectionPrimitive::TextLines);
            let range = prepared(
                &artifact,
                if inclusive {
                    NativeCollectionPrimitive::RangeInclusive
                } else {
                    NativeCollectionPrimitive::RangeExclusive
                },
            );
            let select = prepared(&artifact, NativeCollectionPrimitive::MatrixRangeAccess);
            let join = prepared(&artifact, NativeCollectionPrimitive::TextJoin);
            let mech_core::ResolvedOperationContract::Declared(contract) = join.template.contract()
            else {
                panic!("declared join")
            };
            for shared in [false, true] {
                let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
                let work = work(&memory);
                let separator = admitted(
                    &artifact,
                    &memory,
                    contract.inputs[1].schema,
                    ValueDataDraft::String("|".into()),
                );
                let baseline = memory.used_bytes();
                let matrix = lines(&producer, &work);
                let source_bytes = matrix.retained_bytes(&memory).unwrap();
                let end = if inclusive { 3 } else { 4 };
                let selector = materialized(&artifact, &range, 2, end, &work).unwrap();
                let ordinary = select
                    .selected_text_line_matrix(
                        &matrix,
                        &selector,
                        limits(),
                        &work,
                        ResourceDemand::default(),
                    )
                    .unwrap();
                let geometry = compact(&range, 2, end, &work).unwrap();
                let view = select
                    .selected_text_line_matrix_range(
                        &matrix,
                        &geometry,
                        limits(),
                        &work,
                        ResourceDemand::default(),
                    )
                    .unwrap();
                assert_eq!(view.count(), ordinary.count());
                assert_eq!(view.payload_bytes(), ordinary.payload_bytes());
                assert_eq!(view.retained_bytes(&memory).unwrap(), source_bytes);
                assert_eq!(
                    ordinary.retained_bytes(&memory).unwrap(),
                    source_bytes + selector.memory_budget_retained_bytes(&memory).unwrap()
                );
                assert_eq!(append(&join, ordinary, &separator, &work), "猫|e\u{301}");
                drop(selector);
                let shared_owner = shared.then(|| matrix.clone());
                drop(matrix);
                assert_eq!(append(&join, view, &separator, &work), "猫|e\u{301}");
                if let Some(owner) = shared_owner {
                    assert_eq!(owner.line(1), Some("猫"));
                    assert_eq!(owner.line(2), Some("e\u{301}"));
                    assert!(owner.captured_chunks().unwrap().retained_capacity_bytes() > 0);
                    drop(owner);
                }
                assert_eq!(memory.used_bytes(), baseline);
                drop((separator, work));
                assert_eq!(memory.used_bytes(), 0);
            }
        }
    }
}

#[test]
fn compact_range_keeps_range_and_selection_errors_and_recovers_on_the_same_account() {
    for inclusive in [false, true] {
        for artifact in programs(inclusive) {
            let producer = prepared(&artifact, NativeCollectionPrimitive::TextLines);
            let range = prepared(
                &artifact,
                if inclusive {
                    NativeCollectionPrimitive::RangeInclusive
                } else {
                    NativeCollectionPrimitive::RangeExclusive
                },
            );
            let select = prepared(&artifact, NativeCollectionPrimitive::MatrixRangeAccess);
            let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
            let work = work(&memory);
            let matrix = lines(&producer, &work);
            let baseline = memory.used_bytes();
            let invalid = if inclusive { (3, 2) } else { (2, 2) };
            assert_eq!(
                failure(compact(&range, invalid.0, invalid.1, &work)),
                failure(materialized(&artifact, &range, invalid.0, invalid.1, &work))
            );
            assert_eq!(memory.used_bytes(), baseline);
            for (start, end) in [(0, 2), (4, 6)] {
                let selector = materialized(&artifact, &range, start, end, &work).unwrap();
                let geometry = compact(&range, start, end, &work).unwrap();
                let ordinary = failure(select.selected_text_line_matrix(
                    &matrix,
                    &selector,
                    limits(),
                    &work,
                    ResourceDemand::default(),
                ));
                let used = work.used().0;
                let compact_error = failure(select.selected_text_line_matrix_range(
                    &matrix,
                    &geometry,
                    limits(),
                    &work,
                    ResourceDemand::default(),
                ));
                assert_eq!(compact_error, ordinary);
                assert!(work.used().0 > used);
                drop(selector);
                assert_eq!(memory.used_bytes(), baseline);
                assert_eq!(matrix.line(1), Some("猫"));
            }
            let geometry = compact(&range, 2, if inclusive { 3 } else { 4 }, &work).unwrap();
            let view = select
                .selected_text_line_matrix_range(
                    &matrix,
                    &geometry,
                    limits(),
                    &work,
                    ResourceDemand::default(),
                )
                .unwrap();
            assert_eq!(view.count(), 2);
            drop((view, matrix, work));
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}

#[test]
fn compact_range_selection_enforces_exact_work_finite_limits_and_real_memory_ownership() {
    for artifact in programs(false) {
        let producer = prepared(&artifact, NativeCollectionPrimitive::TextLines);
        let range = prepared(&artifact, NativeCollectionPrimitive::RangeExclusive);
        let select = prepared(&artifact, NativeCollectionPrimitive::MatrixRangeAccess);
        let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
        let work = work(&memory);
        let matrix = lines(&producer, &work);
        let geometry = compact(&range, 2, 4, &work).unwrap();
        let baseline = memory.used_bytes();
        let before = work.used().0;
        drop(
            select
                .selected_text_line_matrix_range(
                    &matrix,
                    &geometry,
                    limits(),
                    &work,
                    ResourceDemand::default(),
                )
                .unwrap(),
        );
        let exact = work.used().0 - before;
        assert!(exact > 0);
        for cap in [exact - 1, exact] {
            let mut limited = limits();
            limited.max_compute_work = Some(cap);
            let before = work.used().0;
            let result = select.selected_text_line_matrix_range(
                &matrix,
                &geometry,
                limited,
                &work,
                ResourceDemand::default(),
            );
            if cap == exact {
                assert_eq!(result.unwrap().count(), 2);
                assert_eq!(work.used().0 - before, exact);
            } else {
                assert!(matches!(
                    failure(result),
                    InvokeError::Kernel(ResidentKernelError::ResourceLimit {
                        dimension: Dimension::ComputeWork,
                        ..
                    })
                ));
                assert!(work.used().0 > before);
            }
            assert_eq!(memory.used_bytes(), baseline);
        }
        let mut unbounded = limits();
        unbounded.max_compute_work = None;
        assert!(matches!(
            failure(select.selected_text_line_matrix_range(
                &matrix,
                &geometry,
                unbounded,
                &work,
                ResourceDemand::default()
            )),
            InvokeError::Binding(_)
        ));
        let binding = select.binding_bytes
            + select.selection_shape_bounds.0
            + select.selection_range_shape_bounds.0;
        for available in [binding - 1, binding] {
            let pressure = memory
                .reserve_capacity(memory.limit_bytes() - memory.used_bytes() - available)
                .unwrap();
            let result = select.selected_text_line_matrix_range(
                &matrix,
                &geometry,
                limits(),
                &work,
                ResourceDemand::default(),
            );
            if available == binding {
                assert_eq!(result.unwrap().count(), 2);
            } else {
                assert!(matches!(failure(result), InvokeError::Memory(_)));
            }
            drop(pressure);
            assert_eq!(memory.used_bytes(), baseline);
        }
        let foreign = ManagedMemoryBudget::new(64 * 1024 * 1024);
        let foreign_work = super::work(&foreign);
        let foreign_baseline = foreign.used_bytes();
        assert_eq!(
            failure(select.selected_text_line_matrix_range(
                &matrix,
                &geometry,
                limits(),
                &foreign_work,
                ResourceDemand::default()
            )),
            InvokeError::UnadmittedValue
        );
        assert_eq!(foreign.used_bytes(), foreign_baseline);
        assert_eq!(matrix.line(1), Some("猫"));
        drop((matrix, work, foreign_work));
        assert_eq!(memory.used_bytes(), 0);
        assert_eq!(foreign.used_bytes(), 0);
    }
}
