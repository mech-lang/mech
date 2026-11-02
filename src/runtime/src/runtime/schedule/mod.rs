// ---------------------------------------------------------------------------
// Scheduling methods
// ---------------------------------------------------------------------------

// These methods manage the scheduling of work within the runtime, allowing for enqueuing tasks and actors for execution, collecting scheduled work for execution in ticks, and marking scheduled work as complete or failed. The scheduling system is responsible for determining the order of execution for tasks and actors based on the defined scheduling policy, ensuring that work is executed efficiently and fairly across the runtime.

// The mehods include:

// - `enqueue_work`: Adds a piece of scheduled work (either a task or an actor turn) to the scheduler's queue and emits any resulting events.
// - `collect_tick`: Collects a batch of scheduled work from the scheduler according to the scheduling policy, returning a SchedulerTick that contains the work to be executed and any associated metadata.
// - `complete_scheduled_work`: Marks a piece of scheduled work as completed with a given outcome, allowing the scheduler to update its state and emit any resulting events.
// - `fail_scheduled_work`: Marks a piece of scheduled work as failed with a given message, allowing the scheduler to update its state and emit any resulting events.
use crate::runtime::{MechRuntime, extension};
use crate::scheduler::collect_tick;
use crate::{ActorId, RuntimeContext, RuntimeTurnOutcome, ScheduledWork, SchedulerTick, TaskId};
use mech_core::MResult;

impl MechRuntime {
    pub fn enqueue_work(&mut self, work: ScheduledWork) -> MResult<()> {
        self.ensure_runtime_mutation_allowed("enqueue_work")?;
        let mut context = self.runtime_context()?;
        self.enqueue_work_with_context(&mut context, work)
    }

    pub fn enqueue_work_with_context(
        &mut self,
        context: &mut RuntimeContext,
        work: ScheduledWork,
    ) -> MResult<()> {
        self.ensure_runtime_mutation_allowed("enqueue_work_with_context")?;
        self.validate_context_for_runtime(context)?;
        context.charge_step()?;
        work.validate()?;

        extension::invoke_extension("scheduler", "enqueue_work", || {
            self.scheduler.enqueue_work(work)
        })?;
        self.drain_scheduler_events(context)?;

        Ok(())
    }

    pub fn enqueue_task(&mut self, task_id: TaskId) -> MResult<()> {
        self.ensure_runtime_mutation_allowed("enqueue_task")?;
        self.enqueue_work(ScheduledWork::task(task_id))
    }

    pub fn enqueue_actor(&mut self, actor_id: ActorId) -> MResult<()> {
        self.ensure_runtime_mutation_allowed("enqueue_actor")?;
        self.enqueue_work(ScheduledWork::actor(actor_id))
    }

    pub fn collect_tick(&mut self) -> MResult<SchedulerTick> {
        self.ensure_runtime_mutation_allowed("collect_tick")?;
        let mut context = self.runtime_context()?;
        self.collect_tick_with_context(&mut context)
    }

    pub fn collect_tick_with_context(
        &mut self,
        context: &mut RuntimeContext,
    ) -> MResult<SchedulerTick> {
        self.ensure_runtime_mutation_allowed("collect_tick_with_context")?;
        self.validate_context_for_runtime(context)?;
        context.charge_step()?;

        let tick = extension::invoke_extension("scheduler", "collect_tick", || {
            collect_tick(self.scheduler.as_mut(), &self.scheduler_policy)
        })?;

        self.drain_scheduler_events(context)?;

        Ok(tick)
    }

    pub fn complete_scheduled_work(
        &mut self,
        work: ScheduledWork,
        outcome: RuntimeTurnOutcome,
    ) -> MResult<()> {
        self.ensure_runtime_mutation_allowed("complete_scheduled_work")?;
        let mut context = self.runtime_context()?;
        self.complete_scheduled_work_with_context(&mut context, work, outcome)
    }

    pub fn complete_scheduled_work_with_context(
        &mut self,
        context: &mut RuntimeContext,
        work: ScheduledWork,
        outcome: RuntimeTurnOutcome,
    ) -> MResult<()> {
        self.ensure_runtime_mutation_allowed("complete_scheduled_work_with_context")?;
        self.validate_context_for_runtime(context)?;
        context.charge_step()?;
        work.validate()?;

        extension::invoke_extension("scheduler", "complete_work", || {
            self.scheduler.complete_work(work, outcome)
        })?;
        self.drain_scheduler_events(context)?;

        Ok(())
    }

    pub fn fail_scheduled_work(
        &mut self,
        work: ScheduledWork,
        message: impl Into<String>,
    ) -> MResult<()> {
        self.ensure_runtime_mutation_allowed("fail_scheduled_work")?;
        let mut context = self.runtime_context()?;
        self.fail_scheduled_work_with_context(&mut context, work, message)
    }

    pub fn fail_scheduled_work_with_context(
        &mut self,
        context: &mut RuntimeContext,
        work: ScheduledWork,
        message: impl Into<String>,
    ) -> MResult<()> {
        self.ensure_runtime_mutation_allowed("fail_scheduled_work_with_context")?;
        self.validate_context_for_runtime(context)?;
        context.charge_step()?;
        work.validate()?;

        let message = message.into();
        extension::invoke_extension("scheduler", "fail_work", || {
            self.scheduler.fail_work(work, message)
        })?;
        self.drain_scheduler_events(context)?;

        Ok(())
    }

    fn drain_scheduler_events(&mut self, context: &mut RuntimeContext) -> MResult<()> {
        let events = extension::invoke_extension_value("scheduler", "drain_events", || {
            self.scheduler.drain_events()
        })?;

        for event in events {
            self.emit_event_to_context(context, event)?;
        }

        Ok(())
    }
}

#[cfg(test)]
#[path = "tests/mod.rs"]
mod panic_tests;
