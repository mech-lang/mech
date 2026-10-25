//! Real Mech DEFLATE matching operations; independent C fixtures are the oracle.
use super::{artifacts, equal, inputs, limits};
use mech_core::{ManagedMemoryBudget, ValueDataDraft as Data};
use mech_gpu::native_artifact::JitProgramArtifact;

const KERNEL: &str = include_str!("../fixtures/native/deflate-matchfinder.mec");

#[path = "canonical_program_matchfinder_reads.rs"]
mod reads;

#[path = "canonical_program_matchfinder_capacity.rs"]
mod capacity;

#[derive(serde::Deserialize)]
struct CReference {
    cases: Vec<MatchingCase>,
    nodes: Vec<u16>,
    rebased: Vec<u16>,
}
#[derive(serde::Deserialize)]
struct MatchingCase {
    name: String,
    sequence: u32,
    hash_bits: u64,
    input: Vec<u8>,
    str_at: u64,
    match_at: u64,
    start: u64,
    maximum: u64,
    hash: u64,
    matched: u64,
}
fn c_reference() -> CReference {
    serde_json::from_str(include_str!(
        "../fixtures/native/deflate-matchfinder-reference.json"
    ))
    .unwrap()
}
#[test]
fn deflate_matchfinder_byte_words_hash_and_extend_match_pinned_c() {
    let reference_cases = c_reference();
    assert_eq!(reference_cases.cases.len(), 70);
    let source = format!(
        "{KERNEL}\nhash := lz-hash(sequence<u32>, hash-bits<u64>)\nmatched := extend-length(input<[u8]>, str-at<u64>, match-at<u64>, start<u64>, maximum<u64>)\n(hash, matched)\n"
    );
    for artifact in artifacts(&source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert!(
            jit.descriptor()
                .plan()
                .blocks()
                .iter()
                .any(|block| matches!(
                    block.action(),
                    mech_engine::native_artifact::NativeIterationAction::Scalar(
                        mech_engine::native_artifact::NativeMatchAction::Primitive {
                            op: mech_engine::native_artifact::NativeScalarOp::U64Ctz,
                            ..
                        }
                    )
                ))
        );
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for (index, case) in reference_cases.cases.iter().enumerate() {
            let args = inputs(
                &artifact,
                &[
                    ("sequence", Data::U32(case.sequence)),
                    ("hash-bits", Data::U64(case.hash_bits)),
                    (
                        "input",
                        Data::Matrix(case.input.iter().copied().map(Data::U8).collect()),
                    ),
                    ("str-at", Data::U64(case.str_at)),
                    ("match-at", Data::U64(case.match_at)),
                    ("start", Data::U64(case.start)),
                    ("maximum", Data::U64(case.maximum)),
                ],
                &memory,
            );
            // Every native result is checked against the independent C oracle.
            // Keep the resident engine's ordinary default work cap: recursive
            // array transport currently copies and can exhaust it even though
            // the explicitly qualified native execution below completes.
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap_or_else(|error| panic!("{}: {error:?}", case.name));
            if [0, 35, 69].contains(&index) {
                match super::reference_result(&artifact, &args) {
                    Ok(ordinary) => equal(&artifact, &output, &ordinary),
                    Err(error) => {
                        assert_eq!(index, 35, "{}: {error}", case.name);
                        assert!(
                            error.contains("ResourceLimit")
                                && error.contains("ComputeWork")
                                && error.contains("limit: 16777216"),
                            "{}: {error}",
                            case.name
                        );
                        eprintln!("DEFLATE_RESIDENT_DEFAULT_DENIAL case={} {error}", case.name);
                    }
                }
            }
            assert_eq!(
                output
                    .last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                Data::Tuple(vec![Data::U64(case.hash), Data::U64(case.matched)].into()),
                "{}",
                case.name
            );
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
    }
}

fn check_node_helper(call: &str, cases: &[(Vec<(&str, Data)>, Data)]) {
    let source = format!("{KERNEL}\n{call}\n");
    for artifact in artifacts(&source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for (named, expected) in cases {
            let args = inputs(&artifact, named, &memory);
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap_or_else(|error| panic!("native {call} {named:?}: {error:?}"));
            let ordinary = super::reference_result(&artifact, &args)
                .unwrap_or_else(|error| panic!("resident {call} {named:?}: {error:?}"));
            equal(&artifact, &output, &ordinary);
            assert_eq!(
                &output
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
        assert_eq!(memory.used_bytes(), 0);
    }
}
#[test]
fn deflate_node_initialization_uses_a_comprehension_for_empty_and_live_extents() {
    let cases = [0u64, 5, 512, 1, 0]
        .into_iter()
        .map(|count| {
            (
                vec![("count", Data::U64(count))],
                Data::Matrix(vec![Data::U16(0); count as usize].into()),
            )
        })
        .collect::<Vec<_>>();
    check_node_helper("initial-nodes(count<u64>)", &cases);
}
#[test]
fn deflate_node_rebase_broadcast_matches_the_pinned_c_signed_bias() {
    let golden = c_reference();
    assert_eq!(golden.nodes.len(), 512);
    let cases = [
        vec![],
        vec![0u16, 1, 32767, 32768, 32769, 65535],
        golden.nodes.clone(),
        vec![0],
    ]
    .into_iter()
    .map(|nodes| {
        let expected = if nodes == golden.nodes {
            golden.rebased.clone()
        } else {
            nodes.iter().map(|n| n.saturating_sub(32768)).collect()
        };
        (
            vec![(
                "nodes",
                Data::Matrix(nodes.into_iter().map(Data::U16).collect()),
            )],
            Data::Matrix(expected.into_iter().map(Data::U16).collect()),
        )
    })
    .collect::<Vec<_>>();
    check_node_helper("rebase-nodes(nodes<[u16]>)", &cases);
}
#[test]
fn deflate_biased_node_indices_keep_wrapping_window_arithmetic() {
    let cases = [
        (0u64, 0u16),
        (u64::MAX, 65535),
        (32768, 1),
        (0, 32768),
        (u64::MAX, 32769),
    ]
    .into_iter()
    .map(|(base, node)| {
        (
            vec![("base", Data::U64(base)), ("node", Data::U16(node))],
            Data::U64(base.wrapping_add(u64::from(node)).wrapping_sub(32768)),
        )
    })
    .collect::<Vec<_>>();
    check_node_helper("node-index(base<u64>, node<u16>)", &cases);
}
