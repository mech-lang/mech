//! Return-field controls retained after the direct-completion experiment.
use super::*;

#[test]
fn numeric_ordered_reduction_preserves_current_terminal_result_fields() {
    let base =
        child_prepayment::source().replace("nodes<[u64]>, 2u64, stop", "nodes<[u64]>, 1u64, stop");
    let terminal = ":Done(k, best, item) => {best: best, item: item, k: k}.";
    assert_eq!(base.matches(terminal).count(), 1);
    for expression in [
        "{best: best, item: item, k: k}",
        "{best: best, item: k, k: item}",
        "{best: 7u32, item: fault, k: k}",
        "{best: best, item: item, k: bits/wrapping-add(k, 1u64)}",
    ] {
        let source = base.replace(terminal, &format!(":Done(k, best, item) => {expression}."));
        for artifact in artifacts(&source) {
            let mode = NativeProgramLimitMode::NoLimits;
            let object = crate::native_artifact::emit_program_object_with_limits(
                &artifact,
                &mech_stdlib::source_catalog(),
                mode.compilation(),
            )
            .unwrap();
            let metadata = object.metadata();
            let reductions = [
                "compiled_readonly_numeric_loops",
                "compiled_numeric_write_loops",
            ]
            .into_iter()
            .flat_map(|key| metadata[key].as_array().into_iter().flatten())
            .flat_map(|region| {
                region["guarded_loop_windows"]
                    .as_array()
                    .into_iter()
                    .flatten()
            })
            .filter_map(|window| window["reduction"].as_object())
            .collect::<Vec<_>>();
            assert!(!reductions.is_empty(), "{expression}");
            let native = JitProgramArtifact::compile_with_limits(
                &artifact,
                &mech_stdlib::source_catalog(),
                mode.compilation(),
            )
            .unwrap();
            for restored in [false, true] {
                let memory = mode.memory_budget();
                let mut outputs = vec![None; artifact.outputs().len()];
                for (start, stop, seed) in [(0, 18, u32::MAX), (3, 3, 0), (19, 18, 0)] {
                    let args = current_windows::arguments(
                        &artifact,
                        &memory,
                        40,
                        start,
                        stop,
                        seed,
                        u64::MAX,
                    );
                    let aliases = args.clone();
                    let before = contents(&args);
                    let expected = ordered_reduction_exhaustion::resident(&artifact, &args);
                    let result = if restored {
                        observe::run_restored(
                            &native,
                            &args,
                            &mut outputs,
                            mode.execution(),
                            &memory,
                        )
                        .0
                    } else {
                        observe::run(&native, &args, &mut outputs, mode.execution(), &memory).0
                    };
                    result.unwrap();
                    assert_eq!(
                        outputs
                            .iter()
                            .map(|v| v.as_ref().unwrap().canonical_data_draft().unwrap())
                            .collect::<Vec<_>>(),
                        expected,
                        "{expression}, restored={restored}, start={start}, stop={stop}"
                    );
                    assert_eq!(contents(&args), before);
                    assert_eq!(contents(&aliases), before);
                }
                drop(outputs);
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
        }
    }
}
