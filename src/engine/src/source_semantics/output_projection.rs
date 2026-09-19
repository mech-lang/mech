//! Static symbol projections retain the canonical graph's dependency closure.

use super::*;

impl CanonicalSourceProgram {
    /// Select static binding outputs without evaluating unrelated document work.
    /// Dependencies, state writers and integrity constraints retain their existing
    /// semantics; this only removes nodes outside the selected dependency graph.
    pub fn retain_static_outputs(
        mut self,
        names: &BTreeSet<String>,
    ) -> Result<Self, SourceSemanticError> {
        let mut selected = BTreeSet::new();
        for name in names {
            let encoded = crate::encode_interactive_symbol_output_name(name);
            let output = self
                .program
                .outputs
                .iter()
                .position(|output| output.name == encoded)
                .or_else(|| {
                    self.program
                        .outputs
                        .iter()
                        .position(|output| output.name == *name)
                })
                .ok_or_else(|| SourceSemanticError {
                    code: "source-semantics/unknown-published-binding",
                    message: format!("document does not publish requested output {name}"),
                    anchor: self.source_map.outputs.first().copied().unwrap_or(
                        SourceSemanticAnchor {
                            document: DocumentId(0),
                            revision: Revision(0),
                            range: TextRange::empty(mech_syntax::document::TextSize::ZERO),
                        },
                    ),
                })?;
            selected.insert(output);
        }
        let mut pending = selected
            .iter()
            .map(|index| self.program.outputs[*index].source.clone())
            .chain(
                self.program
                    .constraints
                    .iter()
                    .flat_map(|constraint| constraint.inputs.iter().cloned()),
            )
            .collect::<Vec<_>>();
        let mut inputs = BTreeSet::new();
        let mut nodes = BTreeSet::new();
        let mut states = BTreeSet::new();
        while let Some(value) = pending.pop() {
            match value {
                SourceValue::Input(input) => {
                    inputs.insert(input as usize);
                }
                SourceValue::NodeOutput { node, .. } if nodes.insert(node as usize) => {
                    let node = &self.program.nodes[node as usize];
                    pending.extend(node.inputs.iter().cloned());
                    pending.extend(node.outputs.iter().filter_map(|output| match output {
                        SourceNodeOutput::State(state) => Some(SourceValue::State(*state)),
                        SourceNodeOutput::Derived { .. } => None,
                    }));
                }
                SourceValue::State(state) if states.insert(state as usize) => {
                    let state = &self.program.states[state as usize];
                    pending.extend(state.initializer.iter().cloned());
                    pending.push(SourceValue::NodeOutput {
                        node: state.producer_node,
                        output_ordinal: state.producer_output_ordinal,
                    });
                }
                _ => {}
            }
        }
        let input_ids = inputs
            .iter()
            .enumerate()
            .map(|(new, old)| (*old as u32, new as u32))
            .collect::<BTreeMap<_, _>>();
        let node_ids = nodes
            .iter()
            .enumerate()
            .map(|(new, old)| (*old as u32, new as u32))
            .collect::<BTreeMap<_, _>>();
        let state_ids = states
            .iter()
            .enumerate()
            .map(|(new, old)| (*old as u32, new as u32))
            .collect::<BTreeMap<_, _>>();
        let output_ids = selected
            .iter()
            .enumerate()
            .map(|(new, old)| (*old as u32, new as u32))
            .collect::<BTreeMap<_, _>>();
        let remap = |value: &mut SourceValue| match value {
            SourceValue::NodeOutput { node, .. } => *node = node_ids[node],
            SourceValue::State(state) => *state = state_ids[state],
            SourceValue::Input(input) => *input = input_ids[input],
            _ => {}
        };
        self.program.inputs = retain(self.program.inputs, &inputs);
        self.source_map.inputs = retain(self.source_map.inputs, &inputs);
        self.program.nodes = retain(self.program.nodes, &nodes);
        self.program.requirements =
            retain_node_requirements(&mut self.program.nodes, &self.program.requirements).map_err(
                |error| SourceSemanticError {
                    code: "source-semantics/invalid-static-requirements",
                    message: format!("retained static requirement table is invalid: {error:?}"),
                    anchor: self.source_map.outputs.first().copied().unwrap_or(
                        SourceSemanticAnchor {
                            document: DocumentId(0),
                            revision: Revision(0),
                            range: TextRange::empty(mech_syntax::document::TextSize::ZERO),
                        },
                    ),
                },
            )?;
        self.contracts = retain(self.contracts, &nodes);
        self.source_map.nodes = retain(self.source_map.nodes, &nodes);
        self.program.states = retain(self.program.states, &states);
        self.program.outputs = retain(self.program.outputs, &selected);
        self.source_map.outputs = retain(self.source_map.outputs, &selected);
        for node in &mut self.program.nodes {
            for input in &mut node.inputs {
                remap(input);
            }
            for output in &mut node.outputs {
                if let SourceNodeOutput::State(state) = output {
                    *state = state_ids[state];
                }
            }
        }
        for state in &mut self.program.states {
            state.producer_node = node_ids[&state.producer_node];
            if let Some(initializer) = &mut state.initializer {
                remap(initializer);
            }
        }
        for output in &mut self.program.outputs {
            remap(&mut output.source);
        }
        for constraint in &mut self.program.constraints {
            for input in &mut constraint.inputs {
                remap(input);
            }
        }
        self.document_outputs = self
            .document_outputs
            .into_vec()
            .into_iter()
            .filter_map(|mut output| {
                output.output = *output_ids.get(&output.output)?;
                Some(output)
            })
            .collect();
        self.document_exports = self
            .document_exports
            .into_vec()
            .into_iter()
            .filter_map(|mut output| {
                output.output = *output_ids.get(&output.output)?;
                Some(output)
            })
            .collect();
        Ok(self)
    }
}

fn retain_node_requirements(
    nodes: &mut [SourceNode],
    requirements: &crate::ApplicationRequirementTable,
) -> Result<crate::ApplicationRequirementTable, crate::ArtifactBuildError> {
    let retained = nodes
        .iter()
        .filter_map(|node| match &node.body {
            crate::SourceNodeBody::Operation {
                requirement: Some(requirement),
                ..
            } => Some(requirement.get()),
            crate::SourceNodeBody::Operation {
                requirement: None, ..
            }
            | crate::SourceNodeBody::Match(_)
            | crate::SourceNodeBody::Comprehension(_)
            | crate::SourceNodeBody::Fsm(_) => None,
        })
        .collect::<BTreeSet<_>>();
    let remap = retained
        .iter()
        .enumerate()
        .map(|(new, old)| (*old, mech_core::ApplicationRequirementId::new(new as u32)))
        .collect::<BTreeMap<_, _>>();
    let entries = retained
        .iter()
        .map(|old| {
            requirements
                .get(mech_core::ApplicationRequirementId::new(*old))
                .expect("canonical source node references a known requirement")
                .clone()
        })
        .collect::<Vec<_>>();

    for node in nodes {
        let crate::SourceNodeBody::Operation {
            requirement: Some(requirement),
            ..
        } = &mut node.body
        else {
            continue;
        };
        *requirement = remap[&requirement.get()];
    }

    crate::ApplicationRequirementTable::from_canonical_entries(entries)
}

fn retain<T>(items: Box<[T]>, selected: &BTreeSet<usize>) -> Box<[T]> {
    items
        .into_vec()
        .into_iter()
        .enumerate()
        .filter_map(|(index, item)| selected.contains(&index).then_some(item))
        .collect()
}

#[cfg(test)]
mod requirement_projection_tests {
    use super::*;

    fn requirement(path: &str) -> mech_core::ApplicationRequirement {
        mech_core::ApplicationRequirement::Resource(mech_core::ExecutionResourceRequest {
            base_uri: "test://context".to_owned(),
            path: path.to_owned(),
            context_name: "context".to_owned(),
            operation: "read".to_owned(),
            intent: mech_core::ResourceIntent::Read,
            delivery: mech_core::ResourceDelivery::Snapshot,
        })
    }

    #[test]
    fn retained_nodes_prune_and_remap_application_requirements() {
        let mut entries = vec![requirement("discarded"), requirement("retained")];
        entries.sort_by(mech_core::compare_application_requirements);
        let requirements =
            crate::ApplicationRequirementTable::from_canonical_entries(entries).unwrap();
        let retained_old = requirements
            .iter()
            .find_map(|(id, requirement)| {
                matches!(
                    requirement,
                    mech_core::ApplicationRequirement::Resource(request)
                        if request.path == "retained"
                )
                .then_some(id)
            })
            .unwrap();
        let mut nodes = vec![SourceNode {
            body: crate::SourceNodeBody::Operation {
                operation: OperationReference {
                    module_path: vec!["resource".to_owned(), "read".to_owned()].into_boxed_slice(),
                    operation_name: "read".to_owned(),
                },
                requirement: Some(retained_old),
            },
            inputs: Box::new([]),
            outputs: Box::new([]),
        }];

        let retained = retain_node_requirements(&mut nodes, &requirements).unwrap();
        assert_eq!(retained.len(), 1);
        assert!(matches!(
            retained.get(mech_core::ApplicationRequirementId::new(0)),
            Some(mech_core::ApplicationRequirement::Resource(request))
                if request.path == "retained"
        ));
        let crate::SourceNodeBody::Operation {
            requirement: Some(requirement),
            ..
        } = &nodes[0].body
        else {
            panic!("expected retained resource operation")
        };
        assert_eq!(requirement.get(), 0);
    }
}
