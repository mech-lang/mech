//! Runtime-shaped unsigned buffers remain ordinary tuple components.
use super::{artifacts, equal, inputs, limits, reference};
use mech_core::{ManagedMemoryBudget, ValueDataDraft as Data};
use mech_gpu::native_artifact::JitProgramArtifact;

#[test]
fn dynamic_word_buffers_pack_into_tuples() {
    for width in [8, 16, 32, 64] {
        let source = format!("(values<[u{width}]>, count<u64>)\n");
        for artifact in artifacts(&source).0 {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let mut output = vec![None; artifact.outputs().len()];
            for (rows, columns) in [(1, 0), (2, 3), (3, 1), (1, 7), (0, 2)] {
                let n = rows * columns;
                let values = (0..n)
                    .map(|n| match width {
                        8 => Data::U8(n as u8),
                        16 => Data::U16(n as u16),
                        32 => Data::U32(n as u32),
                        64 => Data::U64(n),
                        _ => unreachable!(),
                    })
                    .collect();
                let mut args = inputs(
                    &artifact,
                    &[("values", Data::Matrix(values)), ("count", Data::U64(n))],
                    &memory,
                );
                let index = artifact
                    .inputs()
                    .iter()
                    .position(|input| {
                        mech_engine::decode_source_input_name(&input.name).unwrap() == "values"
                    })
                    .unwrap();
                let schema = artifact
                    .schemas()
                    .get(artifact.inputs()[index].schema)
                    .unwrap();
                let shape =
                    mech_core::shape_for_resolved_extents(schema, &[rows, columns]).unwrap();
                let value = mech_core::ValueDraft {
                    schema: artifact.inputs()[index].schema,
                    shape_values: shape.parameter_values().to_vec().into_boxed_slice(),
                    data: args[index].canonical_data_draft().unwrap(),
                }
                .finalize(&mech_core::snapshot::SnapshotValidationContext::new(
                    artifact.schemas(),
                ))
                .unwrap();
                let mut claim = memory
                    .reserve_capacity(value.memory_budget_admission_bytes(&memory).unwrap())
                    .unwrap();
                args[index] = value.into_memory_budget(&mut claim).unwrap();
                let ordinary = reference(&artifact, &args);
                jit.evaluate_into(&args, &mut output, limits(), &memory)
                    .unwrap();
                equal(&artifact, &output, &ordinary);
            }
            drop(output);
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}

#[test]
fn dynamic_tuple_returns_keep_independent_current_buffer_shapes() {
    let source = "pair(xs<[u16]>, ys<[u16]>) => (out<([u16],[u16])>) :=\n  out := (xs, ys).\npair(left<[u16]>, right<[u16]>)\n";
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for (left, right) in [(0, 3), (4, 1), (2, 7), (2, 7), (1, 0), (0, 0)] {
            let a = Data::Matrix((0..left).map(Data::U16).collect());
            let b = Data::Matrix((0..right).map(|i| Data::U16(100 + i)).collect());
            let args = inputs(
                &artifact,
                &[("left", a.clone()), ("right", b.clone())],
                &memory,
            );
            let ordinary = reference(&artifact, &args);
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            equal(&artifact, &output, &ordinary);
            assert_eq!(
                output
                    .last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                Data::Tuple(vec![a, b].into())
            );
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn deflate_node_helpers_can_return_their_dynamic_arrays_together() {
    let source = format!(
        "{}\n(initial-nodes(count<u64>), rebase-nodes(nodes<[u16]>), node-index(base<u64>, node<u16>))\n",
        include_str!("../fixtures/native/deflate-matchfinder.mec")
    );
    for artifact in artifacts(&source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for count in [0, 7, 1, 2, 0] {
            let args = inputs(
                &artifact,
                &[
                    ("count", Data::U64(count)),
                    (
                        "nodes",
                        Data::Matrix(vec![Data::U16(0), Data::U16(32768), Data::U16(65535)].into()),
                    ),
                    ("base", Data::U64(100000)),
                    ("node", Data::U16(32769)),
                ],
                &memory,
            );
            let ordinary = reference(&artifact, &args);
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            equal(&artifact, &output, &ordinary);
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
                        Data::Matrix(vec![Data::U16(0); count as usize].into()),
                        Data::Matrix(vec![Data::U16(0), Data::U16(0), Data::U16(32767)].into()),
                        Data::U64(100001),
                    ]
                    .into()
                )
            );
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn dynamic_buffer_tuple_failure_preserves_publication_and_memory_before_retry() {
    let source = "(left<[u16]>, right<[u16]>)\n";
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for (size, fail) in [(2, false), (9, true), (0, false), (3, false)] {
            let args = inputs(
                &artifact,
                &[
                    ("left", Data::Matrix(vec![Data::U16(17); size].into())),
                    ("right", Data::Matrix(vec![Data::U16(29); size + 1].into())),
                ],
                &memory,
            );
            let mut policy = limits();
            if fail {
                let prior = output
                    .iter()
                    .map(|value| value.clone().unwrap())
                    .collect::<Vec<_>>();
                let before = memory.used_bytes();
                policy.library.max_cloned_bytes = Some(0);
                let error = jit
                    .evaluate_into(&args, &mut output, policy, &memory)
                    .unwrap_err();
                assert!(format!("{error:?}").contains("Limit"), "{error:?}");
                equal(&artifact, &output, &prior);
                assert_eq!(memory.used_bytes(), before);
            } else {
                let ordinary = reference(&artifact, &args);
                jit.evaluate_into(&args, &mut output, policy, &memory)
                    .unwrap();
                equal(&artifact, &output, &ordinary);
            }
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn nested_dynamic_buffer_tuple_and_record_components_keep_separate_shapes() {
    let source = "{values: bytes<[u8]>, detail: (words<[u16]>, 9u64)}\n";
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for (bytes, words) in [(0, 5), (7, 0), (2, 3), (1, 1), (0, 0)] {
            let args = inputs(
                &artifact,
                &[
                    ("bytes", Data::Matrix(vec![Data::U8(255); bytes].into())),
                    ("words", Data::Matrix(vec![Data::U16(65535); words].into())),
                ],
                &memory,
            );
            let ordinary = reference(&artifact, &args);
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            equal(&artifact, &output, &ordinary);
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn dynamic_buffer_pack_rebinds_current_shapes_between_visits_in_one_invocation() {
    let source = "make(n<u64>) => (out<u64>) :=\n  xs := [0u16 | i <- 0u64..=n, i < n]\n  packed := (xs, 7u64)\n  out := n.\n[make(n) | n <- counts<[u64]>]\n";
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert!(jit.descriptor().plan().blocks().iter().any(|block|
            block.iteration().is_some() && matches!(block.action(),
                mech_engine::native_artifact::NativeIterationAction::LibraryCall { template, .. }
                    if matches!(template.primitive(), mech_engine::native_artifact::NativeCollectionPrimitive::CompositePack(_)))));
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        let counts = Data::Matrix(
            [0, 4, 4, 1, 0, 7, 7, 0]
                .into_iter()
                .map(Data::U64)
                .collect(),
        );
        let args = inputs(&artifact, &[("counts", counts.clone())], &memory);
        let ordinary = reference(&artifact, &args);
        jit.evaluate_into(&args, &mut output, limits(), &memory)
            .unwrap();
        equal(&artifact, &output, &ordinary);
        assert_eq!(
            output
                .last()
                .unwrap()
                .as_ref()
                .unwrap()
                .canonical_data_draft()
                .unwrap(),
            counts
        );
        drop(output);
        drop(args);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn dynamic_buffer_tuple_images_require_the_exact_maintained_pack_factory() {
    use mech_engine::native_artifact::{
        NativeArtifactLoweringError, lower_native_program_artifact,
    };
    use mech_gpu::native_artifact::NativeProgramDescriptor;
    let source = "(values<[u16]>, 7u64)\n";
    let catalog = mech_stdlib::source_catalog();
    let mut builder = mech_core::FunctionCatalogBuilder::new();
    builder
        .insert_resident_factory(
            ["core"],
            "composite-pack",
            mech_core::ImplementationMemoryClass::CanonicalFinalize,
            |_| panic!("replacement pack factory must never execute during native validation"),
        )
        .unwrap();
    for (key, entry) in catalog.resident_factories() {
        if key.module_path.as_ref() != ["core"] || key.operation_name != "composite-pack" {
            builder
                .insert_resident_factory_entry(entry.clone())
                .unwrap();
        }
    }
    let replacement = builder.build().unwrap();
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let bytes = jit.descriptor().encode_build_owned_image().unwrap();
        // SAFETY: this unchanged descriptor image comes from the exact current
        // checked build; no unrelated executable is paired with it.
        let restored =
            unsafe { NativeProgramDescriptor::decode_build_owned_image(&bytes, &catalog) }.unwrap();
        assert_eq!(restored.encode_build_owned_image().unwrap(), bytes);
        assert!(matches!(
            lower_native_program_artifact(&artifact, &replacement),
            Err(NativeArtifactLoweringError::MissingSemanticWitness { .. })
        ));
        // SAFETY: the same unchanged local build image is used to check that a
        // catalog with a forged same-name implementation is rejected.
        assert!(
            unsafe { NativeProgramDescriptor::decode_build_owned_image(&bytes, &replacement) }
                .is_err()
        );
    }
}
