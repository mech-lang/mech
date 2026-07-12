//! Narrow runtime service traits.
//!
//! Host functions should not receive the whole runtime object. They should
//! receive a narrow service surface that exposes only the operations they are
//! allowed to perform.
//!
//! `RuntimeServices` is the first such surface. It is intentionally small and
//! context-aware, so reads, writes, actor updates, budgets, events, capabilities,
//! and transactions can still be mediated by `RuntimeContext`.

use mech_core::MResult;

#[cfg(feature = "watcher")]
mod workspace_session;

#[cfg(feature = "watcher")]
pub use self::workspace_session::*;

use crate::context::RuntimeContext;

use crate::id::{
  ActorId, ObjectId,
};

use crate::store::{
  ActorRecord, ObjectRecord,
};

// -----------------------------------------------------------------------------
// Runtime Services
// -----------------------------------------------------------------------------

pub trait RuntimeServices {
  fn next_object_id(&mut self) -> ObjectId;

  fn get_object_with_context(
    &mut self,
    context: &mut RuntimeContext,
    id: ObjectId,
  ) -> MResult<Option<ObjectRecord>>;

  fn put_object_with_context(
    &mut self,
    context: &mut RuntimeContext,
    object: ObjectRecord,
  ) -> MResult<ObjectId>;

  fn update_object_with_context(
    &mut self,
    context: &mut RuntimeContext,
    object: ObjectRecord,
  ) -> MResult<ObjectId>;

  fn get_actor_with_context(
    &mut self,
    context: &mut RuntimeContext,
    id: ActorId,
  ) -> MResult<Option<ActorRecord>>;

  fn update_actor_with_context(
    &mut self,
    context: &mut RuntimeContext,
    actor: ActorRecord,
  ) -> MResult<ActorId>;
}

// -----------------------------------------------------------------------------
// Null Services
// -----------------------------------------------------------------------------

/// A service implementation that always fails.
///
/// This is useful for tests or host functions that must be invoked in places
/// where runtime mutation is intentionally unavailable.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoRuntimeServices;

impl RuntimeServices for NoRuntimeServices {
  fn next_object_id(&mut self) -> ObjectId {
    ObjectId(0)
  }

  fn get_object_with_context(
    &mut self,
    _context: &mut RuntimeContext,
    _id: ObjectId,
  ) -> MResult<Option<ObjectRecord>> {
    unavailable_runtime_service("get_object_with_context")
  }

  fn put_object_with_context(
    &mut self,
    _context: &mut RuntimeContext,
    _object: ObjectRecord,
  ) -> MResult<ObjectId> {
    unavailable_runtime_service("put_object_with_context")
  }

  fn update_object_with_context(
    &mut self,
    _context: &mut RuntimeContext,
    _object: ObjectRecord,
  ) -> MResult<ObjectId> {
    unavailable_runtime_service("update_object_with_context")
  }

  fn get_actor_with_context(
    &mut self,
    _context: &mut RuntimeContext,
    _id: ActorId,
  ) -> MResult<Option<ActorRecord>> {
    unavailable_runtime_service("get_actor_with_context")
  }

  fn update_actor_with_context(
    &mut self,
    _context: &mut RuntimeContext,
    _actor: ActorRecord,
  ) -> MResult<ActorId> {
    unavailable_runtime_service("update_actor_with_context")
  }
}

// -----------------------------------------------------------------------------
// Errors
// -----------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct RuntimeServiceUnavailableError {
  pub service: &'static str,
}

impl mech_core::MechErrorKind for RuntimeServiceUnavailableError {
  fn name(&self) -> &str {
    "RuntimeServiceUnavailable"
  }

  fn message(&self) -> String {
    format!("Runtime service `{}` is unavailable in this context", self.service)
  }
}

fn unavailable_runtime_service<T>(service: &'static str) -> MResult<T> {
  Err(mech_core::MechError::new(
    RuntimeServiceUnavailableError { service },
    None,
  ))
}

#[cfg(test)]
mod tests {
  use super::*;

  use crate::id::{
    RuntimeId,
  };

  #[test]
  fn no_runtime_services_rejects_object_get() {
    let mut services = NoRuntimeServices;
    let mut context = RuntimeContext::new(RuntimeId(1), "test");

    assert!(
      services
        .get_object_with_context(&mut context, ObjectId(1))
        .is_err()
    );
  }

  #[test]
  fn no_runtime_services_returns_zero_object_id() {
    let mut services = NoRuntimeServices;

    assert_eq!(services.next_object_id(), ObjectId(0));
  }
}