use super::support::savepoint_effect;
use crate::runtime::gate_a_probe::{gate_a_cost_snapshot, reset_gate_a_costs};
use crate::{
    ActorId, CapabilityId, MechRuntime, MessageId, MessageRecord, ModuleVersionId, ObjectId,
    ObjectRecord, ResourceBudget, RuntimeAuthorityScope, RuntimeConfig, RuntimeEventKind,
    RuntimeId, RuntimeInvalidOperationError, TaskId,
};
use mech_core::{MResult, MechError};

#[test]
fn program_operation_savepoint_truncates_effects_without_reusing_ids() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();

    let first = runtime
        .with_atomic_program_operation(
            &mut context,
            "effect_savepoint_first",
            |runtime, context| {
                runtime.stage_runtime_effect_with_context(context, savepoint_effect("first"))
            },
        )
        .unwrap();
    assert_eq!(first.sequence, 0);

    reset_gate_a_costs();
    let failed: MResult<()> = runtime.with_atomic_program_operation(
        &mut context,
        "effect_savepoint_failed",
        |runtime, context| {
            runtime.stage_runtime_effect_with_context(context, savepoint_effect("rolled-back"))?;
            Err(MechError::new(
                RuntimeInvalidOperationError {
                    operation: "effect_savepoint_failed",
                    reason: "deliberate effect savepoint failure".to_string(),
                },
                None,
            ))
        },
    );
    assert_eq!(failed.unwrap_err().kind_name(), "RuntimeInvalidOperation");
    let costs = gate_a_cost_snapshot();
    assert_eq!(costs.runtime_transaction_savepoint_clone_count, 3);
    assert_eq!(costs.runtime_transaction_savepoint_items, 9);

    let transaction = runtime
        .active_execution_transaction(transaction_id)
        .unwrap();
    assert_eq!(transaction.effects.len(), 1);
    assert_eq!(transaction.effects.next_sequence(), 2);

    let third = runtime
        .with_atomic_program_operation(
            &mut context,
            "effect_savepoint_third",
            |runtime, context| {
                runtime.stage_runtime_effect_with_context(context, savepoint_effect("third"))
            },
        )
        .unwrap();
    assert_eq!(third.sequence, 2);
    assert_eq!(
        runtime
            .active_execution_transaction(transaction_id)
            .unwrap()
            .effects
            .len(),
        2,
    );

    runtime
        .abort_runtime_transaction(&mut context, "discard staged effects")
        .unwrap();
}

#[test]
fn failed_operation_restores_context_and_staging_but_keeps_budget_usage() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    context.authority = RuntimeAuthorityScope::allow_list([CapabilityId(10)]);
    context.budget = ResourceBudget::default()
        .with_max_steps(100)
        .with_max_bytes(100)
        .with_max_items(100)
        .with_max_messages(100);
    context.charge_step().unwrap();
    context.record_read(ObjectId(10));
    let baseline = context.clone();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    let operation_events = context.events.clone();
    let staged_events = runtime
        .active_transaction_mut(transaction_id)
        .unwrap()
        .staged_event_ids();

    let result: MResult<()> = runtime.with_atomic_program_operation(
        &mut context,
        "context_rollback_test",
        |runtime, context| {
            context.charge_steps(3)?;
            context.charge_bytes(4)?;
            context.charge_items(5)?;
            context.charge_messages(6)?;
            context.record_read(ObjectId(11));
            context.record_write(ObjectId(12));
            runtime
                .active_transaction_mut(transaction_id)?
                .stage_put_object(ObjectRecord::text(ObjectId(400), "note", "provisional"))?;
            runtime.emit_event_to_context(
                context,
                RuntimeEventKind::ObjectCreated {
                    object_id: ObjectId(400),
                },
            )?;

            context.runtime = RuntimeId(999);
            context.subject = "mutated-subject".to_string();
            context.task = Some(TaskId(20));
            context.actor = Some(ActorId(21));
            context.module_version = Some(ModuleVersionId(22));
            context.transaction = None;
            context.authority = RuntimeAuthorityScope::allow_list([CapabilityId(23)]);
            context.budget.max_steps = Some(4);
            context.budget.max_bytes = Some(5);
            context.budget.max_items = Some(6);
            context.budget.max_messages = Some(7);
            context.actor_message = Some(MessageRecord::new(
                MessageId(24),
                ActorId(21),
                "mutated",
                Vec::new(),
            ));
            context.actor_state = Some(ObjectId(25));

            Err(MechError::new(
                RuntimeInvalidOperationError {
                    operation: "context_rollback_test",
                    reason: "deliberate failure".to_string(),
                },
                None,
            ))
        },
    );

    assert!(result.is_err());
    assert_eq!(context.runtime, baseline.runtime);
    assert_eq!(context.subject, baseline.subject);
    assert_eq!(context.task, baseline.task);
    assert_eq!(context.actor, baseline.actor);
    assert_eq!(context.module_version, baseline.module_version);
    assert_eq!(context.transaction, Some(transaction_id));
    assert_eq!(context.authority, baseline.authority);
    assert_eq!(context.access, baseline.access);
    assert_eq!(context.events, operation_events);
    assert_eq!(context.actor_message, baseline.actor_message);
    assert_eq!(context.actor_state, baseline.actor_state);
    assert_eq!(context.budget.max_steps, Some(100));
    assert_eq!(context.budget.max_bytes, Some(100));
    assert_eq!(context.budget.max_items, Some(100));
    assert_eq!(context.budget.max_messages, Some(100));
    assert_eq!(context.budget.used_steps, baseline.budget.used_steps + 3);
    assert_eq!(context.budget.used_bytes, baseline.budget.used_bytes + 4);
    assert_eq!(context.budget.used_items, baseline.budget.used_items + 5);
    assert_eq!(
        context.budget.used_messages,
        baseline.budget.used_messages + 6,
    );
    let transaction = runtime.active_transaction_mut(transaction_id).unwrap();
    assert_eq!(transaction.staged_puts().count(), 0);
    assert_eq!(transaction.staged_event_ids(), staged_events);

    runtime
        .abort_runtime_transaction(&mut context, "context rollback complete")
        .unwrap();
    assert_eq!(context.transaction, None);
    assert_eq!(context.budget.used_steps, baseline.budget.used_steps + 3);
}

#[test]
fn rollback_after_retention_overflow_restores_exact_visible_baseline() {
    let mut config = RuntimeConfig::default();
    config.limits.max_in_memory_events = Some(3);
    let mut runtime = MechRuntime::new(config).unwrap();
    let mut context = runtime.runtime_context().unwrap();

    for object in 1..=3 {
        runtime
            .put_object_with_context(
                &mut context,
                ObjectRecord::text(ObjectId(object), "note", object.to_string()),
            )
            .unwrap();
    }

    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    let baseline = context.events().to_vec();
    assert_eq!(baseline.len(), 3);

    let result: MResult<()> = runtime.with_atomic_program_operation(
        &mut context,
        "retention_overflow_rollback",
        |runtime, context| {
            runtime.emit_event_to_context(context, RuntimeEventKind::RuntimeTickStarted)?;
            runtime.emit_event_to_context(
                context,
                RuntimeEventKind::RuntimeTickCompleted { work_count: 0 },
            )?;
            assert_eq!(context.events().len(), 3);
            Err(MechError::new(
                RuntimeInvalidOperationError {
                    operation: "retention_overflow_rollback",
                    reason: "deliberate failure".to_string(),
                },
                None,
            ))
        },
    );

    assert!(result.is_err());
    assert_eq!(context.events(), baseline);
    assert!(context.event_storage_physical_len() > context.events().len());
    runtime
        .abort_runtime_transaction(&mut context, "finish retention test")
        .unwrap();
    assert_eq!(context.events().len(), 3);
    assert!(context.event_storage_physical_len() < 2 * context.events().len());
    assert!(!runtime.active_transactions.contains_key(&transaction_id));
}
