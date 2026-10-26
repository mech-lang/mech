//! Live buffer dimensions survive destructuring, tuple reads, and state updates.
use super::{artifacts, equal, inputs, limits, reference, reference_result};
use mech_core::{ManagedMemoryBudget, ValueDataDraft as Data};
use mech_gpu::native_artifact::JitProgramArtifact;

#[test]
fn dynamic_array_tuple_destructuring_preserves_independent_parameter_arenas() {
    let source = "(left, right, count) := (a<[u16]>, b<[u32]>, n<u64>)\n(right, left, count)\n";
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for ((ar, ac), (br, bc)) in [
            ((1, 0), (2, 3)),
            ((2, 3), (3, 2)),
            ((0, 7), (1, 3)),
            ((1, 2), (2, 1)),
            ((0, 0), (1, 0)),
        ] {
            let a = Data::Matrix(vec![Data::U16(65535); (ar * ac) as usize].into());
            let b = Data::Matrix(vec![Data::U32(u32::MAX); (br * bc) as usize].into());
            let mut args = inputs(
                &artifact,
                &[("a", a.clone()), ("b", b.clone()), ("n", Data::U64(19))],
                &memory,
            );
            for (name, rows, columns) in [("a", ar, ac), ("b", br, bc)] {
                let index = artifact
                    .inputs()
                    .iter()
                    .position(|input| {
                        mech_engine::decode_source_input_name(&input.name).unwrap() == name
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
                    shape_values: shape.parameter_values().to_vec().into(),
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
            }
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
                Data::Tuple(vec![b, a, Data::U64(19)].into())
            );
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn dynamic_array_constant_tuple_reads_preserve_nested_shapes() {
    let source = "values := (a<[u8]>, (b<[u16]>, n<u64>))\nselected := values.2\n(selected.1, values.1, selected.2)\n";
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for (a, b) in [(0, 7), (4, 2), (1, 0), (0, 0)] {
            let a = Data::Matrix(vec![Data::U8(255); a].into());
            let b = Data::Matrix(vec![Data::U16(65535); b].into());
            let args = inputs(
                &artifact,
                &[("a", a.clone()), ("b", b.clone()), ("n", Data::U64(23))],
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
                Data::Tuple(vec![b, a, Data::U64(23)].into())
            );
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn record_fields_preserve_live_buffer_shapes_and_recover_after_later_failure() {
    for source in [
        "values := {left: a<[u16]>, right: b<[u32]>, count: n<u64>}\nleft := values.left\nright := values.right\ncount := values.count\n(right, left, count / divisor<u64>)\n",
        "project(a<[u16]>, b<[u32]>, n<u64>, divisor<u64>) => (out<([u32], [u16], u64)>) :=\n  values := {left: a, right: b, count: n}\n  left := values.left\n  right := values.right\n  count := values.count\n  out := (right, left, count / divisor).\nproject(a<[u16]>, b<[u32]>, n<u64>, divisor<u64>)\n",
    ] {
        for artifact in artifacts(source).0 {
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            // Establish resident support before testing native lowering.
            let args = inputs(
                &artifact,
                &[
                    ("a", Data::Matrix(vec![Data::U16(7); 2].into())),
                    ("b", Data::Matrix(vec![Data::U32(19); 3].into())),
                    ("n", Data::U64(23)),
                    ("divisor", Data::U64(1)),
                ],
                &memory,
            );
            assert_eq!(
                reference(&artifact, &args)
                    .last()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                Data::Tuple(
                    vec![
                        Data::Matrix(vec![Data::U32(19); 3].into()),
                        Data::Matrix(vec![Data::U16(7); 2].into()),
                        Data::U64(23)
                    ]
                    .into()
                )
            );
            drop(args);
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let mut output = vec![None; artifact.outputs().len()];
            for (a_len, b_len, divisor) in [(2, 3, 1), (0, 4, 2), (5, 1, 0), (3, 0, 1)] {
                let a = Data::Matrix(vec![Data::U16(u16::MAX); a_len].into());
                let b = Data::Matrix(vec![Data::U32(u32::MAX); b_len].into());
                let args = inputs(
                    &artifact,
                    &[
                        ("a", a.clone()),
                        ("b", b.clone()),
                        ("n", Data::U64(24)),
                        ("divisor", Data::U64(divisor)),
                    ],
                    &memory,
                );
                let aliases = args.clone();
                let before = memory.used_bytes();
                if divisor == 0 {
                    let prior = output
                        .iter()
                        .map(|value| value.clone().unwrap())
                        .collect::<Vec<_>>();
                    assert!(reference_result(&artifact, &args).is_err());
                    let error = jit
                        .evaluate_into(&args, &mut output, limits(), &memory)
                        .unwrap_err();
                    assert!(matches!(error, mech_gpu::native_artifact::NativeProgramExecutionError::Control(
                        mech_gpu::native_artifact::NativeIterationExecutionError::Arithmetic { .. })), "{error:?}");
                    equal(&artifact, &output, &prior);
                    assert_eq!(memory.used_bytes(), before);
                } else {
                    jit.evaluate_into(&args, &mut output, limits(), &memory)
                        .unwrap();
                    equal(&artifact, &output, &reference(&artifact, &args));
                    assert_eq!(
                        output
                            .last()
                            .unwrap()
                            .as_ref()
                            .unwrap()
                            .canonical_data_draft()
                            .unwrap(),
                        Data::Tuple(vec![b, a, Data::U64(24 / divisor)].into())
                    );
                }
                assert!(
                    args.iter()
                        .zip(&aliases)
                        .all(|(value, alias)| value.shares_frozen_storage(alias))
                );
            }
            drop(output);
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}

#[test]
fn record_field_binding_uses_the_current_selector_and_preserves_factory_evidence() {
    use mech_engine::native_artifact::{
        NativeArtifactLoweringError, NativeCollectionPrimitive as P, NativeIterationAction as A,
        NativeLibraryWorkBudget, NativePreparedLibraryCallCache, bind_native_program_library_call,
        lower_native_program_artifact, prepare_native_program_library_call,
    };
    use mech_gpu::native_artifact::NativeProgramDescriptor;
    let source =
        "values := {left: a<[u16]:1,3>, right: b<[u16]:1,3>, count: n<u64>}\nvalues.left\n";
    let catalog = mech_stdlib::source_catalog();
    let mut replacement = mech_core::FunctionCatalogBuilder::new();
    replacement
        .insert_resident_factory(
            ["access"],
            "column",
            mech_core::ImplementationMemoryClass::CanonicalFinalize,
            |_| panic!("lowering must reject replacement before binding"),
        )
        .unwrap();
    for (key, entry) in catalog.resident_factories() {
        if key.module_path.as_ref() != ["access"] || key.operation_name != "column" {
            replacement
                .insert_resident_factory_entry(entry.clone())
                .unwrap();
        }
    }
    let replacement = replacement.build().unwrap();
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let bytes = jit.descriptor().encode_build_owned_image().unwrap();
        // SAFETY: this is the unchanged image from this checked compilation.
        let restored =
            unsafe { NativeProgramDescriptor::decode_build_owned_image(&bytes, &catalog) }.unwrap();
        assert_eq!(restored.encode_build_owned_image().unwrap(), bytes);
        assert!(matches!(
            lower_native_program_artifact(&artifact, &replacement),
            Err(NativeArtifactLoweringError::MissingSemanticWitness { .. })
        ));
        let template = jit
            .descriptor()
            .plan()
            .blocks()
            .iter()
            .find_map(|block| match block.action() {
                A::LibraryCall { template, .. } if template.primitive() == P::RecordFieldAccess => {
                    Some(template)
                }
                _ => None,
            })
            .unwrap();
        let prepared = prepare_native_program_library_call(&artifact, template, &catalog).unwrap();
        let mech_core::ResolvedOperationContract::Declared(contract) = template.contract() else {
            panic!()
        };
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let record = mech_core::ValueDraft {
            schema: contract.inputs[0].schema,
            shape_values: Box::new([]),
            data: Data::Record(
                [
                    ("left", Data::Matrix(vec![Data::U16(7); 3].into())),
                    ("right", Data::Matrix(vec![Data::U16(19); 3].into())),
                    ("count", Data::U64(23)),
                ]
                .into_iter()
                .map(|(name, value)| mech_core::snapshot::NamedValueDraft {
                    name: name.into(),
                    value,
                })
                .collect(),
            ),
        }
        .finalize(&mech_core::snapshot::SnapshotValidationContext::new(
            artifact.schemas(),
        ))
        .unwrap();
        let mut claim = memory
            .reserve_capacity(record.memory_budget_admission_bytes(&memory).unwrap())
            .unwrap();
        let record = record.into_memory_budget(&mut claim).unwrap();
        drop(claim);
        let work = NativeLibraryWorkBudget::new(1_000_000_000, 1_000_000_000, &memory).unwrap();
        let mut cache = NativePreparedLibraryCallCache::default();
        let mut output = None;
        let output_parameters = vec![0; template.output().dimension_parameters().len()];
        let bound = bind_native_program_library_call(
            &artifact,
            template,
            &catalog,
            &[record.shape().parameter_values(), &[]],
            &output_parameters,
            limits().library,
        )
        .unwrap();
        for (index, field) in ["left", "left", "right", "missing", "left"]
            .into_iter()
            .enumerate()
        {
            let selector = mech_core::ValueDraft {
                schema: contract.inputs[1].schema,
                shape_values: Box::new([]),
                data: Data::Id(mech_core::hash_str(field)),
            }
            .finalize(&mech_core::snapshot::SnapshotValidationContext::new(
                artifact.schemas(),
            ))
            .unwrap();
            let mut claim = memory
                .reserve_capacity(selector.memory_budget_admission_bytes(&memory).unwrap())
                .unwrap();
            let selector = selector.into_memory_budget(&mut claim).unwrap();
            drop(claim);
            let values = [Some(record.clone()), Some(selector)];
            if index == 0 {
                bound.invoke(&values, &mut output, &work).unwrap();
            }
            let prior = output.clone();
            let result = prepared.invoke_current_cached(
                &mut cache,
                &values,
                &mut output,
                limits().library,
                &work,
            );
            if field == "missing" {
                assert!(result.is_err());
                assert!(
                    output
                        .as_ref()
                        .unwrap()
                        .shares_frozen_storage(prior.as_ref().unwrap())
                );
            } else {
                result.unwrap();
                assert_eq!(
                    output.as_ref().unwrap().canonical_data_draft().unwrap(),
                    Data::Matrix(vec![Data::U16(if field == "left" { 7 } else { 19 }); 3].into())
                );
            }
        }
        drop((cache, output, record, work));
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[path = "canonical_program_ordinary_recursion.rs"]
mod recursion;

#[path = "canonical_program_dynamic_projection_admission.rs"]
mod admission;
