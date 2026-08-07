use std::collections::HashSet;
use std::path::PathBuf;
#[cfg(feature = "build")]
use std::sync::{Arc, Mutex};

use mech_core::*;
#[cfg(feature = "test")]
use mech_engine::IntegrityConstraintReport;
#[cfg(feature = "build")]
use mech_runtime::{
    ActorBootstrapConfig, ActorHostPlanningState, HostInstanceConfig, PlannedPureHostFunction,
    RunResourceGrantConfig, RuntimeValueSnapshot,
};
use mech_runtime::{
    FileSourceResolver, MechRuntime, ModuleBuildOptions, RuntimeBuilder, RuntimeConfig,
    SourceRequest,
};

#[cfg(feature = "test")]
pub(crate) struct SourceModuleExecution {
    pub(crate) runtime: MechRuntime,
    pub(crate) integrity: IntegrityConstraintReport,
}

struct SourceModuleExecutionInternal {
    runtime: MechRuntime,
    #[cfg(feature = "test")]
    integrity: IntegrityConstraintReport,
}

#[derive(Debug, Clone)]
struct RuntimeStepLimitConversionError {
    value: usize,
    reason: String,
}

impl MechErrorKind for RuntimeStepLimitConversionError {
    fn name(&self) -> &str {
        "RuntimeStepLimitConversionError"
    }

    fn message(&self) -> String {
        format!(
            "unable to convert runtime step limit `{}` to u64: {}",
            self.value, self.reason
        )
    }
}

#[derive(Debug, Clone)]
struct SourceRootCanonicalizationError {
    path: PathBuf,
    reason: String,
}

impl MechErrorKind for SourceRootCanonicalizationError {
    fn name(&self) -> &str {
        "SourceRootCanonicalizationError"
    }

    fn message(&self) -> String {
        format!(
            "unable to canonicalize source root `{}`: {}",
            self.path.display(),
            self.reason
        )
    }
}

pub(crate) fn module_runtime_config(
    name: String,
    debug_enabled: bool,
    trace_enabled: bool,
    profile_enabled: bool,
    max_steps_per_turn: usize,
) -> MResult<RuntimeConfig> {
    let mut config = RuntimeConfig::default();
    config.name = name;
    config.diagnostics.debug_enabled = debug_enabled;
    config.diagnostics.trace_enabled = trace_enabled;
    config.diagnostics.profile_enabled = profile_enabled;
    config.limits.max_steps_per_turn =
        Some(u64::try_from(max_steps_per_turn).map_err(|error| {
            MechError::new(
                RuntimeStepLimitConversionError {
                    value: max_steps_per_turn,
                    reason: error.to_string(),
                },
                None,
            )
            .with_compiler_loc()
        })?);

    // Legacy `mech build` and `mech test` constrained interpreter rounds but
    // imposed no wall-clock deadline. These commands execute trusted local
    // project sources, so runtime-backed module resolution must preserve that
    // behavior rather than inheriting RuntimeConfig's one-second default.
    config.limits.max_turn_duration_ms = None;

    config.validate()?;
    Ok(config)
}

fn module_build_options() -> ModuleBuildOptions<'static> {
    ModuleBuildOptions::new(env!("CARGO_PKG_VERSION"), "v0.3", "native", &[], &[])
}

fn canonical_source_roots(roots: &[PathBuf]) -> MResult<Vec<PathBuf>> {
    roots
        .iter()
        .map(|root| {
            root.canonicalize().map_err(|error| {
                MechError::new(
                    SourceRootCanonicalizationError {
                        path: root.clone(),
                        reason: error.to_string(),
                    },
                    None,
                )
                .with_compiler_loc()
            })
        })
        .collect()
}

fn resolver_roots(canonical_roots: &[PathBuf]) -> MResult<Vec<PathBuf>> {
    let current_dir = std::env::current_dir()?.canonicalize()?;
    let mut roots = Vec::new();
    let mut seen = HashSet::new();

    let mut add_root = |root: PathBuf| {
        if seen.insert(root.clone()) {
            roots.push(root);
        }
    };

    add_root(current_dir);
    for source_root in canonical_roots {
        if let Some(parent) = source_root.parent() {
            add_root(parent.to_path_buf());
        }
    }

    Ok(roots)
}

pub(crate) fn execute_source_module_roots(
    config: RuntimeConfig,
    roots: &[PathBuf],
) -> MResult<MechRuntime> {
    execute_source_module_roots_internal(config, roots).map(|execution| execution.runtime)
}

/// Compile trusted local roots in plan mode for the build command. This shares
/// the resolver and module execution path with source commands, but installs
/// only effect-free planning hosts and never starts input drivers.
#[cfg(feature = "build")]
pub(crate) fn execute_planning_source_module_roots(
    config: RuntimeConfig,
    configured_hosts: &[HostInstanceConfig],
    run_grants: &[RunResourceGrantConfig],
    actor_bootstrap: Option<&ActorBootstrapConfig>,
    roots: &[PathBuf],
) -> MResult<MechRuntime> {
    let (builder, canonical_roots) = source_module_runtime_builder(config, roots)?;
    let mut builder = builder.planning();
    let providers = crate::cli::host_configuration::configured_provider_names(configured_hosts);
    for provider in &providers {
        builder = builder.host_factory(mech_build::standard_planning_host_factory(provider)?)?;
    }
    (builder, _) = crate::cli::host_configuration::materialize_host_configuration(
        builder,
        configured_hosts,
        run_grants,
        &providers,
    )?;
    builder = install_actor_planning_functions(builder, actor_bootstrap.cloned())?;

    let mut runtime = builder.build()?;
    for root in canonical_roots {
        runtime.resolve_and_run_root_module(
            SourceRequest::from_filesystem_path(&root)?,
            module_build_options(),
        )?;
    }
    Ok(runtime)
}

#[cfg(feature = "build")]
fn actor_planning_call(
    state: &Mutex<Option<ActorHostPlanningState>>,
    name: &str,
    arguments: &[RuntimeValueSnapshot],
) -> MResult<RuntimeValueSnapshot> {
    let mut state = state.lock().map_err(|_| {
        mech_build::error::native_build_error(
            mech_build::error::NativeBuildErrorKind::NativeRuntimeConfigUnsupported {
                reason: "actor planning state lock is poisoned".to_owned(),
            },
            None,
        )
    })?;
    let state = state.as_mut().ok_or_else(|| {
        mech_build::error::native_build_error(
            mech_build::error::NativeBuildErrorKind::NativeActorBootstrapMissing,
            None,
        )
    })?;
    let values = arguments
        .iter()
        .map(RuntimeValueSnapshot::to_value)
        .collect::<Vec<_>>();
    RuntimeValueSnapshot::try_capture(&state.plan(name, &values)?)
}

#[cfg(feature = "build")]
fn install_actor_planning_functions(
    mut builder: RuntimeBuilder,
    bootstrap: Option<ActorBootstrapConfig>,
) -> MResult<RuntimeBuilder> {
    let state = Arc::new(Mutex::new(bootstrap.map(|bootstrap| {
        ActorHostPlanningState::new(
            &bootstrap.subject,
            bootstrap.message_kind,
            bootstrap.message_payload,
            bootstrap.initial_state,
        )
    })));

    let kind_state = Arc::clone(&state);
    builder = builder.host_function(PlannedPureHostFunction::new(
        "actor/message/kind",
        move |_context, arguments| {
            actor_planning_call(&kind_state, "actor/message/kind", arguments)
        },
        |_context, _arguments| panic!("actor/message/kind executed while source planning"),
    ))?;

    let payload_state = Arc::clone(&state);
    builder = builder.host_function(PlannedPureHostFunction::new(
        "actor/message/payload",
        move |_context, arguments| {
            actor_planning_call(&payload_state, "actor/message/payload", arguments)
        },
        |_context, _arguments| panic!("actor/message/payload executed while source planning"),
    ))?;

    let get_state = Arc::clone(&state);
    builder = builder.host_function(PlannedPureHostFunction::new(
        "actor/state/get",
        move |_context, arguments| actor_planning_call(&get_state, "actor/state/get", arguments),
        |_context, _arguments| panic!("actor/state/get executed while source planning"),
    ))?;

    let id_state = Arc::clone(&state);
    builder = builder.host_function(PlannedPureHostFunction::new(
        "actor/state/id",
        move |_context, arguments| actor_planning_call(&id_state, "actor/state/id", arguments),
        |_context, _arguments| panic!("actor/state/id executed while source planning"),
    ))?;

    let put_state = state;
    builder.host_function(PlannedPureHostFunction::new(
        "actor/state/put",
        move |_context, arguments| actor_planning_call(&put_state, "actor/state/put", arguments),
        |_context, _arguments| panic!("actor/state/put executed while source planning"),
    ))
}

#[cfg(feature = "test")]
pub(crate) fn execute_source_module_roots_with_report(
    config: RuntimeConfig,
    roots: &[PathBuf],
) -> MResult<SourceModuleExecution> {
    execute_source_module_roots_internal(config, roots).map(|execution| SourceModuleExecution {
        runtime: execution.runtime,
        integrity: execution.integrity,
    })
}

fn execute_source_module_roots_internal(
    config: RuntimeConfig,
    roots: &[PathBuf],
) -> MResult<SourceModuleExecutionInternal> {
    let (builder, canonical_roots) = source_module_runtime_builder(config, roots)?;
    let mut runtime = builder.build()?;
    #[cfg(feature = "test")]
    let mut integrity_evaluations = Vec::new();
    for root in canonical_roots {
        let request = SourceRequest::from_filesystem_path(&root)?;
        #[cfg(feature = "test")]
        {
            let report =
                runtime.resolve_and_run_root_module_report(request, module_build_options())?;
            integrity_evaluations.extend(report.integrity.evaluations);
        }
        #[cfg(not(feature = "test"))]
        runtime.resolve_and_run_root_module(request, module_build_options())?;
    }
    Ok(SourceModuleExecutionInternal {
        runtime,
        #[cfg(feature = "test")]
        integrity: IntegrityConstraintReport::from_evaluations(integrity_evaluations),
    })
}

fn source_module_runtime_builder(
    config: RuntimeConfig,
    roots: &[PathBuf],
) -> MResult<(RuntimeBuilder, Vec<PathBuf>)> {
    let canonical_roots = canonical_source_roots(roots)?;
    let mut resolver = FileSourceResolver::empty();
    for root in resolver_roots(&canonical_roots)? {
        resolver.add_root(root);
    }

    let builder = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .config(config)
        .source_resolver(resolver);
    Ok((builder, canonical_roots))
}

#[cfg(test)]
mod tests {
    use super::*;
    use mech_runtime::RuntimeValueSnapshot;

    fn temp_root(label: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "mech-module-execution-{label}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    fn config() -> RuntimeConfig {
        module_runtime_config(
            "module-execution-test".to_string(),
            false,
            false,
            false,
            10_000,
        )
        .unwrap()
    }

    #[test]
    fn module_runtime_config_preserves_unbounded_tool_duration() {
        let config = module_runtime_config(
            "tool-duration-test".to_string(),
            false,
            false,
            false,
            12_345,
        )
        .unwrap();

        assert_eq!(config.limits.max_steps_per_turn, Some(12_345));
        assert_eq!(config.limits.max_turn_duration_ms, None);
        assert_eq!(
            config.limits.max_source_bytes,
            RuntimeConfig::default().limits.max_source_bytes,
        );
    }

    fn assert_f64(value: RuntimeValueSnapshot, expected: f64) {
        match value.into_value() {
            Value::F64(value) => assert_eq!(*value.borrow(), expected),
            Value::MutableReference(value) => match &*value.borrow() {
                Value::F64(value) => assert_eq!(*value.borrow(), expected),
                other => panic!("expected f64 value, got {other:?}"),
            },
            other => panic!("expected f64 value, got {other:?}"),
        }
    }

    #[cfg(feature = "build")]
    fn assert_string(value: RuntimeValueSnapshot, expected: &str) {
        match value.into_value() {
            Value::String(value) => assert_eq!(value.borrow().as_str(), expected),
            Value::MutableReference(value) => match &*value.borrow() {
                Value::String(value) => assert_eq!(value.borrow().as_str(), expected),
                other => panic!("expected string value, got {other:?}"),
            },
            other => panic!("expected string value, got {other:?}"),
        }
    }

    #[cfg(feature = "build")]
    #[test]
    fn actor_source_planning_tracks_state_put_before_later_reads() {
        let root = temp_root("actor-state-sequence");
        let source = root.join("main.mec");
        std::fs::write(
            &source,
            "updated := actor/state/put(\"created\")\nstate := actor/state/get()\nidentifier := actor/state/id()\nidentifier\n",
        )
        .unwrap();
        let bootstrap = ActorBootstrapConfig {
            subject: "actor:planning".to_owned(),
            message_kind: "test".to_owned(),
            message_payload: String::new(),
            initial_state: None,
        };

        let runtime =
            execute_planning_source_module_roots(config(), &[], &[], Some(&bootstrap), &[source])
                .unwrap();

        assert_string(runtime.root_symbol_value("state").unwrap(), "created");
        match runtime
            .root_symbol_value("identifier")
            .unwrap()
            .into_value()
        {
            Value::String(value) => assert!(value.borrow().starts_with("planning-state-put-")),
            Value::MutableReference(value) => match &*value.borrow() {
                Value::String(value) => {
                    assert!(value.borrow().starts_with("planning-state-put-"))
                }
                other => panic!("expected string value, got {other:?}"),
            },
            other => panic!("expected string value, got {other:?}"),
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn resolves_sibling_relative_imports_from_absolute_roots() {
        let root = temp_root("sibling");
        let main = root.join("main.mec");
        std::fs::write(&main, "+> ./dep.mec\nanswer := dep/value + 1\nanswer\n").unwrap();
        std::fs::write(root.join("dep.mec"), "value := 41\n<+ value\n").unwrap();

        let runtime = execute_source_module_roots(config(), &[main]).unwrap();

        assert_f64(runtime.root_symbol_value("answer").unwrap(), 42.0);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn resolves_nested_relative_imports_from_importing_module() {
        let root = temp_root("nested");
        let main = root.join("main.mec");
        let lib = root.join("lib");
        std::fs::create_dir_all(&lib).unwrap();
        std::fs::write(
            &main,
            "+> ./lib/first.mec\nanswer := first/value + 1\nanswer\n",
        )
        .unwrap();
        std::fs::write(
            lib.join("first.mec"),
            "+> ./second.mec\nvalue := second/value + 1\n<+ value\n",
        )
        .unwrap();
        std::fs::write(lib.join("second.mec"), "value := 40\n<+ value\n").unwrap();

        let runtime = execute_source_module_roots(config(), &[main]).unwrap();

        assert_f64(runtime.root_symbol_value("answer").unwrap(), 42.0);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn missing_dependencies_preserve_runtime_dependency_errors() {
        let root = temp_root("missing");
        let main = root.join("main.mec");
        std::fs::write(&main, "+> ./missing.mec\nanswer := 1\n").unwrap();

        let error = execute_source_module_roots(config(), &[main]).unwrap_err();

        assert!(
            error
                .kind_as::<mech_runtime::RuntimeModuleDependencyMissingError>()
                .is_some()
        );
        let chain = error.full_chain_message();
        assert!(chain.contains("missing.mec"));
        assert!(chain.contains("main.mec"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn multiple_roots_execute_in_input_order() {
        let root = temp_root("multiple-roots");
        let first_dir = root.join("first");
        let second_dir = root.join("second");
        std::fs::create_dir_all(&first_dir).unwrap();
        std::fs::create_dir_all(&second_dir).unwrap();
        let first = first_dir.join("first.mec");
        let second = second_dir.join("second.mec");
        std::fs::write(&first, "marker := 1\n").unwrap();
        std::fs::write(&second, "answer := marker + 1\n").unwrap();

        let runtime = execute_source_module_roots(config(), &[first, second]).unwrap();

        assert_f64(runtime.root_symbol_value("answer").unwrap(), 2.0);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn canonical_source_roots_preserves_input_order() {
        let root = temp_root("canonical-order");
        let first = root.join("first.mec");
        let second = root.join("second.mec");
        std::fs::write(&first, "first := 1\n").unwrap();
        std::fs::write(&second, "second := 2\n").unwrap();

        let roots = canonical_source_roots(&[second.clone(), first.clone()]).unwrap();

        assert_eq!(
            roots,
            vec![
                second.canonicalize().unwrap(),
                first.canonicalize().unwrap()
            ]
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn canonical_source_roots_reports_original_path_and_filesystem_error() {
        let root = temp_root("missing-root");
        let missing = root.join("missing.mec");

        let error = canonical_source_roots(&[missing.clone()]).unwrap_err();

        let message = error.full_chain_message();
        assert!(message.contains(&missing.display().to_string()));
        assert!(message.contains("canonicalize source root"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn build_module_execution_preserves_non_utf8_root_path() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        let root = temp_root("non-utf8-build");
        let source = root.join(OsString::from_vec(b"build-\xFF.mec".to_vec()));
        std::fs::write(&source, "answer := 42\nanswer\n").unwrap();

        let runtime = execute_source_module_roots(config(), &[source]).unwrap();

        assert_f64(runtime.root_symbol_value("answer").unwrap(), 42.0);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[cfg(all(target_os = "linux", feature = "test"))]
    #[test]
    fn test_module_execution_preserves_non_utf8_root_path() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        let root = temp_root("non-utf8-test");
        let source = root.join(OsString::from_vec(b"test-\xFF.mec".to_vec()));
        std::fs::write(
            &source,
            "answer := 42\n\nnon-utf8-root-pass! := answer == 42\n\nanswer\n",
        )
        .unwrap();

        let execution = execute_source_module_roots_with_report(config(), &[source]).unwrap();

        assert_eq!(execution.integrity.evaluations.len(), 1);
        let evaluation = &execution.integrity.evaluations[0];
        assert!(evaluation.passed);
        assert!(evaluation.name.contains("non-utf8-root-pass!"));
        assert_f64(execution.runtime.root_symbol_value("answer").unwrap(), 42.0);
        std::fs::remove_dir_all(root).unwrap();
    }
}
