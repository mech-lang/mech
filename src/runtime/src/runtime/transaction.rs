// ---------------------------------------------------------------------------
// Transaction methods
// ---------------------------------------------------------------------------

// These methods handle the creation, retrieval, and management of transactions within the runtime. Transactions are used to group a set of operations together, allowing for atomic commits or rollbacks in case of errors. The methods:

// - `commit_transaction`: Commits a transaction record to the store and emits a TransactionCommitted event.
// - `get_transaction`: Retrieves a transaction record by its ID.
// - `list_transactions`: Lists transaction records with an optional limit.
// - `append_event`: Appends a runtime event to the store and returns its ID.
// - `get_event`: Retrieves a runtime event by its ID.
// - `list_events`: Lists runtime events with an optional limit.
// - `begin_transaction`: Starts a new transaction in the context and emits a TransactionStarted event.
// - `commit_runtime_transaction`: Commits the active transaction in the context, applying all staged changes to the store, and emits a TransactionCommitted event.
// - `abort_runtime_transaction`: Aborts the active transaction in the context with a given reason and emits a TransactionAborted event.
// - `active_transaction_mut`: Retrieves a mutable reference to an active transaction by its ID.
// - `context_transaction_id`: Retrieves the active transaction ID from the context.

use super::*;

impl MechRuntime {

  pub fn commit_transaction(
    &mut self,
    transaction: TransactionRecord,
  ) -> MResult<TransactionId> {
    let mut context = self.context_for_transaction(&transaction)?;
    self.commit_transaction_with_context(&mut context, transaction)
  }

  pub fn commit_transaction_with_context(
    &mut self,
    context: &mut RuntimeContext,
    transaction: TransactionRecord,
  ) -> MResult<TransactionId> {
    self.validate_context_for_runtime(context)?;
    context.charge_step()?;

    let id = self.store.commit_transaction(transaction)?;

    self.emit_event_to_context(
      context,
      RuntimeEventKind::TransactionCommitted {
        transaction_id: id,
      },
    )?;

    Ok(id)
  }

  pub fn get_transaction(
    &self,
    id: TransactionId,
  ) -> MResult<Option<TransactionRecord>> {
    self.store.get_transaction(id)
  }

  pub fn list_transactions(
    &self,
    limit: Option<usize>,
  ) -> MResult<Vec<TransactionRecord>> {
    self.store.list_transactions(limit)
  }

  pub fn append_event(&mut self, event: RuntimeEvent) -> MResult<EventId> {
    self.store.append_event(event)
  }

  pub fn get_event(&self, id: EventId) -> MResult<Option<RuntimeEvent>> {
    self.store.get_event(id)
  }

  pub fn list_events(&self, limit: Option<usize>) -> MResult<Vec<RuntimeEvent>> {
    self.store.list_events(limit)
  }

  pub fn begin_transaction(
    &mut self,
    context: &mut RuntimeContext,
  ) -> MResult<TransactionId> {
    self.validate_context_for_runtime(context)?;

    if context.transaction.is_some() {
      return Err(MechError::new(
        RuntimeInvalidOperationError {
          operation: "begin_transaction",
          reason: "context already has an active transaction".to_string(),
        },
        None,
      ));
    }

    let id = self.next_transaction_id();
    let transaction = RuntimeTransaction::new(id, context.subject.clone());
    self.active_transactions.insert(id, transaction);
    context.transaction = Some(id);

    let started_event = match self.emit_event_immediate_to_context(
      context,
      RuntimeEventKind::TransactionStarted {
        transaction_id: id,
      },
    ) {
      Ok(event) => event,
      Err(error) => {
        self.active_transactions.remove(&id);
        context.transaction = None;
        return Err(error);
      }
    };

    if let Err(error) = self
      .active_transaction_mut(id)?
      .record_event(started_event)
    {
      self.active_transactions.remove(&id);
      context.transaction = None;
      return Err(error);
    }

    Ok(id)
  }

  pub fn commit_runtime_transaction(
    &mut self,
    context: &mut RuntimeContext,
  ) -> MResult<TransactionId> {
    self.validate_context_for_runtime(context)?;

    let transaction_id = Self::context_transaction_id(context)?;

    let commit_event = self.make_event(RuntimeEventKind::TransactionCommitted {
      transaction_id,
    });

    let commit = {
      let transaction = self
        .active_transactions
        .get_mut(&transaction_id)
        .ok_or_else(|| {
          MechError::new(
            RuntimeTransactionNotFoundError { transaction_id },
            None,
          )
        })?;

      transaction.merge_read_set(&context.access.reads)?;
      transaction.merge_write_set(&context.access.writes)?;

      let staged_puts: Vec<ObjectRecord> =
        transaction.staged_puts().cloned().collect();

      let staged_updates: Vec<ObjectRecord> =
        transaction.staged_updates().cloned().collect();

      let staged_task_updates: Vec<TaskRecord> =
        transaction.staged_task_updates().cloned().collect();

      let staged_actor_updates: Vec<ActorRecord> =
        transaction.staged_actor_updates().cloned().collect();

      let staged_message_acks: Vec<(ActorId, MessageId)> = transaction
        .staged_message_acks()
        .flat_map(|(actor, messages)| {
          messages.iter().map(move |message| (*actor, *message))
        })
        .collect();

      let staged_message_enqueues: Vec<(ActorId, MessageRecord)> = transaction
        .staged_message_enqueues()
        .flat_map(|(actor, messages)| {
          messages.iter().cloned().map(move |message| (*actor, message))
        })
        .collect();

      let mut staged_events: Vec<RuntimeEvent> =
        transaction.staged_events().cloned().collect();
      staged_events.push(commit_event.clone());

      let mut transaction_snapshot = transaction.clone();
      transaction_snapshot.record_event(commit_event.id)?;
      let transaction_record = transaction_snapshot.into_record()?;

      RuntimeStoreCommit {
        transaction: transaction_record,
        object_puts: staged_puts,
        object_updates: staged_updates,
        task_updates: staged_task_updates,
        actor_updates: staged_actor_updates,
        message_acks: staged_message_acks,
        message_enqueues: staged_message_enqueues,
        events: staged_events,
      }
    };

    let id = self.store.commit_runtime(commit)?;

    self.active_transactions.remove(&transaction_id);
    context.transaction = None;

    self.push_persisted_event_to_context(context, commit_event);

    Ok(id)
  }

  pub fn abort_runtime_transaction(
    &mut self,
    context: &mut RuntimeContext,
    reason: impl Into<String>,
  ) -> MResult<()> {
    self.validate_context_for_runtime(context)?;

    let transaction_id = Self::context_transaction_id(context)?;
    let reason = reason.into();

    let staged_event_ids = self
      .active_transactions
      .get(&transaction_id)
      .ok_or_else(|| {
        MechError::new(
          RuntimeTransactionNotFoundError { transaction_id },
          None,
        )
      })?
      .staged_event_ids();

    let transaction = self
      .active_transactions
      .remove(&transaction_id)
      .ok_or_else(|| {
        MechError::new(
          RuntimeTransactionNotFoundError { transaction_id },
          None,
        )
      })?;

    let _ = transaction.abort(reason.clone())?;

    context
      .events
      .retain(|event| !staged_event_ids.contains(&event.id));

    context.transaction = None;

    self.emit_event_immediate_to_context(
      context,
      RuntimeEventKind::TransactionAborted {
        transaction_id,
        message: reason,
      },
    )?;

    Ok(())
  }

  pub(super) fn active_transaction_mut(
    &mut self,
    transaction_id: TransactionId,
  ) -> MResult<&mut RuntimeTransaction> {
    self
      .active_transactions
      .get_mut(&transaction_id)
      .ok_or_else(|| {
        MechError::new(
          RuntimeTransactionNotFoundError { transaction_id },
          None,
        )
      })
  }

  pub(super) fn context_transaction_id(context: &RuntimeContext) -> MResult<TransactionId> {
    context.transaction.ok_or_else(|| {
      MechError::new(
        RuntimeInvalidOperationError {
          operation: "context_transaction_id",
          reason: "context has no active transaction".to_string(),
        },
        None,
      )
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn event_count(
    events: &[RuntimeEvent],
    kind: RuntimeEventKind,
  ) -> usize {
    events.iter().filter(|event| event.kind == kind).count()
  }

  fn new_runtime() -> MechRuntime {
    MechRuntime::builder().build().unwrap()
  }

  #[test]
  fn transaction_commit_failure_is_atomic() {
    let mut runtime = new_runtime();
    let mut context = runtime.runtime_context().unwrap();

    runtime.begin_transaction(&mut context).unwrap();
    runtime
      .put_object_with_context(
        &mut context,
        ObjectRecord::text(ObjectId(100), "note", "hello"),
      )
      .unwrap();
    runtime
      .update_object_with_context(
        &mut context,
        ObjectRecord::text(ObjectId(200), "note", "missing"),
      )
      .unwrap();

    assert!(runtime.commit_runtime_transaction(&mut context).is_err());

    assert!(runtime.get_object(ObjectId(100)).unwrap().is_none());
    assert!(runtime.get_object(ObjectId(200)).unwrap().is_none());
    assert!(runtime.get_transaction(TransactionId(1)).unwrap().is_none());

    let events = runtime.list_events(None).unwrap();
    assert_eq!(
      event_count(
        &events,
        RuntimeEventKind::ObjectCreated {
          object_id: ObjectId(100),
        },
      ),
      0,
    );
    assert_eq!(
      event_count(
        &events,
        RuntimeEventKind::ObjectUpdated {
          object_id: ObjectId(200),
        },
      ),
      0,
    );
  }

  #[test]
  fn transaction_commit_failure_keeps_transaction_active() {
    let mut runtime = new_runtime();
    let mut context = runtime.runtime_context().unwrap();

    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    runtime
      .put_object_with_context(
        &mut context,
        ObjectRecord::text(ObjectId(100), "note", "hello"),
      )
      .unwrap();
    runtime
      .update_object_with_context(
        &mut context,
        ObjectRecord::text(ObjectId(200), "note", "missing"),
      )
      .unwrap();

    assert!(runtime.commit_runtime_transaction(&mut context).is_err());
    assert_eq!(context.transaction, Some(transaction_id));
    assert!(runtime.active_transactions.contains_key(&transaction_id));

    runtime
      .abort_runtime_transaction(&mut context, "failed commit")
      .unwrap();
    assert_eq!(context.transaction, None);
    assert!(!runtime.active_transactions.contains_key(&transaction_id));
  }

  #[test]
  fn transaction_abort_discards_staged_events() {
    let mut runtime = new_runtime();
    let mut context = runtime.runtime_context().unwrap();

    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    runtime
      .put_object_with_context(
        &mut context,
        ObjectRecord::text(ObjectId(100), "note", "hello"),
      )
      .unwrap();

    let staged_event_id = context
      .events
      .iter()
      .find(|event| {
        event.kind == (RuntimeEventKind::ObjectCreated {
          object_id: ObjectId(100),
        })
      })
      .map(|event| event.id)
      .unwrap();

    runtime
      .abort_runtime_transaction(&mut context, "abort")
      .unwrap();

    assert!(!context.events.iter().any(|event| event.id == staged_event_id));
    assert!(runtime.get_event(staged_event_id).unwrap().is_none());
    assert!(runtime.get_object(ObjectId(100)).unwrap().is_none());
    assert!(runtime.get_transaction(transaction_id).unwrap().is_none());

    let events = runtime.list_events(None).unwrap();
    assert_eq!(
      event_count(
        &events,
        RuntimeEventKind::TransactionStarted { transaction_id },
      ),
      1,
    );
    assert_eq!(
      event_count(
        &events,
        RuntimeEventKind::TransactionAborted {
          transaction_id,
          message: "abort".to_string(),
        },
      ),
      1,
    );
  }

  #[test]
  fn transaction_commit_persists_staged_events_once() {
    let mut runtime = new_runtime();
    let mut context = runtime.runtime_context().unwrap();

    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    let started_id = context
      .events
      .iter()
      .find(|event| {
        event.kind == (RuntimeEventKind::TransactionStarted { transaction_id })
      })
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
          RuntimeEventKind::ObjectCreated { .. }
            | RuntimeEventKind::ObjectUpdated { .. }
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
      event_count(
        &events,
        RuntimeEventKind::ObjectCreated {
          object_id: ObjectId(100),
        },
      ),
      1,
    );
    assert_eq!(
      event_count(
        &events,
        RuntimeEventKind::ObjectUpdated {
          object_id: ObjectId(100),
        },
      ),
      1,
    );
    assert_eq!(
      event_count(
        &events,
        RuntimeEventKind::TransactionCommitted { transaction_id },
      ),
      1,
    );
    let commit_event_id = context
      .events
      .iter()
      .find(|event| {
        event.kind == (RuntimeEventKind::TransactionCommitted { transaction_id })
      })
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
  }
  #[test]
  fn rejects_foreign_runtime_context_before_object_write_and_events() {
    let runtime_a = new_runtime();
    let mut runtime_b = new_runtime();
    let mut context = runtime_a.runtime_context().unwrap();
    let events_before = runtime_b.list_events(None).unwrap();

    assert!(runtime_b
      .put_object_with_context(
        &mut context,
        ObjectRecord::text(ObjectId(900), "note", "foreign"),
      )
      .is_err());

    assert!(runtime_b.get_object(ObjectId(900)).unwrap().is_none());
    assert_eq!(runtime_b.list_events(None).unwrap(), events_before);
    assert!(context.events.is_empty());
  }

  #[test]
  fn nonexistent_transaction_context_does_not_fall_through_to_durable_writes() {
    let mut runtime = new_runtime();
    runtime.put_actor(ActorRecord::new(ActorId(1), "actor:1")).unwrap();
    let mut context = runtime.runtime_context().unwrap();
    context.transaction = Some(TransactionId(404));
    let events_before = runtime.list_events(None).unwrap();

    assert!(runtime
      .put_object_with_context(
        &mut context,
        ObjectRecord::text(ObjectId(901), "note", "missing-tx"),
      )
      .is_err());
    assert!(runtime
      .send_message_with_context(&mut context, ActorId(1), "ping", b"missing-tx".to_vec())
      .is_err());

    assert!(runtime.get_object(ObjectId(901)).unwrap().is_none());
    assert!(runtime.pop_message(ActorId(1)).unwrap().is_none());
    assert_eq!(runtime.list_events(None).unwrap(), events_before);
    assert!(context.events.is_empty());
  }

  #[test]
  fn transaction_subject_mismatch_cannot_stage_commit_or_abort_owner_can_finish() {
    let mut runtime = new_runtime();
    runtime.put_actor(ActorRecord::new(ActorId(1), "owner")).unwrap();
    let mut owner_context = runtime.runtime_context().unwrap();
    owner_context.subject = "owner".to_string();
    let transaction_id = runtime.begin_transaction(&mut owner_context).unwrap();
    let events_after_begin = runtime.list_events(None).unwrap();

    let mut other_context = runtime.runtime_context().unwrap();
    other_context.subject = "other".to_string();
    other_context.transaction = Some(transaction_id);

    assert!(runtime
      .put_object_with_context(
        &mut other_context,
        ObjectRecord::text(ObjectId(902), "note", "wrong-owner"),
      )
      .is_err());
    assert!(runtime
      .send_message_with_context(&mut other_context, ActorId(1), "ping", b"wrong-owner".to_vec())
      .is_err());
    assert!(runtime.commit_runtime_transaction(&mut other_context).is_err());
    assert!(runtime.abort_runtime_transaction(&mut other_context, "wrong-owner").is_err());

    assert!(runtime.active_transactions.contains_key(&transaction_id));
    assert!(runtime.get_object(ObjectId(902)).unwrap().is_none());
    assert!(runtime.pop_message(ActorId(1)).unwrap().is_none());
    assert_eq!(runtime.list_events(None).unwrap(), events_after_begin);
    assert!(other_context.events.is_empty());

    assert_eq!(runtime.commit_runtime_transaction(&mut owner_context).unwrap(), transaction_id);
    assert!(!runtime.active_transactions.contains_key(&transaction_id));
  }

  #[test]
  fn stale_aborted_transaction_context_is_rejected_not_durable() {
    let mut runtime = new_runtime();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    let mut stale_context = context.clone();
    runtime.abort_runtime_transaction(&mut context, "rollback").unwrap();
    let events_after_abort = runtime.list_events(None).unwrap();

    assert!(runtime
      .put_object_with_context(
        &mut stale_context,
        ObjectRecord::text(ObjectId(903), "note", "stale"),
      )
      .is_err());

    assert_eq!(stale_context.transaction, Some(transaction_id));
    assert!(runtime.get_object(ObjectId(903)).unwrap().is_none());
    assert_eq!(runtime.list_events(None).unwrap(), events_after_abort);
  }

  #[test]
  fn foreign_context_rejected_before_host_and_capability_boundaries() {
    let runtime_a = new_runtime();
    let mut runtime_b = new_runtime();
    let mut context = runtime_a.runtime_context().unwrap();
    let events_before = runtime_b.list_events(None).unwrap();

    assert!(runtime_b
      .call_host_with_context(&mut context, HostCall::new("missing/host", Vec::new()))
      .is_err());
    assert!(runtime_b
      .check_capability_with_context(
        &mut context,
        &CapabilityRequest::from_keys("subject", "op", "resource"),
      )
      .is_err());

    assert_eq!(runtime_b.list_events(None).unwrap(), events_before);
    assert!(context.events.is_empty());
  }

  #[test]
  fn historical_transaction_record_context_is_valid_without_active_transaction() {
    let mut runtime = new_runtime();
    let mut context = runtime.runtime_context().unwrap();
    context.subject = "historical-owner".to_string();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    runtime.commit_runtime_transaction(&mut context).unwrap();

    let record = runtime.get_transaction(transaction_id).unwrap().unwrap();
    let mut record_context = runtime.context_for_transaction(&record).unwrap();

    assert_eq!(record_context.runtime, runtime.id);
    assert_eq!(record_context.subject, record.subject);
    assert_eq!(record_context.transaction, None);
    assert!(runtime.get_object_with_context(&mut record_context, ObjectId(404)).unwrap().is_none());
    assert!(!runtime.active_transactions.contains_key(&transaction_id));
  }

  #[test]
  fn active_transaction_context_is_valid_and_can_finish_transaction() {
    let mut runtime = new_runtime();
    let mut owner_context = runtime.runtime_context().unwrap();
    owner_context.subject = "active-owner".to_string();
    let transaction_id = runtime.begin_transaction(&mut owner_context).unwrap();

    let mut active_context = runtime
      .context_for_active_transaction(transaction_id)
      .unwrap();

    assert_eq!(active_context.subject, "active-owner");
    assert_eq!(active_context.transaction, Some(transaction_id));
    runtime
      .put_object_with_context(
        &mut active_context,
        ObjectRecord::text(ObjectId(904), "note", "active"),
      )
      .unwrap();
    assert!(runtime.get_object(ObjectId(904)).unwrap().is_none());

    assert_eq!(
      runtime.commit_runtime_transaction(&mut active_context).unwrap(),
      transaction_id,
    );
    assert!(runtime.get_object(ObjectId(904)).unwrap().is_some());
  }

  #[test]
  fn missing_active_transaction_context_is_rejected_immediately() {
    let runtime = new_runtime();
    let error = runtime
      .context_for_active_transaction(TransactionId(404))
      .unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeTransactionNotFound");
  }

}
