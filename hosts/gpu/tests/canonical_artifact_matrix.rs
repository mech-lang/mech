//! Maintained resident matrix leaves through the canonical Cranelift program.
use mech_core::{
    FunctionCatalogBuilder, ManagedMemoryBudget, MemoryBudgetLimits, ResidentKernelFactoryEntry,
    SchemaBody, Value, ValueDataDraft as Data,
    snapshot::{F64Bits, SnapshotValidationContext, ValueDraft},
};
use mech_engine::{CanonicalSourceFrontend, ProgramArtifact};
use mech_gpu::native_artifact::{
    JitProgramArtifact, NativeExecutionLimits, NativeProgramExecutionLimits, emit_program_object,
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
            let value = ValueDraft {
                schema: input.schema,
                shape_values: mech_core::shape_for_resolved_extents(schema, &[*rows, *columns])
                    .unwrap()
                    .parameter_values()
                    .into(),
                data: Data::Matrix(
                    numbers
                        .iter()
                        .map(|v| Data::F64(F64Bits::from_f64(*v)))
                        .collect(),
                ),
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
fn copied_public_factory_metadata_cannot_authorize_native_matrix_lowering() {
    let artifact = artifacts("left<[f64]> ** right<[f64]>\n")
        .into_iter()
        .next()
        .unwrap();
    let catalog = mech_stdlib::source_catalog();
    let entry = catalog
        .resident_factory(&["matrix".into()], "matmul")
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
    let error = JitProgramArtifact::compile(&artifact, &forged)
        .err()
        .expect("private witness is required");
    assert!(format!("{error:?}").contains("MissingSemanticWitness"));
}
