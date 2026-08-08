use mech_core::InstanceEpoch;

use super::{Candidate, NODES_PER_EKF, ResidentExecutionError, efficacy::ekf};

#[inline]
fn validate_candidate(
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

#[cfg(test)]
pub(crate) fn node_executed(candidate: &Candidate<'_>, node: usize) -> bool {
    candidate.instance.workspace.node_execution_marks[node]
        == InstanceEpoch(candidate.working_epoch.0)
}
