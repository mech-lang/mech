use crate::RuntimeValueSnapshot;
use crate::runtime::{MechRuntime, RuntimeInvalidOperationError};
use mech_core::{MResult, MechError};

const ROOT_INTERPRETER_ID: u64 = 0;

impl MechRuntime {
    pub fn root_interpreter_id(&self) -> u64 {
        ROOT_INTERPRETER_ID
    }

    pub fn root_plan_len(&self) -> usize {
        #[cfg(feature = "resident-routing")]
        if let Some((_, instance)) = self.resident_artifact_and_instance() {
            return instance.plan.execution_node_count();
        }
        0
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
        let _ = (interpreter_id, output_id);
        Ok(None)
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
        let _ = (interpreter_id, output_id);
        None
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
        if interpreter_id == ROOT_INTERPRETER_ID {
            Ok(Some(Vec::new()))
        } else {
            Ok(None)
        }
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
        Err(missing_resident_symbol("root_symbol_value", name))
    }

    pub fn root_symbol_values(
        &self,
        names: &[&str],
    ) -> MResult<Vec<(String, RuntimeValueSnapshot)>> {
        #[cfg(feature = "resident-routing")]
        if self.resident_artifact_and_instance().is_some() {
            return self.resident_symbol_values(names.iter().copied());
        }
        match names.first() {
            Some(name) => Err(missing_resident_symbol("root_symbol_values", name)),
            None => Ok(Vec::new()),
        }
    }

    pub fn root_symbol_values_all(&self) -> MResult<Vec<(String, RuntimeValueSnapshot)>> {
        #[cfg(feature = "resident-routing")]
        if let Some((artifact, _)) = self.resident_artifact_and_instance() {
            return self.resident_symbol_values(
                artifact.outputs().iter().map(|output| output.name.as_str()),
            );
        }
        Ok(Vec::new())
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
            ActiveProgramExecution::None => None,
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

fn missing_resident_symbol(operation: &'static str, name: &str) -> MechError {
    MechError::new(
        RuntimeInvalidOperationError {
            operation,
            reason: format!("resident output symbol `{name}` was not found"),
        },
        None,
    )
}
