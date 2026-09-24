use std::{env, hint::black_box, slice, time::Instant};
use wide::f32x4;
const LANES: usize = 4;
const DT: f32 = 0.1;
const SYMMETRY_TOLERANCE: f32 = 1.0e-4;
#[derive(Clone, Copy)]
struct V4(f32x4);
impl V4 {
    const ZERO: Self = Self(f32x4::ZERO);
    #[inline(always)]
    fn splat(value: f32) -> Self {
        Self(f32x4::splat(value))
    }
    #[inline(always)]
    fn load(values: &[f32], base: usize) -> Self {
        Self(f32x4::new([
            values[base],
            values[base + 1],
            values[base + 2],
            values[base + 3],
        ]))
    }
    #[inline(always)]
    fn store(self, values: &mut [f32], base: usize) {
        values[base..base + LANES].copy_from_slice(&self.0.to_array());
    }
    #[inline(always)]
    fn lanes(self) -> [f32; LANES] {
        self.0.to_array()
    }
    #[inline(always)]
    fn sin_cos(self) -> (Self, Self) {
        let mut sin = [0.0; LANES];
        let mut cos = [0.0; LANES];
        for (index, value) in self.lanes().into_iter().enumerate() {
            let (s, c) = value.sin_cos();
            sin[index] = s;
            cos[index] = c;
        }
        (Self(f32x4::new(sin)), Self(f32x4::new(cos)))
    }
    #[inline(always)]
    fn atan2(self, other: Self) -> Self {
        let left = self.lanes();
        let right = other.lanes();
        let mut result = [0.0; LANES];
        for index in 0..LANES {
            result[index] = left[index].atan2(right[index]);
        }
        Self(f32x4::new(result))
    }
    #[inline(always)]
    fn is_finite(self) -> [bool; LANES] {
        self.lanes().map(f32::is_finite)
    }
}
impl std::ops::Add for V4 {
    type Output = Self;
    #[inline(always)]
    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0)
    }
}
impl std::ops::Sub for V4 {
    type Output = Self;
    #[inline(always)]
    fn sub(self, rhs: Self) -> Self {
        Self(self.0 - rhs.0)
    }
}
impl std::ops::Mul for V4 {
    type Output = Self;
    #[inline(always)]
    fn mul(self, rhs: Self) -> Self {
        Self(self.0 * rhs.0)
    }
}
impl std::ops::Div for V4 {
    type Output = Self;
    #[inline(always)]
    fn div(self, rhs: Self) -> Self {
        Self(self.0 / rhs.0)
    }
}
impl std::ops::Neg for V4 {
    type Output = Self;
    #[inline(always)]
    fn neg(self) -> Self {
        Self(-self.0)
    }
}
type Matrix<const ROWS: usize, const COLS: usize> = [[V4; COLS]; ROWS];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Fault {
    turn: u32,
    instance: usize,
    constraint: u8,
}
#[inline(always)]
fn transpose<const ROWS: usize, const COLS: usize>(
    input: &Matrix<ROWS, COLS>,
) -> Matrix<COLS, ROWS> {
    std::array::from_fn(|row| std::array::from_fn(|column| input[column][row]))
}
#[inline(always)]
fn matmul<const ROWS: usize, const INNER: usize, const COLS: usize>(
    left: &Matrix<ROWS, INNER>,
    right: &Matrix<INNER, COLS>,
) -> Matrix<ROWS, COLS> {
    std::array::from_fn(|row| {
        std::array::from_fn(|column| {
            (0..INNER).fold(V4::ZERO, |sum, index| {
                sum + left[row][index] * right[index][column]
            })
        })
    })
}
#[inline(always)]
fn candidate_faults(state: &[V4; 3], covariance: &Matrix<3, 3>) -> [u8; LANES] {
    let mut faults = [0; LANES];
    for value in state
        .iter()
        .copied()
        .chain(covariance.iter().flat_map(|row| row.iter().copied()))
    {
        for (index, finite) in value.is_finite().into_iter().enumerate() {
            if !finite && faults[index] == 0 {
                faults[index] = 1;
            }
        }
    }
    for diagonal in 0..3 {
        for (index, value) in covariance[diagonal][diagonal]
            .lanes()
            .into_iter()
            .enumerate()
        {
            if value <= 0.0 && faults[index] == 0 {
                faults[index] = 2;
            }
        }
    }
    for row in 0..3 {
        for column in row + 1..3 {
            let left = covariance[row][column].lanes();
            let right = covariance[column][row].lanes();
            for index in 0..LANES {
                if (left[index] - right[index]).abs() > SYMMETRY_TOLERANCE
                    && faults[index] == 0
                {
                    faults[index] = 3;
                }
            }
        }
    }
    faults
}
#[inline(always)]
fn step_group(
    state: &mut [V4; 3],
    covariance: &mut Matrix<3, 3>,
    velocity: V4,
    angular_velocity: V4,
    bearing: V4,
    checked: bool,
) -> [u8; LANES] {
    let theta = state[2];
    let (sin_theta, cos_theta) = theta.sin_cos();
    let distance = velocity * V4::splat(DT);
    let predicted_state = [
        state[0] + distance * cos_theta,
        state[1] + distance * sin_theta,
        state[2] + angular_velocity * V4::splat(DT),
    ];
    let f: Matrix<3, 3> = [
        [V4::splat(1.0), V4::ZERO, -distance * sin_theta],
        [V4::ZERO, V4::splat(1.0), distance * cos_theta],
        [V4::ZERO, V4::ZERO, V4::splat(1.0)],
    ];
    let g: Matrix<3, 2> = [
        [cos_theta * V4::splat(DT), V4::ZERO],
        [sin_theta * V4::splat(DT), V4::ZERO],
        [V4::ZERO, V4::splat(DT)],
    ];
    let ft = transpose(&f);
    let gt = transpose(&g);
    let process_noise: Matrix<2, 2> = [[V4::splat(0.01), V4::ZERO], [V4::ZERO, V4::splat(0.0025)]];
    let predicted_covariance = {
        let first = matmul(&matmul(&f, covariance), &ft);
        let second = matmul(&matmul(&g, &process_noise), &gt);
        std::array::from_fn(|row| {
            std::array::from_fn(|column| first[row][column] + second[row][column])
        })
    };
    let delta_x = V4::splat(140.0) - predicted_state[0];
    let delta_y = V4::splat(12.0) - predicted_state[1];
    let squared_range = delta_x * delta_x + delta_y * delta_y;
    let predicted_bearing = delta_y.atan2(delta_x) - predicted_state[2];
    let raw_innovation = bearing - predicted_bearing;
    let (innovation_sin, innovation_cos) = raw_innovation.sin_cos();
    let innovation = innovation_sin.atan2(innovation_cos);
    let h = [
        delta_y / squared_range,
        -delta_x / squared_range,
        V4::splat(-1.0),
    ];
    let ph_t: [V4; 3] = std::array::from_fn(|row| {
        (0..3).fold(V4::ZERO, |sum, index| {
            sum + predicted_covariance[row][index] * h[index]
        })
    });
    let innovation_variance =
        (0..3).fold(V4::splat(0.25), |sum, index| sum + h[index] * ph_t[index]);
    let gain = ph_t.map(|value| value / innovation_variance);
    let next_state = std::array::from_fn(|row| predicted_state[row] + gain[row] * innovation);
    let a: Matrix<3, 3> = std::array::from_fn(|row| {
        std::array::from_fn(|column| {
            let identity = if row == column {
                V4::splat(1.0)
            } else {
                V4::ZERO
            };
            identity - gain[row] * h[column]
        })
    });
    let at = transpose(&a);
    let corrected_base = matmul(&matmul(&a, &predicted_covariance), &at);
    let next_covariance = std::array::from_fn(|row| {
        std::array::from_fn(|column| {
            corrected_base[row][column] + gain[row] * gain[column] * V4::splat(0.25)
        })
    });
    if !checked {
        *state = next_state;
        *covariance = next_covariance;
        return [0; LANES];
    }
    let faults = candidate_faults(&next_state, &next_covariance);
    if faults.into_iter().all(|constraint| constraint == 0) {
        *state = next_state;
        *covariance = next_covariance;
    }
    faults
}
fn inputs(instances: usize) -> (Vec<f32>, Vec<f32>, Vec<f32>) {
    let denominator = instances as f32;
    let mut velocity = vec![0.0; instances];
    let mut angular_velocity = vec![0.0; instances];
    let mut bearing = vec![0.0; instances];
    for index in 0..instances {
        let phase = std::f32::consts::TAU * index as f32 / denominator;
        velocity[index] = 1.0 + 0.05 * (phase * 3.0).sin();
        angular_velocity[index] = 0.015 * (1.0 + 0.1 * (phase * 2.0).sin());
        bearing[index] = -0.55 + 0.01 * (phase * 7.0).sin() + 0.005 * (phase * 11.0).sin();
    }
    (velocity, angular_velocity, bearing)
}
fn reset(state: &mut [Vec<f32>; 3], covariance: &mut [Vec<f32>; 9]) {
    for values in state.iter_mut() {
        values.fill(0.0);
    }
    for values in covariance.iter_mut() {
        values.fill(0.0);
    }
    state[0].fill(55.0);
    state[1].fill(25.0);
    state[2].fill(0.4);
    covariance[0].fill(100.0);
    covariance[4].fill(100.0);
    covariance[8].fill(0.15);
}
fn dispatch(
    state: &mut [Vec<f32>; 3],
    covariance: &mut [Vec<f32>; 9],
    velocity: &[f32],
    angular_velocity: &[f32],
    bearing: &[f32],
    turns: u32,
    checked: bool,
) -> Option<Fault> {
    // Checked mode snapshots each publication boundary. A failed candidate
    // rejects the whole turn and restores the previously published arrays.
    for turn in 0..turns {
        let checkpoint = checked.then(|| (state.clone(), covariance.clone()));
        for base in (0..velocity.len()).step_by(LANES) {
            let mut packed_state = std::array::from_fn(|row| V4::load(&state[row], base));
            let packed_covariance: [V4; 9] =
                std::array::from_fn(|index| V4::load(&covariance[index], base));
            let mut matrix = std::array::from_fn(|row| {
                std::array::from_fn(|column| packed_covariance[row * 3 + column])
            });
            let faults = step_group(
                &mut packed_state,
                &mut matrix,
                V4::load(velocity, base),
                V4::load(angular_velocity, base),
                V4::load(bearing, base),
                checked,
            );
            if let Some(lane) = faults.iter().position(|constraint| *constraint != 0) {
                if let Some((checkpoint_state, checkpoint_covariance)) = checkpoint {
                    *state = checkpoint_state;
                    *covariance = checkpoint_covariance;
                }
                return Some(Fault {
                    turn,
                    instance: base + lane,
                    constraint: faults[lane],
                });
            }
            for row in 0..3 {
                packed_state[row].store(&mut state[row], base);
            }
            for row in 0..3 {
                for column in 0..3 {
                    matrix[row][column].store(&mut covariance[row * 3 + column], base);
                }
            }
        }
    }
    None
}

fn dispatch_parallel_fused(
    state: &mut [Vec<f32>; 3],
    covariance: &mut [Vec<f32>; 9],
    velocity: &[f32],
    angular_velocity: &[f32],
    bearing: &[f32],
    turns: u32,
    checked: bool,
    workers: usize,
) -> Option<Fault> {
    let groups = velocity.len() / LANES;
    let workers = workers.max(1).min(groups.max(1));
    // Convert the disjoint buffer starts to integers before crossing the
    // scoped-thread boundary. Each worker reconstructs only its own slices;
    // ranges never overlap, and all backing Vecs remain live until the scope
    // joins.
    let state_ptrs = [
        state[0].as_mut_ptr() as usize,
        state[1].as_mut_ptr() as usize,
        state[2].as_mut_ptr() as usize,
    ];
    let covariance_ptrs: [usize; 9] =
        std::array::from_fn(|index| covariance[index].as_mut_ptr() as usize);
    let velocity_ptr = velocity.as_ptr() as usize;
    let angular_velocity_ptr = angular_velocity.as_ptr() as usize;
    let bearing_ptr = bearing.as_ptr() as usize;
    // Checked mode uses one block-start checkpoint, matching Mech's fused
    // boundary: no partial worker result becomes externally visible.
    let checkpoints = checked.then(|| (state.clone(), covariance.clone()));
    let faults = std::thread::scope(|scope| {
        let mut handles = Vec::with_capacity(workers);
        for worker in 0..workers {
            let start_group = groups * worker / workers;
            let end_group = groups * (worker + 1) / workers;
            let group_count = end_group - start_group;
            let state_ptrs = state_ptrs;
            let covariance_ptrs = covariance_ptrs;
            handles.push(scope.spawn(move || {
                let count = group_count * LANES;
                let offset = start_group * LANES;
                let state_slices = unsafe {
                    [
                        slice::from_raw_parts_mut(
                            (state_ptrs[0] as *mut f32).add(offset),
                            count,
                        ),
                        slice::from_raw_parts_mut(
                            (state_ptrs[1] as *mut f32).add(offset),
                            count,
                        ),
                        slice::from_raw_parts_mut(
                            (state_ptrs[2] as *mut f32).add(offset),
                            count,
                        ),
                    ]
                };
                let covariance_slices: [&mut [f32]; 9] = unsafe {
                    std::array::from_fn(|index| {
                        slice::from_raw_parts_mut(
                            (covariance_ptrs[index] as *mut f32).add(offset),
                            count,
                        )
                    })
                };
                let velocities = unsafe {
                    slice::from_raw_parts((velocity_ptr as *const f32).add(offset), count)
                };
                let angular_velocities = unsafe {
                    slice::from_raw_parts(
                        (angular_velocity_ptr as *const f32).add(offset),
                        count,
                    )
                };
                let bearings = unsafe {
                    slice::from_raw_parts((bearing_ptr as *const f32).add(offset), count)
                };
                let mut first_fault = None;
                'groups: for group in 0..group_count {
                    let base = group * LANES;
                    let mut packed_state =
                        std::array::from_fn(|row| V4::load(state_slices[row], base));
                    let packed_covariance: [V4; 9] =
                        std::array::from_fn(|index| V4::load(covariance_slices[index], base));
                    let mut matrix = std::array::from_fn(|row| {
                        std::array::from_fn(|column| packed_covariance[row * 3 + column])
                    });
                    let velocity = V4::load(velocities, base);
                    let angular_velocity = V4::load(angular_velocities, base);
                    let bearing = V4::load(bearings, base);
                    for turn in 0..turns {
                        let faults = step_group(
                            &mut packed_state,
                            &mut matrix,
                            velocity,
                            angular_velocity,
                            bearing,
                            checked,
                        );
                        if let Some(lane) = faults.iter().position(|constraint| *constraint != 0) {
                            first_fault = Some(Fault {
                                turn,
                                instance: offset + base + lane,
                                constraint: faults[lane],
                            });
                            break 'groups;
                        }
                    }
                    for row in 0..3 {
                        packed_state[row].store(state_slices[row], base);
                    }
                    for row in 0..3 {
                        for column in 0..3 {
                            matrix[row][column].store(
                                covariance_slices[row * 3 + column],
                                base,
                            );
                        }
                    }
                }
                first_fault
            }));
        }
        handles
            .into_iter()
            .filter_map(|handle| handle.join().expect("Rust EKF worker must not panic"))
            .min_by_key(|fault| (fault.turn, fault.instance))
    });
    if let Some((checkpoint_state, checkpoint_covariance)) = checkpoints {
        if faults.is_some() {
            *state = checkpoint_state;
            *covariance = checkpoint_covariance;
        }
    }
    faults
}
fn argument<T: std::str::FromStr>(index: usize, default: T) -> T {
    env::args()
        .nth(index)
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}
fn main() {
    let instances = argument(1, 100_000_usize).max(1);
    let turns = argument(2, 5_u32).max(1);
    let checked = env::args()
        .nth(3)
        .is_some_and(|value| value.eq_ignore_ascii_case("checked"));
    let fused = env::args()
        .nth(4)
        .is_some_and(|value| matches!(value.to_ascii_lowercase().as_str(), "fused" | "batched"));
    let workers = argument(5, 8_usize).max(1);
    assert!(
        instances % LANES == 0,
        "Rust SIMD requires instances divisible by four"
    );
    let (velocity, angular_velocity, bearing) = inputs(instances);
    let mut state = std::array::from_fn(|_| vec![0.0; instances]);
    let mut covariance = std::array::from_fn(|_| vec![0.0; instances]);
    reset(&mut state, &mut covariance);
    let warmup_fault = if fused {
        dispatch_parallel_fused(
            &mut state,
            &mut covariance,
            &velocity,
            &angular_velocity,
            &bearing,
            5,
            checked,
            workers,
        )
    } else {
        dispatch(
            &mut state,
            &mut covariance,
            &velocity,
            &angular_velocity,
            &bearing,
            5,
            checked,
        )
    };
    if let Some(fault) = warmup_fault {
        panic!(
            "warmup rejected candidate at turn {} instance {} constraint {}",
            fault.turn, fault.instance, fault.constraint
        );
    }
    reset(&mut state, &mut covariance);
    let started = Instant::now();
    let fault = if fused {
        dispatch_parallel_fused(
            &mut state,
            &mut covariance,
            &velocity,
            &angular_velocity,
            &bearing,
            turns,
            checked,
            workers,
        )
    } else {
        dispatch(
            &mut state,
            &mut covariance,
            &velocity,
            &angular_velocity,
            &bearing,
            turns,
            checked,
        )
    };
    if let Some(fault) = fault {
        panic!(
            "timed run rejected candidate at turn {} instance {} constraint {}",
            fault.turn, fault.instance, fault.constraint
        );
    }
    let elapsed = started.elapsed().as_secs_f64();
    let throughput = instances as f64 * turns as f64 / elapsed;
    let checksum = state
        .iter()
        .chain(covariance.iter())
        .flat_map(|values| values.iter())
        .map(|value| *value as f64)
        .sum::<f64>();
    black_box((&state, &covariance));
    println!("lane: Rust packed SIMD (wide f32x4)");
    println!(
        "validation: {}",
        if checked { "checked" } else { "unchecked" }
    );
    println!("instances: {instances}");
    println!("turns: {turns}");
    println!("workers: {workers}");
    println!(
        "synchronization: {}",
        if fused { "once after fused block" } else { "per-turn" }
    );
    println!("elapsed_s: {elapsed:.9}");
    println!("throughput: {throughput:.3}");
    println!("checksum: {checksum:.9}");
    println!("faults: {}", u64::from(fault.is_some()));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_fused_rejects_a_block_atomically() {
        let instances = 8;
        let (velocity, angular_velocity, mut bearing) = inputs(instances);
        bearing[0] = f32::NAN;
        let mut state = std::array::from_fn(|_| vec![0.0; instances]);
        let mut covariance = std::array::from_fn(|_| vec![0.0; instances]);
        reset(&mut state, &mut covariance);
        let published_state = state.clone();
        let published_covariance = covariance.clone();
        let fault = dispatch_parallel_fused(
            &mut state,
            &mut covariance,
            &velocity,
            &angular_velocity,
            &bearing,
            3,
            true,
            2,
        );
        assert_eq!(fault.unwrap().instance, 0);
        assert_eq!(fault.unwrap().constraint, 1);
        assert_eq!(state, published_state);
        assert_eq!(covariance, published_covariance);
    }
}
