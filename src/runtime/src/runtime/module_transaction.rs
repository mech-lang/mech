//! Transaction-local staging for module and module-version records.

use super::*;

#[derive(Debug)]
pub(super) enum RuntimeModuleMutation {
  PutModule(ModuleRecord),
  PutVersion(ModuleVersionRecord),
}

#[derive(Debug, Default)]
pub(super) struct RuntimeModuleJournal {
  operations: Vec<RuntimeModuleMutation>,
  module_index: HashMap<ModuleId, usize>,
  module_name_index: HashMap<String, ModuleId>,
  version_index: HashMap<ModuleVersionId, usize>,
}

impl RuntimeModuleJournal {
  pub(super) fn new() -> Self {
    Self::default()
  }

  pub(super) fn is_empty(&self) -> bool {
    self.operations.is_empty()
  }

  pub(super) fn mark(&self) -> usize {
    self.operations.len()
  }

  pub(super) fn rollback_to(&mut self, mark: usize) -> MResult<()> {
    if mark > self.operations.len() {
      return Err(MechError::new(
        RuntimeInvalidOperationError {
          operation: "rollback_module_journal",
          reason: format!(
            "module journal mark {} exceeds operation length {}",
            mark,
            self.operations.len(),
          ),
        },
        None,
      ));
    }

    self.operations.truncate(mark);
    self.rebuild_indexes()
  }

  pub(super) fn get_module(
    &self,
    id: ModuleId,
  ) -> Option<&ModuleRecord> {
    let index = *self.module_index.get(&id)?;
    match self.operations.get(index)? {
      RuntimeModuleMutation::PutModule(module) => Some(module),
      RuntimeModuleMutation::PutVersion(_) => None,
    }
  }

  pub(super) fn find_module_by_name(
    &self,
    canonical_uri: &str,
  ) -> Option<&ModuleRecord> {
    let id = *self.module_name_index.get(canonical_uri)?;
    self.get_module(id)
  }

  pub(super) fn get_version(
    &self,
    id: ModuleVersionId,
  ) -> Option<&ModuleVersionRecord> {
    let index = *self.version_index.get(&id)?;
    match self.operations.get(index)? {
      RuntimeModuleMutation::PutVersion(version) => Some(version),
      RuntimeModuleMutation::PutModule(_) => None,
    }
  }

  pub(super) fn stage_module(
    &mut self,
    module: ModuleRecord,
  ) -> MResult<bool> {
    module.validate()?;
    if module.id != module_id(&module.name) {
      return module_journal_conflict(
        "module",
        module.id.to_string(),
        "module ID does not match its canonical URI",
      );
    }

    if let Some(existing) = self.get_module(module.id) {
      if existing.name == module.name {
        return Ok(false);
      }
      return module_journal_conflict(
        "module",
        module.id.to_string(),
        "module ID is already staged for another canonical URI",
      );
    }

    if let Some(existing_id) =
      self.module_name_index.get(&module.name)
    {
      return module_journal_conflict(
        "module.name",
        module.name.clone(),
        format!(
          "canonical URI is already staged for module {}",
          existing_id,
        ),
      );
    }

    let index = self.operations.len();
    self.module_index.insert(module.id, index);
    self
      .module_name_index
      .insert(module.name.clone(), module.id);
    self
      .operations
      .push(RuntimeModuleMutation::PutModule(module));
    Ok(true)
  }

  pub(super) fn stage_version(
    &mut self,
    version: ModuleVersionRecord,
  ) -> MResult<bool> {
    version.validate()?;
    version.validate_import_edges()?;

    if let Some(existing) = self.get_version(version.id) {
      if existing == &version {
        return Ok(false);
      }
      return module_journal_conflict(
        "module_version",
        version.id.to_string(),
        "version ID is already staged with different contents",
      );
    }

    let index = self.operations.len();
    self.version_index.insert(version.id, index);
    self
      .operations
      .push(RuntimeModuleMutation::PutVersion(version));
    Ok(true)
  }

  pub(super) fn module_puts(
    &self,
  ) -> impl Iterator<Item = &ModuleRecord> {
    self.operations.iter().filter_map(|operation| match operation {
      RuntimeModuleMutation::PutModule(module) => Some(module),
      RuntimeModuleMutation::PutVersion(_) => None,
    })
  }

  pub(super) fn version_puts(
    &self,
  ) -> impl Iterator<Item = &ModuleVersionRecord> {
    self.operations.iter().filter_map(|operation| match operation {
      RuntimeModuleMutation::PutVersion(version) => Some(version),
      RuntimeModuleMutation::PutModule(_) => None,
    })
  }

  fn rebuild_indexes(&mut self) -> MResult<()> {
    self.module_index.clear();
    self.module_name_index.clear();
    self.version_index.clear();

    for (index, operation) in self.operations.iter().enumerate() {
      match operation {
        RuntimeModuleMutation::PutModule(module) => {
          module.validate()?;
          if module.id != module_id(&module.name) {
            return module_journal_conflict(
              "module",
              module.id.to_string(),
              "rollback retained a module whose ID does not match its canonical URI",
            );
          }
          if let Some(existing_index) =
            self.module_index.get(&module.id)
          {
            let RuntimeModuleMutation::PutModule(existing) =
              &self.operations[*existing_index]
            else {
              unreachable!();
            };
            if existing.name != module.name {
              return module_journal_conflict(
                "module",
                module.id.to_string(),
                "rollback retained conflicting canonical URIs",
              );
            }
            continue;
          }
          if let Some(existing_id) =
            self.module_name_index.get(&module.name)
          {
            if existing_id != &module.id {
              return module_journal_conflict(
                "module.name",
                module.name.clone(),
                "rollback retained conflicting module IDs",
              );
            }
            continue;
          }
          self.module_index.insert(module.id, index);
          self
            .module_name_index
            .insert(module.name.clone(), module.id);
        }
        RuntimeModuleMutation::PutVersion(version) => {
          version.validate()?;
          version.validate_import_edges()?;
          if let Some(existing_index) =
            self.version_index.get(&version.id)
          {
            let RuntimeModuleMutation::PutVersion(existing) =
              &self.operations[*existing_index]
            else {
              unreachable!();
            };
            if existing != version {
              return module_journal_conflict(
                "module_version",
                version.id.to_string(),
                "rollback retained conflicting version records",
              );
            }
            continue;
          }
          self.version_index.insert(version.id, index);
        }
      }
    }
    Ok(())
  }
}

fn module_journal_conflict<T>(
  record_type: &'static str,
  identity: impl Into<String>,
  reason: impl Into<String>,
) -> MResult<T> {
  Err(MechError::new(
    RuntimeModuleJournalConflict {
      record_type,
      identity: identity.into(),
      reason: reason.into(),
    },
    None,
  ))
}

impl MechRuntime {
  pub fn get_module(
    &self,
    id: ModuleId,
  ) -> MResult<Option<ModuleRecord>> {
    self.store.get_module(id)
  }

  pub fn get_module_version(
    &self,
    id: ModuleVersionId,
  ) -> MResult<Option<ModuleVersionRecord>> {
    self.store.get_module_version(id)
  }

  pub(super) fn get_module_visible(
    &self,
    context: &RuntimeContext,
    id: ModuleId,
  ) -> MResult<Option<ModuleRecord>> {
    if let Some(transaction_id) = context.transaction {
      let transaction =
        self.active_execution_transaction(transaction_id)?;
      if let Some(module) = transaction.modules.get_module(id) {
        return Ok(Some(module.clone()));
      }
    }
    self.store.get_module(id)
  }

  pub(super) fn find_module_by_name_visible(
    &self,
    context: &RuntimeContext,
    canonical_uri: &str,
  ) -> MResult<Option<ModuleRecord>> {
    if let Some(transaction_id) = context.transaction {
      let transaction =
        self.active_execution_transaction(transaction_id)?;
      if let Some(module) = transaction
        .modules
        .find_module_by_name(canonical_uri)
      {
        return Ok(Some(module.clone()));
      }
    }
    self.store.find_module_by_name(canonical_uri)
  }

  pub(super) fn get_module_version_visible(
    &self,
    context: &RuntimeContext,
    id: ModuleVersionId,
  ) -> MResult<Option<ModuleVersionRecord>> {
    if let Some(transaction_id) = context.transaction {
      let transaction =
        self.active_execution_transaction(transaction_id)?;
      if let Some(version) = transaction.modules.get_version(id) {
        return Ok(Some(version.clone()));
      }
    }
    self.store.get_module_version(id)
  }

  pub(super) fn with_atomic_module_operation<T>(
    &mut self,
    context: &mut RuntimeContext,
    operation: &'static str,
    execute: impl FnOnce(
      &mut MechRuntime,
      &mut RuntimeContext,
    ) -> MResult<T>,
  ) -> MResult<T> {
    self.ensure_runtime_mutation_allowed(operation)?;
    self.validate_context_for_runtime(context)?;
    self.reject_program_operation_reentrancy(operation)?;

    let implicit = context.transaction.is_none();
    if implicit {
      self.begin_runtime_transaction_internal(
        context,
        RuntimeExecutionTransactionMode::ImplicitModuleOperation,
      )?;
    }

    let transaction_id = Self::context_transaction_id(context)?;
    if self.active_execution_transaction(transaction_id)?.mode
      != if implicit {
        RuntimeExecutionTransactionMode::ImplicitModuleOperation
      } else {
        RuntimeExecutionTransactionMode::Explicit
      }
    {
      return Err(MechError::new(
        RuntimeInvalidOperationError {
          operation,
          reason: format!(
            "transaction {} has an incompatible execution mode",
            transaction_id,
          ),
        },
        None,
      ));
    }

    let savepoint =
      self.capture_runtime_operation_savepoint(context, transaction_id)?;
    match execute(self, context) {
      Ok(value) if !implicit => Ok(value),
      Ok(value) => match self
        .commit_runtime_transaction_internal(context)
      {
        Ok(super::transaction::RuntimeCommitResolution::Committed(_)) => {
          Ok(value)
        }
        Ok(
          super::transaction::RuntimeCommitResolution::CommittedWithError {
            error,
            ..
          },
        ) => Err(error),
        Err(error) => self.finish_failed_module_operation(
          context,
          operation,
          transaction_id,
          &savepoint,
          error,
          true,
        ),
      },
      Err(error) => self.finish_failed_module_operation(
        context,
        operation,
        transaction_id,
        &savepoint,
        error,
        implicit,
      ),
    }
  }

  fn finish_failed_module_operation<T>(
    &mut self,
    context: &mut RuntimeContext,
    operation: &'static str,
    transaction_id: TransactionId,
    savepoint: &RuntimeOperationSavepoint,
    original_error: MechError,
    implicit: bool,
  ) -> MResult<T> {
    let original_error_text = format!("{:?}", original_error);
    let mut rollback_failures = self.rollback_runtime_operation(
      context,
      transaction_id,
      savepoint,
    );

    if implicit {
      rollback_failures.extend(
        self.cleanup_failed_implicit_operation(
          context,
          operation,
          transaction_id,
          &format!("module operation `{}` failed", operation),
        ),
      );
    }

    if rollback_failures.is_empty() {
      return Err(original_error);
    }
    Err(self.poison_program_operation(
      operation,
      Some(transaction_id),
      original_error_text,
      rollback_failures,
    ))
  }
}

#[cfg(test)]
#[path = "module_transaction/tests/mod.rs"]
mod tests;
