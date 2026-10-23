//! Observe actual emitted execution, never substitute a callback or a value.
use super::super::tests::{artifact_source, limits};
use super::*;
use crate::native_artifact::JitProgramArtifact;
use mech_core::{
    Value, ValueData,
    snapshot::{SnapshotValidationContext, ValueDataDraft, ValueDraft},
};
use std::cell::Cell;

#[derive(Clone, Copy, Debug)]
struct Observation {
    private: bool,
    matrix_present: bool,
    rows: usize,
}
thread_local! { static LAST: Cell<Option<Observation>> = const { Cell::new(None) }; }
pub(super) fn observe(ctx: &Context<'_, '_>, ordinal: usize) {
    let region = &ctx.control.descriptor.line_matrices.regions[ordinal];
    let view = ctx.line_matrices.views[ordinal].as_ref();
    LAST.with(|last| {
        last.set(Some(Observation {
            private: view.is_some(),
            matrix_present: ctx
                .arena
                .test_current_value(region.matrix().get())
                .is_some(),
            rows: view.map_or(0, |view| view.count()),
        }))
    });
}

fn argument(
    artifact: &mech_engine::ProgramArtifact,
    count: u64,
    memory: &ManagedMemoryBudget,
) -> Value {
    let value = ValueDraft {
        schema: artifact.inputs()[0].schema,
        shape_values: Box::new([]),
        data: ValueDataDraft::U64(count),
    }
    .finalize(&SnapshotValidationContext::new(artifact.schemas()))
    .unwrap();
    let mut claim = memory
        .reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap())
        .unwrap();
    value.into_memory_budget(&mut claim).unwrap()
}

#[test]
fn emitted_scanner_uses_no_matrix_and_preserves_owner_over_activation_and_errors() {
    for text in ["stop\nx\nstop", "stop\né\nstop"] {
        let source = format!(
            "+> text\n+> sequence\n#Scan(lines<[string]>, n<u64>) => <u64>\n  | :Step(i<u64>, found<u64>)\n  | :Done(found<u64>).\n#Scan(lines, n) -> :Step(1u64, 0u64)\n  :Step(i, found)\n    | i > n -> :Done(found)\n    | lines[i] == \"stop\" -> :Step(i + 1u64, found + 1u64)\n    | * -> :Step(i + 1u64, found)\n  :Done(found) => found.\nlines := text/lines({})\nfound := #Scan(lines, count<u64>)\ntext/join(lines[1u64..=found], \"|\")\n",
            serde_json::to_string(text).unwrap()
        );
        let original = artifact_source(&source);
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(
            &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
        )
        .unwrap();
        for artifact in [original, decoded] {
            let catalog = mech_stdlib::source_catalog();
            let object = crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
            let identity = *object.identity().bytes();
            assert_eq!(u64::from_le_bytes(identity[16..24].try_into().unwrap()), 36);
            assert_eq!(u64::from_le_bytes(identity[24..32].try_into().unwrap()), 20);
            assert_eq!(u64::from_le_bytes(identity[32..40].try_into().unwrap()), 54);
            for (offset, preceding) in [
                (16, 20u64),
                (16, 21),
                (16, 22),
                (16, 23),
                (16, 24),
                (16, 25),
                (16, 26),
                (16, 27),
                (16, 28),
                (16, 29),
                (24, 8),
                (24, 9),
                (24, 10),
                (24, 11),
                (24, 12),
                (24, 13),
                (32, 30),
                (32, 31),
                (32, 32),
                (32, 33),
                (32, 34),
                (32, 35),
                (32, 36),
                (32, 37),
                (32, 38),
                (24, 14),
                (32, 39),
                (32, 40),
                (32, 41),
                (32, 42),
                (32, 43),
                (32, 44),
                (32, 45),
                (16, 30),
                (24, 15),
                (32, 46),
                (16, 31),
                (24, 16),
                (32, 47),
                (32, 48),
                (16, 32),
                (24, 17),
                (32, 50),
                (32, 51),
                (16, 33),
                (24, 18),
                (32, 53),
            ] {
                let mut stale = identity;
                stale[offset..offset + 8].copy_from_slice(&preceding.to_le_bytes());
                assert!(
                    crate::native_artifact::program_linked::prepare_descriptor(
                        &artifact, &catalog, &stale,
                    )
                    .is_err()
                );
            }
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            assert_eq!(jit.descriptor().line_matrices.regions.len(), 1);
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let mut outputs = vec![None; artifact.outputs().len()];
            let input = argument(&artifact, 3, &memory);
            LAST.with(|last| last.set(None));
            jit.evaluate_into(&[input], &mut outputs, limits(), &memory)
                .unwrap();
            let observation = LAST.with(Cell::take).expect("actual producer visit");
            assert_eq!(observation.private, text.is_ascii());
            assert_eq!(observation.matrix_present, !text.is_ascii());
            if text.is_ascii() {
                assert_eq!(observation.rows, 3);
            }
            let ValueData::String(actual) = outputs.last().unwrap().as_ref().unwrap().data() else {
                panic!("canonical String output")
            };
            assert_eq!(
                actual.as_ref(),
                if text.is_ascii() { "stop|x" } else { "stop|é" }
            );
            let prior = outputs.last().unwrap().as_ref().unwrap().clone();
            let baseline = memory.used_bytes();
            let bad = argument(&artifact, 4, &memory);
            assert!(
                jit.evaluate_into(&[bad], &mut outputs, limits(), &memory)
                    .is_err()
            );
            assert!(
                outputs
                    .last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .shares_frozen_storage(&prior)
            );
            assert_eq!(memory.used_bytes(), baseline);
            let input = argument(&artifact, 3, &memory);
            jit.evaluate_into(&[input], &mut outputs, limits(), &memory)
                .unwrap();
            drop(prior);
            drop(outputs);
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}
