//! Runtime transaction state.
//!
//! `TransactionRecord` is the durable store record.
//! `RuntimeTransaction` is the live transaction used while a task or actor turn
//! is executing.
//!
//! The transaction stages object writes until commit. This is the first real
//! transactional boundary. Later, the same shape can be extended to staged actor,
//! task, module, message, and capability mutations.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use std::collections::HashMap;

use mech_core::{MResult, MechError, MechErrorKind};

use crate::id::{
  ActorId, EventId, MessageId, ObjectId, TaskId, TransactionId,
};

use crate::store::{
  ActorRecord, MessageRecord, ObjectRecord, TaskRecord, TransactionRecord,
};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TransactionStatus {
  Open,
  Committed,
  Aborted {
    reason: String,
  },
}

impl TransactionStatus {
  pub fn is_open(&self) -> bool {
    matches!(self, Self::Open)
  }

  pub fn is_committed(&self) -> bool {
    matches!(self, Self::Committed)
  }

  pub fn is_aborted(&self) -> bool {
    matches!(self, Self::Aborted { .. })
  }
}

#[derive(Clone, Debug)]
pub struct RuntimeTransaction {
  pub id: TransactionId,
  pub subject: String,
  pub read_set: Vec<ObjectId>,
  pub write_set: Vec<ObjectId>,
  pub events: Vec<EventId>,
  pub staged_puts: HashMap<ObjectId, ObjectRecord>,
  pub staged_updates: HashMap<ObjectId, ObjectRecord>,
  pub status: TransactionStatus,
  pub staged_task_updates: HashMap<TaskId, TaskRecord>,
  pub staged_actor_updates: HashMap<ActorId, ActorRecord>,
  pub staged_message_enqueues: HashMap<ActorId, Vec<MessageRecord>>,
  pub staged_message_dequeues: HashMap<ActorId, Vec<MessageRecord>>,
}

impl RuntimeTransaction {
  pub fn new(
    id: TransactionId,
    subject: impl Into<String>,
  ) -> Self {
    Self {
      id,
      subject: subject.into(),
      read_set: Vec::new(),
      write_set: Vec::new(),
      events: Vec::new(),
      staged_puts: HashMap::new(),
      staged_updates: HashMap::new(),
      status: TransactionStatus::Open,
      staged_task_updates: HashMap::new(),
      staged_actor_updates: HashMap::new(),
      staged_message_enqueues: HashMap::new(),
      staged_message_dequeues: HashMap::new(),
    }
  }

  pub fn validate(&self) -> MResult<()> {
    if self.id.is_zero() {
      return Err(MechError::new(
        InvalidRuntimeTransactionError {
          field: "id",
          reason: "must not be zero",
        },
        None,
      ));
    }

    if self.subject.trim().is_empty() {
      return Err(MechError::new(
        InvalidRuntimeTransactionError {
          field: "subject",
          reason: "must not be empty",
        },
        None,
      ));
    }

    Ok(())
  }

  pub fn ensure_open(&self) -> MResult<()> {
    if !self.status.is_open() {
      return Err(MechError::new(
        InvalidRuntimeTransactionStateError {
          message: "transaction is not open".to_string(),
        },
        None,
      ));
    }

    Ok(())
  }

  pub fn record_read(&mut self, object: ObjectId) -> MResult<()> {
    self.ensure_open()?;

    if object.is_zero() {
      return invalid_runtime_transaction("object", "must not be zero");
    }

    if !self.read_set.contains(&object) {
      self.read_set.push(object);
    }

    Ok(())
  }

  pub fn record_write(&mut self, object: ObjectId) -> MResult<()> {
    self.ensure_open()?;

    if object.is_zero() {
      return invalid_runtime_transaction("object", "must not be zero");
    }

    if !self.write_set.contains(&object) {
      self.write_set.push(object);
    }

    Ok(())
  }

  pub fn record_event(&mut self, event: EventId) -> MResult<()> {
    self.ensure_open()?;

    if event.is_zero() {
      return invalid_runtime_transaction("event", "must not be zero");
    }

    if !self.events.contains(&event) {
      self.events.push(event);
    }

    Ok(())
  }

  pub fn stage_put_object(&mut self, object: ObjectRecord) -> MResult<ObjectId> {
    self.ensure_open()?;

    let id = object.id;

    if id.is_zero() {
      return invalid_runtime_transaction("object.id", "must not be zero");
    }

    self.record_write(id)?;
    self.staged_puts.insert(id, object);

    Ok(id)
  }

  pub fn stage_update_object(&mut self, object: ObjectRecord) -> MResult<ObjectId> {
    self.ensure_open()?;

    let id = object.id;

    if id.is_zero() {
      return invalid_runtime_transaction("object.id", "must not be zero");
    }

    self.record_write(id)?;

    if self.staged_puts.contains_key(&id) {
      self.staged_puts.insert(id, object);
    } else {
      self.staged_updates.insert(id, object);
    }

    Ok(id)
  }

  pub fn get_staged_object(&self, id: ObjectId) -> Option<ObjectRecord> {
    self
      .staged_updates
      .get(&id)
      .cloned()
      .or_else(|| self.staged_puts.get(&id).cloned())
  }

  pub fn has_staged_writes(&self) -> bool {
    !self.staged_puts.is_empty() || !self.staged_updates.is_empty()
  }

  pub fn staged_puts(&self) -> impl Iterator<Item = &ObjectRecord> {
    self.staged_puts.values()
  }

  pub fn staged_updates(&self) -> impl Iterator<Item = &ObjectRecord> {
    self.staged_updates.values()
  }

  pub fn merge_read_set(&mut self, reads: &[ObjectId]) -> MResult<()> {
    for object in reads {
      self.record_read(*object)?;
    }

    Ok(())
  }

  pub fn merge_write_set(&mut self, writes: &[ObjectId]) -> MResult<()> {
    for object in writes {
      self.record_write(*object)?;
    }

    Ok(())
  }

  pub fn merge_events(&mut self, events: &[EventId]) -> MResult<()> {
    for event in events {
      self.record_event(*event)?;
    }

    Ok(())
  }

  pub fn into_record(mut self) -> MResult<TransactionRecord> {
    self.validate()?;
    self.ensure_open()?;

    self.status = TransactionStatus::Committed;

    Ok(TransactionRecord::new(self.id, self.subject)
      .with_read_set(self.read_set)
      .with_write_set(self.write_set)
      .with_events(self.events))
  }

  pub fn abort(mut self, reason: impl Into<String>) -> MResult<Self> {
    self.validate()?;
    self.ensure_open()?;

    self.staged_puts.clear();
    self.staged_updates.clear();

    self.status = TransactionStatus::Aborted {
      reason: reason.into(),
    };
    self.staged_task_updates.clear();
    self.staged_actor_updates.clear();
    self.staged_message_enqueues.clear();
    self.staged_message_dequeues.clear();
    Ok(self)
  }

  pub fn stage_task_update(&mut self, task: TaskRecord) -> MResult<TaskId> {
    self.ensure_open()?;

    let id = task.id;

    if id.is_zero() {
      return invalid_runtime_transaction("task.id", "must not be zero");
    }

    self.staged_task_updates.insert(id, task);

    Ok(id)
  }

  pub fn get_staged_task(&self, id: TaskId) -> Option<TaskRecord> {
    self.staged_task_updates.get(&id).cloned()
  }

  pub fn stage_actor_update(&mut self, actor: ActorRecord) -> MResult<ActorId> {
    self.ensure_open()?;

    let id = actor.id;

    if id.is_zero() {
      return invalid_runtime_transaction("actor.id", "must not be zero");
    }

    self.staged_actor_updates.insert(id, actor);

    Ok(id)
  }

  pub fn get_staged_actor(&self, id: ActorId) -> Option<ActorRecord> {
    self.staged_actor_updates.get(&id).cloned()
  }

  pub fn stage_message_enqueue(
    &mut self,
    actor: ActorId,
    message: MessageRecord,
  ) -> MResult<MessageId> {
    self.ensure_open()?;

    if actor.is_zero() {
      return invalid_runtime_transaction("actor", "must not be zero");
    }

    if message.id.is_zero() {
      return invalid_runtime_transaction("message.id", "must not be zero");
    }

    let id = message.id;

    self
      .staged_message_enqueues
      .entry(actor)
      .or_default()
      .push(message);

    Ok(id)
  }

  pub fn stage_message_dequeue(
    &mut self,
    actor: ActorId,
    message: MessageRecord,
  ) -> MResult<MessageId> {
    self.ensure_open()?;

    if actor.is_zero() {
      return invalid_runtime_transaction("actor", "must not be zero");
    }

    if message.id.is_zero() {
      return invalid_runtime_transaction("message.id", "must not be zero");
    }

    let id = message.id;

    self
      .staged_message_dequeues
      .entry(actor)
      .or_default()
      .push(message);

    Ok(id)
  }

  pub fn pop_staged_enqueued_message(
    &mut self,
    actor: ActorId,
  ) -> Option<MessageRecord> {
    let queue = self.staged_message_enqueues.get_mut(&actor)?;
    if queue.is_empty() {
      None
    } else {
      Some(queue.remove(0))
    }
  }

  pub fn peek_staged_enqueued_message(
    &self,
    actor: ActorId,
  ) -> Option<MessageRecord> {
    self
      .staged_message_enqueues
      .get(&actor)
      .and_then(|queue| queue.first())
      .cloned()
  }

  pub fn staged_task_updates(&self) -> impl Iterator<Item = &TaskRecord> {
    self.staged_task_updates.values()
  }

  pub fn staged_actor_updates(&self) -> impl Iterator<Item = &ActorRecord> {
    self.staged_actor_updates.values()
  }

  pub fn staged_message_enqueues(
    &self,
  ) -> impl Iterator<Item = (&ActorId, &Vec<MessageRecord>)> {
    self.staged_message_enqueues.iter()
  }

  pub fn staged_message_dequeues(
    &self,
  ) -> impl Iterator<Item = (&ActorId, &Vec<MessageRecord>)> {
    self.staged_message_dequeues.iter()
  }
}

#[derive(Debug, Clone)]
pub struct InvalidRuntimeTransactionError {
  pub field: &'static str,
  pub reason: &'static str,
}

impl MechErrorKind for InvalidRuntimeTransactionError {
  fn name(&self) -> &str {
    "InvalidRuntimeTransaction"
  }

  fn message(&self) -> String {
    format!(
      "Invalid runtime transaction field `{}`: {}",
      self.field,
      self.reason
    )
  }
}

fn invalid_runtime_transaction<T>(
  field: &'static str,
  reason: &'static str,
) -> MResult<T> {
  Err(MechError::new(
    InvalidRuntimeTransactionError { field, reason },
    None,
  ))
}

#[derive(Debug, Clone)]
pub struct InvalidRuntimeTransactionStateError {
  pub message: String,
}

impl MechErrorKind for InvalidRuntimeTransactionStateError {
  fn name(&self) -> &str {
    "InvalidRuntimeTransactionState"
  }

  fn message(&self) -> String {
    self.message.clone()
  }
}

#[derive(Debug, Clone)]
pub struct RuntimeTransactionNotFoundError {
  pub transaction_id: TransactionId,
}

impl MechErrorKind for RuntimeTransactionNotFoundError {
  fn name(&self) -> &str {
    "RuntimeTransactionNotFound"
  }

  fn message(&self) -> String {
    format!("Runtime transaction not found: {}", self.transaction_id)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn transaction_records_read_write_and_event_sets() {
    let mut tx = RuntimeTransaction::new(TransactionId(1), "task:1");

    tx.record_read(ObjectId(1)).unwrap();
    tx.record_read(ObjectId(1)).unwrap();
    tx.record_write(ObjectId(2)).unwrap();
    tx.record_event(EventId(3)).unwrap();

    assert_eq!(tx.read_set, vec![ObjectId(1)]);
    assert_eq!(tx.write_set, vec![ObjectId(2)]);
    assert_eq!(tx.events, vec![EventId(3)]);
  }

  #[test]
  fn transaction_stages_object_puts() {
    let mut tx = RuntimeTransaction::new(TransactionId(1), "task:1");

    let object = ObjectRecord::text(ObjectId(7), "note", "hello");

    tx.stage_put_object(object).unwrap();

    assert_eq!(tx.write_set, vec![ObjectId(7)]);
    assert!(tx.get_staged_object(ObjectId(7)).is_some());
    assert!(tx.has_staged_writes());
  }

  #[test]
  fn staged_update_replaces_staged_put() {
    let mut tx = RuntimeTransaction::new(TransactionId(1), "task:1");

    tx.stage_put_object(ObjectRecord::text(ObjectId(7), "note", "hello"))
      .unwrap();

    tx.stage_update_object(ObjectRecord::text(ObjectId(7), "note", "updated"))
      .unwrap();

    assert_eq!(tx.staged_puts.len(), 1);
    assert_eq!(tx.staged_updates.len(), 0);

    let object = tx.get_staged_object(ObjectId(7)).unwrap();
    assert_eq!(object.data, b"updated");
  }

  #[test]
  fn transaction_commits_to_record() {
    let mut tx = RuntimeTransaction::new(TransactionId(1), "task:1");

    tx.record_read(ObjectId(1)).unwrap();
    tx.record_write(ObjectId(2)).unwrap();
    tx.record_event(EventId(3)).unwrap();

    let record = tx.into_record().unwrap();

    assert_eq!(record.id, TransactionId(1));
    assert_eq!(record.subject, "task:1");
    assert_eq!(record.read_set, vec![ObjectId(1)]);
    assert_eq!(record.write_set, vec![ObjectId(2)]);
    assert_eq!(record.events, vec![EventId(3)]);
  }

  #[test]
  fn abort_discards_staged_writes() {
    let mut tx = RuntimeTransaction::new(TransactionId(1), "task:1");

    tx.stage_put_object(ObjectRecord::text(ObjectId(7), "note", "hello"))
      .unwrap();

    let tx = tx.abort("boom").unwrap();

    assert!(tx.status.is_aborted());
    assert!(!tx.has_staged_writes());
  }
}