use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use mech_core::Value;

use crate::runtime::test_support::capabilities::grant_host_call;
use crate::{
    module_id, CapabilityId, ObjectId, ObjectRecord, PlannedStagedHostFunction,
    PreparedRuntimeEffect, RuntimeEventKind, RuntimePreparedHostCall,
};

use super::support::{
    counting_after_commit_effect, runtime_builder_with_sources, runtime_with_sources,
    staged_test_capability, test_module_options,
};

#[test]
fn explicit_abort_discards_provisional_graph() {
    let mut runtime = runtime_with_sources(&[("main.mec", "answer := 42\nanswer\n")]);
    let mut context = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut context).unwrap();
    let version = runtime
        .build_module_from_request_with_context(&mut context, "main.mec", test_module_options())
        .unwrap()
        .unwrap();

    runtime
        .abort_runtime_transaction(&mut context, "discard graph")
        .unwrap();

    assert!(runtime
        .store
        .get_module(module_id("memory:main.mec"))
        .unwrap()
        .is_none(),);
    assert!(runtime.store.get_module_version(version).unwrap().is_none(),);
}

#[test]
fn retained_root_failure_rolls_back_graph_events_and_program() {
    let mut runtime = runtime_with_sources(&[("root.mec", "answer := missing\nanswer\n")]);
    runtime.run_string("baseline := 7").unwrap();

    let error = runtime
        .resolve_and_run_root_module("root.mec", test_module_options())
        .unwrap_err();

    assert!(format!("{error:?}").contains("missing"));
    assert!(runtime
        .store
        .find_module_by_name("memory:root.mec")
        .unwrap()
        .is_none(),);
    assert!(runtime.root_symbol_value("baseline").is_ok());
    assert!(runtime.root_symbol_value("answer").is_err());
    let events = runtime.list_events(None).unwrap();
    assert!(events.iter().all(|event| !matches!(
        event.kind,
        RuntimeEventKind::SourceResolved { .. } | RuntimeEventKind::ModuleCompiled { .. }
    )));
    assert!(events
        .iter()
        .any(|event| matches!(event.kind, RuntimeEventKind::ProgramFailed { .. })));
    assert!(events
        .iter()
        .any(|event| matches!(event.kind, RuntimeEventKind::ModuleExecutionFailed { .. })));
    assert!(!runtime.is_poisoned());
}

#[test]
fn retained_root_dependency_execution_failure_commits_no_graph() {
    let mut runtime = runtime_with_sources(&[
        ("root.mec", "+> ./dep.mec\nanswer := dep/value\nanswer\n"),
        ("dep.mec", "value := missing\n<+ value\n"),
    ]);

    assert!(runtime
        .resolve_and_run_root_module("root.mec", test_module_options(),)
        .is_err(),);

    for uri in ["memory:root.mec", "memory:dep.mec"] {
        assert!(
            runtime.store.find_module_by_name(uri).unwrap().is_none(),
            "dependency failure exposed {uri}",
        );
    }
}

#[test]
fn failed_root_does_not_deliver_dependency_after_commit_effect() {
    let deliveries = Arc::new(AtomicUsize::new(0));
    let deliveries_for_host = deliveries.clone();
    let mut runtime = runtime_builder_with_sources(&[
        ("root.mec", "+> ./dep.mec\nanswer := missing\nanswer\n"),
        ("dep.mec", "value := dependency/after_commit()\n<+ value\n"),
    ])
    .host_function(PlannedStagedHostFunction::new(
        "dependency/after_commit",
        |_context, _args| Ok(Value::F64(mech_core::Ref::new(1.0)).into()),
        move |_context, _args| {
            Ok(RuntimePreparedHostCall {
                value: Value::F64(mech_core::Ref::new(1.0)).into(),
                effect: PreparedRuntimeEffect::AfterCommit(Box::new(counting_after_commit_effect(
                    deliveries_for_host.clone(),
                ))),
            })
        },
    ))
    .unwrap()
    .build()
    .unwrap();
    grant_host_call(&mut runtime, CapabilityId(910), "dependency/after_commit");

    assert!(runtime
        .resolve_and_run_root_module("root.mec", test_module_options(),)
        .is_err(),);

    assert_eq!(deliveries.load(Ordering::SeqCst), 0);
    assert!(runtime
        .store
        .find_module_by_name("memory:dep.mec")
        .unwrap()
        .is_none(),);
}

#[test]
fn explicit_retained_root_is_provisional_and_abort_restores_baseline() {
    let mut runtime = runtime_with_sources(&[("root.mec", "answer := 42\nanswer\n")]);
    runtime.run_string("baseline := 7").unwrap();
    let mut owner = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut owner).unwrap();

    runtime
        .resolve_and_run_root_module_with_context(&mut owner, "root.mec", test_module_options())
        .unwrap();

    assert!(runtime.root_symbol_value("answer").is_ok());
    assert!(runtime
        .store
        .find_module_by_name("memory:root.mec")
        .unwrap()
        .is_none(),);
    runtime
        .abort_runtime_transaction(&mut owner, "discard provisional retained root")
        .unwrap();
    assert!(runtime.root_symbol_value("baseline").is_ok());
    assert!(runtime.root_symbol_value("answer").is_err());
    assert!(runtime
        .store
        .find_module_by_name("memory:root.mec")
        .unwrap()
        .is_none(),);
}

#[test]
fn outer_abort_discards_graph_object_capability_effect_and_program() {
    let mut runtime = runtime_with_sources(&[("root.mec", "answer := 42\nanswer\n")]);
    runtime.run_string("baseline := 7").unwrap();
    let mut context = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut context).unwrap();
    runtime
        .resolve_and_run_root_module_with_context(&mut context, "root.mec", test_module_options())
        .unwrap();
    let object = ObjectRecord::text(ObjectId(921), "module-transaction", "discarded");
    runtime
        .put_object_with_context(&mut context, object.clone())
        .unwrap();
    let (capability, request) = staged_test_capability(&runtime, CapabilityId(921));
    runtime
        .grant_capability_with_context(&mut context, capability)
        .unwrap();
    let deliveries = Arc::new(AtomicUsize::new(0));
    runtime
        .stage_runtime_effect_with_context(
            &mut context,
            PreparedRuntimeEffect::AfterCommit(Box::new(counting_after_commit_effect(
                deliveries.clone(),
            ))),
        )
        .unwrap();

    runtime
        .abort_runtime_transaction(&mut context, "discard all")
        .unwrap();

    assert!(runtime
        .store
        .find_module_by_name("memory:root.mec")
        .unwrap()
        .is_none(),);
    assert!(runtime.get_object(object.id).unwrap().is_none());
    assert!(runtime.check_capability(&request).is_err());
    assert_eq!(deliveries.load(Ordering::SeqCst), 0);
    assert!(runtime.root_symbol_value("baseline").is_ok());
    assert!(runtime.root_symbol_value("answer").is_err());
}
