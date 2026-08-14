use super::{event_count, new_runtime};
use crate::runtime::gate_a_probe::{gate_a_cost_snapshot, reset_gate_a_costs};
use crate::{EventId, ObjectId, ObjectRecord, RuntimeConfig, RuntimeEventKind};

#[test]
fn transaction_commit_persists_staged_events_once() {
    let mut runtime = new_runtime();
    let mut context = runtime.runtime_context().unwrap();

    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    let started_id = context
        .events
        .iter()
        .find(|event| event.kind == (RuntimeEventKind::TransactionStarted { transaction_id }))
        .map(|event| event.id)
        .unwrap();

    runtime
        .put_object_with_context(
            &mut context,
            ObjectRecord::text(ObjectId(100), "note", "hello"),
        )
        .unwrap();
    runtime
        .update_object_with_context(
            &mut context,
            ObjectRecord::text(ObjectId(100), "note", "updated"),
        )
        .unwrap();

    let staged_event_ids: Vec<EventId> = context
        .events
        .iter()
        .filter(|event| {
            matches!(
                event.kind,
                RuntimeEventKind::ObjectCreated { .. } | RuntimeEventKind::ObjectUpdated { .. }
            )
        })
        .map(|event| event.id)
        .collect();

    assert_eq!(
        runtime.commit_runtime_transaction(&mut context).unwrap(),
        transaction_id,
    );

    let object = runtime.get_object(ObjectId(100)).unwrap().unwrap();
    assert_eq!(object.data, b"updated");

    let events = runtime.list_events(None).unwrap();
    assert_eq!(
        event_count(&events, |kind| kind
            == &RuntimeEventKind::ObjectCreated {
                object_id: ObjectId(100),
            },),
        1,
    );
    assert_eq!(
        event_count(&events, |kind| kind
            == &RuntimeEventKind::ObjectUpdated {
                object_id: ObjectId(100),
            },),
        1,
    );
    assert_eq!(
        event_count(&events, |kind| kind
            == &RuntimeEventKind::TransactionCommitted { transaction_id },),
        1,
    );
    let commit_event_id = context
        .events
        .iter()
        .find(|event| event.kind == (RuntimeEventKind::TransactionCommitted { transaction_id }))
        .map(|event| event.id)
        .unwrap();
    assert_eq!(
        events
            .iter()
            .filter(|event| event.id == commit_event_id)
            .count(),
        1,
    );

    let record = runtime.get_transaction(transaction_id).unwrap().unwrap();
    assert!(record.events.contains(&started_id));
    assert!(record.events.contains(&commit_event_id));
    for event_id in &staged_event_ids {
        assert!(record.events.contains(event_id));
        assert_eq!(
            events.iter().filter(|event| event.id == *event_id).count(),
            1,
        );
    }

    let mut unique = record.events.clone();
    unique.sort_by_key(|id| id.as_u128());
    unique.dedup();
    assert_eq!(unique.len(), record.events.len());
    assert!(!runtime.active_transactions.contains_key(&transaction_id));
    assert_eq!(context.transaction, None);
    assert_eq!(context.event_storage_physical_len(), context.events().len());
}

#[test]
fn active_transaction_bounds_hidden_event_history() {
    const LIMIT: usize = 3;
    let mut config = RuntimeConfig::default();
    config.limits.max_in_memory_events = Some(LIMIT as u64);
    let mut runtime = crate::MechRuntime::new(config).unwrap();
    let mut context = runtime.runtime_context().unwrap();

    for object in 1..=LIMIT as u128 {
        runtime
            .put_object_with_context(
                &mut context,
                ObjectRecord::text(ObjectId(object), "seed", object.to_string()),
            )
            .unwrap();
    }

    let baseline_physical = context.event_storage_physical_len();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    reset_gate_a_costs();
    for object in 100..=355 {
        runtime
            .put_object_with_context(
                &mut context,
                ObjectRecord::text(ObjectId(object), "staged", object.to_string()),
            )
            .unwrap();
        assert_eq!(context.events().len(), LIMIT);
        assert!(context.event_storage_physical_len() <= baseline_physical + 4 * LIMIT);
    }

    assert_eq!(gate_a_cost_snapshot().context_event_snapshot_items, 0);
    runtime
        .abort_runtime_transaction(&mut context, "bounded active history")
        .unwrap();
    assert!(!runtime.active_transactions.contains_key(&transaction_id));
    assert_eq!(context.events().len(), LIMIT);
    assert!(context.event_storage_physical_len() < 2 * LIMIT);
}

#[test]
fn outer_commit_bounds_hidden_context_event_history() {
    let mut config = RuntimeConfig::default();
    config.limits.max_in_memory_events = Some(3);
    let mut runtime = crate::MechRuntime::new(config).unwrap();
    let mut context = runtime.runtime_context().unwrap();

    for object in 1..=3 {
        runtime
            .put_object_with_context(
                &mut context,
                ObjectRecord::text(ObjectId(object), "seed", object.to_string()),
            )
            .unwrap();
    }

    reset_gate_a_costs();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    for object in 10..=13 {
        runtime
            .put_object_with_context(
                &mut context,
                ObjectRecord::text(ObjectId(object), "staged", object.to_string()),
            )
            .unwrap();
    }

    assert_eq!(context.events().len(), 3);
    assert!(context.event_storage_physical_len() > context.events().len());
    runtime.commit_runtime_transaction(&mut context).unwrap();

    assert!(!runtime.active_transactions.contains_key(&transaction_id));
    assert_eq!(context.transaction, None);
    assert_eq!(context.events().len(), 3);
    assert!(context.event_storage_physical_len() < 2 * context.events().len());
    let costs = gate_a_cost_snapshot();
    assert_eq!(costs.context_event_snapshot_count, 0);
    assert_eq!(costs.context_event_snapshot_items, 0);
}

#[test]
fn context_event_retention_is_bounded() {
    let mut config = RuntimeConfig::default();
    config.limits.max_in_memory_events = Some(2);
    let mut runtime = crate::MechRuntime::new(config).unwrap();
    let mut context = runtime.runtime_context().unwrap();
    runtime
        .put_object_with_context(&mut context, ObjectRecord::text(ObjectId(1), "text", "one"))
        .unwrap();
    runtime
        .put_object_with_context(&mut context, ObjectRecord::text(ObjectId(2), "text", "two"))
        .unwrap();
    runtime
        .put_object_with_context(
            &mut context,
            ObjectRecord::text(ObjectId(3), "text", "three"),
        )
        .unwrap();
    let object_ids = context
        .events()
        .iter()
        .filter_map(|event| match event.kind {
            RuntimeEventKind::ObjectCreated { object_id } => Some(object_id),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(object_ids, vec![ObjectId(2), ObjectId(3)]);
    assert!(context.event_storage_physical_len() < 2 * context.events().len());
    assert_eq!(context.event_storage_physical_len(), 3);
}

#[test]
fn context_event_retention_steady_state_is_amortized_and_snapshot_free() {
    for limit in [1usize, 3, 32] {
        let mut config = RuntimeConfig::default();
        config.limits.max_in_memory_events = Some(limit as u64);
        let mut runtime = crate::MechRuntime::new(config).unwrap();
        let mut context = runtime.runtime_context().unwrap();

        for index in 0..limit {
            let id = ObjectId(10_000 + index as u128);
            runtime
                .put_object_with_context(
                    &mut context,
                    ObjectRecord::text(id, "seed", index.to_string()),
                )
                .unwrap();
        }
        assert_eq!(context.events().len(), limit);

        reset_gate_a_costs();
        let appended = 4 * limit;
        for index in 0..appended {
            let id = ObjectId(20_000 + index as u128);
            runtime
                .put_object_with_context(
                    &mut context,
                    ObjectRecord::text(id, "steady", index.to_string()),
                )
                .unwrap();
            assert_eq!(context.events().len(), limit);
            assert!(
                context.event_storage_physical_len() < 2 * limit,
                "limit {limit} exceeded the post-scope physical bound after append {index}",
            );
        }

        let costs = gate_a_cost_snapshot();
        assert_eq!(costs.context_event_snapshot_count, 0);
        assert_eq!(costs.context_event_snapshot_items, 0);
        assert_eq!(costs.events_appended, appended as u64);
        assert!(costs.context_event_compaction_moved_items <= costs.events_appended);
    }
}
