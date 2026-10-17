//! Replacement projections use the input snapshot of the published state.
use super::*;

const SOURCE: &str = r#"
@fast := test://clock/fast{:read(delta-seconds)}
fast := @fast/delta-seconds
~state := 0.0
state += fast
safe! := fast >= 0.0
answer := state + fast
answer
"#;

// The new observation shares a host source with the old one, but its artifact
// identity and projection are new. It must inherit the published snapshot.
const EXTRA: &str = r#"
@fast-copy := test://clock/fast{:read(delta-seconds)}
new-fast := @fast-copy/delta-seconds
extra := state + new-fast + 10.0
extra
"#;

fn load(source: &str, decoded: bool) -> (crate::MechRuntime, Arc<AtomicUsize>) {
    let (mut runtime, reads) = independent_external_runtime_with_source(source);
    if decoded {
        let (artifact, _) = resident_parts(&runtime);
        let bytes = encode_program_artifact_bytecode_v1(artifact).unwrap();
        runtime.unload_active_program().unwrap();
        runtime
            .load_bytecode_program(&bytes, crate::ResidentDurabilityPolicy::Retained)
            .unwrap();
    }
    (runtime, reads)
}

fn value(runtime: &crate::MechRuntime, name: &str) -> f64 {
    canonical_f64(runtime.root_symbol_value(name).unwrap().value())
}

fn submit(runtime: &mut crate::MechRuntime, value: f64) -> crate::ResidentExternalTurnOutcome {
    runtime
        .ingress()
        .submit(crate::RuntimeHostInput::single(
            RuntimeHostInputSource::new("test://clock/fast", "delta-seconds").unwrap(),
            RuntimeHostInputValue::F64(value),
        ))
        .unwrap();
    runtime.drain_resident_host_inputs(1).unwrap().turn.unwrap()
}

fn replace_coordinator_with_replay(runtime: &mut crate::MechRuntime) {
    let ActiveProgramExecution::ResidentExternal(execution) = &mut runtime.active_program else {
        panic!("expected an external resident program")
    };
    let instance = mech_engine::__resident::activate_external(
        execution.coordinator.instance().id,
        &execution.artifact,
        &mech_stdlib::source_catalog(),
        &mech_engine::resident::ActivationFacts::default(),
        mech_engine::resident::ResidentIntegrityMode::Checked,
    )
    .unwrap();
    let mut replay = external::ResidentExternalCoordinator::new_replay(
        instance,
        Arc::clone(&execution.artifact),
        execution.coordinator.replay_bootstrap(),
        crate::ResidentDurabilityPolicy::Retained,
        external::ResidentExternalLimits::default(),
    )
    .unwrap();
    let batches = execution.coordinator.input_facts().collect::<Vec<_>>();
    let records = execution.coordinator.receipts().collect::<Vec<_>>();
    assert_eq!(batches.len(), records.len());
    for ((_, batch), (_, record)) in batches.into_iter().zip(records) {
        replay.execute_replay_batch(Some(batch), record).unwrap();
    }
    execution.coordinator = replay;
}

#[test]
fn replacement_projections_keep_published_inputs_after_rejected_turn_and_replay() {
    for decoded in [false, true] {
        for replay in [false, true] {
            let (mut previous, previous_reads) = load(SOURCE, decoded);
            assert!(matches!(
                submit(&mut previous, 2.0),
                crate::ResidentExternalTurnOutcome::Accepted { .. }
            ));
            assert_eq!(value(&previous, "state"), 2.0);
            assert_eq!(value(&previous, "answer"), 4.0);
            assert!(matches!(
                submit(&mut previous, -3.0),
                crate::ResidentExternalTurnOutcome::Rejected {
                    phase: crate::turn_record::TurnFailurePhase::Integrity,
                    ..
                }
            ));
            if replay {
                replace_coordinator_with_replay(&mut previous);
            }
            assert_eq!(value(&previous, "state"), 2.0);
            assert_eq!(value(&previous, "answer"), 4.0);

            let (mut candidate, candidate_reads) = load(&format!("{SOURCE}{EXTRA}"), decoded);
            let epoch = resident_parts(&candidate).1.published_epoch();
            candidate
                .preserve_compatible_resident_state_from(&previous, &BTreeSet::new())
                .unwrap();
            assert_eq!(
                resident_parts(&candidate).1.published_epoch(),
                epoch,
                "refresh must not publish a synthetic turn"
            );
            assert_eq!(value(&candidate, "state"), 2.0);
            assert_eq!(value(&candidate, "answer"), 4.0);
            assert_eq!(value(&candidate, "new-fast"), 2.0);
            assert_eq!(value(&candidate, "extra"), 14.0);
            assert_eq!(previous_reads.load(Ordering::SeqCst), 0);
            assert_eq!(candidate_reads.load(Ordering::SeqCst), 0);

            let migrated_hash = resident_parts(&candidate).1.published_state_hash();
            assert!(matches!(
                submit(&mut candidate, -4.0),
                crate::ResidentExternalTurnOutcome::Rejected { .. }
            ));
            let ActiveProgramExecution::ResidentExternal(execution) = &candidate.active_program
            else {
                unreachable!()
            };
            assert_eq!(
                execution
                    .coordinator
                    .receipts()
                    .last()
                    .unwrap()
                    .1
                    .body
                    .state_hash,
                migrated_hash,
                "a rejected turn must record the migrated publication"
            );
            assert_eq!(value(&candidate, "state"), 2.0);
            assert_eq!(value(&candidate, "extra"), 14.0);

            assert!(matches!(
                submit(&mut candidate, 3.0),
                crate::ResidentExternalTurnOutcome::Accepted { .. }
            ));
            assert_eq!(value(&candidate, "state"), 5.0);
            assert_eq!(value(&candidate, "answer"), 8.0);
            assert_eq!(value(&candidate, "extra"), 18.0);
            assert_eq!(value(&previous, "state"), 2.0);
            assert_eq!(candidate_reads.load(Ordering::SeqCst), 0);
        }
    }
}
