//! Full generated execution with no input String or line Matrix snapshot.
use crate::native_artifact::JitProgramArtifact;
use crate::native_artifact::program::runtime::session::prefix_tests::{
    assert_unloaned, limits, source, string_value,
};
use crate::native_artifact::program::runtime::session::*;

fn check(jit: &JitProgramArtifact, text: &str) -> u64 {
    let descriptor = jit.descriptor();
    let expected_memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
    let input = string_value(
        descriptor,
        descriptor.metadata().plan.transport_input_type(0).schema(),
        text,
        &expected_memory,
    );
    let (inputs, observations) = if descriptor.metadata().plan.inputs().len() == 1 {
        (vec![input], vec![])
    } else {
        (vec![], vec![input])
    };
    let mut expected = vec![None; descriptor.metadata().plan.transport_output_count()];
    let mut entry = jit.test_generated_entry();
    let mut ordinary = Prepared::new(descriptor, limits(), &expected_memory).unwrap();
    ordinary.activate(&mut entry).unwrap();
    ordinary
        .finish(&mut entry, &inputs, &observations, &mut expected)
        .unwrap();

    // Separate accounting and owners: the expected run's snapshot is never
    // imported or reachable from the candidate execution.
    let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
    let mut prepared = Prepared::new(descriptor, limits(), &memory).unwrap();
    prepared.activate(&mut entry).unwrap();
    let mut owner = prepared.begin_line_capture(text.len() as u64, 4).unwrap();
    let state = prepared.state.as_ref().unwrap();
    let ordinal = state.line_matrices.captured.as_ref().unwrap().ordinal;
    let lines = state
        .line_matrices
        .retained_demand(Some(ordinal), &memory)
        .unwrap();
    let collections = state
        .collections
        .as_ref()
        .map_or(mech_core::ResourceDemand::default(), |state| {
            state.retained_demand().unwrap()
        });
    let coexistence =
        crate::native_artifact::program::runtime::session::capture::capture_source_coexistence(
            state, ordinal, &memory,
        )
        .unwrap();
    assert_eq!(
        coexistence.turn_peak_bytes,
        lines.turn_peak_bytes + collections.turn_peak_bytes
    );
    assert_eq!(
        coexistence.retained_nodes,
        lines.retained_nodes + collections.retained_nodes
    );
    let source = descriptor
        .metadata()
        .prefix
        .as_ref()
        .unwrap()
        .frontier
        .read()
        .count();
    let start = descriptor.metadata().plan.transport_input_count();
    for count in source.counts() {
        prepared.storage.values_mut()[start + count.get() as usize] = u64::MAX;
    }
    let mut available_before = 0;
    let mut sealed = None;
    // Visit source/length and initialize the emitted scanner even for empty
    // input; its first header pauses before inspecting an unavailable cell.
    let initial = owner.borrow_window(1, 1).unwrap();
    prepared
        .run_prefix_window(&mut entry, 0, false, Some(&initial))
        .unwrap();
    drop(initial);
    for character in text.chars() {
        let mut bytes = [0; 4];
        owner.push(character.encode_utf8(&mut bytes)).unwrap();
        let available = owner.available_lines().unwrap();
        let first = if sealed.is_some() {
            available + 1
        } else {
            available_before + 1
        };
        let window = owner.borrow_window(first, available + 1).unwrap();
        prepared
            .run_prefix_window(&mut entry, available, false, Some(&window))
            .unwrap();
        assert_unloaned(&prepared);
        for count in source.counts() {
            assert_eq!(
                prepared.storage.values()[start + count.get() as usize],
                u64::MAX,
                "an unfinished final count was read/stored as a language value"
            );
        }
        assert!(
            prepared
                .state
                .as_ref()
                .unwrap()
                .arena
                .test_current_value(source.lines().source().get())
                .is_none()
        );
        assert!(
            prepared
                .state
                .as_ref()
                .unwrap()
                .arena
                .test_current_value(source.lines().matrix().get())
                .is_none()
        );
        drop(window);
        let retained = prepared.retention.unwrap();
        if let Some(end) = retained.end {
            if sealed.is_none() {
                owner.retain_range(retained.first, end).unwrap();
                sealed = Some(retained);
            }
        } else {
            owner.retain_from(retained.first).unwrap();
        }
        available_before = available;
    }
    let facts = owner.finish().unwrap();
    let view = prepared.admit_line_capture(owner).unwrap();
    assert!(view.source().is_none());
    assert!(view.rows().is_none());
    let first = if sealed.is_some() {
        facts.lines() + 1
    } else {
        available_before + 1
    };
    let window = view
        .captured_chunks()
        .unwrap()
        .borrow_window(first, facts.lines() + 1)
        .unwrap();
    prepared
        .run_prefix_window(&mut entry, facts.lines(), true, Some(&window))
        .unwrap();
    drop(window);
    drop(view);
    prepared.resume_capture_tail(&mut entry).unwrap();
    let mut actual = vec![None; expected.len()];
    let report = prepared.publish(&mut actual).unwrap();
    for (a, b) in actual.iter().zip(&expected) {
        assert_eq!(
            a.as_ref()
                .unwrap()
                .canonical_snapshot_bytes(&descriptor.metadata().schemas)
                .unwrap(),
            b.as_ref()
                .unwrap()
                .canonical_snapshot_bytes(&descriptor.metadata().schemas)
                .unwrap()
        );
    }
    drop(actual);
    assert_eq!(memory.used_bytes(), 0);
    drop((expected, inputs, observations));
    assert_eq!(expected_memory.used_bytes(), 0);
    report.fused_string_map_bytes
}

#[test]
fn original_and_decoded_scanners_and_join_need_no_complete_input_snapshot() {
    for artifact in
        crate::native_artifact::program::runtime::session::call_cache_tests::artifacts(source())
    {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        for text in [
            "skip\nopen\nA\nB\nclose\nafter\n",
            "",
            "missing\n",
            "open\nA",
            "skip\r\nopen\r\ne\u{301}\r\n猫\r\nclose\r\nafter\r",
        ] {
            check(&jit, text);
        }
    }
}

#[test]
#[cfg(feature = "canonical-native-program-host")]
fn full_knucleotide_original_and_decoded_execute_without_input_or_line_snapshots() {
    let mut compiler = mech_runtime::RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .host_factory(Box::new(mech_terminal::CliHostFactory::new().unwrap()))
        .unwrap()
        .host_instance(mech_runtime::HostInstanceConfig {
            name: "cli".into(),
            provider: "cli".into(),
            settings: mech_runtime::ConfigValue::Map(Default::default()),
        })
        .build_compiler()
        .unwrap();
    let product = compiler
        .compile_canonical_source(include_str!(
            "../../../../../tests/fixtures/native/knucleotide-idiomatic.mec"
        ))
        .unwrap();
    for artifact in [
        product.artifact().clone(),
        mech_engine::decode_program_artifact_bytecode_v1(product.bytecode()).unwrap(),
    ] {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert_eq!(
            check(
                &jit,
                ">ONE\nAAA\n>TWO\nCCCC\n>THREE\nggtattttaatttatagt\nggtac\n>FOUR\nT\n",
            ),
            23
        );
    }
}

pub(in crate::native_artifact::program::runtime::session) fn mapped_scan(
    separator: &str,
    shared: bool,
) -> String {
    let body = source()
        .strip_suffix("result\n")
        .unwrap()
        .replace(
            "rows := text/lines",
            concat!(
                r#"entries := {"a": "A", "b": "B", "x": "\r", "y": "\n", "\r": "R", "é": "É"}"#,
                "\nrows := text/lines"
            ),
        )
        .replace(
            "text/join(rows[first..last], \"|\")",
            "text/join(rows[first..last], \"\")",
        );
    format!(
        "+> map\n{body}normalized := text/join([entries{{ch}} | ch <- result], {separator:?})\n{}\n",
        if shared {
            "(result, normalized)"
        } else {
            "normalized"
        }
    )
}

#[test]
fn fused_join_map_captured_chunks_preserve_original_decoded_unicode_crlf_and_empty_results() {
    for artifact in crate::native_artifact::program::runtime::session::call_cache_tests::artifacts(
        &mapped_scan("", false),
    ) {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert_eq!(
            jit.descriptor()
                .metadata()
                .selected_joins
                .maps
                .iter()
                .flatten()
                .count(),
            1
        );
        for (text, bytes) in [
            ("open\nab\nba\nclose\n", 4),
            ("open\r\na\r\r\nb\r\nclose\r\n", 3),
            ("open\nxy\nclose\n", 2),
            ("open\né\nb\nclose\n", 0),
            ("", 0),
            ("missing\n", 0),
            ("open\n", 0),
            ("ignoredé\nopen\nab\nclose\n", 0),
            ("open\nab", 2),
            ("open\n\nab\n\nclose\n", 2),
        ] {
            assert_eq!(check(&jit, text), bytes, "{text:?}");
        }
    }
}

#[test]
fn fused_join_map_keeps_observed_sources_and_nonempty_map_join_separators_on_the_ordinary_path() {
    for source in [mapped_scan("", true), mapped_scan(",", false)] {
        for artifact in
            crate::native_artifact::program::runtime::session::call_cache_tests::artifacts(&source)
        {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            assert_eq!(check(&jit, "open\nab\nclose\n"), 0);
        }
    }
}
