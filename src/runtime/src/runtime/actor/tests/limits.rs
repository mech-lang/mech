use crate::actor_behavior::{ActorBehaviorDriver, ActorBehaviorRuntime};
use crate::{
    ActorId, ActorRecord, ActorTurn, MechRuntime, MessageId, MessageRecord,
    ResourceBudgetExceededError, RuntimeBuilder, RuntimeConfig, RuntimeContext, RuntimeEventKind,
};
use mech_core::MResult;

#[derive(Debug)]
struct SleepingActorBehaviorDriver;

impl ActorBehaviorDriver for SleepingActorBehaviorDriver {
    fn run_actor_turn(
        &mut self,
        _runtime: &mut dyn ActorBehaviorRuntime,
        _context: &mut RuntimeContext,
        _turn: &ActorTurn,
    ) -> MResult<()> {
        std::thread::sleep(std::time::Duration::from_millis(30));
        Ok(())
    }
}

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
fn turn_duration_limit_reports_overrun() {
    let mut config = RuntimeConfig::default();
    config.limits.max_turn_duration_ms = Some(5);
    let mut runtime = RuntimeBuilder::new()
        .config(config)
        .actor_behavior_driver(SleepingActorBehaviorDriver)
        .build()
        .unwrap();
    let actor = ActorRecord::new(ActorId(1), "actor:1");
    let message = MessageRecord::new(MessageId(1), ActorId(1), "tick", Vec::new());
    let turn = ActorTurn::new(actor, message).unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let error = runtime
        .run_actor_turn_envelope(&mut context, &turn)
        .unwrap_err();
    let budget = error.kind_as::<ResourceBudgetExceededError>().unwrap();
    assert_eq!(budget.resource, "turn_duration_ms");
    assert!(budget.requested > 5);
    assert_eq!(budget.max, Some(5));
    assert!(
        !context
            .events
            .iter()
            .any(|event| matches!(event.kind, RuntimeEventKind::ActorTurnCompleted { .. }))
    );
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
