#[cfg(feature = "resident-routing-source")]
#[path = "query_control.rs"]
mod control_comparison;
#[cfg(feature = "resident-routing-source")]
use control_comparison::node_bodies_semantically_equal;

use crate::RuntimeValueSnapshot;
use crate::runtime::{MechRuntime, RuntimeInvalidOperationError};
use mech_core::{MResult, MechError, OutputId};

#[cfg(feature = "resident-routing-source")]
use mech_engine::resident::StateMigrationMapping;
#[cfg(feature = "resident-routing-source")]
use mech_engine::{
    ArtifactSource, BindingDeclaration, InitializerReference, ProducerReference, ProgramArtifact,
    SlotDeclaration, SlotRole,
};

impl MechRuntime {
    /// Carry compatible live resident storage into an independently activated
    /// replacement. The candidate remains discardable until this succeeds.
    #[cfg(feature = "resident-routing-source")]
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
        if !mappings.is_empty() {
            let mapped_targets = mappings
                .iter()
                .map(|mapping| mapping.target)
                .collect::<std::collections::BTreeSet<_>>();
            let projection_refresh_targets = candidate_artifact
                .outputs()
                .iter()
                .map(|output| output.source)
                .filter(|slot| {
                    !mapped_targets.contains(slot)
                        && candidate_artifact
                            .slots()
                            .get(slot.get() as usize)
                            .is_some_and(|declaration| declaration.role == SlotRole::Output)
                })
                .collect::<std::collections::BTreeSet<_>>();

            let candidate_artifact = std::sync::Arc::new(candidate_artifact.clone());
            let (_, candidate_instance) = self
                .resident_artifact_and_instance_mut()
                .expect("candidate artifact was checked above");
            candidate_instance
                .migrate_compatible_state_from(previous_instance, &mappings)
                .map_err(|error| {
                    super::diagnostics::activation_failure_for_artifact(&candidate_artifact, error)
                })?;
            candidate_instance
                .refresh_output_projections(&candidate_artifact, &projection_refresh_targets)
                .map_err(super::diagnostics::projection_refresh_failure)?;
        }

        use crate::runtime::program::ActiveProgramExecution;
        if let (
            ActiveProgramExecution::ResidentExternal(candidate),
            ActiveProgramExecution::ResidentExternal(previous),
        ) = (&mut self.active_program, &previous.active_program)
        {
            candidate
                .coordinator
                .preserve_compatible_live_inputs_from(&previous.coordinator)?;
        }
        Ok(())
    }

    pub fn root_plan_len(&self) -> usize {
        #[cfg(feature = "resident-routing")]
        if let Some((_, instance)) = self.resident_artifact_and_instance() {
            return instance.plan.execution_node_count();
        }
        0
    }

    pub fn output_value(
        &self,
        #[cfg(feature = "resident-routing")] output_id: OutputId,
        #[cfg(not(feature = "resident-routing"))] _: OutputId,
    ) -> MResult<Option<RuntimeValueSnapshot>> {
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
        Ok(None)
    }

    pub fn output_name(
        &self,
        #[cfg(feature = "resident-routing")] output_id: OutputId,
        #[cfg(not(feature = "resident-routing"))] _: OutputId,
    ) -> Option<String> {
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
        None
    }

    /// Return the implicit result's ordinary output identity, using its
    /// interactive binding when present. Integrity results stay separate.
    pub fn program_output_id(&self) -> Option<OutputId> {
        #[cfg(feature = "resident-routing")]
        if let Some((artifact, _)) = self.resident_artifact_and_instance() {
            return super::value::program_result_output_index(artifact)
                .map(|index| artifact.outputs()[index].output);
        }
        None
    }

    /// Snapshot the currently published implicit program result.
    ///
    /// Replacement runtimes may migrate live state and refresh derived
    /// projections after activation, so callers must read this value from the
    /// accepted candidate instead of retaining its activation-time snapshot.
    pub fn program_output_value(&self) -> MResult<Option<RuntimeValueSnapshot>> {
        let Some(output_id) = self.program_output_id() else {
            return Ok(None);
        };
        self.output_value(output_id)
    }

    pub fn program_output_values(
        &self,
        #[cfg(feature = "resident-routing")] names: &[String],
        #[cfg(not(feature = "resident-routing"))] _: &[String],
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
    pub fn root_symbol_output_id(
        &self,
        #[cfg(feature = "resident-routing")] name: &str,
        #[cfg(not(feature = "resident-routing"))] _: &str,
    ) -> Option<OutputId> {
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
        #[cfg(feature = "resident-routing")] names: &[&str],
        #[cfg(not(feature = "resident-routing"))] _: &[&str],
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

    #[cfg(feature = "resident-routing-source")]
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

#[cfg(feature = "resident-routing-source")]
fn compatible_state_mappings(
    source: &mech_engine::ProgramArtifact,
    target: &mech_engine::ProgramArtifact,
    changed_state_names: &std::collections::BTreeSet<String>,
) -> Vec<StateMigrationMapping> {
    use std::collections::{BTreeMap, BTreeSet};

    let source_named = source
        .interactive_symbol_bindings()
        .filter(|binding| binding.lexical_name != "ans")
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
    let source_named_states = source_named.values().copied().collect::<BTreeSet<_>>();
    let target_named_states = target
        .interactive_symbol_bindings()
        .filter(|binding| binding.lexical_name != "ans")
        .filter_map(|binding| match binding.artifact_source {
            ArtifactSource::Slot(slot)
                if target
                    .slots()
                    .get(slot.get() as usize)
                    .is_some_and(|declaration| declaration.role == SlotRole::State) =>
            {
                Some(slot)
            }
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let mut mapped_sources = BTreeSet::new();
    let mut mapped_targets = BTreeSet::new();
    let excluded_targets = target
        .interactive_symbol_bindings()
        .filter(|binding| {
            binding.lexical_name != "ans" && changed_state_names.contains(&binding.lexical_name)
        })
        .filter_map(|binding| match binding.artifact_source {
            ArtifactSource::Slot(slot) => Some(slot),
            ArtifactSource::Constant(_) => None,
        })
        .collect::<BTreeSet<_>>();
    let affected_targets = artifact_slots_depending_on(target, &excluded_targets);
    let mut semantic_equivalence = BTreeMap::new();
    let mut compatible_state_pairs = BTreeSet::new();
    let mut mappings = Vec::new();

    let mut named_candidates = target
        .interactive_symbol_bindings()
        // `ans` is a synthetic projection of the document result, not a
        // declaration identity. Letting it participate can pair the previous
        // final state with an unrelated target state before that state's real
        // lexical name is considered.
        .filter(|binding| {
            binding.lexical_name != "ans" && !changed_state_names.contains(&binding.lexical_name)
        })
        .filter_map(|binding| {
            let ArtifactSource::Slot(target_slot) = binding.artifact_source else {
                return None;
            };
            // Several lexical projections can name the same state cell (`ans`
            // and the source variable are the common case). Once an explicitly
            // mutated name excludes that cell, none of its aliases may migrate it
            // back from the retired runtime.
            if excluded_targets.contains(&target_slot) {
                return None;
            }
            source_named
                .get(binding.lexical_name.as_str())
                .copied()
                .map(|source_slot| (source_slot, target_slot))
        })
        .collect::<Vec<_>>();

    // Interactive symbols are published in lexical-name order, while state
    // initializers follow source dependency order. Resolve named states to a
    // fixed point so a state such as `a := z` can wait for the compatible `z`
    // pair even though `a` is visited first. Clear cached negative comparisons
    // between passes because an unresolved state boundary can become valid as
    // compatible dependency pairs are added.
    loop {
        semantic_equivalence.clear();
        let mut progress = false;
        named_candidates.retain(|&(source_slot, target_slot)| {
            if mapped_sources.contains(&source_slot) || mapped_targets.contains(&target_slot) {
                return false;
            }
            let compatible = target
                .slots()
                .get(target_slot.get() as usize)
                .zip(source.slots().get(source_slot.get() as usize))
                .is_some_and(|(target_declaration, source_declaration)| {
                    target_declaration.role == SlotRole::State
                        && source_declaration.role == SlotRole::State
                        && target.schemas().get(target_declaration.schema)
                            == source.schemas().get(source_declaration.schema)
                        && state_initializers_semantically_equal(
                            source,
                            source_declaration,
                            target,
                            target_declaration,
                            &compatible_state_pairs,
                            &mut semantic_equivalence,
                        )
                });
            if compatible {
                mapped_sources.insert(source_slot);
                mapped_targets.insert(target_slot);
                compatible_state_pairs.insert((source_slot, target_slot));
                mappings.push(StateMigrationMapping {
                    source: source_slot,
                    target: target_slot,
                });
                progress = true;
                false
            } else {
                true
            }
        });
        if !progress {
            break;
        }
    }
    semantic_equivalence.clear();

    // Only genuinely unnamed state may use structural matching. Artifact-local
    // producer positions do not establish lexical declaration identity: a named
    // state whose named match failed must retain its replacement initializer,
    // and a retired named state must not supply an unnamed replacement cell.
    for target_slot in target
        .slots()
        .iter()
        .filter(|slot| slot.role == SlotRole::State && !target_named_states.contains(&slot.slot))
    {
        if mapped_targets.contains(&target_slot.slot)
            || excluded_targets.contains(&target_slot.slot)
        {
            continue;
        }
        let Some(source_slot) = source.slots().iter().find(|source_slot| {
            source_slot.role == SlotRole::State
                && !source_named_states.contains(&source_slot.slot)
                && source_slot.producer == target_slot.producer
                && source.schemas().get(source_slot.schema)
                    == target.schemas().get(target_slot.schema)
                && state_initializers_semantically_equal(
                    source,
                    source_slot,
                    target,
                    target_slot,
                    &compatible_state_pairs,
                    &mut semantic_equivalence,
                )
                && !mapped_sources.contains(&source_slot.slot)
        }) else {
            continue;
        };
        mapped_sources.insert(source_slot.slot);
        mapped_targets.insert(target_slot.slot);
        compatible_state_pairs.insert((source_slot.slot, target_slot.slot));
        mappings.push(StateMigrationMapping {
            source: source_slot.slot,
            target: target_slot.slot,
        });
    }

    // A replacement is activated from source defaults before compatible live
    // state is installed. Its materialized derived/output slots therefore
    // describe that default snapshot, not the migrated state snapshot. When
    // the submission did not explicitly mutate state, migrate those persistent
    // projections by their public output identity as one transaction with the
    // state cells. Executing a synthetic turn here would advance transitions;
    // copying the already-published projections preserves the exact epoch.
    // A state mutation invalidates only projections that transitively read the
    // mutated cells. Independent projections remain part of the previous
    // published epoch and must migrate with their own compatible state.
    for target_output in target.outputs() {
        let target_slot = target_output.source;
        let target_lexical_name = target_output
            .interactive_binding
            .as_ref()
            .map(|binding| binding.lexical_name.as_str());
        let semantic_source = target_output
            .interactive_binding
            .as_ref()
            .map(|binding| binding.artifact_source)
            .or_else(
                || match target.slots().get(target_slot.get() as usize)?.producer {
                    ProducerReference::Output { source, .. } => Some(source),
                    _ => Some(ArtifactSource::Slot(target_slot)),
                },
            );
        if mapped_targets.contains(&target_slot)
            || target_lexical_name == Some("ans")
            || matches!(semantic_source, Some(ArtifactSource::Slot(slot)) if affected_targets.contains(&slot))
            || !target
                .slots()
                .get(target_slot.get() as usize)
                .is_some_and(|slot| slot.role == SlotRole::Output)
        {
            continue;
        }
        let Some(target_semantic_source) = semantic_source else {
            continue;
        };
        let Some(source_output) = source.outputs().iter().find(|source_output| {
            let source_semantic_source = source_output
                .interactive_binding
                .as_ref()
                .map(|binding| binding.artifact_source)
                .or_else(|| {
                    match source
                        .slots()
                        .get(source_output.source.get() as usize)?
                        .producer
                    {
                        ProducerReference::Output { source, .. } => Some(source),
                        _ => Some(ArtifactSource::Slot(source_output.source)),
                    }
                });
            source_output.name == target_output.name
                && source_output
                    .interactive_binding
                    .as_ref()
                    .map(|binding| binding.lexical_name.as_str())
                    == target_lexical_name
                && !mapped_sources.contains(&source_output.source)
                && source
                    .slots()
                    .get(source_output.source.get() as usize)
                    .is_some_and(|slot| slot.role == SlotRole::Output)
                && source_semantic_source.is_some_and(|source_semantic_source| {
                    artifact_sources_semantically_equal(
                        source,
                        source_semantic_source,
                        target,
                        target_semantic_source,
                        &compatible_state_pairs,
                        &mut semantic_equivalence,
                    )
                })
        }) else {
            continue;
        };
        mapped_sources.insert(source_output.source);
        mapped_targets.insert(target_slot);
        mappings.push(StateMigrationMapping {
            source: source_output.source,
            target: target_slot,
        });
    }
    mappings
}

#[cfg(feature = "resident-routing-source")]
fn state_initializers_semantically_equal(
    source_artifact: &ProgramArtifact,
    source: &SlotDeclaration,
    target_artifact: &ProgramArtifact,
    target: &SlotDeclaration,
    compatible_state_pairs: &std::collections::BTreeSet<(
        mech_core::CellSlotId,
        mech_core::CellSlotId,
    )>,
    cache: &mut std::collections::BTreeMap<(ArtifactSource, ArtifactSource), bool>,
) -> bool {
    let source = match source.initializer {
        Some(InitializerReference::Constant(constant)) => ArtifactSource::Constant(constant),
        Some(InitializerReference::Activation(slot)) => ArtifactSource::Slot(slot),
        None => return target.initializer.is_none(),
    };
    let target = match target.initializer {
        Some(InitializerReference::Constant(constant)) => ArtifactSource::Constant(constant),
        Some(InitializerReference::Activation(slot)) => ArtifactSource::Slot(slot),
        None => return false,
    };
    artifact_sources_semantically_equal(
        source_artifact,
        source,
        target_artifact,
        target,
        compatible_state_pairs,
        cache,
    )
}

#[cfg(feature = "resident-routing-source")]
fn artifact_slots_depending_on(
    artifact: &ProgramArtifact,
    roots: &std::collections::BTreeSet<mech_core::CellSlotId>,
) -> std::collections::BTreeSet<mech_core::CellSlotId> {
    let mut dependents = vec![Vec::new(); artifact.slots().len()];
    for declaration in artifact.slots() {
        let target = declaration.slot;
        let mut add_source = |source: ArtifactSource| {
            if let ArtifactSource::Slot(source) = source
                && let Some(slots) = dependents.get_mut(source.get() as usize)
            {
                slots.push(target);
            }
        };
        match declaration.producer {
            ProducerReference::Input(_) => {}
            ProducerReference::Output { source, .. } => add_source(source),
            ProducerReference::NodeOutput { node, .. } => {
                let Some(node) = artifact.nodes().get(node.get() as usize) else {
                    continue;
                };
                for binding in &artifact.bindings()
                    [node.input_bindings.start as usize..node.input_bindings.end as usize]
                {
                    if let BindingDeclaration::Input { source, .. } = binding {
                        add_source(*source);
                    }
                }
            }
        }
    }

    let mut affected = roots.clone();
    let mut pending = roots.iter().copied().collect::<Vec<_>>();
    while let Some(source) = pending.pop() {
        let Some(slots) = dependents.get(source.get() as usize) else {
            continue;
        };
        for target in slots {
            if affected.insert(*target) {
                pending.push(*target);
            }
        }
    }
    affected
}

#[cfg(feature = "resident-routing-source")]
fn artifact_sources_semantically_equal(
    source_artifact: &ProgramArtifact,
    source: ArtifactSource,
    target_artifact: &ProgramArtifact,
    target: ArtifactSource,
    compatible_state_pairs: &std::collections::BTreeSet<(
        mech_core::CellSlotId,
        mech_core::CellSlotId,
    )>,
    cache: &mut std::collections::BTreeMap<(ArtifactSource, ArtifactSource), bool>,
) -> bool {
    let root = (source, target);
    if let Some(equal) = cache.get(&root) {
        return *equal;
    }

    let mut pending = vec![root];
    let mut visited = std::collections::BTreeSet::new();
    while let Some((source, target)) = pending.pop() {
        if let Some(equal) = cache.get(&(source, target)) {
            if !equal {
                cache.insert(root, false);
                return false;
            }
            continue;
        }
        if !visited.insert((source, target)) {
            continue;
        }
        match (source, target) {
            (ArtifactSource::Constant(source), ArtifactSource::Constant(target)) => {
                let source = source_artifact
                    .constants()
                    .entry(source)
                    .map(|entry| entry.hash());
                let target = target_artifact
                    .constants()
                    .entry(target)
                    .map(|entry| entry.hash());
                if source.is_none() || source != target {
                    cache.insert(root, false);
                    return false;
                }
            }
            (ArtifactSource::Slot(source), ArtifactSource::Slot(target)) => {
                let (Some(source), Some(target)) = (
                    source_artifact.slots().get(source.get() as usize),
                    target_artifact.slots().get(target.get() as usize),
                ) else {
                    cache.insert(root, false);
                    return false;
                };
                if source.role != target.role
                    || source_artifact.schemas().get(source.schema)
                        != target_artifact.schemas().get(target.schema)
                {
                    cache.insert(root, false);
                    return false;
                }
                // A derived projection may only cross a state boundary through
                // the exact state-cell pair selected by migration planning.
                // Structural producer similarity is deliberately insufficient:
                // two same-shaped live states can have different values and a
                // replacement may rewire an otherwise identical expression
                // from one of them to the other.
                if source.role == SlotRole::State {
                    if !compatible_state_pairs.contains(&(source.slot, target.slot)) {
                        cache.insert(root, false);
                        return false;
                    }
                    continue;
                }
                match (source.producer, target.producer) {
                    (ProducerReference::Input(source), ProducerReference::Input(target)) => {
                        let (Some(source), Some(target)) = (
                            source_artifact.inputs().get(source.get() as usize),
                            target_artifact.inputs().get(target.get() as usize),
                        ) else {
                            cache.insert(root, false);
                            return false;
                        };
                        if source.name != target.name
                            || source_artifact.schemas().get(source.schema)
                                != target_artifact.schemas().get(target.schema)
                        {
                            cache.insert(root, false);
                            return false;
                        }
                    }
                    (
                        ProducerReference::Output { source, .. },
                        ProducerReference::Output { source: target, .. },
                    ) => pending.push((source, target)),
                    (
                        ProducerReference::NodeOutput {
                            node: source_node,
                            output_ordinal: source_ordinal,
                        },
                        ProducerReference::NodeOutput {
                            node: target_node,
                            output_ordinal: target_ordinal,
                        },
                    ) => {
                        let (Some(source_node), Some(target_node)) = (
                            source_artifact.nodes().get(source_node.get() as usize),
                            target_artifact.nodes().get(target_node.get() as usize),
                        ) else {
                            cache.insert(root, false);
                            return false;
                        };
                        if source_ordinal != target_ordinal
                            || !node_bodies_semantically_equal(
                                source_artifact,
                                &source_node.body,
                                target_artifact,
                                &target_node.body,
                            )
                        {
                            cache.insert(root, false);
                            return false;
                        }
                        let source_inputs =
                            source_artifact.bindings()[source_node.input_bindings.start as usize
                                ..source_node.input_bindings.end as usize]
                                .iter()
                                .filter_map(|binding| match binding {
                                    BindingDeclaration::Input {
                                        port_ordinal,
                                        source,
                                        ..
                                    } => Some((*port_ordinal, *source)),
                                    BindingDeclaration::Output { .. } => None,
                                });
                        let target_inputs =
                            target_artifact.bindings()[target_node.input_bindings.start as usize
                                ..target_node.input_bindings.end as usize]
                                .iter()
                                .filter_map(|binding| match binding {
                                    BindingDeclaration::Input {
                                        port_ordinal,
                                        source,
                                        ..
                                    } => Some((*port_ordinal, *source)),
                                    BindingDeclaration::Output { .. } => None,
                                });
                        let source_inputs = source_inputs.collect::<Vec<_>>();
                        let target_inputs = target_inputs.collect::<Vec<_>>();
                        if source_inputs.len() != target_inputs.len()
                            || source_inputs
                                .iter()
                                .zip(&target_inputs)
                                .any(|(source, target)| source.0 != target.0)
                        {
                            cache.insert(root, false);
                            return false;
                        }
                        pending.extend(
                            source_inputs
                                .into_iter()
                                .zip(target_inputs)
                                .map(|(source, target)| (source.1, target.1)),
                        );
                    }
                    _ => {
                        cache.insert(root, false);
                        return false;
                    }
                }
            }
            _ => {
                cache.insert(root, false);
                return false;
            }
        }
    }

    for pair in visited {
        cache.insert(pair, true);
    }
    true
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

#[cfg(all(test, feature = "resident-routing-source"))]
mod tests {
    use super::{ArtifactSource, ProgramArtifact, SlotRole, compatible_state_mappings};

    fn artifact(source: &str) -> ProgramArtifact {
        artifact_with_interactive_projection(source, true)
    }

    fn artifact_with_interactive_projection(source: &str, interactive: bool) -> ProgramArtifact {
        let mut compiler = crate::RuntimeBuilder::new()
            .function_catalog(mech_stdlib::source_catalog())
            .build_compiler()
            .unwrap();
        let document = crate::SourceDocument::parse_resolved(
            "test:state-migration",
            mech_syntax::document::Revision(0),
            source,
            mech_syntax::document::ParseConfig::default(),
        )
        .unwrap();
        let product = if interactive {
            compiler.compile_interactive_document(&document)
        } else {
            compiler.compile_document(&document)
        };
        product.unwrap().into_parts().0
    }

    fn named_state(artifact: &ProgramArtifact, name: &str) -> mech_core::CellSlotId {
        artifact
            .interactive_symbol_bindings()
            .find_map(|binding| (binding.lexical_name == name).then_some(binding.artifact_source))
            .and_then(|source| match source {
                ArtifactSource::Slot(slot)
                    if artifact
                        .slots()
                        .get(slot.get() as usize)
                        .is_some_and(|declaration| declaration.role == SlotRole::State) =>
                {
                    Some(slot)
                }
                ArtifactSource::Slot(_) | ArtifactSource::Constant(_) => None,
            })
            .unwrap_or_else(|| panic!("state binding {name}"))
    }

    #[test]
    fn named_state_initializer_dependencies_survive_shifted_producers() {
        let previous = artifact("~z := 0\n~a := z\nz += 1\na += 1\na\n");
        let shifted = artifact("~prefix := 1\nprefix += 1\n~z := 0\n~a := z\nz += 1\na += 1\na\n");
        let previous_z = named_state(&previous, "z");
        let previous_a = named_state(&previous, "a");
        let shifted_z = named_state(&shifted, "z");
        let shifted_a = named_state(&shifted, "a");
        assert_ne!(
            previous.slots()[previous_a.get() as usize].producer,
            shifted.slots()[shifted_a.get() as usize].producer,
            "the witness must not be recoverable through producer identity",
        );

        let mappings = compatible_state_mappings(&previous, &shifted, &Default::default());
        assert!(
            mappings
                .iter()
                .any(|mapping| { mapping.source == previous_z && mapping.target == shifted_z })
        );
        assert!(
            mappings
                .iter()
                .any(|mapping| { mapping.source == previous_a && mapping.target == shifted_a })
        );
    }

    #[test]
    fn renamed_state_cannot_reuse_an_artifact_local_producer_identity() {
        let previous = artifact("~a := 0\na += 1\na\n");
        let replacement = artifact("~b := 0\nb += 1\nb\n");
        let previous_a = named_state(&previous, "a");
        let replacement_b = named_state(&replacement, "b");
        assert_eq!(
            previous.slots()[previous_a.get() as usize].producer,
            replacement.slots()[replacement_b.get() as usize].producer,
            "the witness must reuse the same artifact-local producer identity",
        );

        let mappings = compatible_state_mappings(&previous, &replacement, &Default::default());
        assert!(
            mappings
                .iter()
                .all(|mapping| mapping.target != replacement_b),
            "a different named declaration must remain freshly initialized",
        );
    }

    #[test]
    fn structural_state_matching_requires_unnamed_cells_on_both_sides() {
        let source = "~a := 0\na += 1\na\n";
        let named = artifact(source);
        let unnamed = artifact_with_interactive_projection(source, false);
        assert_eq!(unnamed.interactive_symbol_bindings().count(), 0);
        let named_state = named_state(&named, "a");
        let unnamed_state = unnamed
            .slots()
            .iter()
            .find(|slot| slot.role == SlotRole::State)
            .unwrap()
            .slot;
        assert_eq!(
            named.slots()[named_state.get() as usize].producer,
            unnamed.slots()[unnamed_state.get() as usize].producer,
        );

        let named_to_unnamed = compatible_state_mappings(&named, &unnamed, &Default::default());
        assert!(
            named_to_unnamed
                .iter()
                .all(|mapping| mapping.target != unnamed_state)
        );
        let unnamed_to_named = compatible_state_mappings(&unnamed, &named, &Default::default());
        assert!(
            unnamed_to_named
                .iter()
                .all(|mapping| mapping.target != named_state)
        );

        let unnamed_to_unnamed = compatible_state_mappings(&unnamed, &unnamed, &Default::default());
        assert!(
            unnamed_to_unnamed.iter().any(|mapping| {
                mapping.source == unnamed_state && mapping.target == unnamed_state
            })
        );
    }

    #[test]
    fn deleted_state_cannot_migrate_into_an_inserted_same_shaped_declaration() {
        let previous = artifact("~a := 0\n~b := 0\na += 1\nb += 10\nb\n");
        let replacement = artifact("~c := 0\n~b := 0\nc += 1\nb += 10\nb\n");
        let previous_a = named_state(&previous, "a");
        let previous_b = named_state(&previous, "b");
        let replacement_c = named_state(&replacement, "c");
        let replacement_b = named_state(&replacement, "b");
        assert_eq!(
            previous.slots()[previous_a.get() as usize].producer,
            replacement.slots()[replacement_c.get() as usize].producer,
        );

        let mappings = compatible_state_mappings(&previous, &replacement, &Default::default());
        assert!(
            mappings
                .iter()
                .all(|mapping| mapping.target != replacement_c)
        );
        assert!(
            mappings
                .iter()
                .any(|mapping| { mapping.source == previous_b && mapping.target == replacement_b })
        );
    }

    #[test]
    fn reordered_same_shaped_states_migrate_only_by_their_lexical_names() {
        let previous = artifact("~a := 0\n~b := 0\na += 1\nb += 10\nb\n");
        let replacement = artifact("~b := 0\n~a := 0\na += 1\nb += 10\nb\n");
        let mappings = compatible_state_mappings(&previous, &replacement, &Default::default());

        for name in ["a", "b"] {
            let source = named_state(&previous, name);
            let target = named_state(&replacement, name);
            assert_ne!(
                previous.slots()[source.get() as usize].producer,
                replacement.slots()[target.get() as usize].producer,
                "reordering must change the artifact-local producer for {name}",
            );
            assert!(
                mappings
                    .iter()
                    .any(|mapping| { mapping.source == source && mapping.target == target })
            );
            assert!(
                mappings
                    .iter()
                    .all(|mapping| { mapping.target != target || mapping.source == source })
            );
        }
    }

    #[test]
    fn state_migration_requires_a_semantically_equal_initializer() {
        let previous = artifact("~counter := 0\ncounter += 1\ncounter\n");
        let compatible = artifact("~counter := 0\ncounter += 2\ncounter\n");
        let redefined =
            artifact("~prefix := 1\nprefix += 1\n~counter := 11\ncounter += 2\ncounter\n");
        let previous_state = named_state(&previous, "counter");
        let compatible_state = named_state(&compatible, "counter");
        let redefined_state = named_state(&redefined, "counter");
        assert_ne!(
            previous.slots()[previous_state.get() as usize].producer,
            redefined.slots()[redefined_state.get() as usize].producer,
            "the negative witness must also cover shifted producer identities",
        );

        let compatible_mappings =
            compatible_state_mappings(&previous, &compatible, &Default::default());
        assert!(compatible_mappings.iter().any(|mapping| {
            mapping.source == previous_state && mapping.target == compatible_state
        }));

        let redefined_mappings =
            compatible_state_mappings(&previous, &redefined, &Default::default());
        assert!(
            !redefined_mappings
                .iter()
                .any(|mapping| mapping.target == redefined_state)
        );
    }

    #[test]
    fn synthetic_ans_never_claims_an_unrelated_state_migration_pair() {
        let previous = artifact("~a := 0\n~b := 11\na += 7\nb += 5\nb\n");
        let replacement = artifact("~a := 11\n~b := 11\na\n");
        let previous_b = named_state(&previous, "b");
        let replacement_a = named_state(&replacement, "a");
        let replacement_b = named_state(&replacement, "b");

        let mappings = compatible_state_mappings(&previous, &replacement, &Default::default());
        assert!(
            mappings
                .iter()
                .any(|mapping| { mapping.source == previous_b && mapping.target == replacement_b })
        );
        assert!(
            mappings
                .iter()
                .all(|mapping| mapping.target != replacement_a),
            "the replacement's redefined `a` state must keep its new initializer",
        );
    }
}
