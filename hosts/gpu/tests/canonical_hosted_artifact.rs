//! Original canonical artifacts through checked native bodies and real CLI services.
use mech_core::{ManagedMemoryBudget, MechError};
use mech_gpu::native_artifact::{
    JitProgramArtifact, NativeExecutionLimits, NativeProgramExecutionError,
    NativeProgramExecutionLimits, NativeProgramExecutionReport,
};
use mech_runtime::*;
use std::sync::Arc;

const FULL: &str = include_str!("fixtures/native/knucleotide-idiomatic.mec");
const ECHO: &str = "+> @in := cli/stdin\n+> @out := cli/stdout\n@out/text <- @in/text\n";
const TWO: &str = "+> @out := cli/stdout\n@out/text <- \"first\"\n@out/text <- \"second\"\n";
const CLOSED_FAIL: &str =
    "+> @in := cli/stdin\n+> @out := cli/stdout\nclosed := 1u64 / 0u64\n@out/text <- @in/text\n";
const CLOSED_LIVE: &str = "+> @in := cli/stdin\n+> @out := cli/stdout\nprefix := \"closed\" + \"-\"\n@out/text <- prefix + @in/text\n";
const FAIL: &str = "+> sequence\n+> number\n+> @in := cli/stdin\n+> @out := cli/stdout\nn := sequence/length(@in/text)\nbad := 1u64 / (n - n)\n@out/text <- number/format(bad, 0u64)\n";
const GGTA_INPUT: &[u8] = b">ONE\nAAAA\n>THREE\nGGTA\n>FOUR\nTTTT\n";
const GGTA_OUTPUT: &str = "G 50.000\nA 25.000\nT 25.000\n\nGG 33.333\nGT 33.333\nTA 33.333\n\n1\tGGT\n1\tGGTA\n0\tGGTATT\n0\tGGTATTTTAATT\n0\tGGTATTTTAATTTATAGT\n";
const EMPTY_OUTPUT: &str =
    "\n\n\n\n0\tGGT\n0\tGGTA\n0\tGGTATT\n0\tGGTATTTTAATT\n0\tGGTATTTTAATTTATAGT\n";

#[path = "support/canonical_hosted_fixture.rs"]
mod fixture;
use fixture::{artifacts, bridge_limits, execute, limits, no_resident, quiet, runtime};
#[path = "support/canonical_hosted_text_capture.rs"]
mod text_capture;
#[test]
fn larger_scalar_string_observation_uses_real_secured_capture_and_atomic_publication() {
    let (variants, catalog) = artifacts(ECHO);
    for artifact in variants {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        for text in [String::new(), "é🧑‍🚀".into(), "éACGT".repeat(16_384)] {
            let (mut runtime, trace) = runtime(text.as_bytes(), true);
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let report = execute(&mut runtime, &artifact, &jit, limits(), &memory).unwrap();
            assert_eq!(trace.lock().unwrap().output, text);
            assert_eq!(memory.used_bytes(), 0);
            no_resident(&runtime);
            runtime.shutdown().unwrap();

            let (mut runtime, trace) = self::runtime(text.as_bytes(), true);
            let mut policy = limits();
            policy.execution.max_work = report.work - 1;
            assert!(matches!(
                execute(&mut runtime, &artifact, &jit, policy, &memory),
                Err(CanonicalNativeResourceRunError::BeforeCommit {
                    phase: "native execution",
                    ..
                }),
            ));
            let snapshot = trace.lock().unwrap().clone();
            assert_eq!(
                (
                    snapshot.reads,
                    snapshot.acquisitions,
                    snapshot.prepares,
                    snapshot.deliveries
                ),
                (1, 1, 0, 0)
            );
            assert!(snapshot.output.is_empty());
            assert_eq!(memory.used_bytes(), 0);
            no_resident(&runtime);
            runtime.shutdown().unwrap();

            let (mut runtime, trace) = self::runtime(text.as_bytes(), true);
            execute(&mut runtime, &artifact, &jit, limits(), &memory).unwrap();
            assert_eq!(trace.lock().unwrap().output, text);
            assert_eq!(memory.used_bytes(), 0);
            runtime.shutdown().unwrap();
        }
    }
}

#[test]
fn complete_knucleotide_original_and_decoded_run_through_actual_secured_cli() {
    let (variants, catalog) = artifacts(FULL);
    for artifact in variants {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        assert_eq!(
            mech_gpu::native_artifact::emit_program_object(&artifact, &catalog)
                .unwrap()
                .metadata()["closed_tail_regions"],
            1,
            "the actual secured artifact must select callback-free Tail control"
        );
        assert_eq!(
            jit.descriptor().plan().borrowed_string_regions().len(),
            1,
            "the complete record scanner must select owner-bound native String reads"
        );
        let lines = jit.descriptor().plan().text_line_matrix_regions();
        assert_eq!(
            lines.len(),
            1,
            "the secured source must prove its private line matrix"
        );
        assert_eq!(lines[0].borrowed_regions(), [0]);
        assert_eq!(lines[0].lengths().len(), 1);
        assert_eq!(lines[0].selections().len(), 1);
        let frontiers = jit.descriptor().plan().text_line_count_frontiers();
        assert_eq!(
            frontiers.len(),
            1,
            "the actual secured source must prove its EOF-dependent count uses"
        );
        assert_eq!(frontiers[0].guards().len(), 2);
        assert!(!frontiers[0].exact().is_empty());
        let reads = jit.descriptor().plan().text_line_read_frontiers();
        assert_eq!(
            reads.len(),
            1,
            "the actual secured source must guard every borrowed line in its current Tail visit"
        );
        assert_eq!(reads[0].reads().len(), 2);
        let retention = jit.descriptor().plan().text_line_retention_frontiers();
        assert_eq!(
            retention.len(),
            1,
            "the actual secured source must retain every future selected-range start"
        );
        assert_eq!(retention[0].states().len(), 3);
        assert_eq!(
            jit.descriptor().plan().borrowed_string_regions()[0]
                .actions()
                .iter()
                .filter(|(_, action)| matches!(
                    action,
                    mech_engine::native_artifact::NativeBorrowedStringAction::ConstantRange { .. }
                ))
                .count(),
            1,
            "the actual hosted source must elide its private constant selector"
        );
        assert!(
            jit.descriptor().plan().scalar_aggregates().value_count() > 0,
            "the complete source must use native scalar FSM state, not repeated heap packing"
        );
        assert_eq!(
            jit.descriptor().plan().window_query_regions().len(),
            1,
            "the full five-query program must use the same compiled window body"
        );
        assert_eq!(
            jit.descriptor().plan().window_histogram_regions().len(),
            1,
            "both frequency widths must use the same compiled Histogram body"
        );
        assert_eq!(
            jit.descriptor().plan().string_map_regions().len(),
            1,
            "uppercase map/join must use the same generic compiled byte-map body"
        );
        assert_eq!(
            jit.descriptor()
                .plan()
                .collections()
                .iter()
                .filter(|collection| collection.string_element_map_projection().is_some())
                .count(),
            1,
            "unchanged full source must use its generic String element/map/TextJoin projection"
        );
        assert_eq!(
            jit.descriptor()
                .plan()
                .collections()
                .iter()
                .filter(|collection| collection.string_span_projection().is_some())
                .count(),
            2,
            "the complete unchanged secured source must really use both selected String span bodies"
        );
        // Both ordinary histogram call bodies must use the generic selected
        // range strategy. Testing only a separate leaf cannot qualify FULL.
        assert_eq!(
            jit.descriptor()
                .plan()
                .iterations()
                .iter()
                .filter(|iteration| matches!(iteration,
                    mech_engine::native_artifact::NativeControlLoop::Collection(generator)
                        if matches!(generator.source(),
                            mech_engine::native_artifact::NativeCollectionSource::U64Range(_))))
                .count(),
            2,
            "the complete secured source must really use both range cursors"
        );
        assert!(
            jit.descriptor()
                .plan()
                .collections()
                .iter()
                .filter(|collection| matches!(
                    collection.result(),
                    mech_engine::native_artifact::NativeCollectionResult::TextJoin(_)
                ))
                .count()
                >= 4,
            "the complete secured source must actually use the uppercase, formatting and outer join sinks"
        );
        let plan = jit.descriptor().plan();
        let query = plan
            .collections()
            .iter()
            .filter_map(|collection| {
                if let mech_engine::native_artifact::NativeCollectionResult::GuardedHistogramQuery(
                    sink,
                ) = collection.result()
                {
                    Some((collection, sink))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        assert_eq!(
            query.len(),
            1,
            "the unchanged full source must select its generic five-query demand body, including the18mer"
        );
        assert_eq!(
            plan.collections()
                .iter()
                .filter(|collection| matches!(
                    collection.result(),
                    mech_engine::native_artifact::NativeCollectionResult::Histogram(_)
                ))
                .count(),
            1,
            "the separate two-width frequency body must retain its full histogram result"
        );
        let (collection, sink) = query[0];
        let mech_engine::native_artifact::NativeControlLoop::Collection(generator) = &plan
            .iterations()[collection
            .parent()
            .expect("query demand is inside its original outer generator")
            as usize]
        else {
            panic!("query demand must keep its original five-query generator");
        };
        assert_eq!(
            generator.element(),
            Some(sink.query()),
            "the query is the same immutable outer SSA, not a literal or recomputation"
        );
        let source = generator
            .source()
            .materialized_value()
            .expect("query strings remain an ordinary Matrix");
        assert!(
            matches!(plan.values()[source.get() as usize].definition().body(),
            mech_core::SchemaBody::Matrix { element, dimensions }
                if element.as_ref() == &mech_core::SchemaBody::String
                && dimensions.as_ref() == &[mech_core::DimensionExpr::Constant(1), mech_core::DimensionExpr::Constant(5)]),
            "this actual unchanged source iterates all five query strings, not a one-query toy"
        );
        assert_eq!(jit.descriptor().plan().resource_requirements().len(), 2);
        assert_eq!(
            jit.descriptor().plan().observations()[0].request().delivery,
            mech_core::ResourceDelivery::Live
        );
        for (input, expected) in [
            (GGTA_INPUT, GGTA_OUTPUT),
            (b">THREE\n>FOUR\nA\n".as_slice(), EMPTY_OUTPUT),
            (b">ONE\nGGTA\n".as_slice(), EMPTY_OUTPUT),
        ] {
            let (mut runtime, trace) = runtime(input, true);
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let report = execute(&mut runtime, &artifact, &jit, limits(), &memory).unwrap();
            assert!(report.visits > 0 && report.work > 0);
            no_resident(&runtime);
            let snapshot = trace.lock().unwrap().clone();
            assert_eq!(snapshot.output, expected);
            assert_eq!(
                (
                    snapshot.reads,
                    snapshot.acquisitions,
                    snapshot.prepares,
                    snapshot.deliveries,
                    snapshot.flushes
                ),
                (1, 1, 1, 1, 1)
            );
            assert_eq!(memory.used_bytes(), 0);
            runtime.shutdown().unwrap();
        }
    }
}

#[test]
fn complete_knucleotide_selected_stages_100_400_keep_real_secured_host_lifecycle() {
    let (variants, catalog) = artifacts(FULL);
    for artifact in variants {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        assert_eq!(
            jit.descriptor()
                .plan()
                .collections()
                .iter()
                .filter(|c| c.string_span_projection().is_some())
                .count(),
            2
        );
        assert_eq!(jit.descriptor().plan().resource_requirements().len(), 2);
        for repeats in [25usize, 100] {
            // SAME unchanged 100/400-base gate and independent output formula.
            let input = format!(
                ">ONE\nAAAA\n>THREE timing\n{}\n>FOUR\nCC\n",
                "GGTA".repeat(repeats)
            );
            let windows = 4 * repeats - 1;
            let common = 100.0 * repeats as f64 / windows as f64;
            let other = 100.0 * (repeats - 1) as f64 / windows as f64;
            let expected = format!(
                "G 50.000\nA 25.000\nT 25.000\n\nGG {common:.3}\nGT {common:.3}\nTA {common:.3}\nAG {other:.3}\n\n{repeats}\tGGT\n{repeats}\tGGTA\n0\tGGTATT\n0\tGGTATTTTAATT\n0\tGGTATTTTAATTTATAGT\n"
            );
            let (mut runtime, trace) = runtime(input.as_bytes(), true);
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let report = execute(&mut runtime, &artifact, &jit, limits(), &memory).unwrap();
            assert!(report.visits > 0 && report.work <= limits().execution.max_work);
            let snapshot = trace.lock().unwrap().clone();
            assert_eq!(snapshot.output.as_bytes(), expected.as_bytes());
            assert_eq!(
                (
                    snapshot.reads,
                    snapshot.acquisitions,
                    snapshot.prepares,
                    snapshot.deliveries,
                    snapshot.flushes
                ),
                (1, 1, 1, 1, 1)
            );
            no_resident(&runtime);
            assert_eq!(memory.used_bytes(), 0);
            runtime.shutdown().unwrap();
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}

#[test]
fn all_grants_are_required_before_any_observation_and_raw_inputs_cannot_bypass_them() {
    let (variants, catalog) = artifacts(ECHO);
    for artifact in variants {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
        let mut fake = vec![None; jit.descriptor().plan().transport_output_count()];
        assert!(matches!(
            jit.evaluate_into(&[], &mut fake, limits(), &memory),
            Err(NativeProgramExecutionError::Base(
                mech_gpu::native_artifact::NativeExecutionError::Descriptor { .. }
            ))
        ));
        for read_only in [false, true] {
            let (mut runtime, trace) = runtime(b"payload", false);
            if read_only {
                runtime
                    .install_run_resource_grant(&RunResourceGrantConfig {
                        target: "cli/stdin".into(),
                        operations: vec!["read".into()],
                        paths: vec!["text".into()],
                    })
                    .unwrap();
            }
            assert!(matches!(
                execute(&mut runtime, &artifact, &jit, limits(), &memory),
                Err(CanonicalNativeResourceRunError::BeforeCommit {
                    phase: "admission",
                    ..
                })
            ));
            quiet(&trace);
            no_resident(&runtime);
            assert_eq!(memory.used_bytes(), 0);
            runtime.shutdown().unwrap();
        }
    }
}

#[test]
fn finite_use_grant_admission_matches_the_existing_canonical_resident_policy() {
    let (variants, catalog) = artifacts(ECHO);
    for artifact in variants {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let (mut runtime, trace) = runtime(b"payload", false);
        runtime
            .install_run_resource_grant(&RunResourceGrantConfig {
                target: "cli/stdout".into(),
                operations: vec!["write".into()],
                paths: vec!["text".into()],
            })
            .unwrap();
        let selected = CanonicalNativeResourcePlan::select(&artifact, 16).unwrap();
        let read = selected.observations()[0].request();
        let key = RuntimeResourceKey::new(&read.base_uri, &read.path).unwrap();
        let subject = runtime.runtime_context().unwrap().subject().to_owned();
        let capability = BasicCapability::from_keys(
            runtime.next_capability_id(),
            subject,
            key.capability_resource(),
            ["read"],
        )
        .with_constraints(BasicConstraints {
            max_uses: Some(1),
            ..BasicConstraints::default()
        });
        runtime.grant_capability(Arc::new(capability)).unwrap();
        let native = execute(
            &mut runtime,
            &artifact,
            &jit,
            limits(),
            &ManagedMemoryBudget::new(64 * 1024 * 1024),
        )
        .unwrap_err();
        let CanonicalNativeResourceRunError::BeforeCommit { error: native, .. } = native else {
            panic!("finite-use capability must be rejected during admission")
        };
        let resident = runtime
            .load_compiled_program(artifact.clone(), ResidentDurabilityPolicy::Volatile)
            .unwrap_err();
        assert_eq!(native.kind_name(), resident.kind_name());
        assert!(native.display_message().contains("finite-use"));
        assert!(resident.display_message().contains("finite-use"));
        quiet(&trace);
        no_resident(&runtime);
        runtime.shutdown().unwrap();
    }
}

#[test]
fn failed_native_body_prepares_no_effect_and_recovers_without_retaining_native_memory() {
    let (variants, catalog) = artifacts(ECHO);
    for artifact in variants {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let (mut runtime, trace) = runtime(b"payload", true);
        let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
        let mut denied = limits();
        denied.execution.max_work = 1;
        let error = execute(&mut runtime, &artifact, &jit, denied, &memory).unwrap_err();
        let CanonicalNativeResourceRunError::BeforeCommit { error, cleanup, .. } = error else {
            panic!("work denial must not commit")
        };
        assert!(error.kind_as::<NativeProgramExecutionError>().is_some());
        assert!(cleanup.is_none());
        let snapshot = trace.lock().unwrap().clone();
        assert_eq!(
            (
                snapshot.reads,
                snapshot.acquisitions,
                snapshot.prepares,
                snapshot.deliveries
            ),
            (0, 0, 0, 0)
        );
        assert_eq!(memory.used_bytes(), 0);
        execute(&mut runtime, &artifact, &jit, limits(), &memory).unwrap();
        assert_eq!(trace.lock().unwrap().output, "payload");
        assert_eq!(
            trace.lock().unwrap().acquisitions,
            1,
            "same real provider snapshot"
        );
        assert_eq!(memory.used_bytes(), 0);
        no_resident(&runtime);
        runtime.shutdown().unwrap();
    }
    let (variants, catalog) = artifacts(FAIL);
    for artifact in variants {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let (mut runtime, trace) = runtime(b"payload", true);
        let error = execute(
            &mut runtime,
            &artifact,
            &jit,
            limits(),
            &ManagedMemoryBudget::new(64 * 1024 * 1024),
        )
        .unwrap_err();
        let CanonicalNativeResourceRunError::BeforeCommit { error, .. } = error else {
            panic!("arithmetic failure must not commit")
        };
        assert!(matches!(
            error.kind_as::<NativeProgramExecutionError>(),
            Some(NativeProgramExecutionError::Control(
                mech_gpu::native_artifact::NativeIterationExecutionError::Arithmetic { .. }
            ))
        ));
        let snapshot = trace.lock().unwrap().clone();
        assert_eq!(
            (snapshot.reads, snapshot.prepares, snapshot.deliveries),
            (1, 0, 0)
        );
        no_resident(&runtime);
        runtime.shutdown().unwrap();
    }
}

#[test]
fn token_for_another_original_artifact_cannot_authorize_the_native_entry() {
    let (variants, catalog) = artifacts(ECHO);
    let other = artifacts(&format!("{ECHO}different := 7u64\n")).0.remove(0);
    let wrong = JitProgramArtifact::compile(&other, &catalog).unwrap();
    for artifact in variants {
        let (mut runtime, trace) = runtime(b"payload", true);
        let error = execute(
            &mut runtime,
            &artifact,
            &wrong,
            limits(),
            &ManagedMemoryBudget::new(64 * 1024 * 1024),
        )
        .unwrap_err();
        assert!(matches!(
            error,
            CanonicalNativeResourceRunError::BeforeCommit { .. }
        ));
        let snapshot = trace.lock().unwrap().clone();
        assert_eq!(
            (snapshot.reads, snapshot.prepares, snapshot.deliveries),
            (0, 0, 0)
        );
        no_resident(&runtime);
        runtime.shutdown().unwrap();
    }
}

#[test]
fn late_bad_effect_payload_is_rejected_before_the_first_real_provider_prepare() {
    for artifact in artifacts(TWO).0 {
        let (mut runtime, trace) = runtime(b"", true);
        let error = runtime
            // Privileged malformed-payload fixture, NOT native execution.
            .with_canonical_native_resources_phased(
                &artifact,
                bridge_limits(),
                |_| Ok(()),
                |(), turn| {
                    assert_eq!(turn.plan().effects().len(), 2);
                    let site = &turn.plan().effects()[0];
                    let value = RuntimeHostInputValue::String("first".into())
                        .into_value()
                        .unwrap();
                    let first = value
                        .rebind(site.schema(), value.shape(), artifact.schemas())
                        .unwrap();
                    let bad = RuntimeHostInputValue::U64(7).into_value().unwrap();
                    Ok(CanonicalNativeResourceCompletion {
                        result: (),
                        effects: vec![first, bad].into(),
                    })
                },
            )
            .unwrap_err();
        assert!(matches!(
            error,
            CanonicalNativeResourceRunError::BeforeCommit {
                phase: "effect materialization",
                ..
            }
        ));
        quiet(&trace);
        no_resident(&runtime);
        runtime.shutdown().unwrap();
    }
}

#[test]
fn postcommit_flush_failure_is_not_reported_as_an_atomic_denial() {
    let (variants, catalog) = artifacts(ECHO);
    for artifact in variants {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let (mut runtime, trace) = runtime(b"payload", true);
        trace.lock().unwrap().fail_flush = true;
        let error = execute(
            &mut runtime,
            &artifact,
            &jit,
            limits(),
            &ManagedMemoryBudget::new(64 * 1024 * 1024),
        )
        .unwrap_err();
        let CanonicalNativeResourceRunError::CommittedWithFailures(outcome) = error else {
            panic!("delivery has been attempted after commit; not a retry-safe precommit failure")
        };
        assert_eq!(outcome.delivery_failures.len(), 1);
        let snapshot = trace.lock().unwrap().clone();
        assert_eq!(
            (snapshot.prepares, snapshot.deliveries, snapshot.flushes),
            (1, 1, 1)
        );
        assert_eq!(snapshot.output, "payload");
        no_resident(&runtime);
        runtime.shutdown().unwrap();
    }
}

#[test]
fn missing_authority_prevents_even_the_pre_capture_native_activation_callback() {
    for artifact in artifacts(CLOSED_FAIL).0 {
        let (mut runtime, trace) = runtime(b"payload", false);
        let called = std::cell::Cell::new(false);
        let error = runtime
            .with_canonical_native_resources_phased::<(), ()>(
                &artifact,
                bridge_limits(),
                |_| {
                    called.set(true);
                    Ok(())
                },
                |(), _| panic!("missing grants must not reach captured/native turn"),
            )
            .unwrap_err();
        assert!(matches!(
            error,
            CanonicalNativeResourceRunError::BeforeCommit {
                phase: "admission",
                ..
            }
        ));
        assert!(!called.get());
        quiet(&trace);
        no_resident(&runtime);
        runtime.shutdown().unwrap();
    }
}

#[test]
fn closed_native_arithmetic_failure_precedes_both_successful_and_failing_provider_reads() {
    let (variants, catalog) = artifacts(CLOSED_FAIL);
    for artifact in variants {
        assert!(mech_engine::resident::canonical_resource_activation_required(&artifact).unwrap());
        let divide = artifact
            .nodes()
            .iter()
            .find(|node| {
                node.as_operation()
                    .is_some_and(|operation| operation.operation.canonical_name() == "math/div")
            })
            .unwrap()
            .node;
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        for fail_read in [false, true] {
            // Establish actual canonical resident precedence, not an invented
            // native-only order. All grant/provider metadata checks succeed.
            let (mut resident, resident_trace) = runtime(b"payload", true);
            resident_trace.lock().unwrap().fail_read = fail_read;
            let reference = resident
                .load_compiled_program(artifact.clone(), ResidentDurabilityPolicy::Volatile)
                .unwrap_err();
            assert!(
                reference.display_message().contains("ActivationKernel"),
                "{reference:?}"
            );
            quiet(&resident_trace);
            no_resident(&resident);
            resident.shutdown().unwrap();

            let (mut runtime, trace) = runtime(b"payload", true);
            trace.lock().unwrap().fail_read = fail_read;
            let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
            let failure = execute(&mut runtime, &artifact, &jit, limits(), &memory).unwrap_err();
            let CanonicalNativeResourceRunError::BeforeCommit {
                phase: "native activation",
                error,
                cleanup,
            } = failure
            else {
                panic!("closed failure was masked by capture: {failure:?}")
            };
            assert!(cleanup.is_none());
            assert!(
                matches!(
                    error.kind_as::<NativeProgramExecutionError>(),
                    Some(NativeProgramExecutionError::Control(
                        mech_gpu::native_artifact::NativeIterationExecutionError::Arithmetic { origin }
                    )) if origin.node() == divide
                ),
                "{error:?}"
            );
            quiet(&trace);
            assert_eq!(memory.used_bytes(), 0);
            no_resident(&runtime);
            runtime.shutdown().unwrap();
        }
    }
}

#[test]
fn native_memory_admission_precedes_even_a_failing_provider_acquisition() {
    let (variants, catalog) = artifacts(ECHO);
    for artifact in variants {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let (mut runtime, trace) = runtime(b"payload", true);
        trace.lock().unwrap().fail_read = true;
        let memory = ManagedMemoryBudget::new(0);
        let failure = execute(&mut runtime, &artifact, &jit, limits(), &memory).unwrap_err();
        let CanonicalNativeResourceRunError::BeforeCommit {
            phase: "native activation",
            error,
            cleanup,
        } = failure
        else {
            panic!("provider failure masked memory admission: {failure:?}")
        };
        assert!(error.kind_as::<NativeProgramExecutionError>().is_some());
        assert!(cleanup.is_none());
        quiet(&trace);
        assert_eq!(memory.used_bytes(), 0);
        no_resident(&runtime);
        runtime.shutdown().unwrap();
    }
}

#[test]
fn provider_capture_failure_drops_the_already_prepared_native_session_without_effects() {
    let (variants, catalog) = artifacts(CLOSED_LIVE);
    for artifact in variants {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        assert!(mech_engine::resident::canonical_resource_activation_required(&artifact).unwrap());
        let (mut runtime, trace) = runtime(b"payload", true);
        trace.lock().unwrap().fail_read = true;
        let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
        let failure = execute(&mut runtime, &artifact, &jit, limits(), &memory).unwrap_err();
        let CanonicalNativeResourceRunError::BeforeCommit {
            phase: "capture",
            error,
            cleanup,
        } = failure
        else {
            panic!("{failure:?}")
        };
        assert!(
            error
                .display_message()
                .contains("controlled stdin acquisition failure")
        );
        assert!(cleanup.is_none());
        let snapshot = trace.lock().unwrap().clone();
        assert_eq!(
            (
                snapshot.reads,
                snapshot.acquisitions,
                snapshot.prepares,
                snapshot.deliveries,
                snapshot.flushes
            ),
            (1, 1, 0, 0, 0)
        );
        assert!(snapshot.output.is_empty());
        assert_eq!(
            memory.used_bytes(),
            0,
            "failed capture must drop activation storage/ledger"
        );
        no_resident(&runtime);
        runtime.shutdown().unwrap();
    }
}

#[test]
fn activation_node_work_denial_observes_nothing_and_releases_all_native_storage() {
    use mech_gpu::native_artifact::{
        NativeExecutionError as E, NativeExecutionPhase as Phase,
        NativeIterationExecutionError as I, NativeValueError as V,
    };
    let (variants, catalog) = artifacts(CLOSED_LIVE);
    for artifact in variants {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let (mut runtime, trace) = runtime(b"payload", true);
        let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
        // Move only to the next typed cumulative admission frontier, rather
        // than hard-coding setup/library costs that legitimately evolve.
        let mut cap = 1;
        let mut found_activation_visit = false;
        for _ in 0..128 {
            let mut policy = limits();
            policy.execution.max_work = cap;
            let failure = execute(&mut runtime, &artifact, &jit, policy, &memory).unwrap_err();
            let CanonicalNativeResourceRunError::BeforeCommit {
                phase: "native activation",
                error,
                ..
            } = failure
            else {
                panic!("denial escaped activation/captured a provider: {failure:?}")
            };
            let native = error.kind_as::<NativeProgramExecutionError>().unwrap();
            let (node, required) = match native {
                NativeProgramExecutionError::Base(E::WorkLimit {
                    phase, required, ..
                })
                | NativeProgramExecutionError::Value(V::Base(E::WorkLimit {
                    phase,
                    required,
                    ..
                }))
                | NativeProgramExecutionError::Control(I::Base(E::WorkLimit {
                    phase,
                    required,
                    ..
                })) => (
                    match phase {
                        Phase::Node(node) => Some(*node),
                        _ => None,
                    },
                    *required,
                ),
                NativeProgramExecutionError::Control(I::Work {
                    origin, required, ..
                }) => (origin.as_ref().map(|origin| origin.node()), *required),
                NativeProgramExecutionError::Library {
                    origin,
                    error:
                        mech_engine::native_artifact::NativeCollectionInvocationError::Kernel(
                            mech_core::ResidentKernelError::ResourceLimit {
                                dimension: mech_core::MemoryBudgetDimension::ComputeWork,
                                required,
                                ..
                            },
                        ),
                } => (Some(origin.node()), *required),
                _ => panic!("expected typed cumulative work denial: {native:?}"),
            };
            quiet(&trace);
            assert_eq!(memory.used_bytes(), 0);
            assert!(required > cap);
            if node.is_some() {
                found_activation_visit = true;
                break;
            }
            cap = required;
        }
        assert!(
            found_activation_visit,
            "fixture never reached an activation-node denial"
        );
        no_resident(&runtime);
        runtime.shutdown().unwrap();
    }
}

#[test]
fn phased_linear_native_activation_is_not_replayed_and_shares_the_final_work_limit() {
    let (variants, catalog) = artifacts(CLOSED_LIVE);
    for artifact in variants {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let expected_visits = jit.descriptor().plan().blocks().len() as u64;
        let (mut baseline, trace) = runtime(b"payload", true);
        let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
        let report = execute(&mut baseline, &artifact, &jit, limits(), &memory).unwrap();
        assert_eq!(trace.lock().unwrap().output, "closed-payload");
        // This fixture has no loops, branches or dormant rejects. The phase
        // checkpoint returns BEFORE the turn boundary's visit; resume visits
        // it once, without replaying any closed block or setup.
        assert_eq!(report.visits, expected_visits);
        assert_eq!(memory.used_bytes(), 0);
        no_resident(&baseline);
        baseline.shutdown().unwrap();

        for accepted in [true, false] {
            let (mut runtime, trace) = runtime(b"payload", true);
            let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
            let mut policy = limits();
            policy.execution.max_work = report.work - u64::from(!accepted);
            let result = execute(&mut runtime, &artifact, &jit, policy, &memory);
            if accepted {
                let exact = result.unwrap();
                assert_eq!(exact.work, report.work);
                assert_eq!(exact.visits, expected_visits);
                assert_eq!(trace.lock().unwrap().output, "closed-payload");
            } else {
                let failure = result.unwrap_err();
                let CanonicalNativeResourceRunError::BeforeCommit {
                    phase: "native execution",
                    error,
                    ..
                } = failure
                else {
                    panic!("expected cumulative completion denial: {failure:?}")
                };
                assert!(error.kind_as::<NativeProgramExecutionError>().is_some());
                let snapshot = trace.lock().unwrap().clone();
                assert_eq!(
                    (snapshot.reads, snapshot.prepares, snapshot.deliveries),
                    (1, 0, 0)
                );
                assert!(snapshot.output.is_empty());
            }
            assert_eq!(memory.used_bytes(), 0);
            no_resident(&runtime);
            runtime.shutdown().unwrap();
        }
    }
}

#[test]
fn same_artifact_session_cannot_resume_with_a_different_admitted_transaction_token() {
    let source = "+> @in := cli/stdin\ntext := @in/text\n";
    let (variants, catalog) = artifacts(source);
    for artifact in variants {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
        let (mut first, _) = runtime(b"first", true);
        // Privileged test host intentionally retains an unresumed session.
        // This is NOT successful native evaluation or an application path.
        let prepared = first
            .with_canonical_native_resources_phased(
                &artifact,
                bridge_limits(),
                |admission| {
                    jit.prepare_hosted(admission, limits(), &memory)
                        .map_err(|error| MechError::new(error, None))
                },
                |session, _| {
                    Ok(CanonicalNativeResourceCompletion {
                        result: session,
                        effects: Box::new([]),
                    })
                },
            )
            .unwrap();
        first.shutdown().unwrap();
        assert!(memory.used_bytes() > 0);
        let (mut second, trace) = runtime(b"second", true);
        let mut outputs = vec![None; jit.descriptor().plan().transport_output_count()];
        let failure = second
            .with_canonical_native_resources_phased(
                &artifact,
                bridge_limits(),
                |_| Ok(prepared),
                |session, turn| {
                    let result = session
                        .resume_hosted_into(turn, &[], &mut outputs)
                        .map_err(|error| MechError::new(error, None))?;
                    Ok(CanonicalNativeResourceCompletion {
                        result,
                        effects: Box::new([]),
                    })
                },
            )
            .unwrap_err();
        let CanonicalNativeResourceRunError::BeforeCommit {
            phase: "native execution",
            error,
            ..
        } = failure
        else {
            panic!("{failure:?}")
        };
        assert!(matches!(
            error.kind_as::<NativeProgramExecutionError>(),
            Some(NativeProgramExecutionError::Base(
                mech_gpu::native_artifact::NativeExecutionError::Descriptor { .. }
            ))
        ));
        assert!(outputs.iter().all(Option::is_none));
        assert_eq!(memory.used_bytes(), 0);
        let snapshot = trace.lock().unwrap().clone();
        assert_eq!(
            (snapshot.reads, snapshot.prepares, snapshot.deliveries),
            (1, 0, 0)
        );
        no_resident(&second);
        second.shutdown().unwrap();
    }
}

#[path = "support/canonical_hosted_linked.rs"]
mod linked;

#[path = "support/canonical_hosted_scaling.rs"]
mod scaling;

#[path = "support/canonical_hosted_zero_resources.rs"]
mod zero_resources;

#[path = "support/canonical_hosted_factory_linkage.rs"]
mod factory_linkage;
