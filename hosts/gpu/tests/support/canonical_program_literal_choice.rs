//! Exact literal-only frequencies/codes and fixed code tables.
use super::{artifacts, inputs, limits};
use mech_core::{ManagedMemoryBudget, ValueDataDraft as Data};
use mech_gpu::native_artifact::{JitProgramArtifact, NativeCompileLimits};

const COMPONENTS: &str = concat!(
    include_str!("../fixtures/native/deflate-huffman-level12.mec"),
    "\n",
    include_str!("../fixtures/native/deflate-literal-choice.mec"),
    "\n",
    include_str!("../fixtures/native/deflate-static-codes.mec"),
    "\n"
);
fn matrix(value: &serde_json::Value, byte: bool) -> Data {
    Data::Matrix(
        value
            .as_array()
            .unwrap()
            .iter()
            .map(|v| {
                if byte {
                    Data::U8(v.as_u64().unwrap() as u8)
                } else {
                    Data::U32(v.as_u64().unwrap() as u32)
                }
            })
            .collect(),
    )
}
fn check(value: &mech_core::Value, case: &serde_json::Value) {
    let Data::Record(fields) = value.canonical_data_draft().unwrap() else {
        panic!()
    };
    for (key, expected) in case["expected"].as_object().unwrap() {
        let field = fields
            .iter()
            .find(|f| f.name == key.replace('_', "-"))
            .unwrap();
        assert_eq!(
            field.value,
            matrix(expected, key.ends_with("lens")),
            "{} {key}",
            case["name"]
        );
    }
}
fn policy() -> mech_gpu::native_artifact::NativeProgramExecutionLimits {
    let mut policy = limits();
    policy.execution.max_work = 1_000_000_000_000;
    policy.library.max_compute_work = Some(1_000_000_000_000);
    policy.library.max_comparison_work = Some(1_000_000_000_000);
    policy
}
#[test]
fn literal_choice_and_fixed_codes_match_pinned_reference() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../fixtures/native/deflate-literal-choice-reference.json"
    ))
    .unwrap();
    for fixed in [false, true] {
        let entry = if fixed {
            "block-build-static-codes(huff-build-reversal())\n"
        } else {
            "optimal-choose-all-literals(input<[u8]>, begin<u64>, length<u64>, huff-build-reversal())\n"
        };
        for (form, artifact) in ["original", "decoded"]
            .into_iter()
            .zip(artifacts(&format!("{COMPONENTS}{entry}")).0)
        {
            let jit = JitProgramArtifact::compile_with_limits(
                &artifact,
                &mech_stdlib::source_catalog(),
                NativeCompileLimits {
                    max_nodes: 16384,
                    ..Default::default()
                },
            )
            .unwrap();
            let cases = if fixed {
                vec![&fixture["static"]]
            } else {
                fixture["cases"].as_array().unwrap().iter().collect()
            };
            for case in cases {
                let memory = ManagedMemoryBudget::new(128 * 1024 * 1024);
                let named = if fixed {
                    vec![]
                } else {
                    vec![
                        ("input", matrix(&case["input"], true)),
                        ("begin", Data::U64(case["begin"].as_u64().unwrap())),
                        ("length", Data::U64(case["length"].as_u64().unwrap())),
                    ]
                };
                let args = inputs(&artifact, &named, &memory);
                let aliases = args.clone();
                let mut output = vec![None; artifact.outputs().len()];
                let start = std::time::Instant::now();
                jit.evaluate_into(&args, &mut output, policy(), &memory)
                    .unwrap();
                check(output.last().unwrap().as_ref().unwrap(), case);
                println!(
                    "literal-choice {form} {} seconds={}",
                    case["name"],
                    start.elapsed().as_secs_f64()
                );
                assert!(
                    args.iter()
                        .zip(&aliases)
                        .all(|(a, b)| a.shares_frozen_storage(b))
                );
                drop((args, aliases, output));
                assert_eq!(memory.used_bytes(), 0);
                assert_eq!(memory.retained_snapshot_nodes(), 0);
            }
        }
    }
}
