//! Actual authorized CLI delivery into the generated scanner and atomic send.
use super::*;
use crate::native_artifact::JitProgramArtifact;
use mech_core::{MResult, MechError};
use mech_runtime::*;
use mech_terminal::{CliBackend, CliHostFactory};
use std::{
    io::{self, Read},
    sync::{Arc, Mutex},
};

#[path = "../../tests/support/canonical_hosted_fixture.rs"]
mod fixture;
#[path = "program_join_map_capture_tests.rs"]
mod joined_map_tests;

const FULL: &str = include_str!("../../tests/fixtures/native/knucleotide-idiomatic.mec");
const INPUT: &[u8] = b">ONE\nAAAA\n>THREE\nGGTA\n>FOUR\nTTTT\n";
const ECHO: &str = "+> @in := cli/stdin\n+> @out := cli/stdout\n@out/text <- @in/text\n";
const TEST_MEMORY_LIMIT: u64 = 32 * 1024 * 1024;

#[derive(Debug)]
struct Trace {
    input: Vec<u8>,
    position: usize,
    read_step: usize,
    largest_read: usize,
    captures: usize,
    snapshots: usize,
    writes: usize,
    output: String,
    body_failed_before_eof: bool,
    first_failure_position: Option<usize>,
    fail_at: Option<usize>,
    offered: bool,
}
#[derive(Clone, Debug)]
struct Backend(Arc<Mutex<Trace>>);
struct Reader(Arc<Mutex<Trace>>);
impl Read for Reader {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        let mut trace = self.0.lock().unwrap();
        if trace.fail_at == Some(trace.position) {
            return Err(io::Error::other("controlled late capture I/O failure"));
        }
        let count = output
            .len()
            .min(trace.read_step)
            .min(trace.input.len() - trace.position)
            .min(trace.fail_at.map_or(usize::MAX, |at| at - trace.position));
        output[..count].copy_from_slice(&trace.input[trace.position..trace.position + count]);
        trace.position += count;
        trace.largest_read = trace.largest_read.max(count);
        Ok(count)
    }
}
impl CliBackend for Backend {
    fn env_var(&self, _: &str) -> MResult<Option<String>> {
        Ok(None)
    }
    fn stdin_bytes(&self) -> Option<MResult<Vec<u8>>> {
        self.0.lock().unwrap().snapshots += 1;
        let mut bytes = Vec::new();
        Some(
            Reader(self.0.clone())
                .read_to_end(&mut bytes)
                .map(|_| bytes)
                .map_err(|error| {
                    MechError::new(
                        mech_terminal::CliResourceProviderError {
                            resource: "cli://cli/stdin".into(),
                            reason: error.to_string(),
                        },
                        None,
                    )
                }),
        )
    }
    fn supports_single_use_stdin_text_capture(&self) -> bool {
        self.0.lock().unwrap().offered
    }
    fn with_single_use_stdin_text_reader(
        &self,
        capture: &mut dyn FnMut(&mut dyn Read) -> MResult<()>,
    ) -> MResult<()> {
        self.0.lock().unwrap().captures += 1;
        capture(&mut Reader(self.0.clone()))
    }
    fn write_stdout(&mut self, text: &str) -> MResult<()> {
        let mut trace = self.0.lock().unwrap();
        trace.writes += 1;
        trace.output.push_str(text);
        Ok(())
    }
    fn write_stderr(&mut self, _: &str) -> MResult<()> {
        panic!("unexpected stderr effect")
    }
}

fn runtime(
    input: &[u8],
    offered: bool,
    fail_at: Option<usize>,
) -> (MechRuntime, Arc<Mutex<Trace>>) {
    runtime_delivery(input, offered, fail_at, 1)
}

fn runtime_delivery(
    input: &[u8],
    offered: bool,
    fail_at: Option<usize>,
    read_step: usize,
) -> (MechRuntime, Arc<Mutex<Trace>>) {
    assert!(read_step > 0);
    let trace = Arc::new(Mutex::new(Trace {
        input: input.into(),
        position: 0,
        read_step,
        largest_read: 0,
        captures: 0,
        snapshots: 0,
        writes: 0,
        output: String::new(),
        body_failed_before_eof: false,
        first_failure_position: None,
        fail_at,
        offered,
    }));
    let mut builder = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .host_factory(Box::new(
            CliHostFactory::with_backend(Backend(trace.clone())).unwrap(),
        ))
        .unwrap()
        .host_instance(HostInstanceConfig {
            name: "cli".into(),
            provider: "cli".into(),
            settings: ConfigValue::Map(Default::default()),
        });
    for (target, operation) in [("cli/stdin", "read"), ("cli/stdout", "write")] {
        builder = builder.run_resource_grant(RunResourceGrantConfig {
            target: target.into(),
            operations: vec![operation.into()],
            paths: vec!["text".into()],
        });
    }
    (builder.build().unwrap(), trace)
}

fn execute(
    runtime: &mut MechRuntime,
    artifact: &mech_engine::ProgramArtifact,
    jit: &JitProgramArtifact,
    limits: NativeProgramExecutionLimits,
    cap: u64,
    trace: &Arc<Mutex<Trace>>,
    memory: &ManagedMemoryBudget,
) -> Result<NativeProgramExecutionReport, CanonicalNativeResourceRunError> {
    execute_with_chunk(runtime, artifact, jit, limits, (cap, 7), trace, memory)
}

fn execute_with_chunk(
    runtime: &mut MechRuntime,
    artifact: &mech_engine::ProgramArtifact,
    jit: &JitProgramArtifact,
    limits: NativeProgramExecutionLimits,
    (cap, chunk_bytes): (u64, usize),
    trace: &Arc<Mutex<Trace>>,
    memory: &ManagedMemoryBudget,
) -> Result<NativeProgramExecutionReport, CanonicalNativeResourceRunError> {
    let mut bridge = fixture::bridge_limits();
    bridge.max_input_text_bytes = cap;
    let plan = jit.descriptor().plan();
    let mut outputs = vec![None; plan.transport_output_count()];
    runtime.with_canonical_native_resources_phased_text(
        artifact,
        bridge,
        |admission| {
            let trace = trace.lock().unwrap();
            assert_eq!(
                (trace.captures, trace.snapshots, trace.position),
                (0, 0, 0),
                "storage strategy must be selected before source acquisition"
            );
            drop(trace);
            jit.prepare_hosted(admission, limits, memory)
                .and_then(|session| session.with_text_capture(cap, chunk_bytes))
                .map_err(|error| MechError::new(error, None))
        },
        |session, chunk| {
            assert_eq!(
                trace.lock().unwrap().writes,
                0,
                "no partial output during capture"
            );
            let result = session.capture_chunk(chunk);
            if session.test_capture_has_pending_error() {
                let mut trace = trace.lock().unwrap();
                let position = trace.position;
                trace.first_failure_position.get_or_insert(position);
                if trace.position < trace.input.len() {
                    trace.body_failed_before_eof = true;
                }
            }
            result.map_err(|error| MechError::new(error, None))
        },
        |session, turn| {
            let report = session
                .resume_hosted_into(turn, &[], &mut outputs)
                .map_err(|error| MechError::new(error, None))?;
            Ok(CanonicalNativeResourceCompletion {
                result: report,
                effects: outputs[plan.outputs().len()..]
                    .iter_mut()
                    .map(|value| value.take().expect("checked atomic completion"))
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            })
        },
    )
}

fn hosted_scan() -> String {
    format!(
        "+> @in := cli/stdin\n+> @out := cli/stdout\n{}",
        prefix_tests::source()
            .replace("input<string>", "@in/text")
            .strip_suffix("result\n")
            .unwrap()
            .to_owned()
            + "@out/text <- result\n"
    )
}

#[test]
fn original_and_decoded_full_program_stream_through_authorized_cli_and_match_snapshot_output() {
    let (artifacts, catalog) = fixture::artifacts(FULL);
    for artifact in artifacts {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let mut standard = fixture::limits();
        standard.execution.max_work = 1 << 49;
        standard.library.max_compute_work = Some(1 << 43);
        standard.library.max_comparison_work = Some(1 << 36);
        super::capture_admission::preflight(
            jit.descriptor(),
            standard,
            (0, 0),
            254_166_745,
            64 * 1024,
            3 * 1024 * 1024 * 1024,
        )
        .expect("standard full program must qualify for finite streaming work");
        assert_eq!(
            jit.descriptor()
                .metadata()
                .prefix
                .as_ref()
                .unwrap()
                .frontier
                .scans_per_ordinal(),
            Some(2)
        );
        let descriptor = jit.descriptor();
        let region = descriptor.metadata().prefix.as_ref().unwrap();
        let closed =
            descriptor.metadata().closed_tails.sites[region.tail.header() as usize].unwrap();
        let comparison = descriptor.metadata().closed_tails.regions[closed].comparison;
        assert!(comparison > 0);
        let work_memory = ManagedMemoryBudget::new(4096);
        let work = NativeLibraryWorkBudget::new(1 << 40, comparison * 4, &work_memory).unwrap();
        assert!(super::capture_admission::window_comparison_fits(
            descriptor, &work, 1
        ));
        work.charge(0, 1).unwrap();
        assert!(!super::capture_admission::window_comparison_fits(
            descriptor, &work, 1
        ));
        work.charge(1, 0).unwrap(); // source work remains admissible after suspension
        drop(work);
        assert_eq!(work_memory.used_bytes(), 0);
        for input in [INPUT, b"".as_slice(), b">THREE\r\nGGTA".as_slice()] {
            let memory = ManagedMemoryBudget::new(TEST_MEMORY_LIMIT);
            let (mut ordinary, reference) = fixture::runtime(input, true);
            fixture::execute(&mut ordinary, &artifact, &jit, fixture::limits(), &memory).unwrap();
            let expected = reference.lock().unwrap().output.clone();
            fixture::no_resident(&ordinary);
            ordinary.shutdown().unwrap();
            let (mut streaming, trace) = runtime(input, true, None);
            execute(
                &mut streaming,
                &artifact,
                &jit,
                fixture::limits(),
                input.len() as u64,
                &trace,
                &memory,
            )
            .unwrap();
            let trace = trace.lock().unwrap();
            assert_eq!((trace.captures, trace.snapshots, trace.writes), (1, 0, 1));
            assert_eq!(trace.position, input.len());
            assert_eq!(trace.output, expected);
            assert_eq!(memory.used_bytes(), 0);
            drop(trace);
            fixture::no_resident(&streaming);
            streaming.shutdown().unwrap();
        }
    }
}

#[path = "program_capture_chunk_boundary_tests.rs"]
mod chunk_boundaries;

#[test]
fn capture_failures_precede_a_real_generated_body_failure_and_publish_nothing() {
    let source = hosted_scan().replace("rows[i] == \"open\"", "rows[i][0u64] == \"open\"");
    let (artifacts, catalog) = fixture::artifacts(&source);
    for artifact in artifacts {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        for (input, cap, fail_at, expected) in [
            (b"bad\nlater\n".as_slice(), 10, None, "native execution"),
            (b"bad\nlater\xff".as_slice(), 10, None, "capture"),
            (b"bad\nlater\n".as_slice(), 9, None, "capture"),
            (b"bad\nlater\n".as_slice(), 10, Some(8), "capture"),
        ] {
            let (mut runtime, trace) = runtime(input, true, fail_at);
            let memory = ManagedMemoryBudget::new(TEST_MEMORY_LIMIT);
            let result = execute(
                &mut runtime,
                &artifact,
                &jit,
                fixture::limits(),
                cap,
                &trace,
                &memory,
            );
            let CanonicalNativeResourceRunError::BeforeCommit { phase, error, .. } =
                result.unwrap_err()
            else {
                panic!("failure must remain before atomic publication");
            };
            assert_eq!(phase, expected);
            if expected == "native execution" {
                assert!(
                    error.kind_as::<NativeProgramExecutionError>().is_some(),
                    "{error:?}"
                );
            } else {
                assert!(
                    error
                        .kind_as::<mech_terminal::CliResourceProviderError>()
                        .is_some(),
                    "{error:?}"
                );
            }
            let trace = trace.lock().unwrap();
            assert_eq!((trace.captures, trace.snapshots, trace.writes), (1, 0, 0));
            assert!(
                trace.body_failed_before_eof,
                "test must exercise a saved pre-EOF native failure"
            );
            assert!(trace.output.is_empty());
            assert_eq!(
                trace.position,
                fail_at.unwrap_or(input.len().min(cap as usize + 1))
            );
            assert_eq!(memory.used_bytes(), 0);
            drop(trace);
            runtime.shutdown().unwrap();
        }
    }
}

#[test]
fn initial_scanner_allocation_failure_waits_for_late_capture_errors() {
    let (artifacts, catalog) = fixture::artifacts(&hosted_scan());
    for artifact in artifacts {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let descriptor = jit.descriptor();
        let region = descriptor.metadata().prefix.as_ref().unwrap();
        let borrowed = &descriptor.metadata().borrowed_strings.regions[region.borrowed];
        let captures = borrowed.proof.matrices().len() + borrowed.proof.literals().len();
        // This is one byte below the real borrowed-owner metadata component
        // in prepare_borrowed_region, independent of the source payload. The
        // much smaller initial source/empty-window claims still fit.
        let mut limits = fixture::limits();
        limits.library.max_temporary_bytes =
            Some((captures * core::mem::size_of::<Option<Value>>()) as u64 + 8192 - 1);
        for (input, fail_at) in [
            (b"bad\nlater\xff".as_slice(), None),
            (b"bad\nlater\n".as_slice(), Some(8)),
        ] {
            let (mut runtime, trace) = runtime(input, true, fail_at);
            let memory = ManagedMemoryBudget::new(TEST_MEMORY_LIMIT);
            let result = execute(
                &mut runtime,
                &artifact,
                &jit,
                limits,
                input.len() as u64,
                &trace,
                &memory,
            );
            let CanonicalNativeResourceRunError::BeforeCommit { phase, error, .. } =
                result.unwrap_err()
            else {
                panic!("initial scanner failure must not permit publication");
            };
            assert_eq!(phase, "capture");
            assert!(
                error
                    .kind_as::<mech_terminal::CliResourceProviderError>()
                    .is_some(),
                "{error:?}"
            );
            let trace = trace.lock().unwrap();
            assert_eq!((trace.captures, trace.snapshots, trace.writes), (1, 0, 0));
            assert_eq!(
                trace.first_failure_position,
                Some(1),
                "the initial native entry must have failed before any completed input row"
            );
            assert_eq!(trace.position, fail_at.unwrap_or(input.len()));
            assert!(trace.output.is_empty());
            assert_eq!(memory.used_bytes(), 0);
            drop(trace);
            runtime.shutdown().unwrap();
        }
    }
}

#[test]
fn final_producer_admission_precedes_a_saved_body_failure() {
    let source = hosted_scan().replace("rows[i] == \"open\"", "rows[i][0u64] == \"open\"");
    let (artifacts, catalog) = fixture::artifacts(&source);
    for artifact in artifacts {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        for comparison in [1_000_000, 0] {
            let input = b"bad\nlater\n";
            let (mut runtime, trace) = runtime(input, true, None);
            let memory = ManagedMemoryBudget::new(TEST_MEMORY_LIMIT);
            let mut limits = fixture::limits();
            limits.library.max_output_elements = Some(1);
            limits.library.max_comparison_work = Some(comparison);
            let error = execute(
                &mut runtime,
                &artifact,
                &jit,
                limits,
                input.len() as u64,
                &trace,
                &memory,
            )
            .unwrap_err();
            let CanonicalNativeResourceRunError::BeforeCommit {
                phase: "native execution",
                error,
                ..
            } = error
            else {
                panic!("source admission must fail before publication");
            };
            assert!(
                matches!(
                    error.kind_as::<NativeProgramExecutionError>(),
                    Some(NativeProgramExecutionError::Library {
                        error:
                            mech_engine::native_artifact::NativeCollectionInvocationError::Kernel(
                                mech_core::ResidentKernelError::ResourceLimit {
                                    dimension: mech_core::MemoryBudgetDimension::OutputElements,
                                    required: 2,
                                    limit: 1,
                                    ..
                                }
                            ),
                        ..
                    })
                ),
                "{error:?}"
            );
            let trace = trace.lock().unwrap();
            assert_eq!((trace.captures, trace.snapshots, trace.writes), (1, 0, 0));
            assert_eq!(
                trace.body_failed_before_eof,
                comparison != 0,
                "zero comparison allowance suspends before body; other case saves real body failure"
            );
            assert_eq!(trace.position, input.len());
            assert_eq!(memory.used_bytes(), 0);
            drop(trace);
            runtime.shutdown().unwrap();
        }
    }
}

#[test]
fn unicode_suspension_keeps_canonical_cells_and_finishes_in_the_same_authorized_turn() {
    let (artifacts, catalog) = fixture::artifacts(&hosted_scan());
    let input = "skip\r\nopen\r\ne\u{301}\r\n猫\r\nclose\r\nafter\r".as_bytes();
    for artifact in artifacts {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let memory = ManagedMemoryBudget::new(TEST_MEMORY_LIMIT);
        let (mut ordinary, reference) = fixture::runtime(input, true);
        fixture::execute(&mut ordinary, &artifact, &jit, fixture::limits(), &memory).unwrap();
        let expected = reference.lock().unwrap().output.clone();
        ordinary.shutdown().unwrap();
        let (mut streaming, trace) = runtime(input, true, None);
        execute(
            &mut streaming,
            &artifact,
            &jit,
            fixture::limits(),
            input.len() as u64,
            &trace,
            &memory,
        )
        .unwrap();
        let trace = trace.lock().unwrap();
        assert_eq!((trace.captures, trace.snapshots, trace.writes), (1, 0, 1));
        assert_eq!(trace.output, expected);
        assert_eq!(memory.used_bytes(), 0);
        drop(trace);
        fixture::no_resident(&streaming);
        streaming.shutdown().unwrap();
    }
}

#[test]
fn extra_count_lookahead_uses_snapshot_before_one_byte_delivery() {
    let source = hosted_scan().replace(
        "| i > count -> :Ready(first, count + 1u64)\n",
        "| i > count -> :Ready(first, count + 1u64)\n    | i + 1u64 > count -> :Ready(first, i)\n",
    );
    let input = b"open\nA\nclose\nB\nC\n";
    let (artifacts, catalog) = fixture::artifacts(&source);
    for artifact in artifacts {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let memory = ManagedMemoryBudget::new(TEST_MEMORY_LIMIT);
        let (mut snapshot, reference) = runtime(input, false, None);
        execute(
            &mut snapshot,
            &artifact,
            &jit,
            fixture::limits(),
            input.len() as u64,
            &reference,
            &memory,
        )
        .unwrap();
        assert_eq!(reference.lock().unwrap().output, "A");
        snapshot.shutdown().unwrap();

        // This provider offers capture and returns exactly one byte per read.
        // Before the proof fix, capture saved STATUS_ABI as soon as row i was
        // available but the added count comparison needed row i + 1.
        let (mut offered, trace) = runtime(input, true, None);
        execute(
            &mut offered,
            &artifact,
            &jit,
            fixture::limits(),
            input.len() as u64,
            &trace,
            &memory,
        )
        .unwrap();
        assert!(jit.descriptor().metadata().prefix.is_none());
        let trace = trace.lock().unwrap();
        assert_eq!((trace.captures, trace.snapshots, trace.writes), (0, 1, 1));
        assert_eq!(trace.output, reference.lock().unwrap().output);
        assert_eq!(trace.position, input.len());
        assert!(!trace.body_failed_before_eof);
        assert_eq!(trace.first_failure_position, None);
        assert_eq!(memory.used_bytes(), 0);
        drop(trace);
        fixture::no_resident(&offered);
        offered.shutdown().unwrap();
    }
}

#[test]
fn returned_forward_endpoints_wait_for_real_rows_in_original_and_decoded_capture() {
    let input = b"open\nA\nclose\nB\nC\n";
    for (transition, expected) in [
        (":Ready(first, i + 2u64)", "A|close|B"),
        (":Ready(first, i + 1u64)", "A|close"),
        (":Ready(i + 2u64, i + 3u64)", "C"),
        (":Ready(i + 2u64, i + 2u64)", ""),
        (":Ready(i + 3u64, i + 2u64)", ""),
    ] {
        let source = hosted_scan().replace(":Ready(first, i)", transition);
        let (artifacts, catalog) = fixture::artifacts(&source);
        for artifact in artifacts {
            let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
            assert!(jit.descriptor().metadata().prefix.is_some(), "{transition}");
            let memory = ManagedMemoryBudget::new(TEST_MEMORY_LIMIT);
            let (mut ordinary, reference) = fixture::runtime(input, true);
            fixture::execute(&mut ordinary, &artifact, &jit, fixture::limits(), &memory).unwrap();
            assert_eq!(reference.lock().unwrap().output, expected);
            fixture::no_resident(&ordinary);
            ordinary.shutdown().unwrap();
            // The provider physically returns one byte per read. The closing
            // row arrives before either forward-looking endpoint is available.
            let (mut streaming, trace) = runtime(input, true, None);
            execute(
                &mut streaming,
                &artifact,
                &jit,
                fixture::limits(),
                input.len() as u64,
                &trace,
                &memory,
            )
            .unwrap();
            let trace = trace.lock().unwrap();
            assert_eq!((trace.captures, trace.snapshots, trace.writes), (1, 0, 1));
            assert_eq!(trace.position, input.len());
            assert_eq!(trace.output, expected);
            assert!(!trace.body_failed_before_eof);
            assert_eq!(memory.used_bytes(), 0);
            drop(trace);
            fixture::no_resident(&streaming);
            streaming.shutdown().unwrap();
        }
    }
}

#[test]
fn unavailable_terminal_endpoint_preserves_eof_selector_error_and_source_priority() {
    let source = hosted_scan().replace(":Ready(first, i)", ":Ready(first, i + 2u64)");
    let input = b"open\nA\nclose\n";
    let (artifacts, catalog) = fixture::artifacts(&source);
    for artifact in artifacts {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        assert!(jit.descriptor().metadata().prefix.is_some());
        let memory = ManagedMemoryBudget::new(TEST_MEMORY_LIMIT);
        let (mut ordinary, reference) = fixture::runtime(input, true);
        let error = fixture::execute(&mut ordinary, &artifact, &jit, fixture::limits(), &memory)
            .unwrap_err();
        let CanonicalNativeResourceRunError::BeforeCommit {
            phase: "native execution",
            error,
            ..
        } = error
        else {
            panic!("ordinary reference must fail at its maintained selector");
        };
        let expected = error.kind_as::<NativeProgramExecutionError>().unwrap();
        assert!(
            matches!(
                expected,
                NativeProgramExecutionError::Library {
                    error: mech_engine::native_artifact::NativeCollectionInvocationError::Kernel(
                        mech_core::ResidentKernelError::InvalidInput
                    ),
                    ..
                }
            ),
            "{expected:?}"
        );
        let expected = format!("{expected:?}");
        assert!(reference.lock().unwrap().output.is_empty());
        ordinary.shutdown().unwrap();
        for fail_at in [None, Some(input.len())] {
            let (mut streaming, trace) = runtime(input, true, fail_at);
            let error = execute(
                &mut streaming,
                &artifact,
                &jit,
                fixture::limits(),
                input.len() as u64,
                &trace,
                &memory,
            )
            .unwrap_err();
            let CanonicalNativeResourceRunError::BeforeCommit { phase, error, .. } = error else {
                panic!("failure must remain before atomic publication");
            };
            if fail_at.is_some() {
                assert_eq!(phase, "capture");
                assert!(
                    error
                        .kind_as::<mech_terminal::CliResourceProviderError>()
                        .is_some()
                );
            } else {
                assert_eq!(phase, "native execution");
                let actual = error.kind_as::<NativeProgramExecutionError>().unwrap();
                assert_eq!(
                    format!("{actual:?}"),
                    expected,
                    "preserve the maintained native selector's exact origin and error"
                );
            }
            let trace = trace.lock().unwrap();
            assert_eq!((trace.captures, trace.snapshots, trace.writes), (1, 0, 0));
            assert_eq!(trace.position, input.len());
            assert!(!trace.body_failed_before_eof);
            assert!(trace.output.is_empty());
            assert_eq!(memory.used_bytes(), 0);
            drop(trace);
            fixture::no_resident(&streaming);
            streaming.shutdown().unwrap();
        }
    }
}

#[test]
fn zero_live_retention_seed_preserves_successful_empty_original_and_decoded_execution() {
    let source = hosted_scan()
        .replace(
            "#Locate(rows, count) -> :Skip(1u64)",
            "#Locate(rows, count) -> :Take(0u64, 1u64)",
        )
        .replace(
            "| i > count -> :Ready(first, count + 1u64)",
            "| i > count -> :Ready(first, first)",
        );
    let (artifacts, catalog) = fixture::artifacts(&source);
    for artifact in artifacts {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let memory = ManagedMemoryBudget::new(TEST_MEMORY_LIMIT);
        let (mut snapshot, reference) = runtime(b"", false, None);
        execute(
            &mut snapshot,
            &artifact,
            &jit,
            fixture::limits(),
            0,
            &reference,
            &memory,
        )
        .unwrap();
        assert_eq!(reference.lock().unwrap().writes, 1);
        assert!(reference.lock().unwrap().output.is_empty());
        snapshot.shutdown().unwrap();

        // The complete program returns an empty range without selecting row
        // zero. An unproved initial retention coordinate must not instead
        // save a permanent failure during the zero-available native entry.
        let (mut offered, trace) = runtime(b"", true, None);
        execute(
            &mut offered,
            &artifact,
            &jit,
            fixture::limits(),
            0,
            &trace,
            &memory,
        )
        .unwrap();
        assert!(jit.descriptor().metadata().prefix.is_none());
        let trace = trace.lock().unwrap();
        assert_eq!((trace.captures, trace.snapshots, trace.writes), (0, 1, 1));
        assert!(trace.output.is_empty());
        assert_eq!(trace.position, 0);
        assert!(!trace.body_failed_before_eof);
        assert_eq!(trace.first_failure_position, None);
        assert_eq!(memory.used_bytes(), 0);
        drop(trace);
        fixture::no_resident(&offered);
        offered.shutdown().unwrap();
    }
}

#[test]
fn unsupported_proof_provider_or_work_bound_selects_snapshot_before_acquisition() {
    for (source, input, offered, tight) in [
        (ECHO.to_owned(), INPUT, true, false),
        (FULL.to_owned(), INPUT, false, false),
        (FULL.to_owned(), INPUT, true, true),
        (
            hosted_scan().replace("i + 1u64", "i + 0u64"),
            b"".as_slice(),
            true,
            false,
        ),
    ] {
        let (artifacts, catalog) = fixture::artifacts(&source);
        for artifact in artifacts {
            let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
            let limits = fixture::limits();
            let input_limit = if tight {
                254_166_745
            } else {
                input.len() as u64
            };
            if tight {
                assert!(
                    super::capture_admission::preflight(
                        jit.descriptor(),
                        limits,
                        (0, 0),
                        input_limit,
                        7,
                        TEST_MEMORY_LIMIT,
                    )
                    .is_none()
                );
            }
            let (mut runtime, trace) = runtime(input, offered, None);
            let memory = ManagedMemoryBudget::new(TEST_MEMORY_LIMIT);
            execute(
                &mut runtime,
                &artifact,
                &jit,
                limits,
                input_limit,
                &trace,
                &memory,
            )
            .unwrap();
            let trace = trace.lock().unwrap();
            assert_eq!((trace.captures, trace.snapshots, trace.writes), (0, 1, 1));
            assert_eq!(memory.used_bytes(), 0);
            drop(trace);
            fixture::no_resident(&runtime);
            runtime.shutdown().unwrap();
        }
    }
}

#[test]
fn trapping_prelude_uses_snapshot_capture_and_preserves_late_source_error_priority() {
    let source = hosted_scan()
        .replace(
            "bounds := #Locate(rows, sequence/length(rows))",
            "count := sequence/length(rows)\nbad := count / 0u64\nbounds := #Locate(rows, count)",
        )
        .replace("first < last =>", "first < last + bad =>");
    let (artifacts, catalog) = fixture::artifacts(&source);
    for artifact in artifacts {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let division = jit
            .descriptor()
            .metadata()
            .plan
            .blocks()
            .iter()
            .find(|block| {
                matches!(
                    block.action(),
                    mech_engine::native_artifact::NativeIterationAction::Scalar(
                        mech_engine::native_artifact::NativeMatchAction::Primitive {
                            op: mech_engine::native_artifact::NativeScalarOp::U64Div,
                            ..
                        }
                    )
                )
            })
            .expect("the trapping value must remain live in the real emitted program");
        assert!(
            !jit.descriptor()
                .metadata()
                .plan
                .activation_nodes()
                .contains(&division.origin().node()),
            "source-dependent division must execute after capture, not during activation"
        );
        assert!(
            super::capture_admission::preflight(
                jit.descriptor(),
                fixture::limits(),
                (0, 0),
                10,
                7,
                TEST_MEMORY_LIMIT
            )
            .is_none()
        );
        for fail_at in [None, Some(8)] {
            let input = b"bad\nlater\n";
            let (mut runtime, trace) = runtime(input, true, fail_at);
            let memory = ManagedMemoryBudget::new(TEST_MEMORY_LIMIT);
            let error = execute(
                &mut runtime,
                &artifact,
                &jit,
                fixture::limits(),
                input.len() as u64,
                &trace,
                &memory,
            )
            .unwrap_err();
            let CanonicalNativeResourceRunError::BeforeCommit { phase, error, .. } = error else {
                panic!("prelude/capture failure must remain before publication");
            };
            assert_eq!(
                phase,
                if fail_at.is_some() {
                    "capture"
                } else {
                    "native execution"
                }
            );
            if fail_at.is_some() {
                assert!(
                    error
                        .kind_as::<mech_terminal::CliResourceProviderError>()
                        .is_some(),
                    "{error:?}"
                );
            } else {
                assert!(matches!(error.kind_as::<NativeProgramExecutionError>(), Some(NativeProgramExecutionError::Control(
                    super::super::super::iteration_runtime::NativeIterationExecutionError::Arithmetic { .. }
                ))), "{error:?}");
            }
            let trace = trace.lock().unwrap();
            assert_eq!((trace.captures, trace.snapshots, trace.writes), (0, 1, 0));
            assert_eq!(trace.position, fail_at.unwrap_or(input.len()));
            assert_eq!(memory.used_bytes(), 0);
            drop(trace);
            runtime.shutdown().unwrap();
        }
    }
}
