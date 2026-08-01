//! Narrow runtime service traits.
//!
//! Host functions should not receive the whole runtime object. They should
//! receive a narrow service surface that exposes only the operations they are
//! allowed to perform.
//!
//! `RuntimeManagedServices` is intentionally narrow. Each implementation is
//! bound to one active runtime call, so host code never receives a mutable
//! runtime context or any runtime component trait object.

use mech_core::MResult;

#[cfg(feature = "watcher")]
mod workspace_session;

#[cfg(feature = "watcher")]
pub use self::workspace_session::*;

use crate::id::{ActorId, ObjectId};

use crate::store::{ActorRecord, ObjectRecord};

// -----------------------------------------------------------------------------
// Runtime Services
// -----------------------------------------------------------------------------

pub trait RuntimeManagedServices {
    fn allocate_object_id(&mut self) -> MResult<ObjectId>;

    fn get_object(&mut self, id: ObjectId) -> MResult<Option<ObjectRecord>>;

    fn put_object(&mut self, object: ObjectRecord) -> MResult<ObjectId>;

    fn update_object(&mut self, object: ObjectRecord) -> MResult<ObjectId>;

    fn get_actor(&mut self, id: ActorId) -> MResult<Option<ActorRecord>>;

    fn update_actor(&mut self, actor: ActorRecord) -> MResult<ActorId>;

    fn set_current_actor_state(&mut self, state: ObjectId) -> MResult<()>;
}
