//! Allocation-free mathematics shared by the ordinary-source efficacy fixture
//! and the Gate B resident control.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum EkfMathError {
    LandmarkDistance,
    InnovationDeterminant,
}

#[inline(always)]
fn mat3_mul(a: &[f64; 9], b: &[f64; 9]) -> [f64; 9] {
    let mut result = [0.0; 9];
    for column in 0..3 {
        for row in 0..3 {
            for inner in 0..3 {
                result[column * 3 + row] += a[inner * 3 + row] * b[column * 3 + inner];
            }
        }
    }
    result
}

#[inline(always)]
fn mat3_mul_mat3x2(a: &[f64; 9], b: &[f64; 6]) -> [f64; 6] {
    let mut result = [0.0; 6];
    for column in 0..2 {
        for row in 0..3 {
            for inner in 0..3 {
                result[column * 3 + row] += a[inner * 3 + row] * b[column * 3 + inner];
            }
        }
    }
    result
}

#[inline(always)]
fn mat3x2_mul_mat2(a: &[f64; 6], b: &[f64; 4]) -> [f64; 6] {
    let mut result = [0.0; 6];
    for column in 0..2 {
        for row in 0..3 {
            for inner in 0..2 {
                result[column * 3 + row] += a[inner * 3 + row] * b[column * 2 + inner];
            }
        }
    }
    result
}

#[inline(always)]
fn mat3x2_mul_vec2(a: &[f64; 6], b: &[f64; 2]) -> [f64; 3] {
    let mut result = [0.0; 3];
    for row in 0..3 {
        for inner in 0..2 {
            result[row] += a[inner * 3 + row] * b[inner];
        }
    }
    result
}

#[inline(always)]
fn mat3x2_mul_mat2x3(a: &[f64; 6], b: &[f64; 6]) -> [f64; 9] {
    let mut result = [0.0; 9];
    for column in 0..3 {
        for row in 0..3 {
            for inner in 0..2 {
                result[column * 3 + row] += a[inner * 3 + row] * b[column * 2 + inner];
            }
        }
    }
    result
}

#[inline(always)]
fn mat2x3_mul_mat3(a: &[f64; 6], b: &[f64; 9]) -> [f64; 6] {
    let mut result = [0.0; 6];
    for column in 0..3 {
        for row in 0..2 {
            for inner in 0..3 {
                result[column * 2 + row] += a[inner * 2 + row] * b[column * 3 + inner];
            }
        }
    }
    result
}

#[inline(always)]
fn mat2x3_mul_mat3x2(a: &[f64; 6], b: &[f64; 6]) -> [f64; 4] {
    let mut result = [0.0; 4];
    for column in 0..2 {
        for row in 0..2 {
            for inner in 0..3 {
                result[column * 2 + row] += a[inner * 2 + row] * b[column * 3 + inner];
            }
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
pub(crate) fn trigonometric_state(state: &[f64; 3]) -> [f64; 2] {
    [state[2].cos(), state[2].sin()]
}

/// Runs one transition of the four-state square-drive controller.
///
/// `motion` is `[commanded_speed, actual_speed, dt]` and `bounds` is
/// `[low, high]`. The returned frame is
/// `[phase, commanded_speed, actual_speed, turn_rate]`.
#[inline(always)]
pub(crate) fn square_drive_state_machine(
    state: &[f64; 3],
    motion: &[f64; 3],
    bounds: &[f64; 2],
) -> [f64; 4] {
    let quarter_turn = core::f64::consts::FRAC_PI_2;
    let phase = ((state[2] / quarter_turn).round() as i64).rem_euclid(4) as usize;
    let distance = match phase {
        0 => bounds[1] - state[0],
        1 => bounds[1] - state[1],
        2 => state[0] - bounds[0],
        _ => state[1] - bounds[0],
    };
    let dt = motion[2];
    if !dt.is_finite() || dt <= 0.0 {
        return [phase as f64, 0.0, 0.0, 0.0];
    }
    if distance <= 1.0e-12 {
        return [((phase + 1) % 4) as f64, 0.0, 0.0, quarter_turn / dt];
    }
    let boundary_speed = distance / dt;
    [
        phase as f64,
        motion[0].max(0.0).min(boundary_speed),
        motion[1].max(0.0).min(boundary_speed),
        0.0,
    ]
}

#[inline(always)]
pub(crate) fn motion_jacobian(frame: &[f64; 4], trig: &[f64; 2], dt: f64) -> [f64; 9] {
    [
        1.0,
        0.0,
        0.0,
        0.0,
        1.0,
        0.0,
        -frame[0] * trig[1] * dt,
        frame[0] * trig[0] * dt,
        1.0,
    ]
}

#[inline(always)]
pub(crate) fn control_jacobian(trig: &[f64; 2], dt: f64) -> [f64; 6] {
    [trig[0] * dt, trig[1] * dt, 0.0, 0.0, 0.0, dt]
}

#[inline(always)]
pub(crate) fn predicted_state(
    state: &[f64; 3],
    frame: &[f64; 4],
    trig: &[f64; 2],
    dt: f64,
) -> [f64; 3] {
    [
        state[0] + frame[0] * trig[0] * dt,
        state[1] + frame[0] * trig[1] * dt,
        state[2] + frame[1] * dt,
    ]
}

#[inline(always)]
pub(crate) fn predicted_covariance(
    covariance: &[f64; 9],
    motion: &[f64; 9],
    control: &[f64; 6],
    process_covariance: &[f64; 4],
) -> [f64; 9] {
    let gp = mat3_mul(motion, covariance);
    let predicted = mat3_mul(&gp, &transpose3(motion));
    let vq = mat3x2_mul_mat2(control, process_covariance);
    add(predicted, mat3x2_mul_mat2x3(&vq, &transpose3x2(control)))
}

#[inline(always)]
pub(crate) fn landmark_delta_and_range(
    predicted: &[f64; 3],
    landmark: &[f64; 2],
) -> Result<[f64; 3], EkfMathError> {
    let dx = landmark[0] - predicted[0];
    let dy = landmark[1] - predicted[1];
    let q = dx * dx + dy * dy;
    if q <= 1.0e-12 {
        return Err(EkfMathError::LandmarkDistance);
    }
    Ok([dx, dy, q.sqrt()])
}

#[inline(always)]
pub(crate) fn predicted_measurement(predicted: &[f64; 3], delta: &[f64; 3]) -> [f64; 2] {
    [delta[2], delta[1].atan2(delta[0]) - predicted[2]]
}

#[inline(always)]
pub(crate) fn measurement_jacobian(delta: &[f64; 3]) -> [f64; 6] {
    let q = delta[0] * delta[0] + delta[1] * delta[1];
    [
        -delta[0] / delta[2],
        delta[1] / q,
        -delta[1] / delta[2],
        -delta[0] / q,
        0.0,
        -1.0,
    ]
}

#[inline(always)]
pub(crate) fn innovation_covariance(
    predicted: &[f64; 9],
    measurement: &[f64; 6],
    noise: &[f64; 4],
) -> [f64; 4] {
    let hp = mat2x3_mul_mat3(measurement, predicted);
    add(mat2x3_mul_mat3x2(&hp, &transpose2x3(measurement)), *noise)
}

#[inline(always)]
pub(crate) fn solve_2x2(matrix: &[f64; 4]) -> Result<[f64; 4], EkfMathError> {
    let determinant = matrix[0] * matrix[3] - matrix[2] * matrix[1];
    if determinant.abs() <= 1.0e-12 {
        return Err(EkfMathError::InnovationDeterminant);
    }
    Ok([
        matrix[3] / determinant,
        -matrix[1] / determinant,
        -matrix[2] / determinant,
        matrix[0] / determinant,
    ])
}

#[inline(always)]
pub(crate) fn kalman_gain(
    predicted: &[f64; 9],
    measurement: &[f64; 6],
    inverse: &[f64; 4],
) -> [f64; 6] {
    let pht = mat3_mul_mat3x2(predicted, &transpose2x3(measurement));
    mat3x2_mul_mat2(&pht, inverse)
}

#[inline(always)]
pub(crate) fn innovation(measurement: &[f64; 2], predicted: &[f64; 2]) -> [f64; 2] {
    [measurement[0] - predicted[0], measurement[1] - predicted[1]]
}

/// Adapts the frozen efficacy workload's historical `[v, ω, range, bearing]`
/// input frame without exposing that fixture layout through the public EKF API.
#[inline(always)]
pub(crate) fn innovation_from_frame(frame: &[f64; 4], predicted: &[f64; 2]) -> [f64; 2] {
    innovation(&[frame[2], frame[3]], predicted)
}

#[inline(always)]
pub(crate) fn corrected_state(
    predicted: &[f64; 3],
    gain: &[f64; 6],
    innovation: &[f64; 2],
) -> [f64; 3] {
    let correction = mat3x2_mul_vec2(gain, innovation);
    [
        predicted[0] + correction[0],
        predicted[1] + correction[1],
        predicted[2] + correction[2],
    ]
}

#[inline(always)]
pub(crate) fn joseph_covariance_update(
    predicted: &[f64; 9],
    measurement: &[f64; 6],
    gain: &[f64; 6],
    noise: &[f64; 4],
) -> [f64; 9] {
    let kh = mat3x2_mul_mat2x3(gain, measurement);
    let identity = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
    let mut a = [0.0; 9];
    for index in 0..9 {
        a[index] = identity[index] - kh[index];
    }
    let ap = mat3_mul(&a, predicted);
    let joseph = mat3_mul(&ap, &transpose3(&a));
    let kr = mat3x2_mul_mat2(gain, noise);
    add(joseph, mat3x2_mul_mat2x3(&kr, &transpose3x2(gain)))
}

#[inline(always)]
pub(crate) fn covariance_symmetrization(covariance: &[f64; 9]) -> [f64; 9] {
    let transposed = transpose3(covariance);
    let mut result = [0.0; 9];
    for index in 0..9 {
        result[index] = 0.5 * (covariance[index] + transposed[index]);
    }
    result
}

#[inline(always)]
pub(crate) fn candidate_finite(state: &[f64; 3], covariance: &[f64; 9]) -> bool {
    state
        .iter()
        .chain(covariance)
        .all(|value| value.is_finite())
}

#[inline(always)]
pub(crate) fn covariance_positive_diagonal(covariance: &[f64; 9]) -> bool {
    [0, 4, 8].into_iter().all(|index| covariance[index] > 0.0)
}

#[inline(always)]
pub(crate) fn covariance_symmetric(covariance: &[f64; 9]) -> bool {
    let mut maximum_error = 0.0_f64;
    for column in 0..3 {
        for row in 0..3 {
            maximum_error = maximum_error
                .max((covariance[column * 3 + row] - covariance[row * 3 + column]).abs());
        }
    }
    maximum_error <= 1.0e-10
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

    #[test]
    fn square_drive_clips_translation_at_each_boundary() {
        let bounds = [2.5, 7.5];
        let motion = [2.0, 1.5, 0.25];

        assert_eq!(
            square_drive_state_machine(&[7.25, 2.5, 0.0], &motion, &bounds),
            [0.0, 1.0, 1.0, 0.0]
        );
        assert_eq!(
            square_drive_state_machine(
                &[7.5, 7.25, core::f64::consts::FRAC_PI_2],
                &motion,
                &bounds,
            ),
            [1.0, 1.0, 1.0, 0.0]
        );
        assert_eq!(
            square_drive_state_machine(&[2.75, 7.5, core::f64::consts::PI], &motion, &bounds),
            [2.0, 1.0, 1.0, 0.0]
        );
        assert_eq!(
            square_drive_state_machine(
                &[2.5, 2.75, 3.0 * core::f64::consts::FRAC_PI_2],
                &motion,
                &bounds,
            ),
            [3.0, 1.0, 1.0, 0.0]
        );
    }

    #[test]
    fn square_drive_turns_in_place_and_advances_phase() {
        let bounds = [2.5, 7.5];
        let motion = [1.0, 0.9, 0.25];

        assert_eq!(
            square_drive_state_machine(&[7.5, 2.5, 0.0], &motion, &bounds),
            [1.0, 0.0, 0.0, 2.0 * core::f64::consts::PI]
        );
        assert_eq!(
            square_drive_state_machine(&[7.5, 7.5, core::f64::consts::FRAC_PI_2], &motion, &bounds),
            [2.0, 0.0, 0.0, 2.0 * core::f64::consts::PI]
        );
        assert_eq!(
            square_drive_state_machine(&[2.5, 7.5, core::f64::consts::PI], &motion, &bounds),
            [3.0, 0.0, 0.0, 2.0 * core::f64::consts::PI]
        );
        assert_eq!(
            square_drive_state_machine(
                &[2.5, 2.5, 3.0 * core::f64::consts::FRAC_PI_2],
                &motion,
                &bounds,
            ),
            [0.0, 0.0, 0.0, 2.0 * core::f64::consts::PI]
        );
    }

    #[test]
    fn square_drive_rejects_invalid_time_steps() {
        assert_eq!(
            square_drive_state_machine(&[2.5, 2.5, 0.0], &[1.0, 0.9, 0.0], &[2.5, 7.5]),
            [0.0, 0.0, 0.0, 0.0]
        );
    }
}
