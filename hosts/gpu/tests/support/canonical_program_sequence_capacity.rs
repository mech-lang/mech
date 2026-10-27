//! Source buffer lifecycle qualification, using the real path/output capacities.
use super::{allocation_probe, artifacts, equal, inputs, limits, reference_result};
use mech_core::{ManagedMemoryBudget, ValueData, ValueDataDraft as Data};
use mech_gpu::native_artifact::JitProgramArtifact;

#[test]
fn reserve_append_clear_agrees_in_resident_and_native_original_and_decoded() {
    for width in [8, 16, 32, 64] {
        let source = format!(
            "+> sequence\nsequence/append(sequence/clear(sequence/append(sequence/reserve(sequence/with-capacity(0u{width}, capacity<u64>), 3u64), first<u{width}>)), last<u{width}>)\n"
        );
        let (forms, last) = artifacts(&source);
        for artifact in forms {
            let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
            let data = |word| match width {
                8 => Data::U8(word as u8),
                16 => Data::U16(word as u16),
                32 => Data::U32(word as u32),
                _ => Data::U64(word),
            };
            let args = inputs(
                &artifact,
                &[
                    ("capacity", Data::U64(32)),
                    ("first", data(17)),
                    ("last", data(129)),
                ],
                &memory,
            );
            let expected = Data::Matrix(vec![data(129)].into());
            let reference = reference_result(&artifact, &args).unwrap();
            assert_eq!(reference[last].canonical_data_draft().unwrap(), expected);
            drop(reference);
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let mut output = vec![None; artifact.outputs().len()];
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            assert_eq!(
                output[last]
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                expected
            );
            let ValueData::Matrix(matrix) = output[last].as_ref().unwrap().data() else {
                panic!()
            };
            assert_eq!(matrix.unsigned_buffer_capacity(), Some(32));
            drop((args, output));
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}

#[test]
fn real_reserved_path_and_output_buffers_allocate_the_payload_once_per_computation() {
    for (width, real_capacity) in [(64u64, 305257u64), (8, 6633832)] {
        let source = format!(
            "+> sequence\nsequence/append(sequence/clear(sequence/append(sequence/reserve(sequence/with-capacity(0u{width}, capacity<u64>), 3u64), 17u{width})), 129u{width})\n"
        );
        let (forms, last) = artifacts(&source);
        for artifact in forms {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let mut costs = Vec::new();
            // The descriptor's lazy common metadata is initialized once. Both
            // measured capacities then use the same warmed descriptor state.
            for (round, capacity) in [16, 16, real_capacity].into_iter().enumerate() {
                let memory = ManagedMemoryBudget::new(128 * 1024 * 1024);
                let args = inputs(&artifact, &[("capacity", Data::U64(capacity))], &memory);
                let mut output = vec![None; artifact.outputs().len()];
                let mut policy = limits();
                policy.execution.max_work = 1_000_000_000_000;
                policy.library.max_compute_work = Some(1_000_000_000_000);
                let (result, requests, bytes) = allocation_probe::measured_with_bytes(|| {
                    jit.evaluate_into(&args, &mut output, policy, &memory)
                });
                result.unwrap();
                let ValueData::Matrix(matrix) = output[last].as_ref().unwrap().data() else {
                    panic!()
                };
                assert_eq!(matrix.unsigned_buffer_capacity(), Some(capacity as usize));
                if round > 0 {
                    costs.push((requests, bytes));
                }
                drop((args, output));
                assert_eq!(memory.used_bytes(), 0);
            }
            println!(
                "width={width} capacity={real_capacity} small={:?} real={:?}",
                costs[0], costs[1]
            );
            assert_eq!(costs[0].0, costs[1].0);
            assert_eq!(
                costs[1].1 - costs[0].1,
                ((real_capacity - 16) * (width / 8)) as usize,
                "only the initial reservation may allocate a payload backing"
            );
        }
    }
}

#[test]
fn shared_buffer_append_preserves_aliases_and_failed_publication_then_retries() {
    let source = "+> sequence\n<row> := <[u64]:1,2>\nchecked(xs<row>, n<u64>) => (out<[u64]>) :=\n  changed := sequence/append(xs, n)\n  positive! := n > 0u64\n  out := sequence/append(changed, 12u64 / n).\nchecked(values<row>, input<u64>)\n";
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(128 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for n in [2, 0, 3] {
            let original = Data::Matrix(vec![Data::U64(7), Data::U64(8)].into());
            let args = inputs(
                &artifact,
                &[("values", original.clone()), ("input", Data::U64(n))],
                &memory,
            );
            let aliases = args.clone();
            let before = memory.used_bytes();
            if n == 0 {
                let prior = output
                    .iter()
                    .map(|v| v.clone().unwrap())
                    .collect::<Vec<_>>();
                assert!(
                    reference_result(&artifact, &args)
                        .unwrap_err()
                        .contains("LexicalIntegrity")
                );
                let error = jit
                    .evaluate_into(&args, &mut output, limits(), &memory)
                    .unwrap_err();
                assert!(
                    matches!(
                        error,
                        mech_gpu::native_artifact::NativeProgramExecutionError::Control(
                            mech_gpu::native_artifact::NativeIterationExecutionError::Integrity { .. }
                        )
                    ),
                    "{error:?}"
                );
                equal(&artifact, &output, &prior);
                assert_eq!(memory.used_bytes(), before);
            } else {
                let expected = reference_result(&artifact, &args).unwrap();
                jit.evaluate_into(&args, &mut output, limits(), &memory)
                    .unwrap();
                equal(&artifact, &output, &expected);
            }
            for (input, alias) in args.iter().zip(&aliases) {
                assert!(input.shares_frozen_storage(alias));
            }
            let index = artifact
                .inputs()
                .iter()
                .position(|v| mech_engine::decode_source_input_name(&v.name).unwrap() == "values")
                .unwrap();
            assert_eq!(args[index].canonical_data_draft().unwrap(), original);
            assert_eq!(aliases[index].canonical_data_draft().unwrap(), original);
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn unselected_reservation_does_not_allocate_and_resource_denial_recovers() {
    let source = "+> sequence\nselect(ok<bool>, capacity<u64>) => (out<[u8]>) :=\n  out := sequence/append((ok? | true => sequence/with-capacity(0u8, capacity) | false => sequence/with-capacity(0u8, 18446744073709551615u64)), 7u8).\nselect(ok<bool>, capacity<u64>)\n";
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(16 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for (ok, capacity) in [(true, 32), (false, 32), (true, 64)] {
            let args = inputs(
                &artifact,
                &[("ok", Data::Bool(ok)), ("capacity", Data::U64(capacity))],
                &memory,
            );
            let before = memory.used_bytes();
            if ok {
                jit.evaluate_into(&args, &mut output, limits(), &memory)
                    .unwrap();
                equal(
                    &artifact,
                    &output,
                    &reference_result(&artifact, &args).unwrap(),
                );
            } else {
                let prior = output
                    .iter()
                    .map(|v| v.clone().unwrap())
                    .collect::<Vec<_>>();
                assert!(reference_result(&artifact, &args).is_err());
                assert!(
                    jit.evaluate_into(&args, &mut output, limits(), &memory)
                        .is_err()
                );
                equal(&artifact, &output, &prior);
                assert_eq!(memory.used_bytes(), before);
            }
        }
        let args = inputs(
            &artifact,
            &[("ok", Data::Bool(true)), ("capacity", Data::U64(32))],
            &memory,
        );
        let prior = output
            .iter()
            .map(|v| v.clone().unwrap())
            .collect::<Vec<_>>();
        let before = memory.used_bytes();
        let ballast = memory
            .reserve_capacity(memory.limit_bytes() - before)
            .unwrap();
        assert!(
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .is_err()
        );
        equal(&artifact, &output, &prior);
        assert_eq!(memory.used_bytes(), memory.limit_bytes());
        drop(ballast);
        assert_eq!(memory.used_bytes(), before);
        jit.evaluate_into(&args, &mut output, limits(), &memory)
            .unwrap();
        drop((args, output, prior));
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn scalar_recursion_keeps_fresh_buffers_and_immutable_outer_capture_reads() {
    let source = "+> sequence\n<seed-row> := <[u64]:1,1>\nvisit(n<u64>, seed<seed-row>, total<u64>) => (out<u64>) :=\n  fresh := sequence/append(sequence/with-capacity(0u64, 32u64), n)\n  copied := sequence/append(seed, fresh[1u64])\n  first := copied[1u64]\n  out := ((n > 0u64)? | true => visit(n - 1u64, seed, total + first + fresh[1u64]) | false => total + first + fresh[1u64]).\nvisit(input<u64>, values<seed-row>, 0u64)\n";
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(128 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for n in [0, 3, 1, 4, 0] {
            let args = inputs(
                &artifact,
                &[
                    ("input", Data::U64(n)),
                    ("values", Data::Matrix(vec![Data::U64(7)].into())),
                ],
                &memory,
            );
            let expected = reference_result(&artifact, &args).unwrap();
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            equal(&artifact, &output, &expected);
            assert_eq!(
                output
                    .last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                Data::U64(7 * (n + 1) + n * (n + 1) / 2)
            );
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn initial_append_growth_and_exact_reserve_keep_all_initialized_words() {
    for width in [8, 16, 32, 64] {
        let source = format!(
            "+> sequence\nsequence/reserve(sequence/append(sequence/append(sequence/with-capacity(0u{width}, 0u64), 17u{width}), 129u{width}), 100u64)\n"
        );
        for artifact in artifacts(&source).0 {
            let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
            let expected = reference_result(&artifact, &[]).unwrap();
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let mut output = vec![None; artifact.outputs().len()];
            jit.evaluate_into(&[], &mut output, limits(), &memory)
                .unwrap();
            equal(&artifact, &output, &expected);
            let ValueData::Matrix(matrix) = output.last().unwrap().as_ref().unwrap().data() else {
                panic!()
            };
            assert_eq!(matrix.unsigned_buffer_capacity(), Some(102));
            drop(output);
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}

#[test]
fn generic_buffer_parameters_check_current_rows_without_partial_publication() {
    for width in [8, 16, 32, 64] {
        let source = format!(
            "+> sequence\nadd(xs<[u{width}]>, next<u{width}>) => (out<[u{width}]>) :=\n  out := sequence/append(xs, next).\nadd(values<[u{width}]>, next<u{width}>)\n"
        );
        for artifact in artifacts(&source).0 {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
            let mut output = vec![None; artifact.outputs().len()];
            let word = |n| match width {
                8 => Data::U8(n as u8),
                16 => Data::U16(n as u16),
                32 => Data::U32(n as u32),
                _ => Data::U64(n),
            };
            for (rows, columns) in [(1, 0), (2, 2), (1, 3)] {
                let words = Data::Matrix(vec![word(17); (rows * columns) as usize].into());
                let mut args = inputs(
                    &artifact,
                    &[("values", words.clone()), ("next", word(129))],
                    &memory,
                );
                let index = artifact
                    .inputs()
                    .iter()
                    .position(|v| {
                        mech_engine::decode_source_input_name(&v.name).unwrap() == "values"
                    })
                    .unwrap();
                let input = &artifact.inputs()[index];
                let shape = mech_core::shape_for_resolved_extents(
                    artifact.schemas().get(input.schema).unwrap(),
                    &[rows, columns],
                )
                .unwrap();
                let value = mech_core::ValueDraft {
                    schema: input.schema,
                    shape_values: shape.parameter_values().to_vec().into(),
                    data: words.clone(),
                }
                .finalize(&mech_core::snapshot::SnapshotValidationContext::new(
                    artifact.schemas(),
                ))
                .unwrap();
                let mut charge = memory
                    .reserve_capacity(value.memory_budget_admission_bytes(&memory).unwrap())
                    .unwrap();
                args[index] = value.into_memory_budget(&mut charge).unwrap();
                drop(charge);
                let aliases = args.clone();
                let before = memory.used_bytes();
                if rows == 2 {
                    let prior = output
                        .iter()
                        .map(|v| v.clone().unwrap())
                        .collect::<Vec<_>>();
                    assert!(reference_result(&artifact, &args).is_err());
                    assert!(
                        jit.evaluate_into(&args, &mut output, limits(), &memory)
                            .is_err()
                    );
                    equal(&artifact, &output, &prior);
                    assert_eq!(memory.used_bytes(), before);
                } else {
                    jit.evaluate_into(&args, &mut output, limits(), &memory)
                        .unwrap();
                    equal(
                        &artifact,
                        &output,
                        &reference_result(&artifact, &args).unwrap(),
                    );
                }
                assert_eq!(args[index].canonical_data_draft().unwrap(), words);
                assert!(args[index].shares_frozen_storage(&aliases[index]));
            }
            drop(output);
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}

#[test]
fn consuming_growth_admits_full_old_backing_work_and_copy_quota() {
    let source = "+> sequence\nsequence/reserve(sequence/clear(sequence/with-capacity(0u8, 1000000u64)), target<u64>)\n";
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let growth_origin = jit.descriptor().plan().blocks().iter().find_map(|block| {
            matches!(block.action(), mech_engine::native_artifact::NativeIterationAction::LibraryCall { template, .. }
                if template.primitive() == mech_engine::native_artifact::NativeCollectionPrimitive::SequenceReserve).then_some(block.origin())
        }).unwrap();
        let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        let mut allowed = limits();
        allowed.library.max_cloned_bytes = Some(1_000_000);
        let control_args = inputs(&artifact, &[("target", Data::U64(1_000_000))], &memory);
        let growth_args = inputs(&artifact, &[("target", Data::U64(1_000_001))], &memory);
        jit.evaluate_into(&control_args, &mut output, allowed, &memory)
            .unwrap();
        let template = jit
            .descriptor()
            .plan()
            .blocks()
            .iter()
            .find_map(|block| match block.action() {
                mech_engine::native_artifact::NativeIterationAction::LibraryCall {
                    template,
                    ..
                } if template.primitive()
                    == mech_engine::native_artifact::NativeCollectionPrimitive::SequenceReserve =>
                {
                    Some(template)
                }
                _ => None,
            })
            .unwrap();
        let call = mech_engine::native_artifact::prepare_native_program_library_call(
            &artifact,
            template,
            &mech_stdlib::source_catalog(),
        )
        .unwrap();
        let work = mech_engine::native_artifact::NativeLibraryWorkBudget::new(
            allowed.library.max_compute_work.unwrap(),
            allowed.library.max_comparison_work.unwrap(),
            &memory,
        )
        .unwrap();
        let source = output.last_mut().unwrap().take().unwrap();
        let source = call
            .unsigned_buffer(
                Some(source),
                1_000_000,
                true,
                allowed.library,
                &work,
                mech_core::ResourceDemand::default(),
            )
            .unwrap();
        let mut compute_policy = allowed;
        // Measure only this selected call's no-growth work. The full Program's
        // report also includes native transport and other library calls.
        compute_policy.library.max_compute_work = Some(work.used().0);
        *output.last_mut().unwrap() = Some(source);
        drop(work);
        let mut control_output = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&control_args, &mut control_output, compute_policy, &memory)
            .unwrap();
        drop(control_output);
        jit.evaluate_into(&growth_args, &mut output, allowed, &memory)
            .unwrap();
        let prior = output
            .iter()
            .map(|value| value.clone().unwrap())
            .collect::<Vec<_>>();
        for (dimension, policy) in [
            ("ClonedBytes", {
                let mut policy = allowed;
                policy.library.max_cloned_bytes = Some(999_999);
                policy
            }),
            ("ComputeWork", compute_policy),
        ] {
            let before = memory.used_bytes();
            let error = jit
                .evaluate_into(&growth_args, &mut output, policy, &memory)
                .unwrap_err();
            assert!(
                matches!(&error, mech_gpu::native_artifact::NativeProgramExecutionError::Library { origin, .. } if *origin == growth_origin),
                "{error:?}"
            );
            assert!(format!("{error:?}").contains(dimension), "{error:?}");
            assert!(
                output
                    .iter()
                    .zip(&prior)
                    .all(|(current, old)| current.as_ref().unwrap().shares_frozen_storage(old))
            );
            assert_eq!(memory.used_bytes(), before);
        }
        jit.evaluate_into(&growth_args, &mut output, allowed, &memory)
            .unwrap();
        equal(&artifact, &output, &prior);
        let ValueData::Matrix(matrix) = output.last().unwrap().as_ref().unwrap().data() else {
            panic!()
        };
        assert_eq!(matrix.unsigned_buffer_capacity(), Some(1_000_001));
        drop((control_args, growth_args, output, prior));
        assert_eq!(memory.used_bytes(), 0);
    }
}
