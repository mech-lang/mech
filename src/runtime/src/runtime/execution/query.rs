use crate::runtime::{
  MechRuntime,
  RuntimeInvalidOperationError,
  RuntimeProgramBusy,
};
use crate::RuntimeValueSnapshot;
use mech_core::{
  MResult,
  MechError,
  Value,
};

impl MechRuntime {
  #[cfg(feature = "invariant_define")]
  pub fn integrity_constraint_report(
    &self,
  ) -> MResult<mech_program::IntegrityConstraintReport> {
    self.program.integrity_constraint_report()
  }

  pub fn out_string(&self) -> String {
    self.program.out_string()
  }

  pub fn has_interpreter(&self, interpreter_id: u64) -> bool {
    self.program.has_interpreter(interpreter_id)
  }

  pub fn root_plan_len(&self) -> usize {
    self.program.interpreter().plan_len()
  }

  pub fn output_value_for_interpreter(
    &self,
    interpreter_id: u64,
    output_id: u64,
  ) -> MResult<Option<RuntimeValueSnapshot>> {
    self
      .program
      .output_value_for_interpreter(interpreter_id, output_id)
      .as_ref()
      .map(RuntimeValueSnapshot::try_capture)
      .transpose()
  }

  pub fn symbol_name_for_interpreter_output(
    &self,
    interpreter_id: u64,
    output_id: u64,
  ) -> Option<String> {
    self.program.symbol_name_for_interpreter_output(interpreter_id, output_id)
  }

  pub fn symbol_values_for_interpreter(
    &self,
    interpreter_id: u64,
    names: &[String],
  ) -> MResult<Option<Vec<(String, RuntimeValueSnapshot)>>> {
    self
      .program
      .symbol_values_for_interpreter(interpreter_id, names)
      .map(|values| {
        values
          .into_iter()
          .map(|(name, value)| {
            Ok((
              name,
              RuntimeValueSnapshot::try_capture(&value)?,
            ))
          })
          .collect::<MResult<Vec<_>>>()
      })
      .transpose()
  }

  pub fn root_symbol_value(
    &self,
    name: &str,
  ) -> MResult<RuntimeValueSnapshot> {
    self
      .program
      .root_symbol_value(name)
      .and_then(|value| {
        RuntimeValueSnapshot::try_capture(&value)
      })
  }

  pub fn root_symbol_values(
    &self,
    names: &[&str],
  ) -> MResult<Vec<(String, RuntimeValueSnapshot)>> {
    self
      .program
      .root_symbol_values(names)
      .and_then(|values| {
        values
          .into_iter()
          .map(|(name, value)| {
            Ok((
              name,
              RuntimeValueSnapshot::try_capture(&value)?,
            ))
          })
          .collect::<MResult<Vec<_>>>()
      })
  }

  pub fn root_symbol_values_all(
    &self,
  ) -> MResult<Vec<(String, RuntimeValueSnapshot)>> {
    self
      .program
      .root_symbol_values_all()
      .into_iter()
      .map(|(name, value)| {
        Ok((
          name,
          RuntimeValueSnapshot::try_capture(&value)?,
        ))
      })
      .collect::<MResult<Vec<_>>>()
  }

  pub fn bind_ans_for_interpreter(
    &mut self,
    interpreter_id: u64,
    value: &Value,
  ) -> MResult<()> {
    if let Some(owner) = self.program_transaction_owner {
      return Err(MechError::new(
        RuntimeProgramBusy {
          operation: "bind_ans_for_interpreter",
          owner,
          requester: None,
        },
        None,
      ));
    }

    if self.program.bind_ans_for_interpreter(interpreter_id, value) {
      return Ok(());
    }

    Err(MechError::new(
      RuntimeInvalidOperationError {
        operation: "bind_ans_for_interpreter",
        reason: format!("interpreter id {} not found", interpreter_id),
      },
      None,
    ))
  }
}
