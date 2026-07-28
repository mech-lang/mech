use super::*;

impl MechRuntime {
  pub fn default_budget(&self) -> ResourceBudget {
    let mut budget = ResourceBudget::default();

    if let Some(max_steps) = self.config.limits.max_steps_per_turn {
      budget = budget.with_max_steps(max_steps);
    }

    if let Some(max_bytes) = self.config.limits.max_memory_bytes {
      budget = budget.with_max_bytes(max_bytes);
    }

    budget
  }

  fn known_source_bytes(source: &MechSourceCode) -> MResult<Option<u64>> {
    match source {
      MechSourceCode::String(source) | MechSourceCode::Html(source) => Ok(Some(
        u64::try_from(source.as_bytes().len()).map_err(|_| {
          MechError::new(
            ResourceBudgetExceededError {
              resource: "source_bytes",
              used: u64::MAX,
              requested: 1,
              max: None,
            },
            None,
          )
        })?,
      )),
      MechSourceCode::ByteCode(bytes) => Ok(Some(u64::try_from(bytes.len()).map_err(|_| {
        MechError::new(
          ResourceBudgetExceededError {
            resource: "source_bytes",
            used: u64::MAX,
            requested: 1,
            max: None,
          },
          None,
        )
      })?)),
      MechSourceCode::Image(_, bytes) => Ok(Some(u64::try_from(bytes.len()).map_err(|_| {
        MechError::new(
          ResourceBudgetExceededError {
            resource: "source_bytes",
            used: u64::MAX,
            requested: 1,
            max: None,
          },
          None,
        )
      })?)),
      MechSourceCode::Program(sources) => {
        let mut total = 0u64;
        for source in sources {
          let Some(bytes) = Self::known_source_bytes(source)? else {
            return Ok(None);
          };
          total = total.checked_add(bytes).ok_or_else(|| {
            MechError::new(
              ResourceBudgetExceededError {
                resource: "source_bytes",
                used: total,
                requested: bytes,
                max: None,
              },
              None,
            )
          })?;
        }
        Ok(Some(total))
      }
      MechSourceCode::Tree(_) => Ok(None),
    }
  }

  pub(in crate::runtime) fn enforce_source_limits(
    &self,
    context: &mut RuntimeContext,
    source: &MechSourceCode,
  ) -> MResult<()> {
    let Some(source_bytes) = Self::known_source_bytes(source)? else {
      return Ok(());
    };

    self.enforce_source_byte_count(context, source_bytes)
  }

  pub(in crate::runtime) fn enforce_source_byte_count(
    &self,
    context: &mut RuntimeContext,
    source_bytes: u64,
  ) -> MResult<()> {
    if let Some(max) = self.config.limits.max_source_bytes {
      if source_bytes > max {
        return Err(MechError::new(
          ResourceBudgetExceededError {
            resource: "source_bytes",
            used: 0,
            requested: source_bytes,
            max: Some(max),
          },
          None,
        ));
      }
    }

    context.charge_bytes(source_bytes)
  }

  pub(in crate::runtime) fn trim_events_to_retention(&self, events: &mut Vec<RuntimeEvent>) {
    let Some(max_events) = self.config.limits.max_in_memory_events else {
      return;
    };
    let max_events = usize::try_from(max_events).unwrap_or(usize::MAX);
    if events.len() > max_events {
      events.drain(0..(events.len() - max_events));
    }
  }

  pub(in crate::runtime) fn enforce_turn_duration(&self, started: Instant) -> MResult<()> {
    let Some(max) = self.config.limits.max_turn_duration_ms else {
      return Ok(());
    };
    let requested = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    if requested > max {
      return Err(MechError::new(
        ResourceBudgetExceededError {
          resource: "turn_duration_ms",
          used: 0,
          requested,
          max: Some(max),
        },
        None,
      ));
    }
    Ok(())
  }

  pub fn runtime_context(&self) -> MResult<RuntimeContext> {
    RuntimeContextBuilder::new(self.id)
      .budget(self.default_budget())
      .build()
  }

  pub fn context_for_task(&self, task: &TaskRecord) -> MResult<RuntimeContext> {
    let mut builder = RuntimeContextBuilder::new(self.id)
      .subject(task.subject.clone())
      .task(task.id)
      .capabilities(task.capabilities.clone())
      .budget(self.default_budget());

    if let Some(module_version) = task.module_version {
      builder = builder.module_version(module_version);
    }

    builder.build()
  }

  pub fn context_for_actor(&self, actor: &ActorRecord) -> MResult<RuntimeContext> {
    let mut builder = RuntimeContextBuilder::new(self.id)
      .subject(actor.subject.clone())
      .actor(actor.id)
      .capabilities(actor.capabilities.clone())
      .budget(self.default_budget());

    if let Some(module_version) = actor.behavior {
      builder = builder.module_version(module_version);
    }

    builder.build()
  }

  pub fn context_for_actor_turn(&self, turn: &ActorTurn) -> MResult<RuntimeContext> {
    turn.validate()?;
    let actor = self.store.get_actor(turn.actor)?.ok_or_else(|| {
      MechError::new(
        RuntimeInvalidOperationError {
          operation: "context_for_actor_turn",
          reason: format!("actor record {} was not found", turn.actor,),
        },
        None,
      )
    })?;
    if actor.subject != turn.subject || actor.behavior != turn.behavior || actor.state != turn.state
    {
      return Err(MechError::new(
        RuntimeInvalidOperationError {
          operation: "context_for_actor_turn",
          reason: format!(
            "actor turn metadata does not match actor record {}",
            turn.actor,
          ),
        },
        None,
      ));
    }

    let mut builder = RuntimeContextBuilder::new(self.id)
      .subject(actor.subject)
      .actor(actor.id)
      .actor_message(turn.message.clone())
      .capabilities(actor.capabilities)
      .budget(self.default_budget());
    if let Some(module_version) = actor.behavior {
      builder = builder.module_version(module_version);
    }
    if let Some(state) = actor.state {
      builder = builder.actor_state(state);
    }
    builder.build()
  }

  /// Build a subject context from a persisted transaction record.
  ///
  /// Transaction records are historical metadata. This context does not reopen,
  /// resume, or attach to the recorded transaction, and `transaction` remains
  /// unset.
  pub fn context_for_transaction(
    &self,
    transaction: &TransactionRecord,
  ) -> MResult<RuntimeContext> {
    RuntimeContextBuilder::new(self.id)
      .subject(transaction.subject.clone())
      .budget(self.default_budget())
      .build()
  }

  pub(in crate::runtime) fn validate_context_for_runtime(
    &self,
    context: &RuntimeContext,
  ) -> MResult<()> {
    context.validate()?;

    if context.runtime != self.id {
      return Err(MechError::new(
        RuntimeInvalidOperationError {
          operation: "validate_context_for_runtime",
          reason: format!(
            "runtime context mismatch: expected runtime {}, supplied runtime {}",
            self.id, context.runtime,
          ),
        },
        None,
      ));
    }

    if let Some(transaction_id) = context.transaction {
      let transaction = self.active_execution_transaction(transaction_id)?;

      if let Some(reason) = transaction.context_identity.mismatch_reason(context) {
        return Err(MechError::new(
          RuntimeTransactionContextMismatch {
            transaction_id,
            reason,
          },
          None,
        ));
      }
    }

    Ok(())
  }

  // ---------------------------------------------------------------------------
  // Event helpers
  // ---------------------------------------------------------------------------
}
