//! The counting traversal, increment and state transitions are compiled Mech.
use super::*;
use crate::native_artifact::JitProgramArtifact;
use std::collections::BTreeMap;

pub(super) const COUNT: &str = r#"
+> map
increment(h<{string:u64}>, key<string>) => (out<{string:u64}>) :=
  previous := (key ∈ h)? | true => h{key} | false => 0u64.
  out := map/insert(h, key, previous + 1u64).
#Count(s<string>, width<u64>, n<u64>, initial<{string:u64}>) => <{string:u64}>
  | :Next(index<u64>, counts<{string:u64}>)
  | :Done(counts<{string:u64}>).
#Count(s, width, n, initial) -> :Next(0u64, initial)
  :Next(index, counts)
    | width == 0u64 -> :Done(counts)
    | index >= n -> :Done(counts)
    | width > n - index -> :Done(counts)
    | * -> :Next(index + 1u64, increment(counts, s[(index + 1u64)..=(index + width)]))
  :Done(counts) => counts.
empty<{string:u64}> := map/new()
"#;

#[test]
fn native_unconditional_map_states_execute_without_source_boolean_expressions() {
    let source = include_str!("../../tests/fixtures/native/map-state-two-updates.mec");
    let original = tests::artifact_source(source);
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    assert_eq!(
        bytes,
        mech_engine::encode_program_artifact_bytecode_v1(&decoded).unwrap()
    );
    for artifact in [original, decoded] {
        assert!(
            artifact
                .schemas()
                .entries()
                .any(|entry| entry.schema().body() == &mech_core::SchemaBody::Bool)
        );
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
        let mut outputs = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&[], &mut outputs, tests::limits(), &memory)
            .unwrap();
        assert_eq!(outputs.len(), 1);
        let ValueData::Map(map) = outputs[0].as_ref().unwrap().data() else {
            panic!("map");
        };
        assert_eq!(map.entries().len(), 2);
        for (entry, (key, value)) in map.entries().iter().zip([("one", 11), ("two", 22)]) {
            assert!(matches!((entry.key().data(), entry.value()),
                (ValueData::String(actual), ValueData::U64(count)) if actual.as_ref() == key && *count == value));
        }
        drop((outputs, jit));
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn native_structural_map_states_execute_all_seven_counting_widths() {
    let widths = [1usize, 2, 3, 4, 6, 12, 18];
    let source = format!("{COUNT}\n#Count(text<string>, width<u64>, n<u64>, empty)\n");
    let original = tests::artifact_source(&source);
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    assert_eq!(
        bytes,
        mech_engine::encode_program_artifact_bytecode_v1(&decoded).unwrap()
    );
    for artifact in [original, decoded] {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let image = jit.descriptor().encode_build_owned_image().unwrap();
        assert_eq!(jit.descriptor().map_owner_words.as_ref().unwrap().spans().len(), 1);
        // SAFETY: these metadata bytes come from this exact unchanged checked
        // compilation, and remain confined to this test's matching JIT owner.
        let restored = unsafe {
            NativeProgramDescriptor::decode_build_owned_image(
                &image,
                &mech_stdlib::source_catalog(),
            )
        }
        .unwrap();
        assert_eq!(
            restored.typed_ir_sha256(),
            jit.descriptor().typed_ir_sha256()
        );
        let object =
            crate::native_artifact::emit_program_object(&artifact, &mech_stdlib::source_catalog())
                .unwrap();
        let identity = object.identity().bytes();
        assert_eq!(u64::from_le_bytes(identity[16..24].try_into().unwrap()), 40);
        assert_eq!(u64::from_le_bytes(identity[24..32].try_into().unwrap()), 21);
        assert_eq!(u64::from_le_bytes(identity[32..40].try_into().unwrap()), 61);
        assert!(jit.descriptor().plan().blocks().iter().any(|block| matches!(block.action(), NativeIterationAction::Structural { observation: mech_engine::native_artifact::NativeStructuralObservation::CanonicalValue, .. })));
        for units in [
            "ACGTACGTNACGTTACGTACGTACGT".split("").filter(|s| !s.is_empty()).collect::<Vec<_>>(),
            vec![], vec!["G", "G", "T", "A", "G", "G", "T"],
            vec!["A", "é", "A", "é", "x"], vec!["A", "\r\n", "B", "A"],
            vec!["e\u{301}", "X", "e\u{301}", "X"],
        ] {
            let text = units.concat();
            for width in widths {
                let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
                let inputs = artifact
                    .inputs()
                    .iter()
                    .map(|input| {
                        let name = mech_engine::decode_source_input_name(&input.name).unwrap();
                        let data = match name.as_str() {
                            "text" => ValueDataDraft::String(text.clone().into()),
                            "n" => ValueDataDraft::U64(units.len() as u64),
                            "width" => ValueDataDraft::U64(width as u64),
                            _ => panic!("unexpected {name}"),
                        };
                        let value = ValueDraft {
                            schema: input.schema,
                            shape_values: Box::new([]),
                            data,
                        }
                        .finalize(&SnapshotValidationContext::new(artifact.schemas()))
                        .unwrap();
                        let mut reservation = memory
                            .reserve_capacity(value.memory_budget_admission_bytes(&memory).unwrap())
                            .unwrap();
                        value.into_memory_budget(&mut reservation).unwrap()
                    })
                    .collect::<Vec<_>>();
                let mut output = vec![None; artifact.outputs().len()];
                let mut limits = tests::limits();
                limits.library.max_comparison_work = Some(10_000_000);
                let updates = if width <= units.len() { units.len() - width + 1 } else { 0 };
                map_owners::tests::evaluate_count_case(&jit, &inputs, &mut output, limits, &memory, updates as u64)
                    .unwrap();
                let mut native_output = vec![None; artifact.outputs().len()];
                let report = jit.evaluate_into(&inputs, &mut native_output, limits, &memory).unwrap();
                assert_eq!(report.hash_table_updates, updates as u64);
                let mut restored_output = vec![None; artifact.outputs().len()];
                let entry = jit.test_checked_entry();
                // SAFETY: restored metadata was produced by this unchanged JIT.
                let report = execute(&restored, |frame| unsafe { entry(frame) },
                    &inputs, &mut restored_output, limits, &memory).unwrap();
                assert_eq!(report.hash_table_updates, updates as u64);
                assert_eq!(output.len(), 1);
                for actual in output.iter().chain(&native_output).chain(&restored_output) {
                    let mut expected = BTreeMap::<String, u64>::new();
                    if width <= units.len() {
                        for window in units.windows(width) {
                            *expected.entry(window.concat()).or_default() += 1;
                        }
                    }
                    let ValueData::Map(map) = actual.as_ref().unwrap().data() else {
                        panic!("map result")
                    };
                    let counts = map
                        .entries()
                        .iter()
                        .map(|entry| {
                            let (ValueData::String(key), ValueData::U64(count)) =
                                (entry.key().data(), entry.value())
                            else {
                                panic!("string/u64 entry")
                            };
                            (key.to_string(), *count)
                        })
                        .collect::<BTreeMap<_, _>>();
                    assert_eq!(counts, expected, "{text:?} width {width}");
                }
            }
        }
    }
}

#[test]
fn native_structural_metadata_names_only_a_selected_window_accumulator() {
    let window = include_str!("../../tests/fixtures/native/window-histogram.mec").replace(
        "histogram(input<string>, width<u64>)",
        "histogram(input<string>, 3u64)",
    );
    for (source, selected) in [
        ("+> sequence\nsequence/length(input<string>)\n", false),
        (window.as_str(), true),
    ] {
        let original = tests::artifact_source(source);
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(
            &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
        )
        .unwrap();
        for artifact in [original, decoded] {
            let object = crate::native_artifact::emit_program_object(
                &artifact,
                &mech_stdlib::source_catalog(),
            )
            .unwrap();
            let metadata = object.metadata();
            assert_eq!(
                metadata["compiled_window_histogram_regions"]
                    .as_u64()
                    .unwrap()
                    > 0,
                selected
            );
            if selected {
                assert_eq!(
                    metadata["compiled_window_accumulator"],
                    "hashbrown-0.15.5-entry-updates"
                );
            } else {
                assert!(metadata["compiled_window_accumulator"].is_null());
            }
        }
    }
}

#[test]
fn map_key_projection_declines_other_key_observers_and_preserves_bounds_failure() {
    let base = format!("{COUNT}\n#Count(\"abba\", 2u64, 4u64, empty)\n");
    let observed = base.replace("+> map", "+> map\n+> sequence")
        .replace("previous + 1u64", "previous + sequence/length(key)");
    for (source, projections, succeeds) in [
        (observed, 0, true),
        (base.replace("4u64, empty", "5u64, empty"), 1, false),
        (base.replace("index + 1u64)..=", "index)..="), 1, false),
    ] {
        let original = tests::artifact_source(&source);
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(
            &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap()).unwrap();
        for artifact in [original, decoded] {
            let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            assert_eq!(jit.descriptor().map_owner_words.as_ref().unwrap().spans().len(), projections);
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let mut outputs = vec![None; artifact.outputs().len()];
            let result = jit.evaluate_into(&[], &mut outputs, tests::limits(), &memory);
            if succeeds {
                result.unwrap();
                let ValueData::Map(map) = outputs[0].as_ref().unwrap().data() else { panic!("map") };
                assert_eq!(map.entries().len(), 3);
                assert!(map.entries().iter().all(|entry| matches!(entry.value(), ValueData::U64(2))));
            } else {
                let error = result.unwrap_err();
                assert!(matches!(error, NativeProgramExecutionError::Library { .. }), "{error:?}");
                assert!(outputs.iter().all(Option::is_none));
            }
            drop(outputs);
            assert_eq!(memory.used_bytes(), 0);
            assert_eq!(memory.retained_snapshot_nodes(), 0);
        }
    }
}
