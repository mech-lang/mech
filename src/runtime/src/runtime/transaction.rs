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
// - `has_active_context_transaction`: Checks if the context has an active transaction.

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
    context.validate()?;
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
    context.validate()?;

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
    context.transaction = Some(id);

    let transaction = RuntimeTransaction::new(id, context.subject.clone());
    self.active_transactions.insert(id, transaction);

    self.emit_event_to_context(
      context,
      RuntimeEventKind::TransactionStarted {
        transaction_id: id,
      },
    )?;

    Ok(id)
  }

  pub fn commit_runtime_transaction(
    &mut self,
    context: &mut RuntimeContext,
  ) -> MResult<TransactionId> {
    context.validate()?;

    let transaction_id = Self::context_transaction_id(context)?;

    let mut transaction = self
      .active_transactions
      .remove(&transaction_id)
      .ok_or_else(|| {
        MechError::new(
          RuntimeTransactionNotFoundError { transaction_id },
          None,
        )
      })?;

    transaction.merge_read_set(&context.access.reads)?;
    transaction.merge_write_set(&context.access.writes)?;
    transaction.merge_events(&context.emitted_event_ids())?;

    let staged_puts: Vec<ObjectRecord> =
      transaction.staged_puts().cloned().collect();

    let staged_updates: Vec<ObjectRecord> =
      transaction.staged_updates().cloned().collect();

    let staged_task_updates: Vec<TaskRecord> =
      transaction.staged_task_updates().cloned().collect();

    let staged_actor_updates: Vec<ActorRecord> =
      transaction.staged_actor_updates().cloned().collect();

    let staged_message_acks: Vec<(ActorId, Vec<MessageId>)> = transaction
      .staged_message_acks()
      .map(|(actor, messages)| (*actor, messages.clone()))
      .collect();

    let staged_message_enqueues: Vec<(ActorId, Vec<MessageRecord>)> = transaction
      .staged_message_enqueues()
      .map(|(actor, messages)| (*actor, messages.clone()))
      .collect();    

    for object in staged_puts {
      self.store.put_object(object)?;
    }

    for object in staged_updates {
      self.store.update_object(object)?;
    }

    for task in staged_task_updates {
      self.store.update_task(task)?;
    }

    for actor in staged_actor_updates {
      self.store.update_actor(actor)?;
    }

    for (actor, messages) in staged_message_acks {
      for message in messages {
        self.store.ack_message(actor, message)?;
      }
    }

    for (actor, messages) in staged_message_enqueues {
      for message in messages {
        self.store.enqueue_message(actor, message)?;
      }
    }

    let record = transaction.into_record()?;
    let id = record.id;

    self.store.commit_transaction(record)?;

    self.emit_event_to_context(
      context,
      RuntimeEventKind::TransactionCommitted {
        transaction_id: id,
      },
    )?;

    context.transaction = None;

    Ok(id)
  }

  pub fn abort_runtime_transaction(
    &mut self,
    context: &mut RuntimeContext,
    reason: impl Into<String>,
  ) -> MResult<()> {
    context.validate()?;

    let transaction_id = Self::context_transaction_id(context)?;
    let reason = reason.into();

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

    self.emit_event_to_context(
      context,
      RuntimeEventKind::TransactionAborted {
        transaction_id,
        message: reason,
      },
    )?;

    context.transaction = None;

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

  pub(super) fn has_active_context_transaction(&self, context: &RuntimeContext) -> bool {
    context
      .transaction
      .map(|id| self.active_transactions.contains_key(&id))
      .unwrap_or(false)
  }
}