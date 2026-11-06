//! Complete source maps, including fresh captures, tails and finite fallback.
use super::*;
use crate::native_artifact::codegen::control::plan::CodePlan;
use crate::native_artifact::program::codegen::word_collections::vector_selected;
use crate::native_artifact::program::prepare::PreparedNativeProgram;

const SOURCE: &str = r#"
clamp(value<u16>, threshold<u64>) => (out<u16>) :=
  wide<u64> := value
  result := ((wide < threshold)? | true => 0u64 | false => wide - threshold)
  out<u16> := result.
transform(values<[u16]>, threshold<u64>) => (out<[u16]>) :=
  out := [clamp(value, threshold) | value <- values].
answer := transform(input<[u16]>, threshold<u64>)
answer
"#;

#[test]
fn vector_word_clamps_preserve_widths_tails_and_current_captures() {
    use crate::native_artifact::codegen::options::OptimizationPass;
    for (kind, maximum, cell, width) in [
        (
            "u8",
            u8::MAX as u64,
            (|v| Data::U8(v as u8)) as fn(u64) -> Data,
            1,
        ),
        (
            "u16",
            u16::MAX as u64,
            (|v| Data::U16(v as u16)) as fn(u64) -> Data,
            2,
        ),
        (
            "u32",
            u32::MAX as u64,
            (|v| Data::U32(v as u32)) as fn(u64) -> Data,
            4,
        ),
        ("u64", u64::MAX, Data::U64 as fn(u64) -> Data, 8),
    ] {
        for artifact in artifacts(&SOURCE.replace("u16", kind)) {
            let mode = NativeProgramLimitMode::NoLimits;
            let catalog = mech_stdlib::source_catalog();
            let ordinary = JitProgramArtifact::compile_for_test(
                &artifact,
                &catalog,
                OptimizationPass::ScalarLoops,
            )
            .unwrap();
            let native =
                JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
                    .unwrap_or_else(|error| panic!("{kind}: {error:?}"));
            let metadata = native.descriptor().metadata();
            assert_eq!(metadata.word_collections.len(), 1);
            let prepared =
                PreparedNativeProgram::prepare(&artifact, &catalog, metadata.plan.clone()).unwrap();
            assert!(vector_selected(
                CodePlan::Program(&prepared),
                &metadata.word_collections[0]
            ));
            let memory = mode.memory_budget();
            let quantum = 64 / width;
            for length in [0, 1, quantum - 1, quantum, quantum + 1, quantum * 2 + 3] {
                let cells = (0..length)
                    .map(|i| [0, 1, maximum / 2, maximum / 2 + 1, maximum][i % 5])
                    .collect::<Vec<_>>();
                for threshold in [
                    maximum / 2 + 1,
                    0,
                    maximum,
                    maximum.saturating_add(1),
                    u64::MAX,
                    1,
                ] {
                    let args = inputs(
                        &artifact,
                        &[
                            (
                                "input",
                                Data::Matrix(cells.iter().copied().map(cell).collect()),
                            ),
                            ("threshold", Data::U64(threshold)),
                        ],
                        &memory,
                    );
                    let aliases = args.clone();
                    let before = contents(&args);
                    let expected = Data::Matrix(
                        cells
                            .iter()
                            .map(|v| cell(v.saturating_sub(threshold)))
                            .collect(),
                    );
                    let mut work = None;
                    for jit in [&ordinary, &native] {
                        for restored in [false, true] {
                            let mut output = vec![None; artifact.outputs().len()];
                            let (result, _) = if restored {
                                observe::run_restored(
                                    jit,
                                    &args,
                                    &mut output,
                                    mode.execution(),
                                    &memory,
                                )
                            } else {
                                observe::run(jit, &args, &mut output, mode.execution(), &memory)
                            };
                            let report = result.unwrap();
                            assert_eq!(
                                report.work,
                                *work.get_or_insert(report.work),
                                "{kind}/{length}/{threshold}"
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
                            assert_eq!(contents(&args), before);
                            assert_eq!(contents(&aliases), before);
                        }
                    }
                }
            }
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn vector_word_clamps_refuse_changed_results_and_unproved_paths() {
    for source in [
        SOURCE.replace("true => 0u64", "true => 1u64"),
        SOURCE.replace("wide < threshold", "wide <= threshold"),
        SOURCE.replace("wide - threshold", "threshold - wide"),
        SOURCE.replace(
            "((wide < threshold)? | true => 0u64 | false => wide - threshold)",
            "wide - threshold",
        ),
    ] {
        for artifact in artifacts(&source) {
            let jit = JitProgramArtifact::compile_with_limits(
                &artifact,
                &mech_stdlib::source_catalog(),
                NativeProgramLimitMode::NoLimits.compilation(),
            )
            .unwrap();
            let metadata = jit.descriptor().metadata();
            let prepared = PreparedNativeProgram::prepare(
                &artifact,
                &mech_stdlib::source_catalog(),
                metadata.plan.clone(),
            )
            .unwrap();
            assert!(
                metadata
                    .word_collections
                    .iter()
                    .all(|region| !vector_selected(CodePlan::Program(&prepared), region))
            );
        }
    }
}

#[test]
fn vector_word_clamps_keep_finite_failure_publication_and_retry() {
    resources::finite_boundaries(SOURCE, |artifact, memory| {
        inputs(
            artifact,
            &[
                (
                    "input",
                    Data::Matrix(
                        (0..65)
                            .map(|i| Data::U16([0, 1, 32767, 32768, 65535][i % 5]))
                            .collect(),
                    ),
                ),
                ("threshold", Data::U64(32768)),
            ],
            memory,
        )
    });
}
