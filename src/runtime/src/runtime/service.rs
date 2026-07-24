// -----------------------------------------------------------------------------
// Runtime Services Implementation
// -----------------------------------------------------------------------------

// Services are how the runtime provides functionality to actors and tasks. They are the public API of the runtime that allows for interaction with objects, actors, capabilities, transactions, and other runtime features. The methods in this section implement the RuntimeServices trait for the MechRuntime struct, providing the actual logic for each service method.

use super::*;

impl RuntimeServices for MechRuntime {
  fn next_object_id(&mut self) -> ObjectId {
    MechRuntime::next_object_id(self)
  }

  fn get_object_with_context(
    &mut self,
    context: &mut RuntimeContext,
    id: ObjectId,
  ) -> MResult<Option<ObjectRecord>> {
    MechRuntime::get_object_with_context(self, context, id)
  }

  fn put_object_with_context(
    &mut self,
    context: &mut RuntimeContext,
    object: ObjectRecord,
  ) -> MResult<ObjectId> {
    MechRuntime::put_object_with_context(self, context, object)
  }

  fn update_object_with_context(
    &mut self,
    context: &mut RuntimeContext,
    object: ObjectRecord,
  ) -> MResult<ObjectId> {
    MechRuntime::update_object_with_context(self, context, object)
  }

  fn get_actor_with_context(
    &mut self,
    context: &mut RuntimeContext,
    id: ActorId,
  ) -> MResult<Option<ActorRecord>> {
    MechRuntime::get_actor_with_context(self, context, id)
  }

  fn update_actor_with_context(
    &mut self,
    context: &mut RuntimeContext,
    actor: ActorRecord,
  ) -> MResult<ActorId> {
    MechRuntime::update_actor_with_context(self, context, actor)
  }
}