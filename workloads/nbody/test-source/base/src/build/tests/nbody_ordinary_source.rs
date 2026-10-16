//! Full five-body ordinary-source qualification through activated CLI resources.
//! These are resident artifact checks, not Cranelift or performance evidence.
#![cfg(feature = "standard-hosts")]

use mech_core::snapshot::SequenceView;
use mech_core::{
    ApplicationRequirement, MResult, ParsedProgram, ResourceIntent, SchemaBody, ValueData,
};
use mech_engine::ProgramArtifact;
use mech_runtime::{
    ConfigValue, HostInstanceConfig, ModuleBuildOptions, ResidentDurabilityPolicy,
    RunResourceGrantConfig, RuntimeBuilder, RuntimeProgramRoute, SourceRequest,
    resolver::InMemorySourceResolver,
};
use mech_terminal::{ArgumentSnapshotCliBackend, CliBackend, CliHostFactory};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    sync::{Arc, Mutex},
};

const SOURCE: &str = include_str!("../../../hosts/gpu/tests/fixtures/native/nbody-idiomatic.mec");
const SOURCE_SHA256: &str = "10fc98c51465474b5b6ea2e6b0ae4826394863c1a5fa7d1a74616fb16cd7c750";

#[derive(Debug, Default)]
struct Trace {
    argument_reads: usize,
    environment_reads: usize,
    stdout: Vec<String>,
    stderr: Vec<String>,
}

#[derive(Clone, Debug)]
struct OutputBackend(Arc<Mutex<Trace>>);
impl CliBackend for OutputBackend {
    fn argument(&self, _: usize) -> MResult<Option<OsString>> {
        panic!("the maintained argument snapshot must not consult ambient argv")
    }
    fn env_var(&self, _: &str) -> MResult<Option<String>> {
        self.0.lock().unwrap().environment_reads += 1;
        Ok(None)
    }
    fn write_stdout(&mut self, text: &str) -> MResult<()> {
        self.0.lock().unwrap().stdout.push(text.into());
        Ok(())
    }
    fn write_stderr(&mut self, text: &str) -> MResult<()> {
        self.0.lock().unwrap().stderr.push(text.into());
        Ok(())
    }
}

#[derive(Clone, Debug)]
struct Backend {
    snapshot: ArgumentSnapshotCliBackend<OutputBackend>,
    trace: Arc<Mutex<Trace>>,
}
impl Backend {
    fn new(arguments: Vec<OsString>) -> Self {
        let trace = Arc::new(Mutex::new(Trace::default()));
        Self {
            snapshot: ArgumentSnapshotCliBackend::new(OutputBackend(trace.clone()), arguments),
            trace,
        }
    }
}
impl CliBackend for Backend {
    fn argument(&self, index: usize) -> MResult<Option<OsString>> {
        self.trace.lock().unwrap().argument_reads += 1;
        self.snapshot.argument(index)
    }
    fn env_var(&self, name: &str) -> MResult<Option<String>> {
        self.snapshot.env_var(name)
    }
    fn write_stdout(&mut self, text: &str) -> MResult<()> {
        self.snapshot.write_stdout(text)
    }
    fn write_stderr(&mut self, text: &str) -> MResult<()> {
        self.snapshot.write_stderr(text)
    }
}

// This is the same maintained factory/grant route as canonical_resource_execution.
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

fn artifacts(interactive: bool) -> [ProgramArtifact; 2] {
    assert_eq!(SOURCE.lines().count(), 77, "qualify the complete source");
    assert_eq!(
        format!("{:x}", Sha256::digest(SOURCE.as_bytes())),
        SOURCE_SHA256
    );
    let backend = Backend::new(vec!["0".into()]);
    let mut compiler = builder(backend.clone())
        .source_resolver(InMemorySourceResolver::new().with_string("nbody.mec", SOURCE))
        .build_compiler()
        .unwrap();
    let options = ModuleBuildOptions::new(env!("CARGO_PKG_VERSION"), "v0.3", "native", &[], &[]);
    let product = if interactive {
        // Reflect the unchanged document's initializers; do not append probe code
        // or substitute a short arithmetic program for the full workload.
        compiler.compile_canonical_interactive_root_with_options(
            SourceRequest::new("nbody.mec"),
            options,
        )
    } else {
        compiler.compile_canonical_roots(&[SourceRequest::new("nbody.mec")], options)
    }
    .unwrap();
    {
        let trace = backend.trace.lock().unwrap();
        assert_eq!(trace.argument_reads, 0, "compilation must not observe argv");
        assert_eq!(trace.environment_reads, 0);
        assert!(trace.stdout.is_empty());
        assert!(trace.stderr.is_empty());
    }
    let parsed = ParsedProgram::from_bytes(product.bytecode()).unwrap();
    assert!(
        parsed.instructions.is_empty(),
        "no legacy instruction program"
    );
    assert!(!parsed.artifact.is_empty());
    for (context, path, intent) in [
        ("args", "1", ResourceIntent::Read),
        ("stdout", "text", ResourceIntent::Send),
    ] {
        assert!(parsed.requirements.iter().any(|requirement| matches!(
            requirement, ApplicationRequirement::Resource(request)
                if request.base_uri == format!("cli://cli/{context}")
                    && request.context_name == context && request.path == path
                    && request.intent == intent
        )));
    }
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
                        matches!(requirement,
                ApplicationRequirement::Resource(request) if request.intent == ResourceIntent::Send)
                    })
            })
            .count(),
        2,
        "both source sends must survive as distinct effects"
    );
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    [original, decoded]
}

fn assert_output(trace: &Trace, expected: &str) {
    assert_eq!(trace.argument_reads, 1);
    assert_eq!(trace.environment_reads, 0);
    assert!(trace.stderr.is_empty());
    let lines = expected
        .split_inclusive('\n')
        .map(str::to_owned)
        .collect::<Vec<_>>();
    assert_eq!(
        trace.stdout, lines,
        "initial and final energy sends in order"
    );
    assert_eq!(trace.stdout.concat().as_bytes(), expected.as_bytes());
}

fn exact_case(argument: &str, expected: &str) {
    for (encoding, artifact) in artifacts(false).into_iter().enumerate() {
        let backend = Backend::new(vec![argument.into()]);
        let mut runtime = builder(backend.clone()).build().unwrap();
        let outcome = runtime
            .load_compiled_program(artifact, ResidentDurabilityPolicy::Volatile)
            .unwrap_or_else(|error| panic!("n={argument}, encoding={encoding}: {error:?}"));
        assert_eq!(outcome.route, RuntimeProgramRoute::ResidentExternal);
        runtime.shutdown().unwrap();
        assert_output(&backend.trace.lock().unwrap(), expected);
    }
}

#[test]
fn full_source_n_0_exact_output_original_and_decoded() {
    exact_case("0", include_str!("fixtures/nbody-source/n-0.stdout"));
}
#[test]
fn full_source_n_1_exact_output_original_and_decoded() {
    exact_case("1", include_str!("fixtures/nbody-source/n-1.stdout"));
}
#[test]
fn full_source_n_2_exact_output_original_and_decoded() {
    exact_case("2", include_str!("fixtures/nbody-source/n-2.stdout"));
}
#[test]
fn full_source_n_10_exact_output_original_and_decoded() {
    exact_case("10", include_str!("fixtures/nbody-source/n-10.stdout"));
}
#[test]
fn full_source_n_1000_exact_output_original_and_decoded() {
    exact_case("1000", include_str!("fixtures/nbody-source/n-1000.stdout"));
}
#[test]
fn full_source_n_10000_exact_output_original_and_decoded() {
    exact_case(
        "10000",
        include_str!("fixtures/nbody-source/n-10000.stdout"),
    );
}

// Exact retained generated.c state, column-major: 15 positions then 15
// velocities after the momentum offset. These are copied reference constants,
// not a second n-body implementation or a runtime oracle calculation.
const RETAINED_INITIAL: [f64; 30] = [
    0.00000000000000000e0,
    4.84143144246472090e0,
    8.34336671824457987e0,
    1.28943695621391310e1,
    1.53796971148509165e1,
    0.00000000000000000e0,
    -1.16032004402742839e0,
    4.12479856412430479e0,
    -1.51111514016986312e1,
    -2.59193146099879641e1,
    0.00000000000000000e0,
    -1.03622044471123109e-1,
    -4.03523417114321381e-1,
    -2.23307578892655734e-1,
    1.79258772950371181e-1,
    -3.87663407198742666e-4,
    6.06326392995832020e-1,
    -1.01077434617879236e0,
    1.08279100644153536e0,
    9.79090732243897977e-1,
    -3.27535903717657067e-3,
    2.81198684491626016e0,
    1.82566237123041186e0,
    8.68713018169608220e-1,
    5.94698998647676169e-1,
    2.39357340800029987e-5,
    -2.52183616598876288e-2,
    8.41576137658415352e-3,
    -1.08326374013636359e-2,
    -3.47559555040781037e-2,
];

fn assert_f64_matrix(value: &mech_core::Value, dimensions: [u64; 2], expected: &[f64]) {
    let owner = value.schemas().unwrap();
    let SchemaBody::Matrix {
        dimensions: axes, ..
    } = owner.get(value.schema()).unwrap().body()
    else {
        panic!("expected a canonical matrix")
    };
    assert_eq!(
        axes.iter()
            .map(|axis| value.shape().resolve_dimension(axis).unwrap())
            .collect::<Vec<_>>(),
        dimensions
    );
    let ValueData::Matrix(matrix) = value.data() else {
        panic!("expected matrix data")
    };
    let SequenceView::F64(values) = matrix.elements() else {
        panic!("expected exact F64 lanes")
    };
    assert_eq!(
        values.iter().map(|value| value.bits()).collect::<Vec<_>>(),
        expected
            .iter()
            .map(|value| value.to_bits())
            .collect::<Vec<_>>()
    );
}

#[test]
fn unchanged_full_source_initializers_match_all_retained_lanes_and_masses() {
    for artifact in artifacts(true) {
        let backend = Backend::new(vec!["0".into()]);
        let mut runtime = builder(backend.clone()).build().unwrap();
        runtime
            .load_compiled_program(artifact, ResidentDurabilityPolicy::Volatile)
            .unwrap();
        // Canonical matrix snapshots expose row-major elements. Map the retained
        // column-major state explicitly, without transposing the source program.
        for (name, offset) in [
            ("initial-x", 0),
            ("initial-v", 15),
            ("final-x", 0),
            ("final-v", 15),
        ] {
            let expected = (0..5)
                .flat_map(|row| {
                    (0..3).map(move |column| RETAINED_INITIAL[offset + row + column * 5])
                })
                .collect::<Vec<_>>();
            assert_f64_matrix(
                &runtime.root_symbol_value(name).unwrap().to_value(),
                [5, 3],
                &expected,
            );
        }
        let solar_mass = 4.0f64 * 3.141592653589793f64 * 3.141592653589793f64;
        let expected_masses = [
            1.0,
            0.000954791938424326609,
            0.000285885980666130812,
            0.0000436624404335156298,
            0.0000515138902046611451,
        ]
        .map(|ratio| ratio * solar_mass);
        assert_f64_matrix(
            &runtime.root_symbol_value("masses").unwrap().to_value(),
            [5, 1],
            &expected_masses,
        );
        runtime.shutdown().unwrap();
        assert_output(
            &backend.trace.lock().unwrap(),
            include_str!("fixtures/nbody-source/n-0.stdout"),
        );
    }
}

fn reject_arguments(arguments: Vec<OsString>) {
    for artifact in artifacts(false) {
        let backend = Backend::new(arguments.clone());
        let mut runtime = builder(backend.clone()).build().unwrap();
        let result = runtime.load_compiled_program(artifact, ResidentDurabilityPolicy::Volatile);
        assert!(result.is_err(), "invalid payload must fail: {arguments:?}");
        runtime.shutdown().unwrap();
        let trace = backend.trace.lock().unwrap();
        assert_eq!(
            trace.argument_reads, 1,
            "failure follows the real observation"
        );
        assert!(
            trace.stdout.is_empty(),
            "no initial energy from a failed execution"
        );
        assert!(trace.stderr.is_empty());
    }
}

#[test]
fn missing_step_argument_is_rejected_without_program_output() {
    reject_arguments(vec![]);
}
#[test]
fn invalid_decimal_and_u64_overflow_are_rejected_atomically() {
    for text in [
        "",
        "-1",
        "+1",
        " 1",
        "1 ",
        "1.0",
        "1x",
        "é",
        "18446744073709551616",
    ] {
        reject_arguments(vec![text.into()]);
    }
    exact_case("000", include_str!("fixtures/nbody-source/n-0.stdout"));
}

// Required historical launcher contract, deliberately separate from numerical
// qualification. The present source requests only @args/1 and has no count
// observation; this test exposes that unresolved gap instead of crediting an
// ignored extra argument as strict arity compliance. Do not filter it from a
// reported complete qualification.
#[test]
fn required_single_payload_arity_rejects_extra_arguments() {
    for artifact in artifacts(false) {
        let backend = Backend::new(vec!["0".into(), "extra".into()]);
        let mut runtime = builder(backend.clone()).build().unwrap();
        assert!(
            runtime
                .load_compiled_program(artifact, ResidentDurabilityPolicy::Volatile)
                .is_err(),
            "the historical launcher requires exactly one payload argument"
        );
        runtime.shutdown().unwrap();
        let trace = backend.trace.lock().unwrap();
        assert!(trace.stdout.is_empty());
        assert!(trace.stderr.is_empty());
        // A future genuine generic arity boundary may reject before observing
        // argument1. Do not prescribe observations for that unimplemented API.
    }
}

#[cfg(unix)]
#[test]
fn invalid_os_argument_bytes_fail_at_provider_without_output() {
    use std::os::unix::ffi::OsStringExt;
    reject_arguments(vec![OsString::from_vec(vec![0xff])]);
}

#[cfg(windows)]
#[test]
fn invalid_os_argument_units_fail_at_provider_without_output() {
    use std::os::windows::ffi::OsStringExt;
    reject_arguments(vec![OsString::from_wide(&[0xd800])]);
}
