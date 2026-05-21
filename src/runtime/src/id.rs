use std::fmt;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RuntimeId(pub u128);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModuleId(pub u128);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModuleVersionId(pub u128);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectId(pub u128);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ActorId(pub u128);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaskId(pub u128);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CapabilityId(pub u128);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TransactionId(pub u128);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EventId(pub u128);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(pub u128);

pub trait IdGenerator {
  fn runtime_id(&mut self) -> RuntimeId;
  fn object_id(&mut self) -> ObjectId;
  fn actor_id(&mut self) -> ActorId;
  fn task_id(&mut self) -> TaskId;
  fn capability_id(&mut self) -> CapabilityId;
  fn transaction_id(&mut self) -> TransactionId;
  fn event_id(&mut self) -> EventId;
  fn node_id(&mut self) -> NodeId;
}

pub struct DefaultIdGenerator;

impl DefaultIdGenerator {
  fn random_u128() -> u128 {
    Uuid::now_v7().as_u128()
  }
}

impl IdGenerator for DefaultIdGenerator {
  fn runtime_id(&mut self) -> RuntimeId {
    RuntimeId(Self::random_u128())
  }

  fn object_id(&mut self) -> ObjectId {
    ObjectId(Self::random_u128())
  }

  fn actor_id(&mut self) -> ActorId {
    ActorId(Self::random_u128())
  }

  fn task_id(&mut self) -> TaskId {
    TaskId(Self::random_u128())
  }

  fn capability_id(&mut self) -> CapabilityId {
    CapabilityId(Self::random_u128())
  }

  fn transaction_id(&mut self) -> TransactionId {
    TransactionId(Self::random_u128())
  }

  fn event_id(&mut self) -> EventId {
    EventId(Self::random_u128())
  }

  fn node_id(&mut self) -> NodeId {
    NodeId(Self::random_u128())
  }
}

fn blake3_u128(domain: &'static [u8], parts: &[&[u8]]) -> u128 {
  let mut hasher = blake3::Hasher::new();
  hasher.update(domain);

  for part in parts {
    let len = part.len() as u64;
    hasher.update(&len.to_le_bytes());
    hasher.update(part);
  }

  let hash = hasher.finalize();
  u128::from_le_bytes(hash.as_bytes()[0..16].try_into().unwrap())
}

pub fn module_id(name: &str) -> ModuleId {
  ModuleId(blake3_u128(
    b"mech.module.id.v1",
    &[name.as_bytes()],
  ))
}

pub fn module_version_id(
  source: &str,
  compiler_version: &str,
  dependencies: &[ModuleVersionId],
) -> ModuleVersionId {
  let mut deps = dependencies.to_vec();
  deps.sort();

  let dep_bytes: Vec<u8> = deps
    .iter()
    .flat_map(|dep| dep.0.to_le_bytes())
    .collect();

  ModuleVersionId(blake3_u128(
    b"mech.module.version.v1",
    &[
      compiler_version.as_bytes(),
      source.as_bytes(),
      &dep_bytes,
    ],
  ))
}