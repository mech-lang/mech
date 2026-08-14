use crate::RuntimeValueSnapshot;
use crate::runtime::MechRuntime;
#[cfg(feature = "resident-routing")]
use crate::runtime::RuntimeInvalidOperationError;
use mech_core::MResult;
#[cfg(feature = "resident-routing")]
use mech_core::MechError;

impl MechRuntime {
    pub fn root_interpreter_id(&self) -> u64 {
        self.program.interpreter().id
    }

    pub fn root_plan_len(&self) -> usize {
        #[cfg(feature = "resident-routing")]
        if let Some((_, instance)) = self.resident_artifact_and_instance() {
            return instance.plan.execution_node_count();
        }
        self.program.interpreter().plan_len()
    }

    pub fn output_value_for_interpreter(
        &self,
        interpreter_id: u64,
        output_id: u64,
    ) -> MResult<Option<RuntimeValueSnapshot>> {
        #[cfg(feature = "resident-routing")]
        if let Some((artifact, instance)) = self.resident_artifact_and_instance() {
            if interpreter_id != self.root_interpreter_id() {
                return Ok(None);
            }
            let Some(index) = artifact
                .outputs()
                .iter()
                .position(|output| u64::from(output.output.get()) == output_id)
            else {
                return Ok(None);
            };
            return super::super::resident_program::output_value(instance, index);
        }
        self.program
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
        #[cfg(feature = "resident-routing")]
        if let Some((artifact, _)) = self.resident_artifact_and_instance() {
            if interpreter_id != self.root_interpreter_id() {
                return None;
            }
            return artifact
                .outputs()
                .iter()
                .find(|output| u64::from(output.output.get()) == output_id)
                .map(|output| output.name.clone());
        }
        self.program
            .symbol_name_for_interpreter_output(interpreter_id, output_id)
    }

    pub fn symbol_values_for_interpreter(
        &self,
        interpreter_id: u64,
        names: &[String],
    ) -> MResult<Option<Vec<(String, RuntimeValueSnapshot)>>> {
        #[cfg(feature = "resident-routing")]
        if self.resident_artifact_and_instance().is_some() {
            if interpreter_id != self.root_interpreter_id() {
                return Ok(None);
            }
            return self
                .resident_symbol_values(names.iter().map(String::as_str))
                .map(Some);
        }
        self.program
            .symbol_values_for_interpreter(interpreter_id, names)
            .map(|values| {
                values
                    .into_iter()
                    .map(|(name, value)| Ok((name, RuntimeValueSnapshot::try_capture(&value)?)))
                    .collect::<MResult<Vec<_>>>()
            })
            .transpose()
    }

    pub fn root_symbol_value(&self, name: &str) -> MResult<RuntimeValueSnapshot> {
        #[cfg(feature = "resident-routing")]
        if self.resident_artifact_and_instance().is_some() {
            return self
                .resident_symbol_values(std::iter::once(name))?
                .pop()
                .map(|(_, value)| value)
                .ok_or_else(|| {
                    MechError::new(
                        RuntimeInvalidOperationError {
                            operation: "root_symbol_value",
                            reason: format!("resident output symbol `{name}` was not found"),
                        },
                        None,
                    )
                });
        }
        self.program
            .root_symbol_value(name)
            .and_then(|value| RuntimeValueSnapshot::try_capture(&value))
    }

    pub fn root_symbol_values(
        &self,
        names: &[&str],
    ) -> MResult<Vec<(String, RuntimeValueSnapshot)>> {
        #[cfg(feature = "resident-routing")]
        if self.resident_artifact_and_instance().is_some() {
            return self.resident_symbol_values(names.iter().copied());
        }
        self.program.root_symbol_values(names).and_then(|values| {
            values
                .into_iter()
                .map(|(name, value)| Ok((name, RuntimeValueSnapshot::try_capture(&value)?)))
                .collect::<MResult<Vec<_>>>()
        })
    }

    pub fn root_symbol_values_all(&self) -> MResult<Vec<(String, RuntimeValueSnapshot)>> {
        #[cfg(feature = "resident-routing")]
        if let Some((artifact, _)) = self.resident_artifact_and_instance() {
            return self.resident_symbol_values(
                artifact.outputs().iter().map(|output| output.name.as_str()),
            );
        }
        self.program
            .root_symbol_values_all()
            .into_iter()
            .map(|(name, value)| Ok((name, RuntimeValueSnapshot::try_capture(&value)?)))
            .collect::<MResult<Vec<_>>>()
    }

    #[cfg(feature = "resident-routing")]
    fn resident_artifact_and_instance(
        &self,
    ) -> Option<(
        &mech_engine::ProgramArtifact,
        &mech_engine::resident::ReactiveInstance,
    )> {
        use crate::runtime::resident_program::ActiveProgramExecution;
        match &self.active_program {
            ActiveProgramExecution::ResidentPure(execution) => {
                Some((&execution.artifact, &execution.instance))
            }
            ActiveProgramExecution::ResidentExternal(execution) => {
                Some((&execution.artifact, execution.coordinator.instance()))
            }
            ActiveProgramExecution::None | ActiveProgramExecution::Legacy => None,
        }
    }

    #[cfg(feature = "resident-routing")]
    fn resident_symbol_values<'a>(
        &self,
        names: impl IntoIterator<Item = &'a str>,
    ) -> MResult<Vec<(String, RuntimeValueSnapshot)>> {
        let (artifact, instance) = self
            .resident_artifact_and_instance()
            .expect("resident symbol queries require an active resident owner");
        let mut values = Vec::new();
        for name in names {
            let Some(index) = artifact
                .outputs()
                .iter()
                .position(|output| output.name == name)
            else {
                return Err(MechError::new(
                    RuntimeInvalidOperationError {
                        operation: "resident_symbol_values",
                        reason: format!("resident output symbol `{name}` was not found"),
                    },
                    None,
                ));
            };
            let value = super::super::resident_program::output_value(instance, index)?.ok_or_else(
                || {
                    MechError::new(
                        RuntimeInvalidOperationError {
                            operation: "resident_symbol_values",
                            reason: format!("resident output symbol `{name}` has no value"),
                        },
                        None,
                    )
                },
            )?;
            values.push((name.to_owned(), value));
        }
        Ok(values)
    }
}
