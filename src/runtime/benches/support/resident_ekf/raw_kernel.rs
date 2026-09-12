use super::contract::{
    DT, EkfInput, EkfState, LANDMARK, MEASUREMENT_COVARIANCE, PROCESS_COVARIANCE,
    assert_state_close, quantized_trajectory_hash, reference_trajectory, trace,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IntegrityError {
    pub reason: &'static str,
}

fn multiply<const OUTPUT: usize>(
    left: &[f64],
    left_rows: usize,
    left_columns: usize,
    right: &[f64],
    right_columns: usize,
) -> [f64; OUTPUT] {
    debug_assert_eq!(OUTPUT, left_rows * right_columns);
    debug_assert_eq!(left.len(), left_rows * left_columns);
    debug_assert_eq!(right.len(), left_columns * right_columns);
    let mut result = [0.0; OUTPUT];
    for column in 0..right_columns {
        for row in 0..left_rows {
            let mut total = 0.0;
            for inner in 0..left_columns {
                total += left[inner * left_rows + row] * right[column * left_columns + inner];
            }
            result[column * left_rows + row] = total;
        }
    }
    result
}

fn transpose<const OUTPUT: usize>(matrix: &[f64], rows: usize, columns: usize) -> [f64; OUTPUT] {
    debug_assert_eq!(OUTPUT, matrix.len());
    let mut result = [0.0; OUTPUT];
    for column in 0..columns {
        for row in 0..rows {
            result[row * columns + column] = matrix[column * rows + row];
        }
    }
    result
}

fn add<const OUTPUT: usize>(left: [f64; OUTPUT], right: [f64; OUTPUT]) -> [f64; OUTPUT] {
    let mut result = [0.0; OUTPUT];
    for index in 0..OUTPUT {
        result[index] = left[index] + right[index];
    }
    result
}

pub fn step(current: EkfState, input: EkfInput) -> Result<EkfState, IntegrityError> {
    let [px, py, theta] = current.state;
    let cosine = theta.cos();
    let sine = theta.sin();
    let motion_jacobian = [
        1.0,
        0.0,
        0.0,
        0.0,
        1.0,
        0.0,
        -input.velocity * sine * DT,
        input.velocity * cosine * DT,
        1.0,
    ];
    let control_jacobian = [cosine * DT, sine * DT, 0.0, 0.0, 0.0, DT];
    let predicted_state = [
        px + input.velocity * cosine * DT,
        py + input.velocity * sine * DT,
        theta + input.angular_velocity * DT,
    ];

    let gp = multiply::<9>(&motion_jacobian, 3, 3, &current.covariance, 3);
    let predicted_covariance = multiply::<9>(&gp, 3, 3, &transpose::<9>(&motion_jacobian, 3, 3), 3);
    let vq = multiply::<6>(&control_jacobian, 3, 2, &PROCESS_COVARIANCE, 2);
    let process_covariance = multiply::<9>(&vq, 3, 2, &transpose::<6>(&control_jacobian, 3, 2), 3);
    let predicted_covariance = add(predicted_covariance, process_covariance);

    let delta_x = LANDMARK[0] - predicted_state[0];
    let delta_y = LANDMARK[1] - predicted_state[1];
    let q = delta_x * delta_x + delta_y * delta_y;
    if q <= 1.0e-12 {
        return Err(IntegrityError {
            reason: "landmark distance",
        });
    }
    let distance = q.sqrt();
    let predicted_measurement = [distance, delta_y.atan2(delta_x) - predicted_state[2]];
    let measurement_jacobian = [
        -delta_x / distance,
        delta_y / q,
        -delta_y / distance,
        -delta_x / q,
        0.0,
        -1.0,
    ];
    let hp = multiply::<6>(&measurement_jacobian, 2, 3, &predicted_covariance, 3);
    let innovation_covariance = add(
        multiply::<4>(&hp, 2, 3, &transpose::<6>(&measurement_jacobian, 2, 3), 2),
        MEASUREMENT_COVARIANCE,
    );
    let determinant = innovation_covariance[0] * innovation_covariance[3]
        - innovation_covariance[2] * innovation_covariance[1];
    if determinant.abs() <= 1.0e-12 {
        return Err(IntegrityError {
            reason: "innovation determinant",
        });
    }
    let inverse_innovation = [
        innovation_covariance[3] / determinant,
        -innovation_covariance[1] / determinant,
        -innovation_covariance[2] / determinant,
        innovation_covariance[0] / determinant,
    ];
    let pht = multiply::<6>(
        &predicted_covariance,
        3,
        3,
        &transpose::<6>(&measurement_jacobian, 2, 3),
        2,
    );
    let gain = multiply::<6>(&pht, 3, 2, &inverse_innovation, 2);
    let innovation = [
        input.measured_range - predicted_measurement[0],
        input.measured_bearing - predicted_measurement[1],
    ];
    let correction = multiply::<3>(&gain, 3, 2, &innovation, 1);
    let corrected_state = [
        predicted_state[0] + correction[0],
        predicted_state[1] + correction[1],
        predicted_state[2] + correction[2],
    ];

    let kh = multiply::<9>(&gain, 3, 2, &measurement_jacobian, 3);
    let identity = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
    let mut joseph_a = [0.0; 9];
    for index in 0..9 {
        joseph_a[index] = identity[index] - kh[index];
    }
    let ap = multiply::<9>(&joseph_a, 3, 3, &predicted_covariance, 3);
    let joseph_covariance = multiply::<9>(&ap, 3, 3, &transpose::<9>(&joseph_a, 3, 3), 3);
    let kr = multiply::<6>(&gain, 3, 2, &MEASUREMENT_COVARIANCE, 2);
    let measurement_covariance = multiply::<9>(&kr, 3, 2, &transpose::<6>(&gain, 3, 2), 3);
    let corrected_covariance = add(joseph_covariance, measurement_covariance);
    let corrected_covariance_transpose = transpose::<9>(&corrected_covariance, 3, 3);
    let mut corrected_covariance_symmetric = [0.0; 9];
    for index in 0..9 {
        corrected_covariance_symmetric[index] =
            0.5 * (corrected_covariance[index] + corrected_covariance_transpose[index]);
    }

    let candidate = EkfState {
        state: corrected_state,
        covariance: corrected_covariance_symmetric,
    };
    validate(candidate)?;
    Ok(candidate)
}

pub fn validate(candidate: EkfState) -> Result<(), IntegrityError> {
    if !candidate.values().into_iter().all(f64::is_finite) {
        return Err(IntegrityError {
            reason: "non-finite state",
        });
    }
    if ![0, 4, 8]
        .into_iter()
        .all(|index| candidate.covariance[index] > 0.0)
    {
        return Err(IntegrityError {
            reason: "covariance diagonal",
        });
    }
    let mut symmetry_error = 0.0_f64;
    for column in 0..3 {
        for row in 0..3 {
            symmetry_error = symmetry_error.max(
                (candidate.covariance[column * 3 + row] - candidate.covariance[row * 3 + column])
                    .abs(),
            );
        }
    }
    if symmetry_error > 1.0e-10 {
        return Err(IntegrityError {
            reason: "covariance symmetry",
        });
    }
    Ok(())
}

pub struct KernelFixture {
    states: Vec<EkfState>,
}

impl KernelFixture {
    pub fn new(instances: usize) -> Self {
        Self {
            states: vec![EkfState::INITIAL; instances],
        }
    }

    pub fn run_episode(&mut self) {
        for input in trace() {
            for state in &mut self.states {
                *state = step(*state, *input).expect("frozen EKF turn");
            }
        }
    }

    pub fn run_and_validate_every_turn(&mut self) -> String {
        let mut trajectory = Vec::with_capacity(super::contract::EPISODE_LENGTH);
        for (turn, (input, expected)) in trace().iter().zip(reference_trajectory()).enumerate() {
            for state in &mut self.states {
                *state = step(*state, *input).expect("frozen EKF turn");
                assert_state_close(*state, *expected, turn + 1);
            }
            trajectory.push(self.states[0]);
        }
        quantized_trajectory_hash(&trajectory)
    }

    pub fn states(&self) -> &[EkfState] {
        &self.states
    }
}
