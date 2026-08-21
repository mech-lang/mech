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
                .map(|output| {
                    output
                        .interactive_binding
                        .as_ref()
                        .map(|binding| binding.lexical_name.clone())
                        .unwrap_or_else(|| output.name.clone())
                });
        }
        let _ = output_id;
        None
    }

    /// Return the resident output that represents the program's implicit
    /// result. Formatted-document projections precede this result and
    /// interactive symbol aliases follow it. Integrity constraints are a
    /// separate inspection surface and never become the program display.
    pub fn program_output_id(&self) -> Option<OutputId> {
        #[cfg(feature = "resident-routing")]
        if let Some((artifact, _)) = self.resident_artifact_and_instance() {
            return artifact
                .outputs()
                .iter()
                .rfind(|output| {
                    output.interactive_binding.is_none()
                        && !artifact
                            .constraints()
                            .iter()
                            .any(|constraint| constraint.name == output.name)
                })
                .map(|output| output.output);
        }
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
        if let Some((artifact, _)) = self.resident_artifact_and_instance() {
            if artifact
                .constraints()
                .iter()
                .any(|constraint| constraint.name == name)
            {
                return Err(missing_resident_symbol("root_symbol_value", name));
            }
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

    /// Return the resident output identity that owns an interactive root
    /// symbol. Names are lookup labels; the output ID is the binding identity
    /// that reflective hosts use to correlate repeated representations.
    pub fn root_symbol_output_id(&self, name: &str) -> Option<OutputId> {
        #[cfg(feature = "resident-routing")]
        if let Some((artifact, _)) = self.resident_artifact_and_instance() {
            if artifact
                .constraints()
                .iter()
                .any(|constraint| constraint.name == name)
            {
                return None;
            }
            return artifact
                .outputs()
                .iter()
                .find(|output| {
                    output
                        .interactive_binding
                        .as_ref()
                        .is_some_and(|binding| binding.lexical_name == name)
                })
                .or_else(|| artifact.outputs().iter().find(|output| output.name == name))
                .map(|output| output.output);
        }
        let _ = name;
        None
    }

    pub fn root_symbol_values(
        &self,
        names: &[&str],
    ) -> MResult<Vec<(String, RuntimeValueSnapshot)>> {
        #[cfg(feature = "resident-routing")]
        if let Some((artifact, _)) = self.resident_artifact_and_instance() {
            let ordinary_names = names
                .iter()
                .copied()
                .filter(|name| {
                    !artifact
                        .constraints()
                        .iter()
                        .any(|constraint| constraint.name == *name)
                })
                .collect::<Vec<_>>();
            return self.resident_symbol_values(ordinary_names);
        }
        match names.first() {
            Some(name) => Err(missing_resident_symbol("root_symbol_values", name)),
            None => Ok(Vec::new()),
        }
    }

    pub fn root_symbol_values_all(&self) -> MResult<Vec<(String, RuntimeValueSnapshot)>> {
        #[cfg(feature = "resident-routing")]
        if let Some((artifact, _)) = self.resident_artifact_and_instance() {
            let lexical_names = artifact
                .outputs()
                .iter()
                .filter_map(|output| {
                    output
                        .interactive_binding
                        .as_ref()
                        .map(|binding| binding.lexical_name.clone())
                })
                .filter(|name| {
                    !artifact
                        .constraints()
                        .iter()
                        .any(|constraint| constraint.name == *name)
                })
                .collect::<Vec<_>>();
            if !lexical_names.is_empty() {
                return self.resident_symbol_values(lexical_names.iter().map(String::as_str));
            }
            return self.resident_symbol_values(artifact.outputs().iter().filter_map(|output| {
                (!artifact
                    .constraints()
                    .iter()
                    .any(|constraint| constraint.name == output.name))
                .then_some(output.name.as_str())
            }));
        }
        Ok(Vec::new())
    }

    /// Return live integrity-constraint results as their own interactive
    /// projection. Constraints are artifact declarations, not ordinary root
    /// symbols, even though the compiler publishes both through output cells.
    pub fn root_integrity_constraint_values(
        &self,
        names: &[&str],
    ) -> MResult<Vec<(String, RuntimeValueSnapshot)>> {
        #[cfg(feature = "resident-routing")]
        if let Some((artifact, _)) = self.resident_artifact_and_instance() {
            let constraint_names = artifact
                .constraints()
                .iter()
                .map(|constraint| constraint.name.as_str())
                .filter(|name| names.is_empty() || names.contains(name))
                .collect::<Vec<_>>();
            return self.resident_symbol_values(constraint_names);
        }
        let _ = names;
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
                .position(|output| {
                    output
                        .interactive_binding
                        .as_ref()
                        .is_some_and(|binding| binding.lexical_name == name)
                })
                .or_else(|| {
                    artifact
                        .outputs()
                        .iter()
                        .position(|output| output.name == name)
                })
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
