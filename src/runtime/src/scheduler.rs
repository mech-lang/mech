//! Runtime scheduler.
//!
//! The scheduler decides what work should run next.
//!
//! It does not own the interpreter, store, capability kernel, source resolver,
//! host registry, or runtime itself. Those stay owned by `MechRuntime`.
//!
//! This module owns scheduling policy only:
//!
//! - queued tasks
//! - queued actors
//! - FIFO turn selection
//! - optional deduplication of queued work
//! - completed turn outcomes
//! - failed work records
//! - scheduler event intents
//! - tick collection
//!
//! The scheduler emits `RuntimeEventKind`, not `RuntimeEvent`. The runtime owns
//! EventId generation, sequencing, storage, and event persistence.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use std::collections::{HashSet, VecDeque};

use mech_core::{MResult, MechError, MechErrorKind};

use crate::context::RuntimeTurnOutcome;
use crate::event::RuntimeEventKind;
use crate::id::{ActorId, TaskId};

// -----------------------------------------------------------------------------
// Scheduled Work
// -----------------------------------------------------------------------------

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScheduledWork {
  Task {
    task_id: TaskId,
  },

  Actor {
    actor_id: ActorId,
  },
}

impl ScheduledWork {
  pub fn task(task_id: TaskId) -> Self {
    Self::Task { task_id }
  }

  pub fn actor(actor_id: ActorId) -> Self {
    Self::Actor { actor_id }
  }

  pub fn validate(&self) -> MResult<()> {
    match self {
      ScheduledWork::Task { task_id } => {
        if task_id.is_zero() {
          return invalid_scheduled_work("task_id", "must not be zero");
        }
      }
      ScheduledWork::Actor { actor_id } => {
        if actor_id.is_zero() {
          return invalid_scheduled_work("actor_id", "must not be zero");
        }
      }
    }

    Ok(())
  }

  pub fn label(&self) -> String {
    match self {
      ScheduledWork::Task { task_id } => format!("task:{}", task_id),
      ScheduledWork::Actor { actor_id } => format!("actor:{}", actor_id),
    }
  }

  pub fn is_task(&self) -> bool {
    matches!(self, ScheduledWork::Task { .. })
  }

  pub fn is_actor(&self) -> bool {
    matches!(self, ScheduledWork::Actor { .. })
  }

  pub fn queued_event(&self) -> RuntimeEventKind {
    RuntimeEventKind::SchedulerWorkQueued {
      work: self.label(),
    }
  }

  pub fn started_event(&self) -> RuntimeEventKind {
    RuntimeEventKind::SchedulerWorkStarted {
      work: self.label(),
    }
  }

  pub fn completed_event(&self) -> RuntimeEventKind {
    RuntimeEventKind::SchedulerWorkCompleted {
      work: self.label(),
    }
  }

  pub fn failed_event(&self, message: impl Into<String>) -> RuntimeEventKind {
    RuntimeEventKind::SchedulerWorkFailed {
      work: self.label(),
      message: message.into(),
    }
  }
}

// -----------------------------------------------------------------------------
// Scheduler Trait
// -----------------------------------------------------------------------------

/// Scheduling policy interface.
///
/// This trait is object-safe. Do not add generic methods here if the runtime
/// needs `Box<dyn Scheduler>`.
pub trait Scheduler: std::fmt::Debug + Send {
  fn enqueue_work(&mut self, work: ScheduledWork) -> MResult<()>;

  fn enqueue_task(&mut self, task_id: TaskId) -> MResult<()> {
    self.enqueue_work(ScheduledWork::task(task_id))
  }

  fn enqueue_actor(&mut self, actor_id: ActorId) -> MResult<()> {
    self.enqueue_work(ScheduledWork::actor(actor_id))
  }

  fn next_work(&mut self) -> MResult<Option<ScheduledWork>>;

  fn complete_work(
    &mut self,
    work: ScheduledWork,
    outcome: RuntimeTurnOutcome,
  ) -> MResult<()>;

  fn fail_work(
    &mut self,
    work: ScheduledWork,
    message: String,
  ) -> MResult<()>;

  fn begin_tick(&mut self) -> MResult<()>;

  fn complete_tick(&mut self, work_count: u64) -> MResult<()>;

  fn len(&self) -> usize;

  fn is_empty(&self) -> bool {
    self.len() == 0
  }

  fn queued_work(&self) -> Vec<ScheduledWork>;

  fn completed(&self) -> &[ScheduledWorkOutcome];

  fn failures(&self) -> &[ScheduledWorkFailure];

  fn pending_events(&self) -> &[RuntimeEventKind];

  fn drain_events(&mut self) -> Vec<RuntimeEventKind>;
}

// -----------------------------------------------------------------------------
// In-Memory FIFO Scheduler
// -----------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct InMemoryScheduler {
  queue: VecDeque<ScheduledWork>,
  queued: HashSet<ScheduledWork>,
  completed: Vec<ScheduledWorkOutcome>,
  failures: Vec<ScheduledWorkFailure>,
  pending_events: Vec<RuntimeEventKind>,
  deduplicate: bool,
}

impl Default for InMemoryScheduler {
  fn default() -> Self {
    Self {
      queue: VecDeque::new(),
      queued: HashSet::new(),
      completed: Vec::new(),
      failures: Vec::new(),
      pending_events: Vec::new(),
      deduplicate: true,
    }
  }
}

impl InMemoryScheduler {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn without_deduplication() -> Self {
    Self {
      deduplicate: false,
      ..Self::default()
    }
  }

  pub fn set_deduplicate(&mut self, deduplicate: bool) {
    self.deduplicate = deduplicate;

    if !deduplicate {
      self.queued.clear();
    }
  }

  pub fn deduplicate(&self) -> bool {
    self.deduplicate
  }

  pub fn clear(&mut self) {
    self.queue.clear();
    self.queued.clear();
    self.completed.clear();
    self.failures.clear();
    self.pending_events.clear();
  }

  pub fn contains(&self, work: ScheduledWork) -> bool {
    if self.deduplicate {
      self.queued.contains(&work)
    } else {
      self.queue.iter().any(|queued| *queued == work)
    }
  }

  fn push_event(&mut self, event: RuntimeEventKind) {
    self.pending_events.push(event);
  }
}

impl Scheduler for InMemoryScheduler {
  fn enqueue_work(&mut self, work: ScheduledWork) -> MResult<()> {
    work.validate()?;

    if self.deduplicate && self.queued.contains(&work) {
      return Ok(());
    }

    self.queue.push_back(work);

    if self.deduplicate {
      self.queued.insert(work);
    }

    self.push_event(work.queued_event());

    Ok(())
  }

  fn next_work(&mut self) -> MResult<Option<ScheduledWork>> {
    let Some(work) = self.queue.pop_front() else {
      return Ok(None);
    };

    if self.deduplicate {
      self.queued.remove(&work);
    }

    self.push_event(work.started_event());

    Ok(Some(work))
  }

  fn complete_work(
    &mut self,
    work: ScheduledWork,
    outcome: RuntimeTurnOutcome,
  ) -> MResult<()> {
    work.validate()?;

    self.completed.push(ScheduledWorkOutcome {
      work,
      outcome,
    });

    self.push_event(work.completed_event());

    Ok(())
  }

  fn fail_work(
    &mut self,
    work: ScheduledWork,
    message: String,
  ) -> MResult<()> {
    work.validate()?;

    self.failures.push(ScheduledWorkFailure {
      work,
      message: message.clone(),
    });

    self.push_event(work.failed_event(message));

    Ok(())
  }

  fn begin_tick(&mut self) -> MResult<()> {
    self.push_event(RuntimeEventKind::RuntimeTickStarted);
    Ok(())
  }

  fn complete_tick(&mut self, work_count: u64) -> MResult<()> {
    self.push_event(RuntimeEventKind::RuntimeTickCompleted {
      work_count,
    });

    Ok(())
  }

  fn len(&self) -> usize {
    self.queue.len()
  }

  fn queued_work(&self) -> Vec<ScheduledWork> {
    self.queue.iter().copied().collect()
  }

  fn completed(&self) -> &[ScheduledWorkOutcome] {
    &self.completed
  }

  fn failures(&self) -> &[ScheduledWorkFailure] {
    &self.failures
  }

  fn pending_events(&self) -> &[RuntimeEventKind] {
    &self.pending_events
  }

  fn drain_events(&mut self) -> Vec<RuntimeEventKind> {
    std::mem::take(&mut self.pending_events)
  }
}

// -----------------------------------------------------------------------------
// Scheduler Policy
// -----------------------------------------------------------------------------

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SchedulerPolicy {
  pub max_turns_per_tick: Option<u64>,
  pub requeue_actor_after_message: bool,
  pub deduplicate_queued_work: bool,
}

impl Default for SchedulerPolicy {
  fn default() -> Self {
    Self {
      max_turns_per_tick: Some(1),
      requeue_actor_after_message: true,
      deduplicate_queued_work: true,
    }
  }
}

impl SchedulerPolicy {
  pub fn unbounded() -> Self {
    Self {
      max_turns_per_tick: None,
      requeue_actor_after_message: true,
      deduplicate_queued_work: true,
    }
  }

  pub fn with_max_turns_per_tick(mut self, max_turns: u64) -> Self {
    self.max_turns_per_tick = Some(max_turns);
    self
  }

  pub fn without_turn_limit(mut self) -> Self {
    self.max_turns_per_tick = None;
    self
  }

  pub fn requeue_actor_after_message(mut self, enabled: bool) -> Self {
    self.requeue_actor_after_message = enabled;
    self
  }

  pub fn deduplicate_queued_work(mut self, enabled: bool) -> Self {
    self.deduplicate_queued_work = enabled;
    self
  }

  pub fn validate(&self) -> MResult<()> {
    if let Some(max_turns) = self.max_turns_per_tick {
      if max_turns == 0 {
        return Err(MechError::new(
          InvalidSchedulerPolicyError {
            field: "max_turns_per_tick",
            reason: "must be greater than zero when present",
          },
          None,
        ));
      }
    }

    Ok(())
  }
}

// -----------------------------------------------------------------------------
// Scheduler Tick
// -----------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SchedulerTick {
  pub work: Vec<ScheduledWork>,
}

impl SchedulerTick {
  pub fn new(work: Vec<ScheduledWork>) -> Self {
    Self { work }
  }

  pub fn is_empty(&self) -> bool {
    self.work.is_empty()
  }

  pub fn len(&self) -> usize {
    self.work.len()
  }
}

/// Pull up to the policy's allowed number of work items from a scheduler.
///
/// This helper does not execute work. Runtime code should execute each returned
/// item and then report completion or failure back to the scheduler.
///
/// The scheduler records event intents for tick start, each work-start, and
/// tick completion. The runtime should drain and persist those events.
pub fn collect_tick(
  scheduler: &mut dyn Scheduler,
  policy: &SchedulerPolicy,
) -> MResult<SchedulerTick> {
  policy.validate()?;

  scheduler.begin_tick()?;

  let limit = policy.max_turns_per_tick.unwrap_or(u64::MAX);
  let mut work = Vec::new();

  for _ in 0..limit {
    let Some(item) = scheduler.next_work()? else {
      break;
    };

    work.push(item);
  }

  scheduler.complete_tick(work.len() as u64)?;

  Ok(SchedulerTick::new(work))
}

// -----------------------------------------------------------------------------
// Scheduler Results
// -----------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScheduledWorkOutcome {
  pub work: ScheduledWork,
  pub outcome: RuntimeTurnOutcome,
}

impl ScheduledWorkOutcome {
  pub fn new(work: ScheduledWork, outcome: RuntimeTurnOutcome) -> Self {
    Self { work, outcome }
  }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScheduledWorkFailure {
  pub work: ScheduledWork,
  pub message: String,
}

impl ScheduledWorkFailure {
  pub fn new(work: ScheduledWork, message: impl Into<String>) -> Self {
    Self {
      work,
      message: message.into(),
    }
  }
}

// -----------------------------------------------------------------------------
// Errors
// -----------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct InvalidScheduledWorkError {
  pub field: &'static str,
  pub reason: &'static str,
}

impl MechErrorKind for InvalidScheduledWorkError {
  fn name(&self) -> &str {
    "InvalidScheduledWork"
  }

  fn message(&self) -> String {
    format!("Invalid scheduled work field `{}`: {}", self.field, self.reason)
  }
}

fn invalid_scheduled_work<T>(
  field: &'static str,
  reason: &'static str,
) -> MResult<T> {
  Err(MechError::new(
    InvalidScheduledWorkError { field, reason },
    None,
  ))
}

#[derive(Debug, Clone)]
pub struct InvalidSchedulerPolicyError {
  pub field: &'static str,
  pub reason: &'static str,
}

impl MechErrorKind for InvalidSchedulerPolicyError {
  fn name(&self) -> &str {
    "InvalidSchedulerPolicy"
  }

  fn message(&self) -> String {
    format!("Invalid scheduler policy field `{}`: {}", self.field, self.reason)
  }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
  use super::*;

  use crate::context::AccessSet;
  use crate::id::{EventId, ObjectId, TransactionId};

  #[test]
  fn scheduler_enqueues_and_pops_task() {
    let mut scheduler = InMemoryScheduler::new();

    scheduler.enqueue_task(TaskId(1)).unwrap();

    assert_eq!(scheduler.len(), 1);
    assert_eq!(
      scheduler.next_work().unwrap(),
      Some(ScheduledWork::task(TaskId(1))),
    );
    assert!(scheduler.is_empty());
  }

  #[test]
  fn scheduler_enqueues_and_pops_actor() {
    let mut scheduler = InMemoryScheduler::new();

    scheduler.enqueue_actor(ActorId(1)).unwrap();

    assert_eq!(scheduler.len(), 1);
    assert_eq!(
      scheduler.next_work().unwrap(),
      Some(ScheduledWork::actor(ActorId(1))),
    );
    assert!(scheduler.is_empty());
  }

  #[test]
  fn scheduler_records_queue_and_start_events() {
    let mut scheduler = InMemoryScheduler::new();

    scheduler.enqueue_task(TaskId(1)).unwrap();
    scheduler.next_work().unwrap();

    let events = scheduler.pending_events();

    assert_eq!(events[0].name(), "scheduler.work.queued");
    assert_eq!(events[1].name(), "scheduler.work.started");
  }

  #[test]
  fn scheduler_deduplicates_work() {
    let mut scheduler = InMemoryScheduler::new();

    scheduler.enqueue_task(TaskId(1)).unwrap();
    scheduler.enqueue_task(TaskId(1)).unwrap();

    assert_eq!(scheduler.len(), 1);
    assert_eq!(scheduler.pending_events().len(), 1);
  }

  #[test]
  fn scheduler_can_disable_deduplication() {
    let mut scheduler = InMemoryScheduler::without_deduplication();

    scheduler.enqueue_task(TaskId(1)).unwrap();
    scheduler.enqueue_task(TaskId(1)).unwrap();

    assert_eq!(scheduler.len(), 2);
    assert_eq!(scheduler.pending_events().len(), 2);
  }

  #[test]
  fn scheduler_requeues_after_pop() {
    let mut scheduler = InMemoryScheduler::new();

    scheduler.enqueue_task(TaskId(1)).unwrap();

    let work = scheduler.next_work().unwrap().unwrap();
    assert_eq!(work, ScheduledWork::task(TaskId(1)));

    scheduler.enqueue_work(work).unwrap();

    assert_eq!(scheduler.len(), 1);
  }

  #[test]
  fn scheduler_rejects_zero_task_id() {
    let mut scheduler = InMemoryScheduler::new();

    assert!(scheduler.enqueue_task(TaskId(0)).is_err());
  }

  #[test]
  fn scheduler_rejects_zero_actor_id() {
    let mut scheduler = InMemoryScheduler::new();

    assert!(scheduler.enqueue_actor(ActorId(0)).is_err());
  }

  #[test]
  fn scheduler_records_completed_work_and_event() {
    let mut scheduler = InMemoryScheduler::new();

    let mut access = AccessSet::new();
    access.read(ObjectId(1));
    access.write(ObjectId(2));

    let outcome = RuntimeTurnOutcome::new()
      .with_transaction(TransactionId(1))
      .with_events(vec![EventId(1)])
      .with_access(access);

    let work = ScheduledWork::task(TaskId(1));

    scheduler.complete_work(work, outcome).unwrap();

    assert_eq!(scheduler.completed().len(), 1);
    assert_eq!(scheduler.completed()[0].work, work);
    assert_eq!(
      scheduler.completed()[0].outcome.transaction,
      Some(TransactionId(1)),
    );
    assert_eq!(
      scheduler.pending_events().last().unwrap().name(),
      "scheduler.work.completed",
    );
  }

  #[test]
  fn scheduler_records_failed_work_and_event() {
    let mut scheduler = InMemoryScheduler::new();

    scheduler
      .fail_work(ScheduledWork::task(TaskId(1)), "boom".to_string())
      .unwrap();

    assert_eq!(scheduler.failures().len(), 1);
    assert_eq!(scheduler.failures()[0].message, "boom");
    assert_eq!(
      scheduler.pending_events().last().unwrap().name(),
      "scheduler.work.failed",
    );
  }

  #[test]
  fn queued_work_returns_fifo_order() {
    let mut scheduler = InMemoryScheduler::new();

    scheduler.enqueue_task(TaskId(1)).unwrap();
    scheduler.enqueue_actor(ActorId(2)).unwrap();

    assert_eq!(
      scheduler.queued_work(),
      vec![
        ScheduledWork::task(TaskId(1)),
        ScheduledWork::actor(ActorId(2)),
      ],
    );
  }

  #[test]
  fn collect_tick_respects_limit_and_records_events() {
    let mut scheduler = InMemoryScheduler::new();

    scheduler.enqueue_task(TaskId(1)).unwrap();
    scheduler.enqueue_task(TaskId(2)).unwrap();
    scheduler.enqueue_actor(ActorId(3)).unwrap();

    scheduler.drain_events();

    let policy = SchedulerPolicy::default()
      .with_max_turns_per_tick(2);

    let tick = collect_tick(&mut scheduler, &policy).unwrap();

    assert_eq!(tick.len(), 2);
    assert_eq!(scheduler.len(), 1);

    let events = scheduler.pending_events();

    assert_eq!(events[0].name(), "runtime.tick.started");
    assert_eq!(events[1].name(), "scheduler.work.started");
    assert_eq!(events[2].name(), "scheduler.work.started");
    assert_eq!(events[3].name(), "runtime.tick.completed");
  }

  #[test]
  fn collect_tick_with_unbounded_policy_collects_all() {
    let mut scheduler = InMemoryScheduler::new();

    scheduler.enqueue_task(TaskId(1)).unwrap();
    scheduler.enqueue_task(TaskId(2)).unwrap();
    scheduler.enqueue_actor(ActorId(3)).unwrap();

    let tick = collect_tick(&mut scheduler, &SchedulerPolicy::unbounded()).unwrap();

    assert_eq!(tick.len(), 3);
    assert!(scheduler.is_empty());
  }

  #[test]
  fn scheduler_policy_rejects_zero_turn_limit() {
    let policy = SchedulerPolicy {
      max_turns_per_tick: Some(0),
      requeue_actor_after_message: true,
      deduplicate_queued_work: true,
    };

    assert!(policy.validate().is_err());
  }

  #[test]
  fn scheduled_work_labels_are_stable() {
    assert_eq!(
      ScheduledWork::task(TaskId(1)).label(),
      "task:00000000000000000000000000000001",
    );

    assert_eq!(
      ScheduledWork::actor(ActorId(2)).label(),
      "actor:00000000000000000000000000000002",
    );
  }

  #[test]
  fn drain_events_clears_pending_events() {
    let mut scheduler = InMemoryScheduler::new();

    scheduler.enqueue_task(TaskId(1)).unwrap();

    let events = scheduler.drain_events();

    assert_eq!(events.len(), 1);
    assert!(scheduler.pending_events().is_empty());
  }
}