use mech_core::InstanceEpoch;

use super::{Candidate, NODES_PER_EKF, ResidentExecutionError, efficacy::ekf};

#[inline]
pub(crate) fn validate_candidate(
    state: &[f64; 3],
    covariance: &[f64; 9],
) -> Result<(), ResidentExecutionError> {
    if !state
        .iter()
        .chain(covariance)
        .all(|value| value.is_finite())
    {
        return Err(ResidentExecutionError::NonFiniteState);
    }
    if ![0, 4, 8].into_iter().all(|index| covariance[index] > 0.0) {
        return Err(ResidentExecutionError::CovarianceDiagonal);
    }
    let mut symmetry_error = 0.0_f64;
    for column in 0..3 {
        for row in 0..3 {
            symmetry_error = symmetry_error
                .max((covariance[column * 3 + row] - covariance[row * 3 + column]).abs());
        }
    }
    if symmetry_error > 1.0e-10 {
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

fn execute_fused_ekf_candidate_with_tracking<
    const MARK_NODES: bool,
    const TRACK_OUTPUTS: bool,
    const VALIDATE: bool,
>(
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
        for instance in 0..plan.instances {
            ekf::execute_fused(
                &input,
                &states[instance],
                &covariances[instance],
                &mut candidate_states[instance],
                &mut candidate_covariances[instance],
                &mut resident.workspace.scratch[instance],
                &plan.constants,
            )?;
            if MARK_NODES {
                let node_start = instance * NODES_PER_EKF;
                resident.workspace.node_execution_marks[node_start..node_start + NODES_PER_EKF]
                    .fill(working_epoch);
            }
            if VALIDATE {
                validate_candidate(
                    &candidate_states[instance],
                    &candidate_covariances[instance],
                )?;
            }
        }
    }
    if TRACK_OUTPUTS {
        for instance in 0..plan.instances as u32 {
            resident
                .workspace
                .record_candidate_outputs(instance, working_epoch);
            resident.workspace.record_changed_outputs(instance);
        }
    }
    Ok(())
}

pub(crate) fn execute_fused_ekf_candidate(
    candidate: &mut Candidate<'_>,
) -> Result<(), ResidentExecutionError> {
    execute_fused_ekf_candidate_with_tracking::<true, true, true>(candidate)
}

pub(crate) fn execute_fused_boundary_ekf_candidate(
    candidate: &mut Candidate<'_>,
) -> Result<(), ResidentExecutionError> {
    execute_fused_ekf_candidate_with_tracking::<false, true, true>(candidate)
}

pub(crate) fn execute_fused_untracked_ekf_candidate(
    candidate: &mut Candidate<'_>,
) -> Result<(), ResidentExecutionError> {
    execute_fused_ekf_candidate_with_tracking::<false, false, true>(candidate)
}

pub(crate) fn execute_fused_failstop_ekf_candidate(
    candidate: &mut Candidate<'_>,
) -> Result<(), ResidentExecutionError> {
    execute_fused_ekf_candidate_with_tracking::<false, false, false>(candidate)
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

pub(crate) fn execute_scheduled_count_only_ekf_candidate(
    candidate: &mut Candidate<'_>,
) -> Result<(), ResidentExecutionError> {
    let working_epoch = candidate.working_epoch;
    let published_buffer = candidate.published_buffer;
    let candidate_buffer = candidate.candidate_buffer;
    let resident = &mut *candidate.instance;
    let plan = &resident.plan;
    let input = resident.workspace.input;
    let mut dirty_nodes = 0;
    for node in plan.topology.turn_root_nodes.iter().copied() {
        dirty_nodes += usize::from(
            resident
                .workspace
                .mark_dirty_count_only(node, working_epoch),
        );
    }
    let mut executed_nodes = 0;
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
                .record_node_execution_count_only(node_index, working_epoch);
            executed_nodes += 1;
            for downstream in plan.topology.same_turn_downstream(node_index) {
                dirty_nodes += usize::from(
                    resident
                        .workspace
                        .mark_dirty_count_only(*downstream, working_epoch),
                );
            }
        }
        for instance in 0..plan.instances {
            validate_candidate(
                &candidate_states[instance],
                &candidate_covariances[instance],
            )?;
        }
    }
    let mut touched_slots = 0;
    for instance in 0..plan.instances as u32 {
        touched_slots += resident
            .workspace
            .record_candidate_outputs_count_only(instance, working_epoch);
    }
    let changed_slots = plan.instances * 2;
    resident.workspace.count_only_totals =
        [touched_slots, changed_slots, dirty_nodes, executed_nodes];
    debug_assert_eq!(executed_nodes, plan.instances * NODES_PER_EKF);
    Ok(())
}

#[cfg(test)]
pub(crate) fn node_executed(candidate: &Candidate<'_>, node: usize) -> bool {
    candidate.instance.workspace.node_execution_marks[node]
        == InstanceEpoch(candidate.working_epoch.0)
}
