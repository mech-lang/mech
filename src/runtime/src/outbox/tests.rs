use super::*;
use crate::RecordEstimate;

fn effect(turn: u64, ordinal: u32, payload: &'static [u8]) -> OwnedEffectIntent<Box<[u8]>> {
    OwnedEffectIntent {
        id: OutboxEffectId {
            turn_id: TurnId::new(turn).unwrap(),
            ordinal,
        },
        operation: "write".to_string(),
        target: "test".to_string(),
        payload: payload.into(),
        idempotency_key: format!("{turn}:{ordinal}"),
        delivery: OutboxDeliveryPolicy::default(),
    }
}

fn estimate(effects: &[OwnedEffectIntent<Box<[u8]>>]) -> RecordEstimate {
    RecordEstimate {
        records: effects.len(),
        bytes: effects.iter().map(AccountedRecord::retained_bytes).sum(),
    }
}

#[test]
fn batch_is_sorted_by_turn_then_ordinal_and_transferred_once() {
    let mut outbox = RetainedEffectOutbox::new(3, 256).unwrap();
    let effects = vec![effect(2, 0, b"c"), effect(1, 1, b"b"), effect(1, 0, b"a")];
    let permit = outbox.reserve(estimate(&effects)).unwrap();
    let prepared = outbox.prepare_batch(permit, effects).unwrap();
    outbox.append(prepared);
    let ids = outbox.iter().map(|effect| effect.id).collect::<Vec<_>>();
    assert_eq!(ids[0].turn_id.get(), 1);
    assert_eq!(ids[0].ordinal, 0);
    assert_eq!(ids[1].ordinal, 1);
    assert_eq!(ids[2].turn_id.get(), 2);
    assert!(
        outbox
            .iter()
            .all(|effect| effect.delivery == OutboxDeliveryPolicy::AtLeastOnce)
    );
    assert_eq!(outbox.drain().count(), 3);
    assert!(outbox.is_empty());
}

#[test]
fn duplicate_effect_id_is_rejected_before_publication() {
    let outbox = RetainedEffectOutbox::new(2, 256).unwrap();
    let effects = vec![effect(1, 0, b"a"), effect(1, 0, b"b")];
    let permit = outbox.reserve(estimate(&effects)).unwrap();
    assert_eq!(
        outbox
            .prepare_batch(permit, effects)
            .unwrap_err()
            .kind_name(),
        "DuplicateOutboxEffectId"
    );
    assert!(outbox.is_empty());
}

#[test]
fn later_batch_cannot_break_retained_effect_order() {
    let mut outbox = RetainedEffectOutbox::new(2, 256).unwrap();
    let retained = vec![effect(2, 0, b"retained")];
    let permit = outbox.reserve(estimate(&retained)).unwrap();
    let prepared = outbox.prepare_batch(permit, retained).unwrap();
    outbox.append(prepared);

    let earlier = vec![effect(1, 0, b"earlier")];
    let permit = outbox.reserve(estimate(&earlier)).unwrap();
    assert_eq!(
        outbox
            .prepare_batch(permit, earlier)
            .unwrap_err()
            .kind_name(),
        "InvalidOutboxEffectOrder"
    );
    assert_eq!(outbox.len(), 1);
}

#[test]
fn delivery_does_not_erase_the_effect_ordering_watermark() {
    let mut outbox = RetainedEffectOutbox::new(2, 256).unwrap();
    let delivered = vec![effect(2, 0, b"delivered")];
    let permit = outbox.reserve(estimate(&delivered)).unwrap();
    let prepared = outbox.prepare_batch(permit, delivered).unwrap();
    outbox.append(prepared);
    assert_eq!(outbox.drain().count(), 1);

    for stale in [effect(1, 0, b"older"), effect(2, 0, b"duplicate")] {
        let effects = vec![stale];
        let permit = outbox.reserve(estimate(&effects)).unwrap();
        assert_eq!(
            outbox
                .prepare_batch(permit, effects)
                .unwrap_err()
                .kind_name(),
            "InvalidOutboxEffectOrder"
        );
    }

    let newer = vec![effect(3, 0, b"newer")];
    let permit = outbox.reserve(estimate(&newer)).unwrap();
    let prepared = outbox.prepare_batch(permit, newer).unwrap();
    outbox.append(prepared);
    assert_eq!(outbox.iter().next().unwrap().id.turn_id.get(), 3);
}

#[test]
fn dropping_prepared_batch_releases_capacity_and_owned_payloads() {
    let outbox = RetainedEffectOutbox::new(1, 128).unwrap();
    let effects = vec![effect(1, 0, b"payload")];
    let estimate = estimate(&effects);
    let permit = outbox.reserve(estimate).unwrap();
    let prepared = outbox.prepare_batch(permit, effects).unwrap();
    drop(prepared);
    assert!(outbox.reserve(estimate).is_ok());
}
