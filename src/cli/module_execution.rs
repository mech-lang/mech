use std::collections::HashSet;
use std::path::PathBuf;

use mech_core::*;
use mech_runtime::{
    FileSourceResolver, MechRuntime, ModuleBuildOptions, RuntimeBuilder, RuntimeConfig,
    SourceRequest,
};
#[cfg(feature = "test")]
use mech_program::IntegrityConstraintReport;

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
    execute_source_module_roots_internal(config, roots)
        .map(|execution| execution.runtime)
}

#[cfg(feature = "test")]
pub(crate) fn execute_source_module_roots_with_report(
    config: RuntimeConfig,
    roots: &[PathBuf],
) -> MResult<SourceModuleExecution> {
    execute_source_module_roots_internal(config, roots).map(|execution| {
        SourceModuleExecution {
            runtime: execution.runtime,
            integrity: execution.integrity,
        }
    })
}

fn execute_source_module_roots_internal(
    config: RuntimeConfig,
    roots: &[PathBuf],
) -> MResult<SourceModuleExecutionInternal> {
    let canonical_roots = canonical_source_roots(roots)?;
    let mut resolver = FileSourceResolver::empty();
    for root in resolver_roots(&canonical_roots)? {
        resolver.add_root(root);
    }

    let mut runtime = RuntimeBuilder::new()
        .config(config)
        .source_resolver(resolver)
        .build()?;
    #[cfg(feature = "test")]
    let mut integrity_evaluations = Vec::new();
    for root in canonical_roots {
        #[cfg(feature = "test")]
        {
            let report = runtime.resolve_and_run_root_module_report(
                SourceRequest::new(root.to_string_lossy().to_string()),
                module_build_options(),
            )?;
            integrity_evaluations.extend(report.integrity.evaluations);
        }
        #[cfg(not(feature = "test"))]
        runtime.resolve_and_run_root_module(
            SourceRequest::new(root.to_string_lossy().to_string()),
            module_build_options(),
        )?;
    }
    Ok(SourceModuleExecutionInternal {
        runtime,
        #[cfg(feature = "test")]
        integrity: IntegrityConstraintReport::from_evaluations(
            integrity_evaluations,
        ),
    })
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
}
