//! Focused evaluation-only frame measurement through maintained public APIs.
use mech_core::{
    ManagedMemoryBudget, Value, ValueDataDraft as Data,
    snapshot::{F64Bits, SnapshotValidationContext, ValueDraft},
};
use mech_engine::{
    CanonicalSourceFrontend, ProgramArtifact,
    native_artifact::{NativeScalarValue as Scalar, lower_pure_native_artifact},
};
use mech_gpu::native_artifact::{
    JitArtifact, JitProgramArtifact, NativeExecutionLimits, NativeProgramExecutionLimits,
};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};
use sha2::{Digest, Sha256};
use std::{hint::black_box, sync::Arc, time::Instant};

#[path = "../../../src/core/tests/support/allocation_probe.rs"]
mod allocation_probe;
#[global_allocator]
static ALLOCATOR: allocation_probe::ProbeAllocator = allocation_probe::ProbeAllocator;

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn artifact(source: &str, decoded: bool) -> ProgramArtifact {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x4652414d), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
    let original = CanonicalSourceFrontend
        .compile_document_with_catalog(
            &DocumentSyntax::cast(parsed.syntax()).unwrap(),
            mech_stdlib::source_catalog(),
        )
        .unwrap()
        .compile_artifact()
        .unwrap();
    if decoded {
        mech_engine::decode_program_artifact_bytecode_v1(
            &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
        )
        .unwrap()
    } else {
        original
    }
}

fn inputs(
    artifact: &ProgramArtifact,
    named: &[(&str, Data)],
    memory: &ManagedMemoryBudget,
) -> Vec<Value> {
    let schemas = Arc::new(artifact.schemas().clone());
    artifact
        .inputs()
        .iter()
        .map(|input| {
            let name = mech_engine::decode_source_input_name(&input.name).unwrap();
            let data = named
                .iter()
                .find(|(key, _)| *key == name)
                .unwrap()
                .1
                .clone();
            let shape_values: Box<[u64]> = if let Data::Matrix(values) = &data {
                mech_core::shape_for_resolved_extents(
                    schemas.get(input.schema).unwrap(),
                    &[1, values.len() as u64],
                )
                .unwrap()
                .parameter_values()
                .into()
            } else {
                Box::new([])
            };
            let value = ValueDraft {
                schema: input.schema,
                shape_values,
                data,
            }
            .finalize(&SnapshotValidationContext::with_shared_schemas(&schemas))
            .unwrap();
            let mut admission = memory
                .reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap())
                .unwrap();
            value.into_memory_budget(&mut admission).unwrap()
        })
        .collect()
}

fn policy() -> NativeProgramExecutionLimits {
    let mut library = mech_core::TargetMemoryProfile::current_resident_cpu()
        .unwrap()
        .limits;
    library.max_compute_work = Some(1_000_000_000_000);
    library.max_comparison_work = Some(1_000_000_000);
    library.max_temporary_bytes = Some(128 * 1024 * 1024);
    NativeProgramExecutionLimits {
        execution: NativeExecutionLimits {
            max_work: 1_000_000_000_000,
            ..Default::default()
        },
        library,
    }
}

fn managed(case: &str, decoded: bool, steps: u64, samples: usize, repetitions: usize) {
    let body = "walk(words<[u32]>, left<u64>, total<u64>) => <u64>\n  | (words, left, total) => ((left > 0u64)? | true => walk(words, left - 1u64, total + 3u64) | false => sequence/length(words) + total).\n";
    let source = match case {
        "exclusive" => format!("+> sequence\n{body}walk(sequence/with-capacity(0u32, 64u64), steps<u64>, seed<u64>)\n"),
        "aliased" => format!("+> sequence\n{body}walk(values<[u32]>, steps<u64>, seed<u64>)\n"),
        "scalar-loop" => "walk(left<u64>, total<u64>) => <u64>\n  | (left, total) => ((left > 0u64)? | true => walk(left - 1u64, total + 3u64) | false => total).\nwalk(steps<u64>, seed<u64>)\n".to_owned(),
        _ => panic!("unknown managed case"),
    };
    let artifact = artifact(&source, decoded);
    let encoded = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
    let hash = digest(&encoded);
    let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
    let frames = jit.descriptor().plan().facts().buffer_frames().elements();
    assert_eq!(frames > 0, case != "scalar-loop");
    let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
    let args = inputs(
        &artifact,
        &[
            ("steps", Data::U64(steps)),
            ("seed", Data::U64(17)),
            ("values", Data::Matrix((0..64).map(Data::U32).collect())),
        ],
        &memory,
    );
    let aliases = args.clone();
    let retained = memory.used_bytes();
    let expected = Data::U64(17 + steps * 3 + if case == "aliased" { 64 } else { 0 });
    let policy = policy();
    for sample in 0..=samples {
        // Output containers and all previous publications stay outside timing.
        let mut outputs = (0..repetitions)
            .map(|_| vec![None; artifact.outputs().len()])
            .collect::<Vec<_>>();
        let ((elapsed, last), allocations, bytes) = allocation_probe::measured_with_bytes(|| {
            let start = Instant::now();
            let mut last = None;
            for output in &mut outputs {
                last = Some(
                    jit.evaluate_into(black_box(&args), output, policy, &memory)
                        .unwrap(),
                );
            }
            (start.elapsed().as_secs_f64(), last.unwrap())
        });
        for output in &outputs {
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
        assert!(
            args.iter()
                .zip(&aliases)
                .all(|(a, b)| a.shares_frozen_storage(b))
        );
        println!(
            "{}",
            serde_json::json!({"case":case,"form":if decoded {"decoded"} else {"original"},"steps":steps,"sample":sample,"warmup":sample==0,"invocations":repetitions,"seconds":elapsed,"allocation_requests":allocations,"requested_bytes":bytes,"work_per_invocation":last.work,"visits_per_invocation":last.visits,"private_frame_fields":frames,"artifact_sha256":hash})
        );
        drop(outputs);
        assert_eq!(memory.used_bytes(), retained);
    }
    drop((args, aliases));
    assert_eq!(memory.used_bytes(), 0);
    assert_eq!(memory.retained_snapshot_nodes(), 0);
}

fn scalar(case: &str, decoded: bool, samples: usize, repetitions: usize) {
    let (kind, op, left, right, expected) = match case {
        "scalar-u64" => ("u64", "+", Scalar::U64(41), Scalar::U64(1), Scalar::U64(42)),
        "scalar-i64" => (
            "i64",
            "+",
            Scalar::I64(-41),
            Scalar::I64(1),
            Scalar::I64(-40),
        ),
        "scalar-f64" => (
            "f64",
            "+",
            Scalar::F64(F64Bits::from_f64(1.25)),
            Scalar::F64(F64Bits::from_f64(3.5)),
            Scalar::F64(F64Bits::from_f64(4.75)),
        ),
        "scalar-bool" => (
            "bool",
            "&&",
            Scalar::Bool(true),
            Scalar::Bool(false),
            Scalar::Bool(false),
        ),
        _ => panic!("unknown scalar case"),
    };
    let artifact = artifact(&format!("left<{kind}> {op} right<{kind}>\n"), decoded);
    let hash = digest(&mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap());
    let plan = lower_pure_native_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
    let jit = JitArtifact::compile(&plan).unwrap();
    let args = artifact
        .inputs()
        .iter()
        .map(|input| {
            match mech_engine::decode_source_input_name(&input.name)
                .unwrap()
                .as_str()
            {
                "left" => left,
                "right" => right,
                _ => panic!(),
            }
        })
        .collect::<Vec<_>>();
    let mut output = vec![expected; artifact.outputs().len()];
    for sample in 0..=samples {
        let (elapsed, allocations, bytes) = allocation_probe::measured_with_bytes(|| {
            let start = Instant::now();
            for _ in 0..repetitions {
                jit.evaluate_into(
                    black_box(&args),
                    black_box(&mut output),
                    NativeExecutionLimits::default(),
                    None,
                )
                .unwrap();
            }
            start.elapsed().as_secs_f64()
        });
        assert_eq!(output.last(), Some(&expected));
        println!(
            "{}",
            serde_json::json!({"case":case,"form":if decoded {"decoded"} else {"original"},"sample":sample,"warmup":sample==0,"invocations":repetitions,"seconds":elapsed,"allocation_requests":allocations,"requested_bytes":bytes,"artifact_sha256":hash,"scalar_operations":jit.descriptor().lets().len()})
        );
    }
}

fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    assert_eq!(
        args.len(),
        6,
        "CASE original|decoded STEPS SAMPLES REPETITIONS"
    );
    let decoded = match args[2].as_str() {
        "original" => false,
        "decoded" => true,
        _ => panic!("unknown artifact form"),
    };
    let steps = args[3].parse().unwrap();
    let samples = args[4].parse().unwrap();
    let repetitions = args[5].parse::<usize>().unwrap();
    assert!(repetitions > 0);
    assert_eq!(allocation_probe::measured(|| ()).1, 0);
    match args[1].as_str() {
        "exclusive" | "aliased" | "scalar-loop" => {
            managed(&args[1], decoded, steps, samples, repetitions)
        }
        _ => scalar(&args[1], decoded, samples, repetitions),
    }
}
