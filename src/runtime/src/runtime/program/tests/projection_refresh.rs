use super::*;

fn load(source: &str, decoded: bool) -> crate::MechRuntime {
    let mut loaded = runtime();
    loaded
        .load_interactive_source_program(source, crate::ResidentDurabilityPolicy::Retained)
        .unwrap();
    if !decoded {
        return loaded;
    }
    let ActiveProgramExecution::ResidentPure(execution) = &loaded.active_program else {
        panic!("projection fixture must remain a pure resident program")
    };
    let artifact = decode_program_artifact_bytecode_v1(
        &encode_program_artifact_bytecode_v1(&execution.artifact).unwrap(),
    )
    .unwrap();
    let mut loaded = runtime();
    loaded
        .load_compiled_program(artifact, crate::ResidentDurabilityPolicy::Retained)
        .unwrap();
    loaded
}

#[test]
fn compatible_replacement_refreshes_new_control_outputs_without_advancing_state() {
    let source = "~state := 1.0\nstate = state + 1.0\nstate\n";
    for decoded in [false, true] {
        for (projection, expected) in [
            (
                "extra := (state ? | 0.0 => 0.0 | * => state + 10.0)\nextra\n",
                vec![14.0],
            ),
            (
                "extra := [state + item | item <- [1.0 2.0]]\nextra\n",
                vec![5.0, 6.0],
            ),
            (
                "increment(value<f64>) => <f64> := value + 1.0\nextra := state |2> increment\nextra\n",
                vec![6.0],
            ),
            (
                "increment(value<f64>) => <f64> := value + 1.0\nextra := state |[2]> increment\nextra\n",
                vec![6.0],
            ),
        ] {
            let mut previous = load(source, decoded);
            previous.step_active_program().unwrap();
            previous.step_active_program().unwrap();
            assert_eq!(
                canonical_f64(previous.root_symbol_value("state").unwrap().value()),
                4.0
            );
            let mut candidate = load(&format!("{source}{projection}"), decoded);
            let ActiveProgramExecution::ResidentPure(execution) = &candidate.active_program else {
                unreachable!()
            };
            let candidate_epoch = execution.instance.published_epoch();
            candidate
                .preserve_compatible_resident_state_from(&previous, &BTreeSet::new())
                .unwrap();
            let ActiveProgramExecution::ResidentPure(execution) = &candidate.active_program else {
                unreachable!()
            };
            assert_eq!(execution.instance.published_epoch(), candidate_epoch);
            assert_eq!(
                canonical_f64(candidate.root_symbol_value("state").unwrap().value()),
                4.0
            );
            let output = candidate.root_symbol_value("extra").unwrap();
            let actual = match output.value().data() {
                ValueData::F64(_) => vec![canonical_f64(output.value())],
                ValueData::Matrix(_) => canonical_f64_matrix(output.value()),
                other => panic!("unexpected projection {other:?}"),
            };
            assert_eq!(actual, expected, "decoded={decoded}: {projection}");
            assert_eq!(candidate.program_output_value().unwrap().unwrap(), output);
            candidate.step_active_program().unwrap();
            assert_eq!(
                canonical_f64(candidate.root_symbol_value("state").unwrap().value()),
                5.0
            );
        }
    }
}

#[test]
fn replacement_keeps_ordered_candidate_projections_of_explicitly_mutated_state() {
    let source = "~counter := 0.0\ncounter += 1.0\n~steady := 5.0\nsteady += 1.0\ndisplay := counter + 10.0\ndisplay\n";
    for decoded in [false, true] {
        let mut previous = load(source, decoded);
        previous.step_active_program().unwrap();
        previous.step_active_program().unwrap();
        assert_eq!(
            canonical_f64(previous.root_symbol_value("display").unwrap().value()),
            13.0
        );
        let mut candidate = load(
            &format!(
                "{source}counter += 1.0\nextra := steady + 100.0\nsafe! := display < 12.0\nextra\n"
            ),
            decoded,
        );
        let epoch = resident_parts(&candidate).1.published_epoch();
        candidate
            .preserve_compatible_resident_state_from(
                &previous,
                &BTreeSet::from(["counter".to_owned()]),
            )
            .unwrap();
        assert_eq!(resident_parts(&candidate).1.published_epoch(), epoch);
        for (name, expected) in [
            ("counter", 2.0),
            ("display", 11.0),
            ("steady", 8.0),
            ("extra", 108.0),
        ] {
            assert_eq!(
                canonical_f64(candidate.root_symbol_value(name).unwrap().value()),
                expected,
                "decoded={decoded}, projection={name}",
            );
        }
        assert_eq!(
            canonical_f64(previous.root_symbol_value("counter").unwrap().value()),
            3.0
        );
        assert_eq!(
            canonical_f64(previous.root_symbol_value("display").unwrap().value()),
            13.0
        );
    }
}

#[test]
fn replacement_keeps_hidden_invariant_order_around_an_explicit_state_edit() {
    for decoded in [false, true] {
        for retained_state in [false, true] {
            let steady = if retained_state {
                "~steady := 5.0\nsteady += 1.0\n"
            } else {
                "steady := 0.0\n"
            };
            let mut previous = load(
                &format!("~counter := 0.0\ncounter += 1.0\n{steady}counter\n"),
                decoded,
            );
            previous.step_active_program().unwrap();
            previous.step_active_program().unwrap();
            let mut candidate = load(
                &format!(
                    "~counter := 0.0\ncounter += 1.0\n{steady}safe! := counter + steady < 10.0\ncounter += 1.0\ncounter\n"
                ),
                decoded,
            );
            candidate
                .preserve_compatible_resident_state_from(
                    &previous,
                    &BTreeSet::from(["counter".to_owned()]),
                )
                .unwrap();
            assert_eq!(
                canonical_f64(candidate.root_symbol_value("counter").unwrap().value()),
                2.0
            );
            assert_eq!(
                canonical_f64(candidate.root_symbol_value("steady").unwrap().value()),
                if retained_state { 8.0 } else { 0.0 }
            );
            assert_eq!(
                canonical_f64(previous.root_symbol_value("counter").unwrap().value()),
                3.0
            );
        }
    }
}

#[test]
fn compatible_replacement_reuses_published_fsm_projection_without_resuming_it() {
    use mech_syntax::document::{DocumentId, ParseConfig, Revision, TextSnapshot};

    let source = r#"
@fast := test://clock/fast{:read(delta-seconds)}
fast := @fast/delta-seconds
~state := 1.0
#Deferred(value<f64>) => <f64>
  | :Start(value<f64>)
  | :Done(value<f64>).
#Deferred(value) -> :Start(value)
  :Start(value) ~> :Done(value)
  :Done(value) => value.
deferred := #Deferred(fast)
state
"#;
    // These are revisions of one document, not independently owned nominal
    // declarations. Preserve the same origin exactly as an interactive host.
    let origin = crate::SourceDocument::new_standalone_origin();
    let load = |source: &str, decoded: bool, origin: &mech_core::CanonicalNominalPath| {
        let (mut loaded, reads) = independent_external_runtime_with_source("true\n");
        loaded.unload_active_program().unwrap();
        let document = crate::SourceDocument::parse(
            TextSnapshot::new(DocumentId(0x901), Revision(1), source).unwrap(),
            ParseConfig::default(),
        )
        .with_nominal_origin(origin.clone());
        loaded
            .load_interactive_document_program(&document, crate::ResidentDurabilityPolicy::Retained)
            .unwrap();
        if decoded {
            let ActiveProgramExecution::ResidentExternal(execution) = &loaded.active_program else {
                unreachable!()
            };
            let artifact = decode_program_artifact_bytecode_v1(
                &encode_program_artifact_bytecode_v1(&execution.artifact).unwrap(),
            )
            .unwrap();
            loaded.unload_active_program().unwrap();
            loaded
                .load_compiled_program(artifact, crate::ResidentDurabilityPolicy::Retained)
                .unwrap();
        }
        (loaded, reads)
    };
    for decoded in [false, true] {
        let (mut previous, previous_reads) = load(source, decoded, &origin);
        previous
            .ingress()
            .submit(crate::RuntimeHostInput::single(
                RuntimeHostInputSource::new("test://clock/fast", "delta-seconds").unwrap(),
                RuntimeHostInputValue::F64(7.0),
            ))
            .unwrap();
        previous.drain_resident_host_inputs(1).unwrap();
        assert_eq!(
            canonical_f64(previous.root_symbol_value("deferred").unwrap().value()),
            7.0
        );

        let (mut candidate, candidate_reads) = load(
            &format!("{source}extra := deferred + state\nextra\n"),
            decoded,
            &origin,
        );
        let ActiveProgramExecution::ResidentExternal(execution) = &candidate.active_program else {
            unreachable!()
        };
        let epoch = execution.coordinator.instance().published_epoch();
        let ready = execution.coordinator.instance().ready_continuation_count();
        candidate
            .preserve_compatible_resident_state_from(&previous, &BTreeSet::new())
            .unwrap();
        assert_eq!(
            canonical_f64(candidate.root_symbol_value("deferred").unwrap().value()),
            7.0
        );
        assert_eq!(
            canonical_f64(candidate.root_symbol_value("extra").unwrap().value()),
            8.0
        );
        let ActiveProgramExecution::ResidentExternal(execution) = &candidate.active_program else {
            unreachable!()
        };
        assert_eq!(execution.coordinator.instance().published_epoch(), epoch);
        assert_eq!(
            execution.coordinator.instance().ready_continuation_count(),
            ready
        );
        assert_eq!(previous_reads.load(Ordering::SeqCst), 0);
        assert_eq!(candidate_reads.load(Ordering::SeqCst), 0);

        let unrelated_origin = crate::SourceDocument::new_standalone_origin();
        let (mut unrelated, unrelated_reads) = load(
            &format!("{source}extra := deferred + state\nextra\n"),
            decoded,
            &unrelated_origin,
        );
        unrelated
            .preserve_compatible_resident_state_from(&previous, &BTreeSet::new())
            .unwrap();
        assert!(unrelated.root_symbol_value("deferred").is_err());
        assert!(unrelated.root_symbol_value("extra").is_err());
        assert_eq!(unrelated_reads.load(Ordering::SeqCst), 0);
    }
}

#[test]
fn anonymous_state_migration_preserves_exact_identity_without_guessing_across_revisions() {
    let source =
        "~left := 1.0\n~right := 1.0\nleft = left + 1.0\nright = right + 1.0\nleft + right\n";
    for decoded in [false, true] {
        let configure = |source: &str| {
            let mut loaded = runtime();
            loaded
                .load_source_program(source, crate::ResidentDurabilityPolicy::Retained)
                .unwrap();
            if decoded {
                let bytes = encode_program_artifact_bytecode_v1(resident_parts(&loaded).0).unwrap();
                loaded.unload_active_program().unwrap();
                loaded
                    .load_bytecode_program(&bytes, crate::ResidentDurabilityPolicy::Retained)
                    .unwrap();
            }
            loaded
        };
        let mut previous = configure(source);
        previous.step_active_program().unwrap();
        previous.step_active_program().unwrap();
        assert_eq!(
            canonical_f64(previous.program_output_value().unwrap().unwrap().value()),
            8.0
        );
        let mut unchanged = configure(source);
        assert_eq!(
            resident_parts(&unchanged).0.revision(),
            resident_parts(&previous).0.revision()
        );
        unchanged
            .preserve_compatible_resident_state_from(&previous, &BTreeSet::new())
            .unwrap();
        assert_eq!(
            canonical_f64(unchanged.program_output_value().unwrap().unwrap().value()),
            8.0
        );
        unchanged.step_active_program().unwrap();
        assert_eq!(
            canonical_f64(unchanged.program_output_value().unwrap().unwrap().value()),
            10.0,
            "retained cells, not only a cached output, must migrate"
        );
        let mut candidate = configure(&format!("{source}safe! := left + right < 5.0\n"));
        assert_ne!(
            resident_parts(&candidate).0.revision(),
            resident_parts(&previous).0.revision()
        );
        candidate
            .preserve_compatible_resident_state_from(&previous, &BTreeSet::new())
            .unwrap();
        assert_eq!(
            canonical_f64(candidate.program_output_value().unwrap().unwrap().value()),
            4.0
        );
        assert_eq!(
            canonical_f64(previous.program_output_value().unwrap().unwrap().value()),
            8.0
        );
    }
}

#[test]
fn replacement_rejects_a_suspended_frame_and_leaves_the_old_runtime_resumable() {
    let source = "#Deferred() => <u64>\n  | :Start\n  | :Done.\n#Deferred() -> :Start\n  :Start ~> :Done\n  :Done => 42u64.\n#Deferred()\n";
    for decoded in [false, true] {
        let mut previous = load(source, decoded);
        let artifact = resident_parts(&previous).0.clone();
        let mut suspended = mech_engine::resident::activate(
            mech_core::ReactiveInstanceId::new(0x903, 0),
            &artifact,
            &mech_stdlib::source_catalog(),
            &mech_engine::resident::ActivationFacts::default(),
        )
        .unwrap();
        suspended.turn(&[]).unwrap();
        assert!(suspended.has_suspended_continuation());
        let wakeup = suspended.continuation_wakeup().unwrap();
        let epoch = suspended.published_epoch();
        let ActiveProgramExecution::ResidentPure(execution) = &mut previous.active_program else {
            unreachable!()
        };
        execution.instance = suspended;
        let mut candidate = load(source, decoded);
        let candidate_hash = resident_parts(&candidate).1.published_state_hash();
        let error = candidate
            .preserve_compatible_resident_state_from(&previous, &BTreeSet::new())
            .unwrap_err();
        assert!(
            error
                .display_message()
                .contains("SuspendedContinuationMigration"),
            "{error:?}"
        );
        assert_eq!(
            resident_parts(&candidate).1.published_state_hash(),
            candidate_hash
        );
        assert_eq!(resident_parts(&previous).1.published_epoch(), epoch);
        assert!(
            resident_parts(&previous)
                .1
                .accepts_continuation_wakeup(wakeup)
        );
        previous.step_active_program().unwrap();
        assert!(!resident_parts(&previous).1.has_suspended_continuation());
        assert_eq!(
            previous
                .program_output_value()
                .unwrap()
                .unwrap()
                .value()
                .canonical_data_draft()
                .unwrap(),
            ValueDataDraft::U64(42)
        );
        candidate
            .preserve_compatible_resident_state_from(&previous, &BTreeSet::new())
            .unwrap();
    }
}

#[test]
fn replacement_refreshes_renamed_outputs_without_any_state_mapping() {
    let original =
        "@fast := test://clock/fast{:read(delta-seconds)}\nold := @fast/delta-seconds + 1.0\nold\n";
    let replacement = "@fast := test://clock/fast{:read(delta-seconds)}\nnew := @fast/delta-seconds + 10.0\nnew\n";
    for decoded in [false, true] {
        let configure = |source| {
            let (mut loaded, reads) = independent_external_runtime_with_source(source);
            if decoded {
                let (artifact, _) = resident_parts(&loaded);
                let bytes = encode_program_artifact_bytecode_v1(artifact).unwrap();
                loaded.unload_active_program().unwrap();
                loaded
                    .load_bytecode_program(&bytes, crate::ResidentDurabilityPolicy::Retained)
                    .unwrap();
            }
            (loaded, reads)
        };
        let (mut previous, previous_reads) = configure(original);
        previous
            .ingress()
            .submit(crate::RuntimeHostInput::single(
                RuntimeHostInputSource::new("test://clock/fast", "delta-seconds").unwrap(),
                RuntimeHostInputValue::F64(7.0),
            ))
            .unwrap();
        previous.drain_resident_host_inputs(1).unwrap();
        assert_eq!(
            canonical_f64(previous.root_symbol_value("old").unwrap().value()),
            8.0
        );
        let (mut candidate, candidate_reads) = configure(replacement);
        let (artifact, instance) = resident_parts(&candidate);
        assert!(
            artifact
                .slots()
                .iter()
                .all(|slot| slot.role != SlotRole::State)
        );
        let named = artifact
            .interactive_symbol_bindings()
            .map(|binding| binding.lexical_name.as_str())
            .filter(|name| *name != "ans")
            .collect::<Vec<_>>();
        assert_eq!(named, ["new"]);
        let epoch = instance.published_epoch();
        candidate
            .preserve_compatible_resident_state_from(&previous, &BTreeSet::new())
            .unwrap();
        assert_eq!(
            canonical_f64(candidate.root_symbol_value("new").unwrap().value()),
            17.0
        );
        assert_eq!(resident_parts(&candidate).1.published_epoch(), epoch);
        assert_eq!(previous_reads.load(Ordering::SeqCst), 0);
        assert_eq!(candidate_reads.load(Ordering::SeqCst), 0);
    }
}

#[test]
fn replacement_leaves_new_observation_projections_unavailable_until_a_real_turn() {
    let original = "@fast := test://clock/fast{:read(delta-seconds)}\nfast := @fast/delta-seconds\n~state := 1.0\nstate = state + fast\nanswer := state + fast\nanswer\n";
    let additions = "@slow := test://clock/slow{:read(delta-seconds)}\nslow := @slow/delta-seconds\nnew := (slow ? | 0.0 => 1.0 | * => slow + state)\npositive! := new > 0.0\nindependent := state + 10.0\nindependent\n";
    for decoded in [false, true] {
        let configure = |source| {
            let (mut loaded, reads) = independent_external_runtime_with_source(source);
            if decoded {
                let (artifact, _) = resident_parts(&loaded);
                let bytes = encode_program_artifact_bytecode_v1(artifact).unwrap();
                loaded.unload_active_program().unwrap();
                loaded
                    .load_bytecode_program(&bytes, crate::ResidentDurabilityPolicy::Retained)
                    .unwrap();
            }
            (loaded, reads)
        };
        let (mut previous, previous_reads) = configure(original);
        previous
            .ingress()
            .submit(crate::RuntimeHostInput::single(
                RuntimeHostInputSource::new("test://clock/fast", "delta-seconds").unwrap(),
                RuntimeHostInputValue::F64(2.0),
            ))
            .unwrap();
        previous.drain_resident_host_inputs(1).unwrap();
        let (mut candidate, candidate_reads) = configure(&format!("{original}{additions}"));
        candidate
            .preserve_compatible_resident_state_from(&previous, &BTreeSet::new())
            .unwrap();
        assert_eq!(
            canonical_f64(candidate.root_symbol_value("state").unwrap().value()),
            3.0
        );
        assert_eq!(
            canonical_f64(candidate.root_symbol_value("independent").unwrap().value()),
            13.0
        );
        assert!(candidate.root_symbol_value("slow").is_err());
        assert!(candidate.root_symbol_value("new").is_err());
        assert_eq!(candidate_reads.load(Ordering::SeqCst), 0);
        candidate
            .ingress()
            .submit(crate::RuntimeHostInput::single(
                RuntimeHostInputSource::new("test://clock/slow", "delta-seconds").unwrap(),
                RuntimeHostInputValue::F64(5.0),
            ))
            .unwrap();
        candidate.drain_resident_host_inputs(1).unwrap();
        assert_eq!(
            canonical_f64(candidate.root_symbol_value("new").unwrap().value()),
            10.0
        );
        assert_eq!(
            canonical_f64(candidate.root_symbol_value("state").unwrap().value()),
            5.0
        );
        assert_eq!(previous_reads.load(Ordering::SeqCst), 0);
        assert_eq!(candidate_reads.load(Ordering::SeqCst), 0);
    }
}

#[test]
fn replacement_checks_unpublished_document_invariants_against_migrated_state() {
    let source = "~state := 1.0\nstate = state + 1.0\nstate\n";
    for decoded in [false, true] {
        for extra in ["", "extra := state + 10.0\nextra\n"] {
            let configure = |source: &str| {
                let mut loaded = runtime();
                loaded
                    .load_source_program(source, crate::ResidentDurabilityPolicy::Retained)
                    .unwrap();
                if decoded {
                    let (artifact, _) = resident_parts(&loaded);
                    let bytes = encode_program_artifact_bytecode_v1(artifact).unwrap();
                    loaded.unload_active_program().unwrap();
                    loaded
                        .load_bytecode_program(&bytes, crate::ResidentDurabilityPolicy::Retained)
                        .unwrap();
                }
                loaded
            };
            let mut previous = configure(source);
            previous.step_active_program().unwrap();
            previous.step_active_program().unwrap();
            let before = previous.program_output_value().unwrap().unwrap();
            assert_eq!(canonical_f64(before.value()), 4.0);
            let mut candidate = configure(&format!("{source}valid! := state < 3.0\n{extra}"));
            let error = candidate
                .preserve_compatible_resident_state_from(&previous, &BTreeSet::new())
                .expect_err("a predicate true for activation state2 must reject migrated state4");
            assert!(error.display_message().contains("Integrity"), "{error:?}");
            assert_eq!(previous.program_output_value().unwrap().unwrap(), before);
            previous.step_active_program().unwrap();
            assert_eq!(
                canonical_f64(previous.program_output_value().unwrap().unwrap().value()),
                5.0
            );
        }
    }
}
