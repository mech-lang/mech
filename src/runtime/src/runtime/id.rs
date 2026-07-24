// ---------------------------------------------------------------------------
// ID generation methods
// ---------------------------------------------------------------------------

// These methods generate unique IDs for various entities in the runtime, such as objects, actors, tasks, capabilities, transactions, events, and messages. The IDs are generated using an internal ID generator that ensures uniqueness across the runtime.

// The methods include:

// - `next_object_id`: Generates the next unique ObjectId.
// - `next_actor_id`: Generates the next unique ActorId.
// - `next_task_id`: Generates the next unique TaskId.
// - `next_capability_id`: Generates the next unique CapabilityId.
// - `next_transaction_id`: Generates the next unique TransactionId.
// - `next_event_id`: Generates the next unique EventId.
// - `next_message_id`: Generates the next unique MessageId, which is derived from the next EventId to ensure uniqueness.

use super::*;
  
impl MechRuntime {
  pub fn next_object_id(&mut self) -> ObjectId {
    self.id_generator.object_id()
  }

  pub fn next_actor_id(&mut self) -> ActorId {
    self.id_generator.actor_id()
  }

  pub fn next_task_id(&mut self) -> TaskId {
    self.id_generator.task_id()
  }

  pub fn next_capability_id(&mut self) -> CapabilityId {
    self.id_generator.capability_id()
  }

  pub fn next_transaction_id(&mut self) -> TransactionId {
    self.id_generator.transaction_id()
  }

  pub fn next_event_id(&mut self) -> EventId {
    self.id_generator.event_id()
  }

  pub fn next_message_id(&mut self) -> MessageId {
    MessageId(self.id_generator.event_id().as_u128())
  }
}