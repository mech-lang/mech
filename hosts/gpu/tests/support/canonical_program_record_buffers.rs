//! Private record returns preserve reusable buffers and ordinary value semantics.
use super::{allocation_probe, artifacts, inputs, limits, reference_result};
use mech_core::{ManagedMemoryBudget, ValueData, ValueDataDraft as Data};
use mech_engine::native_artifact::lower_native_program_artifact;
use mech_gpu::native_artifact::JitProgramArtifact;

const PACK: &str = "+> sequence\npack(nodes<[u64]>) => (out<{nodes<[u64]>}>) :=\n  out := {nodes: nodes}.\nreturned := pack(sequence/with-capacity(0u64, capacity<u64>))\nsequence/append(returned.nodes, 17u64)\n";

#[test]
fn record_annotation_renaming_keeps_signed_float_siblings_and_independent_buffers() {
    let source = "pack(words<[u16]>, bytes<[u8]>, signed<i64>, real<f64>) => (out<{bytes<[u8]> words<[u16]> signed<i64> real<f64>}>) :=\n  out := {bytes: bytes, words: words, signed: signed, real: real}.\npack(words<[u16]>, bytes<[u8]>, signed<i64>, real<f64>)\n";
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(32 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for (bytes, words, real) in [
            (3, 5, 0x8000000000000000),
            (0, 2, 0x7ff8000000000042),
            (5, 1, 0),
        ] {
            let args = inputs(
                &artifact,
                &[
                    (
                        "bytes",
                        Data::Matrix((0..bytes).map(|i| Data::U8(i as u8)).collect()),
                    ),
                    (
                        "words",
                        Data::Matrix((0..words).map(|i| Data::U16(i as u16 + 257)).collect()),
                    ),
                    ("signed", Data::I64(i64::MIN)),
                    (
                        "real",
                        Data::F64(mech_core::snapshot::F64Bits::from_bits(real)),
                    ),
                ],
                &memory,
            );
            let aliases = args.clone();
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            super::equal(
                &artifact,
                &output,
                &reference_result(&artifact, &args).unwrap(),
            );
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

#[test]
fn record_matrix_construction_and_update_preserve_publication_and_retry() {
    let source = "<entry> := <{length<u32> offset<u16> slot<u16>}>\nupdate(length<u32>, index<u64>) => (out<[entry]>) :=\n  ~entries := [{length: 0u32, offset: 3u16, slot: 5u16}, {length: 7u32, offset: 11u16, slot: 13u16}]\n  entries[index] = {length: length, offset: 17u16, slot: 19u16}\n  out := entries.\nupdate(length<u32>, index<u64>)\n";
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(32 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for (length, index) in [(320, 1), (91, 3), (640, 2), (17, 0), (19, 1)] {
            let args = inputs(
                &artifact,
                &[("length", Data::U32(length)), ("index", Data::U64(index))],
                &memory,
            );
            let aliases = args.clone();
            let prior = output.clone();
            let retained = memory.used_bytes();
            let nodes = memory.retained_snapshot_nodes();
            let native = jit.evaluate_into(&args, &mut output, limits(), &memory);
            let resident = reference_result(&artifact, &args);
            if (1..=2).contains(&index) {
                native.unwrap();
                super::equal(&artifact, &output, &resident.unwrap());
                let record = |a, b, c| {
                    Data::Record(
                        [
                            ("length", Data::U32(a)),
                            ("offset", Data::U16(b)),
                            ("slot", Data::U16(c)),
                        ]
                        .into_iter()
                        .map(|(name, value)| mech_core::snapshot::NamedValueDraft {
                            name: name.into(),
                            value,
                        })
                        .collect(),
                    )
                };
                let mut expected = vec![record(0, 3, 5), record(7, 11, 13)];
                expected[index as usize - 1] = record(length, 17, 19);
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
            } else {
                assert!(native.is_err());
                assert!(resident.is_err());
                assert_eq!(memory.used_bytes(), retained);
                assert_eq!(memory.retained_snapshot_nodes(), nodes);
                assert!(output.iter().zip(&prior).all(|(value, previous)| {
                    value
                        .as_ref()
                        .unwrap()
                        .shares_frozen_storage(previous.as_ref().unwrap())
                }));
            }
            assert!(
                args.iter()
                    .zip(&aliases)
                    .all(|(value, alias)| value.shares_frozen_storage(alias))
            );
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
        assert_eq!(memory.retained_snapshot_nodes(), 0);
    }
}

#[test]
fn record_matrix_update_only_executes_on_the_selected_path() {
    let source = "<entry> := <{length<u32> offset<u16> slot<u16>}>\nupdate(index<u64>) => (out<[entry]>) :=\n  ~entries := [{length: 7u32, offset: 11u16, slot: 13u16}]\n  entries[index] = {length: 19u32, offset: 17u16, slot: 23u16}\n  out := entries.\n(selected<bool>? | true => update(index<u64>) | false => update(1u64))\n";
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(32 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for (selected, index) in [(false, 99), (true, 99), (true, 1), (false, 0)] {
            let args = inputs(
                &artifact,
                &[
                    ("selected", Data::Bool(selected)),
                    ("index", Data::U64(index)),
                ],
                &memory,
            );
            let prior = output.clone();
            let retained = memory.used_bytes();
            let nodes = memory.retained_snapshot_nodes();
            let native = jit.evaluate_into(&args, &mut output, limits(), &memory);
            let resident = reference_result(&artifact, &args);
            if selected && index != 1 {
                assert!(native.is_err());
                assert!(resident.is_err());
                assert_eq!(memory.used_bytes(), retained);
                assert_eq!(memory.retained_snapshot_nodes(), nodes);
                assert!(output.iter().zip(&prior).all(|(value, previous)| {
                    value
                        .as_ref()
                        .unwrap()
                        .shares_frozen_storage(previous.as_ref().unwrap())
                }));
            } else {
                native.unwrap();
                super::equal(&artifact, &output, &resident.unwrap());
            }
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
        assert_eq!(memory.retained_snapshot_nodes(), 0);
    }
}

#[test]
fn record_matrix_horizontal_and_vertical_constructors_preserve_geometry() {
    for separator in [",", ";"] {
        let source = format!(
            "[{{length: value<u32>, offset: 3u16, slot: 5u16}}{separator} {{length: 7u32, offset: 11u16, slot: 13u16}}]\n"
        );
        for artifact in artifacts(&source).0 {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let memory = ManagedMemoryBudget::new(32 * 1024 * 1024);
            let args = inputs(&artifact, &[("value", Data::U32(320))], &memory);
            let mut output = vec![None; artifact.outputs().len()];
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            super::equal(
                &artifact,
                &output,
                &reference_result(&artifact, &args).unwrap(),
            );
            let Data::Matrix(values) = output
                .last()
                .unwrap()
                .as_ref()
                .unwrap()
                .canonical_data_draft()
                .unwrap()
            else {
                panic!()
            };
            assert_eq!(values.len(), 2);
            drop((args, output));
            assert_eq!(memory.used_bytes(), 0);
            assert_eq!(memory.retained_snapshot_nodes(), 0);
        }
    }
}

#[test]
fn record_matrix_scalar_reads_preserve_fields_aliases_and_retry() {
    // Control capture of record matrices is a separate source-semantics gap.
    // This witness exercises selection from an external matrix directly.
    let source = "<entry> := <{length<u32> offset<u16> slot<u16>}>\nitems := entries<[entry]>\nindex := (advance<bool>? | true => at<u64> + 1u64 | false => at<u64>)\ncurrent := items[index]\n(current, current.length, current.offset, current.slot)\n";
    let record = |length, offset, slot| {
        Data::Record(
            [
                ("length", Data::U32(length)),
                ("offset", Data::U16(offset)),
                ("slot", Data::U16(slot)),
            ]
            .into_iter()
            .map(|(name, value)| mech_core::snapshot::NamedValueDraft {
                name: name.into(),
                value,
            })
            .collect(),
        )
    };
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(32 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for (at, advance, expected) in [
            (1, false, Some((0x81234567, 3, 5))),
            (0, false, None),
            (2, false, Some((258, 32768, 29))),
            (3, false, None),
            (u64::MAX, true, None),
            (1, true, Some((258, 32768, 29))),
        ] {
            let args = inputs(
                &artifact,
                &[
                    (
                        "entries",
                        Data::Matrix(vec![record(0x81234567, 3, 5), record(258, 32768, 29)].into()),
                    ),
                    ("at", Data::U64(at)),
                    ("advance", Data::Bool(advance)),
                ],
                &memory,
            );
            let aliases = args.clone();
            let prior = output.clone();
            let before = memory.used_bytes();
            let result = jit.evaluate_into(&args, &mut output, limits(), &memory);
            let resident = reference_result(&artifact, &args);
            if let Some((length, offset, slot)) = expected {
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
                    Data::Tuple(
                        vec![
                            record(length, offset, slot),
                            Data::U32(length),
                            Data::U16(offset),
                            Data::U16(slot)
                        ]
                        .into()
                    )
                );
            } else {
                assert!(result.is_err());
                assert!(resident.is_err());
                for (old, current) in prior.iter().zip(&output) {
                    assert!(
                        old.as_ref()
                            .unwrap()
                            .shares_frozen_storage(current.as_ref().unwrap())
                    );
                }
                assert_eq!(memory.used_bytes(), before);
            }
            assert!(
                args.iter()
                    .zip(&aliases)
                    .all(|(a, b)| a.shares_frozen_storage(b))
            );
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn record_matrix_nested_elements_select_whole_values_with_finite_admission() {
    let source = "<entry> := <{parts<(u16,{length<u32>})> label<string>}>\nitems := entries<[entry]>\nitems[at<u64>]\n";
    let record = Data::Record(
        vec![
            mech_core::snapshot::NamedValueDraft {
                name: "parts".into(),
                value: Data::Tuple(
                    vec![
                        Data::U16(32768),
                        Data::Record(
                            vec![mech_core::snapshot::NamedValueDraft {
                                name: "length".into(),
                                value: Data::U32(258),
                            }]
                            .into(),
                        ),
                    ]
                    .into(),
                ),
            },
            mech_core::snapshot::NamedValueDraft {
                name: "label".into(),
                value: Data::String("selected é".into()),
            },
        ]
        .into(),
    );
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(32 * 1024 * 1024);
        let args = inputs(
            &artifact,
            &[
                ("entries", Data::Matrix(vec![record.clone()].into())),
                ("at", Data::U64(1)),
            ],
            &memory,
        );
        let mut output = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut output, limits(), &memory)
            .unwrap();
        let expected = reference_result(&artifact, &args).unwrap();
        super::equal(&artifact, &output, &expected);
        for denied in [
            {
                let mut p = limits();
                p.library.max_compute_work = Some(0);
                p
            },
            {
                let mut p = limits();
                p.library.max_output_bytes = Some(0);
                p
            },
        ] {
            let prior = output.clone();
            let before = memory.used_bytes();
            assert!(
                jit.evaluate_into(&args, &mut output, denied, &memory)
                    .is_err()
            );
            assert_eq!(memory.used_bytes(), before);
            for (old, current) in prior.iter().zip(&output) {
                assert!(
                    old.as_ref()
                        .unwrap()
                        .shares_frozen_storage(current.as_ref().unwrap())
                );
            }
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            super::equal(&artifact, &output, &expected);
        }
        drop((args, output, expected));
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn record_matrix_scalar_read_does_not_admit_child_container_elements() {
    let source = "<entry> := <{words<[u64]:1,2>}>\nitems := entries<[entry]>\nitems[at<u64>]\n";
    for artifact in artifacts(source).0 {
        let error = match JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()) {
            Ok(_) => panic!("child containers require their own maintained witness"),
            Err(error) => error,
        };
        assert!(
            format!("{error:?}").contains("WitnessTypeMismatch"),
            "{error:?}"
        );
    }
}

#[test]
fn record_matrix_scalar_read_only_executes_on_selected_path() {
    let source = "read(at<u64>) => (out<u32>) :=\n  entries := [{length: 258u32, offset: 3u16, slot: 0u16}]\n  current := entries[at]\n  out := current.length.\n(enabled<bool>? | true => read(at<u64>) | false => 7u32)\n";
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(32 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for (enabled, at) in [(false, 0), (true, 0), (true, 1), (false, u64::MAX)] {
            let args = inputs(
                &artifact,
                &[("enabled", Data::Bool(enabled)), ("at", Data::U64(at))],
                &memory,
            );
            let prior = output.clone();
            let retained = memory.used_bytes();
            let nodes = memory.retained_snapshot_nodes();
            let result = jit.evaluate_into(&args, &mut output, limits(), &memory);
            let resident = reference_result(&artifact, &args);
            if enabled && at != 1 {
                assert!(result.is_err());
                assert!(resident.is_err());
                assert_eq!(memory.used_bytes(), retained);
                assert_eq!(memory.retained_snapshot_nodes(), nodes);
                assert!(output.iter().zip(&prior).all(|(a, b)| {
                    a.as_ref()
                        .unwrap()
                        .shares_frozen_storage(b.as_ref().unwrap())
                }));
            } else {
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
                    Data::U32(if enabled { 258 } else { 7 })
                );
            }
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
        assert_eq!(memory.retained_snapshot_nodes(), 0);
    }
}

#[test]
fn recursive_record_matrix_arguments_keep_fresh_fields_aliases_and_retry() {
    let source = "+> sequence\n<entry> := <{length<u32> enabled<bool> offset<u16>}>\nbump(entries<[entry]>, left<u32>, at<u64>) => (out<[entry]>) :=\n  current := entries[at]\n  ~next := entries\n  next[at] = {length: current.length + left, enabled: (current.enabled? | true => false | false => true), offset: current.offset}\n  out := next.\nwalk(entries<[entry]>, left<u32>, at<u64>) => (out<(u32,bool,u64)>) :=\n  current := entries[at]\n  next := ((left > 0u32)? | true => bump(entries, left, at) | false => entries)\n  out := ((left > 0u32)? | true => walk(next, left - 1u32, at) | false => (current.length, current.enabled, sequence/length(entries))).\nwalk(entries<[entry]>, left<u32>, at<u64>)\n";
    let record = |length, enabled| {
        Data::Record(
            [
                ("length", Data::U32(length)),
                ("enabled", Data::Bool(enabled)),
                ("offset", Data::U16(32768)),
            ]
            .into_iter()
            .map(|(name, value)| mech_core::snapshot::NamedValueDraft {
                name: name.into(),
                value,
            })
            .collect(),
        )
    };
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(32 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for (initial, left, at, len, expected) in [
            (10, 3, 1, 2, Some((16, false))),
            (10, 3, 0, 2, None),
            (21, 2, 1, 1, Some((24, true))),
            (u32::MAX, 1, 1, 2, None),
            (31, 0, 1, 2, Some((31, true))),
        ] {
            let original = Data::Matrix(vec![record(initial, true); len].into());
            let args = inputs(
                &artifact,
                &[
                    ("entries", original.clone()),
                    ("left", Data::U32(left)),
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
            if let Some((length, enabled)) = expected {
                native.unwrap();
                super::equal(&artifact, &output, &resident.unwrap());
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
                            Data::U32(length),
                            Data::Bool(enabled),
                            Data::U64(len as u64)
                        ]
                        .into()
                    )
                );
            } else {
                assert!(native.is_err());
                assert!(resident.is_err());
                assert_eq!(memory.used_bytes(), retained);
                assert_eq!(memory.retained_snapshot_nodes(), nodes);
                assert!(output.iter().zip(&prior).all(|(a, b)| {
                    a.as_ref()
                        .unwrap()
                        .shares_frozen_storage(b.as_ref().unwrap())
                }));
            }
            assert!(
                args.iter()
                    .zip(&aliases)
                    .all(|(a, b)| a.shares_frozen_storage(b))
            );
            assert!(
                aliases
                    .iter()
                    .any(|value| value.canonical_data_draft().unwrap() == original)
            );
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
        assert_eq!(memory.retained_snapshot_nodes(), 0);
    }
}

#[test]
fn selected_record_matrix_capture_skips_failed_reads_and_recovers() {
    let source = "<entry> := <{length<u32> offset<u16>}>\nread(entries<[entry]>, at<u64>) => (out<u32>) :=\n  current := entries[at]\n  out := current.length.\n(enabled<bool>? | true => read(entries<[entry]>, at<u64>) | false => 7u32)\n";
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(32 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for (enabled, at, length) in [
            (false, 0, 11),
            (true, 0, 23),
            (true, 1, 31),
            (false, u64::MAX, 41),
        ] {
            let record = Data::Record(
                [("length", Data::U32(length)), ("offset", Data::U16(32768))]
                    .into_iter()
                    .map(|(name, value)| mech_core::snapshot::NamedValueDraft {
                        name: name.into(),
                        value,
                    })
                    .collect(),
            );
            let args = inputs(
                &artifact,
                &[
                    ("entries", Data::Matrix(vec![record].into())),
                    ("at", Data::U64(at)),
                    ("enabled", Data::Bool(enabled)),
                ],
                &memory,
            );
            let prior = output.clone();
            let retained = memory.used_bytes();
            let nodes = memory.retained_snapshot_nodes();
            let native = jit.evaluate_into(&args, &mut output, limits(), &memory);
            let resident = reference_result(&artifact, &args);
            if enabled && at == 0 {
                assert!(native.is_err());
                assert!(resident.is_err());
                assert_eq!(memory.used_bytes(), retained);
                assert_eq!(memory.retained_snapshot_nodes(), nodes);
                assert!(output.iter().zip(&prior).all(|(a, b)| {
                    a.as_ref()
                        .unwrap()
                        .shares_frozen_storage(b.as_ref().unwrap())
                }));
            } else {
                native.unwrap();
                super::equal(&artifact, &output, &resident.unwrap());
                assert_eq!(
                    output
                        .last()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    Data::U32(if enabled { length } else { 7 })
                );
            }
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
        assert_eq!(memory.retained_snapshot_nodes(), 0);
    }
}

#[test]
fn recursive_tuple_matrix_arguments_preserve_nested_fixed_elements() {
    let source = "walk(entries<[(u32,bool)]>, left<u64>) => (out<[(u32,bool)]>) :=\n  out := ((left > 0u64)? | true => walk(entries, left - 1u64) | false => entries).\nwalk(entries<[(u32,bool)]>, left<u64>)\n";
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(32 * 1024 * 1024);
        for count in [0, 1, 2] {
            let expected = Data::Matrix(
                (0..count)
                    .map(|n| Data::Tuple(vec![Data::U32(n + 17), Data::Bool(n % 2 == 0)].into()))
                    .collect(),
            );
            let args = inputs(
                &artifact,
                &[("entries", expected.clone()), ("left", Data::U64(2))],
                &memory,
            );
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
                expected
            );
        }
        assert_eq!(memory.used_bytes(), 0);
        assert_eq!(memory.retained_snapshot_nodes(), 0);
    }
}

#[test]
fn mixed_record_buffer_frames_keep_capacity_aliases_and_failure_recovery() {
    let source = "+> sequence\n<entry> := <{length<u32> offset<u16>}>\nappend-byte(bytes<[u8]>, at<u64>) => (out<[u8]>) :=\n  byte<u8> := at\n  out := sequence/append(bytes, byte).\nwalk(bytes<[u8]>, entries<[entry]>, at<u64>, reject-at<u64>) => (out<{bytes<[u8]>}>) :=\n  allowed! := at != reject-at\n  more := at < sequence/length(entries)\n  next := (more? | true => append-byte(bytes, at) | false => bytes)\n  out := (more? | true => walk(next, entries, at + 1u64, reject-at) | false => {bytes: next}).\nwritten := walk(sequence/with-capacity(0u8, capacity<u64>), entries<[entry]>, 0u64, reject-at<u64>)\nsequence/append(sequence/clear(written.bytes), 17u8)\n";
    let record = Data::Record(
        [("length", Data::U32(258)), ("offset", Data::U16(32768))]
            .into_iter()
            .map(|(name, value)| mech_core::snapshot::NamedValueDraft {
                name: name.into(),
                value,
            })
            .collect(),
    );
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let mut measured = Vec::new();
        for capacity in [512, 6633832] {
            let memory = ManagedMemoryBudget::new(128 * 1024 * 1024);
            let mut output = vec![None; artifact.outputs().len()];
            for (count, reject) in [(3, u64::MAX), (3, 1), (2, u64::MAX), (2, 3)] {
                let entries = Data::Matrix(vec![record.clone(); count].into());
                let args = inputs(
                    &artifact,
                    &[
                        ("entries", entries.clone()),
                        ("capacity", Data::U64(capacity)),
                        ("reject-at", Data::U64(reject)),
                    ],
                    &memory,
                );
                let aliases = args.clone();
                let prior = output.clone();
                let retained = memory.used_bytes();
                let nodes = memory.retained_snapshot_nodes();
                let mut policy = limits();
                policy.library.max_cloned_bytes = Some(0);
                let result = jit.evaluate_into(&args, &mut output, policy, &memory);
                // The resident snapshot path exceeds its unchanged 16 MiB temporary
                // budget at the real reservation. Native zero-copy ownership is
                // qualified at both sizes; semantic agreement uses the small one.
                let resident = (capacity == 512).then(|| reference_result(&artifact, &args));
                if reject == 1 {
                    if let Some(resident) = resident {
                        assert!(resident.is_err());
                    }
                    let error = format!("{:?}", result.unwrap_err());
                    assert!(error.contains("Integrity"), "{error}");
                    assert_eq!(memory.used_bytes(), retained);
                    assert_eq!(memory.retained_snapshot_nodes(), nodes);
                    assert!(output.iter().zip(&prior).all(|(a, b)| {
                        a.as_ref()
                            .unwrap()
                            .shares_frozen_storage(b.as_ref().unwrap())
                    }));
                } else {
                    result.unwrap();
                    if let Some(resident) = resident {
                        super::equal(&artifact, &output, &resident.unwrap());
                    }
                    let value = output.last().unwrap().as_ref().unwrap();
                    assert_eq!(
                        value.canonical_data_draft().unwrap(),
                        Data::Matrix(vec![Data::U8(17)].into())
                    );
                    let ValueData::Matrix(matrix) = value.data() else {
                        panic!()
                    };
                    assert_eq!(matrix.unsigned_buffer_capacity(), Some(capacity as usize));
                    if count == 3 {
                        let (result, requests, bytes) =
                            allocation_probe::measured_with_bytes(|| {
                                jit.evaluate_into(&args, &mut output, policy, &memory)
                            });
                        result.unwrap();
                        measured.push(bytes);
                        println!(
                            "{}",
                            serde_json::json!({"witness":"mixed-record-buffer-frames","capacity":capacity,"requests":requests,"allocated_bytes":bytes})
                        );
                    }
                }
                assert!(
                    args.iter()
                        .zip(&aliases)
                        .all(|(a, b)| a.shares_frozen_storage(b))
                );
                assert!(
                    aliases
                        .iter()
                        .any(|value| value.canonical_data_draft().unwrap() == entries)
                );
            }
            drop(output);
            assert_eq!(memory.used_bytes(), 0);
            assert_eq!(memory.retained_snapshot_nodes(), 0);
        }
        assert_eq!(measured[1] - measured[0], 6633832 - 512);
    }
}

#[path = "canonical_program_opaque_frame_siblings.rs"]
mod opaque_siblings;
