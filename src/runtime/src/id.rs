//! Runtime identity types for Mech.
//!
//! This module separates two classes of identifiers:
//!
//! 1. Generated runtime IDs:
//!    RuntimeId, ObjectId, ActorId, TaskId, CapabilityId,
//!    TransactionId, EventId, NodeId.
//!
//!    These are generated with UUID v7 and stored as u128 values.
//!
//! 2. Deterministic content/name IDs:
//!    ModuleId, ModuleVersionId.
//!
//!    These are derived from BLAKE3 with explicit domain separation.
//!
//! Do not use these IDs as replacements for Mech's existing internal
//! symbol hashes. Symbol IDs, function IDs, kind IDs, and compiler-local
//! identifiers may continue to use the existing hash_str path.

use core::fmt;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use blake3::*;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RuntimeId(pub u128);

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModuleId(pub u128);

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModuleVersionId(pub u128);

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectId(pub u128);

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ActorId(pub u128);

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaskId(pub u128);

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CapabilityId(pub u128);

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TransactionId(pub u128);

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EventId(pub u128);

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(pub u128);

macro_rules! impl_id {
  ($id:ident) => {
    impl $id {
      pub const ZERO: Self = Self(0);

      pub const fn new(raw: u128) -> Self {
        Self(raw)
      }

      pub const fn as_u128(self) -> u128 {
        self.0
      }

      pub const fn is_zero(self) -> bool {
        self.0 == 0
      }

      pub fn to_hex(self) -> String {
        format!("{:032x}", self.0)
      }

      pub fn from_hex(input: &str) -> Result<Self, IdParseError> {
        parse_u128_hex(input).map(Self)
      }
    }

    impl From<u128> for $id {
      fn from(value: u128) -> Self {
        Self(value)
      }
    }

    impl From<$id> for u128 {
      fn from(value: $id) -> Self {
        value.0
      }
    }

    impl fmt::Display for $id {
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:032x}", self.0)
      }
    }
  };
}

impl_id!(RuntimeId);
impl_id!(ModuleId);
impl_id!(ModuleVersionId);
impl_id!(ObjectId);
impl_id!(ActorId);
impl_id!(TaskId);
impl_id!(CapabilityId);
impl_id!(TransactionId);
impl_id!(EventId);
impl_id!(NodeId);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IdParseError {
  Empty,
  TooLong {
    max_len: usize,
    actual_len: usize,
  },
  InvalidHex {
    value: String,
  },
}

impl fmt::Display for IdParseError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      IdParseError::Empty => write!(f, "ID string is empty"),
      IdParseError::TooLong {
        max_len,
        actual_len,
      } => {
        write!(
          f,
          "ID string is too long: expected at most {} hex characters, got {}",
          max_len, actual_len
        )
      }
      IdParseError::InvalidHex { value } => {
        write!(f, "ID string is not valid hexadecimal: {}", value)
      }
    }
  }
}

#[cfg(feature = "std")]
impl std::error::Error for IdParseError {}

fn parse_u128_hex(input: &str) -> Result<u128, IdParseError> {
  let input = input.trim();

  if input.is_empty() {
    return Err(IdParseError::Empty);
  }

  let input = input
    .strip_prefix("0x")
    .or_else(|| input.strip_prefix("0X"))
    .unwrap_or(input);

  if input.len() > 32 {
    return Err(IdParseError::TooLong {
      max_len: 32,
      actual_len: input.len(),
    });
  }

  u128::from_str_radix(input, 16).map_err(|_| IdParseError::InvalidHex {
    value: input.to_string(),
  })
}

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

#[derive(Clone, Debug, Default)]
pub struct DefaultIdGenerator;

impl DefaultIdGenerator {
  pub fn new() -> Self {
    Self
  }

  fn uuid_v7_u128() -> u128 {
    Uuid::now_v7().as_u128()
  }
}

impl IdGenerator for DefaultIdGenerator {
  fn runtime_id(&mut self) -> RuntimeId {
    RuntimeId(Self::uuid_v7_u128())
  }

  fn object_id(&mut self) -> ObjectId {
    ObjectId(Self::uuid_v7_u128())
  }

  fn actor_id(&mut self) -> ActorId {
    ActorId(Self::uuid_v7_u128())
  }

  fn task_id(&mut self) -> TaskId {
    TaskId(Self::uuid_v7_u128())
  }

  fn capability_id(&mut self) -> CapabilityId {
    CapabilityId(Self::uuid_v7_u128())
  }

  fn transaction_id(&mut self) -> TransactionId {
    TransactionId(Self::uuid_v7_u128())
  }

  fn event_id(&mut self) -> EventId {
    EventId(Self::uuid_v7_u128())
  }

  fn node_id(&mut self) -> NodeId {
    NodeId(Self::uuid_v7_u128())
  }
}

/// Deterministic generator for tests.
///
/// This is useful for snapshot tests, predictable event logs, and examples.
/// Do not use this in production runtime state.
#[derive(Clone, Debug)]
pub struct SequentialIdGenerator {
  next: u128,
}

impl Default for SequentialIdGenerator {
  fn default() -> Self {
    Self { next: 1 }
  }
}

impl SequentialIdGenerator {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn starting_at(next: u128) -> Self {
    Self { next }
  }

  fn next_raw(&mut self) -> u128 {
    let id = self.next;
    self.next = self.next.saturating_add(1);
    id
  }
}

impl IdGenerator for SequentialIdGenerator {
  fn runtime_id(&mut self) -> RuntimeId {
    RuntimeId(self.next_raw())
  }

  fn object_id(&mut self) -> ObjectId {
    ObjectId(self.next_raw())
  }

  fn actor_id(&mut self) -> ActorId {
    ActorId(self.next_raw())
  }

  fn task_id(&mut self) -> TaskId {
    TaskId(self.next_raw())
  }

  fn capability_id(&mut self) -> CapabilityId {
    CapabilityId(self.next_raw())
  }

  fn transaction_id(&mut self) -> TransactionId {
    TransactionId(self.next_raw())
  }

  fn event_id(&mut self) -> EventId {
    EventId(self.next_raw())
  }

  fn node_id(&mut self) -> NodeId {
    NodeId(self.next_raw())
  }
}

/// Derive a stable ModuleId from a module name or canonical module path.
///
/// This is deterministic. It should produce the same ModuleId for the same
/// canonical name across processes, databases, and machines.
pub fn module_id(name: &str) -> ModuleId {
  ModuleId(blake3_u128(
    b"mech.module.id.v1",
    &[name.as_bytes()],
  ))
}

/// Derive a stable ModuleVersionId from all inputs that affect module meaning.
pub fn module_version_id(
  source: &str,
  compiler_version: &str,
  language_edition: &str,
  target: &str,
  feature_flags: &[&str],
  dependencies: &[ModuleVersionId],
  capability_requirements: &[&str],
) -> ModuleVersionId {
  let mut flags = feature_flags.to_vec();
  flags.sort();

  let mut caps = capability_requirements.to_vec();
  caps.sort();

  let mut deps = dependencies.to_vec();
  deps.sort();

  let flag_bytes = join_str_parts(&flags);
  let cap_bytes = join_str_parts(&caps);
  let dep_bytes: Vec<u8> = deps
    .iter()
    .flat_map(|dep| dep.0.to_le_bytes())
    .collect();

  ModuleVersionId(blake3_u128(
    b"mech.module.version.full.v1",
    &[
      compiler_version.as_bytes(),
      language_edition.as_bytes(),
      target.as_bytes(),
      source.as_bytes(),
      &flag_bytes,
      &dep_bytes,
      &cap_bytes,
    ],
  ))
}

fn join_str_parts(parts: &[&str]) -> Vec<u8> {
  let mut out = Vec::new();

  for part in parts {
    let bytes = part.as_bytes();
    let len = bytes.len() as u64;
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(bytes);
  }

  out
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
  u128::from_le_bytes(
    hash.as_bytes()[0..16]
      .try_into()
      .expect("BLAKE3 hash output is always at least 16 bytes"),
  )
}

/// Full 256-bit BLAKE3 digest for blobs, snapshots, bytecode blobs,
/// or integrity-sensitive content.
///
/// For compact database keys, the u128 IDs above are convenient. For integrity
/// checking, use the full digest.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContentHash(pub [u8; 32]);

impl ContentHash {
  pub fn as_bytes(&self) -> &[u8; 32] {
    &self.0
  }

  pub fn to_hex(&self) -> String {
    self.0.iter().map(|b| format!("{:02x}", b)).collect()
  }
}

impl fmt::Display for ContentHash {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    for byte in self.0 {
      write!(f, "{:02x}", byte)?;
    }
    Ok(())
  }
}

pub fn content_hash(domain: &'static [u8], parts: &[&[u8]]) -> ContentHash {
  let mut hasher = blake3::Hasher::new();

  hasher.update(domain);

  for part in parts {
    let len = part.len() as u64;
    hasher.update(&len.to_le_bytes());
    hasher.update(part);
  }

  ContentHash(*hasher.finalize().as_bytes())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn display_is_32_hex_chars() {
    let id = RuntimeId(0x1234);
    assert_eq!(id.to_string(), "00000000000000000000000000001234");
  }

  #[test]
  fn parse_hex_accepts_prefix() {
    let id = RuntimeId::from_hex("0x1234").unwrap();
    assert_eq!(id, RuntimeId(0x1234));
  }

  #[test]
  fn sequential_generator_is_predictable() {
    let mut ids = SequentialIdGenerator::new();

    assert_eq!(ids.runtime_id(), RuntimeId(1));
    assert_eq!(ids.object_id(), ObjectId(2));
    assert_eq!(ids.actor_id(), ActorId(3));
    assert_eq!(ids.task_id(), TaskId(4));
  }

  #[test]
  fn module_id_is_deterministic() {
    assert_eq!(module_id("foo"), module_id("foo"));
    assert_ne!(module_id("foo"), module_id("bar"));
  }

  #[test]
  fn module_version_id_is_deterministic() {
    let a = module_version_id("x := 1", "0.3.5", "2021", "x86_64-unknown-linux-gnu", &["flag1"], &[ModuleVersionId(1)], &["cap1"]);
    let b = module_version_id("x := 1", "0.3.5", "2021", "x86_64-unknown-linux-gnu", &["flag1"], &[ModuleVersionId(1)], &["cap1"]);
    let c = module_version_id("x := 2", "0.3.5", "2021", "x86_64-unknown-linux-gnu", &["flag1"], &[ModuleVersionId(1)], &["cap1"]);

    assert_eq!(a, b);
    assert_ne!(a, c);
  }

  #[test]
  fn dependency_order_is_normalized() {
    let d1 = ModuleVersionId(1);
    let d2 = ModuleVersionId(2);

    let a = module_version_id("x := 1", "0.3.5", "2021", "x86_64-unknown-linux-gnu", &["flag1"], &[d1, d2], &["cap1"]);
    let b = module_version_id("x := 1", "0.3.5", "2021", "x86_64-unknown-linux-gnu", &["flag1"], &[d2, d1], &["cap1"]);

    assert_eq!(a, b);
  }

  #[test]
  fn content_hash_is_deterministic() {
    let a = content_hash(b"test", &[b"abc", b"def"]);
    let b = content_hash(b"test", &[b"abc", b"def"]);
    let c = content_hash(b"test", &[b"abcdef"]);

    assert_eq!(a, b);
    assert_ne!(a, c);
  }
}