//! Actual immutable payload metadata through canonical original/decoded sessions.
#![cfg(feature = "standard-hosts")]

use mech_core::{MResult, MechError, ParsedProgram, ResourceIntent};
use mech_engine::ProgramArtifact;
use mech_runtime::{
    ConfigValue, HostInstanceConfig, ModuleBuildOptions, ResidentDurabilityPolicy,
    RunResourceGrantConfig, RuntimeBuilder, RuntimeProgramRoute, SourceRequest,
    resolver::InMemorySourceResolver,
};
use mech_terminal::{
    ArgumentSnapshotCliBackend, CliBackend, CliHostFactory, CliResourceProviderError,
};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    sync::{Arc, Mutex},
};

#[derive(Debug, Default)]
struct State {
    count_reads: usize,
    argument_reads: usize,
    output: Vec<String>,
    attempts: usize,
}

#[derive(Clone, Debug, Default)]
struct Output(Arc<Mutex<State>>);

impl CliBackend for Output {
    fn env_var(&self, _: &str) -> MResult<Option<String>> {
        Ok(None)
    }
    fn argument(&self, _: usize) -> MResult<Option<OsString>> {
        panic!("the immutable wrapper owns payload indexed reads")
    }
    fn argument_count(&self) -> MResult<u64> {
        panic!("the immutable wrapper owns payload cardinality")
    }
    fn write_stdout(&mut self, text: &str) -> MResult<()> {
        let mut state = self.0.lock().unwrap();
        state.attempts += 1;
        state.output.push(text.into());
        Ok(())
    }
    fn write_stderr(&mut self, _: &str) -> MResult<()> {
        panic!("the count fixtures do not send stderr")
    }
}

#[derive(Clone, Debug)]
struct Snapshot {
    inner: ArgumentSnapshotCliBackend<Output>,
    state: Arc<Mutex<State>>,
}

impl Snapshot {
    fn new(arguments: Vec<OsString>) -> Self {
        let output = Output::default();
        Self {
            state: output.0.clone(),
            inner: ArgumentSnapshotCliBackend::new(output, arguments),
        }
    }
}

impl CliBackend for Snapshot {
    fn env_var(&self, name: &str) -> MResult<Option<String>> {
        self.inner.env_var(name)
    }
    fn argument(&self, index: usize) -> MResult<Option<OsString>> {
        self.state.lock().unwrap().argument_reads += 1;
        self.inner.argument(index)
    }
    fn argument_count(&self) -> MResult<u64> {
        self.state.lock().unwrap().count_reads += 1;
        self.inner.argument_count()
    }
    fn write_stdout(&mut self, text: &str) -> MResult<()> {
        self.inner.write_stdout(text)
    }
    fn write_stderr(&mut self, text: &str) -> MResult<()> {
        self.inner.write_stderr(text)
    }
}

fn builder(backend: Snapshot, paths: &[&str]) -> RuntimeBuilder {
    RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .host_factory(Box::new(CliHostFactory::with_backend(backend).unwrap()))
        .unwrap()
        .host_instance(HostInstanceConfig {
            name: "cli".into(),
            provider: "cli".into(),
            settings: ConfigValue::Map(BTreeMap::new()),
        })
        .run_resource_grant(RunResourceGrantConfig {
            target: "cli/args".into(),
            operations: vec!["read".into()],
            paths: paths.iter().map(|path| (*path).into()).collect(),
        })
        .run_resource_grant(RunResourceGrantConfig {
            target: "cli/stdout".into(),
            operations: vec!["write".into()],
            paths: vec!["text".into(), "line".into()],
        })
}

fn artifacts(source: &str) -> [ProgramArtifact; 2] {
    // A strict-one guard would fail for this empty vector if compilation ever
    // evaluated a planning placeholder or observed the actual payload count.
    let backend = Snapshot::new(Vec::new());
    let mut compiler = builder(backend.clone(), &["count", "1"])
        .source_resolver(InMemorySourceResolver::new().with_string("count.mec", source))
        .build_compiler()
        .unwrap();
    let product = compiler
        .compile_canonical_roots(
            &[SourceRequest::new("count.mec")],
            ModuleBuildOptions::new(env!("CARGO_PKG_VERSION"), "v0.3", "native", &[], &[]),
        )
        .unwrap();
    let parsed = ParsedProgram::from_bytes(product.bytecode()).unwrap();
    assert!(parsed.instructions.is_empty());
    assert!(!parsed.artifact.is_empty());
    assert!(parsed.requirements.iter().any(|requirement| {
        matches!(requirement, mech_core::ApplicationRequirement::Resource(resource)
            if resource.path == "count" && resource.intent == ResourceIntent::Read)
    }));
    let state = backend.state.lock().unwrap();
    assert_eq!(state.count_reads, 0, "planning must not ask for real argc");
    assert_eq!(
        state.argument_reads, 0,
        "planning must not read payload text"
    );
    assert_eq!(state.attempts, 0);
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

const STRICT_ONE: &str = "+> number\nparse-input(count<u64>, raw<string>) => <u64>\n  | (1u64, value) => number/parse-u64(value).\n+> @args := cli/args\n+> @out := cli/stdout\n@out/text <- \"prefix\"\nn := parse-input(@args/count, @args/1)\n@out/text <- number/format(n, 0u64)\n";

#[test]
fn strict_one_partial_pattern_accepts_one_and_aborts_missing_extra_and_parse_errors() {
    for artifact in artifacts(STRICT_ONE) {
        for (arguments, succeeds) in [
            (vec![OsString::from("19")], true),
            (Vec::new(), false),
            (vec![OsString::from("19"), OsString::from("extra")], false),
            (vec![OsString::from("19"), OsString::from("")], false),
            (vec![OsString::from("")], false),
            (vec![OsString::from("not-a-number")], false),
        ] {
            let extra_argument = arguments.len() > 1;
            let backend = Snapshot::new(arguments);
            let mut runtime = builder(backend.clone(), &["count", "1"]).build().unwrap();
            let result =
                runtime.load_compiled_program(artifact.clone(), ResidentDurabilityPolicy::Volatile);
            assert_eq!(result.is_ok(), succeeds);
            match result {
                Ok(loaded) => assert_eq!(loaded.route, RuntimeProgramRoute::ResidentExternal),
                Err(error) if extra_argument => assert!(
                    error.display_message().contains("InvalidInput"),
                    "the partial arity guard must reject the extra payload: {error:?}"
                ),
                Err(_) => {}
            }
            runtime.shutdown().unwrap();
            let state = backend.state.lock().unwrap();
            if succeeds {
                assert_eq!(state.output, vec!["prefix", "19"]);
                assert_eq!(state.attempts, 2);
            } else {
                assert!(
                    state.output.is_empty(),
                    "the earlier source send must not commit"
                );
                assert_eq!(state.attempts, 0);
            }
        }
    }
}

#[test]
fn strict_one_requires_both_exact_grants_and_wildcard_explicitly_covers_count() {
    for artifact in artifacts(STRICT_ONE) {
        for paths in [vec!["1"], vec!["count"], vec!["*"]] {
            let backend = Snapshot::new(vec![OsString::from("19")]);
            let mut runtime = builder(backend.clone(), &paths).build().unwrap();
            let result =
                runtime.load_compiled_program(artifact.clone(), ResidentDurabilityPolicy::Volatile);
            assert_eq!(result.is_ok(), paths == vec!["*"]);
            runtime.shutdown().unwrap();
            let state = backend.state.lock().unwrap();
            if paths == vec!["*"] {
                assert_eq!(state.output, vec!["prefix", "19"]);
            } else {
                assert_eq!(state.count_reads, 0);
                assert_eq!(state.argument_reads, 0);
                assert_eq!(state.attempts, 0);
                assert!(state.output.is_empty());
            }
        }
    }
}

#[test]
fn failed_strict_load_can_recover_without_mutating_the_immutable_payload() {
    let accept_two = "accept-two(count<u64>) => <string>\n  | 2u64 => \"recovered\".\n+> @args := cli/args\n+> @out := cli/stdout\n@out/text <- accept-two(@args/count)\n";
    for (strict, recovered) in artifacts(STRICT_ONE).into_iter().zip(artifacts(accept_two)) {
        let backend = Snapshot::new(vec![OsString::from("19"), OsString::from("extra")]);
        let mut runtime = builder(backend.clone(), &["count", "1"]).build().unwrap();
        assert!(
            runtime
                .load_compiled_program(strict, ResidentDurabilityPolicy::Volatile)
                .is_err()
        );
        assert!(backend.state.lock().unwrap().output.is_empty());
        assert_eq!(backend.inner.argument_count().unwrap(), 2);
        runtime
            .load_compiled_program(recovered, ResidentDurabilityPolicy::Volatile)
            .unwrap();
        runtime.shutdown().unwrap();
        let state = backend.state.lock().unwrap();
        assert_eq!(state.output, vec!["recovered"]);
        assert_eq!(state.attempts, 1);
    }
}

#[test]
fn count_and_index_preserve_empty_unicode_and_flag_looking_payload_values() {
    let source = "+> number\n+> @args := cli/args\n+> @out := cli/stdout\n@out/line <- number/format(@args/count, 0u64)\n@out/text <- @args/1\n";
    for artifact in artifacts(source) {
        for argument in ["", "héllo\r\n", "--once", "--runtime-info", "--"] {
            let backend = Snapshot::new(vec![OsString::from(argument)]);
            let mut runtime = builder(backend.clone(), &["count", "1"]).build().unwrap();
            runtime
                .load_compiled_program(artifact.clone(), ResidentDurabilityPolicy::Volatile)
                .unwrap();
            runtime.shutdown().unwrap();
            assert_eq!(backend.state.lock().unwrap().output, vec!["1\n", argument]);
        }
    }
}

#[cfg(unix)]
#[test]
fn count_observes_non_unicode_arity_without_decoding_unrequested_extra_payloads() {
    use std::os::unix::ffi::OsStringExt;
    let count_only = "+> number\n+> @args := cli/args\n+> @out := cli/stdout\n@out/text <- number/format(@args/count, 0u64)\n";
    for artifact in artifacts(count_only) {
        let backend = Snapshot::new(vec![
            OsString::from_vec(vec![0xff]),
            OsString::from(""),
            OsString::from("--once"),
        ]);
        let mut runtime = builder(backend.clone(), &["count"]).build().unwrap();
        runtime
            .load_compiled_program(artifact, ResidentDurabilityPolicy::Volatile)
            .unwrap();
        runtime.shutdown().unwrap();
        let state = backend.state.lock().unwrap();
        assert_eq!(state.output, vec!["3"]);
        assert_eq!(state.argument_reads, 0);
    }
    for artifact in artifacts(STRICT_ONE) {
        for arguments in [
            vec![OsString::from_vec(vec![0xff])],
            vec![OsString::from("19"), OsString::from_vec(vec![0xff])],
        ] {
            let first_is_invalid = arguments.len() == 1;
            let backend = Snapshot::new(arguments);
            let mut runtime = builder(backend.clone(), &["count", "1"]).build().unwrap();
            let error = runtime
                .load_compiled_program(artifact.clone(), ResidentDurabilityPolicy::Volatile)
                .unwrap_err();
            assert_eq!(
                error.display_message().contains("Unicode"),
                first_is_invalid
            );
            runtime.shutdown().unwrap();
            let state = backend.state.lock().unwrap();
            assert!(state.output.is_empty());
            assert_eq!(state.attempts, 0);
        }
    }
}

#[test]
fn legacy_backend_count_failure_does_not_substitute_a_planning_zero() {
    #[derive(Clone, Debug)]
    struct Legacy(Output);
    impl CliBackend for Legacy {
        fn env_var(&self, _: &str) -> MResult<Option<String>> {
            Ok(None)
        }
        fn argument(&self, index: usize) -> MResult<Option<OsString>> {
            Ok((index == 1).then(|| OsString::from("19")))
        }
        fn write_stdout(&mut self, text: &str) -> MResult<()> {
            self.0.write_stdout(text)
        }
        fn write_stderr(&mut self, _: &str) -> MResult<()> {
            Err(MechError::new(
                CliResourceProviderError {
                    resource: "legacy".into(),
                    reason: "no stderr".into(),
                },
                None,
            ))
        }
    }
    for artifact in artifacts(STRICT_ONE) {
        let output = Output::default();
        let mut runtime = RuntimeBuilder::new()
            .function_catalog(mech_stdlib::source_catalog())
            .host_factory(Box::new(
                CliHostFactory::with_backend(Legacy(output.clone())).unwrap(),
            ))
            .unwrap()
            .host_instance(HostInstanceConfig {
                name: "cli".into(),
                provider: "cli".into(),
                settings: ConfigValue::Map(BTreeMap::new()),
            })
            .run_resource_grant(RunResourceGrantConfig {
                target: "cli/args".into(),
                operations: vec!["read".into()],
                paths: vec!["count".into(), "1".into()],
            })
            .run_resource_grant(RunResourceGrantConfig {
                target: "cli/stdout".into(),
                operations: vec!["write".into()],
                paths: vec!["text".into()],
            })
            .build()
            .unwrap();
        let error = runtime
            .load_compiled_program(artifact, ResidentDurabilityPolicy::Volatile)
            .unwrap_err();
        assert!(error.display_message().contains("count is unavailable"));
        runtime.shutdown().unwrap();
        assert_eq!(output.0.lock().unwrap().attempts, 0);
    }
}
