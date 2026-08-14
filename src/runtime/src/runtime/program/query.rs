use crate::RuntimeValueSnapshot;
use crate::runtime::{MechRuntime, RuntimeInvalidOperationError};
use mech_core::{MResult, MechError, OutputId};

impl MechRuntime {
    pub fn root_plan_len(&self) -> usize {
        #[cfg(feature = "resident-routing")]
        if let Some((_, instance)) = self.resident_artifact_and_instance() {
            return instance.plan.execution_node_count();
        }
        0
    }

    pub fn output_value(&self, output_id: OutputId) -> MResult<Option<RuntimeValueSnapshot>> {
        #[cfg(feature = "resident-routing")]
        if let Some((artifact, instance)) = self.resident_artifact_and_instance() {
            let Some(index) = artifact
                .outputs()
                .iter()
                .position(|output| output.output == output_id)
            else {
                return Ok(None);
            };
            return super::output_value(instance, index);
        }
        let _ = output_id;
        Ok(None)
    }

    pub fn output_name(&self, output_id: OutputId) -> Option<String> {
        #[cfg(feature = "resident-routing")]
        if let Some((artifact, _)) = self.resident_artifact_and_instance() {
            return artifact
                .outputs()
                .iter()
                .find(|output| output.output == output_id)
                .map(|output| output.name.clone());
        }
        let _ = output_id;
        None
    }

    pub fn program_output_values(
        &self,
        names: &[String],
    ) -> MResult<Vec<(String, RuntimeValueSnapshot)>> {
        #[cfg(feature = "resident-routing")]
        if self.resident_artifact_and_instance().is_some() {
            return self.resident_symbol_values(names.iter().map(String::as_str));
        }
        Ok(Vec::new())
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
        use crate::runtime::program::ActiveProgramExecution;
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
            let value = super::output_value(instance, index)?.ok_or_else(|| {
                MechError::new(
                    RuntimeInvalidOperationError {
                        operation: "resident_symbol_values",
                        reason: format!("resident output symbol `{name}` has no value"),
                    },
                    None,
                )
            })?;
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
