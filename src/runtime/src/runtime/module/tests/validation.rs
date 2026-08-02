use mech_core::{MResult, MechSourceCode};

use crate::{
    MechRuntime, ModuleBuildOptions, NonExecutableModuleSource, ResolvedSource,
    ResourceBudgetExceededError, RuntimeConfig, RuntimeEventKind, SourceKind, SourceRequest,
    SourceResolver,
};

use super::support::{runtime_with_sources, test_module_options};

#[derive(Debug)]
struct PanickingSourceResolver;

impl SourceResolver for PanickingSourceResolver {
    fn resolve(&self, _request: &SourceRequest) -> MResult<Option<ResolvedSource>> {
        panic!("deliberate source resolver panic");
    }
}

#[test]
fn max_source_bytes_rejects_module_source() {
    let mut config = RuntimeConfig::default();
    config.limits.max_source_bytes = Some(3);
    let mut runtime = MechRuntime::new(config).unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let canonical_uri = "memory://big-module.mec";
    let resolved = ResolvedSource::new(
        "big-module",
        canonical_uri,
        MechSourceCode::String("1234".to_string()),
    )
    .with_kind(SourceKind::Mech);

    let error = runtime
        .build_module_from_resolved_source_with_context(
            &mut context,
            resolved,
            ModuleBuildOptions::new("test", "v0.3", "native", &[], &[]),
        )
        .unwrap_err();
    let budget = error.kind_as::<ResourceBudgetExceededError>().unwrap();
    assert_eq!(budget.resource, "source_bytes");
    assert_eq!(budget.requested, 4);
    assert_eq!(budget.max, Some(3));
    assert!(
        runtime
            .store
            .find_module_by_name(canonical_uri)
            .unwrap()
            .is_none()
    );
    assert!(
        runtime
            .list_events(None)
            .unwrap()
            .iter()
            .all(|event| !matches!(event.kind, RuntimeEventKind::ModuleCompiled { .. }))
    );
}

#[test]
fn non_executable_module_source_is_rejected_before_indexing() {
    let mut runtime = MechRuntime::new(RuntimeConfig::default()).unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let resolved = ResolvedSource::new(
        "style.css",
        "memory://style.css",
        MechSourceCode::String("not valid Mech source".to_string()),
    )
    .with_kind(SourceKind::Css);

    let error = runtime
        .build_module_from_resolved_source_with_context(
            &mut context,
            resolved,
            ModuleBuildOptions::new("test", "v0.3", "native", &[], &[]),
        )
        .unwrap_err();

    assert!(error.kind_as::<NonExecutableModuleSource>().is_some());
}

#[test]
fn retained_root_integrity_failure_exposes_no_graph_or_completion() {
    let mut runtime = runtime_with_sources(&[(
        "root.mec",
        "integrity-root-value := 2.0\nintegrity-root-safe! := false\nintegrity-root-value\n",
    )]);
    runtime.run_string("integrity-baseline := 7").unwrap();
    let events_before = runtime.list_events(None).unwrap().len();

    let error = runtime
        .resolve_and_run_root_module("root.mec", test_module_options())
        .unwrap_err();

    assert_eq!(error.kind_name(), "IntegrityConstraintViolationSet");
    assert!(
        runtime
            .program
            .root_symbol_value("integrity-baseline")
            .is_ok()
    );
    assert!(
        runtime
            .program
            .root_symbol_value("integrity-root-value")
            .is_err()
    );
    assert!(
        runtime
            .store
            .find_module_by_name("memory:root.mec")
            .unwrap()
            .is_none()
    );
    let events = runtime.list_events(None).unwrap();
    let operation_events = &events[events_before..];
    assert!(
        operation_events
            .iter()
            .any(|event| matches!(event.kind, RuntimeEventKind::ProgramFailed { .. }))
    );
    assert!(operation_events.iter().any(|event| matches!(
        event.kind,
        RuntimeEventKind::IntegrityConstraintViolated { .. }
    )));
    assert!(operation_events.iter().all(|event| !matches!(
        event.kind,
        RuntimeEventKind::ProgramCompleted { .. }
            | RuntimeEventKind::ModuleExecutionCompleted { .. }
    )));
    assert!(!runtime.is_poisoned());
}

#[test]
fn isolated_dependency_integrity_failure_prevents_root_materialization() {
    let mut runtime = runtime_with_sources(&[
        (
            "root.mec",
            "+> ./dep.mec\nintegrity-root-ran := dep/value\nintegrity-root-ran\n",
        ),
        ("dep.mec", "value := 2.0\ndep-safe! := false\n<+ value\n"),
    ]);
    let events_before = runtime.list_events(None).unwrap().len();

    let error = runtime
        .resolve_and_run_root_module("root.mec", test_module_options())
        .unwrap_err();

    assert_eq!(error.kind_name(), "IntegrityConstraintViolationSet");
    assert!(
        runtime
            .program
            .root_symbol_value("integrity-root-ran")
            .is_err()
    );
    for uri in ["memory:root.mec", "memory:dep.mec"] {
        assert!(
            runtime.store.find_module_by_name(uri).unwrap().is_none(),
            "invalid dependency exposed {uri}",
        );
    }
    let events = runtime.list_events(None).unwrap();
    let operation_events = &events[events_before..];
    assert!(
        operation_events
            .iter()
            .any(|event| matches!(event.kind, RuntimeEventKind::ProgramFailed { .. }))
    );
    assert!(operation_events.iter().any(|event| matches!(
        event.kind,
        RuntimeEventKind::IntegrityConstraintViolated { .. }
    )));
    assert!(operation_events.iter().all(|event| !matches!(
        event.kind,
        RuntimeEventKind::ProgramCompleted { .. }
            | RuntimeEventKind::ModuleExecutionCompleted { .. }
    )));
    assert!(!runtime.is_poisoned());
}

#[test]
fn source_resolver_panic_is_converted_without_poisoning() {
    let runtime = MechRuntime::builder()
        .source_resolver(PanickingSourceResolver)
        .build()
        .unwrap();

    let error = runtime.resolve_source("panic.mec").unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeExtensionPanicked");
    assert!(format!("{error:?}").contains("deliberate source resolver panic"));
    assert!(!runtime.is_poisoned());
    runtime.list_events(None).unwrap();
}
