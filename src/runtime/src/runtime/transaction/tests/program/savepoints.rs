use super::support::savepoint_effect;
use crate::{
    ActorId, CapabilityId, MechRuntime, MessageId, MessageRecord, ModuleVersionId, ObjectId,
    ObjectRecord, ResourceBudget, RuntimeAuthorityScope, RuntimeEventKind, RuntimeId,
    RuntimeInvalidOperationError, TaskId,
};
use mech_core::{MResult, MechError, MechSourceCode};

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
fn explicit_program_operations_use_savepoints_before_outer_abort() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();

    runtime
        .run_string_with_context(&mut context, "savepoint-before-failure := 1")
        .unwrap();
    let plan_len_after_a = runtime.program.interpreter().plan_len();
    let events_after_a = context.events.clone();
    let access_after_a = context.access.clone();
    let staged_events_after_a = runtime
        .active_transaction_mut(transaction_id)
        .unwrap()
        .staged_event_ids();

    let failure: MResult<()> = runtime.with_atomic_program_operation(
        &mut context,
        "explicit_b_test",
        |runtime, context| {
            runtime.program.run_source(&MechSourceCode::String(
                "savepoint-rolled-back := savepoint-before-failure + 1".to_string(),
            ))?;
            runtime
                .active_transaction_mut(transaction_id)?
                .stage_put_object(ObjectRecord::text(ObjectId(350), "note", "B provisional"))?;
            context.record_write(ObjectId(350));
            runtime.emit_event_to_context(
                context,
                RuntimeEventKind::ObjectCreated {
                    object_id: ObjectId(350),
                },
            )?;
            Err(MechError::new(
                RuntimeInvalidOperationError {
                    operation: "explicit_b_test",
                    reason: "deliberate B failure".to_string(),
                },
                None,
            ))
        },
    );

    assert!(failure.is_err());
    assert!(
        runtime
            .program
            .root_symbol_value("savepoint-before-failure")
            .is_ok()
    );
    assert!(
        runtime
            .program
            .root_symbol_value("savepoint-rolled-back")
            .is_err()
    );
    assert_eq!(runtime.program.interpreter().plan_len(), plan_len_after_a);
    assert_eq!(context.events, events_after_a);
    assert_eq!(context.access, access_after_a);
    let transaction = runtime.active_transaction_mut(transaction_id).unwrap();
    assert_eq!(transaction.staged_puts().count(), 0);
    assert_eq!(transaction.staged_event_ids(), staged_events_after_a);
    assert_eq!(context.transaction, Some(transaction_id));
    assert_eq!(runtime.program_transaction_owner, Some(transaction_id));

    runtime
        .run_string_with_context(
            &mut context,
            "savepoint-after-failure := savepoint-before-failure + 2",
        )
        .unwrap();
    assert!(
        runtime
            .program
            .root_symbol_value("savepoint-after-failure")
            .is_ok()
    );

    runtime
        .abort_runtime_transaction(&mut context, "discard A and C")
        .unwrap();

    assert!(
        runtime
            .program
            .root_symbol_value("savepoint-before-failure")
            .is_err()
    );
    assert!(
        runtime
            .program
            .root_symbol_value("savepoint-rolled-back")
            .is_err()
    );
    assert!(
        runtime
            .program
            .root_symbol_value("savepoint-after-failure")
            .is_err()
    );
    assert!(runtime.get_object(ObjectId(350)).unwrap().is_none());
    assert_eq!(runtime.program_transaction_owner, None);
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
