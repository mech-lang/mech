//! Real capability delivery, original error sites, limits and protocol denials.
use super::*;
use crate::native_artifact::program_runtime::selected_join::{
    JoinMapControl as Control, JoinMapFault as Fault, JoinMapLimit as Limit,
    join_map_demand as demand, join_map_fault_fired as fired, join_map_started as started,
    with_join_map_control as with_control,
};

fn program() -> String {
    let scan =
        super::super::capture_tests::mapped_scan("", false).replace("input<string>", "@in/text");
    format!(
        "+> @in := cli/stdin\n+> @out := cli/stdout\n{}@out/text <- normalized\n",
        scan.strip_suffix("normalized\n").unwrap()
    )
}
fn cleanup(
    runtime: &mut MechRuntime,
    trace: &Arc<Mutex<Trace>>,
    memory: &ManagedMemoryBudget,
    length: usize,
) {
    let trace = trace.lock().unwrap();
    assert_eq!((trace.captures, trace.snapshots, trace.writes), (1, 0, 0));
    assert_eq!(trace.position, length);
    assert!(!trace.body_failed_before_eof);
    assert!(trace.output.is_empty());
    assert_eq!(memory.used_bytes(), 0);
    assert_eq!(memory.retained_nodes(), 0);
    drop(trace);
    fixture::no_resident(runtime);
    runtime.shutdown().unwrap();
}

#[test]
fn fused_join_map_missing_key_keeps_the_original_lookup_origin_after_complete_input() {
    let input = b"open\nazb\nclose\nignored\n";
    let (artifacts, catalog) = fixture::artifacts(&program());
    for artifact in artifacts {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let region = &jit.descriptor().string_map_regions[0];
        let origin = jit.descriptor().plan.blocks()[region.selection_site() as usize].origin();
        let mut expected = None;
        for disable in [true, false] {
            let memory = ManagedMemoryBudget::new(TEST_MEMORY_LIMIT);
            let (mut runtime, trace) = runtime(input, true, None);
            let result = with_control(
                Control {
                    disable,
                    ..Default::default()
                },
                || {
                    let result = execute(
                        &mut runtime,
                        &artifact,
                        &jit,
                        fixture::limits(),
                        input.len() as u64,
                        &trace,
                        &memory,
                    );
                    assert_eq!(started(), !disable);
                    result
                },
            );
            let CanonicalNativeResourceRunError::BeforeCommit {
                phase: "native execution",
                error,
                ..
            } = result.unwrap_err()
            else {
                panic!("lookup must fail before publication");
            };
            let actual = error.kind_as::<NativeProgramExecutionError>().unwrap();
            assert!(
                matches!(actual, NativeProgramExecutionError::Library { origin: found,
                error: mech_engine::native_artifact::NativeCollectionInvocationError::Kernel(mech_core::ResidentKernelError::InvalidInput) } if *found == origin),
                "{actual:?}"
            );
            let actual = format!("{actual:?}");
            if let Some(expected) = &expected {
                assert_eq!(&actual, expected);
            } else {
                expected = Some(actual);
            }
            cleanup(&mut runtime, &trace, &memory, input.len());
        }
    }
}

#[test]
fn fused_join_map_late_io_and_utf8_failures_precede_any_composed_byte_work() {
    let (artifacts, catalog) = fixture::artifacts(&program());
    for artifact in artifacts {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        for (input, fail_at) in [
            (b"open\nazb\nclose\nlater\xff".as_slice(), None),
            (b"open\nazb\nclose\nlater\n".as_slice(), Some(21)),
        ] {
            let memory = ManagedMemoryBudget::new(TEST_MEMORY_LIMIT);
            let (mut runtime, trace) = runtime(input, true, fail_at);
            let result = with_control(
                Control {
                    fault: Some(Fault::ReplayPrepare),
                    ..Default::default()
                },
                || {
                    let result = execute(
                        &mut runtime,
                        &artifact,
                        &jit,
                        fixture::limits(),
                        input.len() as u64,
                        &trace,
                        &memory,
                    );
                    assert!(
                        !started() && !fired(),
                        "a join must not run before successful EOF"
                    );
                    result
                },
            );
            let CanonicalNativeResourceRunError::BeforeCommit {
                phase: "capture",
                error,
                ..
            } = result.unwrap_err()
            else {
                panic!("source failure must retain precedence");
            };
            assert!(
                error
                    .kind_as::<mech_terminal::CliResourceProviderError>()
                    .is_some(),
                "{error:?}"
            );
            cleanup(
                &mut runtime,
                &trace,
                &memory,
                fail_at.unwrap_or(input.len()),
            );
        }
    }
}

#[test]
fn fused_join_map_original_join_one_short_limits_precede_the_later_missing_key() {
    let input = b"open\nazb\nclose\n";
    let (artifacts, catalog) = fixture::artifacts(&program());
    for artifact in artifacts {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let join = &jit.descriptor().selected_joins.regions[0];
        let join_origin = jit.descriptor().plan.blocks()[join.consumer() as usize].origin();
        let memory = ManagedMemoryBudget::new(TEST_MEMORY_LIMIT);
        let (mut initial, trace) = runtime(input, true, None);
        let sizes = with_control(Control::default(), || {
            assert!(
                execute(
                    &mut initial,
                    &artifact,
                    &jit,
                    fixture::limits(),
                    input.len() as u64,
                    &trace,
                    &memory
                )
                .is_err()
            );
            demand()
        });
        cleanup(&mut initial, &trace, &memory, input.len());
        for (index, dimension) in [
            Limit::Output,
            Limit::Cloned,
            Limit::Temporary,
            Limit::Nodes,
            Limit::Compute,
        ]
        .into_iter()
        .enumerate()
        {
            for short in [true, false] {
                let mut reference = None;
                for disable in [true, false] {
                    let memory = ManagedMemoryBudget::new(TEST_MEMORY_LIMIT);
                    let (mut runtime, trace) = runtime(input, true, None);
                    let result = with_control(
                        Control {
                            disable,
                            limit: Some((dimension, sizes[index] - u64::from(short))),
                            ..Default::default()
                        },
                        || {
                            let result = execute(
                                &mut runtime,
                                &artifact,
                                &jit,
                                fixture::limits(),
                                input.len() as u64,
                                &trace,
                                &memory,
                            );
                            assert!(fired());
                            if short {
                                assert!(!started());
                            }
                            result
                        },
                    );
                    let CanonicalNativeResourceRunError::BeforeCommit {
                        phase: "native execution",
                        error,
                        ..
                    } = result.unwrap_err()
                    else {
                        panic!("failure must not publish");
                    };
                    let actual = error.kind_as::<NativeProgramExecutionError>().unwrap();
                    if short {
                        assert!(
                            matches!(actual, NativeProgramExecutionError::Library { origin, .. } if *origin == join_origin),
                            "{dimension:?} {actual:?}"
                        );
                    }
                    let actual = format!("{actual:?}");
                    if let Some(reference) = &reference {
                        assert_eq!(&actual, reference, "{dimension:?}, short={short}");
                    } else {
                        reference = Some(actual);
                    }
                    cleanup(&mut runtime, &trace, &memory, input.len());
                }
            }
        }
    }
}

#[test]
fn fused_join_map_global_byte_and_node_pressure_preserve_ordinary_failure_and_cleanup() {
    let input = b"open\nazb\nclose\n";
    let (artifacts, catalog) = fixture::artifacts(&program());
    for artifact in artifacts {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let memory = ManagedMemoryBudget::new(TEST_MEMORY_LIMIT);
        let (mut initial, trace) = runtime(input, true, None);
        let envelope = with_control(Control::default(), || {
            assert!(
                execute(
                    &mut initial,
                    &artifact,
                    &jit,
                    fixture::limits(),
                    input.len() as u64,
                    &trace,
                    &memory
                )
                .is_err()
            );
            demand()[5]
        });
        cleanup(&mut initial, &trace, &memory, input.len());
        for (bytes, nodes) in [
            (Some(envelope - 1), None),
            (Some(envelope), None),
            (None, Some(0)),
            (None, Some(1)),
            (None, Some(2)),
            (None, Some(3)),
            (None, Some(4)),
        ] {
            let mut reference = None;
            for disable in [true, false] {
                let memory = ManagedMemoryBudget::new(TEST_MEMORY_LIMIT);
                let (mut runtime, trace) = runtime(input, true, None);
                let result = with_control(
                    Control {
                        disable,
                        global_bytes: bytes,
                        global_nodes: nodes,
                        ..Default::default()
                    },
                    || {
                        let result = execute(
                            &mut runtime,
                            &artifact,
                            &jit,
                            fixture::limits(),
                            input.len() as u64,
                            &trace,
                            &memory,
                        );
                        assert!(fired());
                        result
                    },
                );
                let CanonicalNativeResourceRunError::BeforeCommit {
                    phase: "native execution",
                    error,
                    ..
                } = result.unwrap_err()
                else {
                    panic!("failure must remain atomic");
                };
                let actual = format!(
                    "{:?}",
                    error.kind_as::<NativeProgramExecutionError>().unwrap()
                );
                if let Some(reference) = &reference {
                    assert_eq!(&actual, reference, "bytes={bytes:?}, nodes={nodes:?}");
                } else {
                    reference = Some(actual);
                }
                cleanup(&mut runtime, &trace, &memory, input.len());
            }
        }
    }
}

#[test]
fn fused_join_map_replay_foreign_generation_and_missing_owner_denials_remain_atomic() {
    let input = b"open\nab\nclose\n";
    let (artifacts, catalog) = fixture::artifacts(&program());
    for artifact in artifacts {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        for fault in [
            Fault::ReplayPrepare,
            Fault::FinishWithoutLoan,
            Fault::WrongSite,
            Fault::ExcessCompletion,
            Fault::SkipCompletion,
            Fault::WrongMapGeneration,
            Fault::WrongCopySource,
            Fault::MissingOwner,
            Fault::PrivateWrongAccount,
            Fault::PrivateShortCapacity,
            Fault::PrivateWrongFactory,
        ] {
            let memory = ManagedMemoryBudget::new(TEST_MEMORY_LIMIT);
            let (mut runtime, trace) = runtime(input, true, None);
            let result = with_control(
                Control {
                    fault: Some(fault),
                    ..Default::default()
                },
                || {
                    let result = execute(
                        &mut runtime,
                        &artifact,
                        &jit,
                        fixture::limits(),
                        input.len() as u64,
                        &trace,
                        &memory,
                    );
                    assert!(
                        fired() && started(),
                        "{fault:?} must reach its actual native boundary"
                    );
                    result
                },
            );
            let CanonicalNativeResourceRunError::BeforeCommit {
                phase: "native execution",
                error,
                ..
            } = result.unwrap_err()
            else {
                panic!("protocol denial must not publish");
            };
            assert!(
                error.kind_as::<NativeProgramExecutionError>().is_some(),
                "{fault:?}: {error:?}"
            );
            if matches!(
                fault,
                Fault::PrivateWrongAccount
                    | Fault::PrivateShortCapacity
                    | Fault::PrivateWrongFactory
            ) {
                let actual = error.kind_as::<NativeProgramExecutionError>().unwrap();
                let NativeProgramExecutionError::Library { error, .. } = actual else {
                    panic!("{fault:?} must be rejected by the actual buffer factory: {actual:?}");
                };
                assert!(match fault {
                    Fault::PrivateWrongFactory => matches!(error, mech_engine::native_artifact::NativeCollectionInvocationError::InvalidInput),
                    _ => matches!(error, mech_engine::native_artifact::NativeCollectionInvocationError::UnadmittedValue),
                }, "{fault:?}: {error:?}");
            }
            cleanup(&mut runtime, &trace, &memory, input.len());
        }
        let memory = ManagedMemoryBudget::new(TEST_MEMORY_LIMIT);
        let (mut runtime, trace) = runtime(input, true, None);
        let report = execute(
            &mut runtime,
            &artifact,
            &jit,
            fixture::limits(),
            input.len() as u64,
            &trace,
            &memory,
        )
        .unwrap();
        assert_eq!(report.fused_string_map_bytes, 2);
        assert_eq!(trace.lock().unwrap().output, "AB");
        assert_eq!(memory.used_bytes(), 0);
        fixture::no_resident(&runtime);
        runtime.shutdown().unwrap();
    }
}
