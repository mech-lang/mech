//! A projected leaf borrows its current frame handle until destination admission.
use super::*;
use crate::native_artifact::program::runtime::buffer_frames::Field;

#[test]
fn managed_projection_borrows_the_stored_handle_with_original_work_and_aliases() {
    for artifact in artifacts(FLAT) {
        let plan = mech_engine::native_artifact::lower_native_program_artifact(
            &artifact,
            &mech_stdlib::source_catalog(),
        )
        .unwrap();
        let layout = plan.facts().buffer_frames();
        let (source, _) = pair(&layout, &plan, false);
        let fields = layout.fields(source).unwrap();
        let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
        let mut work = NativeWorkMeter::new(u64::MAX).unwrap();
        let mut frames = BufferFrames::new(&layout, u64::MAX, &memory, &mut work).unwrap();
        for seed in [7, 18, 29] {
            fill(&layout, fields, &artifact, &mut frames, &memory, seed);
            let prior = snapshot(&frames);
            let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
            for (ordinal, field) in fields.iter().enumerate() {
                let before = work.used();
                let projected = frames.project(&layout, source, ordinal, &mut work).unwrap();
                assert_eq!(work.used() - before, 64);
                match &projected {
                    Field::Managed(value) => {
                        let value: &Value = value;
                        let stored = frames.managed_owner(&layout, source, ordinal).unwrap();
                        assert!(
                            core::ptr::eq(value, stored),
                            "projection must borrow the frame owner without cloning a temporary"
                        );
                        assert!(value.shares_frozen_storage(stored));
                        assert_eq!(value.schema(), field.schema());
                    }
                    Field::Scalar { schema, value } => {
                        assert_eq!(*schema, field.schema());
                        assert_eq!(value.to_bits(), frames.current_leaf(*field).unwrap().1);
                    }
                }
            }
            unchanged(&frames, &prior);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                retained
            );
        }
        drop(frames);
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

#[test]
fn projection_preserves_invalid_field_work_and_current_scalar_error_order() {
    fn error<T>(result: Result<T, NativeProgramExecutionError>) -> String {
        match result {
            Err(error) => format!("{error:?}"),
            Ok(_) => panic!("projection must reject this visit"),
        }
    }
    for artifact in artifacts(FLAT) {
        let plan = mech_engine::native_artifact::lower_native_program_artifact(
            &artifact,
            &mech_stdlib::source_catalog(),
        )
        .unwrap();
        let layout = plan.facts().buffer_frames();
        let (source, _) = pair(&layout, &plan, false);
        let fields = layout.fields(source).unwrap();
        let (ordinal, at) = fields
            .iter()
            .enumerate()
            .find_map(|(i, field)| match field.lane() {
                Lane::Scalar {
                    at,
                    kind: mech_engine::native_artifact::NativeScalarKind::Bool,
                } => Some((i, at)),
                _ => None,
            })
            .unwrap();
        let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
        let mut work = NativeWorkMeter::new(u64::MAX).unwrap();
        let mut frames = BufferFrames::new(&layout, u64::MAX, &memory, &mut work).unwrap();
        fill(&layout, fields, &artifact, &mut frames, &memory, 7);
        frames.inner.as_mut().unwrap().words.values_mut()[at * 2] = 2;
        let prior = snapshot(&frames);
        let mut denied = NativeWorkMeter::new(63).unwrap();
        assert!(
            error(frames.project(&layout, source, fields.len(), &mut denied))
                .contains("outside layout")
        );
        assert_eq!(denied.used(), 0);
        assert!(error(frames.project(&layout, source, ordinal, &mut denied)).contains("WorkLimit"));
        assert_eq!(denied.used(), 0);
        let mut exact = NativeWorkMeter::new(64).unwrap();
        assert!(
            error(frames.project(&layout, source, ordinal, &mut exact)).contains("invalid bits")
        );
        assert_eq!(exact.used(), 64);
        unchanged(&frames, &prior);
        drop(prior);
        for word in [0, 1] {
            frames.inner.as_mut().unwrap().words.values_mut()[at * 2] = word;
            let mut exact = NativeWorkMeter::new(64).unwrap();
            let Field::Scalar { value, .. } = frames
                .project(&layout, source, ordinal, &mut exact)
                .unwrap()
            else {
                panic!("current scalar word")
            };
            assert_eq!(value.to_bits(), word);
            assert_eq!(exact.used(), 64);
        }
        drop(frames);
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}
