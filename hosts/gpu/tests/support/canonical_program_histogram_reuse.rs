//! Source-only histogram reuse, including the real first histogram in Mech.
use super::{artifacts, inputs, limits};
use mech_core::{ManagedMemoryBudget, Value, ValueDataDraft as Data};
use mech_gpu::native_artifact::JitProgramArtifact;

#[path = "../../../../benchmarks/mech-deflate/source-variants/histogram_reuse.rs"]
mod variant;

const HEADER: &str = concat!(
    include_str!("../fixtures/native/deflate-matchfinder.mec"),
    "\n",
    include_str!("../fixtures/native/deflate-block-heuristics.mec"),
    "\n",
    include_str!("../fixtures/native/deflate-huffman-level12.mec"),
    "\n",
    include_str!("../fixtures/native/deflate-literal-choice.mec"),
    "\n",
    include_str!("../fixtures/native/deflate-optimal-costs.mec"),
    "\n"
);

fn source() -> String {
    format!(
        "{}histogram-reuse-choice(input<[u8]>, begin<u64>, length<u64>, frequencies<[u32]>, depth<u64>, table0<[u8]>, table1<[u8]>, table2<[u8]>, symbols<[u8]>) => (out<optimal-default-costs>) :=\n  zero := [0u32 | i <- 0u64..=288u64, i < 288u64]\n  counted := optimal-literal-counts(input, begin, length, 0u64, zero)\n  counts := huff-set32(counted, 256u64, 1u32)\n  out := optimal-choose-default-costs-from-counts(counts, length, frequencies, depth, table0, table1, table2, symbols).\nhistogram-reuse-choice(input<[u8]>, begin<u64>, length<u64>, frequencies<[u32]>, depth<u64>, table0<[u8]>, table1<[u8]>, table2<[u8]>, symbols<[u8]>)\n",
        variant::cost_source(HEADER),
    )
}

fn fixture() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../fixtures/native/deflate-default-costs-reference.json"
    ))
    .unwrap()
}

fn matrix(value: &serde_json::Value, narrow: bool) -> Data {
    Data::Matrix(
        value
            .as_array()
            .unwrap()
            .iter()
            .map(|v| {
                if narrow {
                    Data::U8(v.as_u64().unwrap() as u8)
                } else {
                    Data::U32(v.as_u64().unwrap() as u32)
                }
            })
            .collect(),
    )
}

fn arguments(
    artifact: &mech_engine::ProgramArtifact,
    fixture: &serde_json::Value,
    case: &serde_json::Value,
    memory: &ManagedMemoryBudget,
) -> Vec<Value> {
    inputs(
        artifact,
        &[
            ("input", matrix(&case["input"], true)),
            ("begin", Data::U64(case["begin"].as_u64().unwrap())),
            ("length", Data::U64(case["length"].as_u64().unwrap())),
            ("frequencies", matrix(&case["frequencies"], false)),
            ("depth", Data::U64(case["depth"].as_u64().unwrap())),
            (
                "table0",
                matrix(&fixture["tables"]["default_lit_cost_0"], true),
            ),
            (
                "table1",
                matrix(&fixture["tables"]["default_lit_cost_1"], true),
            ),
            (
                "table2",
                matrix(&fixture["tables"]["default_lit_cost_2"], true),
            ),
            (
                "symbols",
                matrix(&fixture["tables"]["default_len_sym_costs"], true),
            ),
        ],
        memory,
    )
}

fn policy() -> mech_gpu::native_artifact::NativeProgramExecutionLimits {
    let mut policy = limits();
    policy.execution.max_work = 1_000_000_000_000;
    policy.library.max_compute_work = Some(1_000_000_000_000);
    policy.library.max_comparison_work = Some(1_000_000_000_000);
    policy
}

#[test]
fn reused_literal_histogram_matches_all_pinned_default_cost_cases() {
    let fixture = fixture();
    let cases = fixture["cases"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|case| {
            let mut shifted = case.clone();
            let mut input = vec![serde_json::json!(255u8); 5];
            input.extend(case["input"].as_array().unwrap().iter().cloned());
            shifted["input"] = input.into();
            shifted["begin"] = (case["begin"].as_u64().unwrap() + 5).into();
            [case.clone(), shifted]
        })
        .collect::<Vec<_>>();
    for artifact in artifacts(&source()).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for case in &cases {
            let args = arguments(&artifact, &fixture, case, &memory);
            let aliases = args.clone();
            let before = args
                .iter()
                .map(|v| v.canonical_data_draft().unwrap())
                .collect::<Vec<_>>();
            jit.evaluate_into(&args, &mut output, policy(), &memory)
                .unwrap();
            let Data::Record(fields) = output
                .last()
                .unwrap()
                .as_ref()
                .unwrap()
                .canonical_data_draft()
                .unwrap()
            else {
                panic!()
            };
            for (name, at) in [("lit-cost", 0), ("len-sym-cost", 1)] {
                assert_eq!(
                    fields.iter().find(|f| f.name == name).unwrap().value,
                    Data::U64(case["expected"][at].as_u64().unwrap()),
                    "{} {name}",
                    case["name"]
                );
            }
            for (at, alias) in aliases.iter().enumerate() {
                assert_eq!(alias.canonical_data_draft().unwrap(), before[at]);
                assert!(args[at].shares_frozen_storage(alias));
            }
        }
        drop(output);
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

#[test]
fn reused_histogram_overflow_preserves_publication_aliases_and_retry() {
    let fixture = fixture();
    for artifact in artifacts(&source()).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        let case = &fixture["cases"][1];
        let valid = arguments(&artifact, &fixture, case, &memory);
        jit.evaluate_into(&valid, &mut output, policy(), &memory)
            .unwrap();
        let previous = output.last().unwrap().as_ref().unwrap().clone();
        let mut bad = case.clone();
        bad["begin"] = u64::MAX.into();
        bad["length"] = 2u64.into();
        let rejected = arguments(&artifact, &fixture, &bad, &memory);
        let aliases = rejected.clone();
        let before = aliases
            .iter()
            .map(|v| v.canonical_data_draft().unwrap())
            .collect::<Vec<_>>();
        assert!(
            jit.evaluate_into(&rejected, &mut output, policy(), &memory)
                .is_err()
        );
        assert!(
            output
                .last()
                .unwrap()
                .as_ref()
                .unwrap()
                .shares_frozen_storage(&previous)
        );
        for (value, expected) in aliases.iter().zip(&before) {
            assert_eq!(&value.canonical_data_draft().unwrap(), expected);
        }
        jit.evaluate_into(&valid, &mut output, policy(), &memory)
            .unwrap();
        assert_eq!(
            output
                .last()
                .unwrap()
                .as_ref()
                .unwrap()
                .canonical_data_draft()
                .unwrap(),
            previous.canonical_data_draft().unwrap()
        );
        drop((valid, rejected, aliases, previous, output));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

#[test]
fn histogram_variant_only_replaces_the_intended_pinned_calls() {
    let source = format!(
        "{}\n{}",
        HEADER,
        include_str!("../fixtures/native/deflate-plan-block.mec")
    );
    let changed = variant::source(&source);
    assert_ne!(changed, source);
    assert!(changed.contains(include_str!("../fixtures/native/deflate-optimal-costs.mec")));
    assert_eq!(
        changed
            .matches(
                "  defaults := optimal-choose-default-costs-from-counts(literals.freqs-litlen,"
            )
            .count(),
        1
    );
    assert_eq!(
        changed
            .matches("  counts := optimal-cost-count-literals(zero, input, begin, length, 0u64)")
            .count(),
        1
    );
    assert!(std::panic::catch_unwind(|| variant::source(&changed)).is_err());
}
