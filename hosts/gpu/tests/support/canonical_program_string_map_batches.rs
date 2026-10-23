//! Exercise the actual original/decoded compiler, canonical reference and an
//! independent byte-map oracle for longer cases beyond the resident work cap.
use super::*;

#[test]
fn compiled_byte_map_batches_cover_ascii_replacements_and_every_short_tail() {
    let chars = (0..128u8)
        .map(|byte| String::from_utf8(vec![byte]).unwrap())
        .collect::<Vec<_>>();
    let entries = (0..128)
        .map(|i| (chars[i].as_str(), chars[127 - i].as_str()))
        .collect::<Vec<_>>();
    let (variants, last) = artifacts(SOURCE);
    for artifact in variants {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert_eq!(jit.descriptor().plan().string_map_regions().len(), 1);
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        let mut visits = None;
        for len in (0..=129).chain([255, 256, 257, 4095, 4096, 4097]) {
            // Odd-stride permutations include NUL/CR/DEL and all byte values,
            // but never an adjacent CRLF. Replacement bytes cover all ASCII.
            let text = String::from_utf8(
                (0..len)
                    .map(|i| ((i * 37 + len * 13) % 128) as u8)
                    .collect(),
            )
            .unwrap();
            assert!(!text.as_bytes().windows(2).any(|pair| pair == b"\r\n"));
            let mapped = text
                .bytes()
                .map(|byte| char::from(127 - byte))
                .collect::<String>();
            let args = inputs(
                &artifact,
                &[
                    ("source", Data::String(text)),
                    ("entries", dictionary(&entries)),
                    ("separator", Data::String("".into())),
                ],
                &memory,
            );
            // The generic 128-entry map on 4K characters exceeds the
            // ordinary oracle's finite work cap. Keep that cap unchanged:
            // compare all short cases to it and every case to the byte oracle.
            let expected = (len <= 257).then(|| {
                reference_result(&artifact, &args)
                    .unwrap_or_else(|error| panic!("len={len}: {error}"))
            });
            let report = jit
                .evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            if let Some(expected) = expected {
                equal(&artifact, &output, &expected);
            }
            let mech_core::ValueData::String(actual) = output[last].as_ref().unwrap().data() else {
                panic!("expected canonical String");
            };
            assert_eq!(actual.as_ref(), mapped, "len={len}");
            if len > 0 {
                assert_eq!(*visits.get_or_insert(report.visits), report.visits);
            }
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn compiled_byte_map_batch_missing_keys_preserve_prior_owners_and_recover() {
    for artifact in artifacts(SOURCE).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert_eq!(jit.descriptor().plan().string_map_regions().len(), 1);
        let projection = jit
            .descriptor()
            .plan()
            .collections()
            .iter()
            .find_map(|c| c.string_element_map_projection())
            .unwrap();
        let origin = projection.selection_template().origin();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        let args_for = |text: String| {
            inputs(
                &artifact,
                &[
                    ("source", Data::String(text)),
                    ("entries", dictionary(&[("a", "\0"), ("b", "\u{7f}")])),
                    ("separator", Data::String("".into())),
                ],
                &memory,
            )
        };
        let good = args_for("ab".repeat(32));
        jit.evaluate_into(&good, &mut output, limits(), &memory)
            .unwrap();
        // Every position within several batches, including a later missing
        // byte in the same batch. Failure must not publish its partial output.
        for position in 0..64 {
            let mut text = vec![b'a'; 64];
            text[position] = b'?';
            if position + 1 < text.len() {
                text[position + 1] = b'!';
            }
            let bad = args_for(String::from_utf8(text).unwrap());
            let expected = reference_result(&artifact, &bad).unwrap_err();
            assert!(expected.contains("InvalidInput"), "{expected}");
            let prior = output.clone();
            let before = memory.used_bytes();
            let error = jit
                .evaluate_into(&bad, &mut output, limits(), &memory)
                .unwrap_err();
            assert!(
                matches!(error, NativeProgramExecutionError::Library { origin: actual,
                    error: NativeCollectionInvocationError::Kernel(ResidentKernelError::InvalidInput) }
                    if actual == origin),
                "{error:?}"
            );
            for (actual, previous) in output.iter().zip(&prior) {
                assert!(
                    actual
                        .as_ref()
                        .unwrap()
                        .shares_frozen_storage(previous.as_ref().unwrap())
                );
            }
            assert_eq!(memory.used_bytes(), before);
            drop((bad, prior));
            jit.evaluate_into(&good, &mut output, limits(), &memory)
                .unwrap();
            equal(&artifact, &output, &reference(&artifact, &good));
        }
        drop((good, output));
        assert_eq!(memory.used_bytes(), 0);
    }
}
