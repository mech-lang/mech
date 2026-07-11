// ---------------------------------------------------------------------------
// Actor methods
// ---------------------------------------------------------------------------

// Actors are the primary entities in the Mech runtime that encapsulate state and behavior. They can receive messages, execute turns, and interact with other actors. The methods in this section allow you to create, retrieve, update, and manage actors, as well as send messages to them and run their turns.

use super::*;

impl MechRuntime {

  pub fn put_actor(&mut self, actor: ActorRecord) -> MResult<ActorId> {
    let mut context = self.context_for_actor(&actor)?;
    self.put_actor_with_context(&mut context, actor)
  }

  pub fn put_actor_with_context(
    &mut self,
    context: &mut RuntimeContext,
    actor: ActorRecord,
  ) -> MResult<ActorId> {
    context.validate()?;
    context.charge_step()?;

    if self.store.get_actor(actor.id)?.is_none() {
      if let Some(max) = self.config.limits.max_actors {
        let used = self.store.actor_count()?;
        if used.saturating_add(1) > max {
          return Err(MechError::new(
            ResourceBudgetExceededError {
              resource: "actors",
              used,
              requested: 1,
              max: Some(max),
            },
            None,
          ));
        }
      }
    }

    let id = self.store.put_actor(actor)?;

    self.emit_event_to_context(
      context,
      RuntimeEventKind::ActorCreated {
        actor_id: id,
      },
    )?;

    Ok(id)
  }

  pub fn create_actor(
    &mut self,
    subject: impl Into<String>,
    behavior: Option<ModuleVersionId>,
    state: Option<ObjectId>,
    capabilities: Vec<CapabilityId>,
  ) -> MResult<ActorId> {
    let id = self.next_actor_id();

    let mut actor = ActorRecord::new(id, subject)
      .with_capabilities(capabilities);

    if let Some(behavior) = behavior {
      actor = actor.with_behavior(behavior);
    }

    if let Some(state) = state {
      actor = actor.with_state(state);
    }

    self.put_actor(actor)
  }

  pub fn get_actor(&self, id: ActorId) -> MResult<Option<ActorRecord>> {
    self.store.get_actor(id)
  }

  pub fn get_actor_with_context(
    &mut self,
    context: &mut RuntimeContext,
    id: ActorId,
  ) -> MResult<Option<ActorRecord>> {
    context.validate()?;

    if let Some(transaction_id) = context.transaction {
      if let Some(transaction) = self.active_transactions.get(&transaction_id) {
        if let Some(actor) = transaction.get_staged_actor(id) {
          return Ok(Some(actor));
        }
      }
    }

    self.store.get_actor(id)
  }

  pub fn update_actor(&mut self, actor: ActorRecord) -> MResult<ActorId> {
    self.store.update_actor(actor)
  }

  pub fn update_actor_with_context(
    &mut self,
    context: &mut RuntimeContext,
    actor: ActorRecord,
  ) -> MResult<ActorId> {
    context.validate()?;

    if self.has_active_context_transaction(context) {
      let transaction_id = Self::context_transaction_id(context)?;
      let id = actor.id;

      self
        .active_transaction_mut(transaction_id)?
        .stage_actor_update(actor)?;

      return Ok(id);
    }

    self.store.update_actor(actor)
  }

  pub fn send_message(
    &mut self,
    actor: ActorId,
    kind: impl Into<String>,
    payload: Vec<u8>,
  ) -> MResult<MessageId> {
    let Some(actor_record) = self.store.get_actor(actor)? else {
      return Err(MechError::new(
        RuntimeRecordNotFoundError {
          record_type: "actor",
          id: actor.to_string(),
        },
        None,
      ));
    };

    let mut context = self.context_for_actor(&actor_record)?;
    self.send_message_with_context(&mut context, actor, kind, payload)
  }

  pub fn send_message_with_context(
    &mut self,
    context: &mut RuntimeContext,
    actor: ActorId,
    kind: impl Into<String>,
    payload: Vec<u8>,
  ) -> MResult<MessageId> {
    context.validate()?;
    context.charge_messages(1)?;
    context.charge_bytes(payload.len() as u64)?;

    self.enforce_actor_mailbox_limit(context, actor)?;

    let id = self.next_message_id();
    let message = MessageRecord::new(id, actor, kind, payload);

    if self.has_active_context_transaction(context) {
      let transaction_id = Self::context_transaction_id(context)?;

      self
        .active_transaction_mut(transaction_id)?
        .stage_message_enqueue(actor, message)?;

      self.emit_event_to_context(
        context,
        RuntimeEventKind::ActorMessageSent {
          actor_id: actor,
          message_id: id,
        },
      )?;

      return Ok(id);
    }

    self.store.enqueue_message(actor, message)?;

    self.emit_event_to_context(
      context,
      RuntimeEventKind::ActorMessageSent {
        actor_id: actor,
        message_id: id,
      },
    )?;

    Ok(id)
  }

  fn enforce_actor_mailbox_limit(
    &self,
    context: &RuntimeContext,
    actor: ActorId,
  ) -> MResult<()> {
    let Some(max) = self.config.limits.max_actor_mailbox_len else {
      return Ok(());
    };

    let durable_len = self.store.mailbox_len(actor)?;
    let mut effective_len = durable_len;

    if let Some(transaction_id) = context.transaction {
      if let Some(transaction) = self.active_transactions.get(&transaction_id) {
        let ack_count = transaction.staged_message_ack_count(actor);
        effective_len = effective_len.checked_sub(ack_count).ok_or_else(|| {
          MechError::new(
            RuntimeInvalidOperationError {
              operation: "send_message",
              reason: "staged message acknowledgements exceed durable mailbox length".to_string(),
            },
            None,
          )
        })?;
        effective_len = effective_len
          .checked_add(transaction.staged_message_enqueue_count(actor))
          .ok_or_else(|| {
            MechError::new(
              ResourceBudgetExceededError {
                resource: "actor_mailbox",
                used: effective_len,
                requested: 1,
                max: None,
              },
              None,
            )
          })?;
      }
    }

    let next_len = effective_len.checked_add(1).ok_or_else(|| {
      MechError::new(
        ResourceBudgetExceededError {
          resource: "actor_mailbox",
          used: effective_len,
          requested: 1,
          max: None,
        },
        None,
      )
    })?;

    if next_len > max {
      return Err(MechError::new(
        ResourceBudgetExceededError {
          resource: "actor_mailbox",
          used: effective_len,
          requested: 1,
          max: Some(max),
        },
        None,
      ));
    }

    Ok(())
  }

  pub fn pop_message(&mut self, actor: ActorId) -> MResult<Option<MessageRecord>> {
    self.store.pop_message(actor)
  }

  pub fn pop_message_with_context(
    &mut self,
    context: &mut RuntimeContext,
    actor: ActorId,
  ) -> MResult<Option<MessageRecord>> {
    context.validate()?;

    if self.has_active_context_transaction(context) {
      let transaction_id = Self::context_transaction_id(context)?;

      if let Some(message) = self
        .active_transaction_mut(transaction_id)?
        .pop_staged_enqueued_message(actor)
      {
        return Ok(Some(message));
      }

      let Some(message) = self.store.peek_message(actor)? else {
        return Ok(None);
      };

      self
        .active_transaction_mut(transaction_id)?
        .stage_message_ack(actor, message.id)?;

      return Ok(Some(message));
    }

    self.store.pop_message(actor)
  }

  pub fn peek_message(&self, actor: ActorId) -> MResult<Option<MessageRecord>> {
    self.store.peek_message(actor)
  }

  pub fn peek_message_with_context(
    &mut self,
    context: &mut RuntimeContext,
    actor: ActorId,
  ) -> MResult<Option<MessageRecord>> {
    context.validate()?;

    if let Some(transaction_id) = context.transaction {
      if let Some(transaction) = self.active_transactions.get(&transaction_id) {
        if let Some(message) = transaction.peek_staged_enqueued_message(actor) {
          return Ok(Some(message));
        }
      }
    }

    self.store.peek_message(actor)
  }

  pub fn next_actor_turn_with_context(
    &mut self,
    context: &mut RuntimeContext,
    actor_id: ActorId,
  ) -> MResult<Option<ActorTurn>> {
    context.validate()?;

    let Some(actor) = self.get_actor_with_context(context, actor_id)? else {
      return Err(MechError::new(
        RuntimeRecordNotFoundError {
          record_type: "actor",
          id: actor_id.to_string(),
        },
        None,
      ));
    };

    let Some(message) = self.pop_message_with_context(context, actor_id)? else {
      return Ok(None);
    };

    Ok(Some(ActorTurn::new(actor, message)?))
  }

  pub fn run_actor_turn_envelope(
    &mut self,
    context: &mut RuntimeContext,
    turn: &ActorTurn,
  ) -> MResult<()> {
    context.validate()?;
    turn.validate()?;

    context.bind_actor_turn(turn);

    self.emit_event_to_context(
      context,
      RuntimeEventKind::ActorTurnStarted {
        actor_id: turn.actor_id(),
      },
    )?;

    if let Some(behavior) = turn.behavior {
      self.run_module_with_context(context, behavior)?;
    }

    let mut driver = std::mem::replace(
      &mut self.actor_behavior_driver,
      Box::new(NoActorBehaviorDriver::new()),
    );

    let driver_result = driver.run_actor_turn(self, context, turn);

    self.actor_behavior_driver = driver;

    driver_result?;

    self.emit_event_to_context(
      context,
      RuntimeEventKind::ActorTurnCompleted {
        actor_id: turn.actor_id(),
      },
    )?;

    Ok(())
  }
}

impl ActorBehaviorRuntime for MechRuntime {
  fn call_host_with_context(
    &mut self,
    context: &mut RuntimeContext,
    call: HostCall,
  ) -> MResult<Value> {
    MechRuntime::call_host_with_context(self, context, call)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

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
      .send_message_with_context(
        &mut context,
        ActorId(1),
        "ping",
        b"three".to_vec(),
      )
      .unwrap();

    let error = runtime
      .send_message_with_context(
        &mut context,
        ActorId(1),
        "ping",
        b"four".to_vec(),
      )
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
}
