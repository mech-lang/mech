use crate::RuntimeValueSnapshot;
use crate::runtime::{MechRuntime, RuntimeInvalidOperationError};
use mech_core::{MResult, MechError, OutputId};

#[cfg(feature = "resident-routing")]
use mech_engine::resident::StateMigrationMapping;
#[cfg(feature = "resident-routing")]
use mech_engine::{ArtifactSource, SlotRole};

impl MechRuntime {
    /// Carry compatible live resident storage into an independently activated
    /// replacement. The candidate remains discardable until this succeeds.
    #[cfg(feature = "resident-routing")]
    pub(crate) fn preserve_compatible_resident_state_from(
        &mut self,
        previous: &MechRuntime,
        changed_state_names: &std::collections::BTreeSet<String>,
    ) -> MResult<()> {
        let Some((previous_artifact, previous_instance)) =
            previous.resident_artifact_and_instance()
        else {
            return Ok(());
        };
        let Some((candidate_artifact, _)) = self.resident_artifact_and_instance() else {
            return Ok(());
        };
        let mappings =
            compatible_state_mappings(previous_artifact, candidate_artifact, changed_state_names);
        if mappings.is_empty() {
            return Ok(());
        }

        let candidate_artifact = std::sync::Arc::new(candidate_artifact.clone());
        let (_, candidate_instance) = self
            .resident_artifact_and_instance_mut()
            .expect("candidate artifact was checked above");
        candidate_instance
            .migrate_compatible_state_from(previous_instance, &mappings)
            .map_err(|error| {
                super::diagnostics::activation_failure_for_artifact(&candidate_artifact, error)
            })?;
        Ok(())
    }

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
    fn resident_artifact_and_instance_mut(
        &mut self,
    ) -> Option<(
        &mech_engine::ProgramArtifact,
        &mut mech_engine::resident::ReactiveInstance,
    )> {
        use crate::runtime::program::ActiveProgramExecution;
        match &mut self.active_program {
            ActiveProgramExecution::ResidentPure(execution) => {
                Some((&execution.artifact, &mut execution.instance))
            }
            ActiveProgramExecution::ResidentExternal(execution) => {
                Some((&execution.artifact, execution.coordinator.instance_mut()))
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

#[cfg(feature = "resident-routing")]
fn compatible_state_mappings(
    source: &mech_engine::ProgramArtifact,
    target: &mech_engine::ProgramArtifact,
    changed_state_names: &std::collections::BTreeSet<String>,
) -> Vec<StateMigrationMapping> {
    use std::collections::{BTreeMap, BTreeSet};

    let source_named = source
        .interactive_symbol_bindings()
        .filter_map(|binding| match binding.artifact_source {
            ArtifactSource::Slot(slot)
                if source
                    .slots()
                    .get(slot.get() as usize)
                    .is_some_and(|declaration| declaration.role == SlotRole::State) =>
            {
                Some((binding.lexical_name.as_str(), slot))
            }
            _ => None,
        })
        .collect::<BTreeMap<_, _>>();
    let mut mapped_sources = BTreeSet::new();
    let mut mapped_targets = BTreeSet::new();
    let excluded_targets = target
        .interactive_symbol_bindings()
        .filter(|binding| changed_state_names.contains(&binding.lexical_name))
        .filter_map(|binding| match binding.artifact_source {
            ArtifactSource::Slot(slot) => Some(slot),
            ArtifactSource::Constant(_) => None,
        })
        .collect::<BTreeSet<_>>();
    let mut mappings = Vec::new();

    for binding in target.interactive_symbol_bindings() {
        if changed_state_names.contains(&binding.lexical_name) {
            continue;
        }
        let ArtifactSource::Slot(target_slot) = binding.artifact_source else {
            continue;
        };
        // Several lexical projections can name the same state cell (`ans`
        // and the source variable are the common case). Once an explicitly
        // mutated name excludes that cell, none of its aliases may migrate it
        // back from the retired runtime.
        if excluded_targets.contains(&target_slot) {
            continue;
        }
        let Some(source_slot) = source_named.get(binding.lexical_name.as_str()).copied() else {
            continue;
        };
        if target
            .slots()
            .get(target_slot.get() as usize)
            .is_some_and(|declaration| declaration.role == SlotRole::State)
            && mapped_sources.insert(source_slot)
            && mapped_targets.insert(target_slot)
        {
            mappings.push(StateMigrationMapping {
                source: source_slot,
                target: target_slot,
            });
        }
    }

    // Preserve non-exported state when its producer remains structurally
    // identical. This intentionally requires producer identity as well as slot
    // compatibility so deleting or reordering unrelated definitions cannot
    // migrate state into a different computation that merely reused an index.
    for target_slot in target
        .slots()
        .iter()
        .filter(|slot| slot.role == SlotRole::State)
    {
        if mapped_targets.contains(&target_slot.slot)
            || excluded_targets.contains(&target_slot.slot)
        {
            continue;
        }
        let Some(source_slot) = source.slots().iter().find(|source_slot| {
            source_slot.role == SlotRole::State
                && source_slot.producer == target_slot.producer
                && !mapped_sources.contains(&source_slot.slot)
        }) else {
            continue;
        };
        mapped_sources.insert(source_slot.slot);
        mapped_targets.insert(target_slot.slot);
        mappings.push(StateMigrationMapping {
            source: source_slot.slot,
            target: target_slot.slot,
        });
    }
    mappings
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
