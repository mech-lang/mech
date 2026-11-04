//! The Huffman parent projects current unsigned arrays before entering a child.
//! These aliases belong in native pieces; actual escaping owners stay canonical.
use super::*;
use mech_engine::native_artifact::NativeBufferFrameOperation as Frame;
use std::collections::BTreeSet;

const SOURCE: &str = include_str!("../../../../../tests/fixtures/native/numeric-parent-child.mec");

#[test]
fn managed_projection_aliases_execute_inside_parent_pieces() {
    for artifact in artifacts(SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let metadata = jit.descriptor().metadata();
        let projections = selected_native_sites(&jit)
            .into_iter()
            .filter(|&site| {
                metadata
                    .buffer_frames
                    .action(site as usize)
                    .is_some_and(|action| {
                        matches!(action.operation(), Frame::Project { output, .. }
                    if metadata.plan.values()[output.get() as usize].scalar_kind().is_none())
                    })
            })
            .collect::<BTreeSet<_>>();
        assert!(
            !projections.is_empty(),
            "managed fields still force ordinary parent boundaries"
        );
        for restored in [false, true] {
            let args = huffman_inputs(&artifact, 3, vec![0, 1, 1, 2], &memory);
            let before = contents(&args);
            let aliases = args.clone();
            let mut output = vec![None; artifact.outputs().len()];
            let (result, counts) = if restored {
                observe::run_restored(&jit, &args, &mut output, mode.execution(), &memory)
            } else {
                observe::run(&jit, &args, &mut output, mode.execution(), &memory)
            };
            result.unwrap();
            assert!(counts.selected + counts.phases > 0);
            assert!(
                counts
                    .helpers
                    .keys()
                    .all(|(site, _)| !projections.contains(site)),
                "{counts:?}"
            );
            assert_eq!(
                output
                    .last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                Data::Matrix([3, 1, 3, 2].map(Data::U8).into())
            );
            assert_eq!(contents(&args), before);
            assert_eq!(contents(&aliases), before);
            drop((args, aliases, output));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}
