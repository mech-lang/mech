//! Ordered sends and failed-turn isolation through ordinary canonical artifacts.
#![cfg(feature = "standard-hosts")]

use mech_core::{MResult, ResourceIntent};
use mech_engine::ProgramArtifact;
use mech_runtime::{
    ConfigValue, HostInstanceConfig, ModuleBuildOptions, ResidentDurabilityPolicy,
    RunResourceGrantConfig, RuntimeBuilder, SourceRequest, resolver::InMemorySourceResolver,
};
use mech_terminal::{CliBackend, CliHostFactory};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    sync::{Arc, Mutex},
};

#[derive(Debug, Default)]
struct Trace {
    argument_reads: usize,
    output: Vec<String>,
}
#[derive(Clone, Debug)]
struct Backend {
    argument: OsString,
    trace: Arc<Mutex<Trace>>,
}
impl CliBackend for Backend {
    fn argument(&self, index: usize) -> MResult<Option<OsString>> {
        self.trace.lock().unwrap().argument_reads += 1;
        Ok((index == 1).then(|| self.argument.clone()))
    }
    fn env_var(&self, _: &str) -> MResult<Option<String>> {
        Ok(None)
    }
    fn write_stdout(&mut self, text: &str) -> MResult<()> {
        self.trace.lock().unwrap().output.push(text.into());
        Ok(())
    }
    fn write_stderr(&mut self, _: &str) -> MResult<()> {
        panic!("unexpected stderr effect")
    }
}
fn builder(backend: Backend) -> RuntimeBuilder {
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
            paths: vec!["1".into()],
        })
        .run_resource_grant(RunResourceGrantConfig {
            target: "cli/stdout".into(),
            operations: vec!["write".into()],
            paths: vec!["text".into()],
        })
}
fn artifacts(source: &str, backend: Backend) -> [ProgramArtifact; 2] {
    let mut compiler = builder(backend.clone())
        .source_resolver(InMemorySourceResolver::new().with_string("ordered.mec", source))
        .build_compiler()
        .unwrap();
    let product = compiler
        .compile_canonical_roots(
            &[SourceRequest::new("ordered.mec")],
            ModuleBuildOptions::new(env!("CARGO_PKG_VERSION"), "v0.3", "native", &[], &[]),
        )
        .unwrap();
    assert_eq!(backend.trace.lock().unwrap().argument_reads, 0);
    assert!(backend.trace.lock().unwrap().output.is_empty());
    let original = product.artifact().clone();
    assert_eq!(
        original
            .nodes()
            .iter()
            .filter(|node| {
                node.as_operation()
                    .and_then(|operation| operation.requirement)
                    .and_then(|requirement| original.requirements().get(requirement))
                    .is_some_and(|requirement| {
                        matches!(requirement, mech_core::ApplicationRequirement::Resource(request)
            if request.intent == ResourceIntent::Send)
                    })
            })
            .count(),
        3,
        "duplicate equal sends must remain distinct effects"
    );
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    [original, decoded]
}

#[test]
fn equal_sends_to_one_address_keep_source_order_and_multiplicity() {
    let backend = Backend {
        argument: "C".into(),
        trace: Default::default(),
    };
    let source = "+> @args := cli/args\n+> @out := cli/stdout\n@out/text <- \"A\"\n@out/text <- \"A\"\n@out/text <- @args/1\n";
    for artifact in artifacts(source, backend.clone()) {
        *backend.trace.lock().unwrap() = Trace::default();
        let mut runtime = builder(backend.clone()).build().unwrap();
        runtime
            .load_compiled_program(artifact, ResidentDurabilityPolicy::Volatile)
            .unwrap();
        runtime.shutdown().unwrap();
        let trace = backend.trace.lock().unwrap();
        assert_eq!(trace.argument_reads, 1);
        assert_eq!(trace.output, ["A", "A", "C"]);
    }
}

#[test]
fn failed_value_computation_does_not_deliver_earlier_sends() {
    let backend = Backend {
        argument: "not-a-decimal".into(),
        trace: Default::default(),
    };
    let source = "+> number\n+> @args := cli/args\n+> @out := cli/stdout\n@out/text <- \"A\"\n@out/text <- \"A\"\n@out/text <- number/format(number/parse-u64(@args/1), 0u64)\n";
    for artifact in artifacts(source, backend.clone()) {
        *backend.trace.lock().unwrap() = Trace::default();
        let mut runtime = builder(backend.clone()).build().unwrap();
        assert!(
            runtime
                .load_compiled_program(artifact, ResidentDurabilityPolicy::Volatile)
                .is_err()
        );
        runtime.shutdown().unwrap();
        let trace = backend.trace.lock().unwrap();
        assert_eq!(
            trace.argument_reads, 1,
            "failure must follow the actual observation"
        );
        assert!(
            trace.output.is_empty(),
            "failed turns must publish no sends"
        );
    }
}
