//! Move only the selected call's disposable owners after the complete preflight.
use super::*;
use crate::native_artifact::program::runtime::buffer_frames::BufferFrames;
use mech_engine::native_artifact::{NativeBufferFrameLane as Lane, NativeBufferFrameOperation};

#[test]
fn pack_transfers_temporary_owners_after_validation_and_preserves_aliases_on_denial() {
    for artifact in artifacts(record_lookup::SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let plan = jit.descriptor().plan();
        let layout = &plan.facts().buffer_frames();
        let (output, operands, fields) = plan
            .blocks()
            .iter()
            .enumerate()
            .find_map(|(site, block)| {
                let NativeBufferFrameOperation::Pack { output } = layout.action(site)?.operation()
                else {
                    return None;
                };
                let NativeIterationAction::LibraryCall { operands, .. } = block.action() else {
                    return None;
                };
                let fields = layout.fields(*output)?;
                (fields.len() == 5
                    && fields
                        .iter()
                        .any(|f| matches!(f.lane(), Lane::Managed { .. }))
                    && fields.iter().any(|f| {
                        matches!(
                            f.lane(),
                            Lane::Scalar {
                                kind: NativeScalarKind::Bool,
                                ..
                            }
                        )
                    }))
                .then_some((*output, operands.as_ref(), fields))
            })
            .unwrap();
        assert!(fields.iter().all(|f| layout.children(*f).is_none()));
        let buffer = fields
            .iter()
            .position(|f| matches!(f.lane(), Lane::Managed { .. }))
            .unwrap();
        let boolean = fields
            .iter()
            .position(|f| {
                matches!(
                    f.lane(),
                    Lane::Scalar {
                        kind: NativeScalarKind::Bool,
                        ..
                    }
                )
            })
            .unwrap();
        assert!(
            boolean > buffer,
            "the invalid field must follow the managed owner"
        );
        let memory = mode.memory_budget();
        let mut work = NativeWorkMeter::new(u64::MAX).unwrap();
        let mut frames = BufferFrames::new(layout, u64::MAX, &memory, &mut work).unwrap();
        let make = |word| {
            let schema = fields[buffer].schema();
            let shape = mech_core::shape_for_resolved_extents(
                artifact.schemas().get(schema).unwrap(),
                &[1, 1],
            )
            .unwrap();
            let raw = ValueDraft {
                schema,
                shape_values: shape.parameter_values().into(),
                data: Data::Matrix(vec![Data::U8(word)].into()),
            }
            .finalize(&SnapshotValidationContext::new(artifact.schemas()))
            .unwrap();
            let mut reservation = memory
                .reserve_capacity(raw.memory_budget_admission_bytes(&memory).unwrap())
                .unwrap();
            raw.into_memory_budget(&mut reservation).unwrap()
        };
        let first = make(7);
        let mut pending = vec![None; fields.len()];
        pending[buffer] = Some(first.clone());
        let words = vec![0; fields.len()];
        let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
        let before = work.used();
        frames
            .pack(layout, output, operands, &mut pending, &words, &mut work)
            .unwrap();
        let charge = work.used() - before;
        assert_eq!(charge, 64 * fields.len() as u64 + 64);
        assert!(
            pending.iter().all(Option::is_none),
            "successful pack must consume disposable call inputs"
        );
        assert!(
            frames
                .managed_owner(layout, output, buffer)
                .unwrap()
                .shares_frozen_storage(&first)
        );
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            retained
        );

        let next = make(9);
        pending[buffer] = Some(next.clone());
        let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
        let mut invalid = words.clone();
        invalid[boolean] = 2;
        assert!(
            frames
                .pack(layout, output, operands, &mut pending, &invalid, &mut work)
                .is_err()
        );
        assert!(
            pending[buffer]
                .as_ref()
                .unwrap()
                .shares_frozen_storage(&next)
        );
        assert!(
            frames
                .managed_owner(layout, output, buffer)
                .unwrap()
                .shares_frozen_storage(&first)
        );
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            retained
        );

        let mut denied = NativeWorkMeter::new(charge - 1).unwrap();
        assert!(
            frames
                .pack(layout, output, operands, &mut pending, &words, &mut denied)
                .is_err()
        );
        assert!(
            pending[buffer]
                .as_ref()
                .unwrap()
                .shares_frozen_storage(&next)
        );
        assert!(
            frames
                .managed_owner(layout, output, buffer)
                .unwrap()
                .shares_frozen_storage(&first)
        );
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            retained
        );

        frames
            .pack(layout, output, operands, &mut pending, &words, &mut work)
            .unwrap();
        assert!(pending.iter().all(Option::is_none));
        assert!(
            frames
                .managed_owner(layout, output, buffer)
                .unwrap()
                .shares_frozen_storage(&next)
        );
        assert_eq!(
            contents(&[first.clone(), next.clone()]),
            [
                Data::Matrix(vec![Data::U8(7)].into()),
                Data::Matrix(vec![Data::U8(9)].into())
            ]
        );
        drop((frames, pending, first, next, work, denied));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}
