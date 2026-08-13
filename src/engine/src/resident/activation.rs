use mech_core::{CellSlotId, SlotIndex};

use super::artifact::{
    GateBControlFixture, LOGICAL_SLOTS_PER_EKF, NODES_PER_EKF, SlotKind, SlotRole,
};
use crate::efficacy::ekf::operation::{EkfConstants, EkfKernel};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub(crate) struct NodeIndex(pub(crate) u32);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum EdgeTiming {
    SameTurn,
    NextTurn,
}

pub(crate) type ActivatedKernel = EkfKernel;

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
    pub(crate) same_turn_downstream_start: u32,
    pub(crate) same_turn_downstream_len: u16,
}

#[derive(Clone, Debug)]
pub(crate) struct DependencyTopology {
    pub(crate) linear_node_order: Box<[NodeIndex]>,
    pub(crate) consumer_offsets: Box<[u32]>,
    pub(crate) consumer_nodes: Box<[NodeIndex]>,
    pub(crate) same_turn_downstream_offsets: Box<[u32]>,
    pub(crate) same_turn_downstream_nodes: Box<[NodeIndex]>,
    pub(crate) next_turn_consumer_offsets: Box<[u32]>,
    pub(crate) next_turn_consumer_nodes: Box<[NodeIndex]>,
    pub(crate) turn_root_nodes: Box<[NodeIndex]>,
}

impl DependencyTopology {
    pub(crate) fn consumers(&self, slot: CellSlotId) -> &[NodeIndex] {
        let index = slot.0 as usize;
        let start = self.consumer_offsets[index] as usize;
        let end = self.consumer_offsets[index + 1] as usize;
        &self.consumer_nodes[start..end]
    }

    pub(crate) fn same_turn_downstream(&self, node: NodeIndex) -> &[NodeIndex] {
        let index = node.0 as usize;
        let start = self.same_turn_downstream_offsets[index] as usize;
        let end = self.same_turn_downstream_offsets[index + 1] as usize;
        &self.same_turn_downstream_nodes[start..end]
    }

    pub(crate) fn next_turn_consumers(&self, slot: CellSlotId) -> &[NodeIndex] {
        let index = slot.0 as usize;
        let start = self.next_turn_consumer_offsets[index] as usize;
        let end = self.next_turn_consumer_offsets[index + 1] as usize;
        &self.next_turn_consumer_nodes[start..end]
    }
}

#[derive(Clone, Debug)]
pub(crate) struct GateBPlan {
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

impl GateBPlan {
    pub(crate) fn from_control_fixture(artifact: GateBControlFixture) -> Self {
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

        let mut same_turn_downstream = vec![Vec::<NodeIndex>::new(); artifact.nodes.len()];
        let mut next_turn_consumers = vec![Vec::<NodeIndex>::new(); logical_slot_count];
        let mut turn_roots = Vec::<NodeIndex>::new();
        for slot in &artifact.slots {
            if matches!(slot.role, SlotRole::Input | SlotRole::Stateful) {
                turn_roots.extend_from_slice(&consumers[slot.id.0 as usize]);
            }
            if slot.role == SlotRole::Stateful {
                next_turn_consumers[slot.id.0 as usize]
                    .extend_from_slice(&consumers[slot.id.0 as usize]);
            }
        }
        turn_roots.sort_by_key(|index| index.0);
        turn_roots.dedup();

        for (node, declaration) in artifact.nodes.iter().enumerate() {
            for output in &declaration.writes {
                let timing = if artifact.slots[output.0 as usize].role == SlotRole::Stateful {
                    EdgeTiming::NextTurn
                } else {
                    EdgeTiming::SameTurn
                };
                for consumer in &consumers[output.0 as usize] {
                    if timing == EdgeTiming::SameTurn
                        && !same_turn_downstream[node].contains(consumer)
                    {
                        same_turn_downstream[node].push(*consumer);
                    }
                }
            }
            same_turn_downstream[node].sort_by_key(|index| index.0);
        }
        let (same_turn_downstream_offsets, same_turn_downstream_nodes) =
            flatten(&same_turn_downstream);
        let (next_turn_consumer_offsets, next_turn_consumer_nodes) = flatten(&next_turn_consumers);
        let nodes: Box<[_]> = artifact
            .nodes
            .iter()
            .enumerate()
            .map(|(index, declaration)| {
                let start = same_turn_downstream_offsets[index];
                let end = same_turn_downstream_offsets[index + 1];
                ActivatedNode {
                    kernel: declaration.op,
                    instance: declaration.instance,
                    same_turn_downstream_start: start,
                    same_turn_downstream_len: u16::try_from(end - start)
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
            same_turn_downstream_offsets,
            same_turn_downstream_nodes,
            next_turn_consumer_offsets,
            next_turn_consumer_nodes,
            turn_root_nodes: turn_roots.into_boxed_slice(),
        };
        for node in topology.linear_node_order.iter().copied() {
            for downstream in topology.same_turn_downstream(node) {
                assert!(
                    downstream.0 > node.0,
                    "same-turn resident edges must point forward"
                );
            }
        }
        for slot in artifact
            .slots
            .iter()
            .filter(|slot| slot.role == SlotRole::Stateful)
        {
            for consumer in topology.next_turn_consumers(slot.id) {
                assert!(
                    topology.turn_root_nodes.contains(consumer),
                    "next-turn feedback must target a legal turn root"
                );
            }
        }
        for instance in 0..artifact.instances {
            let final_node = NodeIndex((instance * NODES_PER_EKF + NODES_PER_EKF - 1) as u32);
            assert!(
                topology.same_turn_downstream(final_node).is_empty(),
                "final state publication node must not feed back in the same turn"
            );
        }
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
            let artifact = GateBControlFixture::new(instances);
            let declarations = artifact.nodes.clone();
            let left = GateBPlan::from_control_fixture(artifact);
            let right = GateBPlan::from_control_fixture(GateBControlFixture::new(instances));
            assert_eq!(left.nodes.len(), NODES_PER_EKF * instances);
            assert_eq!(left.slots.len(), LOGICAL_SLOTS_PER_EKF as usize * instances);
            assert_eq!(left.topology.linear_node_order.len(), left.nodes.len());
            assert_eq!(
                left.topology.consumer_offsets,
                right.topology.consumer_offsets
            );
            assert_eq!(left.topology.consumer_nodes, right.topology.consumer_nodes);
            assert_eq!(
                left.topology.same_turn_downstream_offsets,
                right.topology.same_turn_downstream_offsets
            );
            assert_eq!(
                left.topology.same_turn_downstream_nodes,
                right.topology.same_turn_downstream_nodes
            );
            assert_eq!(
                left.topology.turn_root_nodes,
                right.topology.turn_root_nodes
            );
            for (expected, node) in left.nodes.iter().enumerate() {
                assert_eq!(node.instance as usize, expected / NODES_PER_EKF);
                let downstream = left
                    .topology
                    .same_turn_downstream(NodeIndex(expected as u32));
                assert_eq!(
                    node.same_turn_downstream_start,
                    left.topology.same_turn_downstream_offsets[expected]
                );
                assert_eq!(node.same_turn_downstream_len as usize, downstream.len());
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
                let downstream = left.topology.same_turn_downstream(node);
                for output in &declaration.writes {
                    for consumer in left.topology.consumers(*output) {
                        let role = left.slots[output.0 as usize].role;
                        if role == SlotRole::Stateful {
                            assert!(!downstream.contains(consumer));
                            assert!(
                                left.topology
                                    .next_turn_consumers(*output)
                                    .contains(consumer)
                            );
                        } else {
                            assert!(downstream.contains(consumer));
                        }
                    }
                }
            }
        }
    }
}
