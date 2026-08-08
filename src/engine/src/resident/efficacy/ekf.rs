use super::super::{ActivatedKernel, EkfConstants, EkfScratch, ResidentExecutionError};

#[inline(always)]
fn mat3_mul(a: &[f64; 9], b: &[f64; 9]) -> [f64; 9] {
    let mut result = [0.0; 9];
    for column in 0..3 {
        for row in 0..3 {
            let mut total = 0.0;
            for inner in 0..3 {
                total += a[inner * 3 + row] * b[column * 3 + inner];
            }
            result[column * 3 + row] = total;
        }
    }
    result
}

#[inline(always)]
fn mat3_mul_mat3x2(a: &[f64; 9], b: &[f64; 6]) -> [f64; 6] {
    let mut result = [0.0; 6];
    for column in 0..2 {
        for row in 0..3 {
            let mut total = 0.0;
            for inner in 0..3 {
                total += a[inner * 3 + row] * b[column * 3 + inner];
            }
            result[column * 3 + row] = total;
        }
    }
    result
}

#[inline(always)]
fn mat3x2_mul_mat2(a: &[f64; 6], b: &[f64; 4]) -> [f64; 6] {
    let mut result = [0.0; 6];
    for column in 0..2 {
        for row in 0..3 {
            let mut total = 0.0;
            for inner in 0..2 {
                total += a[inner * 3 + row] * b[column * 2 + inner];
            }
            result[column * 3 + row] = total;
        }
    }
    result
}

#[inline(always)]
fn mat3x2_mul_vec2(a: &[f64; 6], b: &[f64; 2]) -> [f64; 3] {
    let mut result = [0.0; 3];
    for row in 0..3 {
        let mut total = 0.0;
        for inner in 0..2 {
            total += a[inner * 3 + row] * b[inner];
        }
        result[row] = total;
    }
    result
}

#[inline(always)]
fn mat3x2_mul_mat2x3(a: &[f64; 6], b: &[f64; 6]) -> [f64; 9] {
    let mut result = [0.0; 9];
    for column in 0..3 {
        for row in 0..3 {
            let mut total = 0.0;
            for inner in 0..2 {
                total += a[inner * 3 + row] * b[column * 2 + inner];
            }
            result[column * 3 + row] = total;
        }
    }
    result
}

#[inline(always)]
fn mat2x3_mul_mat3(a: &[f64; 6], b: &[f64; 9]) -> [f64; 6] {
    let mut result = [0.0; 6];
    for column in 0..3 {
        for row in 0..2 {
            let mut total = 0.0;
            for inner in 0..3 {
                total += a[inner * 2 + row] * b[column * 3 + inner];
            }
            result[column * 2 + row] = total;
        }
    }
    result
}

#[inline(always)]
fn mat2x3_mul_mat3x2(a: &[f64; 6], b: &[f64; 6]) -> [f64; 4] {
    let mut result = [0.0; 4];
    for column in 0..2 {
        for row in 0..2 {
            let mut total = 0.0;
            for inner in 0..3 {
                total += a[inner * 2 + row] * b[column * 3 + inner];
            }
            result[column * 2 + row] = total;
        }
    }
    result
}

#[inline(always)]
fn transpose3(a: &[f64; 9]) -> [f64; 9] {
    [a[0], a[3], a[6], a[1], a[4], a[7], a[2], a[5], a[8]]
}

#[inline(always)]
fn transpose3x2(a: &[f64; 6]) -> [f64; 6] {
    [a[0], a[3], a[1], a[4], a[2], a[5]]
}

#[inline(always)]
fn transpose2x3(a: &[f64; 6]) -> [f64; 6] {
    [a[0], a[2], a[4], a[1], a[3], a[5]]
}

#[inline(always)]
fn add<const N: usize>(left: [f64; N], right: [f64; N]) -> [f64; N] {
    let mut result = [0.0; N];
    for index in 0..N {
        result[index] = left[index] + right[index];
    }
    result
}

#[inline(always)]
pub(crate) fn execute(
    kernel: ActivatedKernel,
    input: &[f64; 4],
    state: &[f64; 3],
    covariance: &[f64; 9],
    candidate_state: &mut [f64; 3],
    candidate_covariance: &mut [f64; 9],
    scratch: &mut EkfScratch,
    constants: &EkfConstants,
) -> Result<(), ResidentExecutionError> {
    use super::super::EkfOp::*;
    match kernel {
        TrigonometricState => {
            scratch.trig = [state[2].cos(), state[2].sin()];
        }
        MotionJacobian => {
            scratch.motion_jacobian = [
                1.0,
                0.0,
                0.0,
                0.0,
                1.0,
                0.0,
                -input[0] * scratch.trig[1] * constants.dt,
                input[0] * scratch.trig[0] * constants.dt,
                1.0,
            ];
        }
        ControlJacobian => {
            scratch.control_jacobian = [
                scratch.trig[0] * constants.dt,
                scratch.trig[1] * constants.dt,
                0.0,
                0.0,
                0.0,
                constants.dt,
            ];
        }
        PredictedState => {
            scratch.predicted_state = [
                state[0] + input[0] * scratch.trig[0] * constants.dt,
                state[1] + input[0] * scratch.trig[1] * constants.dt,
                state[2] + input[1] * constants.dt,
            ];
        }
        PredictedCovariance => {
            let gp = mat3_mul(&scratch.motion_jacobian, covariance);
            let predicted = mat3_mul(&gp, &transpose3(&scratch.motion_jacobian));
            let vq = mat3x2_mul_mat2(&scratch.control_jacobian, &constants.process_covariance);
            let process = mat3x2_mul_mat2x3(&vq, &transpose3x2(&scratch.control_jacobian));
            scratch.predicted_covariance = add(predicted, process);
        }
        LandmarkDeltaAndRange => {
            let dx = constants.landmark[0] - scratch.predicted_state[0];
            let dy = constants.landmark[1] - scratch.predicted_state[1];
            let q = dx * dx + dy * dy;
            if q <= 1.0e-12 {
                return Err(ResidentExecutionError::LandmarkDistance);
            }
            scratch.delta_range = [dx, dy, q.sqrt()];
        }
        PredictedMeasurement => {
            scratch.predicted_measurement = [
                scratch.delta_range[2],
                scratch.delta_range[1].atan2(scratch.delta_range[0]) - scratch.predicted_state[2],
            ];
        }
        MeasurementJacobian => {
            let q = scratch.delta_range[0] * scratch.delta_range[0]
                + scratch.delta_range[1] * scratch.delta_range[1];
            scratch.measurement_jacobian = [
                -scratch.delta_range[0] / scratch.delta_range[2],
                scratch.delta_range[1] / q,
                -scratch.delta_range[1] / scratch.delta_range[2],
                -scratch.delta_range[0] / q,
                0.0,
                -1.0,
            ];
        }
        InnovationCovariance => {
            let hp = mat2x3_mul_mat3(&scratch.measurement_jacobian, &scratch.predicted_covariance);
            let measurement_transpose = transpose2x3(&scratch.measurement_jacobian);
            scratch.innovation_covariance = add(
                mat2x3_mul_mat3x2(&hp, &measurement_transpose),
                constants.measurement_covariance,
            );
        }
        Solve2x2 => {
            let matrix = scratch.innovation_covariance;
            let determinant = matrix[0] * matrix[3] - matrix[2] * matrix[1];
            if determinant.abs() <= 1.0e-12 {
                return Err(ResidentExecutionError::InnovationDeterminant);
            }
            scratch.inverse_innovation = [
                matrix[3] / determinant,
                -matrix[1] / determinant,
                -matrix[2] / determinant,
                matrix[0] / determinant,
            ];
        }
        KalmanGain => {
            let measurement_transpose = transpose2x3(&scratch.measurement_jacobian);
            let pht = mat3_mul_mat3x2(&scratch.predicted_covariance, &measurement_transpose);
            scratch.gain = mat3x2_mul_mat2(&pht, &scratch.inverse_innovation);
        }
        Innovation => {
            scratch.innovation = [
                input[2] - scratch.predicted_measurement[0],
                input[3] - scratch.predicted_measurement[1],
            ];
        }
        CorrectedState => {
            let correction = mat3x2_mul_vec2(&scratch.gain, &scratch.innovation);
            scratch.corrected_state = [
                scratch.predicted_state[0] + correction[0],
                scratch.predicted_state[1] + correction[1],
                scratch.predicted_state[2] + correction[2],
            ];
        }
        JosephCovarianceUpdate => {
            let kh = mat3x2_mul_mat2x3(&scratch.gain, &scratch.measurement_jacobian);
            let identity = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
            let mut a = [0.0; 9];
            for index in 0..9 {
                a[index] = identity[index] - kh[index];
            }
            let ap = mat3_mul(&a, &scratch.predicted_covariance);
            let joseph = mat3_mul(&ap, &transpose3(&a));
            let kr = mat3x2_mul_mat2(&scratch.gain, &constants.measurement_covariance);
            let measurement = mat3x2_mul_mat2x3(&kr, &transpose3x2(&scratch.gain));
            scratch.corrected_covariance = add(joseph, measurement);
        }
        CovarianceSymmetrization => {
            let transposed = transpose3(&scratch.corrected_covariance);
            for index in 0..9 {
                candidate_covariance[index] =
                    0.5 * (scratch.corrected_covariance[index] + transposed[index]);
            }
            *candidate_state = scratch.corrected_state;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_shape_transposes_preserve_column_major_layout() {
        assert_eq!(
            transpose3(&[0., 1., 2., 3., 4., 5., 6., 7., 8.]),
            [0., 3., 6., 1., 4., 7., 2., 5., 8.]
        );
        assert_eq!(
            transpose3x2(&[0., 1., 2., 3., 4., 5.]),
            [0., 3., 1., 4., 2., 5.]
        );
        assert_eq!(
            transpose2x3(&[0., 1., 2., 3., 4., 5.]),
            [0., 2., 4., 1., 3., 5.]
        );
    }
}
