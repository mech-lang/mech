//! Resource-delivery boundary probes, NOT Rust evaluations of the Mech source.
//! The final test uses an actual native session to reject this new storage form
//! until the compiler/native continuation has separately implemented it.
use super::*;
use mech_core::{GenericError, MResult};
use mech_terminal::{CliBackend, CliHostFactory};
use std::{
    cell::Cell,
    ffi::OsString,
    io::{self, Read},
    sync::{
        Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};

#[derive(Clone, Copy, Debug)]
enum Loan {
    Once,
    None,
    Twice,
}
#[derive(Debug)]
struct Trace {
    input: Vec<u8>,
    opted: bool,
    fail_at: Option<usize>,
    loan: Loan,
    position: AtomicUsize,
    reads: AtomicUsize,
    captures: AtomicUsize,
    snapshots: AtomicUsize,
    lock: Mutex<()>,
    output: Mutex<Vec<String>>,
}
#[derive(Clone, Debug)]
struct Backend(Arc<Trace>);

#[derive(Debug)]
struct MutableReadGrant {
    inner: ResourcePathCapability,
    enabled: Arc<AtomicBool>,
}
impl Capability for MutableReadGrant {
    fn id(&self) -> CapabilityId {
        self.inner.id()
    }
    fn subject_key(&self) -> &str {
        self.inner.subject_key()
    }
    fn validate(&self) -> MResult<()> {
        self.inner.validate()
    }
    fn check(&self, request: &CapabilityRequest) -> MResult<CapabilityDecision> {
        self.preview_check(request)
    }
    fn preview_check(&self, request: &CapabilityRequest) -> MResult<CapabilityDecision> {
        if self.enabled.load(Ordering::SeqCst) {
            self.inner.preview_check(request)
        } else {
            Ok(CapabilityDecision::deny(
                "capture fixture authority revoked",
            ))
        }
    }
}
struct Reader(Arc<Trace>);
impl Read for Reader {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        self.0.reads.fetch_add(1, Ordering::SeqCst);
        let position = self.0.position.load(Ordering::SeqCst);
        if self.0.fail_at == Some(position) {
            return Err(io::Error::other("late source failure"));
        }
        let count = output
            .len()
            .min(2)
            .min(self.0.input.len() - position)
            .min(self.0.fail_at.map_or(usize::MAX, |at| at - position));
        output[..count].copy_from_slice(&self.0.input[position..position + count]);
        self.0.position.store(position + count, Ordering::SeqCst);
        Ok(count)
    }
}
impl CliBackend for Backend {
    fn env_var(&self, _: &str) -> MResult<Option<String>> {
        Ok(Some("meta".into()))
    }
    fn argument(&self, _: usize) -> MResult<Option<OsString>> {
        Ok(Some("argument".into()))
    }
    fn stdin_bytes(&self) -> Option<MResult<Vec<u8>>> {
        self.0.snapshots.fetch_add(1, Ordering::SeqCst);
        self.0.position.store(self.0.input.len(), Ordering::SeqCst);
        Some(Ok(self.0.input.clone()))
    }
    fn supports_single_use_stdin_text_capture(&self) -> bool {
        self.0.opted
    }
    fn with_single_use_stdin_text_reader(
        &self,
        capture: &mut dyn FnMut(&mut dyn Read) -> MResult<()>,
    ) -> MResult<()> {
        self.0.captures.fetch_add(1, Ordering::SeqCst);
        let _loan = self.0.lock.lock().unwrap();
        let mut reader = Reader(self.0.clone());
        match self.0.loan {
            Loan::None => Ok(()),
            Loan::Once => capture(&mut reader),
            Loan::Twice => {
                capture(&mut reader)?;
                let _ignored = capture(&mut reader);
                Ok(())
            }
        }
    }
    fn write_stdout(&mut self, text: &str) -> MResult<()> {
        self.0.output.lock().unwrap().push(text.into());
        Ok(())
    }
    fn write_stderr(&mut self, _: &str) -> MResult<()> {
        panic!("unexpected stderr effect");
    }
}
fn setup(
    input: &[u8],
    opted: bool,
    grants: bool,
    fail_at: Option<usize>,
    loan: Loan,
) -> (MechRuntime, Arc<Trace>) {
    let trace = Arc::new(Trace {
        input: input.into(),
        opted,
        fail_at,
        loan,
        position: AtomicUsize::new(0),
        reads: AtomicUsize::new(0),
        captures: AtomicUsize::new(0),
        snapshots: AtomicUsize::new(0),
        lock: Mutex::new(()),
        output: Mutex::new(Vec::new()),
    });
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
    if grants {
        for (target, operation, paths) in [
            ("cli/stdin", "read", vec!["text".into()]),
            ("cli/stdout", "write", vec!["text".into()]),
            ("cli/env", "read", vec!["FOO".into()]),
        ] {
            builder = builder.run_resource_grant(RunResourceGrantConfig {
                target: target.into(),
                operations: vec![operation.into()],
                paths,
            });
        }
    }
    (builder.build().unwrap(), trace)
}
fn policy(limit: u64) -> CanonicalNativeResourceLimits {
    let mut policy = bridge_limits();
    policy.max_input_text_bytes = limit;
    policy
}
fn failure(message: &str) -> MechError {
    MechError::new(
        GenericError {
            msg: message.into(),
        },
        None,
    )
}
fn completion(result: u64) -> MResult<CanonicalNativeResourceCompletion<u64>> {
    Ok(CanonicalNativeResourceCompletion {
        result,
        effects: vec![RuntimeHostInputValue::String("committed".into()).into_value()?]
            .into_boxed_slice(),
    })
}
struct Probe {
    invocation: CanonicalNativeResourceInvocation,
    revision: mech_core::ProgramRevision,
    delivered: u64,
}
fn prepare(
    admission: CanonicalNativeResourceAdmission<'_>,
    memory: &ManagedMemoryBudget,
    limit: u64,
) -> MResult<CanonicalNativeResourcePrepared<Probe>> {
    assert!(admission.text_capture_site().is_some());
    let state = Probe {
        invocation: admission.invocation(),
        revision: admission.program_revision(),
        delivered: 0,
    };
    let buffer = RuntimeTextCaptureBuffer::prepare(limit, 3, memory)
        .map_err(|error| failure(&error.to_string()))?;
    Ok(CanonicalNativeResourcePrepared::single_use_text(
        state, buffer,
    ))
}

#[test]
fn scoped_original_and_decoded_capture_has_one_transaction_and_no_partial_value() {
    let (variants, _) = artifacts(ECHO);
    for artifact in variants {
        for input in ["", "é🦎\r\nlast", "a\0b"] {
            let (mut runtime, trace) = setup(input.as_bytes(), true, true, None, Loan::Once);
            let memory = ManagedMemoryBudget::new(64);
            let result = runtime
                .with_canonical_native_resources_phased_text(
                    &artifact,
                    policy(input.len() as u64),
                    |admission| prepare(admission, &memory, input.len() as u64),
                    |state, chunk| {
                        assert_eq!(chunk.invocation(), state.invocation);
                        assert_eq!(chunk.program_revision(), state.revision);
                        assert_eq!(chunk.offset(), state.delivered);
                        assert!(memory.used_bytes() > 0);
                        assert!(trace.output.lock().unwrap().is_empty());
                        state.delivered += chunk.text().len() as u64;
                        Ok(())
                    },
                    |state, turn| {
                        assert_eq!(state.invocation, turn.invocation());
                        assert_eq!(state.delivered, input.len() as u64);
                        assert!(
                            turn.observations().is_empty(),
                            "no fake canonical String lane"
                        );
                        assert_eq!(turn.text_capture().unwrap().input_bytes(), state.delivered);
                        assert_eq!(memory.used_bytes(), 0);
                        assert_eq!(trace.position.load(Ordering::SeqCst), input.len());
                        assert!(
                            trace.lock.try_lock().is_ok(),
                            "source loan must be released before completion"
                        );
                        completion(state.delivered)
                    },
                )
                .unwrap();
            assert_eq!(result, input.len() as u64);
            assert_eq!(trace.captures.load(Ordering::SeqCst), 1);
            assert_eq!(trace.snapshots.load(Ordering::SeqCst), 0);
            assert_eq!(*trace.output.lock().unwrap(), ["committed"]);
            no_resident(&runtime);
            runtime.shutdown().unwrap();
        }
    }
}

#[test]
fn missing_authority_and_wrong_cap_prevent_even_one_source_read() {
    let (variants, _) = artifacts(ECHO);
    for artifact in variants {
        for granted in [false, true] {
            let (mut runtime, trace) = setup(b"input", true, granted, None, Loan::Once);
            let memory = ManagedMemoryBudget::new(64);
            let prepares = Cell::new(0);
            let result: Result<u64, _> = runtime.with_canonical_native_resources_phased_text(
                &artifact,
                policy(5),
                |admission| {
                    prepares.set(prepares.get() + 1);
                    prepare(admission, &memory, 6)
                },
                |_, _| panic!("denied capture cannot run consumer"),
                |_, _| completion(0),
            );
            assert!(result.is_err());
            assert_eq!(prepares.get(), usize::from(granted));
            assert_eq!(trace.reads.load(Ordering::SeqCst), 0);
            assert_eq!(trace.captures.load(Ordering::SeqCst), 0);
            assert!(trace.output.lock().unwrap().is_empty());
            assert_eq!(memory.used_bytes(), 0);
            runtime.shutdown().unwrap();
        }
    }
}

#[test]
fn capture_scratch_denial_precedes_even_a_failing_source() {
    let (variants, _) = artifacts(ECHO);
    for artifact in variants {
        for fail_at in [None, Some(0)] {
            let (mut runtime, trace) = setup(b"input", true, true, fail_at, Loan::Once);
            // A three-byte chunk plus three carry bytes requires six bytes.
            let memory = ManagedMemoryBudget::new(5);
            let result = runtime.with_canonical_native_resources_phased_text(
                &artifact,
                policy(5),
                |admission| prepare(admission, &memory, 5),
                |_, _| panic!("unfunded capture cannot receive a fragment"),
                |_, _| completion(0),
            );
            assert!(matches!(
                result,
                Err(CanonicalNativeResourceRunError::BeforeCommit {
                    phase: "native activation",
                    ..
                })
            ));
            assert_eq!(trace.reads.load(Ordering::SeqCst), 0);
            assert_eq!(trace.captures.load(Ordering::SeqCst), 0);
            assert_eq!(trace.snapshots.load(Ordering::SeqCst), 0);
            assert!(trace.output.lock().unwrap().is_empty());
            assert_eq!(memory.used_bytes(), 0);
            runtime.shutdown().unwrap();
        }
    }
}

#[test]
fn late_capture_failures_outrank_native_errors_and_never_stage_output() {
    let (variants, _) = artifacts(ECHO);
    for artifact in variants {
        for (input, fail_io, limit, expected) in [
            (b"valid".as_slice(), true, 5, "capture"),
            (b"a\xffbad".as_slice(), false, 5, "capture"),
            (b"valid".as_slice(), false, 2, "capture"),
            (b"valid".as_slice(), false, 5, "native execution"),
        ] {
            let (mut runtime, trace) = setup(
                input,
                true,
                true,
                fail_io.then_some(input.len()),
                Loan::Once,
            );
            let memory = ManagedMemoryBudget::new(64);
            let calls = Cell::new(0);
            let result = runtime.with_canonical_native_resources_phased_text(
                &artifact,
                policy(limit),
                |admission| prepare(admission, &memory, limit),
                |_, _| {
                    calls.set(calls.get() + 1);
                    Err(failure("native capture consumer failed"))
                },
                |_, _| -> MResult<CanonicalNativeResourceCompletion<u64>> {
                    panic!("failed capture has no completion token")
                },
            );
            let Err(CanonicalNativeResourceRunError::BeforeCommit { phase, error, .. }) = result
            else {
                panic!("expected capture failure");
            };
            assert_eq!(phase, expected);
            if expected == "native execution" {
                assert!(error.kind_as::<GenericError>().is_some());
            }
            if input.contains(&0xff) {
                assert!(
                    error
                        .kind_as::<mech_terminal::CliResourceProviderError>()
                        .is_some()
                );
                assert!(
                    error
                        .display_message()
                        .contains("standard input is not valid UTF-8")
                );
            }
            assert_eq!(calls.get(), 1);
            assert_eq!(
                trace.position.load(Ordering::SeqCst),
                input.len().min(limit as usize + 1)
            );
            assert_eq!(trace.captures.load(Ordering::SeqCst), 1);
            assert_eq!(trace.snapshots.load(Ordering::SeqCst), 0);
            assert!(trace.output.lock().unwrap().is_empty());
            assert_eq!(memory.used_bytes(), 0);
            no_resident(&runtime);
            runtime.shutdown().unwrap();
        }
    }
}

#[test]
fn consumer_panic_propagates_after_reader_release_without_poisoning_provider() {
    let (variants, _) = artifacts(ECHO);
    for artifact in variants {
        let (mut runtime, trace) = setup(b"whole input", true, true, None, Loan::Once);
        let memory = ManagedMemoryBudget::new(64);
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            runtime.with_canonical_native_resources_phased_text(
                &artifact,
                policy(11),
                |admission| prepare(admission, &memory, 11),
                |_, _| std::panic::panic_any(0x1234u32),
                |_, _| completion(0),
            )
        }));
        assert_eq!(*caught.unwrap_err().downcast::<u32>().unwrap(), 0x1234);
        assert_eq!(trace.position.load(Ordering::SeqCst), 11);
        assert!(trace.lock.lock().is_ok());
        assert!(trace.output.lock().unwrap().is_empty());
        assert_eq!(memory.used_bytes(), 0);
        runtime.shutdown().unwrap();
    }
}

#[test]
fn post_capture_revalidation_precedes_deferred_consumer_error_or_panic() {
    let (variants, _) = artifacts(ECHO);
    for artifact in variants {
        for panic in [false, true] {
            let (mut runtime, trace) = setup(b"input", true, false, None, Loan::Once);
            runtime
                .install_run_resource_grant(&RunResourceGrantConfig {
                    target: "cli/stdout".into(),
                    operations: vec!["write".into()],
                    paths: vec!["text".into()],
                })
                .unwrap();
            let enabled = Arc::new(AtomicBool::new(true));
            let capability = ResourcePathCapability::new(
                runtime.next_capability_id(),
                runtime.runtime_context().unwrap().subject().to_owned(),
                "cli://cli/stdin",
                ["read"],
                [ResourcePathScope::Exact("text".into())],
            )
            .unwrap();
            runtime
                .grant_capability(Arc::new(MutableReadGrant {
                    inner: capability,
                    enabled: enabled.clone(),
                }))
                .unwrap();
            let memory = ManagedMemoryBudget::new(64);
            let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                runtime.with_canonical_native_resources_phased_text(
                    &artifact,
                    policy(5),
                    |admission| prepare(admission, &memory, 5),
                    |_, _| {
                        enabled.store(false, Ordering::SeqCst);
                        if panic {
                            std::panic::panic_any(0x9876u32);
                        }
                        Err(failure("deferred native checkpoint failed"))
                    },
                    |_, _| completion(0),
                )
            }));
            let result = caught.expect("revoked authority must precede the deferred native panic");
            assert!(matches!(
                result,
                Err(CanonicalNativeResourceRunError::BeforeCommit {
                    phase: "authorization after capture",
                    ..
                })
            ));
            assert_eq!(trace.position.load(Ordering::SeqCst), 5);
            assert!(trace.lock.lock().is_ok());
            assert!(trace.output.lock().unwrap().is_empty());
            assert_eq!(memory.used_bytes(), 0);
            runtime.shutdown().unwrap();
        }
    }
}

#[test]
fn provider_omission_or_replay_cannot_forge_capture_completion() {
    let (variants, _) = artifacts(ECHO);
    for artifact in variants {
        for loan in [Loan::None, Loan::Twice] {
            let (mut runtime, trace) = setup(b"input", true, true, None, loan);
            let memory = ManagedMemoryBudget::new(64);
            let result = runtime.with_canonical_native_resources_phased_text(
                &artifact,
                policy(5),
                |admission| prepare(admission, &memory, 5),
                |_, _| Ok(()),
                |_, _| -> MResult<CanonicalNativeResourceCompletion<u64>> {
                    panic!("invalid provider cannot complete")
                },
            );
            assert!(matches!(
                result,
                Err(CanonicalNativeResourceRunError::BeforeCommit {
                    phase: "capture",
                    ..
                })
            ));
            assert!(trace.output.lock().unwrap().is_empty());
            assert_eq!(memory.used_bytes(), 0);
            runtime.shutdown().unwrap();
        }
    }
}

#[test]
fn unsupported_or_multi_observation_sources_retain_real_snapshot_delivery() {
    for source in [
        ECHO,
        "+> @in := cli/stdin\n+> @env := cli/env\n+> @out := cli/stdout\n@out/text <- @in/text + @env/FOO\n",
    ] {
        let (variants, _) = artifacts(source);
        for artifact in variants {
            let (mut runtime, trace) = setup(b"input", source != ECHO, true, None, Loan::Once);
            let result = runtime
                .with_canonical_native_resources_phased_text(
                    &artifact,
                    policy(64),
                    |admission| {
                        assert!(admission.text_capture_site().is_none());
                        Ok(CanonicalNativeResourcePrepared::snapshot(()))
                    },
                    |_, _| panic!("ordinary provider cannot receive partial capture"),
                    |_, turn| {
                        assert!(turn.text_capture().is_none());
                        assert!(!turn.observations().is_empty());
                        completion(turn.observations().len() as u64)
                    },
                )
                .unwrap();
            assert_eq!(result, if source == ECHO { 1 } else { 2 });
            assert_eq!(trace.snapshots.load(Ordering::SeqCst), 1);
            assert_eq!(trace.captures.load(Ordering::SeqCst), 0);
            runtime.shutdown().unwrap();
        }
    }
}

#[test]
fn snapshot_native_session_refuses_scoped_capture_without_a_new_lowering() {
    let (variants, catalog) = artifacts(ECHO);
    for artifact in variants {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let (mut runtime, trace) = setup(b"input", true, true, None, Loan::Once);
        let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
        let mut outputs = vec![None; jit.descriptor().plan().transport_output_count()];
        let result = runtime.with_canonical_native_resources_phased_text(
            &artifact,
            policy(5),
            |admission| {
                let buffer = RuntimeTextCaptureBuffer::prepare(5, 3, &memory).unwrap();
                let state = jit
                    .prepare_hosted(admission, limits(), &memory)
                    .map_err(|e| MechError::new(e, None))?;
                Ok(CanonicalNativeResourcePrepared::single_use_text(
                    state, buffer,
                ))
            },
            |_, _| Ok(()),
            |session, turn| {
                session
                    .resume_hosted_into(turn, &[], &mut outputs)
                    .map_err(|e| MechError::new(e, None))?;
                completion(0)
            },
        );
        assert!(matches!(
            result,
            Err(CanonicalNativeResourceRunError::BeforeCommit {
                phase: "native execution",
                ..
            })
        ));
        assert!(outputs.iter().all(Option::is_none));
        assert!(trace.output.lock().unwrap().is_empty());
        assert_eq!(memory.used_bytes(), 0);
        runtime.shutdown().unwrap();
    }
}
