//! Full MINI live-outs from unmodified PolyBench/C kernels, without a C toolchain at test time.
use mech_core::{
    ManagedMemoryBudget, MemoryBudgetLimits, Value, ValueDataDraft as Data,
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
use serde_json::Value as Json;
use sha2::{Digest, Sha256};

const SOURCES: &[(&str, &str)] = &[
    ("2mm", include_str!("fixtures/polybench/2mm.mec")),
    ("3mm", include_str!("fixtures/polybench/3mm.mec")),
    ("adi", include_str!("fixtures/polybench/adi.mec")),
    ("atax", include_str!("fixtures/polybench/atax.mec")),
    ("bicg", include_str!("fixtures/polybench/bicg.mec")),
    ("cholesky", include_str!("fixtures/polybench/cholesky.mec")),
    (
        "correlation",
        include_str!("fixtures/polybench/correlation.mec"),
    ),
    (
        "covariance",
        include_str!("fixtures/polybench/covariance.mec"),
    ),
    ("deriche", include_str!("fixtures/polybench/deriche.mec")),
    ("doitgen", include_str!("fixtures/polybench/doitgen.mec")),
    ("durbin", include_str!("fixtures/polybench/durbin.mec")),
    ("fdtd-2d", include_str!("fixtures/polybench/fdtd-2d.mec")),
    (
        "floyd-warshall",
        include_str!("fixtures/polybench/floyd-warshall.mec"),
    ),
    ("gemm", include_str!("fixtures/polybench/gemm.mec")),
    ("gemver", include_str!("fixtures/polybench/gemver.mec")),
    ("gesummv", include_str!("fixtures/polybench/gesummv.mec")),
    (
        "gramschmidt",
        include_str!("fixtures/polybench/gramschmidt.mec"),
    ),
    ("heat-3d", include_str!("fixtures/polybench/heat-3d.mec")),
    (
        "jacobi-1d",
        include_str!("fixtures/polybench/jacobi-1d.mec"),
    ),
    (
        "jacobi-2d",
        include_str!("fixtures/polybench/jacobi-2d.mec"),
    ),
    ("lu", include_str!("fixtures/polybench/lu.mec")),
    ("ludcmp", include_str!("fixtures/polybench/ludcmp.mec")),
    ("mvt", include_str!("fixtures/polybench/mvt.mec")),
    ("nussinov", include_str!("fixtures/polybench/nussinov.mec")),
    (
        "seidel-2d",
        include_str!("fixtures/polybench/seidel-2d.mec"),
    ),
    ("symm", include_str!("fixtures/polybench/symm.mec")),
    ("syr2k", include_str!("fixtures/polybench/syr2k.mec")),
    ("syrk", include_str!("fixtures/polybench/syrk.mec")),
    ("trisolv", include_str!("fixtures/polybench/trisolv.mec")),
    ("trmm", include_str!("fixtures/polybench/trmm.mec")),
];

fn limits() -> NativeProgramExecutionLimits {
    let cap = 1_000_000_000_000_000;
    let bytes = 256 * 1024 * 1024;
    NativeProgramExecutionLimits {
        execution: NativeExecutionLimits {
            max_work: cap,
            max_transport_bytes: bytes,
            max_native_stack_bytes: 8 * 1024 * 1024,
        },
        library: MemoryBudgetLimits {
            max_output_elements: Some(1_000_000),
            max_output_bytes: Some(bytes),
            max_temporary_bytes: Some(bytes),
            max_cloned_bytes: Some(cap),
            max_retained_nodes: Some(1_000_000),
            max_comparison_work: Some(cap),
            max_compute_work: Some(cap),
            max_scalar_instructions: Some(cap),
            max_transfer_bytes: Some(cap),
            max_storage_buffer_bytes: Some(bytes),
            max_storage_bindings: Some(1024),
        },
    }
}

fn inputs(artifact: &ProgramArtifact, case: &Json, memory: &ManagedMemoryBudget) -> Vec<Value> {
    artifact
        .inputs()
        .iter()
        .map(|input| {
            let name = mech_engine::decode_source_input_name(&input.name).unwrap();
            let spec = &case["inputs"][name];
            let schema = artifact.schemas().get(input.schema).unwrap();
            let (data, shape_values) = if let Some(values) = spec["values"].as_array() {
                let extents = spec["shape"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_u64().unwrap())
                    .collect::<Vec<_>>();
                (
                    Data::Matrix(
                        values
                            .iter()
                            .map(|v| Data::F64(F64Bits::from_f64(v.as_f64().unwrap())))
                            .collect(),
                    ),
                    mech_core::shape_for_resolved_extents(schema, &extents)
                        .unwrap()
                        .parameter_values()
                        .into(),
                )
            } else if let Some(value) = spec["u64"].as_u64() {
                (Data::U64(value), Box::new([]) as Box<[u64]>)
            } else {
                (
                    Data::F64(F64Bits::from_f64(spec.as_f64().unwrap())),
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

fn flatten(data: &Data, output: &mut Vec<f64>) {
    match data {
        Data::F64(value) => output.push(value.to_f64()),
        Data::Matrix(values) | Data::Tuple(values) => {
            for value in values.iter() {
                flatten(value, output)
            }
        }
        _ => panic!("unexpected PolyBench result: {data:?}"),
    }
}

fn check_case(name: &str, source: &str) {
    let fixture: Json =
        serde_json::from_str(include_str!("fixtures/polybench/cases.json")).unwrap();
    assert_eq!(fixture["schema"], "mech.polybench.native-mini.v1");
    let cases = fixture["cases"].as_array().unwrap();
    assert_eq!(cases.len(), SOURCES.len());
    let catalog = mech_stdlib::source_catalog();
    let case = cases.iter().find(|case| case["name"] == name).unwrap();
    assert_eq!(
        case["mech_sha256"].as_str().unwrap(),
        Sha256::digest(source.as_bytes())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    );
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x504f4c59), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(
        parsed.is_strictly_clean(),
        "{name}: {:?}",
        parsed.diagnostics
    );
    let original = CanonicalSourceFrontend
        .compile_document_with_catalog(
            &DocumentSyntax::cast(parsed.syntax()).unwrap(),
            catalog.clone(),
        )
        .unwrap()
        .compile_artifact()
        .unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    for (form, artifact) in ["original", "decoded"].into_iter().zip([original, decoded]) {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        assert!(
            !emit_program_object(&artifact, &catalog)
                .unwrap()
                .bytes()
                .is_empty()
        );
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let args = inputs(&artifact, case, &memory);
        let aliases = args.clone();
        let mut output = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut output, limits(), &memory)
            .unwrap_or_else(|error| panic!("{name}/{form}: {error:?}"));
        let mut actual = Vec::new();
        flatten(
            &output
                .last()
                .unwrap()
                .as_ref()
                .unwrap()
                .canonical_data_draft()
                .unwrap(),
            &mut actual,
        );
        let expected = case["expected"].as_array().unwrap();
        assert_eq!(actual.len(), expected.len(), "{name}/{form}");
        for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
            let expected = expected.as_f64().unwrap();
            let error = (actual - expected).abs();
            assert!(
                actual.is_finite()
                    && expected.is_finite()
                    && (error <= 1e-9 || error <= 1e-10 * actual.abs().max(expected.abs())),
                "{name}/{form}[{index}]: {actual} != {expected}"
            );
        }
        assert!(
            args.iter()
                .zip(&aliases)
                .all(|(a, b)| a.shares_frozen_storage(b))
        );
        println!("{name}/{form}: {} finite C outputs matched", actual.len());
        drop((args, aliases, output));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

macro_rules! mini_case {
    ($test:ident, $name:literal) => {
        #[test]
        fn $test() {
            let (_, source) = SOURCES.iter().find(|(name, _)| *name == $name).unwrap();
            check_case($name, source);
        }
    };
}

mini_case!(mini_2mm, "2mm");
mini_case!(mini_3mm, "3mm");
mini_case!(mini_adi, "adi");
mini_case!(mini_atax, "atax");
mini_case!(mini_bicg, "bicg");
mini_case!(mini_cholesky, "cholesky");
mini_case!(mini_correlation, "correlation");
mini_case!(mini_covariance, "covariance");
mini_case!(mini_deriche, "deriche");
mini_case!(mini_doitgen, "doitgen");
mini_case!(mini_durbin, "durbin");
mini_case!(mini_fdtd_2d, "fdtd-2d");
mini_case!(mini_floyd_warshall, "floyd-warshall");
mini_case!(mini_gemm, "gemm");
mini_case!(mini_gemver, "gemver");
mini_case!(mini_gesummv, "gesummv");
mini_case!(mini_gramschmidt, "gramschmidt");
mini_case!(mini_heat_3d, "heat-3d");
mini_case!(mini_jacobi_1d, "jacobi-1d");
mini_case!(mini_jacobi_2d, "jacobi-2d");
mini_case!(mini_lu, "lu");
mini_case!(mini_ludcmp, "ludcmp");
mini_case!(mini_mvt, "mvt");
mini_case!(mini_nussinov, "nussinov");
mini_case!(mini_seidel_2d, "seidel-2d");
mini_case!(mini_symm, "symm");
mini_case!(mini_syr2k, "syr2k");
mini_case!(mini_syrk, "syrk");
mini_case!(mini_trisolv, "trisolv");
mini_case!(mini_trmm, "trmm");
