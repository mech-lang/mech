use crate::{ActorId, ActorRecord, MechRuntime, ResourceBudgetExceededError, RuntimeConfig};

#[test]
fn max_actors_is_enforced() {
    let mut config = RuntimeConfig::default();
    config.limits.max_actors = Some(1);
    let mut runtime = MechRuntime::new(config).unwrap();

    runtime
        .put_actor(ActorRecord::new(ActorId(1), "actor:1"))
        .unwrap();

    let error = runtime
        .put_actor(ActorRecord::new(ActorId(2), "actor:2"))
        .unwrap_err();
    let budget = error.kind_as::<ResourceBudgetExceededError>().unwrap();
    assert_eq!(budget.resource, "actors");
    assert_eq!(budget.used, 1);
    assert_eq!(budget.requested, 1);
    assert_eq!(budget.max, Some(1));

    let duplicate = runtime
        .put_actor(ActorRecord::new(ActorId(1), "actor:1"))
        .unwrap_err();
    assert_eq!(duplicate.kind_name(), "StoreRecordAlreadyExists");
}

#[test]
fn mailbox_limit_survives_fresh_contexts() {
    let mut config = RuntimeConfig::default();
    config.limits.max_actor_mailbox_len = Some(2);
    let mut runtime = MechRuntime::new(config).unwrap();
    runtime
        .put_actor(ActorRecord::new(ActorId(1), "actor:1"))
        .unwrap();

    runtime
        .send_message(ActorId(1), "ping", b"one".to_vec())
        .unwrap();
    runtime
        .send_message(ActorId(1), "ping", b"two".to_vec())
        .unwrap();

    let error = runtime
        .send_message(ActorId(1), "ping", b"three".to_vec())
        .unwrap_err();
    let budget = error.kind_as::<ResourceBudgetExceededError>().unwrap();
    assert_eq!(budget.resource, "actor_mailbox");
    assert_eq!(budget.used, 2);
    assert_eq!(budget.requested, 1);
    assert_eq!(budget.max, Some(2));
}

#[test]
fn transactional_mailbox_limit_uses_effective_length() {
    let mut config = RuntimeConfig::default();
    config.limits.max_actor_mailbox_len = Some(2);
    let mut runtime = MechRuntime::new(config).unwrap();
    runtime
        .put_actor(ActorRecord::new(ActorId(1), "actor:1"))
        .unwrap();
    runtime
        .send_message(ActorId(1), "ping", b"one".to_vec())
        .unwrap();
    runtime
        .send_message(ActorId(1), "ping", b"two".to_vec())
        .unwrap();

    let mut context = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut context).unwrap();
    let popped = runtime
        .pop_message_with_context(&mut context, ActorId(1))
        .unwrap()
        .unwrap();
    assert_eq!(popped.payload, b"one");

    runtime
        .send_message_with_context(&mut context, ActorId(1), "ping", b"three".to_vec())
        .unwrap();

    let error = runtime
        .send_message_with_context(&mut context, ActorId(1), "ping", b"four".to_vec())
        .unwrap_err();
    let budget = error.kind_as::<ResourceBudgetExceededError>().unwrap();
    assert_eq!(budget.resource, "actor_mailbox");
    assert_eq!(budget.used, 2);

    runtime
        .abort_runtime_transaction(&mut context, "rollback")
        .unwrap();

    assert_eq!(
        runtime.pop_message(ActorId(1)).unwrap().unwrap().payload,
        b"one",
    );
    assert_eq!(
        runtime.pop_message(ActorId(1)).unwrap().unwrap().payload,
        b"two",
    );
    assert!(runtime.pop_message(ActorId(1)).unwrap().is_none());
}
