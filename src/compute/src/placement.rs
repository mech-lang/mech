use std::collections::{BTreeMap, BTreeSet};

use mech_core::snapshot::SequenceView;
use mech_core::{
    AccessMode, CellSlotId, ComputePlacement, DeliveryMode, ExternalInteraction, FloatWidth,
    NodeId, OutputConstruction, ResolvedOperationContract, SchemaBody,
};
use mech_engine::{
    ArtifactSource, BindingDeclaration, ComputeRegionDeclaration, ProducerReference,
    ProgramArtifact, SlotRole,
};

use crate::{
    ElementwiseLowering, display_operation, elementwise_lowering, resolve_compute_slot_dimensions,
    turn_required_nodes,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ComputeExecutionTarget {
    Structural,
    Cpu,
    Gpu,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodePlacement {
    pub node: NodeId,
    pub operation: String,
    pub target: ComputeExecutionTarget,
    pub reason: String,
    pub region: Option<u32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SlotResidence {
    Host,
    DeviceTemporary,
    DeviceState,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SlotPlacement {
    pub slot: CellSlotId,
    pub role: SlotRole,
    pub residence: SlotResidence,
    pub elements: Option<u64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum TransferDirection {
    Upload,
    Readback,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct TransferBoundary {
    pub direction: TransferDirection,
    pub slot: CellSlotId,
    pub consumer: Option<NodeId>,
    pub interface_name: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComputeRegionPlan {
    pub region: u32,
    pub name: Option<String>,
    pub requested: Option<ComputePlacement>,
    pub nodes: Vec<NodeId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlacementViolation {
    pub region: String,
    pub node: Option<NodeId>,
    pub reason: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ComputePhysicalPlan {
    pub nodes: Vec<NodePlacement>,
    pub slots: Vec<SlotPlacement>,
    pub transfers: Vec<TransferBoundary>,
    pub regions: Vec<ComputeRegionPlan>,
    pub violations: Vec<PlacementViolation>,
    pub fully_accelerated: bool,
}

/// Plans CPU/device residence and explicit transfer boundaries without
/// selecting or invoking a physical backend.
pub fn plan_compute_artifact(
    artifact: &ProgramArtifact,
    explicit_regions: &[ComputeRegionDeclaration],
) -> ComputePhysicalPlan {
    let turn_nodes = turn_required_nodes(artifact);
    let slot_dimensions = resolve_compute_slot_dimensions(artifact);
    let mut nodes = artifact
        .nodes()
        .iter()
        .map(|node| {
            let operation = display_operation(&node.operation);
            let (target, reason) = if turn_nodes.contains(&node.node) {
                classify_node(artifact, node, &slot_dimensions)
            } else {
                (
                    ComputeExecutionTarget::Structural,
                    "initialization only; captured in the typed artifact".to_owned(),
                )
            };
            NodePlacement {
                node: node.node,
                operation,
                target,
                reason,
                region: None,
            }
        })
        .collect::<Vec<_>>();

    let mut explicit_by_node = BTreeMap::<NodeId, usize>::new();
    let mut violations = Vec::new();
    for (region_index, region) in explicit_regions.iter().enumerate() {
        if region.nodes.is_empty() {
            violations.push(PlacementViolation {
                region: region.name.to_string(),
                node: None,
                reason: "the named section produced no semantic artifact nodes".to_owned(),
            });
        }
        for node in region.nodes.iter().copied() {
            let Some(placement) = nodes.get_mut(node.get() as usize) else {
                violations.push(PlacementViolation {
                    region: region.name.to_string(),
                    node: Some(node),
                    reason: "the region references a node outside the artifact".to_owned(),
                });
                continue;
            };
            if let Some(previous) = explicit_by_node.insert(node, region_index) {
                violations.push(PlacementViolation {
                    region: region.name.to_string(),
                    node: Some(node),
                    reason: format!(
                        "the node is already assigned to region `{}`",
                        explicit_regions[previous].name,
                    ),
                });
                continue;
            }
            match region.placement {
                ComputePlacement::Compute => {
                    placement.reason = format!(
                        "named compute region `{}`; {}",
                        region.name, placement.reason,
                    );
                }
                ComputePlacement::Cpu => {
                    if placement.target != ComputeExecutionTarget::Structural {
                        placement.target = ComputeExecutionTarget::Cpu;
                    }
                    placement.reason = format!("region `{}` requires CPU execution", region.name);
                }
                ComputePlacement::Gpu => {
                    if placement.target == ComputeExecutionTarget::Cpu {
                        violations.push(PlacementViolation {
                            region: region.name.to_string(),
                            node: Some(node),
                            reason: placement.reason.clone(),
                        });
                        placement.reason = format!(
                            "region `{}` requires GPU execution, but {}",
                            region.name, placement.reason,
                        );
                    } else {
                        placement.reason = format!(
                            "region `{}` requires GPU execution; {}",
                            region.name, placement.reason,
                        );
                    }
                }
            }
        }
    }

    let mut parent = (0..nodes.len()).collect::<Vec<_>>();
    for node in artifact.nodes() {
        if nodes[node.node.get() as usize].target != ComputeExecutionTarget::Gpu {
            continue;
        }
        if explicit_by_node.contains_key(&node.node) {
            continue;
        }
        for index in node.input_bindings.clone() {
            let Some(BindingDeclaration::Input {
                source: ArtifactSource::Slot(slot),
                ..
            }) = artifact.bindings().get(index as usize)
            else {
                continue;
            };
            let ProducerReference::NodeOutput { node: producer, .. } =
                artifact.slots()[slot.get() as usize].producer
            else {
                continue;
            };
            if nodes[producer.get() as usize].target == ComputeExecutionTarget::Gpu
                && !explicit_by_node.contains_key(&producer)
            {
                union(
                    &mut parent,
                    node.node.get() as usize,
                    producer.get() as usize,
                );
            }
        }
    }

    let mut regions = Vec::<ComputeRegionPlan>::new();
    for (region_index, explicit) in explicit_regions.iter().enumerate() {
        let region_nodes = explicit
            .nodes
            .iter()
            .copied()
            .filter(|node| {
                nodes
                    .get(node.get() as usize)
                    .is_some_and(|placement| placement.target == ComputeExecutionTarget::Gpu)
            })
            .collect::<Vec<_>>();
        if region_nodes.is_empty() {
            continue;
        }
        let region = regions.len() as u32;
        for node in &region_nodes {
            nodes[node.get() as usize].region = Some(region);
        }
        debug_assert!(
            region_nodes
                .iter()
                .all(|node| explicit_by_node.get(node) == Some(&region_index))
        );
        regions.push(ComputeRegionPlan {
            region,
            name: Some(explicit.name.to_string()),
            requested: Some(explicit.placement),
            nodes: region_nodes,
        });
    }

    let mut roots = BTreeMap::<usize, u32>::new();
    for placement in &mut nodes {
        if placement.target != ComputeExecutionTarget::Gpu || placement.region.is_some() {
            continue;
        }
        let root = find(&mut parent, placement.node.get() as usize);
        let region = *roots.entry(root).or_insert_with(|| {
            let region = regions.len() as u32;
            regions.push(ComputeRegionPlan {
                region,
                name: None,
                requested: None,
                nodes: Vec::new(),
            });
            region
        });
        placement.region = Some(region);
        regions[region as usize].nodes.push(placement.node);
    }

    let consumers = slot_consumers(artifact);
    let output_names = artifact
        .outputs()
        .iter()
        .flat_map(|output| {
            physical_output_sources(artifact, output.source)
                .into_iter()
                .enumerate()
                .map(|(index, slot)| {
                    let name = if index == 0 {
                        output.name.clone()
                    } else {
                        format!("{}.{}", output.name, index)
                    };
                    (slot, name)
                })
                .collect::<Vec<_>>()
        })
        .collect::<BTreeMap<_, _>>();
    let mut transfers = BTreeSet::new();
    for node in artifact.nodes() {
        let consumer_target = nodes[node.node.get() as usize].target;
        for index in node.input_bindings.clone() {
            let Some(BindingDeclaration::Input {
                source: ArtifactSource::Slot(slot),
                ..
            }) = artifact.bindings().get(index as usize)
            else {
                continue;
            };
            let producer_target = producer_target(artifact, &nodes, *slot);
            match (producer_target, consumer_target) {
                (
                    ComputeExecutionTarget::Cpu | ComputeExecutionTarget::Structural,
                    ComputeExecutionTarget::Gpu,
                ) => {
                    transfers.insert(TransferBoundary {
                        direction: TransferDirection::Upload,
                        slot: *slot,
                        consumer: Some(node.node),
                        interface_name: artifact
                            .inputs()
                            .iter()
                            .find(|input| input.slot == *slot)
                            .map(|input| input.name.clone()),
                    });
                }
                (ComputeExecutionTarget::Gpu, ComputeExecutionTarget::Cpu) => {
                    transfers.insert(TransferBoundary {
                        direction: TransferDirection::Readback,
                        slot: *slot,
                        consumer: Some(node.node),
                        interface_name: None,
                    });
                }
                _ => {}
            }
        }
    }
    for output in artifact.outputs() {
        let sources = physical_output_sources(artifact, output.source);
        for (index, source) in sources.iter().enumerate() {
            if producer_target(artifact, &nodes, *source) == ComputeExecutionTarget::Gpu {
                transfers.insert(TransferBoundary {
                    direction: TransferDirection::Readback,
                    slot: *source,
                    consumer: None,
                    interface_name: Some(if sources.len() == 1 {
                        output.name.clone()
                    } else {
                        format!("{}.{index}", output.name)
                    }),
                });
            }
        }
    }

    let slots = artifact
        .slots()
        .iter()
        .map(|slot| {
            let produced_on_gpu =
                producer_target(artifact, &nodes, slot.slot) == ComputeExecutionTarget::Gpu;
            let consumed_only_on_gpu =
                consumers.get(&slot.slot).into_iter().flatten().all(|node| {
                    matches!(
                        nodes[node.get() as usize].target,
                        ComputeExecutionTarget::Gpu | ComputeExecutionTarget::Structural
                    )
                });
            let residence =
                if slot.role == SlotRole::State && produced_on_gpu && consumed_only_on_gpu {
                    SlotResidence::DeviceState
                } else if produced_on_gpu
                    && consumed_only_on_gpu
                    && !output_names.contains_key(&slot.slot)
                {
                    SlotResidence::DeviceTemporary
                } else {
                    SlotResidence::Host
                };
            SlotPlacement {
                slot: slot.slot,
                role: slot.role,
                residence,
                elements: schema_elements(
                    artifact,
                    slot.schema,
                    slot_dimensions.get(&slot.slot).map(Box::as_ref),
                ),
            }
        })
        .collect::<Vec<_>>();

    let fully_accelerated = violations.is_empty()
        && artifact.constraints().is_empty()
        && nodes
            .iter()
            .all(|node| node.target != ComputeExecutionTarget::Cpu);
    ComputePhysicalPlan {
        nodes,
        slots,
        transfers: transfers.into_iter().collect(),
        regions,
        violations,
        fully_accelerated,
    }
}

fn classify_node(
    artifact: &ProgramArtifact,
    node: &mech_engine::NodeDeclaration,
    slot_dimensions: &BTreeMap<CellSlotId, Box<[u64]>>,
) -> (ComputeExecutionTarget, String) {
    if node.operation.module_path.as_ref() == ["core"]
        && node.operation.operation_name == "composite-pack"
    {
        return (
            ComputeExecutionTarget::Structural,
            "output shape only; no device kernel".to_owned(),
        );
    }
    let operation = display_operation(&node.operation);
    let state_output = node.output_bindings.clone().any(|index| {
        matches!(
            artifact.bindings().get(index as usize),
            Some(BindingDeclaration::Output { target, .. })
                if artifact.slots()[target.get() as usize].role == SlotRole::State
        )
    });
    if state_output {
        if node.operation.module_path.as_ref() == ["core"]
            && node.operation.operation_name == "assign"
            && contract_supported(artifact, node, true)
        {
            return (
                ComputeExecutionTarget::Gpu,
                "whole-value state commit remains device resident".to_owned(),
            );
        }
        return (
            ComputeExecutionTarget::Cpu,
            "state update is not an admitted whole-value Assign".to_owned(),
        );
    }
    let lowered_operation = elementwise_lowering(&node.operation);
    if lowered_operation.is_none() {
        return (
            ComputeExecutionTarget::Cpu,
            format!("{operation} has no GPU lowering"),
        );
    }
    let host_proven_concatenation =
        matches!(lowered_operation, Some(ElementwiseLowering::Concatenate(_)));
    if !host_proven_concatenation && !contract_supported(artifact, node, false) {
        return (
            ComputeExecutionTarget::Cpu,
            "operation contract does not prove pure full-write execution".to_owned(),
        );
    }
    let schemas_supported = node
        .input_bindings
        .clone()
        .chain(node.output_bindings.clone())
        .all(|index| match &artifact.bindings()[index as usize] {
            BindingDeclaration::Input {
                source: ArtifactSource::Slot(slot),
                ..
            } => schema_elements(
                artifact,
                artifact.slots()[slot.get() as usize].schema,
                slot_dimensions.get(slot).map(Box::as_ref),
            )
            .is_some(),
            BindingDeclaration::Input {
                source: ArtifactSource::Constant(constant),
                ..
            } => match artifact
                .constants()
                .get(*constant)
                .map(|value| value.data())
            {
                Some(mech_core::ValueData::F32(_)) => true,
                Some(mech_core::ValueData::Matrix(matrix)) => {
                    matches!(matrix.elements(), SequenceView::F32(_))
                }
                _ => false,
            },
            BindingDeclaration::Output { target, .. } => schema_elements(
                artifact,
                artifact.slots()[target.get() as usize].schema,
                slot_dimensions.get(target).map(Box::as_ref),
            )
            .is_some(),
        });
    if !schemas_supported {
        return (
            ComputeExecutionTarget::Cpu,
            "value schema or constant representation is not admitted".to_owned(),
        );
    }
    (
        ComputeExecutionTarget::Gpu,
        "pure element-wise f32 operation".to_owned(),
    )
}

fn contract_supported(
    artifact: &ProgramArtifact,
    node: &mech_engine::NodeDeclaration,
    state: bool,
) -> bool {
    let Some(ResolvedOperationContract::Declared(contract)) =
        artifact.contracts().get(node.contract)
    else {
        return false;
    };
    if contract.interaction != ExternalInteraction::Pure
        || contract
            .inputs
            .iter()
            .any(|input| input.access != AccessMode::Read || input.delivery != DeliveryMode::Signal)
    {
        return false;
    }
    contract.outputs.iter().all(|output| {
        output.access == AccessMode::Write
            && output.delivery == DeliveryMode::Signal
            && if state {
                matches!(
                    output.construction,
                    OutputConstruction::Replace { .. } | OutputConstruction::FullWrite { .. }
                )
            } else {
                matches!(output.construction, OutputConstruction::FullWrite { .. })
            }
    })
}

fn schema_elements(
    artifact: &ProgramArtifact,
    schema: mech_core::SchemaId,
    resolved_dimensions: Option<&[u64]>,
) -> Option<u64> {
    match artifact.schemas().get(schema)?.body() {
        SchemaBody::FloatingPoint(FloatWidth::W32) => Some(1),
        SchemaBody::Matrix { element, .. }
            if matches!(element.as_ref(), SchemaBody::FloatingPoint(FloatWidth::W32)) =>
        {
            resolved_dimensions?
                .iter()
                .try_fold(1_u64, |elements, extent| elements.checked_mul(*extent))
        }
        _ => None,
    }
}

fn producer_target(
    artifact: &ProgramArtifact,
    placements: &[NodePlacement],
    slot: CellSlotId,
) -> ComputeExecutionTarget {
    match artifact.slots()[slot.get() as usize].producer {
        ProducerReference::Input(_) => ComputeExecutionTarget::Cpu,
        ProducerReference::NodeOutput { node, .. } => placements[node.get() as usize].target,
        ProducerReference::Output { source, .. } => match source {
            ArtifactSource::Constant(_) => ComputeExecutionTarget::Cpu,
            ArtifactSource::Slot(source) => producer_target(artifact, placements, source),
        },
    }
}

fn slot_consumers(artifact: &ProgramArtifact) -> BTreeMap<CellSlotId, Vec<NodeId>> {
    let mut consumers = BTreeMap::<CellSlotId, Vec<NodeId>>::new();
    for binding in artifact.bindings() {
        if let BindingDeclaration::Input {
            node,
            source: ArtifactSource::Slot(slot),
            ..
        } = binding
        {
            consumers.entry(*slot).or_default().push(*node);
        }
    }
    consumers
}

fn physical_output_sources(artifact: &ProgramArtifact, slot: CellSlotId) -> Vec<CellSlotId> {
    let node = match artifact.slots()[slot.get() as usize].producer {
        ProducerReference::Output {
            source: ArtifactSource::Slot(source),
            ..
        } => return physical_output_sources(artifact, source),
        ProducerReference::Output {
            source: ArtifactSource::Constant(_),
            ..
        }
        | ProducerReference::Input(_) => return vec![slot],
        ProducerReference::NodeOutput { node, .. } => node,
    };
    let producer = &artifact.nodes()[node.get() as usize];
    if producer.operation.module_path.as_ref() != ["core"]
        || producer.operation.operation_name != "composite-pack"
    {
        return vec![slot];
    }
    producer
        .input_bindings
        .clone()
        .filter_map(|index| match artifact.bindings().get(index as usize) {
            Some(BindingDeclaration::Input {
                source: ArtifactSource::Slot(slot),
                ..
            }) => Some(*slot),
            _ => None,
        })
        .collect()
}

fn find(parent: &mut [usize], index: usize) -> usize {
    if parent[index] != index {
        parent[index] = find(parent, parent[index]);
    }
    parent[index]
}

fn union(parent: &mut [usize], left: usize, right: usize) {
    let left = find(parent, left);
    let right = find(parent, right);
    if left != right {
        parent[right] = left;
    }
}
