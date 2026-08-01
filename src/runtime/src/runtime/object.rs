// Object methods
// -----------------------------------------------------------------------------

// These methods manage objects within the runtime, allowing for creating, retrieving, and updating objects. An object in Mech is a data structure that can hold arbitrary data and is identified by a unique ObjectId. Objects can be used to represent state, resources, or any other kind of data that actors and tasks may need to interact with.

// The methods include:

// - `put_object`: Adds a new object record to the store and emits an ObjectCreated event.
// - `get_object`: Retrieves an object record by its ID.
// - `update_object`: Updates an existing object record in the store and emits an ObjectUpdated event.

use crate::runtime::MechRuntime;
use crate::{ObjectId, ObjectRecord, RuntimeContext, RuntimeEventKind};
use mech_core::MResult;

impl MechRuntime {
    pub fn put_object(&mut self, object: ObjectRecord) -> MResult<ObjectId> {
        self.ensure_runtime_mutation_allowed("put_object")?;
        let mut context = self.runtime_context()?;
        self.put_object_with_context(&mut context, object)
    }

    pub fn put_object_with_context(
        &mut self,
        context: &mut RuntimeContext,
        object: ObjectRecord,
    ) -> MResult<ObjectId> {
        self.ensure_runtime_mutation_allowed("put_object_with_context")?;
        self.validate_context_for_runtime(context)?;
        context.charge_bytes(object.data.len() as u64)?;

        if let Some(transaction_id) = context.transaction {
            let id = object.id;

            self.active_transaction_mut(transaction_id)?
                .stage_put_object(object)?;

            context.record_write(id);

            self.emit_event_to_context(context, RuntimeEventKind::ObjectCreated { object_id: id })?;

            return Ok(id);
        }

        let id = self.store.put_object(object)?;
        context.record_write(id);

        self.emit_event_to_context(context, RuntimeEventKind::ObjectCreated { object_id: id })?;

        Ok(id)
    }

    pub fn get_object(&self, id: ObjectId) -> MResult<Option<ObjectRecord>> {
        self.store.get_object(id)
    }

    pub fn get_object_with_context(
        &mut self,
        context: &mut RuntimeContext,
        id: ObjectId,
    ) -> MResult<Option<ObjectRecord>> {
        self.validate_context_for_runtime(context)?;
        context.record_read(id);

        if let Some(transaction_id) = context.transaction {
            if let Some(transaction) = self.active_transactions.get(&transaction_id) {
                if let Some(object) = transaction.store.get_staged_object(id) {
                    return Ok(Some(object));
                }
            }
        }

        self.store.get_object(id)
    }

    pub fn update_object(&mut self, object: ObjectRecord) -> MResult<ObjectId> {
        self.ensure_runtime_mutation_allowed("update_object")?;
        let mut context = self.runtime_context()?;
        self.update_object_with_context(&mut context, object)
    }

    pub fn update_object_with_context(
        &mut self,
        context: &mut RuntimeContext,
        object: ObjectRecord,
    ) -> MResult<ObjectId> {
        self.ensure_runtime_mutation_allowed("update_object_with_context")?;
        self.validate_context_for_runtime(context)?;
        context.charge_bytes(object.data.len() as u64)?;

        if let Some(transaction_id) = context.transaction {
            let id = object.id;

            self.active_transaction_mut(transaction_id)?
                .stage_update_object(object)?;

            context.record_write(id);

            self.emit_event_to_context(context, RuntimeEventKind::ObjectUpdated { object_id: id })?;

            return Ok(id);
        }

        let id = self.store.update_object(object)?;
        context.record_write(id);

        self.emit_event_to_context(context, RuntimeEventKind::ObjectUpdated { object_id: id })?;

        Ok(id)
    }
}
