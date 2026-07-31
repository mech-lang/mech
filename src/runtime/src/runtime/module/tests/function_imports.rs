use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use mech_core::{MechSourceCode, Ref, Value, hash_str};

use crate::runtime::test_support::capabilities::{CapabilityUseProbe, grant_host_call};
use crate::{
    CapabilityId, InMemorySourceResolver, MechRuntime, PlannedPureHostFunction, ResolvedSource,
    RuntimeBuilder, RuntimeCallContext, RuntimeEventKind, RuntimeValueSnapshot,
    SharedCapabilityKernel, SourceImportAlias, SourceImportDeclaration, SourceImportKind,
    SourceKind,
};

use super::support::{runtime_with_sources, test_module_options};

static NEXT_HOST_CAPABILITY_ID: AtomicU64 = AtomicU64::new(40_000);

#[derive(Clone)]
struct HostCounters {
    plans: Arc<AtomicUsize>,
    invocations: Arc<AtomicUsize>,
}

struct HostHarness {
    runtime: MechRuntime,
    counters: HostCounters,
    capability_uses: CapabilityUseProbe,
}

fn snapshot(value: Value) -> RuntimeValueSnapshot {
    RuntimeValueSnapshot::try_capture(&value).expect("acyclic fixture")
}

fn resolver_with_sources(sources: &[(&str, &str)]) -> InMemorySourceResolver {
    let mut resolver = InMemorySourceResolver::new();
    for (specifier, source) in sources {
        resolver.insert_string(*specifier, *source).unwrap();
    }
    resolver
}

fn runtime_with_colliding_host(
    resolver: InMemorySourceResolver,
    host_name: &str,
    sentinel: f64,
) -> HostHarness {
    let plans = Arc::new(AtomicUsize::new(0));
    let invocations = Arc::new(AtomicUsize::new(0));
    let plans_for_host = plans.clone();
    let invocations_for_host = invocations.clone();
    let kernel = SharedCapabilityKernel::new();
    let observed_kernel = kernel.clone();
    let mut runtime = RuntimeBuilder::new()
        .source_resolver(resolver)
        .capability_kernel(kernel)
        .host_function(PlannedPureHostFunction::new(
            host_name,
            move |_context: &RuntimeCallContext, _args: &[RuntimeValueSnapshot]| {
                plans_for_host.fetch_add(1, Ordering::SeqCst);
                Ok(snapshot(Value::F64(Ref::new(sentinel))))
            },
            move |_context: &RuntimeCallContext, _args: Vec<RuntimeValueSnapshot>| {
                invocations_for_host.fetch_add(1, Ordering::SeqCst);
                Ok(snapshot(Value::F64(Ref::new(sentinel))))
            },
        ))
        .unwrap()
        .build()
        .unwrap();
    let capability_id = CapabilityId(
        NEXT_HOST_CAPABILITY_ID
            .fetch_add(1, Ordering::Relaxed)
            .into(),
    );
    grant_host_call(&mut runtime, capability_id, host_name);
    HostHarness {
        runtime,
        counters: HostCounters { plans, invocations },
        capability_uses: CapabilityUseProbe::new(observed_kernel, capability_id),
    }
}

fn snapshot_f64(snapshot: RuntimeValueSnapshot) -> f64 {
    match snapshot.into_value() {
        Value::F64(value) => *value.borrow(),
        Value::MutableReference(value) => match &*value.borrow() {
            Value::F64(value) => *value.borrow(),
            other => panic!("expected f64 mutable reference, got {other:?}"),
        },
        other => panic!("expected f64 result, got {other:?}"),
    }
}

fn assert_missing_function(error: &mech_core::MechError) {
    assert_eq!(error.kind_name(), "MissingFunction", "{error:?}");
    let rendered = format!("{error:?}");
    assert!(!rendered.contains("Capability"), "{rendered}");
    assert!(!rendered.contains("HostFunction"), "{rendered}");
}

fn assert_no_host_activity(harness: &HostHarness) {
    assert_eq!(harness.counters.plans.load(Ordering::SeqCst), 0);
    assert_eq!(harness.counters.invocations.load(Ordering::SeqCst), 0);
    assert_eq!(harness.capability_uses.committed_uses(), 0);
    assert!(
        harness
            .runtime
            .list_events(None)
            .unwrap()
            .iter()
            .all(|event| !matches!(
                event.kind,
                RuntimeEventKind::HostCallStarted { .. }
                    | RuntimeEventKind::HostCallCompleted { .. }
                    | RuntimeEventKind::HostCallFailed { .. }
                    | RuntimeEventKind::EffectStaged { .. }
                    | RuntimeEventKind::EffectDelivered { .. }
                    | RuntimeEventKind::EffectDeliveryFailed { .. }
                    | RuntimeEventKind::TransactionalEffectCommitted { .. }
            ))
    );
    assert!(!harness.runtime.is_poisoned());
}

#[test]
fn failed_retained_root_does_not_leak_function_import_alias() {
    let mut runtime = runtime_with_sources(&[
        (
            "root-a.mec",
            "+> leaked-sin := math/sin\nroot-a-value := leaked-sin(0)\nroot-a-failure := missing\nroot-a-failure\n",
        ),
        (
            "root-b.mec",
            "root-b-value := leaked-sin(0)\nroot-b-value\n",
        ),
    ]);
    runtime.run_string("baseline := 7").unwrap();

    let first_error = runtime
        .resolve_and_run_root_module("root-a.mec", test_module_options())
        .unwrap_err();
    assert_eq!(first_error.kind_name(), "UndefinedVariable");

    let second_error = runtime
        .resolve_and_run_root_module("root-b.mec", test_module_options())
        .unwrap_err();
    assert_missing_function(&second_error);
    assert!(runtime.root_symbol_value("baseline").is_ok());
    assert!(runtime.root_symbol_value("root-a-value").is_err());
    assert!(runtime.root_symbol_value("root-a-failure").is_err());
    assert!(
        runtime
            .store
            .find_module_by_name("memory:root-a.mec")
            .unwrap()
            .is_none()
    );
    assert!(
        runtime
            .store
            .find_module_by_name("memory:root-b.mec")
            .unwrap()
            .is_none()
    );
    assert!(!runtime.is_poisoned());
}

#[test]
fn failed_retained_root_does_not_leak_function_import_glob() {
    let mut runtime = runtime_with_sources(&[
        (
            "root-a.mec",
            "+> math/*\nroot-a-value := sin(0)\nroot-a-failure := missing\nroot-a-failure\n",
        ),
        ("root-b.mec", "root-b-value := sin(0)\nroot-b-value\n"),
    ]);
    runtime.run_string("baseline := 7").unwrap();

    let first_error = runtime
        .resolve_and_run_root_module("root-a.mec", test_module_options())
        .unwrap_err();
    assert_eq!(first_error.kind_name(), "UndefinedVariable");

    let second_error = runtime
        .resolve_and_run_root_module("root-b.mec", test_module_options())
        .unwrap_err();
    assert_missing_function(&second_error);
    assert!(runtime.root_symbol_value("baseline").is_ok());
    assert!(runtime.root_symbol_value("root-a-value").is_err());
    assert!(runtime.root_symbol_value("root-a-failure").is_err());
    assert!(
        runtime
            .store
            .find_module_by_name("memory:root-a.mec")
            .unwrap()
            .is_none()
    );
    assert!(
        runtime
            .store
            .find_module_by_name("memory:root-b.mec")
            .unwrap()
            .is_none()
    );
    assert!(!runtime.is_poisoned());
}

#[test]
fn failed_import_collision_restores_preexisting_host_compiler() {
    let resolver = resolver_with_sources(&[(
        "root-a.mec",
        "+> restore-sin := math/sin\nimported-value := restore-sin(0)\nroot-a-failure := missing\nroot-a-failure\n",
    )]);
    let mut harness = runtime_with_colliding_host(resolver, "restore-sin", 731.0);
    let baseline = harness
        .runtime
        .run_string("host-baseline := restore-sin()\nhost-baseline\n")
        .unwrap();
    assert_eq!(snapshot_f64(baseline), 731.0);
    let plans_before_failure = harness.counters.plans.load(Ordering::SeqCst);
    let invocations_before_failure = harness.counters.invocations.load(Ordering::SeqCst);
    let host_compiler_before = harness
        .runtime
        .program
        .interpreter()
        .functions()
        .borrow()
        .function_compilers
        .get(&hash_str("restore-sin"))
        .unwrap()
        .clone();

    let error = harness
        .runtime
        .resolve_and_run_root_module("root-a.mec", test_module_options())
        .unwrap_err();
    assert_eq!(error.kind_name(), "UndefinedVariable");
    assert_eq!(
        harness.counters.plans.load(Ordering::SeqCst),
        plans_before_failure,
    );
    assert_eq!(
        harness.counters.invocations.load(Ordering::SeqCst),
        invocations_before_failure,
    );
    let host_compiler_after = harness
        .runtime
        .program
        .interpreter()
        .functions()
        .borrow()
        .function_compilers
        .get(&hash_str("restore-sin"))
        .unwrap()
        .clone();
    assert!(Arc::ptr_eq(&host_compiler_before, &host_compiler_after,));

    let recovered = harness
        .runtime
        .run_string("recovered := restore-sin()\nrecovered\n")
        .unwrap();
    assert_eq!(snapshot_f64(recovered), 731.0);
    assert_eq!(
        harness.counters.plans.load(Ordering::SeqCst),
        plans_before_failure + 1,
    );
    assert_eq!(
        harness.counters.invocations.load(Ordering::SeqCst),
        invocations_before_failure + 1,
    );
    assert_eq!(
        harness.capability_uses.committed_uses(),
        invocations_before_failure as u64 + 1,
    );
    assert!(harness.runtime.root_symbol_value("host-baseline").is_ok());
    assert!(harness.runtime.root_symbol_value("imported-value").is_err());
    assert!(
        harness
            .runtime
            .store
            .find_module_by_name("memory:root-a.mec")
            .unwrap()
            .is_none()
    );
    assert!(!harness.runtime.is_poisoned());
}

#[test]
fn explicit_function_import_alias_overrides_same_named_host_in_isolated_module() {
    let resolver = resolver_with_sources(&[
        ("root.mec", "+> ./dep.mec\nresult := dep/value\nresult\n"),
        (
            "dep.mec",
            "+> isolated-sin := math/sin\nvalue := isolated-sin(0)\n<+ value\n",
        ),
    ]);
    let mut harness = runtime_with_colliding_host(resolver, "isolated-sin", 811.0);

    let result = harness
        .runtime
        .resolve_and_run_root_module("root.mec", test_module_options())
        .unwrap();

    assert_eq!(snapshot_f64(result), 0.0);
    assert_no_host_activity(&harness);
}

#[test]
fn explicit_function_import_alias_overrides_same_named_host_in_retained_root() {
    let resolver = resolver_with_sources(&[(
        "root.mec",
        "+> retained-sin := math/sin\nresult := retained-sin(0)\nresult\n",
    )]);
    let mut harness = runtime_with_colliding_host(resolver, "retained-sin", 821.0);

    let result = harness
        .runtime
        .resolve_and_run_root_module("root.mec", test_module_options())
        .unwrap();

    assert_eq!(snapshot_f64(result), 0.0);
    assert_no_host_activity(&harness);
}

#[test]
fn wildcard_function_import_overrides_same_named_host_in_isolated_module() {
    let resolver = resolver_with_sources(&[
        ("root.mec", "+> ./dep.mec\nresult := dep/value\nresult\n"),
        ("dep.mec", "+> math/*\nvalue := sin(0)\n<+ value\n"),
    ]);
    let mut harness = runtime_with_colliding_host(resolver, "sin", 831.0);

    let result = harness
        .runtime
        .resolve_and_run_root_module("root.mec", test_module_options())
        .unwrap();

    assert_eq!(snapshot_f64(result), 0.0);
    assert_no_host_activity(&harness);
}

#[test]
fn wildcard_function_import_overrides_same_named_host_in_retained_root() {
    let resolver = resolver_with_sources(&[("root.mec", "+> math/*\nresult := sin(0)\nresult\n")]);
    let mut harness = runtime_with_colliding_host(resolver, "sin", 841.0);

    let result = harness
        .runtime
        .resolve_and_run_root_module("root.mec", test_module_options())
        .unwrap();

    assert_eq!(snapshot_f64(result), 0.0);
    assert_no_host_activity(&harness);
}

#[test]
fn multi_source_module_does_not_reregister_hosts_after_imports() {
    let import = SourceImportDeclaration {
        specifier: "math/sin".to_string(),
        alias: Some(SourceImportAlias::Value("split-sin".to_string())),
        module: Some("math".to_string()),
        item: Some("sin".to_string()),
        kind: SourceImportKind::Single {
            name: "sin".to_string(),
        },
    };
    let source = ResolvedSource::new(
        "root.mec",
        "memory:root.mec",
        MechSourceCode::Program(vec![
            MechSourceCode::String("+> split-sin := math/sin\n".to_string()),
            MechSourceCode::String("result := split-sin(0)\nresult\n".to_string()),
        ]),
    )
    .with_kind(SourceKind::Mech)
    .with_imports(vec![import]);
    let mut resolver = InMemorySourceResolver::new();
    resolver.insert_source("root.mec", source).unwrap();
    let mut harness = runtime_with_colliding_host(resolver, "split-sin", 851.0);

    let result = harness
        .runtime
        .resolve_and_run_root_module("root.mec", test_module_options())
        .unwrap();

    assert_eq!(snapshot_f64(result), 0.0);
    assert_no_host_activity(&harness);
}
