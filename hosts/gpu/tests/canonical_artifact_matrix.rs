//! Maintained resident matrix leaves through the canonical Cranelift program.
use mech_core::{
    FunctionCatalogBuilder, ManagedMemoryBudget, MemoryBudgetLimits, ResidentKernelFactoryEntry,
    SchemaBody, Value, ValueDataDraft as Data,
    snapshot::{F64Bits, SnapshotValidationContext, ValueDraft},
};
use mech_engine::{CanonicalSourceFrontend, ProgramArtifact};
use mech_gpu::native_artifact::{
    JitProgramArtifact, NativeExecutionLimits, NativeProgramDescriptor,
    NativeProgramExecutionLimits, emit_program_object,
};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

fn artifacts(source: &str) -> [ProgramArtifact; 2] {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x4d415452), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
    let syntax = DocumentSyntax::cast(parsed.syntax()).unwrap();
    let original = CanonicalSourceFrontend
        .compile_document_with_catalog(&syntax, mech_stdlib::source_catalog())
        .unwrap()
        .compile_artifact()
        .unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    [original, decoded]
}

fn check_restored_image(jit: &JitProgramArtifact) {
    let image = jit.descriptor().encode_build_owned_image().unwrap();
    // SAFETY: this JIT build owns the unmodified image; only metadata is inspected.
    let restored = unsafe {
        NativeProgramDescriptor::decode_build_owned_image(&image, &mech_stdlib::source_catalog())
    }
    .unwrap();
    assert_eq!(restored.encode_build_owned_image().unwrap(), image);
}

fn limits() -> NativeProgramExecutionLimits {
    let cap = 1_000_000_000;
    NativeProgramExecutionLimits {
        execution: NativeExecutionLimits {
            max_work: cap,
            max_transport_bytes: 256 * 1024 * 1024,
            max_native_stack_bytes: 8 * 1024 * 1024,
        },
        library: MemoryBudgetLimits {
            max_output_elements: Some(1_000_000),
            max_output_bytes: Some(256 * 1024 * 1024),
            max_temporary_bytes: Some(256 * 1024 * 1024),
            max_cloned_bytes: Some(cap),
            max_retained_nodes: Some(1_000_000),
            max_comparison_work: Some(cap),
            max_compute_work: Some(cap),
            max_scalar_instructions: Some(cap),
            max_transfer_bytes: Some(cap),
            max_storage_buffer_bytes: Some(256 * 1024 * 1024),
            max_storage_bindings: Some(1024),
        },
    }
}

fn inputs(
    artifact: &ProgramArtifact,
    named: &[(&str, u64, u64, &[f64])],
    memory: &ManagedMemoryBudget,
) -> Vec<Value> {
    artifact
        .inputs()
        .iter()
        .map(|input| {
            let name = mech_engine::decode_source_input_name(&input.name).unwrap();
            let (_, rows, columns, numbers) = named.iter().find(|(key, ..)| *key == name).unwrap();
            assert_eq!(numbers.len() as u64, rows * columns);
            let schema = artifact.schemas().get(input.schema).unwrap();
            let (data, shape_values) = if matches!(schema.body(), SchemaBody::Matrix { .. }) {
                (
                    Data::Matrix(
                        numbers
                            .iter()
                            .map(|v| Data::F64(F64Bits::from_f64(*v)))
                            .collect(),
                    ),
                    mech_core::shape_for_resolved_extents(schema, &[*rows, *columns])
                        .unwrap()
                        .parameter_values()
                        .into(),
                )
            } else {
                assert_eq!(numbers.len(), 1);
                (
                    Data::F64(F64Bits::from_f64(numbers[0])),
                    Box::new([]) as Box<[u64]>,
                )
            };
            let value = ValueDraft {
                schema: input.schema,
                shape_values,
                data,
            }
            .finalize(&SnapshotValidationContext::new(artifact.schemas()))
            .unwrap();
            let mut reservation = memory
                .reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap())
                .unwrap();
            value.into_memory_budget(&mut reservation).unwrap()
        })
        .collect()
}

fn typed_inputs(
    artifact: &ProgramArtifact,
    named: &[(&str, [u64; 2], Data)],
    memory: &ManagedMemoryBudget,
) -> Vec<Value> {
    artifact
        .inputs()
        .iter()
        .map(|input| {
            let name = mech_engine::decode_source_input_name(&input.name).unwrap();
            let (_, dimensions, data) = named.iter().find(|(key, ..)| *key == name).unwrap();
            let schema = artifact.schemas().get(input.schema).unwrap();
            let shape_values = if matches!(schema.body(), SchemaBody::Matrix { .. }) {
                mech_core::shape_for_resolved_extents(schema, dimensions)
                    .unwrap()
                    .parameter_values()
                    .into()
            } else {
                Box::new([]) as Box<[u64]>
            };
            let value = ValueDraft {
                schema: input.schema,
                shape_values,
                data: data.clone(),
            }
            .finalize(&SnapshotValidationContext::new(artifact.schemas()))
            .unwrap();
            let mut reservation = memory
                .reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap())
                .unwrap();
            value.into_memory_budget(&mut reservation).unwrap()
        })
        .collect()
}

fn update_cases() -> Vec<(&'static str, usize, Vec<Data>, Data, u64)> {
    let mut cases = Vec::new();
    for (kind, width) in [("u8", 1), ("u16", 2), ("u32", 4), ("u64", 8)] {
        let cell = |word| match kind {
            "u8" => Data::U8(word as u8),
            "u16" => Data::U16(word as u16),
            "u32" => Data::U32(word as u32),
            "u64" => Data::U64(word),
            _ => unreachable!(),
        };
        cases.push((kind, width, (1..=6).map(cell).collect(), cell(127), 127));
    }
    cases.push(("bool", 1, vec![Data::Bool(false); 6], Data::Bool(true), 1));
    cases.push((
        "i64",
        8,
        [i64::MIN, -2, -1, 0, 1, i64::MAX].map(Data::I64).into(),
        Data::I64(i64::MIN),
        i64::MIN as u64,
    ));
    cases.push((
        "f64",
        8,
        [
            0x8000_0000_0000_0000,
            0x7ff8_0000_0000_0001,
            0,
            0x7ff0_0000_0000_0000,
            0xfff0_0000_0000_0000,
            0x3ff0_0000_0000_0000,
        ]
        .map(|bits| Data::F64(F64Bits::from_bits(bits)))
        .into(),
        Data::F64(F64Bits::from_bits(0x7ff8_0000_0000_0002)),
        0x7ff8_0000_0000_0002,
    ));
    cases
}

fn matrix_pointer(value: &Value) -> usize {
    use mech_core::{ValueData, snapshot::SequenceView};
    let ValueData::Matrix(matrix) = value.data() else {
        panic!("matrix")
    };
    match matrix.elements() {
        SequenceView::Bool(v) => v.as_ptr() as usize,
        SequenceView::U8(v) => v.as_ptr() as usize,
        SequenceView::U16(v) => v.as_ptr() as usize,
        SequenceView::U32(v) => v.as_ptr() as usize,
        SequenceView::U64(v) => v.as_ptr() as usize,
        SequenceView::I64(v) => v.as_ptr() as usize,
        SequenceView::F64(v) => v.as_ptr() as usize,
        _ => panic!("native scalar kind"),
    }
}

#[test]
fn selected_scalar_matrix_updates_reuse_owned_storage_and_keep_exact_finite_denials() {
    use mech_core::MemoryBudgetDimension as Dimension;
    use mech_engine::native_artifact::{
        NativeCollectionPrimitive as Primitive, NativeIterationAction as Action,
        NativeLibraryWorkBudget, lower_native_program_artifact,
        prepare_native_program_library_call,
    };
    for (kind, width, cells, incoming, word) in update_cases() {
        for axes in [1, 2] {
            let selector = if axes == 1 { "at" } else { "row,column" };
            let source = format!(
                "replace(xs<[{kind}]>, at<u64>, row<u64>, column<u64>, word<{kind}>) => (out<[{kind}]>) :=\n  ~next := xs\n  next[{selector}] = word\n  out := next.\nreplace(input<[{kind}]>, at<u64>, row<u64>, column<u64>, word<{kind}>)\n"
            );
            for artifact in artifacts(&source) {
                let catalog = mech_stdlib::source_catalog();
                let plan = lower_native_program_artifact(&artifact, &catalog).unwrap();
                let primitive = if axes == 1 {
                    Primitive::MatrixIndexedReplace
                } else {
                    Primitive::MatrixCellReplace
                };
                let template = plan
                    .blocks()
                    .iter()
                    .find_map(|block| match block.action() {
                        Action::LibraryCall { template, .. }
                            if template.primitive() == primitive =>
                        {
                            Some(template)
                        }
                        _ => None,
                    })
                    .unwrap();
                let call =
                    prepare_native_program_library_call(&artifact, template, &catalog).unwrap();
                assert!(call.scalar_matrix_replace_kind().is_some());
                let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
                let args = typed_inputs(
                    &artifact,
                    &[
                        ("input", [2, 3], Data::Matrix(cells.clone().into())),
                        ("at", [1, 1], Data::U64(2)),
                        ("row", [1, 1], Data::U64(2)),
                        ("column", [1, 1], Data::U64(1)),
                        ("word", [1, 1], incoming.clone()),
                    ],
                    &memory,
                );
                let original = &args[artifact
                    .inputs()
                    .iter()
                    .position(|port| {
                        mech_engine::decode_source_input_name(&port.name).unwrap() == "input"
                    })
                    .unwrap()];
                let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
                check_restored_image(&jit);
                assert!(
                    !emit_program_object(&artifact, &catalog)
                        .unwrap()
                        .bytes()
                        .is_empty()
                );
                let mut published = vec![None; artifact.outputs().len()];
                jit.evaluate_into(&args, &mut published, limits(), &memory)
                    .unwrap();
                let mut native_expected = cells.clone();
                native_expected[3] = incoming.clone();
                assert_eq!(
                    published
                        .last()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    Data::Matrix(native_expected.into()),
                    "{kind}/{axes}: native operand transport"
                );
                drop(published);
                let indices: &[u64] = if axes == 1 { &[2] } else { &[2, 1] };
                let work = NativeLibraryWorkBudget::new(1 << 40, 1 << 32, &memory).unwrap();
                let policy = limits().library;
                let start = work.used();
                let (first, changed) = call
                    .scalar_matrix_replace(
                        original.clone(),
                        indices,
                        word,
                        true,
                        policy,
                        &work,
                        Default::default(),
                    )
                    .unwrap();
                let cost = (work.used().0 - start.0, work.used().1 - start.1);
                assert!(changed);
                assert_ne!(matrix_pointer(&first), matrix_pointer(original));
                let mut expected = cells.clone();
                expected[3] = incoming.clone();
                assert_eq!(
                    first.canonical_data_draft().unwrap(),
                    Data::Matrix(expected.into())
                );
                assert_eq!(
                    original.canonical_data_draft().unwrap(),
                    Data::Matrix(cells.clone().into())
                );
                let footprint = original.retained_footprint(artifact.schemas()).unwrap();
                let payload = (6 * width) as u64;
                let metadata = call.binding_metadata_bytes()
                    + artifact.schemas().clone_allocation_bound_bytes().unwrap()
                    + 4096;
                let mut exact = policy;
                exact.max_output_elements = Some(6);
                exact.max_output_bytes = Some(footprint.retained_bytes);
                exact.max_cloned_bytes = Some(payload);
                exact.max_temporary_bytes = Some(
                    original.memory_budget_retained_bytes(&memory).unwrap() + metadata + payload,
                );
                exact.max_retained_nodes = Some(
                    original.memory_budget_retained_nodes(&memory).unwrap()
                        + footprint.node_count
                        + 8,
                );
                exact.max_compute_work = Some(cost.0);
                exact.max_comparison_work = Some(cost.1);
                drop(
                    call.scalar_matrix_replace(
                        original.clone(),
                        indices,
                        word,
                        true,
                        exact,
                        &work,
                        Default::default(),
                    )
                    .unwrap(),
                );
                for dimension in [
                    Dimension::OutputElements,
                    Dimension::OutputBytes,
                    Dimension::ClonedBytes,
                    Dimension::TemporaryBytes,
                    Dimension::RetainedNodes,
                    Dimension::ComputeWork,
                    Dimension::ComparisonWork,
                ] {
                    let mut short = exact;
                    let limit = match dimension {
                        Dimension::OutputElements => &mut short.max_output_elements,
                        Dimension::OutputBytes => &mut short.max_output_bytes,
                        Dimension::ClonedBytes => &mut short.max_cloned_bytes,
                        Dimension::TemporaryBytes => &mut short.max_temporary_bytes,
                        Dimension::RetainedNodes => &mut short.max_retained_nodes,
                        Dimension::ComputeWork => &mut short.max_compute_work,
                        Dimension::ComparisonWork => &mut short.max_comparison_work,
                        _ => unreachable!(),
                    };
                    *limit = Some(limit.unwrap() - 1);
                    let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let error = call
                        .scalar_matrix_replace(
                            original.clone(),
                            indices,
                            word,
                            true,
                            short,
                            &work,
                            Default::default(),
                        )
                        .unwrap_err();
                    let denied = match error {
                        mech_engine::native_artifact::NativeCollectionInvocationError::Plan(
                            mech_core::MemoryPlanError::TargetLimitExceeded { violation },
                        ) => violation.dimension,
                        mech_engine::native_artifact::NativeCollectionInvocationError::Kernel(
                            mech_core::ResidentKernelError::ResourceLimit { dimension, .. },
                        ) => dimension,
                        other => panic!("{kind}/{axes}/{dimension:?}: {other:?}"),
                    };
                    assert_eq!(denied, dimension, "{kind}/{axes}");
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        retained
                    );
                }
                let pointer = matrix_pointer(&first);
                let old_word = match &cells[3] {
                    Data::Bool(v) => u64::from(*v),
                    Data::U8(v) => *v as u64,
                    Data::U16(v) => *v as u64,
                    Data::U32(v) => *v as u64,
                    Data::U64(v) => *v,
                    Data::I64(v) => *v as u64,
                    Data::F64(v) => v.bits(),
                    _ => unreachable!(),
                };
                let mut zero_copy = policy;
                zero_copy.max_cloned_bytes = Some(0);
                let (second, changed) = call
                    .scalar_matrix_replace(
                        first,
                        indices,
                        old_word,
                        true,
                        zero_copy,
                        &work,
                        Default::default(),
                    )
                    .unwrap();
                assert!(changed);
                assert_eq!(
                    matrix_pointer(&second),
                    pointer,
                    "{kind}/{axes}: reuse private backing"
                );
                assert_eq!(
                    second.canonical_data_draft().unwrap(),
                    Data::Matrix(cells.clone().into())
                );
                let (same, changed) = call
                    .scalar_matrix_replace(
                        second.clone(),
                        indices,
                        old_word,
                        true,
                        zero_copy,
                        &work,
                        Default::default(),
                    )
                    .unwrap();
                assert!(!changed);
                assert!(same.shares_frozen_storage(&second));
                drop((same, second, args, work));
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
        }
    }
}

fn check_output(
    artifact: &ProgramArtifact,
    output: &[Option<Value>],
    shape: [u64; 2],
    expected: &[f64],
) {
    let value = output.last().unwrap().as_ref().unwrap();
    let SchemaBody::Matrix { dimensions, .. } =
        artifact.schemas().get(value.schema()).unwrap().body()
    else {
        panic!("matrix output")
    };
    assert_eq!(
        dimensions
            .iter()
            .map(|d| value.shape().resolve_dimension(d).unwrap())
            .collect::<Vec<_>>(),
        shape
    );
    let Data::Matrix(values) = value.canonical_data_draft().unwrap() else {
        panic!("matrix data")
    };
    let actual = values
        .iter()
        .map(|v| match v {
            Data::F64(v) => v.to_f64(),
            _ => panic!("f64"),
        })
        .collect::<Vec<_>>();
    assert_eq!(actual, expected);
}

#[test]
fn typed_comprehension_banks_admit_finish_copies_and_recover_without_publication() {
    for (kind, width) in [("bool", 1usize), ("i64", 8), ("f64", 8)] {
        let source = format!("[value | value <- input<[{kind}]>]\n");
        for artifact in artifacts(&source) {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            check_restored_image(&jit);
            assert!(
                !emit_program_object(&artifact, &mech_stdlib::source_catalog())
                    .unwrap()
                    .bytes()
                    .is_empty()
            );
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let mut output = vec![None; artifact.outputs().len()];
            for count in [3usize, 4, 5] {
                let cells = (0..count)
                    .map(|i| match kind {
                        "bool" => Data::Bool(i % 2 != 0),
                        "i64" => Data::I64(if i % 2 == 0 {
                            i64::MIN + i as i64
                        } else {
                            i64::MAX - i as i64
                        }),
                        "f64" => Data::F64(F64Bits::from_bits(
                            [
                                0x8000_0000_0000_0000,
                                0x7ff8_0000_0000_0001,
                                0x7ff0_0000_0000_0000,
                                0xfff0_0000_0000_0000,
                                0,
                            ][i],
                        )),
                        _ => unreachable!(),
                    })
                    .collect::<Box<[_]>>();
                let data = Data::Matrix(cells);
                let args = typed_inputs(
                    &artifact,
                    &[("input", [1, count as u64], data.clone())],
                    &memory,
                );
                let aliases = args.clone();
                let payload = (count * width) as u64;
                // Source reads and output writes retain their logical copies.
                // A growing Vec may also move when finalized as a boxed slice.
                let cloned = payload * if count.is_power_of_two() { 2 } else { 3 };
                let mut policy = limits();
                policy.library.max_cloned_bytes = Some(cloned);
                jit.evaluate_into(&args, &mut output, policy, &memory)
                    .unwrap();
                assert_eq!(
                    output
                        .last()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    data
                );
                let prior = output.clone();
                let retained = memory.used_bytes();
                policy.library.max_cloned_bytes = Some(cloned - 1);
                let error = jit
                    .evaluate_into(&args, &mut output, policy, &memory)
                    .unwrap_err();
                assert!(
                    matches!(error,
                        mech_gpu::native_artifact::NativeProgramExecutionError::Collection {
                            error: mech_core::ResidentKernelError::ResourceLimit {
                                dimension: mech_core::MemoryBudgetDimension::ClonedBytes,
                                required,
                                limit,
                                ..
                            }, ..
                        } if required == cloned && limit == cloned - 1
                    ),
                    "{kind}/{count}: {error:?}"
                );
                assert_eq!(memory.used_bytes(), retained);
                assert!(output.iter().zip(&prior).all(|(actual, old)| {
                    actual
                        .as_ref()
                        .unwrap()
                        .shares_frozen_storage(old.as_ref().unwrap())
                }));
                assert!(
                    args.iter()
                        .zip(&aliases)
                        .all(|(actual, old)| actual.shares_frozen_storage(old))
                );
                policy.library.max_cloned_bytes = Some(cloned);
                jit.evaluate_into(&args, &mut output, policy, &memory)
                    .unwrap();
            }
            drop(output);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn rectangular_products_and_transposes_preserve_original_and_decoded_geometry() {
    for (source, shape, expected) in [
        (
            "a := left<[f64]>\na'\n",
            [3, 2],
            vec![1., 4., 2., 5., 3., 6.],
        ),
        (
            "left<[f64]> ** right<[f64]>\n",
            [2, 2],
            vec![58., 64., 139., 154.],
        ),
    ] {
        for artifact in artifacts(source) {
            let catalog = mech_stdlib::source_catalog();
            let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
            let object = emit_program_object(&artifact, &catalog).unwrap();
            assert!(!object.bytes().is_empty());
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let args = inputs(
                &artifact,
                &[
                    ("left", 2, 3, &[1., 2., 3., 4., 5., 6.]),
                    ("right", 3, 2, &[7., 8., 9., 10., 11., 12.]),
                ],
                &memory,
            );
            let mut output = vec![None; artifact.outputs().len()];
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            check_output(&artifact, &output, shape, &expected);
        }
    }
}

#[test]
fn packed_f64_products_preserve_reduction_order_and_empty_inner_axes() {
    for artifact in artifacts("left<[f64]> ** right<[f64]>\n") {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        // Reassociation would retain the middle term and produce a different
        // result. Packed products retain the generic numeric sum's left fold.
        let args = inputs(
            &artifact,
            &[
                ("left", 1, 3, &[1e16, 1., -1e16]),
                ("right", 3, 2, &[1., 2., 1., 2., 1., 2.]),
            ],
            &memory,
        );
        jit.evaluate_into(&args, &mut output, limits(), &memory)
            .unwrap();
        check_output(&artifact, &output, [1, 2], &[0., 0.]);
        let args = inputs(
            &artifact,
            &[("left", 2, 0, &[]), ("right", 0, 3, &[])],
            &memory,
        );
        jit.evaluate_into(&args, &mut output, limits(), &memory)
            .unwrap();
        check_output(&artifact, &output, [2, 3], &[0.; 6]);
        let args = inputs(
            &artifact,
            &[("left", 1, 2, &[-0., -0.]), ("right", 2, 1, &[1., 1.])],
            &memory,
        );
        jit.evaluate_into(&args, &mut output, limits(), &memory)
            .unwrap();
        let Data::Matrix(cells) = output
            .last()
            .unwrap()
            .as_ref()
            .unwrap()
            .canonical_data_draft()
            .unwrap()
        else {
            panic!("matrix")
        };
        assert_eq!(cells[0], Data::F64(F64Bits::from_f64(0.0)));
    }
}

#[test]
fn atax_composes_the_existing_matrix_leaves_without_a_resident_program() {
    for artifact in artifacts("a := matrix<[f64]>\na' ** (a ** vector<[f64]>)\n") {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let args = inputs(
            &artifact,
            &[
                ("matrix", 2, 3, &[1., 2., 3., 4., 5., 6.]),
                ("vector", 3, 1, &[1., 2., 3.]),
            ],
            &memory,
        );
        let mut output = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut output, limits(), &memory)
            .unwrap();
        check_output(&artifact, &output, [3, 1], &[142., 188., 234.]);
    }
}

#[test]
fn matrix_arithmetic_reuses_scalar_row_column_and_exact_broadcasting() {
    for (source, rows, columns, right, expected) in [
        (
            "left<[f64]> + right<[f64]>\n",
            1,
            3,
            vec![10., 20., 30.],
            vec![11., 22., 33., 14., 25., 36.],
        ),
        (
            "left<[f64]> - right<[f64]>\n",
            2,
            1,
            vec![1., 2.],
            vec![0., 1., 2., 2., 3., 4.],
        ),
        (
            "left<[f64]> * right<[f64]>\n",
            2,
            3,
            vec![2., 3., 4., 5., 6., 7.],
            vec![2., 6., 12., 20., 30., 42.],
        ),
        (
            "left<[f64]> / right<[f64]>\n",
            1,
            1,
            vec![2.],
            vec![0.5, 1., 1.5, 2., 2.5, 3.],
        ),
        (
            "2.0 * left<[f64]> + right<[f64]> / 2.0\n",
            1,
            3,
            vec![10., 20., 30.],
            vec![7., 14., 21., 13., 20., 27.],
        ),
        (
            "10.0 - left<[f64]>\n",
            1,
            1,
            vec![0.],
            vec![9., 8., 7., 6., 5., 4.],
        ),
    ] {
        for artifact in artifacts(source) {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            assert!(
                !emit_program_object(&artifact, &mech_stdlib::source_catalog())
                    .unwrap()
                    .bytes()
                    .is_empty()
            );
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let args = inputs(
                &artifact,
                &[
                    ("left", 2, 3, &[1., 2., 3., 4., 5., 6.]),
                    ("right", rows, columns, &right),
                ],
                &memory,
            );
            let mut output = vec![None; artifact.outputs().len()];
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            check_output(&artifact, &output, [2, 3], &expected);
        }
    }
}

#[test]
fn numeric_registration_preserves_direct_scalar_arithmetic() {
    for artifact in artifacts("left<f64> * right<f64> + 10.0\n") {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let args = inputs(
            &artifact,
            &[("left", 1, 1, &[3.]), ("right", 1, 1, &[4.])],
            &memory,
        );
        let mut output = vec![None; artifact.outputs().len()];
        assert!(
            jit.descriptor()
                .plan()
                .library_operation_references()
                .is_empty(),
            "scalar arithmetic must not invoke library leaves"
        );
        jit.evaluate_into(&args, &mut output, limits(), &memory)
            .unwrap();
        assert_eq!(
            output
                .last()
                .unwrap()
                .as_ref()
                .unwrap()
                .canonical_data_draft()
                .unwrap(),
            Data::F64(F64Bits::from_f64(22.))
        );
    }
}

#[test]
fn independently_shaped_f64_matrices_can_be_packed_and_projected() {
    for source in [
        "pair := (left<[f64]>, right<[f64]>)\npair[1u64]\n",
        "pair := (left<[f64]>, right<[f64]>)\npair[2u64]\n",
    ] {
        for artifact in artifacts(source) {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let args = inputs(
                &artifact,
                &[
                    ("left", 2, 3, &[1., 2., 3., 4., 5., 6.]),
                    ("right", 3, 1, &[7., 8., 9.]),
                ],
                &memory,
            );
            let mut output = vec![None; artifact.outputs().len()];
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            if source.contains("pair[1u64]") {
                check_output(&artifact, &output, [2, 3], &[1., 2., 3., 4., 5., 6.]);
            } else {
                check_output(&artifact, &output, [3, 1], &[7., 8., 9.]);
            }
        }
    }
}

#[test]
fn nested_comprehensions_preserve_rectangular_matrix_blocks_and_coordinate_access() {
    for artifact in
        artifacts("a := left<[f64]>\n[[a[i,j] * 2.0 | i <- 1u64..=2u64]' | j <- 1u64..=3u64]\n")
    {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let args = inputs(
            &artifact,
            &[("left", 2, 3, &[1., 2., 3., 4., 5., 6.])],
            &memory,
        );
        let mut output = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut output, limits(), &memory)
            .unwrap();
        check_output(&artifact, &output, [2, 3], &[2., 4., 6., 8., 10., 12.]);
    }
}

#[test]
fn dynamic_matrix_blocks_keep_empty_axes_and_reclose_each_invocation() {
    for artifact in artifacts("a := left<[f64]>\n[a | * <- 1u64..=2u64]\n") {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for (rows, columns, values, expected) in [
            (
                2,
                3,
                vec![1., 2., 3., 4., 5., 6.],
                vec![1., 2., 3., 1., 2., 3., 4., 5., 6., 4., 5., 6.],
            ),
            (0, 3, vec![], vec![]),
            (3, 0, vec![], vec![]),
            (1, 2, vec![7., 9.], vec![7., 9., 7., 9.]),
        ] {
            let args = inputs(&artifact, &[("left", rows, columns, &values)], &memory);
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            check_output(&artifact, &output, [rows, columns * 2], &expected);
            let prior = output.clone();
            let retained = memory.used_bytes();
            let mut denied = limits();
            denied.library.max_compute_work = Some(1);
            assert!(
                jit.evaluate_into(&args, &mut output, denied, &memory)
                    .is_err()
            );
            assert_eq!(memory.used_bytes(), retained);
            assert!(output.iter().zip(&prior).all(|(a, b)| {
                a.as_ref()
                    .unwrap()
                    .shares_frozen_storage(b.as_ref().unwrap())
            }));
        }
    }
}

#[test]
fn inconsistent_matrix_blocks_reject_without_publishing_and_recover() {
    let source =
        "a := left<[f64]>\n[[a[i,j] | i <- 1u64..=j]' | j <- 1u64..=2u64, j<f64> <= limit<f64>]\n";
    for artifact in artifacts(source) {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let good = inputs(
            &artifact,
            &[("left", 2, 2, &[1., 2., 3., 4.]), ("limit", 1, 1, &[1.])],
            &memory,
        );
        let bad = inputs(
            &artifact,
            &[("left", 2, 2, &[1., 2., 3., 4.]), ("limit", 1, 1, &[2.])],
            &memory,
        );
        let mut output = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&good, &mut output, limits(), &memory)
            .unwrap();
        let prior = output.clone();
        let retained = memory.used_bytes();
        assert!(
            jit.evaluate_into(&bad, &mut output, limits(), &memory)
                .is_err()
        );
        assert_eq!(memory.used_bytes(), retained);
        assert!(output.iter().zip(&prior).all(|(a, b)| {
            a.as_ref()
                .unwrap()
                .shares_frozen_storage(b.as_ref().unwrap())
        }));
        jit.evaluate_into(&good, &mut output, limits(), &memory)
            .unwrap();
        check_output(&artifact, &output, [1, 1], &[1.]);
    }
}

#[test]
fn square_root_uses_the_existing_matrix_leaf() {
    for artifact in artifacts("+> math\nmath/sqrt(left<[f64]>)\n") {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let args = inputs(
            &artifact,
            &[("left", 2, 3, &[0., 1., 4., 9., 16., 25.])],
            &memory,
        );
        let mut output = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut output, limits(), &memory)
            .unwrap();
        check_output(&artifact, &output, [2, 3], &[0., 1., 2., 3., 4., 5.]);
    }
}

#[test]
fn f64_cell_reads_check_each_axis_preserve_prior_output_and_recover() {
    for artifact in artifacts("a := left<[f64]>\na[2u64,3u64]\n") {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let good = inputs(
            &artifact,
            &[("left", 2, 3, &[1., 2., 3., 4., 5., 6.])],
            &memory,
        );
        let mut output = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&good, &mut output, limits(), &memory)
            .unwrap();
        let prior = output.clone();
        for (rows, columns, values) in [(1, 6, vec![1.; 6]), (3, 2, vec![1.; 6]), (0, 3, vec![])] {
            let bad = inputs(&artifact, &[("left", rows, columns, &values)], &memory);
            let retained = memory.used_bytes();
            assert!(
                jit.evaluate_into(&bad, &mut output, limits(), &memory)
                    .is_err()
            );
            assert_eq!(memory.used_bytes(), retained);
            assert!(output.iter().zip(&prior).all(|(a, b)| {
                a.as_ref()
                    .unwrap()
                    .shares_frozen_storage(b.as_ref().unwrap())
            }));
        }
        let mut denied = limits();
        denied.library.max_output_bytes = Some(7);
        assert!(
            jit.evaluate_into(&good, &mut output, denied, &memory)
                .is_err()
        );
        jit.evaluate_into(&good, &mut output, limits(), &memory)
            .unwrap();
        assert_eq!(
            output
                .last()
                .unwrap()
                .as_ref()
                .unwrap()
                .canonical_data_draft()
                .unwrap(),
            Data::F64(F64Bits::from_f64(6.))
        );
    }
}

#[test]
fn f64_linear_reads_reclose_column_major_indices_without_changing_bits() {
    for artifact in artifacts("a := left<[f64]>\na[2u64]\n") {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for (rows, columns, values, expected) in [
            (2, 3, vec![1., 2., 3., 4., 5., 6.], 4.),
            (1, 6, vec![1., 2., 3., 4., 5., 6.], 2.),
            (2, 1, vec![1., -0.], -0.),
        ] {
            let args = inputs(&artifact, &[("left", rows, columns, &values)], &memory);
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            assert_eq!(
                output
                    .last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                Data::F64(F64Bits::from_f64(expected))
            );
        }
    }
}

#[test]
fn fixed_cardinality_comprehensions_can_cross_f64_fsm_state_transitions() {
    let source = "#Twice(input<[f64]:2,1>) => <[f64]:2,1>\n\
      | :Next(values<[f64]:2,1>, remaining<u64>)\n\
      | :Done(values<[f64]:2,1>).\n\
    #Twice(input) -> :Next(input, 2u64)\n\
      :Next(values, remaining)\n\
        | remaining > 0u64 -> {\n\
            next := [values[i] * 2.0 | i <- 1u64..=2u64]'\n\
          } -> :Next(next, remaining - 1u64)\n\
        | * -> :Done(values)\n\
      :Done(values) => values.\n\
    #Twice(left<[f64]:2,1>)\n";
    for artifact in artifacts(source) {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let args = inputs(&artifact, &[("left", 2, 1, &[3., 5.])], &memory);
        let mut output = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut output, limits(), &memory)
            .unwrap();
        check_output(&artifact, &output, [2, 1], &[12., 20.]);
    }
}

#[test]
fn triangular_substitution_combines_f64_state_indexing_reduction_and_replacement() {
    for artifact in artifacts(include_str!(
        "fixtures/native/matrix-triangular-substitution.mec"
    )) {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let args = inputs(
            &artifact,
            &[
                ("matrix", 3, 3, &[2., 0., 0., 3., 1., 0., 1., -1., 2.]),
                ("rhs", 3, 1, &[2., 5., 3.]),
                ("initial", 3, 1, &[0.; 3]),
            ],
            &memory,
        );
        let mut output = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut output, limits(), &memory)
            .unwrap();
        check_output(&artifact, &output, [3, 1], &[1., 2., 2.]);
    }
}

#[test]
fn private_f64_state_keeps_aliases_atomic_denials_and_fresh_recovery() {
    let source = "#Sweep(initial<[f64]:2,3>) => <[f64]:2,3>\n\
      | :Next(values<[f64]:2,3>, step<u64>, offset<f64>)\n\
      | :Done(values<[f64]:2,3>).\n\
    #Sweep(initial) -> :Next(initial, 0u64, 0.5)\n\
      :Next(values, step, offset)\n\
        | step < 3u64 -> {\n\
            at := ((values[1u64,1u64] < 0.0)? | true => 0u64 | false => step + 1u64)\n\
            ~next := values\n\
            next[2u64,at] = values[2u64,step + 1u64] + offset\n\
          } -> :Next(next, step + 1u64, offset + 1.0)\n\
        | * -> :Done(values)\n\
      :Done(values) => values.\n\
    #Sweep(left<[f64]:2,3>)\n";
    for artifact in artifacts(source) {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert!(
            !emit_program_object(&artifact, &mech_stdlib::source_catalog())
                .unwrap()
                .bytes()
                .is_empty()
        );
        check_restored_image(&jit);
        let frames = jit.descriptor().plan().facts().buffer_frames();
        assert!(frames.managed_fields() > 0 && frames.scalar_fields() > 0);
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let args = inputs(
            &artifact,
            &[("left", 2, 3, &[1., 2., 3., 4., 5., 6.])],
            &memory,
        );
        let aliases = args.clone();
        let original = args[0].canonical_data_draft().unwrap();
        let bad = inputs(
            &artifact,
            &[("left", 2, 3, &[-1., 2., 3., 4., 5., 6.])],
            &memory,
        );
        let mut output = vec![None; artifact.outputs().len()];
        let report = jit
            .evaluate_into(&args, &mut output, limits(), &memory)
            .unwrap();
        check_output(&artifact, &output, [2, 3], &[1., 2., 3., 4.5, 6.5, 8.5]);
        let prior = output.clone();
        let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
        let mut work_denied = limits();
        work_denied.execution.max_work = report.work / 2;
        let mut output_denied = limits();
        output_denied.library.max_output_elements = Some(1);
        let mut clone_denied = limits();
        clone_denied.library.max_cloned_bytes = Some(0);
        for (arguments, admitted) in [
            (&bad, limits()),
            (&args, work_denied),
            (&args, output_denied),
            (&args, clone_denied),
        ] {
            assert!(
                jit.evaluate_into(arguments, &mut output, admitted, &memory)
                    .is_err()
            );
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                retained
            );
            assert!(output.iter().zip(&prior).all(|(a, b)| {
                a.as_ref()
                    .unwrap()
                    .shares_frozen_storage(b.as_ref().unwrap())
            }));
            assert!(args[0].shares_frozen_storage(&aliases[0]));
            assert_eq!(aliases[0].canonical_data_draft().unwrap(), original);
        }
        jit.evaluate_into(&args, &mut output, limits(), &memory)
            .unwrap();
        check_output(&artifact, &output, [2, 3], &[1., 2., 3., 4.5, 6.5, 8.5]);
        drop((args, aliases, bad, prior, output));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

#[test]
fn boolean_and_signed_matrices_use_native_frames_and_checked_word_reads() {
    for (kind, cells) in [
        (
            "i64",
            vec![
                Data::I64(i64::MIN),
                Data::I64(-7),
                Data::I64(0),
                Data::I64(i64::MAX),
                Data::I64(12),
                Data::I64(-9),
            ],
        ),
        (
            "bool",
            vec![
                Data::Bool(false),
                Data::Bool(true),
                Data::Bool(false),
                Data::Bool(true),
                Data::Bool(false),
                Data::Bool(true),
            ],
        ),
    ] {
        for (expression, prefix) in [
            ("values[at<u64>]", String::new()),
            ("values[row<u64>,column<u64>]", String::new()),
            (
                "read-cell(values, 3u64, at<u64>)",
                format!(
                    "read-cell(values<[{kind}]>, remaining<u64>, at<u64>) => <{kind}>\n\
                      | (values, remaining, at) => ((remaining > 0u64)?\n\
                          | true => read-cell(values, remaining - 1u64, at)\n\
                          | false => values[at]).\n"
                ),
            ),
        ] {
            let source = format!(
                "{prefix}frame := (left<[{kind}]>, (1.5, 7u64))\n(values, state) := frame\n({expression}, state.1 + 1.0, state.2)\n"
            );
            for artifact in artifacts(&source) {
                let jit =
                    JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
                assert!(
                    !emit_program_object(&artifact, &mech_stdlib::source_catalog())
                        .unwrap()
                        .bytes()
                        .is_empty()
                );
                check_restored_image(&jit);
                let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
                let mut output = vec![None; artifact.outputs().len()];
                let make_args = |shape, at, row, column| {
                    typed_inputs(
                        &artifact,
                        &[
                            ("left", shape, Data::Matrix(cells.clone().into())),
                            ("at", [1, 1], Data::U64(at)),
                            ("row", [1, 1], Data::U64(row)),
                            ("column", [1, 1], Data::U64(column)),
                        ],
                        &memory,
                    )
                };
                for (shape, expected) in [
                    (
                        [2, 3],
                        if expression.contains("at") {
                            &cells[3]
                        } else {
                            &cells[4]
                        },
                    ),
                    ([1, 6], &cells[1]),
                ] {
                    let args = make_args(shape, 2, if shape[0] == 2 { 2 } else { 1 }, 2);
                    let aliases = args.clone();
                    jit.evaluate_into(&args, &mut output, limits(), &memory)
                        .unwrap();
                    let expected = Data::Tuple(
                        vec![
                            expected.clone(),
                            Data::F64(F64Bits::from_f64(2.5)),
                            Data::U64(7),
                        ]
                        .into(),
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
                    assert!(
                        args.iter()
                            .zip(&aliases)
                            .all(|(a, b)| a.shares_frozen_storage(b))
                    );
                    let prior = output.clone();
                    let bad = make_args(shape, 0, 0, 2);
                    let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    assert!(
                        jit.evaluate_into(&bad, &mut output, limits(), &memory)
                            .is_err()
                    );
                    let mut denied = limits();
                    denied.library.max_output_bytes = Some(0);
                    assert!(
                        jit.evaluate_into(&args, &mut output, denied, &memory)
                            .is_err()
                    );
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        retained
                    );
                    assert!(output.iter().zip(&prior).all(|(a, b)| {
                        a.as_ref()
                            .unwrap()
                            .shares_frozen_storage(b.as_ref().unwrap())
                    }));
                    jit.evaluate_into(&args, &mut output, limits(), &memory)
                        .unwrap();
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
                drop(output);
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
        }
    }
}

#[test]
fn boolean_and_signed_fsm_matrix_updates_keep_aliases_bounds_and_recovery() {
    for (kind, expression, cells, expected) in [
        (
            "i64",
            "values[2u64,step + 1u64] - 1i64",
            vec![
                Data::I64(i64::MIN),
                Data::I64(-7),
                Data::I64(0),
                Data::I64(-2),
                Data::I64(0),
                Data::I64(i64::MAX),
            ],
            vec![
                Data::I64(i64::MIN),
                Data::I64(-7),
                Data::I64(0),
                Data::I64(-3),
                Data::I64(-1),
                Data::I64(i64::MAX - 1),
            ],
        ),
        (
            "bool",
            "(values[2u64,step + 1u64]? | true => false | false => true)",
            vec![
                Data::Bool(false),
                Data::Bool(true),
                Data::Bool(false),
                Data::Bool(true),
                Data::Bool(false),
                Data::Bool(true),
            ],
            vec![
                Data::Bool(false),
                Data::Bool(true),
                Data::Bool(false),
                Data::Bool(false),
                Data::Bool(true),
                Data::Bool(false),
            ],
        ),
    ] {
        let source = format!(
            "#Sweep(initial<[{kind}]:2,3>, target<u64>) => <[{kind}]:2,3>\n\
          | :Next(values<[{kind}]:2,3>, step<u64>)\n\
          | :Done(values<[{kind}]:2,3>).\n\
        #Sweep(initial, target) -> :Next(initial, 0u64)\n\
          :Next(values, step)\n\
            | step < 3u64 -> {{\n\
                ~next := values\n\
                next[target,step + 1u64] = {expression}\n\
              }} -> :Next(next, step + 1u64)\n\
            | * -> :Done(values)\n\
          :Done(values) => values.\n\
        #Sweep(left<[{kind}]:2,3>, row<u64>)\n"
        );
        for artifact in artifacts(&source) {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            assert!(
                !emit_program_object(&artifact, &mech_stdlib::source_catalog())
                    .unwrap()
                    .bytes()
                    .is_empty()
            );
            check_restored_image(&jit);
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let make_args = |row| {
                typed_inputs(
                    &artifact,
                    &[
                        ("left", [2, 3], Data::Matrix(cells.clone().into())),
                        ("row", [1, 1], Data::U64(row)),
                    ],
                    &memory,
                )
            };
            let args = make_args(2);
            let aliases = args.clone();
            let bad = make_args(3);
            let mut output = vec![None; artifact.outputs().len()];
            let report = jit
                .evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            assert_eq!(
                output
                    .last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                Data::Matrix(expected.clone().into())
            );
            let prior = output.clone();
            let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
            let mut denied = limits();
            denied.execution.max_work = report.work / 2;
            for (arguments, admitted) in [(&bad, limits()), (&args, denied)] {
                assert!(
                    jit.evaluate_into(arguments, &mut output, admitted, &memory)
                        .is_err()
                );
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    retained
                );
                assert!(output.iter().zip(&prior).all(|(a, b)| {
                    a.as_ref()
                        .unwrap()
                        .shares_frozen_storage(b.as_ref().unwrap())
                }));
                assert!(args[0].shares_frozen_storage(&aliases[0]));
                assert_eq!(
                    aliases[0].canonical_data_draft().unwrap(),
                    Data::Matrix(cells.clone().into())
                );
            }
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            assert_eq!(
                output
                    .last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                Data::Matrix(expected.clone().into())
            );
            drop((args, aliases, bad, prior, output));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn incompatible_broadcasts_preserve_outputs_and_recover() {
    for artifact in artifacts("left<[f64]> + right<[f64]>\n") {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let good = inputs(
            &artifact,
            &[
                ("left", 2, 3, &[1., 2., 3., 4., 5., 6.]),
                ("right", 1, 3, &[1., 2., 3.]),
            ],
            &memory,
        );
        let bad = inputs(
            &artifact,
            &[
                ("left", 2, 3, &[1., 2., 3., 4., 5., 6.]),
                ("right", 2, 2, &[1., 2., 3., 4.]),
            ],
            &memory,
        );
        let mut output = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&good, &mut output, limits(), &memory)
            .unwrap();
        let prior = output.clone();
        let retained = memory.used_bytes();
        assert!(
            jit.evaluate_into(&bad, &mut output, limits(), &memory)
                .is_err()
        );
        assert_eq!(memory.used_bytes(), retained);
        assert!(output.iter().zip(&prior).all(|(a, b)| {
            a.as_ref()
                .unwrap()
                .shares_frozen_storage(b.as_ref().unwrap())
        }));
        jit.evaluate_into(&good, &mut output, limits(), &memory)
            .unwrap();
        check_output(&artifact, &output, [2, 3], &[2., 4., 6., 5., 7., 9.]);
    }
}

#[test]
fn matrix_geometry_and_finite_budget_rejections_preserve_output_and_recover() {
    for artifact in artifacts("left<[f64]> ** right<[f64]>\n") {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let good = inputs(
            &artifact,
            &[
                ("left", 2, 3, &[1., 2., 3., 4., 5., 6.]),
                ("right", 3, 1, &[1., 2., 3.]),
            ],
            &memory,
        );
        let bad = inputs(
            &artifact,
            &[
                ("left", 2, 3, &[1., 2., 3., 4., 5., 6.]),
                ("right", 2, 1, &[1., 2.]),
            ],
            &memory,
        );
        let mut output = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&good, &mut output, limits(), &memory)
            .unwrap();
        let previous = output.clone();
        let retained = memory.used_bytes();
        for (args, policy) in [
            (&bad, limits()),
            (
                &good,
                NativeProgramExecutionLimits {
                    library: MemoryBudgetLimits {
                        max_compute_work: Some(1),
                        ..limits().library
                    },
                    ..limits()
                },
            ),
        ] {
            assert!(
                jit.evaluate_into(args, &mut output, policy, &memory)
                    .is_err()
            );
            assert_eq!(memory.used_bytes(), retained);
            assert!(output.iter().zip(&previous).all(|(a, b)| {
                a.as_ref()
                    .unwrap()
                    .shares_frozen_storage(b.as_ref().unwrap())
            }));
        }
        jit.evaluate_into(&good, &mut output, limits(), &memory)
            .unwrap();
        check_output(&artifact, &output, [2, 1], &[14., 32.]);
        let empty = inputs(
            &artifact,
            &[("left", 0, 3, &[]), ("right", 3, 1, &[1., 2., 3.])],
            &memory,
        );
        jit.evaluate_into(&empty, &mut output, limits(), &memory)
            .unwrap();
        check_output(&artifact, &output, [0, 1], &[]);
    }
}

#[test]
fn rectangular_cell_replacement_checks_axes_and_preserves_inputs_and_outputs() {
    let source = "replace(source<[f64]>, value<f64>) => (out<[f64]>) :=\n  ~next := source\n  next[2u64,3u64] = value\n  out := next.\nreplace(source<[f64]>, value<f64>)\n";
    for artifact in artifacts(source) {
        let catalog = mech_stdlib::source_catalog();
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        check_restored_image(&jit);
        assert!(
            !emit_program_object(&artifact, &catalog)
                .unwrap()
                .bytes()
                .is_empty()
        );
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let good = inputs(
            &artifact,
            &[
                ("source", 2, 3, &[1., 2., 3., 4., 5., 6.]),
                ("value", 1, 1, &[9.]),
            ],
            &memory,
        );
        let aliases = good.clone();
        let mut output = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&good, &mut output, limits(), &memory)
            .unwrap();
        check_output(&artifact, &output, [2, 3], &[1., 2., 3., 4., 5., 9.]);
        for (rows, cols, data) in [(3, 2, vec![1.; 6]), (0, 3, vec![])] {
            let bad = inputs(
                &artifact,
                &[("source", rows, cols, &data), ("value", 1, 1, &[9.])],
                &memory,
            );
            let prior = output.clone();
            let retained = memory.used_bytes();
            assert!(
                jit.evaluate_into(&bad, &mut output, limits(), &memory)
                    .is_err()
            );
            assert_eq!(memory.used_bytes(), retained);
            assert!(output.iter().zip(&prior).all(|(a, b)| {
                a.as_ref()
                    .unwrap()
                    .shares_frozen_storage(b.as_ref().unwrap())
            }));
        }
        let prior = output.clone();
        let retained = memory.used_bytes();
        let denied = NativeProgramExecutionLimits {
            library: MemoryBudgetLimits {
                max_compute_work: Some(1),
                ..limits().library
            },
            ..limits()
        };
        assert!(
            jit.evaluate_into(&good, &mut output, denied, &memory)
                .is_err()
        );
        assert_eq!(memory.used_bytes(), retained);
        assert!(output.iter().zip(&prior).all(|(a, b)| {
            a.as_ref()
                .unwrap()
                .shares_frozen_storage(b.as_ref().unwrap())
        }));
        assert!(
            good.iter()
                .zip(&aliases)
                .all(|(a, b)| a.shares_frozen_storage(b))
        );
        jit.evaluate_into(&good, &mut output, limits(), &memory)
            .unwrap();
        check_output(&artifact, &output, [2, 3], &[1., 2., 3., 4., 5., 9.]);
        drop((good, aliases, output, prior));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );

        let entry = catalog
            .resident_factory(&["core".into(), "assign".into()], "indexed-rectangle")
            .unwrap();
        let mut builder = FunctionCatalogBuilder::new();
        builder
            .insert_resident_factory_entry(ResidentKernelFactoryEntry::new(
                entry.key().clone(),
                entry.implementation_memory(),
                entry.factory(),
            ))
            .unwrap();
        let forged = builder.build().unwrap();
        assert!(
            format!(
                "{:?}",
                JitProgramArtifact::compile(&artifact, &forged)
                    .err()
                    .unwrap()
            )
            .contains("MissingSemanticWitness")
        );
    }
}

#[test]
fn maintained_real_power_supports_scalars_and_matrix_broadcasts() {
    for (source, expected) in [
        (
            "+> math\nmath/pow(left<[f64]>, right<f64>)\n",
            vec![1., 4., 9., 16., 25., 36.],
        ),
        ("+> math\nmath/pow(left<f64>, right<f64>)\n", vec![4.]),
    ] {
        for artifact in artifacts(source) {
            let catalog = mech_stdlib::source_catalog();
            let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
            check_restored_image(&jit);
            assert!(
                !emit_program_object(&artifact, &catalog)
                    .unwrap()
                    .bytes()
                    .is_empty()
            );
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let matrix = matches!(
                artifact
                    .schemas()
                    .get(
                        artifact
                            .inputs()
                            .iter()
                            .find(
                                |i| mech_engine::decode_source_input_name(&i.name).as_deref()
                                    == Some("left")
                            )
                            .unwrap()
                            .schema
                    )
                    .unwrap()
                    .body(),
                SchemaBody::Matrix { .. }
            );
            let args = if matrix {
                inputs(
                    &artifact,
                    &[
                        ("left", 2, 3, &[1., 2., 3., 4., 5., 6.]),
                        ("right", 1, 1, &[2.]),
                    ],
                    &memory,
                )
            } else {
                inputs(
                    &artifact,
                    &[("left", 1, 1, &[2.]), ("right", 1, 1, &[2.])],
                    &memory,
                )
            };
            let mut output = vec![None; artifact.outputs().len()];
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            if matrix {
                check_output(&artifact, &output, [2, 3], &expected);
            } else {
                assert!(
                    matches!(output.last().unwrap().as_ref().unwrap().canonical_data_draft().unwrap(), Data::F64(v) if v.to_f64() == expected[0])
                );
            }
        }
    }
}

#[test]
fn copied_public_factory_metadata_cannot_authorize_native_matrix_lowering() {
    for (source, module, name) in [
        ("left<[f64]> ** right<[f64]>\n", "matrix", "matmul"),
        ("left<[f64]> + right<[f64]>\n", "math", "add"),
        ("left<[f64]> - right<[f64]>\n", "math", "sub"),
        ("left<[f64]> * right<[f64]>\n", "math", "mul"),
        ("left<[f64]> / right<[f64]>\n", "math", "div"),
        ("+> math\nmath/pow(left<f64>, right<f64>)\n", "math", "pow"),
        ("+> math\nmath/sqrt(left<[f64]>)\n", "math", "sqrt"),
        ("a := left<[f64]>\na[1u64,1u64]\n", "access", "scalar"),
    ] {
        let artifact = artifacts(source).into_iter().next().unwrap();
        let catalog = mech_stdlib::source_catalog();
        let entry = catalog.resident_factory(&[module.into()], name).unwrap();
        let mut builder = FunctionCatalogBuilder::new();
        builder
            .insert_resident_factory_entry(ResidentKernelFactoryEntry::new(
                entry.key().clone(),
                entry.implementation_memory(),
                entry.factory(),
            ))
            .unwrap();
        let forged = builder.build().unwrap();
        let error = JitProgramArtifact::compile(&artifact, &forged)
            .err()
            .expect("private witness is required");
        assert!(format!("{error:?}").contains("MissingSemanticWitness"));
    }
}
