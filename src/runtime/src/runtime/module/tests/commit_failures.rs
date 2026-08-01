use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use mech_core::{MResult, MechError, Value};

use crate::runtime::test_support::capabilities::grant_host_call;
use crate::runtime::test_support::ids::ScriptedEventIdGenerator;
use crate::runtime::test_support::providers::TestAfterCommitEffect;
use crate::{
    Capability, CapabilityId, EventId, InMemorySourceResolver, MechRuntime, ObjectId, ObjectRecord,
    PlannedRuntimeManagedHostFunction, PreparedRuntimeEffect, RuntimeBuilder,
    RuntimeEffectMetadata, RuntimeEffectSource, RuntimeEventKind, RuntimeInvalidOperationError,
    RuntimeTransactionalEffect, RuntimeValueSnapshot,
};

use super::support::{
    CountingSourceResolver, counting_after_commit_effect, runtime_builder_with_sources,
    runtime_with_sources, staged_test_capability, test_module_options,
};

fn snapshot(value: Value) -> RuntimeValueSnapshot {
    RuntimeValueSnapshot::try_capture(&value).expect("acyclic fixture")
}

#[derive(Debug)]
struct FailingCommitEffect {
    aborts: Arc<AtomicUsize>,
}

impl RuntimeTransactionalEffect for FailingCommitEffect {
    fn metadata(&self) -> RuntimeEffectMetadata {
        RuntimeEffectMetadata::new(
            RuntimeEffectSource::Custom {
                name: "failing-participant-commit".to_string(),
            },
            "commit",
        )
    }

    fn prepare(&mut self) -> MResult<()> {
        Ok(())
    }

    fn commit(&mut self) -> MResult<()> {
        Err(MechError::new(
            RuntimeInvalidOperationError {
                operation: "post-store-participant-commit",
                reason: "deliberate post-store failure".to_string(),
            },
            None,
        ))
    }

    fn abort(&mut self) -> MResult<()> {
        self.aborts.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

fn failing_after_commit_effect(deliveries: Arc<AtomicUsize>) -> TestAfterCommitEffect {
    TestAfterCommitEffect::new(
        RuntimeEffectMetadata::new(
            RuntimeEffectSource::Custom {
                name: "after-commit-delivery".to_string(),
            },
            "deliver",
        ),
        move || {
            deliveries.fetch_add(1, Ordering::SeqCst);
            Err(MechError::new(
                RuntimeInvalidOperationError {
                    operation: "deliver_after_commit_effect",
                    reason: "deliberate delivery failure".to_string(),
                },
                None,
            ))
        },
    )
}

#[test]
fn retryable_explicit_store_failure_keeps_graph_without_rebuilding() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut inner = InMemorySourceResolver::new();
    inner
        .insert_string("root.mec", "answer := 42\nanswer\n")
        .unwrap();
    let resolver = CountingSourceResolver {
        inner,
        calls: calls.clone(),
    };
    let mut runtime = RuntimeBuilder::new()
        .source_resolver(resolver)
        .build()
        .unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    runtime
        .resolve_and_run_root_module_with_context(&mut context, "root.mec", test_module_options())
        .unwrap();
    let calls_after_build = calls.load(Ordering::SeqCst);
    runtime
        .update_object_with_context(
            &mut context,
            ObjectRecord::text(ObjectId(911), "missing", "provisional update"),
        )
        .unwrap();

    assert!(
        runtime
            .commit_runtime_transaction_detailed(&mut context)
            .is_err(),
    );
    assert_eq!(context.transaction, Some(transaction_id));
    assert!(
        runtime
            .store
            .find_module_by_name("memory:root.mec")
            .unwrap()
            .is_none(),
    );
    assert!(runtime.root_symbol_value("answer").is_ok());
    assert_eq!(calls.load(Ordering::SeqCst), calls_after_build);

    runtime
        .store
        .put_object(ObjectRecord::text(ObjectId(911), "missing", "baseline"))
        .unwrap();
    runtime
        .commit_runtime_transaction_detailed(&mut context)
        .unwrap();

    assert_eq!(calls.load(Ordering::SeqCst), calls_after_build);
    assert!(
        runtime
            .store
            .find_module_by_name("memory:root.mec")
            .unwrap()
            .is_some(),
    );
    assert!(runtime.root_symbol_value("answer").is_ok());
}

#[test]
fn implicit_store_failure_rolls_back_root_and_stays_healthy() {
    let mut runtime = runtime_builder_with_sources(&[(
        "root.mec",
        "answer := module/stage_invalid_store_update()\nanswer\n",
    )])
    .host_function(PlannedRuntimeManagedHostFunction::new(
        "module/stage_invalid_store_update",
        |_context, _args| Ok(snapshot(Value::F64(mech_core::Ref::new(42.0)))),
        move |services, _context, _args| {
            services.update_object(ObjectRecord::text(
                ObjectId(912),
                "missing",
                "invalid update",
            ))?;
            Ok(snapshot(Value::F64(mech_core::Ref::new(42.0))))
        },
    ))
    .unwrap()
    .build()
    .unwrap();
    runtime.run_string("baseline := 7").unwrap();
    grant_host_call(
        &mut runtime,
        CapabilityId(912),
        "module/stage_invalid_store_update",
    );

    assert!(
        runtime
            .resolve_and_run_root_module("root.mec", test_module_options(),)
            .is_err(),
    );

    assert!(!runtime.is_poisoned());
    assert!(runtime.active_transactions.is_empty());
    assert!(runtime.root_symbol_value("baseline").is_ok());
    assert!(runtime.root_symbol_value("answer").is_err());
    assert!(
        runtime
            .store
            .find_module_by_name("memory:root.mec")
            .unwrap()
            .is_none(),
    );
}

#[test]
fn post_store_participant_failure_keeps_graph_and_program() {
    let mut runtime = runtime_with_sources(&[("root.mec", "answer := 42\nanswer\n")]);
    let mut context = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut context).unwrap();
    runtime
        .resolve_and_run_root_module_with_context(&mut context, "root.mec", test_module_options())
        .unwrap();
    let aborts = Arc::new(AtomicUsize::new(0));
    runtime
        .stage_runtime_effect_with_context(
            &mut context,
            PreparedRuntimeEffect::Transactional(Box::new(FailingCommitEffect {
                aborts: aborts.clone(),
            })),
        )
        .unwrap();

    let error = runtime
        .commit_runtime_transaction_detailed(&mut context)
        .unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeExternalCommitIndeterminate",);
    assert!(runtime.is_poisoned());
    assert_eq!(context.transaction, None);
    assert_eq!(aborts.load(Ordering::SeqCst), 0);
    assert!(
        runtime
            .store
            .find_module_by_name("memory:root.mec")
            .unwrap()
            .is_some(),
    );
    assert!(runtime.root_symbol_value("answer").is_ok());
}

#[test]
fn after_commit_failure_keeps_graph_program_and_runtime_healthy() {
    let mut runtime = runtime_with_sources(&[("root.mec", "answer := 42\nanswer\n")]);
    let mut context = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut context).unwrap();
    runtime
        .resolve_and_run_root_module_with_context(&mut context, "root.mec", test_module_options())
        .unwrap();
    let deliveries = Arc::new(AtomicUsize::new(0));
    runtime
        .stage_runtime_effect_with_context(
            &mut context,
            PreparedRuntimeEffect::AfterCommit(Box::new(failing_after_commit_effect(
                deliveries.clone(),
            ))),
        )
        .unwrap();

    let outcome = runtime
        .commit_runtime_transaction_detailed(&mut context)
        .unwrap();

    assert_eq!(outcome.delivery_failures.len(), 1);
    assert_eq!(deliveries.load(Ordering::SeqCst), 1);
    assert!(!runtime.is_poisoned());
    assert!(
        runtime
            .store
            .find_module_by_name("memory:root.mec")
            .unwrap()
            .is_some(),
    );
    assert!(runtime.root_symbol_value("answer").is_ok());
}

#[test]
fn module_event_staging_failure_removes_journal_and_context_suffix() {
    let mut resolver = InMemorySourceResolver::new();
    resolver
        .insert_string("root.mec", "+> ./dep.mec\nanswer := dep/value\n")
        .unwrap();
    resolver
        .insert_string("dep.mec", "value := 1\n<+ value\n")
        .unwrap();
    let mut runtime = MechRuntime::builder()
        .id_generator(ScriptedEventIdGenerator::new(
            1,
            [
                EventId(100),
                EventId(101),
                EventId(102),
                EventId(102),
                EventId(103),
                EventId(104),
            ],
        ))
        .source_resolver(resolver)
        .build()
        .unwrap();
    let mut context = runtime.runtime_context().unwrap();

    let error = runtime
        .build_module_from_request_with_context(&mut context, "root.mec", test_module_options())
        .unwrap_err();

    assert_eq!(error.kind_name(), "InvalidRuntimeTransaction");
    assert!(runtime.active_transactions.is_empty());
    assert_eq!(context.transaction, None);
    assert!(context.events.iter().all(|event| !matches!(
        event.kind,
        RuntimeEventKind::SourceResolved { .. } | RuntimeEventKind::ModuleCompiled { .. }
    )));
    for uri in ["memory:root.mec", "memory:dep.mec"] {
        assert!(runtime.store.find_module_by_name(uri).unwrap().is_none(),);
    }
}

#[test]
fn store_failure_exposes_none_of_the_staged_categories() {
    let mut runtime = runtime_with_sources(&[("root.mec", "answer := 42\nanswer\n")]);
    runtime.run_string("baseline := 7").unwrap();
    let mut context = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut context).unwrap();
    runtime
        .resolve_and_run_root_module_with_context(&mut context, "root.mec", test_module_options())
        .unwrap();
    let object = ObjectRecord::text(ObjectId(922), "module-transaction", "provisional");
    runtime
        .put_object_with_context(&mut context, object.clone())
        .unwrap();
    let (capability, request) = staged_test_capability(&runtime, CapabilityId(922));
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
        .update_object_with_context(
            &mut context,
            ObjectRecord::text(ObjectId(9_922), "missing", "invalid update"),
        )
        .unwrap();

    assert!(
        runtime
            .commit_runtime_transaction_detailed(&mut context)
            .is_err(),
    );

    assert!(
        runtime
            .store
            .find_module_by_name("memory:root.mec")
            .unwrap()
            .is_none(),
    );
    assert!(runtime.get_object(object.id).unwrap().is_none());
    assert!(runtime.check_capability(&request).is_err());
    assert_eq!(deliveries.load(Ordering::SeqCst), 0);
    runtime
        .abort_runtime_transaction(&mut context, "store failure")
        .unwrap();
    assert!(runtime.root_symbol_value("baseline").is_ok());
    assert!(runtime.root_symbol_value("answer").is_err());
}

#[test]
fn post_store_failure_preserves_every_durable_category() {
    let mut runtime = runtime_with_sources(&[("root.mec", "answer := 42\nanswer\n")]);
    let mut context = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut context).unwrap();
    runtime
        .resolve_and_run_root_module_with_context(&mut context, "root.mec", test_module_options())
        .unwrap();
    let object = ObjectRecord::text(ObjectId(923), "module-transaction", "durable");
    runtime
        .put_object_with_context(&mut context, object.clone())
        .unwrap();
    let (capability, _request) = staged_test_capability(&runtime, CapabilityId(923));
    runtime
        .grant_capability_with_context(&mut context, capability.clone())
        .unwrap();
    let aborts = Arc::new(AtomicUsize::new(0));
    runtime
        .stage_runtime_effect_with_context(
            &mut context,
            PreparedRuntimeEffect::Transactional(Box::new(FailingCommitEffect {
                aborts: aborts.clone(),
            })),
        )
        .unwrap();

    let error = runtime
        .commit_runtime_transaction_detailed(&mut context)
        .unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeExternalCommitIndeterminate",);
    assert!(
        runtime
            .store
            .find_module_by_name("memory:root.mec")
            .unwrap()
            .is_some(),
    );
    assert_eq!(runtime.get_object(object.id).unwrap(), Some(object));
    assert!(runtime.get_capability(capability.id()).unwrap().is_some(),);
    assert_eq!(aborts.load(Ordering::SeqCst), 0);
    assert!(runtime.root_symbol_value("answer").is_ok());
}
