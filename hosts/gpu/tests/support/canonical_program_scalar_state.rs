//! Exact private scalar state used by real mixed-kind numeric loops.
use super::{artifacts, equal, inputs, limits, reference};
use mech_core::{ManagedMemoryBudget, ValueDataDraft as Data};
use mech_engine::native_artifact::NativeControlLoop;
use mech_gpu::native_artifact::{JitProgramArtifact, emit_program_object};

const SIGNED_PAIR: &str = "walk(left<i64>, total<i64>) => <i64>\n  | (left, total) => ((left > 0i64)? | true => walk(left - 1i64, total + 3i64) | false => total).\nwalk(steps<i64>, seed<i64>)\n";
const MIXED_ENUM: &str = "#Choose(n<u64>, best<u32>, item<u32>) => <u32>\n  | :Try(left<u64>, best<u32>, item<u32>)\n  | :Done(value<u32>).\n#Choose(n, best, item) -> :Try(n, best, item)\n  :Try(left, best, item)\n    | left > 0u64 -> :Try(left - 1u64, item, best)\n    | * -> :Done(best)\n  :Done(value) => value.\n#Choose(steps<u64>, best<u32>, item<u32>)\n";

fn check_scalar_loop(source: &str, signed: bool) {
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let plan = jit.descriptor().plan();
        let aggregates = plan.scalar_aggregates();
        let carrier = plan
            .iterations()
            .iter()
            .find_map(|control| match control {
                NativeControlLoop::Tail(t) => Some(t.carrier()),
                _ => None,
            })
            .unwrap();
        assert!(
            aggregates.slot(carrier).is_some(),
            "mixed scalar carrier must use private native words: signed={signed}"
        );
        let object = emit_program_object(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert_eq!(
            object.descriptor().typed_ir_sha256(),
            jit.descriptor().typed_ir_sha256()
        );
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        for steps in [0, 1, 2, 64] {
            let named = if signed {
                vec![("steps", Data::I64(steps)), ("seed", Data::I64(-211))]
            } else {
                vec![
                    ("steps", Data::U64(steps as u64)),
                    ("best", Data::U32(0xfedc_ba98)),
                    ("item", Data::U32(0x8765_4321)),
                ]
            };
            let args = inputs(&artifact, &named, &memory);
            let expected = reference(&artifact, &args);
            let mut output = vec![None; artifact.outputs().len()];
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            equal(&artifact, &output, &expected);
            let actual = output
                .last()
                .unwrap()
                .as_ref()
                .unwrap()
                .canonical_data_draft()
                .unwrap();
            assert_eq!(
                actual,
                if signed {
                    Data::I64(-211 + 3 * steps)
                } else {
                    Data::U32(if steps % 2 == 0 {
                        0xfedc_ba98
                    } else {
                        0x8765_4321
                    })
                }
            );
        }
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn signed_pair_loop_selects_existing_native_word_storage() {
    check_scalar_loop(SIGNED_PAIR, true);
}

#[test]
fn mixed_enum_loop_selects_existing_native_word_storage() {
    check_scalar_loop(MIXED_ENUM, false);
}
