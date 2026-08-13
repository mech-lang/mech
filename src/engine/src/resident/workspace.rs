use mech_core::{InstanceEpoch, SlotIndex};

use super::{GateBPlan, NodeIndex, slot};
use crate::efficacy::ekf::operation::EkfScratch;

#[derive(Debug)]
pub(crate) struct GateBWorkspace {
    pub(crate) input: [f64; 4],
    pub(crate) scratch: Box<[EkfScratch]>,
    pub(crate) slot_epoch_marks: Box<[InstanceEpoch]>,
    pub(crate) touched_slots: Vec<SlotIndex>,
    pub(crate) invalidated_slots: Vec<SlotIndex>,
    pub(crate) changed_slots: Vec<SlotIndex>,
    pub(crate) dirty_node_marks: Box<[InstanceEpoch]>,
    pub(crate) node_execution_marks: Box<[InstanceEpoch]>,
    pub(crate) dirty_nodes: Vec<NodeIndex>,
    pub(crate) executed_nodes: Vec<NodeIndex>,
    pub(crate) linear_node_order: Box<[NodeIndex]>,
}

impl GateBWorkspace {
    pub(crate) fn activate(plan: &GateBPlan) -> Self {
        let persistent_capacity = plan.instances * 2;
        Self {
            input: [0.0; 4],
            scratch: vec![EkfScratch::default(); plan.instances].into_boxed_slice(),
            slot_epoch_marks: vec![InstanceEpoch(0); plan.slots.len()].into_boxed_slice(),
            touched_slots: Vec::with_capacity(persistent_capacity),
            invalidated_slots: Vec::with_capacity(persistent_capacity),
            changed_slots: Vec::with_capacity(persistent_capacity),
            dirty_node_marks: vec![InstanceEpoch(0); plan.nodes.len()].into_boxed_slice(),
            node_execution_marks: vec![InstanceEpoch(0); plan.nodes.len()].into_boxed_slice(),
            dirty_nodes: Vec::with_capacity(plan.nodes.len()),
            executed_nodes: Vec::with_capacity(plan.nodes.len()),
            linear_node_order: plan.topology.linear_node_order.clone(),
        }
    }

    #[inline]
    pub(crate) fn begin(&mut self, input: [f64; 4]) {
        self.input = input;
        self.touched_slots.clear();
        self.invalidated_slots.clear();
        self.changed_slots.clear();
        self.dirty_nodes.clear();
        self.executed_nodes.clear();
    }

    #[inline]
    pub(crate) fn mark_dirty(&mut self, node: NodeIndex, epoch: InstanceEpoch) {
        let mark = &mut self.dirty_node_marks[node.0 as usize];
        if *mark != epoch {
            *mark = epoch;
            self.dirty_nodes.push(node);
        }
    }

    #[inline]
    pub(crate) fn seed_turn_roots(&mut self, roots: &[NodeIndex], epoch: InstanceEpoch) {
        for node in roots.iter().copied() {
            self.mark_dirty(node, epoch);
        }
    }

    #[inline]
    pub(crate) fn is_dirty(&self, node: NodeIndex, epoch: InstanceEpoch) -> bool {
        self.dirty_node_marks[node.0 as usize] == epoch
    }

    #[inline]
    pub(crate) fn record_node_execution(&mut self, node: NodeIndex, epoch: InstanceEpoch) {
        debug_assert_ne!(self.node_execution_marks[node.0 as usize], epoch);
        self.node_execution_marks[node.0 as usize] = epoch;
        self.executed_nodes.push(node);
    }

    #[inline]
    pub(crate) fn record_candidate_outputs(&mut self, instance: u32, epoch: InstanceEpoch) {
        for local in [slot::STATE, slot::COVARIANCE] {
            let index = slot::index(instance, local);
            let mark = &mut self.slot_epoch_marks[index.0 as usize];
            if *mark != epoch {
                *mark = epoch;
                self.touched_slots.push(index);
                self.invalidated_slots.push(index);
            }
        }
    }

    #[inline]
    pub(crate) fn record_changed_outputs(&mut self, instance: u32) {
        self.changed_slots.push(slot::index(instance, slot::STATE));
        self.changed_slots
            .push(slot::index(instance, slot::COVARIANCE));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dense_marks_skip_a_clean_branch_and_deduplicate_invalidations() {
        let plan = GateBPlan::from_control_fixture(super::super::GateBControlFixture::new(1));
        let mut workspace = GateBWorkspace::activate(&plan);
        let epoch = InstanceEpoch(7);
        let order = [
            NodeIndex(0),
            NodeIndex(1),
            NodeIndex(2),
            NodeIndex(3),
            NodeIndex(4),
        ];
        let downstream: [&[NodeIndex]; 5] =
            [&[NodeIndex(1)], &[NodeIndex(3)], &[NodeIndex(4)], &[], &[]];

        workspace.begin([0.0; 4]);
        workspace.mark_dirty(NodeIndex(0), epoch);
        workspace.mark_dirty(NodeIndex(0), epoch);
        assert_eq!(workspace.dirty_nodes, [NodeIndex(0)]);
        for node in order {
            if !workspace.is_dirty(node, epoch) {
                continue;
            }
            workspace.record_node_execution(node, epoch);
            for child in downstream[node.0 as usize] {
                workspace.mark_dirty(*child, epoch);
            }
        }

        assert_eq!(
            workspace.executed_nodes,
            [NodeIndex(0), NodeIndex(1), NodeIndex(3)]
        );
        assert!(!workspace.is_dirty(NodeIndex(2), epoch));
        assert!(!workspace.is_dirty(NodeIndex(4), epoch));
    }
}
