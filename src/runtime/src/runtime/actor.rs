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