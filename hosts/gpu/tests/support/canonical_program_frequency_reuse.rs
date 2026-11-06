//! Explicit convergence-frequency source variant; the pinned source stays intact.
use super::{artifacts, inputs, limits};
use mech_core::{ManagedMemoryBudget, ValueDataDraft as Data};
use mech_gpu::native_artifact::{JitProgramArtifact, NativeProgramLimitMode};

#[path = "../../../../benchmarks/mech-deflate/source-variants/frequency_reuse.rs"]
mod variant;

const COMPONENTS: &str = concat!(
    include_str!("../fixtures/native/deflate-optimal-path.mec"),
    "\n",
    include_str!("../fixtures/native/deflate-huffman-level12.mec"),
    "\n",
    include_str!("../fixtures/native/deflate-tally.mec"),
    "\n"
);
const ENTRY: &str = r#"
reversed := huff-build-reversal()
prior := optimal-frequency-build-codes(current-litlen<[u32]>, current-offset<[u32]>, reversed)
same := optimal-frequency-matches(current-litlen<[u32]>, current-offset<[u32]>, previous-litlen<[u32]>, previous-offset<[u32]>, valid<bool>)
tallied := optimal-frequency-select-codes(current-litlen, current-offset, prior.litlen-lens, prior.litlen-codewords, prior.offset-lens, prior.offset-codewords, same, reversed)
{same: same, tallied: tallied}
"#;

fn fixture() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../fixtures/native/deflate-tally-reference.json"
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

fn arguments(case: &serde_json::Value, kind: u8) -> Vec<(&'static str, Data)> {
    let mut litlen = case["freqs_litlen"].clone();
    let mut offset = case["freqs_offset"].clone();
    if kind == 2 {
        litlen[287] = (litlen[287].as_u64().unwrap() + 1).into();
    } else if kind == 3 {
        offset[31] = (offset[31].as_u64().unwrap() + 1).into();
    }
    let empty = serde_json::json!([]);
    vec![
        ("current-litlen", matrix(&case["freqs_litlen"], false)),
        ("current-offset", matrix(&case["freqs_offset"], false)),
        (
            "previous-litlen",
            matrix(
                if kind == 0 || kind == 4 {
                    &empty
                } else {
                    &litlen
                },
                false,
            ),
        ),
        (
            "previous-offset",
            matrix(if kind == 0 { &empty } else { &offset }, false),
        ),
        ("valid", Data::Bool(kind != 0)),
    ]
}

fn policy() -> mech_gpu::native_artifact::NativeProgramExecutionLimits {
    // Identical to the existing finite tally/Huffman qualification policy.
    let mut policy = limits();
    policy.execution.max_work = 1_000_000_000_000;
    policy.library.max_compute_work = Some(1_000_000_000_000);
    policy.library.max_comparison_work = Some(1_000_000_000_000);
    policy
}

fn check(value: &mech_core::Value, case: &serde_json::Value, reused: bool) {
    let Data::Record(fields) = value.canonical_data_draft().unwrap() else {
        panic!()
    };
    assert_eq!(
        fields.iter().find(|v| v.name == "same").unwrap().value,
        Data::Bool(reused)
    );
    let Data::Record(tallied) = &fields.iter().find(|v| v.name == "tallied").unwrap().value else {
        panic!()
    };
    for (name, key, narrow) in [
        ("freqs-litlen", "freqs_litlen", false),
        ("freqs-offset", "freqs_offset", false),
        ("litlen-lens", "litlen_lens", true),
        ("litlen-codewords", "litlen_codewords", false),
        ("offset-lens", "offset_lens", true),
        ("offset-codewords", "offset_codewords", false),
    ] {
        assert_eq!(
            tallied.iter().find(|v| v.name == name).unwrap().value,
            matrix(&case[key], narrow),
            "{} {name}",
            case["name"]
        );
    }
}

#[test]
fn convergence_frequency_reuse_matches_all_pinned_codes() {
    let source = format!("{}{ENTRY}", variant::tally_source(COMPONENTS));
    for artifact in artifacts(&source).0 {
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            NativeProgramLimitMode::NoLimits.compilation(),
        )
        .unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for case in fixture()["cases"].as_array().unwrap() {
            let mut first_work = None;
            for kind in [0, 1, 2, 3, 4, 1] {
                let args = inputs(&artifact, &arguments(case, kind), &memory);
                let aliases = args.clone();
                let before = args
                    .iter()
                    .map(|v| v.canonical_data_draft().unwrap())
                    .collect::<Vec<_>>();
                let report = jit
                    .evaluate_into(&args, &mut output, policy(), &memory)
                    .unwrap();
                if kind == 0 {
                    first_work = Some(report.work);
                } else if kind == 1 {
                    assert!(report.work < first_work.unwrap());
                }
                check(output.last().unwrap().as_ref().unwrap(), case, kind == 1);
                for ((value, alias), expected) in args.iter().zip(&aliases).zip(&before) {
                    assert!(value.shares_frozen_storage(alias));
                    assert_eq!(&alias.canonical_data_draft().unwrap(), expected);
                }
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
fn convergence_frequency_failure_preserves_publication_and_recovers() {
    let source = format!("{}{ENTRY}", variant::tally_source(COMPONENTS));
    for artifact in artifacts(&source).0 {
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            NativeProgramLimitMode::NoLimits.compilation(),
        )
        .unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let fixture = fixture();
        let case = &fixture["cases"][0];
        let valid = inputs(&artifact, &arguments(case, 1), &memory);
        let mut output = vec![None; artifact.outputs().len()];
        let complete_work = jit
            .evaluate_into(&valid, &mut output, policy(), &memory)
            .unwrap();
        let prior = output.last().unwrap().as_ref().unwrap().clone();
        let prior_outputs = output.clone();
        let bad = inputs(&artifact, &arguments(case, 1), &memory);
        let aliases = bad.clone();
        let contents = aliases
            .iter()
            .map(|v| v.canonical_data_draft().unwrap())
            .collect::<Vec<_>>();
        let before = (memory.used_bytes(), memory.retained_snapshot_nodes());
        let mut denied = policy();
        denied.execution.max_work = complete_work.work / 2;
        assert!(
            jit.evaluate_into(&bad, &mut output, denied, &memory)
                .is_err()
        );
        assert!(
            output
                .last()
                .unwrap()
                .as_ref()
                .unwrap()
                .shares_frozen_storage(&prior)
        );
        assert!(
            bad.iter()
                .zip(&aliases)
                .all(|(a, b)| a.shares_frozen_storage(b))
        );
        for (value, expected) in aliases.iter().zip(&contents) {
            assert_eq!(&value.canonical_data_draft().unwrap(), expected);
        }
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            before
        );
        for (actual, previous) in output.iter().zip(&prior_outputs) {
            assert!(
                actual
                    .as_ref()
                    .unwrap()
                    .shares_frozen_storage(previous.as_ref().unwrap())
            );
        }
        jit.evaluate_into(&valid, &mut output, policy(), &memory)
            .unwrap();
        check(output.last().unwrap().as_ref().unwrap(), case, true);
        drop((valid, bad, aliases, prior, prior_outputs, output));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

#[test]
fn frequency_variant_keeps_original_tally_and_current_path() {
    let original = format!(
        "{COMPONENTS}\n{}\n{}",
        include_str!("../fixtures/native/deflate-optimize.mec"),
        include_str!("../fixtures/native/deflate-plan-block.mec"),
    );
    let changed = variant::source(&original);
    assert!(changed.contains(include_str!("../fixtures/native/deflate-tally.mec")));
    assert!(changed.contains("  nodes := path.nodes\n"));
    assert!(changed.contains("optimal-frequency-count-path(path.nodes, block-length,"));
    assert!(changed.contains("previous-offset-words<[u32]>, previous-valid<bool>"));
    assert!(std::panic::catch_unwind(|| variant::source(&changed)).is_err());
}
