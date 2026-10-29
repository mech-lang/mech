//! Ordinary recursive frames retain immutable buffers rather than copying them.
use super::{
    JitProgramArtifact, ManagedMemoryBudget, ValueDataDraft as Data, allocation_probe, artifacts,
    inputs, limits, reference_result,
};
use mech_core::ValueData;
use mech_engine::native_artifact::lower_native_program_artifact;

const GROW: &str = "+> sequence\ngrow(words<[u64]>, left<u64>) => <[u64]>\n  | (words, left) => ((left > 0u64)?\n      | true => grow(sequence/append(words, left), left - 1u64)\n      | false => words).\ngrow(sequence/with-capacity(0u64, capacity<u64>), steps<u64>)\n";

#[test]
fn recursive_buffers_retain_real_reservations_without_payload_copies() {
    for (width, real) in [(64, 305257), (8, 6633832)] {
        let source = if width == 64 {
            GROW.to_owned()
        } else {
            GROW.replace("[u64]", "[u8]")
                .replace("append(words, left)", "append(words, 129u8)")
                .replace("0u64, capacity", "0u8, capacity")
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
            let mut measurements = Vec::new();
            for capacity in [16, real] {
                for steps in [0, 1, 3] {
                    let memory = ManagedMemoryBudget::new(128 * 1024 * 1024);
                    let args = inputs(
                        &artifact,
                        &[
                            ("capacity", Data::U64(capacity)),
                            ("steps", Data::U64(steps)),
                        ],
                        &memory,
                    );
                    let mut output = vec![None; artifact.outputs().len()];
                    let mut policy = limits();
                    // Same finite diagnostic policy as the saved red acceptance.
                    policy.execution.max_work = 1_000_000_000_000;
                    policy.library.max_compute_work = Some(1_000_000_000_000);
                    // Warm immutable metadata and the allocator probe equally.
                    allocation_probe::measured_with_bytes(|| {
                        jit.evaluate_into(&args, &mut output, policy, &memory)
                    })
                    .0
                    .unwrap();
                    let start = std::time::Instant::now();
                    let (result, requests, bytes) = allocation_probe::measured_with_bytes(|| {
                        jit.evaluate_into(&args, &mut output, policy, &memory)
                    });
                    result.unwrap();
                    let value = output.last().unwrap().as_ref().unwrap();
                    let expected = Data::Matrix(
                        (1..=steps)
                            .rev()
                            .map(|n| {
                                if width == 64 {
                                    Data::U64(n)
                                } else {
                                    Data::U8(129)
                                }
                            })
                            .collect(),
                    );
                    assert_eq!(value.canonical_data_draft().unwrap(), expected);
                    let ValueData::Matrix(matrix) = value.data() else {
                        panic!()
                    };
                    assert_eq!(matrix.unsigned_buffer_capacity(), Some(capacity as usize));
                    println!(
                        "{}",
                        serde_json::json!({"witness":"recursive-reserved-buffer","artifact":form,"width":width,"capacity":capacity,"steps":steps,"requests":requests,"requested_bytes":bytes,"elapsed_seconds":start.elapsed().as_secs_f64()})
                    );
                    measurements.push((capacity, steps, bytes));
                    drop((args, output));
                    assert_eq!(memory.used_bytes(), 0);
                    assert_eq!(memory.retained_snapshot_nodes(), 0);
                }
            }
            for steps in [0, 1, 3] {
                let bytes = |capacity| {
                    measurements
                        .iter()
                        .find(|(c, n, _)| *c == capacity && *n == steps)
                        .unwrap()
                        .2
                };
                // The extra reservation is allocated exactly once, not once per
                // tuple pack, projection, recursive visit or return transport.
                assert_eq!(
                    bytes(real) - bytes(16),
                    (real - 16) as usize * (width / 8) as usize
                );
            }
        }
    }
}

#[test]
fn checked_frames_use_fresh_booleans_preserve_aliases_and_recover() {
    let source = "+> sequence\nwalk(words<[u64]>, left<u64>, allowed<bool>, next<bool>, divisor<u64>) => (out<u64>) :=\n  enabled! := allowed\n  out := ((left > 0u64)? | true => walk(words, left - 1u64, next, next, divisor) | false => sequence/length(words) + 12u64 / divisor).\nwalk(values<[u64]>, 3u64, true, allowed<bool>, divisor<u64>)\n";
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(32 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for (allowed, divisor, expected) in [
            (true, 3, Some(7)),
            (false, 0, None),
            (true, 0, None),
            (true, 2, Some(9)),
        ] {
            let args = inputs(
                &artifact,
                &[
                    (
                        "values",
                        Data::Matrix(vec![Data::U64(41), Data::U64(43), Data::U64(47)].into()),
                    ),
                    ("allowed", Data::Bool(allowed)),
                    ("divisor", Data::U64(divisor)),
                ],
                &memory,
            );
            let aliases = args.clone();
            let prior = output.clone();
            let before = memory.used_bytes();
            let result = jit.evaluate_into(&args, &mut output, limits(), &memory);
            let resident = reference_result(&artifact, &args);
            if let Some(expected) = expected {
                result.unwrap();
                resident.unwrap();
                assert_eq!(
                    output
                        .last()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    Data::U64(expected)
                );
            } else {
                let error = format!("{:?}", result.unwrap_err());
                let resident = resident.unwrap_err();
                if !allowed {
                    assert!(error.contains("Integrity"), "{error}");
                    assert!(resident.contains("LexicalIntegrity"), "{resident}");
                } else {
                    assert!(!error.contains("Integrity"), "{error}");
                    assert!(!resident.contains("LexicalIntegrity"), "{resident}");
                }
                assert_eq!(memory.used_bytes(), before);
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
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn escaping_tuples_keep_canonical_storage_and_scalar_aggregates_stay_separate() {
    for source in [
        "(values<[u16]>, n<u64>)\n",
        "count(n<u64>) => <u64> | n => ((n > 0u64)? | true => count(n - 1u64) | false => n).\ncount(input<u64>)\n",
    ] {
        for artifact in artifacts(source).0 {
            let plan =
                lower_native_program_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
            assert_eq!(plan.facts().buffer_frames().elements(), 0);
        }
    }
}

#[test]
fn real_recursive_arguments_keep_allocation_below_either_payload() {
    let source = "+> sequence\nwalk(table<[u16]>, cache<[u32]>, n<u64>, allowed<bool>) => (out<u64>) :=\n  enabled! := allowed\n  out := ((n == 0u64)? | true => sequence/length(table) + sequence/length(cache) | false => walk(table, cache, n - 1u64, allowed)).\nwalk(table<[u16]>, cache<[u32]>, steps<u64>, allowed<bool>)\n";
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(128 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        let mut policy = limits();
        policy.execution.max_work = 10_000_000_000;
        policy.library.max_compute_work = Some(10_000_000_000);
        policy.library.max_comparison_work = Some(10_000_000_000);
        // Argument transport creates no cloned application payload.
        policy.library.max_cloned_bytes = Some(0);
        for allowed in [true, false, true] {
            let args = inputs(
                &artifact,
                &[
                    (
                        "table",
                        Data::Matrix(vec![Data::U16(0xf123); 262144].into()),
                    ),
                    (
                        "cache",
                        Data::Matrix(vec![Data::U32(0xf0123456); 1500513].into()),
                    ),
                    ("steps", Data::U64(1)),
                    ("allowed", Data::Bool(allowed)),
                ],
                &memory,
            );
            let prior = output.clone();
            let before = memory.used_bytes();
            let (result, requests, bytes) = allocation_probe::measured_with_bytes(|| {
                jit.evaluate_into(&args, &mut output, policy, &memory)
            });
            assert!(
                bytes < 262144 * 2,
                "{requests} allocations / {bytes} bytes: {result:?}"
            );
            if allowed {
                result.unwrap();
                assert_eq!(
                    output
                        .last()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    Data::U64(262144 + 1500513)
                );
            } else {
                assert!(format!("{:?}", result.unwrap_err()).contains("Integrity"));
                assert_eq!(memory.used_bytes(), before);
                assert!(output.iter().zip(&prior).all(|(v, p)| {
                    v.as_ref()
                        .unwrap()
                        .shares_frozen_storage(p.as_ref().unwrap())
                }));
            }
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn nested_repeated_calls_keep_published_buffers_immutable_and_recover_from_denial() {
    let grow = GROW.split("grow(sequence/with-capacity").next().unwrap();
    let source = format!(
        "{grow}original := sequence/with-capacity(0u64, 32u64)\nfirst := grow(original, 2u64)\nsecond := grow(first, 1u64)\ncleared := sequence/clear(second)\nthird := grow(cleared, 3u64)\nsequence/length(original) + sequence/length(first) + sequence/length(second) + sequence/length(third)\n"
    );
    let (forms, last) = super::artifacts_mode(&source, true);
    for artifact in forms {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(32 * 1024 * 1024);
        let args = inputs(&artifact, &[], &memory);
        let mut output = vec![None; artifact.outputs().len()];
        for denied in [false, true, false] {
            let before = memory.used_bytes();
            let prior = output.clone();
            let mut policy = limits();
            if denied {
                policy.execution.max_transport_bytes = 1;
            }
            let result = jit.evaluate_into(&args, &mut output, policy, &memory);
            if denied {
                assert!(result.is_err());
                assert_eq!(memory.used_bytes(), before);
                assert!(output.iter().zip(&prior).all(|(v, p)| {
                    v.as_ref()
                        .unwrap()
                        .shares_frozen_storage(p.as_ref().unwrap())
                }));
            } else {
                result.unwrap();
                let resident = reference_result(&artifact, &args).unwrap();
                super::equal(&artifact, &output, &resident);
                let matrices = output
                    .iter()
                    .filter_map(|v| match v.as_ref().unwrap().data() {
                        ValueData::Matrix(matrix) => Some(matrix),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                assert_eq!(matrices.len(), 5);
                assert!(
                    matrices
                        .iter()
                        .all(|m| m.unsigned_buffer_capacity() == Some(32))
                );
                assert_eq!(
                    output
                        .get(last)
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    Data::U64(8)
                );
            }
        }
        drop((args, output));
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn build_owned_image_rederives_private_frame_and_scalar_metadata() {
    for source in [
        GROW,
        super::scalar_state_qualification::ALL_KINDS,
        super::numeric_tail_qualification::WEIGHTED,
    ] {
        for artifact in artifacts(source).0 {
            let catalog = mech_stdlib::source_catalog();
            let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
            let bytes = jit.descriptor().encode_build_owned_image().unwrap();
            // SAFETY: this checked build produced these bytes. The restored
            // metadata remains confined to comparison and cannot execute code.
            let restored = unsafe {
                mech_gpu::native_artifact::NativeProgramDescriptor::decode_build_owned_image(
                    &bytes, &catalog,
                )
            }
            .unwrap();
            assert_eq!(restored.encode_build_owned_image().unwrap(), bytes);
        }
    }
}

#[test]
fn frame_shape_binding_admits_the_core_metadata_node_bound_before_construction() {
    use mech_engine::native_artifact::{
        NativeCollectionPrimitive as Primitive, NativeIterationAction as Action,
        NativeLibraryWorkBudget, prepare_native_program_library_call,
    };
    let source = "+> sequence\nwalk(words<[u64]>, left<u64>) => (out<u64>) :=\n  out := ((left > 0u64)? | true => walk(words, left - 1u64) | false => sequence/length(words)).\nwalk(values<[u64]>, steps<u64>)\n";
    for artifact in artifacts(source).0 {
        let catalog = mech_stdlib::source_catalog();
        let plan = lower_native_program_artifact(&artifact, &catalog).unwrap();
        let template = plan
            .blocks()
            .iter()
            .find_map(|b| match b.action() {
                Action::LibraryCall { template, .. }
                    if template.primitive() == Primitive::CompositePack(2) =>
                {
                    Some(template)
                }
                _ => None,
            })
            .unwrap();
        let call = prepare_native_program_library_call(&artifact, template, &catalog).unwrap();
        let mech_core::ResolvedOperationContract::Declared(contract) = template.contract() else {
            panic!()
        };
        let bound = mech_core::snapshot::CompositeSnapshotConstructor::binding_cost_for_inputs(
            contract.outputs[0].schema,
            contract.inputs.len(),
            |i| Some(contract.inputs[i].schema),
            artifact.schemas(),
        )
        .unwrap();
        let memory = ManagedMemoryBudget::new(32 * 1024 * 1024);
        let args = inputs(
            &artifact,
            &[
                ("values", Data::Matrix(vec![Data::U64(37)].into())),
                ("steps", Data::U64(1)),
            ],
            &memory,
        );
        let current = contract
            .inputs
            .iter()
            .map(|port| {
                Some(
                    args[artifact
                        .inputs()
                        .iter()
                        .position(|input| input.schema == port.schema)
                        .unwrap()]
                    .clone(),
                )
            })
            .collect::<Vec<_>>();
        let aliases = current.clone();
        let caps = limits().library;
        // One existing per-call allowance for rejection and one for retry.
        let work = NativeLibraryWorkBudget::new(
            caps.max_compute_work.unwrap().checked_mul(2).unwrap(),
            caps.max_comparison_work.unwrap().checked_mul(2).unwrap(),
            &memory,
        )
        .unwrap();
        let before = memory.used_bytes();
        let mut tight = limits().library;
        tight.max_retained_nodes = Some(bound.metadata_nodes + current.len() as u64 + 7);
        let error = call
            .admit_buffer_frame(&current, tight, &work, mech_core::ResourceDemand::default())
            .unwrap_err();
        assert!(format!("{error:?}").contains("RetainedNodes"), "{error:?}");
        assert_eq!(memory.used_bytes(), before);
        call.admit_buffer_frame(
            &current,
            limits().library,
            &work,
            mech_core::ResourceDemand::default(),
        )
        .unwrap();
        assert_eq!(memory.used_bytes(), before);
        assert!(current.iter().zip(&aliases).all(|(v, a)| {
            v.as_ref()
                .unwrap()
                .shares_frozen_storage(a.as_ref().unwrap())
        }));
        drop((args, current, aliases, work));
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn native_frame_scalar_leaves_preserve_exact_kinds_bits_and_current_shapes() {
    use mech_core::snapshot::F64Bits;
    for (kind, values) in [
        ("bool", vec![Data::Bool(false), Data::Bool(true)]),
        ("u8", vec![Data::U8(0), Data::U8(u8::MAX)]),
        ("u16", vec![Data::U16(0), Data::U16(u16::MAX)]),
        ("u32", vec![Data::U32(0), Data::U32(u32::MAX)]),
        ("u64", vec![Data::U64(0), Data::U64(u64::MAX)]),
        ("i64", vec![Data::I64(i64::MIN), Data::I64(i64::MAX)]),
        (
            "f64",
            vec![
                Data::F64(F64Bits::from_bits(0x8000_0000_0000_0000)),
                Data::F64(F64Bits::from_bits(0x7ff8_0000_0000_1234)),
            ],
        ),
    ] {
        let source = format!(
            "keep(words<[u8]>, left<u64>, value<{kind}>) => <{kind}> | (words, left, value) => ((left > 0u64)? | true => keep(words, left - 1u64, value) | false => value).\nkeep(values<[u8]>, steps<u64>, payload<{kind}>)\n"
        );
        for artifact in artifacts(&source).0 {
            let catalog = mech_stdlib::source_catalog();
            let plan = lower_native_program_artifact(&artifact, &catalog).unwrap();
            let frames = plan.facts().buffer_frames();
            assert!(frames.scalar_fields() > 0);
            assert!(frames.managed_fields() > 0);
            assert_eq!(
                frames.scalar_fields() + frames.managed_fields(),
                frames.elements()
            );
            let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
            let image = jit.descriptor().encode_build_owned_image().unwrap();
            // SAFETY: this build produced the unchanged image, confined here
            // to independently restored metadata comparison.
            let restored = unsafe {
                mech_gpu::native_artifact::NativeProgramDescriptor::decode_build_owned_image(
                    &image, &catalog,
                )
            }
            .unwrap();
            assert_eq!(restored.encode_build_owned_image().unwrap(), image);
            let memory = ManagedMemoryBudget::new(32 * 1024 * 1024);
            let mut output = vec![None; artifact.outputs().len()];
            for (length, payload) in [0, 5, 3].into_iter().zip(values.iter().cycle()) {
                let args = inputs(
                    &artifact,
                    &[
                        ("values", Data::Matrix(vec![Data::U8(19); length].into())),
                        ("steps", Data::U64(3)),
                        ("payload", payload.clone()),
                    ],
                    &memory,
                );
                let aliases = args.clone();
                jit.evaluate_into(&args, &mut output, limits(), &memory)
                    .unwrap();
                let resident = reference_result(&artifact, &args).unwrap();
                super::equal(&artifact, &output, &resident);
                assert_eq!(
                    output
                        .last()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    *payload
                );
                assert!(
                    args.iter()
                        .zip(&aliases)
                        .all(|(v, a)| v.shares_frozen_storage(a))
                );
            }
            drop(output);
            assert_eq!(memory.used_bytes(), 0);
            assert_eq!(memory.retained_snapshot_nodes(), 0);
        }
    }
}

#[test]
fn frame_word_admission_rejects_invalid_width_without_retaining_a_visit() {
    use mech_engine::native_artifact::{
        NativeCollectionPrimitive as Primitive, NativeIterationAction as Action,
        NativeLibraryWorkBudget, prepare_native_program_library_call,
    };
    for (kind, maximum) in [
        ("bool", 1),
        ("u8", u8::MAX as u64),
        ("u16", u16::MAX as u64),
        ("u32", u32::MAX as u64),
    ] {
        let source = format!(
            "keep(words<[u8]>, left<u64>, value<{kind}>) => <{kind}> | (words, left, value) => ((left > 0u64)? | true => keep(words, left - 1u64, value) | false => value).\nkeep(values<[u8]>, 2u64, payload<{kind}>)\n"
        );
        for artifact in artifacts(&source).0 {
            let catalog = mech_stdlib::source_catalog();
            let plan = lower_native_program_artifact(&artifact, &catalog).unwrap();
            let template = plan
                .blocks()
                .iter()
                .find_map(|b| match b.action() {
                    Action::LibraryCall { template, .. }
                        if template.primitive() == Primitive::CompositePack(3) =>
                    {
                        Some(template)
                    }
                    _ => None,
                })
                .unwrap();
            let call = prepare_native_program_library_call(&artifact, template, &catalog).unwrap();
            let memory = ManagedMemoryBudget::new(32 * 1024 * 1024);
            let args = inputs(
                &artifact,
                &[
                    ("values", Data::Matrix(vec![Data::U8(7); 5].into())),
                    (
                        "payload",
                        match kind {
                            "bool" => Data::Bool(true),
                            "u8" => Data::U8(1),
                            "u16" => Data::U16(1),
                            _ => Data::U32(1),
                        },
                    ),
                ],
                &memory,
            );
            let buffer = args
                .iter()
                .find(|v| matches!(v.data(), ValueData::Matrix(_)))
                .unwrap();
            let current = [Some(buffer.clone()), None, None];
            let work =
                NativeLibraryWorkBudget::new(10_000_000_000, 10_000_000_000, &memory).unwrap();
            for bits in [maximum, maximum + 1, 0] {
                let before = memory.used_bytes();
                let nodes = memory.retained_snapshot_nodes();
                let result = call.admit_buffer_frame_words(
                    &current,
                    &[0, 2, bits],
                    limits().library,
                    &work,
                    mech_core::ResourceDemand::default(),
                );
                assert_eq!(result.is_ok(), bits <= maximum, "{kind}: {result:?}");
                assert_eq!(memory.used_bytes(), before);
                assert_eq!(memory.retained_snapshot_nodes(), nodes);
            }
            drop((current, args, work));
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}

#[test]
fn ordinary_frame_admission_keeps_exact_work_prefixes_and_current_owners() {
    use mech_engine::native_artifact::{
        NativeCollectionPrimitive as Primitive, NativeIterationAction as Action,
        NativeLibraryWorkBudget, prepare_native_program_library_call,
    };
    let source = "+> sequence\nwalk(a<[u16]>, b<[u32]>, n<u64>, flag<bool>) => <u64> | (a, b, n, flag) => ((n > 0u64)? | true => walk(a, b, n - 1u64, flag) | false => ((flag)? | true => sequence/length(a) | false => sequence/length(b))).\nwalk(left<[u16]>, right<[u32]>, steps<u64>, take<bool>)\n";
    for (form, artifact) in ["original", "decoded"].into_iter().zip(artifacts(source).0) {
        let catalog = mech_stdlib::source_catalog();
        let plan = lower_native_program_artifact(&artifact, &catalog).unwrap();
        let template = plan
            .blocks()
            .iter()
            .find_map(|block| match block.action() {
                Action::LibraryCall { template, .. }
                    if template.primitive() == Primitive::CompositePack(4) =>
                {
                    Some(template)
                }
                _ => None,
            })
            .unwrap();
        let call = prepare_native_program_library_call(&artifact, template, &catalog).unwrap();
        let mech_core::ResolvedOperationContract::Declared(contract) = template.contract() else {
            panic!()
        };
        let memory = ManagedMemoryBudget::new(128 * 1024 * 1024);
        let mut full_work = None;
        for (a, b) in [(0, 0), (7, 13), (262144, 1500513), (2, 1)] {
            let args = inputs(
                &artifact,
                &[
                    ("left", Data::Matrix(vec![Data::U16(17); a].into())),
                    ("right", Data::Matrix(vec![Data::U32(29); b].into())),
                    ("steps", Data::U64(3)),
                    ("take", Data::Bool(true)),
                ],
                &memory,
            );
            let current = contract
                .inputs
                .iter()
                .map(|port| {
                    let value = args
                        .iter()
                        .find(|value| value.schema() == port.schema)
                        .unwrap();
                    if matches!(value.data(), ValueData::Matrix(_)) {
                        Some(value.clone())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            let aliases = current.clone();
            let before = (memory.used_bytes(), memory.retained_snapshot_nodes());
            let observe = |label: &str, words: &[u64], policy: mech_core::MemoryBudgetLimits| {
                let work =
                    NativeLibraryWorkBudget::new(10_000_000_000, 10_000_000_000, &memory).unwrap();
                let before = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let result = call.admit_buffer_frame_words(
                    &current,
                    words,
                    policy,
                    &work,
                    Default::default(),
                );
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    before
                );
                let used = work.used();
                println!(
                    "{}",
                    serde_json::json!({"witness":"ordinary-frame-admission", "form":form,
                    "lengths":[a,b], "case":label, "work":used, "error":result.as_ref().err().map(|e| format!("{e:?}"))})
                );
                (result, used)
            };
            let (result, success) = observe("success", &[0, 0, 3, 1], limits().library);
            result.unwrap();
            assert_eq!(success.1, 0);
            assert_eq!(*full_work.get_or_insert(success.0), success.0);
            let (result, invalid) = observe("invalid-bool", &[0, 0, 3, 2], limits().library);
            assert!(result.is_err());
            assert_eq!(invalid, (success.0 - 192, 0));
            let (result, invalid) = observe("managed-bits", &[1, 0, 3, 1], limits().library);
            assert!(result.is_err());
            // Two packed matrices cost 320 work each; two scalar words cost 192 each.
            assert_eq!(invalid, (success.0 - 1024, 0));
            for withheld in [1, 63, 64, 65, 127, 128, 191, 192, 193, 1024] {
                let mut policy = limits().library;
                policy.max_compute_work = Some(success.0 - withheld);
                let (result, used) =
                    observe(&format!("withhold-{withheld}"), &[0, 0, 3, 1], policy);
                assert!(result.is_err(), "withheld {withheld}: {used:?}");
                assert_eq!(used, (success.0 - withheld.div_ceil(64) * 64, 0));
                let error = format!("{:?}", result.unwrap_err());
                assert!(
                    error.contains(&format!("required: {}", used.0 + 64)),
                    "{error}"
                );
                assert!(
                    error.contains(&format!("limit: {}", success.0 - withheld)),
                    "{error}"
                );
            }
            let mut policy = limits().library;
            policy.max_output_elements = Some(3);
            let (result, used) = observe("output-limit", &[0, 0, 3, 1], policy);
            assert!(result.is_err());
            assert_eq!(used, success);
            policy.max_compute_work = Some(success.0 - 1);
            let (result, used) = observe("compute-before-output", &[0, 0, 3, 1], policy);
            assert!(format!("{:?}", result.unwrap_err()).contains("ComputeWork"));
            assert_eq!(used, (success.0 - 64, 0));
            let (result, used) = observe("bad-scalar-before-output", &[0, 0, 3, 2], policy);
            assert_eq!(format!("{:?}", result.unwrap_err()), "InvalidInput");
            assert_eq!(used, (success.0 - 192, 0));
            policy.max_compute_work = Some(success.0 - 193);
            let (result, used) = observe("compute-before-bad-scalar", &[0, 0, 3, 2], policy);
            assert!(format!("{:?}", result.unwrap_err()).contains("ComputeWork"));
            assert_eq!(used, (success.0 - 256, 0));
            observe("retry", &[0, 0, 5, 0], limits().library).0.unwrap();
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                before
            );
            assert!(
                current
                    .iter()
                    .zip(&aliases)
                    .all(|(value, alias)| match (value, alias) {
                        (Some(value), Some(alias)) => value.shares_frozen_storage(alias),
                        (None, None) => true,
                        _ => false,
                    })
            );
            drop((current, aliases, args));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn restored_frame_type_slots_reject_misalignment_keys_and_definitions() {
    for source in [
        GROW,
        super::scalar_state_qualification::ALL_KINDS,
        super::numeric_tail_qualification::WEIGHTED,
    ] {
        for artifact in artifacts(source).0 {
            let catalog = mech_stdlib::source_catalog();
            let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
            let original: serde_json::Value =
                serde_json::from_slice(&jit.descriptor().encode_build_owned_image().unwrap())
                    .unwrap();
            let types = original["descriptor"]["types"].as_array().unwrap();
            let first = 0;
            let other = types
                .iter()
                .position(|slot| slot["schema"] != types[first]["schema"])
                .unwrap();
            for corruption in ["slots", "key", "definition"] {
                let mut changed = original.clone();
                match corruption {
                    // Both entries remain individually valid for their schemas.
                    // The missing relationship is their exact position in plan.
                    "slots" => changed["descriptor"]["types"]
                        .as_array_mut()
                        .unwrap()
                        .swap(first, other),
                    "key" => {
                        changed["descriptor"]["types"][first]["key"] = types[other]["key"].clone()
                    }
                    "definition" => {
                        let values = original["descriptor"]["plan"]["values"].as_array().unwrap();
                        let other = values
                            .iter()
                            .position(|ty| ty["definition"] != values[first]["definition"])
                            .unwrap();
                        changed["descriptor"]["plan"]["values"][first]["definition"] =
                            values[other]["definition"].clone();
                    }
                    _ => unreachable!(),
                }
                let bytes = serde_json::to_vec(&changed).unwrap();
                // SAFETY: this deliberately corrupt checked-build image stays
                // confined to rejection. No restored entry is executed.
                let result = unsafe {
                    mech_gpu::native_artifact::NativeProgramDescriptor::decode_build_owned_image(
                        &bytes, &catalog,
                    )
                };
                assert!(result.is_err(), "corruption accepted: {corruption}");
            }
        }
    }
}

#[test]
fn restored_numeric_leaf_rejects_stale_views_offsets_prices_and_factory_associations() {
    for artifact in [
        super::numeric_tail_qualification::WEIGHTED,
        super::numeric_write_qualification::WRITE,
        super::numeric_result_qualification::GENERIC,
    ]
    .into_iter()
    .flat_map(|source| artifacts(source).0)
    {
        let catalog = mech_stdlib::source_catalog();
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let original: serde_json::Value =
            serde_json::from_slice(&jit.descriptor().encode_build_owned_image().unwrap()).unwrap();
        let root = "/descriptor/closed_tails/regions/0";
        let mut suffixes = vec![
            "/numeric/arrays/0/field",
            "/numeric/arrays/0/width",
            "/numeric/arrays/0/witness",
            "/numeric/arrays/0/writable",
            "/numeric/result",
            "/numeric/words",
            "/numeric/scalar_words",
            "/words",
            "/compute",
            "/comparison",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
        if original
            .pointer(&format!("{root}/numeric/result/Frame"))
            .is_some()
        {
            suffixes.extend(
                [
                    "/numeric/result/Frame/value",
                    "/numeric/result/Frame/ordinal",
                    "/numeric/result/Frame/array",
                ]
                .map(str::to_owned),
            );
            let proof = original.pointer(&format!("{root}/numeric")).unwrap();
            let value = &proof["result"]["Frame"]["value"];
            let ordinal = proof["result"]["Frame"]["ordinal"].as_u64().unwrap();
            let at = proof["frames"]
                .as_array()
                .unwrap()
                .iter()
                .position(|f| &f["value"] == value)
                .unwrap();
            let scalar = proof["frames"][at]["fields"]
                .as_array()
                .unwrap()
                .iter()
                .position(|f| !f.is_null())
                .unwrap();
            suffixes.extend([
                format!("/numeric/frames/{at}/value"),
                format!("/numeric/frames/{at}/fields/{scalar}"),
                format!("/numeric/frames/{at}/arrays/{ordinal}"),
            ]);
        }
        for suffix in suffixes {
            let mut changed = original.clone();
            let field = changed.pointer_mut(&format!("{root}{suffix}")).unwrap();
            *field = match field {
                serde_json::Value::Bool(value) => serde_json::json!(!*value),
                serde_json::Value::Null => serde_json::json!(0),
                serde_json::Value::Number(_) => serde_json::json!(field.as_u64().unwrap() + 1),
                serde_json::Value::String(_) => serde_json::json!({"Array":{"array":0}}),
                _ => serde_json::json!("Scalar"),
            };
            let bytes = serde_json::to_vec(&changed).unwrap();
            // SAFETY: corrupt cached metadata is confined to rejection; no
            // restored descriptor or entry from this diagnostic may execute.
            assert!(
                unsafe {
                    mech_gpu::native_artifact::NativeProgramDescriptor::decode_build_owned_image(
                        &bytes, &catalog,
                    )
                }
                .is_err(),
                "accepted {suffix}"
            );
        }
        let mut changed = original.clone();
        *changed.pointer_mut(&format!("{root}/numeric")).unwrap() = serde_json::Value::Null;
        let bytes = serde_json::to_vec(&changed).unwrap();
        // SAFETY: removing the proof stays confined to rejection.
        assert!(
            unsafe {
                mech_gpu::native_artifact::NativeProgramDescriptor::decode_build_owned_image(
                    &bytes, &catalog,
                )
            }
            .is_err()
        );
        let witness = original
            .pointer(&format!("{root}/numeric/arrays/0/witness"))
            .unwrap()
            .as_u64()
            .unwrap() as usize;
        let calls = original["descriptor"]["calls"].as_array().unwrap();
        let other = calls
            .iter()
            .enumerate()
            .find(|(site, call)| *site != witness && !call.is_null() && *call != &calls[witness])
            .unwrap()
            .0;
        let mut changed = original.clone();
        changed["descriptor"]["calls"]
            .as_array_mut()
            .unwrap()
            .swap(witness, other);
        let bytes = serde_json::to_vec(&changed).unwrap();
        // SAFETY: independently valid but misassociated factories may not escape.
        assert!(
            unsafe {
                mech_gpu::native_artifact::NativeProgramDescriptor::decode_build_owned_image(
                    &bytes, &catalog,
                )
            }
            .is_err()
        );
    }
}
