//! Actor turn model.
//!
//! An actor turn is the unit of actor execution. It binds together the actor,
//! the message being processed, and the behavior/state metadata needed by the
//! runtime to execute the turn transactionally.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use mech_core::{MResult, MechError, MechErrorKind};

use crate::id::{
  ActorId, MessageId, ModuleVersionId, ObjectId,
};

use crate::store::{
  ActorRecord, MessageRecord,
};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActorTurn {
  pub actor: ActorId,
  pub subject: String,
  pub message: MessageRecord,
  pub behavior: Option<ModuleVersionId>,
  pub state: Option<ObjectId>,
}

impl ActorTurn {
  pub fn new(
    actor: ActorRecord,
    message: MessageRecord,
  ) -> MResult<Self> {
    actor.validate()?;
    message.validate()?;

    if message.actor != actor.id {
      return Err(MechError::new(
        InvalidActorTurnError {
          field: "message.actor",
          reason: "message actor does not match actor",
        },
        None,
      ));
    }

    Ok(Self {
      actor: actor.id,
      subject: actor.subject,
      message,
      behavior: actor.behavior,
      state: actor.state,
    })
  }

  pub fn validate(&self) -> MResult<()> {
    if self.actor.is_zero() {
      return invalid_actor_turn("actor", "must not be zero");
    }

    if self.subject.trim().is_empty() {
      return invalid_actor_turn("subject", "must not be empty");
    }

    self.message.validate()?;

    if self.message.actor != self.actor {
      return invalid_actor_turn(
        "message.actor",
        "message actor does not match actor",
      );
    }

    Ok(())
  }

  pub fn actor_id(&self) -> ActorId {
    self.actor
  }

  pub fn message_id(&self) -> MessageId {
    self.message.id
  }

  pub fn message_kind(&self) -> &str {
    &self.message.kind
  }

  pub fn message_payload(&self) -> &[u8] {
    &self.message.payload
  }

  pub fn has_behavior(&self) -> bool {
    self.behavior.is_some()
  }

  pub fn has_state(&self) -> bool {
    self.state.is_some()
  }
}

#[derive(Debug, Clone)]
pub struct InvalidActorTurnError {
  pub field: &'static str,
  pub reason: &'static str,
}

impl MechErrorKind for InvalidActorTurnError {
  fn name(&self) -> &str {
    "InvalidActorTurn"
  }

  fn message(&self) -> String {
    format!("Invalid actor turn field `{}`: {}", self.field, self.reason)
  }
}

fn invalid_actor_turn<T>(
  field: &'static str,
  reason: &'static str,
) -> MResult<T> {
  Err(MechError::new(
    InvalidActorTurnError { field, reason },
    None,
  ))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn actor_turn_builds_from_actor_and_message() {
    let actor = ActorRecord::new(ActorId(1), "actor:1")
      .with_state(ObjectId(2))
      .with_behavior(ModuleVersionId(3));

    let message = MessageRecord::new(
      MessageId(4),
      ActorId(1),
      "ping",
      b"hello".to_vec(),
    );

    let turn = ActorTurn::new(actor, message).unwrap();

    assert_eq!(turn.actor, ActorId(1));
    assert_eq!(turn.subject, "actor:1");
    assert_eq!(turn.state, Some(ObjectId(2)));
    assert_eq!(turn.behavior, Some(ModuleVersionId(3)));
    assert_eq!(turn.message_id(), MessageId(4));
    assert_eq!(turn.message_kind(), "ping");
    assert_eq!(turn.message_payload(), b"hello");
  }

  #[test]
  fn actor_turn_rejects_mismatched_message_actor() {
    let actor = ActorRecord::new(ActorId(1), "actor:1");

    let message = MessageRecord::new(
      MessageId(4),
      ActorId(2),
      "ping",
      b"hello".to_vec(),
    );

    assert!(ActorTurn::new(actor, message).is_err());
  }
}