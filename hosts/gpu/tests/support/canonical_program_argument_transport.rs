//! Ordinary calls borrow immutable arguments without packing full buffer copies.
use super::{
    JitProgramArtifact, ManagedMemoryBudget, ValueDataDraft as Data, allocation_probe, artifacts,
    equal, inputs, limits, reference_result,
};
use mech_gpu::native_artifact::{NativeIterationExecutionError, NativeProgramExecutionError};

#[test]
fn large_checked_arguments_keep_bounded_allocations_aliases_and_recovery() {
    let source = "+> sequence\nsize(table<[u16]>, cache<[u32]>, n<u64>) => (out<u64>) :=\n  positive! := n > 0u64\n  out := sequence/length(table) + sequence/length(cache) + 12u64 / n.\nsize(table<[u16]>, cache<[u32]>, n<u64>)\n";
    let table_len = 262_144;
    let cache_len = 1_500_513;
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        let mut previous = Vec::new();
        for n in [3, 0, 2] {
            let named = [
                (
                    "table",
                    Data::Matrix((0..table_len).map(|i| Data::U16(i as u16)).collect()),
                ),
                (
                    "cache",
                    Data::Matrix((0..cache_len).map(|i| Data::U32(i as u32)).collect()),
                ),
                ("n", Data::U64(n)),
            ];
            let args = inputs(&artifact, &named, &memory);
            let aliases = args.clone();
            let before = memory.used_bytes();
            let (result, allocations, requested) = allocation_probe::measured_with_bytes(|| {
                jit.evaluate_into(&args, &mut output, limits(), &memory)
            });
            // The table alone is 524,288 bytes; the cache is larger still.
            // Keep the complete call, including setup and scalar publication,
            // below a copy of either buffer, without an allocator-exact count.
            assert!(
                requested < table_len * core::mem::size_of::<u16>(),
                "{allocations} requests totaling {requested} bytes: {result:?}"
            );
            let resident = reference_result(&artifact, &args);
            if n == 0 {
                assert!(
                    matches!(
                        &result,
                        Err(NativeProgramExecutionError::Control(
                            NativeIterationExecutionError::Integrity { .. }
                        ))
                    ),
                    "{result:?}"
                );
                assert!(resident.unwrap_err().contains("LexicalIntegrity"));
                equal(&artifact, &output, &previous);
                assert_eq!(memory.used_bytes(), before);
            } else {
                result.unwrap();
                equal(&artifact, &output, &resident.unwrap());
                assert_eq!(
                    output
                        .last()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    Data::U64((table_len + cache_len) as u64 + 12 / n)
                );
                previous = output.iter().map(|value| value.clone().unwrap()).collect();
            }
            for ((declaration, value), alias) in artifact.inputs().iter().zip(&args).zip(&aliases) {
                assert!(value.shares_frozen_storage(alias));
                let name = mech_engine::decode_source_input_name(&declaration.name).unwrap();
                assert_eq!(
                    &value.canonical_data_draft().unwrap(),
                    &named.iter().find(|(key, _)| *key == name).unwrap().1
                );
            }
        }
        drop((output, previous));
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn unused_unsigned_buffer_argument_keeps_its_check_before_caller_arithmetic() {
    let source = "argument(xs<[u16]>, n<u64>) => (out<[u16]>) :=\n  positive! := n > 0u64\n  out := xs.\ncaller(xs<[u16]>, ok<bool>) => (out<u64>) :=\n  allowed! := ok\n  out := 1u64 / 0u64.\ncaller(argument(input<[u16]>, n<u64>), ok<bool>)\n";
    super::lexical_invariant_qualification::check_cases(
        source,
        &[(
            vec![
                ("input", Data::Matrix(vec![Data::U16(7)].into())),
                ("n", Data::U64(0)),
                ("ok", Data::Bool(true)),
            ],
            None,
        )],
    );
}
