use std::collections::HashSet;
use std::path::PathBuf;
#[cfg(feature = "build")]
use std::sync::{Arc, Mutex};

use mech_core::*;
#[cfg(feature = "build")]
use mech_runtime::{
    ActorBootstrapConfig, ActorHostPlanningState, HostInstanceConfig, PlannedPureHostFunction,
    ProgramCompiler, RunResourceGrantConfig, RuntimeValueSnapshot,
};
use mech_runtime::{FileSourceResolver, RuntimeBuilder, RuntimeConfig};

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

    // Build planning compiles trusted local project sources and intentionally
    // has no wall-clock deadline. Preserve that tool behavior rather than
    // inheriting RuntimeConfig's one-second turn default.
    config.limits.max_turn_duration_ms = None;

    config.validate()?;
    Ok(config)
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

/// Construct the compiler owner for trusted local build roots. It shares the
/// resolver and provider-planning environment with runtime source loading but
/// never constructs a live runtime or attaches input drivers.
#[cfg(feature = "build")]
pub(crate) fn prepare_source_program_compiler(
    config: RuntimeConfig,
    configured_hosts: &[HostInstanceConfig],
    run_grants: &[RunResourceGrantConfig],
    actor_bootstrap: Option<&ActorBootstrapConfig>,
    roots: &[PathBuf],
) -> MResult<(ProgramCompiler, Vec<PathBuf>)> {
    let (builder, canonical_roots) = source_module_runtime_builder(config, roots)?;
    let mut builder = builder;
    let providers = crate::cli::host_configuration::configured_provider_names(configured_hosts);
    for provider in &providers {
        builder = builder.host_factory(mech_build::selected_planning_host_factory(provider)?)?;
    }
    (builder, _) = crate::cli::host_configuration::materialize_host_configuration(
        builder,
        configured_hosts,
        run_grants,
        &providers,
    )?;
    builder = install_actor_planning_functions(builder, actor_bootstrap.cloned())?;

    Ok((builder.build_compiler()?, canonical_roots))
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
}
