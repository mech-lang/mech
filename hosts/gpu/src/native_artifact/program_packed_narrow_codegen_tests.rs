//! Actual generated narrow-slot probes, growth, wide counts and restored ABI.
use super::*;

#[test]
fn narrow_generated_collisions_preserve_all_keys_and_full_u64_counts() {
    use crate::native_artifact::program_packed_map::{hash, slot_bytes};
    // Eight ordinary letters have an injective three-bit low window. Build
    // same-fingerprint keys whose initial group wraps at both capacities.
    let mut candidates: [Vec<Vec<u8>>; 128] = std::array::from_fn(|_| Vec::new());
    for mut ordinal in 0..65536u64 {
        let mut key = vec![b'a'; 6];
        for byte in key.iter_mut().rev() {
            *byte = b'a' + (ordinal & 7) as u8;
            ordinal >>= 3;
        }
        let low = key
            .iter()
            .fold(0u64, |word, byte| (word << 3) | u64::from(byte & 7));
        let mixed = hash(low, 0);
        if mixed & 15 == 15 {
            candidates[(mixed >> 57) as usize].push(key);
        }
    }
    let keys = candidates
        .into_iter()
        .find(|keys| keys.len() >= 9)
        .expect("collision fixture");
    let keys = &keys[..9];
    let text = String::from_utf8(
        keys.iter()
            .chain(keys.iter().rev())
            .flat_map(|key| key.iter().copied())
            .collect(),
    )
    .unwrap();
    let count = structural_projection_tests::COUNT
        .replace(
            ":Next(index + 1u64, increment",
            ":Next(index + 6u64, increment",
        )
        .replace("previous + 1u64", "previous + 9223372036854775807u64");
    let source = format!(
        r#"{count}
+> text
letters := {{"a":"a", "b":"b", "c":"c", "d":"d", "e":"e", "f":"f", "g":"g", "h":"h"}}
sequence := text/join([letters{{char}} | char <- text<string>], "")
#Count(sequence, 6u64, {}u64, empty)
"#,
        text.len()
    );
    let original = tests::artifact_source(&source);
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    for artifact in [original, decoded] {
        let catalog = mech_stdlib::source_catalog();
        let jit = crate::native_artifact::JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let image = jit.descriptor().encode_build_owned_image().unwrap();
        // SAFETY: image, descriptor and generated entry have this exact owner.
        let restored =
            unsafe { NativeProgramDescriptor::decode_build_owned_image(&image, &catalog) }.unwrap();
        for descriptor in [jit.descriptor(), &restored] {
            let proof = descriptor
                .metadata()
                .closed_tails
                .regions
                .iter()
                .find_map(|r| r.packed.as_ref())
                .unwrap();
            assert_eq!(
                (
                    proof.packing_domain().bits(),
                    proof.packing_domain().shift()
                ),
                (3, Some(0))
            );
            assert_eq!(slot_bytes(proof.packing_domain()), 16);
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            assert_eq!(artifact.inputs().len(), 1);
            let value = ValueDraft {
                schema: artifact.inputs()[0].schema,
                shape_values: Box::new([]),
                data: ValueDataDraft::String(text.clone().into()),
            }
            .finalize(&SnapshotValidationContext::new(artifact.schemas()))
            .unwrap();
            let mut reservation = memory
                .reserve_capacity(value.memory_budget_admission_bytes(&memory).unwrap())
                .unwrap();
            let inputs = [value.into_memory_budget(&mut reservation).unwrap()];
            drop(reservation);
            let mut output = vec![None];
            let entry = jit.test_checked_entry();
            let mut limits = tests::limits();
            limits.execution.max_work = 1 << 38;
            limits.library.max_compute_work = Some(1 << 36);
            limits.library.max_comparison_work = Some(1 << 32);
            let report = execute(
                descriptor,
                |frame| unsafe { entry(frame) },
                &inputs,
                &mut output,
                limits,
                &memory,
            )
            .unwrap();
            assert_eq!(
                (
                    report.packed_map_regions,
                    report.packed_ssa_regions,
                    report.packed_map_updates
                ),
                (1, 1, 18)
            );
            let ValueData::Map(map) = output[0].as_ref().unwrap().data() else {
                panic!("map");
            };
            let actual = map
                .entries()
                .iter()
                .map(|entry| match (entry.key().data(), entry.value()) {
                    (ValueData::String(key), ValueData::U64(value)) => (key.to_string(), *value),
                    _ => panic!("String/U64 map"),
                })
                .collect::<std::collections::BTreeMap<_, _>>();
            let expected = keys
                .iter()
                .map(|key| (std::str::from_utf8(key).unwrap().to_owned(), u64::MAX - 1))
                .collect::<std::collections::BTreeMap<_, _>>();
            assert_eq!(actual, expected);
            drop((output, inputs));
            assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
        }
    }
}
