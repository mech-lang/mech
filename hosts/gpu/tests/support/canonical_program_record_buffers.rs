//! Private record returns preserve reusable buffers and ordinary value semantics.
use super::{allocation_probe, artifacts, inputs, limits, reference_result};
use mech_core::{ManagedMemoryBudget, ValueData, ValueDataDraft as Data};
use mech_engine::native_artifact::lower_native_program_artifact;
use mech_gpu::native_artifact::JitProgramArtifact;

const PACK: &str = "+> sequence\npack(nodes<[u64]>) => (out<{nodes<[u64]>}>) :=\n  out := {nodes: nodes}.\nreturned := pack(sequence/with-capacity(0u64, capacity<u64>))\nsequence/append(returned.nodes, 17u64)\n";

#[test]
fn private_record_return_preserves_real_path_and_output_reservations() {
    for (width, real) in [(64, 305257), (8, 6633832)] {
        let source = if width == 64 {
            PACK.to_owned()
        } else {
            PACK.replace("[u64]", "[u8]")
                .replace("0u64, capacity", "0u8, capacity")
                .replace("17u64", "17u8")
        };
        for (form, artifact) in ["original", "decoded"]
            .into_iter()
            .zip(artifacts(&source).0)
        {
            let plan =
                lower_native_program_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
            assert!(plan.facts().buffer_frames().elements() > 0);
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let mut measured = Vec::new();
            for capacity in [16, real] {
                let memory = ManagedMemoryBudget::new(128 * 1024 * 1024);
                let args = inputs(&artifact, &[("capacity", Data::U64(capacity))], &memory);
                let mut output = vec![None; artifact.outputs().len()];
                let mut policy = limits();
                policy.library.max_cloned_bytes = Some(0);
                // Warm immutable metadata equally before comparing capacities.
                jit.evaluate_into(&args, &mut output, policy, &memory)
                    .unwrap();
                let start = std::time::Instant::now();
                let (result, requests, bytes) = allocation_probe::measured_with_bytes(|| {
                    jit.evaluate_into(&args, &mut output, policy, &memory)
                });
                result.unwrap();
                let value = output.last().unwrap().as_ref().unwrap();
                assert_eq!(
                    value.canonical_data_draft().unwrap(),
                    Data::Matrix(
                        vec![if width == 64 {
                            Data::U64(17)
                        } else {
                            Data::U8(17)
                        }]
                        .into()
                    )
                );
                let ValueData::Matrix(matrix) = value.data() else {
                    panic!()
                };
                assert_eq!(matrix.unsigned_buffer_capacity(), Some(capacity as usize));
                println!(
                    "{}",
                    serde_json::json!({"witness":"private-record-reserved-buffer", "artifact":form, "width":width,"capacity":capacity,"requests":requests,"requested_bytes":bytes,"elapsed_seconds":start.elapsed().as_secs_f64()})
                );
                measured.push(bytes);
                drop((args, output));
                assert_eq!(memory.used_bytes(), 0);
                assert_eq!(memory.retained_snapshot_nodes(), 0);
            }
            // Increasing capacity changes allocation by exactly one payload.
            assert_eq!(
                measured[1] - measured[0],
                (real - 16) as usize * (width / 8) as usize
            );
        }
    }
}

#[test]
fn recursive_record_returns_keep_capacity_through_clear_and_repeated_calls() {
    let source = "+> sequence\ngrow(words<[u64]>, left<u64>) => (out<{nodes<[u64]>}>) :=\n  out := ((left > 0u64)? | true => grow(sequence/append(words, left), left - 1u64) | false => {nodes: words}).\nfirst := grow(sequence/with-capacity(0u64, capacity<u64>), steps<u64>)\nsecond := grow(sequence/clear(first.nodes), steps)\nsequence/append(second.nodes, 17u64)\n";
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        for steps in [0, 1, 3] {
            let memory = ManagedMemoryBudget::new(128 * 1024 * 1024);
            let args = inputs(
                &artifact,
                &[("capacity", Data::U64(305257)), ("steps", Data::U64(steps))],
                &memory,
            );
            let mut output = vec![None; artifact.outputs().len()];
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            let value = output.last().unwrap().as_ref().unwrap();
            let mut expected = (1..=steps).rev().map(Data::U64).collect::<Vec<_>>();
            expected.push(Data::U64(17));
            assert_eq!(
                value.canonical_data_draft().unwrap(),
                Data::Matrix(expected.into())
            );
            let ValueData::Matrix(matrix) = value.data() else {
                panic!()
            };
            assert_eq!(matrix.unsigned_buffer_capacity(), Some(305257));
            drop((args, output));
            assert_eq!(memory.used_bytes(), 0);
            assert_eq!(memory.retained_snapshot_nodes(), 0);
        }
    }
}

#[test]
fn record_fields_keep_fresh_values_aliases_publication_and_failure_recovery() {
    let source = "+> sequence\n<packet> := <{count<u64> nodes<[u64]> allowed<bool>}>\npack(nodes<[u64]>, count<u64>, allowed<bool>) => (out<packet>) :=\n  out := {count: count, nodes: nodes, allowed: allowed}.\nchecked(values<[u64]>, count<u64>, allowed<bool>, divisor<u64>) => (out<[u64]>) :=\n  packet := pack(values, count, allowed)\n  enabled! := packet.allowed\n  word := packet.count + 12u64 / divisor\n  out := sequence/append(sequence/clear(packet.nodes), word).\nchecked(values<[u64]>, count<u64>, allowed<bool>, divisor<u64>)\n";
    for artifact in artifacts(source).0 {
        let plan =
            lower_native_program_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert!(plan.facts().buffer_frames().elements() > 0);
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(32 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for (count, allowed, divisor) in [(7, true, 3), (91, false, 0), (6, true, 0), (19, true, 2)]
        {
            let values = Data::Matrix(vec![Data::U64(41), Data::U64(43), Data::U64(47)].into());
            let args = inputs(
                &artifact,
                &[
                    ("values", values.clone()),
                    ("count", Data::U64(count)),
                    ("allowed", Data::Bool(allowed)),
                    ("divisor", Data::U64(divisor)),
                ],
                &memory,
            );
            let aliases = args.clone();
            let prior = output.clone();
            let retained = memory.used_bytes();
            let nodes = memory.retained_snapshot_nodes();
            let result = jit.evaluate_into(&args, &mut output, limits(), &memory);
            let resident = reference_result(&artifact, &args);
            if allowed && divisor != 0 {
                result.unwrap();
                super::equal(&artifact, &output, &resident.unwrap());
                assert_eq!(
                    output
                        .last()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    Data::Matrix(vec![Data::U64(count + 12 / divisor)].into())
                );
            } else {
                let error = format!("{:?}", result.unwrap_err());
                let resident = resident.unwrap_err();
                assert_eq!(error.contains("Integrity"), !allowed, "{error}");
                assert_eq!(
                    resident.contains("LexicalIntegrity"),
                    !allowed,
                    "{resident}"
                );
                assert_eq!(memory.used_bytes(), retained);
                assert_eq!(memory.retained_snapshot_nodes(), nodes);
                assert!(output.iter().zip(&prior).all(|(v, p)| {
                    v.as_ref()
                        .unwrap()
                        .shares_frozen_storage(p.as_ref().unwrap())
                }));
            }
            assert!(
                args.iter()
                    .zip(&aliases)
                    .all(|(v, a)| v.shares_frozen_storage(a))
            );
            let values_index = artifact
                .inputs()
                .iter()
                .position(|input| {
                    mech_engine::decode_source_input_name(&input.name).unwrap() == "values"
                })
                .unwrap();
            assert_eq!(args[values_index].canonical_data_draft().unwrap(), values);
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
        assert_eq!(memory.retained_snapshot_nodes(), 0);
    }
}

#[test]
fn escaping_records_keep_canonical_storage_and_image_layout_is_rederived() {
    for source in [
        "{nodes: values<[u64]>}\n",
        "pack(nodes<[u64]>) => (out<{nodes<[u64]>}>) :=\n  out := {nodes: nodes}.\npack(values<[u64]>)\n",
    ] {
        for artifact in artifacts(source).0 {
            let plan =
                lower_native_program_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
            assert_eq!(plan.facts().buffer_frames().elements(), 0);
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let memory = ManagedMemoryBudget::new(16 * 1024 * 1024);
            let args = inputs(
                &artifact,
                &[(
                    "values",
                    Data::Matrix(vec![Data::U64(37), Data::U64(41)].into()),
                )],
                &memory,
            );
            let mut output = vec![None; artifact.outputs().len()];
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            assert!(matches!(
                output.last().unwrap().as_ref().unwrap().data(),
                ValueData::Record(_)
            ));
            super::equal(
                &artifact,
                &output,
                &reference_result(&artifact, &args).unwrap(),
            );
            drop((args, output));
            assert_eq!(memory.used_bytes(), 0);
        }
    }
    for artifact in artifacts(PACK).0 {
        let catalog = mech_stdlib::source_catalog();
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let bytes = jit.descriptor().encode_build_owned_image().unwrap();
        // SAFETY: these bytes came from this checked build. Restored metadata
        // is confined to comparison; this does not authorize execution.
        let restored = unsafe {
            mech_gpu::native_artifact::NativeProgramDescriptor::decode_build_owned_image(
                &bytes, &catalog,
            )
        }
        .unwrap();
        assert_eq!(restored.encode_build_owned_image().unwrap(), bytes);
    }
}

#[test]
fn record_field_retention_keeps_selected_call_admission_and_zero_copy() {
    use mech_engine::native_artifact::{
        NativeCollectionPrimitive as Primitive, NativeIterationAction as Action,
        NativeLibraryWorkBudget, prepare_native_program_library_call,
    };
    let source = "pack(nodes<[u64]>) => (out<{nodes<[u64]>}>) :=\n  out := {nodes: nodes}.\npacket := pack(values<[u64]>)\npacket.nodes\n";
    for artifact in artifacts(source).0 {
        let catalog = mech_stdlib::source_catalog();
        let plan = lower_native_program_artifact(&artifact, &catalog).unwrap();
        let (template, selector) = plan
            .blocks()
            .iter()
            .find_map(|block| match block.action() {
                Action::LibraryCall {
                    template, operands, ..
                } if template.primitive() == Primitive::RecordFieldAccess => {
                    Some((template, operands[1]))
                }
                _ => None,
            })
            .unwrap();
        let call = prepare_native_program_library_call(&artifact, template, &catalog).unwrap();
        let memory = ManagedMemoryBudget::new(16 * 1024 * 1024);
        let args = inputs(
            &artifact,
            &[(
                "values",
                Data::Matrix(vec![Data::U64(37), Data::U64(41), Data::U64(43)].into()),
            )],
            &memory,
        );
        let constant = plan
            .constants()
            .iter()
            .find(|v| v.value() == selector)
            .unwrap()
            .constant()
            .clone();
        let mut reserve = memory
            .reserve_capacity(constant.memory_budget_admission_bytes(&memory).unwrap())
            .unwrap();
        let selector = constant.into_memory_budget(&mut reserve).unwrap();
        drop(reserve);
        let work = NativeLibraryWorkBudget::new(1 << 40, 1 << 40, &memory).unwrap();
        let retained = memory.used_bytes();
        let nodes = memory.retained_snapshot_nodes();
        let mut policy = limits().library;
        policy.max_cloned_bytes = Some(0);
        policy.max_output_elements = Some(1);
        call.admit_buffer_record_field(&selector, 0, &args[0], policy, &work, Default::default())
            .unwrap();
        let (result, requests, bytes) = allocation_probe::measured_with_bytes(|| {
            call.admit_buffer_record_field(
                &selector,
                0,
                &args[0],
                policy,
                &work,
                Default::default(),
            )
        });
        result.unwrap();
        assert_eq!((requests, bytes), (0, 0));
        assert!(
            call.admit_buffer_record_field(
                &selector,
                1,
                &args[0],
                policy,
                &work,
                Default::default()
            )
            .is_err()
        );
        for dimension in 0..6 {
            let mut tight = policy;
            let expected = match dimension {
                0 => {
                    tight.max_output_elements = Some(0);
                    "OutputElements"
                }
                1 => {
                    tight.max_output_bytes = Some(0);
                    "OutputBytes"
                }
                2 => {
                    tight.max_temporary_bytes = Some(0);
                    "TemporaryBytes"
                }
                3 => {
                    tight.max_retained_nodes = Some(0);
                    "RetainedNodes"
                }
                4 => {
                    tight.max_compute_work = Some(0);
                    "ComputeWork"
                }
                _ => {
                    tight.max_comparison_work = Some(0);
                    "ComparisonWork"
                }
            };
            let error = call
                .admit_buffer_record_field(&selector, 0, &args[0], tight, &work, Default::default())
                .unwrap_err();
            assert!(format!("{error:?}").contains(expected), "{error:?}");
            assert_eq!(memory.used_bytes(), retained);
            assert_eq!(memory.retained_snapshot_nodes(), nodes);
            call.admit_buffer_record_field(
                &selector,
                0,
                &args[0],
                policy,
                &work,
                Default::default(),
            )
            .unwrap();
        }
        drop((args, selector, work));
        assert_eq!(memory.used_bytes(), 0);
        assert_eq!(memory.retained_snapshot_nodes(), 0);
    }
}

#[test]
fn shared_record_children_keep_independent_values_after_clear_and_append() {
    let source = "+> sequence\npacket := {first: values<[u64]>, second: values}\nchanged := sequence/append(sequence/clear(packet.first), 17u64)\n(changed, packet.second)\n";
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(16 * 1024 * 1024);
        let args = inputs(
            &artifact,
            &[(
                "values",
                Data::Matrix(vec![Data::U64(37), Data::U64(41)].into()),
            )],
            &memory,
        );
        let alias = args[0].clone();
        let mut output = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut output, limits(), &memory)
            .unwrap();
        super::equal(
            &artifact,
            &output,
            &reference_result(&artifact, &args).unwrap(),
        );
        assert_eq!(
            output
                .last()
                .unwrap()
                .as_ref()
                .unwrap()
                .canonical_data_draft()
                .unwrap(),
            Data::Tuple(
                vec![
                    Data::Matrix(vec![Data::U64(17)].into()),
                    Data::Matrix(vec![Data::U64(37), Data::U64(41)].into())
                ]
                .into()
            )
        );
        assert!(args[0].shares_frozen_storage(&alias));
        assert_eq!(
            args[0].canonical_data_draft().unwrap(),
            Data::Matrix(vec![Data::U64(37), Data::U64(41)].into())
        );
        drop((args, alias, output));
        assert_eq!(memory.used_bytes(), 0);
        assert_eq!(memory.retained_snapshot_nodes(), 0);
    }
}

#[test]
fn selected_record_buffer_capture_preserves_aliases_failure_and_retry() {
    let source = "+> sequence\nchoose(flag<bool>, bytes<[u8]>, words<[u32]>, at<u64>) => (out<[u32]>) :=\n  tally := {freqs-litlen: words, freqs-offset: words, litlen-lens: bytes, offset-lens: bytes, litlen-codewords: words, offset-codewords: words}\n  out := (flag? | true => sequence/append(tally.litlen-codewords, tally.freqs-litlen[at] + tally.freqs-offset[at] + tally.offset-codewords[at]) | false => sequence/append(tally.offset-codewords, 0u32)).\nchoose(flag<bool>, bytes<[u8]>, words<[u32]>, at<u64>)\n";
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(32 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for (flag, words, at, fails) in [
            (true, vec![7u32, 11], 2, false),
            (false, vec![13], 999, false),
            (true, vec![17], 2, true),
            (true, vec![19, 23, 29], 3, false),
        ] {
            let args = inputs(
                &artifact,
                &[
                    ("flag", Data::Bool(flag)),
                    ("bytes", Data::Matrix(vec![Data::U8(3)].into())),
                    (
                        "words",
                        Data::Matrix(words.iter().copied().map(Data::U32).collect()),
                    ),
                    ("at", Data::U64(at)),
                ],
                &memory,
            );
            let aliases = args.clone();
            let prior = output.clone();
            let retained = memory.used_bytes();
            let nodes = memory.retained_snapshot_nodes();
            let native = jit.evaluate_into(&args, &mut output, limits(), &memory);
            let resident = reference_result(&artifact, &args);
            if fails {
                assert!(native.is_err());
                assert!(resident.is_err());
                assert!(prior.iter().zip(&output).all(|(a, b)| {
                    a.as_ref()
                        .unwrap()
                        .shares_frozen_storage(b.as_ref().unwrap())
                }));
                assert_eq!(memory.used_bytes(), retained);
                assert_eq!(memory.retained_snapshot_nodes(), nodes);
            } else {
                native.unwrap();
                super::equal(&artifact, &output, &resident.unwrap());
                let mut expected = words.iter().copied().map(Data::U32).collect::<Vec<_>>();
                if flag {
                    expected.push(Data::U32(words[at as usize - 1] * 3));
                } else {
                    expected.push(Data::U32(0));
                }
                assert_eq!(
                    output
                        .last()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    Data::Matrix(expected.into())
                );
            }
            assert!(
                args.iter()
                    .zip(&aliases)
                    .all(|(a, b)| a.shares_frozen_storage(b))
            );
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
        assert_eq!(memory.retained_snapshot_nodes(), 0);
    }
}
