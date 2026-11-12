//! Physical omission preserves the full logical setup/admission contract.
use super::*;
use crate::native_artifact::program::runtime::tests::artifact_source;
use crate::native_artifact::{JitProgramArtifact, NativeProgramLimitMode};

#[test]
fn absent_projection_categories_omit_only_physical_carriers_in_original_decoded_and_restored_plans()
{
    for (source, spans, maps) in [
        ("[i | i <- 0u64..count<u64>]\n", false, false),
        (
            "+> text\ntext/join([(seed<string>)[1u64..=2u64] | attempt <- 1u64..=2u64], \"|\")\n",
            true,
            false,
        ),
        (
            "+> text\n+> map\ntext/join([(entries<{string:string}>){piece} | piece <- seed<string>], \"|\")\n",
            false,
            true,
        ),
    ] {
        let original = artifact_source(source);
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(
            &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
        )
        .unwrap();
        for artifact in [original, decoded] {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let restored =
                crate::native_artifact::program::runtime::tests::restore_admitted_helper_descriptor(
                    &jit,
                );
            for descriptor in [jit.descriptor(), &restored] {
                let count = descriptor.metadata().plan.collections().len();
                assert_ne!(count, 0);
                let bytes = State::metadata_bytes(descriptor).unwrap();
                let limits = NativeProgramLimitMode::NoLimits.execution().library;
                let memory = ManagedMemoryBudget::new(bytes);
                let mut work = NativeWorkMeter::new(u64::MAX).unwrap();
                let state = State::new(descriptor, limits, &memory, &mut work)
                    .unwrap()
                    .unwrap();
                assert_eq!(state.string_spans.len(), if spans { count } else { 0 });
                assert_eq!(state.string_maps.len(), if maps { count } else { 0 });
                assert_eq!(
                    state.sources.len(),
                    descriptor.metadata().plan.iterations().len()
                );
                assert_eq!(state.prepared_ranges.len(), state.sources.len());
                assert_eq!(state.builders.len(), count);
                assert!(state.is_complete());
                assert_eq!(state.live_bytes, bytes);
                assert_eq!(memory.used_bytes(), bytes);
                assert_eq!(memory.retained_nodes(), bytes);
                assert_eq!(work.used(), bytes * 8 + 256);
                drop(state);
                assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));

                // Even a physically absent category retains its old exact
                // byte/node floors and refusal cleanup on the same account.
                for tight in [
                    memory.with_limits(bytes - 1, u64::MAX),
                    memory.with_limits(bytes, bytes - 1),
                ] {
                    let mut work = NativeWorkMeter::new(u64::MAX).unwrap();
                    assert!(State::new(descriptor, limits, &tight, &mut work).is_err());
                    assert_eq!(work.used(), bytes * 8 + 256);
                    assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
                }
                let mut work = NativeWorkMeter::new(bytes * 8 + 255).unwrap();
                assert!(State::new(descriptor, limits, &memory, &mut work).is_err());
                assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));

                let mut temporary = limits;
                temporary.max_temporary_bytes = Some(bytes - 1);
                let mut work = NativeWorkMeter::new(u64::MAX).unwrap();
                assert!(State::new(descriptor, temporary, &memory, &mut work).is_err());
                assert_eq!(work.used(), 0);
                assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
            }
        }
    }
}
