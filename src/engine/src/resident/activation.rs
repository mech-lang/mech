use mech_core::{CellSlotId, SlotIndex};

use super::artifact::{
    EkfConstants, EkfOp, LOGICAL_SLOTS_PER_EKF, NODES_PER_EKF, ProgramArtifact, SlotKind, SlotRole,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub(crate) struct NodeIndex(pub(crate) u32);

pub(crate) type ActivatedKernel = EkfOp;

#[derive(Clone, Copy, Debug)]
pub(crate) struct ResolvedSlot {
    pub(crate) id: CellSlotId,
    pub(crate) index: SlotIndex,
    pub(crate) kind: SlotKind,
    pub(crate) role: SlotRole,
    pub(crate) instance: u32,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ActivatedNode {
    pub(crate) kernel: ActivatedKernel,
    pub(crate) instance: u32,
    pub(crate) downstream_start: u32,
    pub(crate) downstream_len: u16,
}

#[derive(Clone, Debug)]
pub(crate) struct DependencyTopology {
    pub(crate) linear_node_order: Box<[NodeIndex]>,
    pub(crate) consumer_offsets: Box<[u32]>,
    pub(crate) consumer_nodes: Box<[NodeIndex]>,
    pub(crate) downstream_offsets: Box<[u32]>,
    pub(crate) downstream_nodes: Box<[NodeIndex]>,
}

impl DependencyTopology {
    pub(crate) fn consumers(&self, slot: CellSlotId) -> &[NodeIndex] {
        let index = slot.0 as usize;
        let start = self.consumer_offsets[index] as usize;
        let end = self.consumer_offsets[index + 1] as usize;
        &self.consumer_nodes[start..end]
    }

    pub(crate) fn downstream(&self, node: NodeIndex) -> &[NodeIndex] {
        let index = node.0 as usize;
        let start = self.downstream_offsets[index] as usize;
        let end = self.downstream_offsets[index + 1] as usize;
        &self.downstream_nodes[start..end]
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ActivatedPlan {
    pub(crate) instances: usize,
    pub(crate) slots: Box<[ResolvedSlot]>,
    pub(crate) nodes: Box<[ActivatedNode]>,
    pub(crate) topology: DependencyTopology,
    pub(crate) constants: EkfConstants,
}

fn flatten<T: Copy>(lists: &[Vec<T>]) -> (Box<[u32]>, Box<[T]>) {
    let mut offsets = Vec::with_capacity(lists.len() + 1);
    let total = lists.iter().map(Vec::len).sum();
    let mut values = Vec::with_capacity(total);
    offsets.push(0);
    for list in lists {
        values.extend_from_slice(list);
        offsets.push(u32::try_from(values.len()).expect("resident topology fits u32"));
    }
    (offsets.into_boxed_slice(), values.into_boxed_slice())
}

impl ActivatedPlan {
    pub(crate) fn activate(artifact: ProgramArtifact) -> Self {
        let logical_slot_count = artifact.slots.len();
        let slots: Box<[_]> = artifact
            .slots
            .iter()
            .enumerate()
            .map(|(index, slot)| {
                assert_eq!(slot.id.0 as usize, index, "logical slot IDs are dense");
                ResolvedSlot {
                    id: slot.id,
                    index: SlotIndex(index.try_into().expect("resident slot count fits u32")),
                    kind: slot.kind,
                    role: slot.role,
                    instance: slot.instance,
                }
            })
            .collect();

        let mut consumers = vec![Vec::<NodeIndex>::new(); logical_slot_count];
        for (node, declaration) in artifact.nodes.iter().enumerate() {
            let node = NodeIndex(node.try_into().expect("resident node count fits u32"));
            for slot in &declaration.reads {
                consumers[slot.0 as usize].push(node);
            }
        }
        let (consumer_offsets, consumer_nodes) = flatten(&consumers);

        let mut downstream = vec![Vec::<NodeIndex>::new(); artifact.nodes.len()];
        for (node, declaration) in artifact.nodes.iter().enumerate() {
            for output in &declaration.writes {
                for consumer in &consumers[output.0 as usize] {
                    if !downstream[node].contains(consumer) {
                        downstream[node].push(*consumer);
                    }
                }
            }
            downstream[node].sort_by_key(|index| index.0);
        }
        let (downstream_offsets, downstream_nodes) = flatten(&downstream);
        let nodes: Box<[_]> = artifact
            .nodes
            .iter()
            .enumerate()
            .map(|(index, declaration)| {
                let start = downstream_offsets[index];
                let end = downstream_offsets[index + 1];
                ActivatedNode {
                    kernel: declaration.op,
                    instance: declaration.instance,
                    downstream_start: start,
                    downstream_len: u16::try_from(end - start)
                        .expect("resident node fanout fits u16"),
                }
            })
            .collect();
        let topology = DependencyTopology {
            linear_node_order: (0..nodes.len())
                .map(|index| NodeIndex(index as u32))
                .collect(),
            consumer_offsets,
            consumer_nodes,
            downstream_offsets,
            downstream_nodes,
        };
        debug_assert_eq!(artifact.instances * NODES_PER_EKF, nodes.len());
        debug_assert_eq!(
            artifact.instances * LOGICAL_SLOTS_PER_EKF as usize,
            slots.len()
        );
        Self {
            instances: artifact.instances,
            slots,
            nodes,
            topology,
            constants: artifact.constants,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resident::slot;

    #[test]
    fn scaled_activation_is_deterministic_and_topologically_complete() {
        for instances in [1, 8, 64] {
            let artifact = ProgramArtifact::frozen_ekf_batch(instances);
            let declarations = artifact.nodes.clone();
            let left = ActivatedPlan::activate(artifact);
            let right = ActivatedPlan::activate(ProgramArtifact::frozen_ekf_batch(instances));
            assert_eq!(left.nodes.len(), NODES_PER_EKF * instances);
            assert_eq!(left.slots.len(), LOGICAL_SLOTS_PER_EKF as usize * instances);
            assert_eq!(left.topology.linear_node_order.len(), left.nodes.len());
            assert_eq!(
                left.topology.consumer_offsets,
                right.topology.consumer_offsets
            );
            assert_eq!(left.topology.consumer_nodes, right.topology.consumer_nodes);
            assert_eq!(
                left.topology.downstream_offsets,
                right.topology.downstream_offsets
            );
            assert_eq!(
                left.topology.downstream_nodes,
                right.topology.downstream_nodes
            );
            for (expected, node) in left.nodes.iter().enumerate() {
                assert_eq!(node.instance as usize, expected / NODES_PER_EKF);
                let downstream = left.topology.downstream(NodeIndex(expected as u32));
                assert_eq!(
                    node.downstream_start,
                    left.topology.downstream_offsets[expected]
                );
                assert_eq!(node.downstream_len as usize, downstream.len());
            }
            for instance in 0..instances as u32 {
                assert_eq!(
                    left.topology
                        .consumers(slot::id(instance, slot::INPUT))
                        .len(),
                    3
                );
                assert_eq!(
                    left.topology
                        .consumers(slot::id(instance, slot::STATE))
                        .len(),
                    3
                );
                assert_eq!(
                    left.topology
                        .consumers(slot::id(instance, slot::CORRECTED_STATE))
                        .len(),
                    1
                );
            }
            for (node, declaration) in declarations.iter().enumerate() {
                let node = NodeIndex(node as u32);
                for input in &declaration.reads {
                    assert!(left.topology.consumers(*input).contains(&node));
                }
                let downstream = left.topology.downstream(node);
                for output in &declaration.writes {
                    for consumer in left.topology.consumers(*output) {
                        assert!(downstream.contains(consumer));
                    }
                }
            }
        }
    }
}
