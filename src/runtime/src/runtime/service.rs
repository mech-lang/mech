// -----------------------------------------------------------------------------
// Runtime Services Implementation
// -----------------------------------------------------------------------------

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