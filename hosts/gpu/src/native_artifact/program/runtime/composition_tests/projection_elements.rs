//! A selected field is one handle but may contain many logical elements.
use super::*;
use mech_engine::native_artifact::{
    NativeCollectionPrimitive as Primitive, NativeIterationAction as Action,
    NativeLibraryWorkBudget, NativePreparedLibraryCallCache, lower_native_program_artifact,
    prepare_native_program_library_call,
};

fn admitted(value: Value, memory: &ManagedMemoryBudget) -> Value {
    let mut reservation = memory
        .reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap())
        .unwrap();
    value.into_memory_budget(&mut reservation).unwrap()
}

#[test]
fn private_projection_counts_current_matrix_elements_against_ordinary_call() {
    for (kind, dimensions, count) in [
        ("[u8]", vec![1, 3], 3u64),
        ("[u8]:2,3", vec![2, 3], 6),
        ("[u8]", vec![1, 0], 0),
        ("u64", vec![], 1),
    ] {
        let source = format!(
            "pack(xs<{kind}>) => (out<{{buffer<{kind}>}}>) :=\n  out := {{buffer: xs}}.\npacket := pack(values<{kind}>)\npacket.buffer\n"
        );
        for artifact in artifacts(&source) {
            let catalog = mech_stdlib::source_catalog();
            let plan = lower_native_program_artifact(&artifact, &catalog).unwrap();
            let (template, selector) = plan
                .blocks()
                .iter()
                .find_map(|block| {
                    let Action::LibraryCall {
                        template, operands, ..
                    } = block.action()
                    else {
                        return None;
                    };
                    (template.primitive() == Primitive::RecordFieldAccess)
                        .then(|| (template, operands[1]))
                })
                .unwrap();
            let call = prepare_native_program_library_call(&artifact, template, &catalog).unwrap();
            let mech_core::ResolvedOperationContract::Declared(contract) = template.contract()
            else {
                panic!()
            };
            let memory = NativeProgramLimitMode::NoLimits.memory_budget();
            let shape =
                mech_core::shape_for_resolved_extents(template.output(), &dimensions).unwrap();
            let data = if dimensions.is_empty() {
                Data::U64(17)
            } else {
                Data::Matrix(vec![Data::U8(97); count as usize].into())
            };
            let child = admitted(
                ValueDraft {
                    schema: contract.outputs[0].schema,
                    shape_values: shape.parameter_values().into(),
                    data: data.clone(),
                }
                .finalize(&SnapshotValidationContext::new(artifact.schemas()))
                .unwrap(),
                &memory,
            );
            let record = admitted(
                ValueDraft {
                    schema: contract.inputs[0].schema,
                    shape_values: child.shape().parameter_values().into(),
                    data: Data::Record(
                        vec![mech_core::snapshot::NamedValueDraft {
                            name: "buffer".into(),
                            value: data,
                        }]
                        .into(),
                    ),
                }
                .finalize(&SnapshotValidationContext::new(artifact.schemas()))
                .unwrap(),
                &memory,
            );
            let selector = admitted(
                plan.constants()
                    .iter()
                    .find(|v| v.value() == selector)
                    .unwrap()
                    .constant()
                    .clone(),
                &memory,
            );
            let aliases = [child.clone(), record.clone()];
            let before = contents(&aliases);
            let work = NativeLibraryWorkBudget::new(1 << 50, 1 << 50, &memory).unwrap();
            let args = [Some(record.clone()), Some(selector.clone())];
            let mut policy = NativeProgramLimitMode::NoLimits.execution().library;
            policy.max_output_elements = Some(count.max(1));
            let mut output = None;
            call.invoke_current_cached(
                &mut NativePreparedLibraryCallCache::default(),
                &args,
                &mut output,
                policy,
                &work,
            )
            .unwrap();
            let required = count.max(1);
            for limit in [1, 2, 3, 5, 6, 0] {
                let prior = output.clone().unwrap();
                let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                policy.max_output_elements = Some(limit);
                let ordinary = call.invoke_current_cached(
                    &mut NativePreparedLibraryCallCache::default(),
                    &args,
                    &mut output,
                    policy,
                    &work,
                );
                let private = call.admit_buffer_record_field(
                    &selector,
                    0,
                    &child,
                    policy,
                    &work,
                    Default::default(),
                );
                assert_eq!(
                    private.is_ok(),
                    ordinary.is_ok(),
                    "{kind} count={count} cap={limit}: private={private:?} ordinary={ordinary:?}"
                );
                assert_eq!(
                    private.is_ok(),
                    limit >= required,
                    "{kind} count={count} cap={limit}"
                );
                if limit < required {
                    for error in [
                        format!("{:?}", private.unwrap_err()),
                        format!("{:?}", ordinary.unwrap_err()),
                    ] {
                        assert!(error.contains("OutputElements"), "{error}");
                        assert!(error.contains(&format!("required: {required}")), "{error}");
                    }
                    assert!(output.as_ref().unwrap().shares_frozen_storage(&prior));
                }
                // Selector failure keeps precedence over insufficient output admission.
                let bad = call
                    .admit_buffer_record_field(
                        &selector,
                        1,
                        &child,
                        policy,
                        &work,
                        Default::default(),
                    )
                    .unwrap_err();
                assert!(format!("{bad:?}").contains("InvalidInput"), "{bad:?}");
                assert_eq!(contents(&aliases), before);
                if limit < required {
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        retained
                    );
                }
                policy.max_output_elements = Some(count.max(1));
                call.admit_buffer_record_field(
                    &selector,
                    0,
                    &child,
                    policy,
                    &work,
                    Default::default(),
                )
                .unwrap();
            }
            drop((aliases, args, child, record, selector, output, work));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

const PARENT: &str =
    include_str!("../../../../../tests/fixtures/native/projection-elements-parent.mec");

#[test]
fn parent_projection_declines_at_positive_insufficient_limits_and_recovers() {
    observe::without_numeric_phase(|| {
        for artifact in artifacts(PARENT) {
            let mode = NativeProgramLimitMode::NoLimits;
            let jit = JitProgramArtifact::compile_with_limits(
                &artifact,
                &mech_stdlib::source_catalog(),
                mode.compilation(),
            )
            .unwrap();
            let fields=jit.descriptor().plan().blocks().iter().enumerate().filter_map(|(site,block)|matches!(block.action(),Action::LibraryCall{template,..} if template.primitive()==Primitive::RecordFieldAccess).then_some(site as u64)).collect::<std::collections::BTreeSet<_>>();
            assert!(!fields.is_empty());
            assert!(
                !jit.descriptor().metadata().compositions.parents.is_empty(),
                "fixture needs parent segments"
            );
            assert!(
                fields
                    .iter()
                    .any(|site| selected_native_sites(&jit).contains(site)),
                "projection must qualify inside a parent segment"
            );
            for restored in [false, true] {
                let memory = mode.memory_budget();
                let args = inputs(
                    &artifact,
                    &[
                        ("bytes", Data::Matrix(vec![Data::U8(97); 3].into())),
                        ("steps", Data::U64(0)),
                    ],
                    &memory,
                );
                let aliases = args.clone();
                let before = contents(&args);
                let mut output = vec![None; artifact.outputs().len()];
                for cap in [3, 1, 3, 2, 3] {
                    let mut policy = mode.execution();
                    policy.library.max_output_elements = Some(cap);
                    let prior = output.clone();
                    let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let (result, counts) = observe::without_numeric_phase(|| {
                        if restored {
                            observe::run_restored(&jit, &args, &mut output, policy, &memory)
                        } else {
                            observe::run(&jit, &args, &mut output, policy, &memory)
                        }
                    });
                    if cap < 3 {
                        let error = format!("{:?}", result.unwrap_err());
                        assert!(
                            error.contains("OutputElements")
                                && error.contains(if cap == 1 {
                                    "required: 2"
                                } else {
                                    "required: 3"
                                }),
                            "{error}"
                        );
                        for (a, b) in output.iter().zip(&prior) {
                            assert!(
                                a.as_ref()
                                    .unwrap()
                                    .shares_frozen_storage(b.as_ref().unwrap())
                            );
                        }
                        assert_eq!(
                            (memory.used_bytes(), memory.retained_snapshot_nodes()),
                            retained
                        );
                        assert!(
                            cap == 1
                                || counts.helpers.keys().any(|(site, _)| fields.contains(site)),
                            "insufficient parent admission must reach the original field call: {counts:?}"
                        );
                    } else {
                        result.unwrap();
                        assert_eq!(
                            output
                                .last()
                                .unwrap()
                                .as_ref()
                                .unwrap()
                                .canonical_data_draft()
                                .unwrap(),
                            Data::U64(3)
                        );
                        assert!(counts.selected > 0, "{counts:?}");
                    }
                    assert_eq!(contents(&aliases), before);
                }
                drop((output, args, aliases));
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
        }
    });
}
