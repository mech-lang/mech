use crate::RuntimeValueSnapshot;
use crate::runtime::{MechRuntime, RuntimeInvalidOperationError};
use mech_core::{MResult, MechError, OutputId};

#[cfg(feature = "resident-routing")]
use mech_engine::resident::StateMigrationMapping;
#[cfg(feature = "resident-routing")]
use mech_engine::{
    ArtifactSource, BindingDeclaration, ProducerReference, ProgramArtifact, SlotRole,
};

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
                .refresh_output_projections(&projection_refresh_targets)
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
    let affected_targets = artifact_slots_depending_on(target, &excluded_targets);
    let mut semantic_equivalence = BTreeMap::new();
    let mut compatible_state_pairs = BTreeSet::new();
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
            .zip(source.slots().get(source_slot.get() as usize))
            .is_some_and(|(target_declaration, source_declaration)| {
                target_declaration.role == SlotRole::State
                    && source_declaration.role == SlotRole::State
                    && target.schemas().get(target_declaration.schema)
                        == source.schemas().get(source_declaration.schema)
            })
            && mapped_sources.insert(source_slot)
            && mapped_targets.insert(target_slot)
        {
            compatible_state_pairs.insert((source_slot, target_slot));
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
                && source.schemas().get(source_slot.schema)
                    == target.schemas().get(target_slot.schema)
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

#[cfg(feature = "resident-routing")]
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

#[cfg(feature = "resident-routing")]
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
                            || source_node.operation != target_node.operation
                            || source_node.requirement.and_then(|requirement| {
                                source_artifact.requirements().get(requirement)
                            }) != target_node.requirement.and_then(|requirement| {
                                target_artifact.requirements().get(requirement)
                            })
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
