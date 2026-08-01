use crate::{
    InMemorySourceResolver, MechRuntime, ModuleRecord, ModuleVersionRecord,
    ResourceBudgetExceededError, RuntimeConfig, RuntimeEventKind,
    RuntimeModuleDependencyCycleError, RuntimeModuleDependencyMissingError, module_id,
};

use super::support::{runtime_with_sources, test_module_options};

#[test]
fn missing_later_dependency_commits_no_graph() {
    let mut runtime = runtime_with_sources(&[
        (
            "main.mec",
            "+> ./first.mec\n+> ./missing.mec\nanswer := 1\n",
        ),
        ("first.mec", "value := 1\n<+ value\n"),
    ]);

    let error = runtime
        .resolve_and_store_module_source("main.mec", test_module_options())
        .unwrap_err();

    assert!(
        error
            .kind_as::<RuntimeModuleDependencyMissingError>()
            .is_some(),
        "expected missing dependency, got {error:?}",
    );
    for uri in ["memory:main.mec", "memory:first.mec"] {
        assert!(
            runtime.store.find_module_by_name(uri).unwrap().is_none(),
            "failed graph exposed {uri}",
        );
    }
    assert!(
        runtime
            .list_events(None)
            .unwrap()
            .iter()
            .all(|event| !matches!(event.kind, RuntimeEventKind::ModuleCompiled { .. })),
    );
}

#[test]
fn deep_parse_failure_commits_no_graph() {
    let mut runtime = runtime_with_sources(&[
        ("main.mec", "+> ./middle.mec\nanswer := 1\n"),
        ("middle.mec", "+> ./leaf.mec\nvalue := 1\n"),
        ("leaf.mec", "value := [1, 2\n"),
    ]);

    assert!(
        runtime
            .resolve_and_store_module_source("main.mec", test_module_options(),)
            .is_err(),
    );

    for uri in ["memory:main.mec", "memory:middle.mec", "memory:leaf.mec"] {
        assert!(
            runtime.store.find_module_by_name(uri).unwrap().is_none(),
            "failed graph exposed {uri}",
        );
    }
}

#[test]
fn cycle_error_retains_deterministic_path() {
    let mut runtime = runtime_with_sources(&[
        ("main.mec", "+> ./middle.mec\nanswer := 1\n"),
        ("middle.mec", "+> ./leaf.mec\nvalue := 1\n"),
        ("leaf.mec", "+> ./main.mec\nvalue := 2\n"),
    ]);

    let error = runtime
        .resolve_and_store_module_source("main.mec", test_module_options())
        .unwrap_err();
    let cycle = error
        .kind_as::<RuntimeModuleDependencyCycleError>()
        .expect("existing cycle error type");

    assert_eq!(
        cycle.cycle,
        vec![
            "memory:main.mec".to_string(),
            "memory:middle.mec".to_string(),
            "memory:leaf.mec".to_string(),
            "memory:main.mec".to_string(),
        ],
    );
}

#[test]
fn retained_root_missing_later_dependency_has_no_partial_graph_or_version_audit() {
    let mut runtime = runtime_with_sources(&[
        (
            "root.mec",
            "+> ./first.mec\n+> ./missing.mec\nanswer := 1\n",
        ),
        ("first.mec", "value := 1\n<+ value\n"),
    ]);

    let error = runtime
        .resolve_and_run_root_module("root.mec", test_module_options())
        .unwrap_err();

    assert!(
        error
            .kind_as::<RuntimeModuleDependencyMissingError>()
            .is_some(),
    );
    for uri in ["memory:root.mec", "memory:first.mec"] {
        assert!(
            runtime.store.find_module_by_name(uri).unwrap().is_none(),
            "missing dependency exposed {uri}",
        );
    }
    let events = runtime.list_events(None).unwrap();
    assert!(
        events
            .iter()
            .any(|event| matches!(event.kind, RuntimeEventKind::ProgramFailed { .. }))
    );
    assert!(
        events
            .iter()
            .all(|event| !matches!(event.kind, RuntimeEventKind::ModuleExecutionFailed { .. }))
    );
}

#[test]
fn valid_dependency_then_parse_failure_leaves_no_graph() {
    let mut runtime = runtime_with_sources(&[
        ("root.mec", "+> ./valid.mec\n+> ./broken.mec\nanswer := 1\n"),
        ("valid.mec", "value := 1\n<+ value\n"),
        ("broken.mec", "value := [1, 2\n"),
    ]);

    assert!(
        runtime
            .resolve_and_store_module_source("root.mec", test_module_options(),)
            .is_err(),
    );

    for uri in ["memory:root.mec", "memory:valid.mec", "memory:broken.mec"] {
        assert!(
            runtime.store.find_module_by_name(uri).unwrap().is_none(),
            "parse failure exposed {uri}",
        );
    }
}

#[test]
fn source_budget_failure_after_dependency_staging_leaves_no_graph() {
    let mut config = RuntimeConfig::default();
    config.limits.max_source_bytes = Some(50);
    let mut resolver = InMemorySourceResolver::new();
    resolver
        .insert_string("root.mec", "+> ./first.mec\n+> ./large.mec\nx := 1\n")
        .unwrap();
    resolver
        .insert_string("first.mec", "x := 1\n<+ x\n")
        .unwrap();
    resolver
        .insert_string(
            "large.mec",
            "this_source_is_deliberately_larger_than_the_configured_fifty_byte_limit := 1\n",
        )
        .unwrap();
    let mut runtime = MechRuntime::new(config).unwrap();
    runtime.set_source_resolver(resolver).unwrap();

    let error = runtime
        .resolve_and_store_module_source("root.mec", test_module_options())
        .unwrap_err();

    assert!(error.kind_as::<ResourceBudgetExceededError>().is_some(),);
    for uri in ["memory:root.mec", "memory:first.mec", "memory:large.mec"] {
        assert!(
            runtime.store.find_module_by_name(uri).unwrap().is_none(),
            "budget failure exposed {uri}",
        );
    }
}

#[test]
fn diamond_graph_reuses_one_shared_version() {
    let mut runtime = runtime_with_sources(&[
        ("root.mec", "+> ./left.mec\n+> ./right.mec\nanswer := 1\n"),
        (
            "left.mec",
            "+> ./shared.mec\nleft := shared/value\n<+ left\n",
        ),
        (
            "right.mec",
            "+> ./shared.mec\nright := shared/value\n<+ right\n",
        ),
        ("shared.mec", "value := 1\n<+ value\n"),
    ]);

    let root = runtime
        .resolve_and_store_module_source("root.mec", test_module_options())
        .unwrap()
        .unwrap();
    let root = runtime.store.get_module_version(root).unwrap().unwrap();
    assert_eq!(root.dependencies.len(), 2);
    let left = runtime
        .store
        .get_module_version(root.dependencies[0])
        .unwrap()
        .unwrap();
    let right = runtime
        .store
        .get_module_version(root.dependencies[1])
        .unwrap()
        .unwrap();

    assert_eq!(left.dependencies.len(), 1);
    assert_eq!(right.dependencies.len(), 1);
    assert_eq!(left.dependencies[0], right.dependencies[0]);
    assert!(
        runtime
            .store
            .find_module_by_name("memory:shared.mec")
            .unwrap()
            .is_some(),
    );
}

#[test]
fn deterministic_version_conflict_fails_atomically() {
    let mut runtime = runtime_with_sources(&[("root.mec", "answer := 42\nanswer\n")]);
    let version = runtime
        .resolve_and_store_module_source("root.mec", test_module_options())
        .unwrap()
        .unwrap();
    let committed = runtime.store.get_module_version(version).unwrap().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    let unrelated = ModuleRecord::new(module_id("memory:unrelated.mec"), "memory:unrelated.mec");
    runtime
        .active_execution_transaction_mut(transaction_id)
        .unwrap()
        .modules
        .stage_module(unrelated.clone())
        .unwrap();
    runtime
        .active_execution_transaction_mut(transaction_id)
        .unwrap()
        .modules
        .stage_version(ModuleVersionRecord::new(
            version,
            committed.module,
            committed.version.saturating_add(1),
        ))
        .unwrap();

    let error = runtime
        .commit_runtime_transaction_detailed(&mut context)
        .unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeModuleJournalConflict");
    assert_eq!(context.transaction, Some(transaction_id));
    assert!(runtime.store.get_module(unrelated.id).unwrap().is_none(),);
    assert_eq!(
        runtime.store.get_module_version(version).unwrap(),
        Some(committed),
    );
    runtime
        .abort_runtime_transaction(&mut context, "discard conflict")
        .unwrap();
}
