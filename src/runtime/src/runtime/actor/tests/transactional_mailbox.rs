use super::runtime_with_actor_and_messages;
use crate::id::SequentialIdGenerator;
use crate::{
    ActorId, ActorRecord, InMemoryStore, MechRuntime, MechStore, MessageId, MessageRecord,
};

#[test]
fn transactional_pops_return_distinct_durable_messages_in_fifo_order() {
    let mut runtime = runtime_with_actor_and_messages(&[b"one", b"two"]);
    let mut context = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut context).unwrap();

    assert_eq!(
        runtime
            .pop_message_with_context(&mut context, ActorId(1))
            .unwrap()
            .unwrap()
            .payload,
        b"one",
    );
    assert_eq!(
        runtime
            .pop_message_with_context(&mut context, ActorId(1))
            .unwrap()
            .unwrap()
            .payload,
        b"two",
    );
}

#[test]
fn transactional_pops_three_durable_messages_without_repetition() {
    let mut runtime = runtime_with_actor_and_messages(&[b"one", b"two", b"three"]);
    let mut context = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut context).unwrap();

    let payloads: Vec<Vec<u8>> = (0..3)
        .map(|_| {
            runtime
                .pop_message_with_context(&mut context, ActorId(1))
                .unwrap()
                .unwrap()
                .payload
        })
        .collect();

    assert_eq!(
        payloads,
        vec![b"one".to_vec(), b"two".to_vec(), b"three".to_vec()]
    );
}

#[test]
fn transactional_mailbox_returns_durable_before_staged_enqueue() {
    let mut runtime = runtime_with_actor_and_messages(&[b"durable"]);
    let mut context = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut context).unwrap();
    runtime
        .send_message_with_context(&mut context, ActorId(1), "ping", b"staged".to_vec())
        .unwrap();

    assert_eq!(
        runtime
            .pop_message_with_context(&mut context, ActorId(1))
            .unwrap()
            .unwrap()
            .payload,
        b"durable",
    );
    assert_eq!(
        runtime
            .pop_message_with_context(&mut context, ActorId(1))
            .unwrap()
            .unwrap()
            .payload,
        b"staged",
    );
}

#[test]
fn transactional_peek_then_pop_returns_same_effective_head() {
    let mut runtime = runtime_with_actor_and_messages(&[b"one", b"two"]);
    let mut context = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut context).unwrap();

    let peeked = runtime
        .peek_message_with_context(&mut context, ActorId(1))
        .unwrap()
        .unwrap();
    let popped = runtime
        .pop_message_with_context(&mut context, ActorId(1))
        .unwrap()
        .unwrap();
    assert_eq!(peeked.id, popped.id);
    assert_eq!(popped.payload, b"one");
}

#[test]
fn transactional_staged_enqueues_fifo_after_durable_exhausted_then_none() {
    let mut runtime = runtime_with_actor_and_messages(&[b"durable"]);
    let mut context = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut context).unwrap();
    runtime
        .send_message_with_context(&mut context, ActorId(1), "ping", b"staged-one".to_vec())
        .unwrap();
    runtime
        .send_message_with_context(&mut context, ActorId(1), "ping", b"staged-two".to_vec())
        .unwrap();

    let payloads: Vec<Option<Vec<u8>>> = (0..4)
        .map(|_| {
            runtime
                .pop_message_with_context(&mut context, ActorId(1))
                .unwrap()
                .map(|m| m.payload)
        })
        .collect();

    assert_eq!(
        payloads,
        vec![
            Some(b"durable".to_vec()),
            Some(b"staged-one".to_vec()),
            Some(b"staged-two".to_vec()),
            None,
        ],
    );
}

#[test]
fn commit_removes_acknowledged_durable_messages_once() {
    let mut runtime = runtime_with_actor_and_messages(&[b"one", b"two", b"three"]);
    let mut context = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut context).unwrap();
    runtime
        .pop_message_with_context(&mut context, ActorId(1))
        .unwrap();
    runtime
        .pop_message_with_context(&mut context, ActorId(1))
        .unwrap();
    runtime.commit_runtime_transaction(&mut context).unwrap();

    assert_eq!(
        runtime.pop_message(ActorId(1)).unwrap().unwrap().payload,
        b"three"
    );
    assert!(runtime.pop_message(ActorId(1)).unwrap().is_none());
}

#[test]
fn abort_leaves_durable_messages_and_discards_staged_enqueues() {
    let mut runtime = runtime_with_actor_and_messages(&[b"one", b"two"]);
    let mut context = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut context).unwrap();
    runtime
        .pop_message_with_context(&mut context, ActorId(1))
        .unwrap();
    runtime
        .send_message_with_context(&mut context, ActorId(1), "ping", b"staged".to_vec())
        .unwrap();
    runtime
        .abort_runtime_transaction(&mut context, "rollback")
        .unwrap();

    assert_eq!(
        runtime.pop_message(ActorId(1)).unwrap().unwrap().payload,
        b"one"
    );
    assert_eq!(
        runtime.pop_message(ActorId(1)).unwrap().unwrap().payload,
        b"two"
    );
    assert!(runtime.pop_message(ActorId(1)).unwrap().is_none());
}

#[test]
fn duplicate_durable_message_ids_are_consumed_by_occurrence() {
    let mut store = InMemoryStore::new();
    store
        .put_actor(ActorRecord::new(ActorId(1), "actor:1"))
        .unwrap();
    store
        .enqueue_message(
            ActorId(1),
            MessageRecord::new(MessageId(5), ActorId(1), "ping", b"durable-one".to_vec()),
        )
        .unwrap();
    store
        .enqueue_message(
            ActorId(1),
            MessageRecord::new(MessageId(5), ActorId(1), "ping", b"durable-two".to_vec()),
        )
        .unwrap();

    let mut runtime = MechRuntime::builder().store(store).build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();

    assert_eq!(
        runtime
            .pop_message_with_context(&mut context, ActorId(1))
            .unwrap()
            .unwrap()
            .payload,
        b"durable-one",
    );
    assert_eq!(
        runtime
            .pop_message_with_context(&mut context, ActorId(1))
            .unwrap()
            .unwrap()
            .payload,
        b"durable-two",
    );
    assert!(
        runtime
            .pop_message_with_context(&mut context, ActorId(1))
            .unwrap()
            .is_none()
    );
    assert_eq!(
        runtime
            .active_transactions
            .get(&transaction_id)
            .unwrap()
            .store
            .staged_message_ack_occurrences(ActorId(1), MessageId(5)),
        2,
    );

    runtime.commit_runtime_transaction(&mut context).unwrap();

    assert!(runtime.pop_message(ActorId(1)).unwrap().is_none());
    assert_eq!(
        runtime
            .get_transaction(transaction_id)
            .unwrap()
            .unwrap()
            .message_acks,
        vec![MessageId(5), MessageId(5)],
    );
}

#[test]
fn duplicate_durable_message_ids_mixed_with_other_ids_preserve_fifo() {
    let mut store = InMemoryStore::new();
    store
        .put_actor(ActorRecord::new(ActorId(1), "actor:1"))
        .unwrap();
    for (id, payload) in [
        (MessageId(5), b"one".to_vec()),
        (MessageId(6), b"two".to_vec()),
        (MessageId(5), b"three".to_vec()),
    ] {
        store
            .enqueue_message(
                ActorId(1),
                MessageRecord::new(id, ActorId(1), "ping", payload),
            )
            .unwrap();
    }

    let mut runtime = MechRuntime::builder().store(store).build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut context).unwrap();

    let payloads: Vec<Vec<u8>> = (0..3)
        .map(|_| {
            runtime
                .pop_message_with_context(&mut context, ActorId(1))
                .unwrap()
                .unwrap()
                .payload
        })
        .collect();

    assert_eq!(
        payloads,
        vec![b"one".to_vec(), b"two".to_vec(), b"three".to_vec()],
    );
    assert!(
        runtime
            .pop_message_with_context(&mut context, ActorId(1))
            .unwrap()
            .is_none()
    );
}

#[test]
fn durable_staged_id_collision_commit_keeps_unpopped_staged_message() {
    let mut store = InMemoryStore::new();
    store
        .put_actor(ActorRecord::new(ActorId(1), "actor:1"))
        .unwrap();
    store
        .enqueue_message(
            ActorId(1),
            MessageRecord::new(MessageId(5), ActorId(1), "ping", b"durable".to_vec()),
        )
        .unwrap();

    let mut runtime = MechRuntime::builder()
        .store(store)
        .id_generator(SequentialIdGenerator::starting_at(1))
        .build()
        .unwrap();
    let mut context = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut context).unwrap();
    let staged_id = runtime
        .send_message_with_context(&mut context, ActorId(1), "ping", b"staged".to_vec())
        .unwrap();
    assert_eq!(staged_id, MessageId(5));

    let popped = runtime
        .pop_message_with_context(&mut context, ActorId(1))
        .unwrap()
        .unwrap();
    assert_eq!(popped.payload, b"durable".to_vec());

    runtime.commit_runtime_transaction(&mut context).unwrap();

    let remaining = runtime.pop_message(ActorId(1)).unwrap().unwrap();
    assert_eq!(remaining.id, MessageId(5));
    assert_eq!(remaining.payload, b"staged".to_vec());
    assert!(runtime.pop_message(ActorId(1)).unwrap().is_none());
}

#[test]
fn transactional_pop_preserves_provenance_when_durable_and_staged_ids_collide() {
    let mut store = InMemoryStore::new();
    store
        .put_actor(ActorRecord::new(ActorId(1), "actor:1"))
        .unwrap();
    store
        .enqueue_message(
            ActorId(1),
            MessageRecord::new(MessageId(5), ActorId(1), "ping", b"durable".to_vec()),
        )
        .unwrap();

    let mut runtime = MechRuntime::builder()
        .store(store)
        .id_generator(SequentialIdGenerator::starting_at(1))
        .build()
        .unwrap();
    let mut context = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut context).unwrap();
    let staged_id = runtime
        .send_message_with_context(&mut context, ActorId(1), "ping", b"staged".to_vec())
        .unwrap();

    assert_eq!(staged_id, MessageId(5));

    let first = runtime
        .pop_message_with_context(&mut context, ActorId(1))
        .unwrap()
        .unwrap();
    let second = runtime
        .pop_message_with_context(&mut context, ActorId(1))
        .unwrap()
        .unwrap();
    let third = runtime
        .pop_message_with_context(&mut context, ActorId(1))
        .unwrap();

    assert_eq!(first.id, MessageId(5));
    assert_eq!(second.id, MessageId(5));
    assert_eq!(first.payload, b"durable".to_vec());
    assert_eq!(second.payload, b"staged".to_vec());
    assert!(third.is_none());

    runtime.commit_runtime_transaction(&mut context).unwrap();

    assert!(runtime.pop_message(ActorId(1)).unwrap().is_none());
}
