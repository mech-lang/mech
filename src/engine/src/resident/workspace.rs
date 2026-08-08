use mech_core::{InstanceEpoch, SlotIndex};

use super::{ActivatedPlan, NodeIndex, slot};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct EkfScratch {
    pub(crate) trig: [f64; 2],
    pub(crate) motion_jacobian: [f64; 9],
    pub(crate) control_jacobian: [f64; 6],
    pub(crate) predicted_state: [f64; 3],
    pub(crate) predicted_covariance: [f64; 9],
    pub(crate) delta_range: [f64; 3],
    pub(crate) predicted_measurement: [f64; 2],
    pub(crate) measurement_jacobian: [f64; 6],
    pub(crate) innovation_covariance: [f64; 4],
    pub(crate) inverse_innovation: [f64; 4],
    pub(crate) gain: [f64; 6],
    pub(crate) innovation: [f64; 2],
    pub(crate) corrected_state: [f64; 3],
    pub(crate) corrected_covariance: [f64; 9],
}

#[derive(Debug)]
pub(crate) struct TurnWorkspace {
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

impl TurnWorkspace {
    pub(crate) fn activate(plan: &ActivatedPlan) -> Self {
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
