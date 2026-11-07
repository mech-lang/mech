//! An invocation keeps its checked local-bank snapshot across callback boundaries.
use super::*;
use mech_engine::native_artifact::NativeScalarOp;

#[test]
fn entry_locals_snapshot_survives_callback_pointer_change_and_fresh_invocations() {
    let original =
        artifact_source("first := seed<u64> + 3u64\nsecond := first * 7u64\nsecond + 11u64\n");
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    let mut work = Vec::new();
    for artifact in [original, decoded] {
        let jit = crate::native_artifact::JitProgramArtifact::compile(
            &artifact,
            &mech_stdlib::source_catalog(),
        )
        .unwrap();
        let operations = jit
            .descriptor()
            .metadata()
            .plan
            .blocks()
            .iter()
            .enumerate()
            .filter_map(|(site, block)| match block.action() {
                NativeIterationAction::Scalar(NativeMatchAction::Primitive { op, .. }) => {
                    Some((u32::try_from(site).unwrap(), *op))
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            operations.len(),
            3,
            "fixture needs three source scalar boundaries"
        );
        assert_eq!(operations[0].1, NativeScalarOp::U64Add);
        assert_eq!(operations[1].1, NativeScalarOp::U64Mul);
        assert_eq!(operations[2].1, NativeScalarOp::U64Add);
        // Scalar admissions can cover the complete arithmetic path. Restore
        // at its actual callback frontier, after crossing the interior source
        // blocks, rather than assuming every primitive invokes the callback.
        let group = jit
            .descriptor()
            .metadata()
            .map_admission
            .group(operations[0].0 as usize)
            .unwrap();
        assert_eq!(group.start(), operations[0].0);
        assert_eq!(group.frontier.len(), 1);
        assert_eq!(group.frontier[0].previous, operations[2].0);
        let restore_site = group.frontier[0].site;
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        let mut ordinary_output = vec![None; artifact.outputs().len()];
        let mut artifact_work = Vec::new();
        for seed in [5u64, 31, 2] {
            assert_eq!(artifact.inputs().len(), 1);
            let value = ValueDraft {
                schema: artifact.inputs()[0].schema,
                shape_values: Box::new([]),
                data: ValueDataDraft::U64(seed),
            }
            .finalize(&SnapshotValidationContext::new(artifact.schemas()))
            .unwrap();
            let mut reserve = memory
                .reserve_capacity(value.memory_budget_admission_bytes(&memory).unwrap())
                .unwrap();
            let args = [value.into_memory_budget(&mut reserve).unwrap()];
            drop(reserve);
            let alias = args[0].clone();
            let ordinary = jit
                .evaluate_into(&args, &mut ordinary_output, limits(), &memory)
                .unwrap();
            let observed = execute_observed_locals_snapshot(
                &jit,
                &args,
                &mut output,
                &memory,
                operations[0].0,
                restore_site,
            )
            .unwrap();
            assert_eq!(observed, ordinary);
            artifact_work.push(observed.work);
            for outputs in [&output, &ordinary_output] {
                assert_eq!(
                    outputs
                        .last()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    ValueDataDraft::U64((seed + 3) * 7 + 11)
                );
            }
            assert_eq!(
                args[0].canonical_data_draft().unwrap(),
                ValueDataDraft::U64(seed)
            );
            assert!(args[0].shares_frozen_storage(&alias));
        }
        work.push(artifact_work);
        drop((output, ordinary_output));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
    assert_eq!(work[0], work[1]);
}
