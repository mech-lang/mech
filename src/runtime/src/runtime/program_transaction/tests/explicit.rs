use super::super::{
    ActorId, MechRuntime, MessageId, MessageRecord, ObjectId, ObjectRecord, RuntimeContext, TaskId,
};

#[test]
fn explicit_program_commit_keeps_program_and_commits_access_delta() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    context.record_read(ObjectId(70));
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();

    runtime
        .run_string_with_context(&mut context, "round3-committed := 41 + 1")
        .unwrap();
    context.record_read(ObjectId(71));
    context.record_write(ObjectId(72));

    runtime.commit_runtime_transaction(&mut context).unwrap();

    assert!(
        runtime
            .program
            .root_symbol_value("round3-committed")
            .is_ok()
    );
    assert_eq!(runtime.program_transaction_owner, None);
    assert_eq!(context.transaction, None);
    let record = runtime.get_transaction(transaction_id).unwrap().unwrap();
    assert!(!record.read_set.contains(&ObjectId(70)));
    assert!(record.read_set.contains(&ObjectId(71)));
    assert!(record.write_set.contains(&ObjectId(72)));
}

#[test]
fn explicit_commit_failure_keeps_program_provisional_until_abort() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();

    runtime
        .run_string_with_context(&mut context, "round3-provisional := 42")
        .unwrap();
    runtime
        .update_object_with_context(
            &mut context,
            ObjectRecord::text(ObjectId(200), "note", "missing"),
        )
        .unwrap();

    assert!(runtime.commit_runtime_transaction(&mut context).is_err());
    assert!(
        runtime
            .program
            .root_symbol_value("round3-provisional")
            .is_ok()
    );
    assert_eq!(context.transaction, Some(transaction_id));
    assert_eq!(runtime.program_transaction_owner, Some(transaction_id));
    assert!(runtime.active_transactions.contains_key(&transaction_id));

    runtime
        .abort_runtime_transaction(&mut context, "failed commit")
        .unwrap();

    assert!(
        runtime
            .program
            .root_symbol_value("round3-provisional")
            .is_err()
    );
    assert_eq!(runtime.program_transaction_owner, None);
}

#[test]
fn one_transaction_owns_program_while_other_store_work_remains_allowed() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context_a = runtime.runtime_context().unwrap();
    let transaction_a = runtime.begin_transaction(&mut context_a).unwrap();
    runtime
        .run_string_with_context(&mut context_a, "round3-owner-a := 1")
        .unwrap();

    let mut context_b = runtime.runtime_context().unwrap();
    let transaction_b = runtime.begin_transaction(&mut context_b).unwrap();
    runtime
        .put_object_with_context(
            &mut context_b,
            ObjectRecord::text(ObjectId(300), "note", "B store-only"),
        )
        .unwrap();

    let b_error = runtime
        .run_string_with_context(&mut context_b, "round3-owner-b := 2")
        .unwrap_err();
    assert_eq!(b_error.kind_name(), "RuntimeProgramBusy");

    let mut unowned_context = runtime.runtime_context().unwrap();
    let implicit_error = runtime
        .run_string_with_context(&mut unowned_context, "round3-unowned := 3")
        .unwrap_err();
    assert_eq!(implicit_error.kind_name(), "RuntimeProgramBusy");
    assert_eq!(runtime.program_transaction_owner, Some(transaction_a));

    runtime
        .abort_runtime_transaction(&mut context_a, "release A")
        .unwrap();
    runtime
        .run_string_with_context(&mut context_b, "round3-owner-b := 2")
        .unwrap();

    assert_eq!(runtime.program_transaction_owner, Some(transaction_b));
    assert!(runtime.program.root_symbol_value("round3-owner-b").is_ok());
    assert!(runtime.get_object(ObjectId(300)).unwrap().is_none());

    runtime
        .abort_runtime_transaction(&mut context_b, "release B")
        .unwrap();
    assert!(runtime.program.root_symbol_value("round3-owner-b").is_err());
    assert!(runtime.get_object(ObjectId(300)).unwrap().is_none());
}

#[test]
fn failed_first_explicit_operation_releases_program_ownership() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context_a = runtime.runtime_context().unwrap();
    let transaction_a = runtime.begin_transaction(&mut context_a).unwrap();

    assert!(
        runtime
            .run_string_with_context(
                &mut context_a,
                "round3-first-fails := missing-round3-first + 1",
            )
            .is_err()
    );
    assert_eq!(runtime.program_transaction_owner, None);
    assert!(runtime.active_transactions.contains_key(&transaction_a));
    assert!(
        runtime
            .active_execution_transaction(transaction_a)
            .unwrap()
            .program
            .is_none()
    );

    let mut context_b = runtime.runtime_context().unwrap();
    let transaction_b = runtime.begin_transaction(&mut context_b).unwrap();
    runtime
        .run_string_with_context(&mut context_b, "round3-after-failure := 2")
        .unwrap();
    assert_eq!(runtime.program_transaction_owner, Some(transaction_b));

    runtime
        .abort_runtime_transaction(&mut context_b, "release B")
        .unwrap();
    runtime
        .abort_runtime_transaction(&mut context_a, "release A")
        .unwrap();
}

#[test]
fn transaction_context_identity_includes_task_actor_message_and_state() {
    fn assert_mismatch(mutate: impl FnOnce(&mut RuntimeContext)) {
        let mut runtime = MechRuntime::builder().build().unwrap();
        let mut context = runtime.runtime_context().unwrap();
        let baseline = context.clone();
        let transaction_id = runtime.begin_transaction(&mut context).unwrap();
        mutate(&mut context);

        let error = runtime
            .run_string_with_context(&mut context, "identity-test := 1")
            .unwrap_err();
        assert_eq!(error.kind_name(), "RuntimeTransactionContextMismatch");
        assert!(runtime.program.root_symbol_value("identity-test").is_err());

        context = baseline;
        context.transaction = Some(transaction_id);
        runtime
            .abort_runtime_transaction(&mut context, "identity mismatch test")
            .unwrap();
    }

    assert_mismatch(|context| context.task = Some(TaskId(1)));
    assert_mismatch(|context| context.actor = Some(ActorId(2)));
    assert_mismatch(|context| {
        context.actor_message = Some(MessageRecord::new(
            MessageId(3),
            ActorId(2),
            "identity",
            Vec::new(),
        ));
    });
    assert_mismatch(|context| context.actor_state = Some(ObjectId(4)));
}
