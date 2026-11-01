//! Actual private record field calls, current words, ordered failure and retry.
use super::*;
use crate::native_artifact::program::protocol::abi::Frame;
use mech_core::snapshot::F64Bits;

pub(super) const SOURCE: &str =
    include_str!("../../../../../tests/fixtures/native/prepared-record-lookup.mec");
#[derive(Clone, Copy)]
pub(super) struct Input {
    word: u8,
    number: u64,
    flag: bool,
    signed: i64,
    ratio: f64,
    left: u64,
    allowed: bool,
    divisor: u64,
    empty: bool,
}
pub(super) const VALID: Input = Input {
    word: 3,
    number: 2,
    flag: true,
    signed: -2,
    ratio: 1.5,
    left: 2,
    allowed: true,
    divisor: 1,
    empty: false,
};
pub(super) fn arguments(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    input: Input,
) -> Vec<Value> {
    inputs(
        artifact,
        &[
            (
                "xs",
                Data::Matrix(
                    if input.empty {
                        vec![]
                    } else {
                        vec![Data::U8(input.word)]
                    }
                    .into(),
                ),
            ),
            ("number", Data::U64(input.number)),
            ("flag", Data::Bool(input.flag)),
            ("signed", Data::I64(input.signed)),
            ("ratio", Data::F64(F64Bits::from_f64(input.ratio))),
            ("left", Data::U64(input.left)),
            ("allowed", Data::Bool(input.allowed)),
            ("divisor", Data::U64(input.divisor)),
        ],
        memory,
    )
}
#[test]
fn private_record_lookup_keeps_current_scalar_words_aliases_errors_and_retry() {
    let mut expected_work = vec![None; 6];
    for artifact in artifacts(SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let catalog = mech_stdlib::source_catalog();
        let jit = JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
            .unwrap();
        let plan = jit.descriptor().plan();
        let frames = plan.facts().buffer_frames();
        let kinds = plan
            .blocks()
            .iter()
            .enumerate()
            .filter_map(|(site, block)| {
                use mech_engine::native_artifact::{
                    NativeBufferFrameOperation as F, NativeCollectionPrimitive as P,
                    NativeIterationAction as A, NativeScalarKind as K,
                };
                let A::LibraryCall { template, .. } = block.action() else {
                    return None;
                };
                if template.primitive() != P::RecordFieldAccess
                    || !matches!(frames.action(site)?.operation(), F::Project { .. })
                {
                    return None;
                }
                K::from_schema_body(template.output().body()).map(|kind| format!("{kind:?}"))
            })
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(
            kinds,
            ["Bool", "F64", "I64", "U64"]
                .map(str::to_string)
                .into_iter()
                .collect()
        );
        let image = jit.descriptor().encode_build_owned_image().unwrap();
        // SAFETY: this exact unchanged image and entry share the live JIT owner.
        let restored =
            unsafe { NativeProgramDescriptor::decode_build_owned_image(&image, &catalog) }.unwrap();
        assert_eq!(restored.encode_build_owned_image().unwrap(), image);
        for descriptor in [jit.descriptor(), &restored] {
            let memory = mode.memory_budget();
            let mut published = vec![None; artifact.outputs().len()];
            for (case, (input, expected, error)) in [
                (VALID, Some(36), None),
                (
                    Input {
                        word: 11,
                        number: 20,
                        flag: false,
                        signed: 2,
                        ratio: 0.5,
                        left: 3,
                        divisor: 2,
                        ..VALID
                    },
                    Some(30),
                    None,
                ),
                (
                    Input {
                        allowed: false,
                        divisor: 0,
                        empty: true,
                        ..VALID
                    },
                    None,
                    Some("Integrity"),
                ),
                (
                    Input {
                        empty: true,
                        ..VALID
                    },
                    None,
                    Some("Library"),
                ),
                (
                    Input {
                        divisor: 0,
                        ..VALID
                    },
                    None,
                    Some("Arithmetic"),
                ),
                (
                    Input {
                        word: 5,
                        flag: false,
                        signed: 0,
                        ratio: 2.5,
                        left: 1,
                        ..VALID
                    },
                    Some(17),
                    None,
                ),
            ]
            .into_iter()
            .enumerate()
            {
                let args = arguments(&artifact, &memory, input);
                let aliases = args.clone();
                let before = contents(&args);
                let prior = published.clone();
                let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let entry = jit.test_checked_entry();
                let result = execute(
                    descriptor,
                    |frame: *mut Frame| {
                        // SAFETY: execute retains this initialized owning frame and the entry owner.
                        unsafe { entry(frame) }
                    },
                    &args,
                    &mut published,
                    mode.execution(),
                    &memory,
                );
                if let Some(expected) = expected {
                    let report = result.unwrap();
                    assert_eq!(
                        published[0]
                            .as_ref()
                            .unwrap()
                            .canonical_data_draft()
                            .unwrap(),
                        Data::U64(expected)
                    );
                    assert_eq!(report.work, *expected_work[case].get_or_insert(report.work));
                } else {
                    let receipt = format!("{:?}", result.unwrap_err());
                    assert!(receipt.contains(error.unwrap()), "{receipt}");
                    assert!(published.iter().zip(&prior).all(|(a, b)| {
                        a.as_ref()
                            .unwrap()
                            .shares_frozen_storage(b.as_ref().unwrap())
                    }));
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        retained
                    );
                }
                assert_eq!(contents(&args), before);
                assert_eq!(contents(&aliases), before);
                assert!(
                    args.iter()
                        .zip(&aliases)
                        .all(|(a, b)| a.shares_frozen_storage(b))
                );
            }
            drop(published);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}
