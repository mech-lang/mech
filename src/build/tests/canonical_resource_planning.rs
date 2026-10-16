//! Resource planning validates ordinary source without evaluating placeholder observations.
#![cfg(feature = "standard-hosts")]

use std::{
    collections::BTreeMap,
    ffi::OsString,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use mech_build::{
    NativeApplicationBuilder, NativeBuildEnvironment, NativeBuildProfile, NativeBuildRequest,
    NativeDependencySource, NativeEmit, NativeRuntimeConfig, PlannedApplicationRequirement,
    selected_planning_host_factory, standard_native_host_catalog,
};
use mech_core::{ApplicationRequirement, MResult, ParsedProgram, ResourceIntent};
use mech_engine::ProgramCompilationProduct;
use mech_runtime::{
    ConfigValue, HostInstanceConfig, ModuleBuildOptions, ProgramCompiler, RunResourceGrantConfig,
    RuntimeBuilder, RuntimeConfig, SourceRequest, resolver::InMemorySourceResolver,
};
use mech_terminal::{CliBackend, CliHostFactory};

#[derive(Debug, Default)]
struct Observations {
    arguments: AtomicUsize,
    stdin: AtomicUsize,
    environment: AtomicUsize,
    output: AtomicUsize,
}

impl Observations {
    fn assert_empty(&self) {
        assert_eq!(self.arguments.load(Ordering::SeqCst), 0);
        assert_eq!(self.stdin.load(Ordering::SeqCst), 0);
        assert_eq!(self.environment.load(Ordering::SeqCst), 0);
        assert_eq!(self.output.load(Ordering::SeqCst), 0);
    }
}

#[derive(Clone, Debug)]
struct CountingCliBackend {
    observations: Arc<Observations>,
    argument: OsString,
}

impl CliBackend for CountingCliBackend {
    fn argument(&self, index: usize) -> MResult<Option<OsString>> {
        self.observations.arguments.fetch_add(1, Ordering::SeqCst);
        Ok((index == 1).then(|| self.argument.clone()))
    }

    fn stdin_text(&self) -> MResult<String> {
        self.observations.stdin.fetch_add(1, Ordering::SeqCst);
        Ok("not-a-decimal".into())
    }

    fn env_var(&self, _: &str) -> MResult<Option<String>> {
        self.observations.environment.fetch_add(1, Ordering::SeqCst);
        Ok(None)
    }

    fn write_stdout(&mut self, _: &str) -> MResult<()> {
        self.observations.output.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn write_stderr(&mut self, _: &str) -> MResult<()> {
        self.observations.output.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

fn host() -> HostInstanceConfig {
    HostInstanceConfig {
        name: "cli".into(),
        provider: "cli".into(),
        settings: ConfigValue::Map(BTreeMap::new()),
    }
}

fn grants() -> Vec<RunResourceGrantConfig> {
    vec![
        RunResourceGrantConfig {
            target: "cli/args".into(),
            operations: vec!["read".into()],
            paths: vec!["1".into()],
        },
        RunResourceGrantConfig {
            target: "cli/stdin".into(),
            operations: vec!["read".into()],
            paths: vec!["text".into()],
        },
        RunResourceGrantConfig {
            target: "cli/stdout".into(),
            operations: vec!["write".into()],
            paths: vec!["text".into()],
        },
    ]
}

fn compiler(source: &str, backend: CountingCliBackend) -> ProgramCompiler {
    let mut builder = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .source_resolver(InMemorySourceResolver::new().with_string("planning.mec", source))
        .host_factory(Box::new(CliHostFactory::with_backend(backend).unwrap()))
        .unwrap()
        .host_instance(host());
    for grant in grants() {
        builder = builder.run_resource_grant(grant);
    }
    builder.build_compiler().unwrap()
}

fn compile(compiler: &mut ProgramCompiler) -> MResult<ProgramCompilationProduct> {
    compiler.compile_canonical_roots(
        &[SourceRequest::new("planning.mec")],
        ModuleBuildOptions::new(env!("CARGO_PKG_VERSION"), "v0.3", "native", &[], &[]),
    )
}

fn assert_retained_read_and_send(product: &ProgramCompilationProduct, read_context: &str) {
    let parsed = ParsedProgram::from_bytes(product.bytecode()).unwrap();
    assert!(parsed.instructions.is_empty());
    assert!(!parsed.artifact.is_empty());
    assert_eq!(parsed.requirements.len(), 2);
    for (context, path, intent) in [
        (
            read_context,
            if read_context == "args" { "1" } else { "text" },
            ResourceIntent::Read,
        ),
        ("stdout", "text", ResourceIntent::Send),
    ] {
        assert!(parsed.requirements.iter().any(|requirement| {
            matches!(requirement, ApplicationRequirement::Resource(request)
                if request.base_uri == format!("cli://cli/{context}")
                    && request.context_name == context
                    && request.path == path
                    && request.intent == intent)
        }));
    }
    let references = product.artifact().operation_references();
    assert!(
        references
            .iter()
            .any(|operation| operation.canonical_name() == "number/parse-u64")
    );
    assert!(
        references
            .iter()
            .any(|operation| operation.canonical_name() == "number/format")
    );
}

fn assert_native_plan(product: &ProgramCompilationProduct, read_context: &str) {
    let request = NativeBuildRequest {
        bytecode: product.bytecode().to_vec(),
        instruction_type_bindings: None,
        instruction_type_binding_requirements: None,
        runtime_config: Some(NativeRuntimeConfig {
            runtime: RuntimeConfig::default(),
            hosts: vec![host()],
            run_grants: grants(),
            actor_bootstrap: None,
        }),
        target: None,
        profile: NativeBuildProfile::Debug,
        binary_name: "canonical-resource-planning".into(),
        output: PathBuf::from("ignored"),
        emit: NativeEmit::Plan,
        keep_project: false,
        offline: true,
    };
    let plan = NativeApplicationBuilder::new(NativeBuildEnvironment {
        function_catalog: mech_stdlib::source_native_plan_catalog(),
        host_catalog: standard_native_host_catalog().unwrap(),
        dependency_source: NativeDependencySource::Registry {
            version: mech_build::MECH_COMPONENT_VERSION.into(),
        },
    })
    .plan(&request)
    .expect("native planning must not compute a payload from fabricated observation text");
    assert!(
        plan.live,
        "driverless observations retain their live delivery contract"
    );
    assert_eq!(plan.hosts.len(), 1);
    assert_eq!(
        plan.hosts[0].factory_path,
        "mech_terminal::CliHostFactory::with_arguments"
    );
    assert_eq!(
        plan.hosts[0].factory_input,
        mech_build::NativeHostFactoryInput::ProgramArguments
    );
    assert!(plan.runtime_functions.is_empty());
    assert_eq!(plan.application_requirements.len(), 2);
    assert!(plan.application_requirements.iter().any(|requirement| {
        matches!(requirement, PlannedApplicationRequirement::Resource { request, .. }
            if request.context_name == read_context && request.intent == ResourceIntent::Read)
    }));
    assert!(
        plan.engine_features
            .iter()
            .any(|feature| feature == "pure_text")
    );
}

#[test]
fn canonical_args_parse_and_send_plan_without_observing_or_parsing_placeholder_text() {
    let source = "+> number\n+> @args := cli/args\n+> @out := cli/stdout\nn := number/parse-u64(@args/1)\n@out/text <- number/format(n, 0)\n";
    for argument in ["42", "not-a-decimal"] {
        let observations = Arc::new(Observations::default());
        let backend = CountingCliBackend {
            observations: observations.clone(),
            argument: argument.into(),
        };
        let result = compile(&mut compiler(source, backend));
        observations.assert_empty();
        let product = result.expect("source planning must retain an unknown String observation rather than evaluate parse-u64 on an empty placeholder");
        assert_retained_read_and_send(&product, "args");
        assert_native_plan(&product, "args");
        observations.assert_empty();
    }
}

#[test]
fn canonical_stdin_parse_and_send_plan_without_consuming_stdin() {
    let source = "+> number\n+> @in := cli/stdin\n+> @out := cli/stdout\nn := number/parse-u64(@in/text)\n@out/text <- number/format(n, 0)\n";
    let observations = Arc::new(Observations::default());
    let result = compile(&mut compiler(
        source,
        CountingCliBackend {
            observations: observations.clone(),
            argument: "42".into(),
        },
    ));
    observations.assert_empty();
    let product = result.expect("stdin contents must remain unknown until an authorized launch");
    assert_retained_read_and_send(&product, "stdin");
    assert_native_plan(&product, "stdin");
    observations.assert_empty();
}

#[test]
fn canonical_planning_still_rejects_invalid_resource_paths_directions_and_payload_schemas() {
    for source in [
        "+> @args := cli/args\nvalue := @args/0\nvalue\n",
        "+> @args := cli/args\nvalue := @args/01\nvalue\n",
        "+> @args := cli/args\nvalue := @args/text\nvalue\n",
        "+> @in := cli/stdin\nvalue := @in/line\nvalue\n",
        "+> @out := cli/stdout\n@out/unknown <- \"hello\"\n",
        "+> @out := cli/stdout\nvalue := @out/text\nvalue\n",
        "+> @args := cli/args\n@args/1 <- \"hello\"\n",
        "+> @out := cli/stdout\n@out/text <- 7u64\n",
        "+> @out := cli/stdout\n@out/text <- [\"hello\" \"world\"]\n",
    ] {
        let observations = Arc::new(Observations::default());
        let result = compile(&mut compiler(
            source,
            CountingCliBackend {
                observations: observations.clone(),
                argument: "42".into(),
            },
        ));
        observations.assert_empty();
        assert!(
            result.is_err(),
            "invalid resource source was admitted: {source}"
        );
    }
}

fn scene_host() -> HostInstanceConfig {
    HostInstanceConfig {
        name: "scene".into(),
        provider: "scene".into(),
        settings: ConfigValue::Map(BTreeMap::from([
            ("renderer".into(), ConfigValue::String("canvas".into())),
            ("selector".into(), ConfigValue::String("#scene".into())),
        ])),
    }
}

fn scene_grant() -> RunResourceGrantConfig {
    RunResourceGrantConfig {
        target: "scene/frame".into(),
        operations: vec!["write".into()],
        paths: vec!["replace".into(), "points".into()],
    }
}

fn scene_compiler() -> ProgramCompiler {
    RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .host_factory(selected_planning_host_factory("scene").unwrap())
        .unwrap()
        .host_instance(scene_host())
        .run_resource_grant(scene_grant())
        .build_compiler()
        .unwrap()
}

fn assert_scene_native_plan(product: &ProgramCompilationProduct, path: &str) {
    let request = NativeBuildRequest {
        bytecode: product.bytecode().to_vec(),
        instruction_type_bindings: None,
        instruction_type_binding_requirements: None,
        runtime_config: Some(NativeRuntimeConfig {
            runtime: RuntimeConfig::default(),
            hosts: vec![scene_host()],
            run_grants: vec![scene_grant()],
            actor_bootstrap: None,
        }),
        target: None,
        profile: NativeBuildProfile::Debug,
        binary_name: "canonical-scene-planning".into(),
        output: PathBuf::from("ignored"),
        emit: NativeEmit::Plan,
        keep_project: false,
        offline: true,
    };
    let plan = NativeApplicationBuilder::new(NativeBuildEnvironment {
        function_catalog: mech_stdlib::source_native_plan_catalog(),
        host_catalog: standard_native_host_catalog().unwrap(),
        dependency_source: NativeDependencySource::Registry {
            version: mech_build::MECH_COMPONENT_VERSION.into(),
        },
    })
    .plan(&request)
    .expect("Scene planning must validate structure without fabricating live field values");
    assert_eq!(plan.application_requirements.len(), 1);
    assert!(plan.application_requirements.iter().any(|requirement| {
        matches!(requirement, PlannedApplicationRequirement::Resource { request, .. }
            if request.base_uri == "scene://scene/frame"
                && request.context_name == "frame"
                && request.path == path
                && request.intent == ResourceIntent::Send)
    }));
}

#[test]
fn scene_planning_preserves_derived_closed_and_data_dependent_payloads() {
    for (path, payload, open) in [
        (
            "replace",
            "{width: 640.0, height: 480.0, background: \"black\"}",
            false,
        ),
        (
            "replace",
            "{width: width<f64>, height: 480.0, background: \"black\"}",
            true,
        ),
        ("points", "points<[f64]:3,2>", true),
    ] {
        let source = format!(
            "@frame := scene://scene/frame{{:write({path})}}\npayload := {payload}\n@frame/{path} <- payload\n"
        );
        let product = scene_compiler()
            .compile_canonical_source(&source)
            .unwrap_or_else(|error| panic!("{source}\n{error:?}"));
        assert_eq!(!product.artifact().inputs().is_empty(), open, "{source}");
        assert_scene_native_plan(&product, path);
    }
}

#[test]
fn scene_planning_keeps_static_schema_and_closed_value_rejections() {
    for (path, payload) in [
        (
            "replace",
            "{width: 0.0, height: 480.0, background: \"black\"}",
        ),
        (
            "replace",
            "{width: 640.0, height: -1.0, background: \"black\"}",
        ),
        (
            "replace",
            "{width: width<string>, height: 480.0, background: \"black\"}",
        ),
        ("points", "points<[f64]:3,3>"),
        ("points", "points<[f64]:0,2>"),
    ] {
        let source = format!(
            "@frame := scene://scene/frame{{:write({path})}}\npayload := {payload}\n@frame/{path} <- payload\n"
        );
        assert!(
            scene_compiler().compile_canonical_source(&source).is_err(),
            "invalid Scene payload was admitted: {source}"
        );
    }
}
