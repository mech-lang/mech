//! Runtime transaction state.
//!
//! `TransactionRecord` is the durable store record.
//! `RuntimeTransaction` is the live transaction used while a task or actor turn
//! is executing.
//!
//! The runtime should begin a transaction at a safe boundary, collect read/write
//! sets and emitted events, then commit or abort the transaction.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use mech_core::{MResult, MechError, MechErrorKind};

use crate::id::{
  EventId, ObjectId, TransactionId,
};

use crate::store::TransactionRecord;

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

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeTransaction {
  pub id: TransactionId,
  pub subject: String,
  pub read_set: Vec<ObjectId>,
  pub write_set: Vec<ObjectId>,
  pub events: Vec<EventId>,
  pub status: TransactionStatus,
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
      status: TransactionStatus::Open,
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
      return Err(MechError::new(
        InvalidRuntimeTransactionError {
          field: "object",
          reason: "must not be zero",
        },
        None,
      ));
    }

    if !self.read_set.contains(&object) {
      self.read_set.push(object);
    }

    Ok(())
  }

  pub fn record_write(&mut self, object: ObjectId) -> MResult<()> {
    self.ensure_open()?;

    if object.is_zero() {
      return Err(MechError::new(
        InvalidRuntimeTransactionError {
          field: "object",
          reason: "must not be zero",
        },
        None,
      ));
    }

    if !self.write_set.contains(&object) {
      self.write_set.push(object);
    }

    Ok(())
  }

  pub fn record_event(&mut self, event: EventId) -> MResult<()> {
    self.ensure_open()?;

    if event.is_zero() {
      return Err(MechError::new(
        InvalidRuntimeTransactionError {
          field: "event",
          reason: "must not be zero",
        },
        None,
      ));
    }

    if !self.events.contains(&event) {
      self.events.push(event);
    }

    Ok(())
  }

  pub fn commit(mut self) -> MResult<TransactionRecord> {
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

    self.status = TransactionStatus::Aborted {
      reason: reason.into(),
    };

    Ok(self)
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
  fn transaction_commits_to_record() {
    let mut tx = RuntimeTransaction::new(TransactionId(1), "task:1");

    tx.record_read(ObjectId(1)).unwrap();
    tx.record_write(ObjectId(2)).unwrap();
    tx.record_event(EventId(3)).unwrap();

    let record = tx.commit().unwrap();

    assert_eq!(record.id, TransactionId(1));
    assert_eq!(record.subject, "task:1");
    assert_eq!(record.read_set, vec![ObjectId(1)]);
    assert_eq!(record.write_set, vec![ObjectId(2)]);
    assert_eq!(record.events, vec![EventId(3)]);
  }

  #[test]
  fn aborted_transaction_cannot_be_committed_later() {
    let tx = RuntimeTransaction::new(TransactionId(1), "task:1");
    let tx = tx.abort("boom").unwrap();

    assert!(tx.status.is_aborted());
  }
}