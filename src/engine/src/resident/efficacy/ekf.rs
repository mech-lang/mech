use crate::efficacy::ekf::math::{self, EkfMathError};
use crate::efficacy::ekf::operation::{EkfConstants, EkfKernel, EkfScratch};

use super::super::ResidentExecutionError;

#[inline(always)]
pub(crate) fn execute(
    kernel: EkfKernel,
    input: &[f64; 4],
    state: &[f64; 3],
    covariance: &[f64; 9],
    candidate_state: &mut [f64; 3],
    candidate_covariance: &mut [f64; 9],
    scratch: &mut EkfScratch,
    constants: &EkfConstants,
) -> Result<(), ResidentExecutionError> {
    use EkfKernel::*;
    match kernel {
        TrigonometricState => scratch.trig = math::trigonometric_state(state),
        MotionJacobian => {
            scratch.motion_jacobian = math::motion_jacobian(input, &scratch.trig, constants.dt);
        }
        ControlJacobian => {
            scratch.control_jacobian = math::control_jacobian(&scratch.trig, constants.dt);
        }
        PredictedState => {
            scratch.predicted_state =
                math::predicted_state(state, input, &scratch.trig, constants.dt);
        }
        PredictedCovariance => {
            scratch.predicted_covariance = math::predicted_covariance(
                covariance,
                &scratch.motion_jacobian,
                &scratch.control_jacobian,
                &constants.process_covariance,
            );
        }
        LandmarkDeltaAndRange => {
            scratch.delta_range =
                math::landmark_delta_and_range(&scratch.predicted_state, &constants.landmark)
                    .map_err(map_math_error)?;
        }
        PredictedMeasurement => {
            scratch.predicted_measurement =
                math::predicted_measurement(&scratch.predicted_state, &scratch.delta_range);
        }
        MeasurementJacobian => {
            scratch.measurement_jacobian = math::measurement_jacobian(&scratch.delta_range);
        }
        InnovationCovariance => {
            scratch.innovation_covariance = math::innovation_covariance(
                &scratch.predicted_covariance,
                &scratch.measurement_jacobian,
                &constants.measurement_covariance,
            );
        }
        Solve2x2 => {
            scratch.inverse_innovation =
                math::solve_2x2(&scratch.innovation_covariance).map_err(map_math_error)?;
        }
        KalmanGain => {
            scratch.gain = math::kalman_gain(
                &scratch.predicted_covariance,
                &scratch.measurement_jacobian,
                &scratch.inverse_innovation,
            );
        }
        Innovation => {
            scratch.innovation = math::innovation(input, &scratch.predicted_measurement);
        }
        CorrectedState => {
            scratch.corrected_state =
                math::corrected_state(&scratch.predicted_state, &scratch.gain, &scratch.innovation);
        }
        JosephCovarianceUpdate => {
            scratch.corrected_covariance = math::joseph_covariance_update(
                &scratch.predicted_covariance,
                &scratch.measurement_jacobian,
                &scratch.gain,
                &constants.measurement_covariance,
            );
        }
        CovarianceSymmetrization => {
            scratch.symmetrized_covariance =
                math::covariance_symmetrization(&scratch.corrected_covariance);
            *candidate_covariance = scratch.symmetrized_covariance;
            *candidate_state = scratch.corrected_state;
        }
    }
    Ok(())
}

#[inline(always)]
fn map_math_error(error: EkfMathError) -> ResidentExecutionError {
    match error {
        EkfMathError::LandmarkDistance => ResidentExecutionError::LandmarkDistance,
        EkfMathError::InnovationDeterminant => ResidentExecutionError::InnovationDeterminant,
    }
}
