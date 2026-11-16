//! Dynamic header repeat encoding and exact block costs from executed reference code.
use super::{allocation_probe, artifacts, inputs, limits};
use mech_core::{ManagedMemoryBudget, ValueDataDraft as Data};
use mech_gpu::native_artifact::{JitProgramArtifact, NativeCompileLimits};

const COMPONENTS: &str = concat!(
    include_str!("../fixtures/native/deflate-huffman-level12.mec"),
    "\n",
    include_str!("../fixtures/native/deflate-header.mec"),
    "\n"
);
const ITEMS: &str =
    "block-compute-precode-items(lens<[u8]>, number<u64>, frequencies<[u32]>, items<[u32]>)\n";
const HEADER_COST: &str = "reversed := huff-build-reversal()\nprecode := block-precompute-huffman-header(litlen<[u8]>, offsets<[u8]>, permutation<[u8]>, reversed)\ncost := optimal-compute-true-cost(litlen, offsets, litlen-frequencies<[u32]>, offset-frequencies<[u32]>, permutation, extra-precode<[u8]>, extra-lengths<[u8]>, extra-offsets<[u8]>, reversed)\n{precode: precode, cost: cost}\n";
fn fixture() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../fixtures/native/deflate-header-reference.json"
    ))
    .unwrap()
}
fn matrix(value: &serde_json::Value, width: u32) -> Data {
    Data::Matrix(
        value
            .as_array()
            .unwrap()
            .iter()
            .map(|v| match width {
                8 => Data::U8(v.as_u64().unwrap() as u8),
                32 => Data::U32(v.as_u64().unwrap() as u32),
                _ => panic!(),
            })
            .collect(),
    )
}
fn field<'a>(value: &'a Data, name: &str) -> &'a Data {
    let Data::Record(fields) = value else {
        panic!("expected record: {name}")
    };
    &fields.iter().find(|f| f.name == name).unwrap().value
}
fn policy() -> mech_gpu::native_artifact::NativeProgramExecutionLimits {
    let mut policy = limits();
    policy.execution.max_work = 1_000_000_000_000;
    policy.library.max_compute_work = Some(1_000_000_000_000);
    policy.library.max_comparison_work = Some(1_000_000_000_000);
    policy
}
fn item_args(case: &serde_json::Value) -> Vec<(&'static str, Data)> {
    vec![
        ("lens", matrix(&case["lens"], 8)),
        ("number", Data::U64(case["num_lens"].as_u64().unwrap())),
        ("frequencies", matrix(&case["freqs"], 32)),
        ("items", matrix(&case["items"], 32)),
    ]
}
fn check_items(value: &mech_core::Value, case: &serde_json::Value) {
    let data = value.canonical_data_draft().unwrap();
    for name in ["freqs", "items"] {
        assert_eq!(
            field(&data, name),
            &matrix(&case["expected"][name], 32),
            "{} {name}",
            case["name"]
        );
    }
    assert_eq!(
        field(&data, "num-items"),
        &Data::U64(case["expected"]["num_items"].as_u64().unwrap())
    );
}

#[test]
fn precode_repeat_boundaries_and_untouched_tails_match_pinned_execution() {
    let fixture = fixture();
    for (form, artifact) in ["original", "decoded"]
        .into_iter()
        .zip(artifacts(&format!("{COMPONENTS}{ITEMS}")).0)
    {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        for case in fixture["precode_cases"].as_array().unwrap() {
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let args = inputs(&artifact, &item_args(case), &memory);
            let aliases = args.clone();
            let mut output = vec![None; artifact.outputs().len()];
            let start = std::time::Instant::now();
            let (result, requests, bytes) = allocation_probe::measured_with_bytes(|| {
                jit.evaluate_into(&args, &mut output, policy(), &memory)
            });
            println!(
                "{}",
                serde_json::json!({"witness":"precode-items","artifact":form,"case":case["name"],"elapsed_seconds":start.elapsed().as_secs_f64(),"allocation_requests":requests,"requested_bytes":bytes,"result":format!("{result:?}")})
            );
            result.unwrap();
            check_items(output.last().unwrap().as_ref().unwrap(), case);
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

#[test]
fn dynamic_headers_and_exact_costs_match_every_pinned_field() {
    let fixture = fixture();
    for (form, artifact) in ["original", "decoded"]
        .into_iter()
        .zip(artifacts(&format!("{COMPONENTS}{HEADER_COST}")).0)
    {
        // Both the independently inspected header and compute_true_cost build
        // their own precode, as in the reference harness: 5,562 control blocks.
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            NativeCompileLimits {
                max_nodes: 16_384,
                ..Default::default()
            },
        )
        .unwrap();
        println!(
            "header native blocks={}",
            jit.descriptor().plan().blocks().len()
        );
        for case in fixture["header_cases"].as_array().unwrap() {
            let tables = &fixture["tables"];
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let args = inputs(
                &artifact,
                &[
                    ("litlen", matrix(&case["litlen_lens"], 8)),
                    ("offsets", matrix(&case["offset_lens"], 8)),
                    ("litlen-frequencies", matrix(&case["freqs_litlen"], 32)),
                    ("offset-frequencies", matrix(&case["freqs_offset"], 32)),
                    (
                        "permutation",
                        matrix(&tables["precode_lens_permutation"], 8),
                    ),
                    ("extra-precode", matrix(&tables["extra_precode_bits"], 8)),
                    ("extra-lengths", matrix(&tables["extra_length_bits"], 8)),
                    ("extra-offsets", matrix(&tables["extra_offset_bits"], 8)),
                ],
                &memory,
            );
            let aliases = args.clone();
            let mut output = vec![None; artifact.outputs().len()];
            let start = std::time::Instant::now();
            let (result, requests, bytes) = allocation_probe::measured_with_bytes(|| {
                jit.evaluate_into(&args, &mut output, policy(), &memory)
            });
            println!(
                "{}",
                serde_json::json!({"witness":"dynamic-header-cost","artifact":form,"case":case["name"],"elapsed_seconds":start.elapsed().as_secs_f64(),"allocation_requests":requests,"requested_bytes":bytes,"result":format!("{result:?}")})
            );
            result.unwrap();
            let data = output
                .last()
                .unwrap()
                .as_ref()
                .unwrap()
                .canonical_data_draft()
                .unwrap();
            let precode = field(&data, "precode");
            for (name, width) in [("freqs", 32), ("lens", 8), ("codewords", 32), ("items", 32)] {
                assert_eq!(
                    field(precode, name),
                    &matrix(&case["expected"][name], width),
                    "{} {name}",
                    case["name"]
                );
            }
            for (name, key) in [
                ("num-items", "num_items"),
                ("num-litlen-syms", "num_litlen_syms"),
                ("num-offset-syms", "num_offset_syms"),
                ("num-explicit-lens", "num_explicit_lens"),
            ] {
                assert_eq!(
                    field(precode, name),
                    &Data::U64(case["expected"][key].as_u64().unwrap()),
                    "{} {name}",
                    case["name"]
                );
            }
            assert_eq!(
                field(&data, "cost"),
                &Data::U64(case["expected"]["cost"].as_u64().unwrap())
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

#[test]
fn precode_overflow_after_partial_writes_preserves_publication_and_recovers() {
    let fixture = fixture();
    let valid = fixture["precode_cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "mixed")
        .unwrap();
    let mut invalid = valid.clone();
    // The first zero-run and the first nonzero item are written before the
    // repeat-previous counter overflows. Those temporary writes must roll back.
    invalid["freqs"][16] = u32::MAX.into();
    for artifact in artifacts(&format!("{COMPONENTS}{ITEMS}")).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for (case, reject) in [(valid, false), (&invalid, true), (valid, false)] {
            let args = inputs(&artifact, &item_args(case), &memory);
            let aliases = args.clone();
            let old = output.clone();
            let retained = memory.used_bytes();
            let nodes = memory.retained_snapshot_nodes();
            let result = jit.evaluate_into(&args, &mut output, policy(), &memory);
            if reject {
                let error = result.unwrap_err();
                assert!(format!("{error:?}").contains("Arithmetic"), "{error:?}");
                assert!(old.iter().zip(&output).all(|(a, b)| {
                    a.as_ref()
                        .unwrap()
                        .shares_frozen_storage(b.as_ref().unwrap())
                }));
                assert_eq!(memory.used_bytes(), retained);
                assert_eq!(memory.retained_snapshot_nodes(), nodes);
            } else {
                result.unwrap();
                check_items(output.last().unwrap().as_ref().unwrap(), valid);
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
