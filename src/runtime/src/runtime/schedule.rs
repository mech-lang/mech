// ---------------------------------------------------------------------------
// Scheduling methods
// ---------------------------------------------------------------------------

use super::*;

impl MechRuntime {

  pub fn enqueue_work(&mut self, work: ScheduledWork) -> MResult<()> {
    let mut context = self.runtime_context()?;
    self.enqueue_work_with_context(&mut context, work)
  }

  pub fn enqueue_work_with_context(
    &mut self,
    context: &mut RuntimeContext,
    work: ScheduledWork,
  ) -> MResult<()> {
    context.validate()?;
    context.charge_step()?;
    work.validate()?;

    self.scheduler.enqueue_work(work)?;
    self.drain_scheduler_events(context)?;

    Ok(())
  }

  pub fn enqueue_task(&mut self, task_id: TaskId) -> MResult<()> {
    self.enqueue_work(ScheduledWork::task(task_id))
  }

  pub fn enqueue_actor(&mut self, actor_id: ActorId) -> MResult<()> {
    self.enqueue_work(ScheduledWork::actor(actor_id))
  }

  pub fn collect_tick(&mut self) -> MResult<SchedulerTick> {
    let mut context = self.runtime_context()?;
    self.collect_tick_with_context(&mut context)
  }

  pub fn collect_tick_with_context(
    &mut self,
    context: &mut RuntimeContext,
  ) -> MResult<SchedulerTick> {
    context.validate()?;
    context.charge_step()?;

    let tick = collect_tick(
      self.scheduler.as_mut(),
      &self.scheduler_policy,
    )?;

    self.drain_scheduler_events(context)?;

    Ok(tick)
  }

  pub fn complete_scheduled_work(
    &mut self,
    work: ScheduledWork,
    outcome: RuntimeTurnOutcome,
  ) -> MResult<()> {
    let mut context = self.runtime_context()?;
    self.complete_scheduled_work_with_context(&mut context, work, outcome)
  }

  pub fn complete_scheduled_work_with_context(
    &mut self,
    context: &mut RuntimeContext,
    work: ScheduledWork,
    outcome: RuntimeTurnOutcome,
  ) -> MResult<()> {
    context.validate()?;
    context.charge_step()?;
    work.validate()?;

    self.scheduler.complete_work(work, outcome)?;
    self.drain_scheduler_events(context)?;

    Ok(())
  }

  pub fn fail_scheduled_work(
    &mut self,
    work: ScheduledWork,
    message: impl Into<String>,
  ) -> MResult<()> {
    let mut context = self.runtime_context()?;
    self.fail_scheduled_work_with_context(&mut context, work, message)
  }

  pub fn fail_scheduled_work_with_context(
    &mut self,
    context: &mut RuntimeContext,
    work: ScheduledWork,
    message: impl Into<String>,
  ) -> MResult<()> {
    context.validate()?;
    context.charge_step()?;
    work.validate()?;

    self.scheduler.fail_work(work, message.into())?;
    self.drain_scheduler_events(context)?;

    Ok(())
  }

  pub fn run_scheduled_work(
    &mut self,
    work: ScheduledWork,
  ) -> MResult<RuntimeTurnOutcome> {
    match work {
      ScheduledWork::Task { task_id } => self.run_scheduled_task(task_id),
      ScheduledWork::Actor { actor_id } => self.run_actor_turn(actor_id),
    }
  }

  pub fn run_scheduled_task(
    &mut self,
    task_id: TaskId,
  ) -> MResult<RuntimeTurnOutcome> {
    let Some(task) = self.store.get_task(task_id)? else {
      return Err(MechError::new(
        RuntimeRecordNotFoundError {
          record_type: "task",
          id: task_id.to_string(),
        },
        None,
      ));
    };

    let mut context = self.context_for_task(&task)?;
    self.begin_transaction(&mut context)?;

    let result = (|| -> MResult<()> {
      let Some(module_version) = task.module_version else {
        return Ok(());
      };

      self.run_module_with_context(&mut context, module_version)?;
      self.complete_task_with_context(&mut context, task_id)?;

      Ok(())
    })();

    match result {
      Ok(()) => {
        let transaction_id = self.commit_runtime_transaction(&mut context)?;

        let outcome = RuntimeTurnOutcome::new()
          .with_task(task_id)
          .with_transaction(transaction_id)
          .with_events(context.emitted_event_ids())
          .with_access(context.access.clone());

        self.complete_scheduled_work(
          ScheduledWork::task(task_id),
          outcome.clone(),
        )?;

        Ok(outcome)
      }
      Err(error) => {
        let message = format!("{:?}", error);

        self.abort_runtime_transaction(
          &mut context,
          message.clone(),
        )?;

        let _ = self.fail_task_with_context(&mut context, task_id, message.clone());

        self.fail_scheduled_work(
          ScheduledWork::task(task_id),
          message,
        )?;

        Err(error)
      }
    }
  }

  pub fn run_actor_turn(
    &mut self,
    actor_id: ActorId,
  ) -> MResult<RuntimeTurnOutcome> {
    let Some(actor) = self.store.get_actor(actor_id)? else {
      return Err(MechError::new(
        RuntimeRecordNotFoundError {
          record_type: "actor",
          id: actor_id.to_string(),
        },
        None,
      ));
    };

    let mut context = self.context_for_actor(&actor)?;
    self.begin_transaction(&mut context)?;

    let result = (|| -> MResult<Option<ActorTurn>> {
      let Some(turn) = self.next_actor_turn_with_context(
        &mut context,
        actor_id,
      )? else {
        return Ok(None);
      };

      self.run_actor_turn_envelope(&mut context, &turn)?;

      Ok(Some(turn))
    })();

    match result {
      Ok(Some(_turn)) => {
        let transaction_id = self.commit_runtime_transaction(&mut context)?;

        let outcome = RuntimeTurnOutcome::new()
          .with_actor(actor_id)
          .with_transaction(transaction_id)
          .with_events(context.emitted_event_ids())
          .with_access(context.access.clone());

        self.complete_scheduled_work(
          ScheduledWork::actor(actor_id),
          outcome.clone(),
        )?;

        Ok(outcome)
      }
      Ok(None) => {
        let transaction_id = self.commit_runtime_transaction(&mut context)?;

        let outcome = RuntimeTurnOutcome::new()
          .with_actor(actor_id)
          .with_transaction(transaction_id)
          .with_events(context.emitted_event_ids())
          .with_access(context.access.clone());

        self.complete_scheduled_work(
          ScheduledWork::actor(actor_id),
          outcome.clone(),
        )?;

        Ok(outcome)
      }
      Err(error) => {
        let message = format!("{:?}", error);

        self.emit_event_to_context(
          &mut context,
          RuntimeEventKind::ActorTurnFailed {
            actor_id,
            message: message.clone(),
          },
        )?;

        self.abort_runtime_transaction(
          &mut context,
          message.clone(),
        )?;

        self.fail_scheduled_work(
          ScheduledWork::actor(actor_id),
          message,
        )?;

        Err(error)
      }
    }
  }

  pub fn run_tick(&mut self) -> MResult<Vec<RuntimeTurnOutcome>> {
    let tick = self.collect_tick()?;
    let mut outcomes = Vec::new();

    for work in tick.work {
      if let Ok(outcome) = self.run_scheduled_work(work) {
        outcomes.push(outcome);
      }
    }

    Ok(outcomes)
  }

  fn drain_scheduler_events(
    &mut self,
    context: &mut RuntimeContext,
  ) -> MResult<()> {
    let events = self.scheduler.drain_events();

    for event in events {
      self.emit_event_to_context(context, event)?;
    }

    Ok(())
  }
}

