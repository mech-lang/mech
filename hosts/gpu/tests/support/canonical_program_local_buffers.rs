//! Transition-local mutation is pure versioned data, including loop buffers.
use super::{artifacts, equal, inputs, limits, reference, reference_result};
use mech_core::{ManagedMemoryBudget, ValueDataDraft as Data};
use mech_gpu::native_artifact::JitProgramArtifact;

#[test]
fn transition_local_scalar_versions_are_recomputed_on_each_visit() {
    let source = "#Sum(n<u64>) => <u64>\n  | :Loop(left<u64>, total<u64>)\n  | :Done(total<u64>).\n#Sum(n) -> :Loop(n, 0u64)\n  :Loop(left, total)\n    | left > 0u64 -> {\n        ~v := left\n        v += 1u64\n        first := v\n        v = v + 2u64\n      } -> :Loop(left - 1u64, total + first + v)\n    | * -> :Done(total)\n  :Done(total) => total.\n#Sum(input<u64>)\n";
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for n in [0u64, 1, 4, 2, 8, 0] {
            let args = inputs(&artifact, &[("input", Data::U64(n))], &memory);
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
                Data::U64(n * n + 5 * n)
            );
        }
    }
}
fn word(width: u32, value: u64) -> Data {
    match width {
        8 => Data::U8(value.try_into().unwrap()),
        16 => Data::U16(value.try_into().unwrap()),
        32 => Data::U32(value.try_into().unwrap()),
        64 => Data::U64(value),
        _ => unreachable!(),
    }
}
fn buffer(width: u32, values: &[u64]) -> Data {
    Data::Matrix(values.iter().map(|value| word(width, *value)).collect())
}
#[test]
fn local_word_buffer_versions_preserve_inputs_and_recover_after_bounds_errors() {
    for width in [8, 16, 32, 64] {
        let source = format!(
            "<buffer> := <[u{width}]:1,4>\n#Replace(xs<buffer>, at<u64>, value<u{width}>) => <(buffer,buffer,buffer)>\n  | :Start(xs<buffer>, at<u64>, value<u{width}>)\n  | :Done(before<buffer>, first<buffer>, last<buffer>).\n#Replace(xs, at, value) -> :Start(xs, at, value)\n  :Start(xs, at, value) -> {{\n      ~scratch := xs\n      before := scratch\n      scratch[at] = value\n      first := scratch\n      scratch[1u64] = 0u{width}\n    }} -> :Done(before, first, scratch)\n  :Done(before, first, last) => (before, first, last).\n#Replace(input<buffer>, position<u64>, replacement<u{width}>)\n"
        );
        for artifact in artifacts(&source).0 {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let mut output = vec![None; artifact.outputs().len()];
            for (at, replacement) in [
                (1u64, 127),
                (4, 4),
                (2, 2),
                (0, 127),
                (5, 127),
                (3, 127),
                (1, 1),
            ] {
                let initial = [1, 2, 3, 4];
                let args = inputs(
                    &artifact,
                    &[
                        ("input", buffer(width, &initial)),
                        ("position", Data::U64(at)),
                        ("replacement", word(width, replacement)),
                    ],
                    &memory,
                );
                if (1..=4).contains(&at) {
                    let ordinary = reference(&artifact, &args);
                    jit.evaluate_into(&args, &mut output, limits(), &memory)
                        .unwrap();
                    equal(&artifact, &output, &ordinary);
                    let mut first = initial;
                    first[at as usize - 1] = replacement;
                    let mut last = first;
                    last[0] = 0;
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
                                buffer(width, &initial),
                                buffer(width, &first),
                                buffer(width, &last)
                            ]
                            .into()
                        )
                    );
                } else {
                    assert!(reference_result(&artifact, &args).is_err());
                    let prior = output
                        .iter()
                        .map(|value| value.clone().unwrap())
                        .collect::<Vec<_>>();
                    let before = memory.used_bytes();
                    assert!(
                        jit.evaluate_into(&args, &mut output, limits(), &memory)
                            .is_err()
                    );
                    equal(&artifact, &output, &prior);
                    assert_eq!(memory.used_bytes(), before);
                }
                // Check the caller's matrix, independently of the before-version.
                for value in &args {
                    if matches!(value.data(), mech_core::ValueData::Matrix(_)) {
                        assert_eq!(
                            value.canonical_data_draft().unwrap(),
                            buffer(width, &initial)
                        );
                    }
                }
            }
        }
    }
}
#[test]
fn transition_buffer_loop_carries_latest_version_between_visits() {
    let source = "<buffer> := <[u64]:1,4>\n#Fill(xs<buffer>, n<u64>) => <buffer>\n  | :Loop(xs<buffer>, left<u64>)\n  | :Done(xs<buffer>).\n#Fill(xs, n) -> :Loop(xs, n)\n  :Loop(xs, left)\n    | left > 0u64 -> {\n        ~scratch := xs\n        scratch[1u64] = scratch[1u64] + left\n      } -> :Loop(scratch, left - 1u64)\n    | * -> :Done(xs)\n  :Done(xs) => xs.\n#Fill(input<buffer>, count<u64>)\n";
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for n in [0u64, 1, 4, 2, 12, 0] {
            let args = inputs(
                &artifact,
                &[
                    ("input", buffer(64, &[10, 20, 30, 40])),
                    ("count", Data::U64(n)),
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
                buffer(64, &[10 + n * (n + 1) / 2, 20, 30, 40])
            );
        }
    }
}

#[test]
fn indexed_replace_images_restore_only_the_maintained_factory() {
    use mech_engine::native_artifact::{
        NativeArtifactLoweringError, NativeCollectionPrimitive as P, NativeIterationAction,
        lower_native_program_artifact,
    };
    use mech_gpu::native_artifact::NativeProgramDescriptor;
    let source = "<buffer> := <[u8]:1,4>\n#Replace(xs<buffer>, at<u64>, value<u8>) => <buffer>\n  | :Start(xs<buffer>, at<u64>, value<u8>)\n  | :Done(xs<buffer>).\n#Replace(xs, at, value) -> :Start(xs, at, value)\n  :Start(xs, at, value) -> {\n      ~scratch := xs\n      scratch[at] = value\n    } -> :Done(scratch)\n  :Done(xs) => xs.\n#Replace(input<buffer>, position<u64>, replacement<u8>)\n";
    let mut builder = mech_core::FunctionCatalogBuilder::new();
    builder
        .insert_resident_factory(
            ["core", "assign"],
            "indexed-axis",
            mech_core::ImplementationMemoryClass::CanonicalFinalize,
            |_| panic!("native validation must not run the replacement factory"),
        )
        .unwrap();
    for (key, entry) in mech_stdlib::source_catalog().resident_factories() {
        if key.module_path.as_ref() != ["core", "assign"] || key.operation_name != "indexed-axis" {
            builder
                .insert_resident_factory_entry(entry.clone())
                .unwrap();
        }
    }
    let replacement = builder.build().unwrap();
    for artifact in artifacts(source).0 {
        let catalog = mech_stdlib::source_catalog();
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        assert!(
            jit.descriptor()
                .plan()
                .blocks()
                .iter()
                .any(|block| matches!(
            block.action(), NativeIterationAction::LibraryCall { template, .. }
                if template.primitive() == P::MatrixIndexedReplace))
        );
        let bytes = jit.descriptor().encode_build_owned_image().unwrap();
        // SAFETY: inspect an unchanged image produced by this exact checked
        // compilation; no foreign executable is paired with the descriptor.
        let restored =
            unsafe { NativeProgramDescriptor::decode_build_owned_image(&bytes, &catalog) }.unwrap();
        assert_eq!(restored.encode_build_owned_image().unwrap(), bytes);
        assert!(matches!(
            lower_native_program_artifact(&artifact, &replacement),
            Err(NativeArtifactLoweringError::MissingSemanticWitness { .. })
        ));
    }
}

#[test]
fn document_mutation_keeps_its_retained_state_semantics() {
    for artifact in artifacts("~counter := 7u64\ncounter = counter + input<u64>\ncounter\n").0 {
        assert!(
            artifact
                .slots()
                .iter()
                .any(|slot| slot.role == mech_engine::SlotRole::State)
        );
        assert!(JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).is_err());
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let args = inputs(&artifact, &[("input", Data::U64(1))], &memory);
        let ordinary = reference(&artifact, &args);
        assert_eq!(
            ordinary.last().unwrap().canonical_data_draft().unwrap(),
            Data::U64(8)
        );
    }
}

#[test]
fn packed_buffer_projection_admits_packed_storage_before_allocating() {
    use mech_core::{
        SchemaBody as B, ValueDraft,
        snapshot::{EnumDraft, SnapshotValidationContext},
    };
    use mech_engine::native_artifact::{
        NativeCollectionInvocationError as E, NativeIterationAction as A, NativeLibraryWorkBudget,
        NativeStructuralObservation as O, NativeStructuralStep as S, lower_native_program_artifact,
        native_project_structural_value,
    };
    use std::sync::Arc;
    fn draft(body: &B, path: &[S]) -> Data {
        match (body, path.first()) {
            (B::Enum { variants, .. }, Some(S::EnumPayload { ordinal })) => Data::Enum(EnumDraft {
                ordinal: *ordinal,
                payload: variants[*ordinal as usize]
                    .payload
                    .as_ref()
                    .map(|body| Box::new(draft(body, &path[1..]))),
            }),
            (B::Tuple(fields), step) => Data::Tuple(
                fields
                    .iter()
                    .enumerate()
                    .map(|(index, body)| {
                        let path = match step {
                            Some(S::TupleItem { ordinal }) if index == *ordinal as usize => {
                                &path[1..]
                            }
                            _ => &[],
                        };
                        draft(body, path)
                    })
                    .collect(),
            ),
            (B::Matrix { dimensions, .. }, None) => {
                let size = dimensions
                    .iter()
                    .map(|dimension| match dimension {
                        mech_core::DimensionExpr::Constant(size) => *size as usize,
                        _ => panic!("closed dimensions"),
                    })
                    .product();
                Data::Matrix(vec![Data::U8(7); size].into())
            }
            _ => panic!("unexpected projection input {body:?} {path:?}"),
        }
    }
    let source = "<buffer> := <[u8]:1,4096>\n#Copy(xs<buffer>) => <buffer>\n  | :Start(xs<buffer>)\n  | :Done(xs<buffer>).\n#Copy(xs) -> :Start(xs)\n  :Start(xs) -> :Done(xs)\n  :Done(xs) => xs.\n#Copy(input<buffer>)\n";
    for artifact in artifacts(source).0 {
        let plan =
            lower_native_program_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let (site, input, path) = plan
            .blocks()
            .iter()
            .enumerate()
            .find_map(|(site, block)| match block.action() {
                A::Structural {
                    source,
                    path,
                    observation: O::CanonicalValue,
                    output,
                } if matches!(
                    plan.values()[output.get() as usize].definition().body(),
                    B::Matrix { .. }
                ) =>
                {
                    Some((site, *source, path))
                }
                _ => None,
            })
            .expect("real matrix projection");
        let schemas = Arc::new(artifact.schemas().clone());
        let schema = plan.values()[input.get() as usize].schema();
        let value = ValueDraft {
            schema,
            shape_values: Box::new([]),
            data: draft(schemas.get(schema).unwrap().body(), path),
        }
        .finalize(&SnapshotValidationContext::new(&schemas))
        .unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut reservation = memory
            .reserve_capacity(value.memory_budget_admission_bytes(&memory).unwrap())
            .unwrap();
        let value = value.into_memory_budget(&mut reservation).unwrap();
        drop(reservation);
        let work = NativeLibraryWorkBudget::new(1_000_000_000, 1_000_000_000, &memory).unwrap();
        let before = memory.used_bytes();
        let mut policy = limits().library;
        policy.max_temporary_bytes = Some(0);
        let error =
            native_project_structural_value(&plan, site, &schemas, &value, None, policy, &work)
                .unwrap_err();
        let E::Plan(mech_core::MemoryPlanError::TargetLimitExceeded { violation }) = error else {
            panic!("expected temporary-memory denial: {error:?}");
        };
        assert_eq!(
            violation.dimension,
            mech_core::MemoryBudgetDimension::TemporaryBytes
        );
        let expansion = 4096
            * (core::mem::size_of::<Data>() + core::mem::size_of::<mech_core::ValueData>()) as u64;
        assert!(
            violation.required > 4096 && violation.required < expansion,
            "projection must fund its packed payload and owners without scalar drafts"
        );
        assert_eq!(memory.used_bytes(), before);
        policy.max_temporary_bytes = Some(violation.required - 1);
        assert!(
            native_project_structural_value(&plan, site, &schemas, &value, None, policy, &work)
                .is_err()
        );
        assert_eq!(memory.used_bytes(), before);
        policy.max_temporary_bytes = Some(violation.required);
        let result =
            native_project_structural_value(&plan, site, &schemas, &value, None, policy, &work)
                .unwrap();
        assert_eq!(
            result.canonical_data_draft().unwrap(),
            Data::Matrix(vec![Data::U8(7); 4096].into())
        );
        drop((result, value, work));
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn enum_state_pack_admits_unsigned_buffer_draft_expansion() {
    use mech_core::{ResolvedOperationContract, ValueDraft, snapshot::SnapshotValidationContext};
    use mech_engine::native_artifact::{
        NativeCollectionInvocationError as E, NativeCollectionPrimitive as P,
        NativeIterationAction as A, NativeLibraryWorkBudget, bind_native_program_library_call,
        lower_native_program_artifact,
    };
    let source = "<buffer> := <[u8]:1,65536>\n#Copy(xs<buffer>) => <buffer>\n  | :Start(xs<buffer>)\n  | :Done(xs<buffer>).\n#Copy(xs) -> :Start(xs)\n  :Start(xs) -> :Done(xs)\n  :Done(xs) => xs.\n#Copy(input<buffer>)\n";
    for artifact in artifacts(source).0 {
        let catalog = mech_stdlib::source_catalog();
        let plan = lower_native_program_artifact(&artifact, &catalog).unwrap();
        let template = plan
            .blocks()
            .iter()
            .find_map(|block| match block.action() {
                A::LibraryCall { template, .. } if template.primitive() == P::EnumPack => {
                    Some(template)
                }
                _ => None,
            })
            .unwrap();
        let ResolvedOperationContract::Declared(contract) = template.contract() else {
            panic!()
        };
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let args = contract
            .inputs
            .iter()
            .zip([
                Data::Index(1),
                Data::Matrix(vec![Data::U8(7); 65536].into()),
            ])
            .map(|(port, data)| {
                let value = ValueDraft {
                    schema: port.schema,
                    shape_values: Box::new([]),
                    data,
                }
                .finalize(&SnapshotValidationContext::new(artifact.schemas()))
                .unwrap();
                let mut reservation = memory
                    .reserve_capacity(value.memory_budget_admission_bytes(&memory).unwrap())
                    .unwrap();
                Some(value.into_memory_budget(&mut reservation).unwrap())
            })
            .collect::<Vec<_>>();
        let expansion = 65536
            * (core::mem::size_of::<Data>() + core::mem::size_of::<mech_core::ValueData>()) as u64;
        let mut policy = limits().library;
        policy.max_temporary_bytes = Some(expansion / 2);
        let call = bind_native_program_library_call(
            &artifact,
            template,
            &catalog,
            &[&[], &[]],
            &[],
            policy,
        )
        .unwrap();
        let work = NativeLibraryWorkBudget::new(1_000_000_000, 1_000_000_000, &memory).unwrap();
        let before = memory.used_bytes();
        let mut output = None;
        let error = call.invoke(&args, &mut output, &work).unwrap_err();
        assert!(
            matches!(error, E::Kernel(mech_core::ResidentKernelError::ResourceLimit {
            dimension: mech_core::MemoryBudgetDimension::TemporaryBytes, required, limit, ..
        }) if required >= expansion && limit == expansion / 2),
            "{error:?}"
        );
        assert!(output.is_none());
        assert_eq!(memory.used_bytes(), before);
        let restored = bind_native_program_library_call(
            &artifact,
            template,
            &catalog,
            &[&[], &[]],
            &[],
            limits().library,
        )
        .unwrap();
        restored.invoke(&args, &mut output, &work).unwrap();
        let mech_core::ValueData::Enum(value) = output.as_ref().unwrap().data() else {
            panic!()
        };
        assert_eq!(value.ordinal(), 0);
        let Some(mech_core::ValueData::Matrix(value)) = value.payload() else {
            panic!()
        };
        assert_eq!(value.elements().len(), 65536);
        drop((output, args, work));
        assert_eq!(memory.used_bytes(), 0);
    }
}
