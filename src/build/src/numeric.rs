//! Native numeric-region analysis for resolved Mech program artifacts.
//!
//! This module establishes the build-time partitioning seam without changing
//! bytecode or generated-runtime behavior. Backends consume only regions they
//! accept; ordinary resident nodes remain the fallback for every rejection.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use mech_core::{
    CellSlotId, ExternalInteraction, FunctionCatalog, NodeId, ParsedProgram, ReactiveInstanceId,
    ResidentValueKind, ResolvedOperationContract,
};
use mech_engine::__resident::{
    ActivatedNode, ActivatedPlan, ActivationFacts, ResidentReadLocation, activate,
};
use mech_engine::artifact::{
    ArtifactSource, BindingDeclaration, ProducerReference, ProgramArtifact, SlotRole,
    decode_program_artifact_sections,
};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum NativeNumericSource {
    Constant(u32),
    Slot(u32),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeNumericRegion {
    pub nodes: Vec<u32>,
    pub live_inputs: Vec<NativeNumericSource>,
    pub live_outputs: Vec<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeNumericNodeRejection {
    pub node: u32,
    pub operation: String,
    pub reason: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeNumericAnalysis {
    pub program_revision: [u8; 32],
    pub regions: Vec<NativeNumericRegion>,
    pub rejections: Vec<NativeNumericNodeRejection>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeNumericAnalysisError {
    reason: String,
}

impl NativeNumericAnalysisError {
    fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

impl fmt::Display for NativeNumericAnalysisError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "native numeric analysis failed: {}", self.reason)
    }
}

impl std::error::Error for NativeNumericAnalysisError {}

pub fn analyze_bytecode(
    bytecode: &[u8],
    catalog: &FunctionCatalog,
) -> Result<NativeNumericAnalysis, NativeNumericAnalysisError> {
    let parsed = ParsedProgram::from_bytes(bytecode)
        .map_err(|error| NativeNumericAnalysisError::new(error.display_message()))?;
    let artifact = decode_program_artifact_sections(&parsed.artifact).map_err(|error| {
        NativeNumericAnalysisError::new(format!("semantic artifact decode failed: {error:?}"))
    })?;
    let instance = activate(
        ReactiveInstanceId::new(0, 0),
        &artifact,
        catalog,
        &ActivationFacts::default(),
    )
    .map_err(|error| {
        NativeNumericAnalysisError::new(format!("resident activation failed: {error:?}"))
    })?;
    Ok(analyze_activated_artifact(&artifact, &instance.plan))
}

pub fn analyze_activated_artifact(
    artifact: &ProgramArtifact,
    plan: &ActivatedPlan,
) -> NativeNumericAnalysis {
    let mut eligible = vec![false; plan.nodes.len()];
    let mut rejections = Vec::new();
    for (index, node) in plan.nodes.iter().enumerate() {
        match node_eligibility(artifact, plan, node) {
            Ok(()) => eligible[index] = true,
            Err(reason) => {
                let declaration = artifact_node(artifact, node.artifact_node);
                rejections.push(NativeNumericNodeRejection {
                    node: node.artifact_node.get(),
                    operation: declaration
                        .map(qualified_operation_name)
                        .unwrap_or_else(|| "<missing>".to_owned()),
                    reason,
                });
            }
        }
    }

    let activated_by_artifact = plan
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.artifact_node, index))
        .collect::<BTreeMap<_, _>>();
    let mut disjoint = DisjointSet::new(plan.nodes.len());
    for (index, node) in plan.nodes.iter().enumerate() {
        if !eligible[index] {
            continue;
        }
        let Some(declaration) = artifact_node(artifact, node.artifact_node) else {
            continue;
        };
        for source in node_input_sources(artifact, declaration) {
            let ArtifactSource::Slot(slot) = source else {
                continue;
            };
            let Some(producer) = slot_producer(artifact, slot) else {
                continue;
            };
            let Some(&producer_index) = activated_by_artifact.get(&producer) else {
                continue;
            };
            if eligible[producer_index] {
                disjoint.union(index, producer_index);
            }
        }
    }

    let mut grouped = BTreeMap::<usize, Vec<usize>>::new();
    for node in &plan.topology.linear_node_order {
        let index = node.get() as usize;
        if eligible[index] {
            grouped.entry(disjoint.find(index)).or_default().push(index);
        }
    }

    let consumers = slot_consumers(artifact);
    let artifact_outputs = artifact
        .outputs()
        .iter()
        .map(|output| output.source)
        .collect::<BTreeSet<_>>();
    let constraint_inputs = artifact
        .constraints()
        .iter()
        .flat_map(|constraint| constraint.inputs.iter())
        .filter_map(|source| match source {
            ArtifactSource::Slot(slot) => Some(*slot),
            ArtifactSource::Constant(_) => None,
        })
        .collect::<BTreeSet<_>>();

    let mut regions = grouped
        .into_values()
        .map(|indices| {
            build_region(
                artifact,
                plan,
                &indices,
                &consumers,
                &artifact_outputs,
                &constraint_inputs,
            )
        })
        .collect::<Vec<_>>();
    regions.sort_by_key(|region| region.nodes.first().copied().unwrap_or(u32::MAX));
    rejections.sort_by_key(|rejection| rejection.node);

    NativeNumericAnalysis {
        program_revision: artifact.revision().into_bytes(),
        regions,
        rejections,
    }
}

fn node_eligibility(
    artifact: &ProgramArtifact,
    plan: &ActivatedPlan,
    node: &ActivatedNode,
) -> Result<(), String> {
    let declaration = artifact_node(artifact, node.artifact_node)
        .ok_or_else(|| "artifact node declaration is missing".to_owned())?;
    let contract = artifact
        .contracts()
        .get(declaration.contract)
        .ok_or_else(|| "operation contract is missing".to_owned())?;
    let ResolvedOperationContract::Declared(contract) = contract else {
        return Err("legacy opaque operation contract has no numeric semantics".to_owned());
    };
    if contract.interaction != ExternalInteraction::Pure {
        return Err(format!(
            "external interaction {:?} must remain in the host/runtime path",
            contract.interaction
        ));
    }
    if !operation_is_lowerable(&declaration.operation.operation_name) {
        return Err("operation has no native numeric opcode lowering".to_owned());
    }
    if node.write.region.kind != ResidentValueKind::F64 {
        return Err(format!(
            "output kind {:?} is not supported by the f64 numeric backend",
            node.write.region.kind
        ));
    }
    if plan.reads[node.reads.start as usize..node.reads.end as usize]
        .iter()
        .any(|read| read_region(*read).kind != ResidentValueKind::F64)
    {
        return Err("one or more inputs are not f64 resident values".to_owned());
    }
    Ok(())
}

fn build_region(
    artifact: &ProgramArtifact,
    plan: &ActivatedPlan,
    indices: &[usize],
    consumers: &BTreeMap<CellSlotId, BTreeSet<NodeId>>,
    artifact_outputs: &BTreeSet<CellSlotId>,
    constraint_inputs: &BTreeSet<CellSlotId>,
) -> NativeNumericRegion {
    let members = indices
        .iter()
        .map(|index| plan.nodes[*index].artifact_node)
        .collect::<BTreeSet<_>>();
    let mut live_inputs = BTreeSet::new();
    let mut live_outputs = BTreeSet::new();

    for index in indices {
        let node = &plan.nodes[*index];
        let Some(declaration) = artifact_node(artifact, node.artifact_node) else {
            continue;
        };
        for source in node_input_sources(artifact, declaration) {
            let external = match source {
                ArtifactSource::Constant(_) => true,
                ArtifactSource::Slot(slot) => {
                    slot_role(artifact, slot) == Some(SlotRole::State)
                        || slot_producer(artifact, slot)
                            .is_none_or(|producer| !members.contains(&producer))
                }
            };
            if external {
                live_inputs.insert(native_source(source));
            }
        }
        for target in node_output_slots(artifact, declaration) {
            let consumed_outside = consumers
                .get(&target)
                .is_some_and(|nodes| nodes.iter().any(|node| !members.contains(node)));
            if consumed_outside
                || artifact_outputs.contains(&target)
                || constraint_inputs.contains(&target)
                || slot_role(artifact, target) == Some(SlotRole::State)
            {
                live_outputs.insert(target.get());
            }
        }
    }

    NativeNumericRegion {
        nodes: indices
            .iter()
            .map(|index| plan.nodes[*index].artifact_node.get())
            .collect(),
        live_inputs: live_inputs.into_iter().collect(),
        live_outputs: live_outputs.into_iter().collect(),
    }
}

fn artifact_node(
    artifact: &ProgramArtifact,
    node: NodeId,
) -> Option<&mech_engine::artifact::NodeDeclaration> {
    artifact
        .nodes()
        .get(node.get() as usize)
        .filter(|declaration| declaration.node == node)
}

fn node_input_sources(
    artifact: &ProgramArtifact,
    node: &mech_engine::artifact::NodeDeclaration,
) -> Vec<ArtifactSource> {
    artifact.bindings()[node.input_bindings.start as usize..node.input_bindings.end as usize]
        .iter()
        .filter_map(|binding| match binding {
            BindingDeclaration::Input { source, .. } => Some(*source),
            BindingDeclaration::Output { .. } => None,
        })
        .collect()
}

fn node_output_slots(
    artifact: &ProgramArtifact,
    node: &mech_engine::artifact::NodeDeclaration,
) -> Vec<CellSlotId> {
    artifact.bindings()[node.output_bindings.start as usize..node.output_bindings.end as usize]
        .iter()
        .filter_map(|binding| match binding {
            BindingDeclaration::Output { target, .. } => Some(*target),
            BindingDeclaration::Input { .. } => None,
        })
        .collect()
}

fn slot_producer(artifact: &ProgramArtifact, slot: CellSlotId) -> Option<NodeId> {
    let declaration = artifact.slots().get(slot.get() as usize)?;
    match declaration.producer {
        ProducerReference::NodeOutput { node, .. } => Some(node),
        ProducerReference::Input(_) => None,
    }
}

fn slot_role(artifact: &ProgramArtifact, slot: CellSlotId) -> Option<SlotRole> {
    artifact
        .slots()
        .get(slot.get() as usize)
        .map(|slot| slot.role)
}

fn slot_consumers(artifact: &ProgramArtifact) -> BTreeMap<CellSlotId, BTreeSet<NodeId>> {
    let mut consumers = BTreeMap::<CellSlotId, BTreeSet<NodeId>>::new();
    for node in artifact.nodes() {
        for source in node_input_sources(artifact, node) {
            if let ArtifactSource::Slot(slot) = source {
                consumers.entry(slot).or_default().insert(node.node);
            }
        }
    }
    consumers
}

fn native_source(source: ArtifactSource) -> NativeNumericSource {
    match source {
        ArtifactSource::Constant(constant) => NativeNumericSource::Constant(constant.get()),
        ArtifactSource::Slot(slot) => NativeNumericSource::Slot(slot.get()),
    }
}

fn read_region(read: ResidentReadLocation) -> mech_engine::__resident::ResidentRegion {
    match read {
        ResidentReadLocation::Constant(region)
        | ResidentReadLocation::Input(region)
        | ResidentReadLocation::Scratch(region) => region,
        ResidentReadLocation::State { region, .. } => region,
    }
}

fn qualified_operation_name(node: &mech_engine::artifact::NodeDeclaration) -> String {
    if node.operation.module_path.is_empty() {
        node.operation.operation_name.clone()
    } else {
        format!(
            "{}/{}",
            node.operation.module_path.join("/"),
            node.operation.operation_name
        )
    }
}

fn operation_is_lowerable(name: &str) -> bool {
    name.starts_with("HorizontalConcatenate")
        || name.starts_with("VerticalConcatenate")
        || name.starts_with("MatMul")
        || name.starts_with("Transpose")
        || name.starts_with("Dot")
        || name.starts_with("Assign<")
        || name.starts_with("ConvertScalarToMat2")
        || name.starts_with("MathSin")
        || name.starts_with("MathCos")
        || name.starts_with("Negate")
        || name.starts_with("Atan2")
        || binary_operation_is_lowerable(name, "Add")
        || binary_operation_is_lowerable(name, "Sub")
        || binary_operation_is_lowerable(name, "Mul")
        || binary_operation_is_lowerable(name, "Div")
}

fn binary_operation_is_lowerable(name: &str, prefix: &str) -> bool {
    name.starts_with(prefix) && !name.starts_with(&format!("{prefix}Assign"))
}

struct DisjointSet {
    parent: Vec<usize>,
}

impl DisjointSet {
    fn new(len: usize) -> Self {
        Self {
            parent: (0..len).collect(),
        }
    }

    fn find(&mut self, value: usize) -> usize {
        let parent = self.parent[value];
        if parent == value {
            value
        } else {
            let root = self.find(parent);
            self.parent[value] = root;
            root
        }
    }

    fn union(&mut self, left: usize, right: usize) {
        let left = self.find(left);
        let right = self.find(right);
        if left != right {
            let (root, child) = if left < right {
                (left, right)
            } else {
                (right, left)
            };
            self.parent[child] = root;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lowerable_binary_families_exclude_read_modify_write_variants() {
        assert!(operation_is_lowerable("AddMDMD<f64>"));
        assert!(operation_is_lowerable("MulSS<f64>"));
        assert!(!operation_is_lowerable(
            "AddAssign2DRAV<f64DMatrixDMatrixDVector>"
        ));
        assert!(!operation_is_lowerable("NChooseKMatrix<f64>"));
    }

    #[test]
    fn disjoint_set_uses_deterministic_smallest_root() {
        let mut set = DisjointSet::new(5);
        set.union(4, 2);
        set.union(3, 4);
        assert_eq!(set.find(2), 2);
        assert_eq!(set.find(3), 2);
        assert_eq!(set.find(4), 2);
    }
}
