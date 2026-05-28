// Task methods
// -----------------------------------------------------------------------------

// These methods handle the creation, retrieval, updating, completion, and failure of tasks within the runtime. Tasks represent units of work that can be executed asynchronously. 

// The methods include:

// - `put_task`: Adds a new task record to the store and emits a TaskCreated event.
// - `start_task`: Creates and starts a new task with the given subject, optional module version, and capabilities. It emits a TaskStarted event.
// - `get_task`: Retrieves a task record by its ID.
// - `get_task_with_context`: Retrieves a task record by its ID, considering any active transaction
// - `update_task`: Updates an existing task record in the store.
// - `update_task_with_context`: Updates an existing task record, considering any active transaction.
// - `complete_task`: Marks a task as completed and emits a TaskCompleted event.
// - `complete_task_with_context`: Marks a task as completed, considering any active transaction, and
//   `emits a TaskCompleted event.
// - `fail_task`: Marks a task as failed with a given reason and emits a TaskFailed event.
// - `fail_task_with_context`: Marks a task as failed with a given reason, considering any active transaction, and emits a TaskFailed event.

use super::*;

impl MechRuntime {

  pub fn put_task(&mut self, task: TaskRecord) -> MResult<TaskId> {
    let mut context = self.context_for_task(&task)?;
    self.put_task_with_context(&mut context, task)
  }

  pub fn put_task_with_context(
    &mut self,
    context: &mut RuntimeContext,
    task: TaskRecord,
  ) -> MResult<TaskId> {
    context.validate()?;
    context.charge_step()?;

    let id = self.store.put_task(task)?;

    self.emit_event_to_context(
      context,
      RuntimeEventKind::TaskCreated {
        task_id: id,
      },
    )?;

    Ok(id)
  }

  pub fn start_task(
    &mut self,
    subject: impl Into<String>,
    module_version: Option<ModuleVersionId>,
    capabilities: Vec<CapabilityId>,
  ) -> MResult<TaskId> {
    let id = self.next_task_id();

    let mut task = TaskRecord::new(id, subject)
      .with_status(TaskStatus::running())
      .with_capabilities(capabilities);

    if let Some(module_version) = module_version {
      task = task.with_module_version(module_version);
    }

    let mut context = self.context_for_task(&task)?;
    self.put_task_with_context(&mut context, task)?;

    self.emit_event_to_context(
      &mut context,
      RuntimeEventKind::TaskStarted {
        task_id: id,
      },
    )?;

    Ok(id)
  }

  pub fn get_task(&self, id: TaskId) -> MResult<Option<TaskRecord>> {
    self.store.get_task(id)
  }

  pub fn get_task_with_context(
    &mut self,
    context: &mut RuntimeContext,
    id: TaskId,
  ) -> MResult<Option<TaskRecord>> {
    context.validate()?;

    if let Some(transaction_id) = context.transaction {
      if let Some(transaction) = self.active_transactions.get(&transaction_id) {
        if let Some(task) = transaction.get_staged_task(id) {
          return Ok(Some(task));
        }
      }
    }

    self.store.get_task(id)
  }    

  pub fn update_task(&mut self, task: TaskRecord) -> MResult<TaskId> {
    self.store.update_task(task)
  }

  pub fn update_task_with_context(
    &mut self,
    context: &mut RuntimeContext,
    task: TaskRecord,
  ) -> MResult<TaskId> {
    context.validate()?;

    if self.has_active_context_transaction(context) {
      let transaction_id = Self::context_transaction_id(context)?;
      let id = task.id;

      self
        .active_transaction_mut(transaction_id)?
        .stage_task_update(task)?;

      return Ok(id);
    }

    self.store.update_task(task)
  }

  pub fn complete_task(&mut self, id: TaskId) -> MResult<()> {
    let Some(task) = self.store.get_task(id)? else {
      return Err(MechError::new(
        RuntimeRecordNotFoundError {
          record_type: "task",
          id: id.to_string(),
        },
        None,
      ));
    };

    let mut context = self.context_for_task(&task)?;
    self.complete_task_with_context(&mut context, id)
  }

  pub fn complete_task_with_context(
    &mut self,
    context: &mut RuntimeContext,
    id: TaskId,
  ) -> MResult<()> {
    let Some(mut task) = self.get_task_with_context(context, id)? else {
      return Err(MechError::new(
        RuntimeRecordNotFoundError {
          record_type: "task",
          id: id.to_string(),
        },
        None,
      ));
    };

    task.status = TaskStatus::completed();

    self.update_task_with_context(context, task)?;

    self.emit_event_to_context(
      context,
      RuntimeEventKind::TaskCompleted {
        task_id: id,
      },
    )?;

    Ok(())
  }

  pub fn fail_task(&mut self, id: TaskId, reason: impl Into<String>) -> MResult<()> {
    let reason = reason.into();

    let Some(task) = self.store.get_task(id)? else {
      return Err(MechError::new(
        RuntimeRecordNotFoundError {
          record_type: "task",
          id: id.to_string(),
        },
        None,
      ));
    };

    let mut context = self.context_for_task(&task)?;
    self.fail_task_with_context(&mut context, id, reason)
  }

  pub fn fail_task_with_context(
    &mut self,
    context: &mut RuntimeContext,
    id: TaskId,
    reason: impl Into<String>,
  ) -> MResult<()> {
    let reason = reason.into();

    let Some(mut task) = self.get_task_with_context(context, id)? else {
      return Err(MechError::new(
        RuntimeRecordNotFoundError {
          record_type: "task",
          id: id.to_string(),
        },
        None,
      ));
    };

    task.status = TaskStatus::failed();

    self.update_task_with_context(context, task)?;

    self.emit_event_to_context(
      context,
      RuntimeEventKind::TaskFailed {
        task_id: id,
        message: reason,
      },
    )?;

    Ok(())
  }

}