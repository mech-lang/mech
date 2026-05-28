// ---------------------------------------------------------------------------
// Object methods
// ---------------------------------------------------------------------------

use super::*;

impl MechRuntime {

  pub fn put_object(&mut self, object: ObjectRecord) -> MResult<ObjectId> {
    let mut context = self.runtime_context()?;
    self.put_object_with_context(&mut context, object)
  }

  pub fn put_object_with_context(
    &mut self,
    context: &mut RuntimeContext,
    object: ObjectRecord,
  ) -> MResult<ObjectId> {
    context.validate()?;
    context.charge_bytes(object.data.len() as u64)?;

    if self.has_active_context_transaction(context) {
      let transaction_id = Self::context_transaction_id(context)?;
      let id = object.id;

      self
        .active_transaction_mut(transaction_id)?
        .stage_put_object(object)?;

      context.record_write(id);

      self.emit_event_to_context(
        context,
        RuntimeEventKind::ObjectCreated {
          object_id: id,
        },
      )?;

      return Ok(id);
    }

    let id = self.store.put_object(object)?;
    context.record_write(id);

    self.emit_event_to_context(
      context,
      RuntimeEventKind::ObjectCreated {
        object_id: id,
      },
    )?;

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
    context.validate()?;
    context.record_read(id);

    if let Some(transaction_id) = context.transaction {
      if let Some(transaction) = self.active_transactions.get(&transaction_id) {
        if let Some(object) = transaction.get_staged_object(id) {
          return Ok(Some(object));
        }
      }
    }

    self.store.get_object(id)
  }

  pub fn update_object(&mut self, object: ObjectRecord) -> MResult<ObjectId> {
    let mut context = self.runtime_context()?;
    self.update_object_with_context(&mut context, object)
  }

  pub fn update_object_with_context(
    &mut self,
    context: &mut RuntimeContext,
    object: ObjectRecord,
  ) -> MResult<ObjectId> {
    context.validate()?;
    context.charge_bytes(object.data.len() as u64)?;

    if self.has_active_context_transaction(context) {
      let transaction_id = Self::context_transaction_id(context)?;
      let id = object.id;

      self
        .active_transaction_mut(transaction_id)?
        .stage_update_object(object)?;

      context.record_write(id);

      self.emit_event_to_context(
        context,
        RuntimeEventKind::ObjectUpdated {
          object_id: id,
        },
      )?;

      return Ok(id);
    }

    let id = self.store.update_object(object)?;
    context.record_write(id);

    self.emit_event_to_context(
      context,
      RuntimeEventKind::ObjectUpdated {
        object_id: id,
      },
    )?;

    Ok(id)
  }
}