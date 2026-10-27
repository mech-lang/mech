//! Candidate-size correctness boundaries, not performance measurements.
use crate::native_artifact::program::runtime::session::capture_host_tests::*;

const LARGE_MEMORY: u64 = 64 * 1024 * 1024;

fn boundary_limits() -> NativeProgramExecutionLimits {
    let mut limits = fixture::limits();
    limits.execution.max_work = 1 << 49;
    limits.library.max_compute_work = Some(1 << 43);
    limits.library.max_comparison_work = Some(1 << 36);
    limits
}

fn boundary_input(chunk: usize) -> Vec<u8> {
    let mut input = "x".repeat(chunk - 1);
    input.push_str("\r\n");
    input.push_str(&"b".repeat(chunk - 2));
    input.push('é');
    input.push_str(&"x".repeat(chunk + 5));
    input.push_str("\r\nopen\r\nA\r\n猫\nclose\nignored\n");
    assert_eq!(&input.as_bytes()[chunk - 1..chunk + 1], b"\r\n");
    assert_eq!(
        &input.as_bytes()[2 * chunk - 1..2 * chunk + 1],
        "é".as_bytes()
    );
    assert!(input.lines().nth(1).unwrap().len() > 2 * chunk);
    assert!(input.len() > 3 * chunk);
    input.into_bytes()
}

#[test]
fn large_full_reads_utf8_crlf_and_long_lines_match_original_and_decoded_snapshots() {
    let (artifacts, catalog) = fixture::artifacts(&hosted_scan());
    for artifact in artifacts {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        for chunk in [256 * 1024, 1024 * 1024] {
            let full = boundary_input(chunk);
            let short = "open\r\nA\r\n猫\nclose\n".as_bytes();
            for (input, step) in [(full.as_slice(), usize::MAX), (short, 1)] {
                let limits = boundary_limits();
                let memory = ManagedMemoryBudget::new(LARGE_MEMORY);
                let (mut ordinary, reference) = runtime_delivery(input, false, None, usize::MAX);
                execute_with_chunk(
                    &mut ordinary,
                    &artifact,
                    &jit,
                    limits,
                    (input.len().max(chunk) as u64, chunk),
                    &reference,
                    &memory,
                )
                .unwrap();
                assert_eq!(reference.lock().unwrap().snapshots, 1);
                let expected = reference.lock().unwrap().output.clone();
                assert_eq!(expected, "A|猫");
                fixture::no_resident(&ordinary);
                ordinary.shutdown().unwrap();
                assert_eq!(memory.used_bytes(), 0);
                let (mut streaming, trace) = runtime_delivery(input, true, None, step);
                execute_with_chunk(
                    &mut streaming,
                    &artifact,
                    &jit,
                    limits,
                    (input.len().max(chunk) as u64, chunk),
                    &trace,
                    &memory,
                )
                .unwrap();
                let trace = trace.lock().unwrap();
                assert_eq!((trace.captures, trace.snapshots, trace.writes), (1, 0, 1));
                assert_eq!(trace.position, input.len());
                assert_eq!(trace.largest_read, if step == 1 { 1 } else { chunk });
                assert_eq!(trace.output, expected);
                assert_eq!(memory.used_bytes(), 0);
                drop(trace);
                fixture::no_resident(&streaming);
                streaming.shutdown().unwrap();
            }
        }
    }
}

#[test]
fn large_full_read_source_failures_outrank_a_saved_generated_body_failure() {
    let source = hosted_scan().replace("rows[i] == \"open\"", "rows[i][0u64] == \"open\"");
    let (artifacts, catalog) = fixture::artifacts(&source);
    let chunk = 1024 * 1024;
    for artifact in artifacts {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        for mode in 0..4 {
            let mut input = b"bad\n".to_vec();
            input.extend(std::iter::repeat_n(b'a', chunk + 64));
            input.push(b'\n');
            if mode == 1 {
                input[chunk + 17] = 0xff;
            }
            let cap = input.len() as u64 - u64::from(mode == 2);
            let fail_at = (mode == 3).then_some(chunk + 17);
            let (mut runtime, trace) = runtime_delivery(&input, true, fail_at, usize::MAX);
            let memory = ManagedMemoryBudget::new(LARGE_MEMORY);
            let error = execute_with_chunk(
                &mut runtime,
                &artifact,
                &jit,
                boundary_limits(),
                (cap, chunk),
                &trace,
                &memory,
            )
            .unwrap_err();
            let CanonicalNativeResourceRunError::BeforeCommit { phase, error, .. } = error else {
                panic!("failure must remain before atomic publication");
            };
            assert_eq!(
                phase,
                if mode == 0 {
                    "native execution"
                } else {
                    "capture"
                }
            );
            if mode == 0 {
                assert!(error.kind_as::<NativeProgramExecutionError>().is_some());
            } else {
                assert!(
                    error
                        .kind_as::<mech_terminal::CliResourceProviderError>()
                        .is_some()
                );
            }
            let trace = trace.lock().unwrap();
            assert_eq!((trace.captures, trace.snapshots, trace.writes), (1, 0, 0));
            assert_eq!(trace.largest_read, chunk);
            assert_eq!(trace.first_failure_position, Some(chunk));
            assert!(trace.body_failed_before_eof);
            assert!(trace.output.is_empty());
            assert_eq!(
                trace.position,
                fail_at.unwrap_or(input.len().min(cap as usize + 1))
            );
            assert_eq!(memory.used_bytes(), 0);
            drop(trace);
            fixture::no_resident(&runtime);
            runtime.shutdown().unwrap();
        }
    }
}

#[test]
fn large_scratch_one_short_selects_snapshot_before_acquisition_without_leaking_claims() {
    let (artifacts, catalog) = fixture::artifacts(&hosted_scan());
    let input = b"open\nA\nclose\n";
    for artifact in artifacts {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        for chunk in [256 * 1024, 1024 * 1024] {
            for fail_at in [None, Some(0)] {
                let (mut runtime, trace) = runtime_delivery(input, true, fail_at, usize::MAX);
                let memory = ManagedMemoryBudget::new(LARGE_MEMORY);
                let limits = boundary_limits();
                let cap = chunk as u64;
                let mut bridge = fixture::bridge_limits();
                bridge.max_input_text_bytes = cap;
                let plan = jit.descriptor().plan();
                let mut outputs = vec![None; plan.transport_output_count()];
                let result = runtime.with_canonical_native_resources_phased_text(
                    &artifact,
                    bridge,
                    |admission| {
                        let session = jit
                            .prepare_hosted(admission, limits, &memory)
                            .map_err(|error| MechError::new(error, None))?;
                        let state = session.prepared.state.as_ref().unwrap();
                        crate::native_artifact::program::runtime::session::capture_admission::preflight(
                            jit.descriptor(),
                            limits,
                            state.library.used(),
                            cap,
                            chunk,
                            memory.limit_bytes(),
                        )
                        .expect("scratch denial, not proof/work fallback, must select snapshot");
                        let available = memory.limit_bytes() - memory.used_bytes();
                        let required = chunk as u64 + 3;
                        assert!(available >= required);
                        let pressure = memory.reserve_capacity(available - (required - 1)).unwrap();
                        assert_eq!(memory.limit_bytes() - memory.used_bytes(), required - 1);
                        let before = trace.lock().unwrap();
                        assert_eq!(
                            (before.position, before.captures, before.snapshots),
                            (0, 0, 0)
                        );
                        drop(before);
                        let prepared = session
                            .with_text_capture(cap, chunk)
                            .map_err(|error| MechError::new(error, None));
                        // Force only setup denial. Snapshot leaf binding has
                        // independent metadata requirements; it need not fit
                        // within the capture scratch size. Strategy selection
                        // is already final, before the first source read.
                        assert_eq!(trace.lock().unwrap().position, 0);
                        drop(pressure);
                        prepared
                    },
                    |_, _| panic!("unfunded capture scratch must select snapshot before reading"),
                    |session, turn| {
                        assert!(turn.text_capture().is_none());
                        let report = session
                            .resume_hosted_into(turn, &[], &mut outputs)
                            .map_err(|error| MechError::new(error, None))?;
                        Ok(CanonicalNativeResourceCompletion {
                            result: report,
                            effects: outputs[plan.outputs().len()..]
                                .iter_mut()
                                .map(|value| value.take().unwrap())
                                .collect::<Vec<_>>()
                                .into_boxed_slice(),
                        })
                    },
                );
                let trace = trace.lock().unwrap();
                assert_eq!((trace.captures, trace.snapshots), (0, 1));
                if fail_at.is_some() {
                    assert!(matches!(
                        result,
                        Err(CanonicalNativeResourceRunError::BeforeCommit {
                            phase: "capture",
                            ..
                        })
                    ));
                    assert_eq!((trace.position, trace.writes), (0, 0));
                    assert!(trace.output.is_empty());
                } else {
                    result.unwrap();
                    assert_eq!((trace.position, trace.writes), (input.len(), 1));
                    assert_eq!(trace.output, "A");
                }
                drop(outputs);
                assert_eq!(memory.used_bytes(), 0);
                drop(trace);
                fixture::no_resident(&runtime);
                runtime.shutdown().unwrap();
            }
        }
    }
}
