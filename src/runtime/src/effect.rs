//! Runtime effect participation contracts.
//!
//! Prepared effects make external mutation policy explicit. Providers and host
//! functions select the strongest protocol they can honestly implement; the
//! runtime owns protocol ordering and lifecycle state.

use std::fmt::Debug;

use mech_core::MResult;

use crate::TransactionId;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RuntimeEffectId {
  pub transaction: TransactionId,
  pub sequence: u64,
}

impl std::fmt::Display for RuntimeEffectId {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}:{}", self.transaction, self.sequence)
  }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RuntimeEffectCost {
  pub bytes: u64,
  pub items: u64,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeEffectSource {
  ResourceProvider {
    scheme: String,
  },
  HostFunction {
    name: String,
  },
  Runtime {
    component: String,
  },
  Custom {
    name: String,
  },
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeEffectProtocol {
  Transactional,
  Compensatable,
  AfterCommit,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeEffectMetadata {
  pub source: RuntimeEffectSource,
  pub operation: String,
  pub resource: Option<String>,
  pub cost: RuntimeEffectCost,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeEffectRecord {
  pub id: RuntimeEffectId,
  pub source: RuntimeEffectSource,
  pub operation: String,
  pub resource: Option<String>,
  pub protocol: RuntimeEffectProtocol,
}

impl RuntimeEffectRecord {
  pub fn new(
    id: RuntimeEffectId,
    metadata: RuntimeEffectMetadata,
    protocol: RuntimeEffectProtocol,
  ) -> Self {
    Self {
      id,
      source: metadata.source,
      operation: metadata.operation,
      resource: metadata.resource,
      protocol,
    }
  }
}

impl RuntimeEffectMetadata {
  pub fn new(
    source: RuntimeEffectSource,
    operation: impl Into<String>,
  ) -> Self {
    Self {
      source,
      operation: operation.into(),
      resource: None,
      cost: RuntimeEffectCost::default(),
    }
  }

  pub fn with_resource(mut self, resource: impl Into<String>) -> Self {
    self.resource = Some(resource.into());
    self
  }

  pub fn with_cost(mut self, cost: RuntimeEffectCost) -> Self {
    self.cost = cost;
    self
  }
}

pub trait RuntimeTransactionalEffect: Debug {
  fn metadata(&self) -> RuntimeEffectMetadata;

  fn prepare(&mut self) -> MResult<()>;
  fn commit(&mut self) -> MResult<()>;
  fn abort(&mut self) -> MResult<()>;
}

pub trait RuntimeCompensatableEffect: Debug {
  fn metadata(&self) -> RuntimeEffectMetadata;

  fn apply(&mut self) -> MResult<()>;
  fn compensate(&mut self) -> MResult<()>;

  fn abort(&mut self) -> MResult<()> {
    Ok(())
  }
}

pub trait RuntimeAfterCommitEffect: Debug {
  fn metadata(&self) -> RuntimeEffectMetadata;
  fn deliver(&mut self) -> MResult<()>;
}

#[derive(Debug)]
pub enum PreparedRuntimeEffect {
  Transactional(Box<dyn RuntimeTransactionalEffect>),
  Compensatable(Box<dyn RuntimeCompensatableEffect>),
  AfterCommit(Box<dyn RuntimeAfterCommitEffect>),
}

impl PreparedRuntimeEffect {
  pub fn metadata(&self) -> RuntimeEffectMetadata {
    match self {
      Self::Transactional(effect) => effect.metadata(),
      Self::Compensatable(effect) => effect.metadata(),
      Self::AfterCommit(effect) => effect.metadata(),
    }
  }

  pub fn protocol(&self) -> RuntimeEffectProtocol {
    match self {
      Self::Transactional(_) => RuntimeEffectProtocol::Transactional,
      Self::Compensatable(_) => RuntimeEffectProtocol::Compensatable,
      Self::AfterCommit(_) => RuntimeEffectProtocol::AfterCommit,
    }
  }

  pub fn cost(&self) -> RuntimeEffectCost {
    self.metadata().cost
  }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeEffectFailurePhase {
  Prepare,
  Apply,
  Compensate,
  Abort,
  Commit,
  Deliver,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActiveRuntimeEffectPhase {
  Preparing,
  Applying,
  Compensating,
  Aborting,
  Committing,
  Delivering,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeEffectFailure {
  pub effect_id: RuntimeEffectId,
  pub phase: RuntimeEffectFailurePhase,
  pub message: String,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeCommitOutcome {
  pub transaction_id: TransactionId,
  pub delivery_failures: Vec<RuntimeEffectFailure>,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[derive(Debug)]
  struct NoopAfterCommit;

  impl RuntimeAfterCommitEffect for NoopAfterCommit {
    fn metadata(&self) -> RuntimeEffectMetadata {
      RuntimeEffectMetadata::new(
        RuntimeEffectSource::Custom {
          name: "noop".to_string(),
        },
        "deliver",
      )
    }

    fn deliver(&mut self) -> MResult<()> {
      Ok(())
    }
  }

  #[test]
  fn prepared_effect_reports_public_protocol_and_metadata() {
    let effect =
      PreparedRuntimeEffect::AfterCommit(Box::new(NoopAfterCommit));

    assert_eq!(effect.protocol(), RuntimeEffectProtocol::AfterCommit);
    assert_eq!(effect.metadata().operation, "deliver");
    assert_eq!(effect.cost(), RuntimeEffectCost::default());
  }
}
