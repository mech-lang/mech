use mech_core::InstanceEpoch;

use super::{Candidate, NODES_PER_EKF, ResidentExecutionError, efficacy::ekf};
use crate::efficacy::ekf::math;

#[inline]
fn validate_candidate(
    state: &[f64; 3],
    covariance: &[f64; 9],
) -> Result<(), ResidentExecutionError> {
    if !math::candidate_finite(state, covariance) {
        return Err(ResidentExecutionError::NonFiniteState);
    }
    if !math::covariance_positive_diagonal(covariance) {
        return Err(ResidentExecutionError::CovarianceDiagonal);
    }
    if !math::covariance_symmetric(covariance) {
        return Err(ResidentExecutionError::CovarianceSymmetry);
    }
    Ok(())
}

pub(crate) fn execute_ekf_candidate(
    candidate: &mut Candidate<'_>,
) -> Result<(), ResidentExecutionError> {
    let working_epoch = candidate.working_epoch;
    let published_buffer = candidate.published_buffer;
    let candidate_buffer = candidate.candidate_buffer;
    let resident = &mut *candidate.instance;
    let plan = &resident.plan;
    let input = resident.workspace.input;
    {
        let (states, candidate_states, covariances, candidate_covariances) = resident
            .state
            .split_buffers(published_buffer, candidate_buffer);
        let scratch = &mut resident.workspace.scratch;
        let marks = &mut resident.workspace.node_execution_marks;
        let order = &resident.workspace.linear_node_order;
        for instance in 0..plan.instances {
            let node_start = instance * NODES_PER_EKF;
            for offset in 0..NODES_PER_EKF {
                let node_index = order[node_start + offset];
                let node = plan.nodes[node_index.0 as usize];
                debug_assert_eq!(node.instance as usize, instance);
                ekf::execute(
                    node.kernel,
                    &input,
                    &states[instance],
                    &covariances[instance],
                    &mut candidate_states[instance],
                    &mut candidate_covariances[instance],
                    &mut scratch[instance],
                    &plan.constants,
                )?;
                marks[node_index.0 as usize] = working_epoch;
            }
            validate_candidate(
                &candidate_states[instance],
                &candidate_covariances[instance],
            )?;
        }
    }
    for instance in 0..plan.instances as u32 {
        resident
            .workspace
            .record_candidate_outputs(instance, working_epoch);
        resident.workspace.record_changed_outputs(instance);
    }
    Ok(())
}

pub(crate) fn execute_scheduled_ekf_candidate(
    candidate: &mut Candidate<'_>,
) -> Result<(), ResidentExecutionError> {
    let working_epoch = candidate.working_epoch;
    let published_buffer = candidate.published_buffer;
    let candidate_buffer = candidate.candidate_buffer;
    let resident = &mut *candidate.instance;
    let plan = &resident.plan;
    let input = resident.workspace.input;
    resident
        .workspace
        .seed_turn_roots(&plan.topology.turn_root_nodes, working_epoch);
    {
        let (states, candidate_states, covariances, candidate_covariances) = resident
            .state
            .split_buffers(published_buffer, candidate_buffer);
        for order_index in 0..resident.workspace.linear_node_order.len() {
            let node_index = resident.workspace.linear_node_order[order_index];
            if !resident.workspace.is_dirty(node_index, working_epoch) {
                continue;
            }
            let node = plan.nodes[node_index.0 as usize];
            let instance = node.instance as usize;
            ekf::execute(
                node.kernel,
                &input,
                &states[instance],
                &covariances[instance],
                &mut candidate_states[instance],
                &mut candidate_covariances[instance],
                &mut resident.workspace.scratch[instance],
                &plan.constants,
            )?;
            resident
                .workspace
                .record_node_execution(node_index, working_epoch);
            for downstream in plan.topology.same_turn_downstream(node_index) {
                resident.workspace.mark_dirty(*downstream, working_epoch);
            }
        }
        for instance in 0..plan.instances {
            validate_candidate(
                &candidate_states[instance],
                &candidate_covariances[instance],
            )?;
        }
    }
    for instance in 0..plan.instances as u32 {
        resident
            .workspace
            .record_candidate_outputs(instance, working_epoch);
        resident.workspace.record_changed_outputs(instance);
    }
    debug_assert_eq!(
        resident.workspace.executed_nodes.len(),
        plan.instances * NODES_PER_EKF
    );
    Ok(())
}

#[cfg(test)]
pub(crate) fn node_executed(candidate: &Candidate<'_>, node: usize) -> bool {
    candidate.instance.workspace.node_execution_marks[node]
        == InstanceEpoch(candidate.working_epoch.0)
}
