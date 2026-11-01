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

#[test]
fn interactive_symbol_queries_keep_new_observations_absent_until_publication() {
    struct ObservationFactory {
        decoded: bool,
        reads: std::rc::Rc<std::cell::RefCell<Vec<Arc<AtomicUsize>>>>,
    }

    impl crate::ResidentReplRuntimeFactory for ObservationFactory {
        fn build(&self, _events: crate::MechEventBuffer) -> mech_core::MResult<crate::MechRuntime> {
            let (mut runtime, reads) = independent_external_runtime_with_source("true\n");
            runtime.unload_active_program()?;
            self.reads.borrow_mut().push(reads);
            Ok(runtime)
        }

        fn activate(
            &self,
            events: crate::MechEventBuffer,
            source: &str,
        ) -> mech_core::MResult<(crate::MechRuntime, crate::RuntimeProgramLoadOutcome)> {
            let mut runtime = self.build(events)?;
            let outcome = runtime.load_interactive_source_program(
                source,
                crate::ResidentDurabilityPolicy::Retained,
            )?;
            if !self.decoded {
                return Ok((runtime, outcome));
            }
            let bytes = encode_program_artifact_bytecode_v1(resident_parts(&runtime).0).unwrap();
            runtime.unload_active_program()?;
            let outcome =
                runtime.load_bytecode_program(&bytes, crate::ResidentDurabilityPolicy::Retained)?;
            Ok((runtime, outcome))
        }
    }

    for decoded in [false, true] {
        let reads = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let mut session = crate::ResidentReplSession::from_source(
            ObservationFactory {
                decoded,
                reads: std::rc::Rc::clone(&reads),
            },
            "@fast := test://clock/fast{:read(delta-seconds)}\nanswer := 7.0\nanswer\n".to_owned(),
        )
        .unwrap();
        session
            .submit("clock-second := @fast/delta-seconds;")
            .unwrap();
        let runtime = session.runtime().unwrap();
        let output = runtime.root_symbol_output_id("clock-second").unwrap();
        assert!(runtime.output_value(output).unwrap().is_none());
        let epoch = resident_parts(runtime).1.published_epoch();
        for _ in 0..3 {
            assert!(session.symbol("clock-second").unwrap().is_none());
            assert!(session.symbol("missing").unwrap().is_none());
            assert_eq!(
                canonical_f64(session.symbol("answer").unwrap().unwrap().value()),
                7.0
            );
            for requested in [vec![], vec!["answer".to_owned(), "clock-second".to_owned()]] {
                let rows = session.symbols(&requested).unwrap();
                assert!(
                    rows.iter().any(
                        |(name, value)| name == "answer" && canonical_f64(value.value()) == 7.0
                    )
                );
                assert!(!rows.iter().any(|(name, _)| name == "clock-second"));
                let rows = session.symbol_inspections(&requested).unwrap();
                assert!(rows.iter().any(|row| row.name == "answer"));
                assert!(!rows.iter().any(|row| row.name == "clock-second"));
            }
            assert!(session.symbols(&["missing".to_owned()]).is_err());
            crate::dispatch_repl_request(
                &mut session,
                crate::parse_repl_request(":whos").unwrap(),
                &crate::ReplHostAvailability::all_available(),
                crate::ReplStepMode::Cooperative,
            )
            .unwrap();
            crate::emit_step_complete(&mut session, 0).unwrap();
            let tables = session
                .drain_events()
                .unwrap()
                .into_iter()
                .filter_map(|envelope| match envelope.event {
                    crate::MechEvent::Repl(crate::ReplEvent::Response(crate::ReplResponse {
                        kind: crate::ReplResponseKind::SymbolInspection,
                        content: crate::OutputContent::Table(table),
                        ..
                    })) => Some(table),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(tables.len(), 2);
            for table in tables {
                assert!(table.rows.iter().any(|row| row[0] == "answer"));
                assert!(!table.rows.iter().any(|row| row[0] == "clock-second"));
            }
        }
        assert_eq!(
            resident_parts(session.runtime().unwrap())
                .1
                .published_epoch(),
            epoch
        );
        session
            .clear_variables(&["clock-second".to_owned()])
            .unwrap();
        // Removing the last observation activates a new pure program. Inspect
        // that program's epoch; comparing it to the retired external program
        // would confuse source activation with an inspection-induced turn.
        let cleared_epoch = resident_parts(session.runtime().unwrap())
            .1
            .published_epoch();
        assert!(session.symbol("clock-second").unwrap().is_none());
        let rows = session.symbols(&[]).unwrap();
        assert!(rows.iter().any(|(name, _)| name == "answer"));
        assert!(!rows.iter().any(|(name, _)| name == "clock-second"));
        assert_eq!(
            resident_parts(session.runtime().unwrap())
                .1
                .published_epoch(),
            cleared_epoch
        );
        session
            .submit("clock-second := @fast/delta-seconds;")
            .unwrap();
        assert!(session.symbol("clock-second").unwrap().is_none());
        assert!(matches!(
            submit(session.runtime_mut().unwrap(), 8.0),
            crate::ResidentExternalTurnOutcome::Accepted { .. }
        ));
        assert_eq!(
            canonical_f64(session.symbol("clock-second").unwrap().unwrap().value()),
            8.0
        );
        assert!(
            session
                .symbols(&[])
                .unwrap()
                .iter()
                .any(|(name, value)| name == "clock-second" && canonical_f64(value.value()) == 8.0)
        );
        assert!(
            session
                .symbol_inspections(&[])
                .unwrap()
                .iter()
                .any(|row| row.name == "clock-second" && canonical_f64(row.value.value()) == 8.0)
        );
        session
            .clear_variables(&["clock-second".to_owned()])
            .unwrap();
        assert!(session.symbol("clock-second").unwrap().is_none());
        assert!(
            session
                .symbols(&[])
                .unwrap()
                .iter()
                .any(|(name, _)| name == "answer")
        );
        assert!(
            !session
                .symbols(&[])
                .unwrap()
                .iter()
                .any(|(name, _)| name == "clock-second")
        );
        assert!(
            reads
                .borrow()
                .iter()
                .all(|reads| reads.load(Ordering::SeqCst) == 0)
        );
    }
}
