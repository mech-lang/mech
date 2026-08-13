use std::collections::{BTreeMap, BTreeSet};

use mech_core::{
    AccessMode, CellSlotId, DeliveryMode, DimensionExpr, ExternalInteraction, FloatWidth, NodeId,
    OutputConstruction, ResolvedOperationContract, SchemaBody,
};
use mech_engine::{
    ArtifactSource, BindingDeclaration, ProducerReference, ProgramArtifact, SlotRole,
};

use super::{GpuHost, binary_operation, display_operation, turn_required_nodes};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecutionTarget {
    Structural,
    Cpu,
    Gpu,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodePlacement {
    pub node: NodeId,
    pub operation: String,
    pub target: ExecutionTarget,
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
pub struct GpuRegion {
    pub region: u32,
    pub nodes: Vec<NodeId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HybridPlacementPlan {
    pub nodes: Vec<NodePlacement>,
    pub slots: Vec<SlotPlacement>,
    pub transfers: Vec<TransferBoundary>,
    pub gpu_regions: Vec<GpuRegion>,
    pub fully_accelerated: bool,
}

impl GpuHost {
    /// Explains automatic placement without silently changing program semantics.
    /// CPU/GPU boundaries are reported even though mixed-region execution is not
    /// yet enabled by this provider.
    pub fn plan(&self, artifact: &ProgramArtifact) -> HybridPlacementPlan {
        plan_artifact(artifact)
    }
}

fn plan_artifact(artifact: &ProgramArtifact) -> HybridPlacementPlan {
    let turn_nodes = turn_required_nodes(artifact);
    let mut nodes = artifact
        .nodes()
        .iter()
        .map(|node| {
            let operation = display_operation(&node.operation);
            let (target, reason) = if turn_nodes.contains(&node.node) {
                classify_node(artifact, node)
            } else {
                (
                    ExecutionTarget::Structural,
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

    let mut parent = (0..nodes.len()).collect::<Vec<_>>();
    for node in artifact.nodes() {
        if nodes[node.node.get() as usize].target != ExecutionTarget::Gpu {
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
            if nodes[producer.get() as usize].target == ExecutionTarget::Gpu {
                union(
                    &mut parent,
                    node.node.get() as usize,
                    producer.get() as usize,
                );
            }
        }
    }

    let mut roots = BTreeMap::<usize, u32>::new();
    let mut gpu_regions = Vec::<GpuRegion>::new();
    for placement in &mut nodes {
        if placement.target != ExecutionTarget::Gpu {
            continue;
        }
        let root = find(&mut parent, placement.node.get() as usize);
        let region = *roots.entry(root).or_insert_with(|| {
            let region = gpu_regions.len() as u32;
            gpu_regions.push(GpuRegion {
                region,
                nodes: Vec::new(),
            });
            region
        });
        placement.region = Some(region);
        gpu_regions[region as usize].nodes.push(placement.node);
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
                (ExecutionTarget::Cpu | ExecutionTarget::Structural, ExecutionTarget::Gpu) => {
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
                (ExecutionTarget::Gpu, ExecutionTarget::Cpu) => {
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
            if producer_target(artifact, &nodes, *source) == ExecutionTarget::Gpu {
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
                producer_target(artifact, &nodes, slot.slot) == ExecutionTarget::Gpu;
            let consumed_only_on_gpu =
                consumers.get(&slot.slot).into_iter().flatten().all(|node| {
                    matches!(
                        nodes[node.get() as usize].target,
                        ExecutionTarget::Gpu | ExecutionTarget::Structural
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
                elements: schema_elements(artifact, slot.schema),
            }
        })
        .collect::<Vec<_>>();

    let fully_accelerated = artifact.constraints().is_empty()
        && nodes.iter().all(|node| node.target != ExecutionTarget::Cpu);
    HybridPlacementPlan {
        nodes,
        slots,
        transfers: transfers.into_iter().collect(),
        gpu_regions,
        fully_accelerated,
    }
}

fn classify_node(
    artifact: &ProgramArtifact,
    node: &mech_engine::NodeDeclaration,
) -> (ExecutionTarget, String) {
    if node.operation.module_path.as_ref() == ["runtime"]
        && node.operation.operation_name.starts_with("VariableDefine")
    {
        return (
            ExecutionTarget::Structural,
            "source name only; no runtime work".to_owned(),
        );
    }
    if node.operation.module_path.as_ref() == ["core"]
        && node.operation.operation_name == "composite-pack"
    {
        return (
            ExecutionTarget::Structural,
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
        if node.operation.module_path.as_ref() == ["runtime"]
            && node.operation.operation_name.starts_with("Assign")
            && contract_supported(artifact, node, true)
        {
            return (
                ExecutionTarget::Gpu,
                "whole-value state commit remains device resident".to_owned(),
            );
        }
        return (
            ExecutionTarget::Cpu,
            "state update is not an admitted whole-value Assign".to_owned(),
        );
    }
    if binary_operation(&node.operation).is_none() {
        return (
            ExecutionTarget::Cpu,
            format!("{operation} has no GPU lowering"),
        );
    }
    if !contract_supported(artifact, node, false) {
        return (
            ExecutionTarget::Cpu,
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
            } => schema_elements(artifact, artifact.slots()[slot.get() as usize].schema).is_some(),
            BindingDeclaration::Input {
                source: ArtifactSource::Constant(constant),
                ..
            } => matches!(
                artifact
                    .constants()
                    .get(*constant)
                    .map(|value| value.data()),
                Some(mech_core::ValueData::F32(_))
            ),
            BindingDeclaration::Output { target, .. } => {
                schema_elements(artifact, artifact.slots()[target.get() as usize].schema).is_some()
            }
        });
    if !schemas_supported {
        return (
            ExecutionTarget::Cpu,
            "value schema or constant representation is not admitted".to_owned(),
        );
    }
    (
        ExecutionTarget::Gpu,
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
                    OutputConstruction::FullWrite { .. } | OutputConstruction::Replace { .. }
                )
            } else {
                matches!(output.construction, OutputConstruction::FullWrite { .. })
            }
    })
}

fn schema_elements(artifact: &ProgramArtifact, schema: mech_core::SchemaId) -> Option<u64> {
    match artifact.schemas().get(schema)?.body() {
        SchemaBody::FloatingPoint(FloatWidth::W32) => Some(1),
        SchemaBody::Matrix {
            element,
            dimensions,
        } if matches!(element.as_ref(), SchemaBody::FloatingPoint(FloatWidth::W32)) => dimensions
            .iter()
            .try_fold(1_u64, |elements, dimension| match dimension {
                DimensionExpr::Constant(extent) => elements.checked_mul(*extent),
                _ => None,
            }),
        _ => None,
    }
}

fn producer_target(
    artifact: &ProgramArtifact,
    placements: &[NodePlacement],
    slot: CellSlotId,
) -> ExecutionTarget {
    match artifact.slots()[slot.get() as usize].producer {
        ProducerReference::Input(_) => ExecutionTarget::Cpu,
        ProducerReference::NodeOutput { node, .. } => placements[node.get() as usize].target,
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
    let ProducerReference::NodeOutput { node, .. } = artifact.slots()[slot.get() as usize].producer
    else {
        return vec![slot];
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
