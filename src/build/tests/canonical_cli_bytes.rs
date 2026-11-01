//! Generic byte ports through ordinary canonical resource artifacts and real CLI hosts.
#![cfg(feature = "standard-hosts")]

use mech_core::{MResult, MechError, ParsedProgram, ResourceIntent};
use mech_engine::ProgramArtifact;
use mech_runtime::{
    ConfigValue, HostInstanceConfig, ModuleBuildOptions, ResidentDurabilityPolicy,
    ResourcePathCapability, ResourcePathScope, RunResourceGrantConfig, RuntimeBuilder,
    RuntimeHostInputDriver, RuntimeHostInputSource, RuntimeIngress, RuntimeProgramRoute,
    RuntimeResourceReadRequest, SourceRequest, resolver::InMemorySourceResolver,
};
use mech_terminal::{CliBackend, CliHostFactory, CliResourceProviderError};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

#[derive(Clone, Debug, Default)]
struct State {
    input: Vec<u8>,
    reads: usize,
    output: Vec<(String, Vec<u8>)>,
    attempts: usize,
    fail_write: bool,
    fail_flush: bool,
}
#[derive(Clone, Debug, Default)]
struct Backend(Arc<Mutex<State>>);
fn failure(reason: &str) -> MechError {
    MechError::new(
        CliResourceProviderError {
            resource: "test://cli".into(),
            reason: reason.into(),
        },
        None,
    )
}
impl Backend {
    fn write(&self, stream: &str, bytes: &[u8]) -> MResult<()> {
        let mut state = self.0.lock().unwrap();
        state.attempts += 1;
        if state.fail_write {
            return Err(failure("injected byte write failure"));
        }
        state.output.push((stream.into(), bytes.to_vec()));
        if state.fail_flush {
            return Err(failure("injected byte flush failure"));
        }
        Ok(())
    }
}
impl CliBackend for Backend {
    fn env_var(&self, _: &str) -> MResult<Option<String>> {
        Ok(None)
    }
    fn stdin_bytes(&self) -> Option<MResult<Vec<u8>>> {
        let mut state = self.0.lock().unwrap();
        state.reads += 1;
        Some(Ok(state.input.clone()))
    }
    fn stdin_text(&self) -> MResult<String> {
        panic!("raw input must be acquired once")
    }
    fn write_stdout(&mut self, text: &str) -> MResult<()> {
        self.write("stdout", text.as_bytes())
    }
    fn write_stderr(&mut self, text: &str) -> MResult<()> {
        self.write("stderr", text.as_bytes())
    }
    fn write_stdout_bytes(&mut self, bytes: &[u8]) -> MResult<()> {
        self.write("stdout", bytes)
    }
    fn write_stderr_bytes(&mut self, bytes: &[u8]) -> MResult<()> {
        self.write("stderr", bytes)
    }
}
fn builder(backend: Backend, input_bytes: bool, output_bytes: bool) -> RuntimeBuilder {
    let mut builder = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .host_factory(Box::new(CliHostFactory::with_backend(backend).unwrap()))
        .unwrap()
        .host_instance(HostInstanceConfig {
            name: "cli".into(),
            provider: "cli".into(),
            settings: ConfigValue::Map(BTreeMap::new()),
        });
    for (context, operation, bytes) in [
        ("stdin", "read", input_bytes),
        ("stdout", "write", output_bytes),
        ("stderr", "write", output_bytes),
    ] {
        let mut paths = vec!["text".into(), "line".into()];
        if bytes {
            paths.push("bytes".into());
        }
        builder = builder.run_resource_grant(RunResourceGrantConfig {
            target: format!("cli/{context}"),
            operations: vec![operation.into()],
            paths,
        });
    }
    builder
}
fn artifacts(source: &str, backend: Backend) -> [ProgramArtifact; 2] {
    let mut compiler = builder(backend.clone(), true, true)
        .source_resolver(InMemorySourceResolver::new().with_string("bytes.mec", source))
        .build_compiler()
        .unwrap();
    let product = compiler
        .compile_canonical_roots(
            &[SourceRequest::new("bytes.mec")],
            ModuleBuildOptions::new(env!("CARGO_PKG_VERSION"), "v0.3", "native", &[], &[]),
        )
        .unwrap();
    let parsed = ParsedProgram::from_bytes(product.bytecode()).unwrap();
    assert!(parsed.instructions.is_empty());
    assert!(!parsed.artifact.is_empty());
    assert!(parsed.requirements.iter().any(|r|matches!(r,mech_core::ApplicationRequirement::Resource(r) if r.path=="bytes" && r.intent==ResourceIntent::Send)));
    let state = backend.0.lock().unwrap();
    assert_eq!(state.reads, 0, "planning cannot consume stdin");
    assert!(state.output.is_empty());
    drop(state);
    let original = product.artifact().clone();
    let encoded = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&encoded).unwrap();
    assert_eq!(
        encoded,
        mech_engine::encode_program_artifact_bytecode_v1(&decoded).unwrap()
    );
    [original, decoded]
}
fn reset(backend: &Backend, input: &[u8]) {
    *backend.0.lock().unwrap() = State {
        input: input.to_vec(),
        ..Default::default()
    };
}
const MIRROR: &str = "+> @in := cli/stdin\n+> @out := cli/stdout\n+> @err := cli/stderr\n@out/bytes <- @in/bytes\n@out/bytes <- @in/bytes\n@err/bytes <- [0u8 128u8 255u8]\n";

#[test]
fn activated_byte_ports_preserve_binary_rows_multiplicity_and_empty_snapshots() {
    let backend = Backend::default();
    for artifact in artifacts(MIRROR, backend.clone()) {
        for input in [
            b"".as_slice(),
            include_bytes!("fixtures/cli-bytes/revcomp-binary-header.stdin").as_slice(),
            include_bytes!("fixtures/cli-bytes/revcomp-binary-header.stdout").as_slice(),
            include_bytes!("fixtures/cli-bytes/mandelbrot-n1.pbm").as_slice(),
            include_bytes!("fixtures/cli-bytes/mandelbrot-n9.pbm").as_slice(),
        ] {
            reset(&backend, input);
            let mut runtime = builder(backend.clone(), true, true).build().unwrap();
            let loaded = runtime
                .load_compiled_program(artifact.clone(), ResidentDurabilityPolicy::Volatile)
                .unwrap();
            assert_eq!(loaded.route, RuntimeProgramRoute::ResidentExternal);
            runtime.shutdown().unwrap();
            let state = backend.0.lock().unwrap();
            assert_eq!(state.reads, 1);
            assert_eq!(
                state.output,
                vec![
                    ("stdout".into(), input.to_vec()),
                    ("stdout".into(), input.to_vec()),
                    ("stderr".into(), vec![0, 128, 255])
                ]
            );
        }
    }
}
#[test]
fn mixed_text_and_byte_sends_share_one_valid_input_snapshot_in_source_order() {
    let backend = Backend::default();
    let source = "+> @in := cli/stdin\n+> @out := cli/stdout\n@out/bytes <- @in/bytes\n@out/line <- @in/text\n@out/bytes <- @in/bytes\n";
    for artifact in artifacts(source, backend.clone()) {
        let input = "héllo\r\n".as_bytes();
        reset(&backend, input);
        let mut runtime = builder(backend.clone(), true, true).build().unwrap();
        runtime
            .load_compiled_program(artifact, ResidentDurabilityPolicy::Volatile)
            .unwrap();
        runtime.shutdown().unwrap();
        let state = backend.0.lock().unwrap();
        assert_eq!(state.reads, 1);
        let mut line = input.to_vec();
        line.push(b'\n');
        assert_eq!(
            state.output,
            vec![
                ("stdout".into(), input.to_vec()),
                ("stdout".into(), line),
                ("stdout".into(), input.to_vec()),
            ]
        );
    }
}
#[test]
fn invalid_utf8_late_text_view_aborts_all_earlier_byte_sends() {
    let backend = Backend::default();
    let source = "+> @in := cli/stdin\n+> @out := cli/stdout\n@out/bytes <- [0u8 255u8]\n@out/bytes <- @in/bytes\n@out/text <- @in/text\n";
    for artifact in artifacts(source, backend.clone()) {
        reset(&backend, &[0x80, 0xff]);
        let mut runtime = builder(backend.clone(), true, true).build().unwrap();
        let error = runtime
            .load_compiled_program(artifact, ResidentDurabilityPolicy::Volatile)
            .unwrap_err();
        assert!(error.display_message().contains("UTF-8"));
        runtime.shutdown().unwrap();
        let state = backend.0.lock().unwrap();
        assert_eq!(state.reads, 1);
        assert_eq!(state.attempts, 0);
        assert!(state.output.is_empty());
    }
}
#[test]
fn text_grants_do_not_authorize_byte_reads_or_sends() {
    let backend = Backend::default();
    for artifact in artifacts(MIRROR, backend.clone()) {
        for (read, write) in [(false, true), (true, false), (false, false)] {
            reset(&backend, &[0xff]);
            let mut runtime = builder(backend.clone(), read, write).build().unwrap();
            assert!(
                runtime
                    .load_compiled_program(artifact.clone(), ResidentDurabilityPolicy::Volatile)
                    .is_err()
            );
            runtime.shutdown().unwrap();
            let state = backend.0.lock().unwrap();
            assert_eq!(state.reads, 0);
            assert_eq!(state.attempts, 0);
            assert!(state.output.is_empty());
        }
    }
}
#[test]
fn revoked_byte_read_grant_denies_observation_through_the_real_provider() {
    let backend = Backend::default();
    let mut runtime = builder(backend.clone(), false, true).build().unwrap();
    let grant = ResourcePathCapability::new(
        runtime.next_capability_id(),
        runtime.runtime_context().unwrap().subject(),
        "cli://cli/stdin",
        ["read"],
        [ResourcePathScope::Exact("bytes".into())],
    )
    .unwrap();
    let id = runtime.grant_capability(Arc::new(grant)).unwrap();
    runtime.revoke_capability(id).unwrap();
    assert!(
        runtime
            .read_resource(RuntimeResourceReadRequest {
                base_uri: "cli://cli/stdin".into(),
                path: "bytes".into(),
                context_name: "stdin".into()
            })
            .is_err()
    );
    runtime.shutdown().unwrap();
    assert_eq!(backend.0.lock().unwrap().reads, 0);
}
#[test]
fn write_and_flush_delivery_failures_surface_and_shutdown_does_not_replay_bytes() {
    let backend = Backend::default();
    let source = "+> @out := cli/stdout\n@out/bytes <- [0u8 128u8 255u8]\n";
    for artifact in artifacts(source, backend.clone()) {
        for flush in [false, true] {
            *backend.0.lock().unwrap() = State {
                fail_write: !flush,
                fail_flush: flush,
                ..Default::default()
            };
            let mut runtime = builder(backend.clone(), true, true).build().unwrap();
            let error = runtime
                .load_compiled_program(artifact.clone(), ResidentDurabilityPolicy::Volatile)
                .unwrap_err();
            assert!(error.display_message().contains(if flush {
                "flush failure"
            } else {
                "write failure"
            }));
            assert!(
                runtime.retry_resident_outbox().is_err(),
                "failed initial activation must not install a retryable active program"
            );
            runtime.shutdown().unwrap();
            let state = backend.0.lock().unwrap();
            assert_eq!(state.attempts, 1);
            assert_eq!(state.output.len(), usize::from(flush));
        }
    }
}
#[derive(Debug)]
struct StopFailure {
    ingress: Option<RuntimeIngress>,
    stops: Arc<Mutex<usize>>,
}
impl RuntimeHostInputDriver for StopFailure {
    fn drives(&self, _: &RuntimeHostInputSource) -> bool {
        false
    }
    fn attach(&mut self, ingress: RuntimeIngress) -> MResult<()> {
        self.ingress = Some(ingress);
        Ok(())
    }
    fn start(&mut self) -> MResult<()> {
        Ok(())
    }
    fn stop(&mut self) -> MResult<()> {
        *self.stops.lock().unwrap() += 1;
        assert!(self.ingress.as_ref().unwrap().is_closed()?);
        Err(failure("injected shutdown stop failure"))
    }
    fn is_live(&self) -> bool {
        false
    }
}
#[test]
fn shutdown_failure_keeps_delivered_byte_output_and_does_not_repeat_cleanup_on_drop() {
    let backend = Backend::default();
    let source = "+> @out := cli/stdout\n@out/bytes <- [0u8 255u8]\n";
    for artifact in artifacts(source, backend.clone()) {
        reset(&backend, &[]);
        let stops = Arc::new(Mutex::new(0));
        let mut runtime = builder(backend.clone(), true, true)
            .input_driver(StopFailure {
                ingress: None,
                stops: stops.clone(),
            })
            .build()
            .unwrap();
        runtime
            .load_compiled_program(artifact, ResidentDurabilityPolicy::Volatile)
            .unwrap();
        assert!(
            runtime
                .shutdown()
                .unwrap_err()
                .display_message()
                .contains("shutdown stop failure")
        );
        drop(runtime);
        assert_eq!(*stops.lock().unwrap(), 1);
        assert_eq!(
            backend.0.lock().unwrap().output,
            vec![("stdout".into(), vec![0, 255])]
        );
    }
}
#[test]
fn denied_load_recovers_in_the_same_runtime_after_explicit_byte_grants() {
    let backend = Backend::default();
    for artifact in artifacts(MIRROR, backend.clone()) {
        reset(&backend, &[0, 255]);
        let mut runtime = builder(backend.clone(), false, false).build().unwrap();
        assert!(
            runtime
                .load_compiled_program(artifact.clone(), ResidentDurabilityPolicy::Volatile)
                .is_err()
        );
        {
            let state = backend.0.lock().unwrap();
            assert_eq!(state.reads, 0);
            assert!(state.output.is_empty());
        }
        for (target, operation) in [
            ("cli/stdin", "read"),
            ("cli/stdout", "write"),
            ("cli/stderr", "write"),
        ] {
            runtime
                .install_run_resource_grant(&RunResourceGrantConfig {
                    target: target.into(),
                    operations: vec![operation.into()],
                    paths: vec!["bytes".into()],
                })
                .unwrap();
        }
        runtime
            .load_compiled_program(artifact, ResidentDurabilityPolicy::Volatile)
            .unwrap();
        runtime.shutdown().unwrap();
        let state = backend.0.lock().unwrap();
        assert_eq!(state.reads, 1);
        assert_eq!(
            state.output,
            vec![
                ("stdout".into(), vec![0, 255]),
                ("stdout".into(), vec![0, 255]),
                ("stderr".into(), vec![0, 128, 255])
            ]
        );
    }
}
