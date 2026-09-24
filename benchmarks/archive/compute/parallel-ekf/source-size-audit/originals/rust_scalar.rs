use std::{env, hint::black_box, time::Instant};
#[derive(Default)]
struct Scratch {
    f: [f32; 9],
    ft: [f32; 9],
    g: [f32; 6],
    gt: [f32; 6],
    left: [f32; 9],
    predicted_p: [f32; 9],
    process_left: [f32; 6],
    process_p: [f32; 9],
    pht: [f32; 3],
    a: [f32; 9],
    at: [f32; 9],
    ap: [f32; 9],
    corrected_p: [f32; 9],
}
fn main() {
    let instances = argument(1, 100_000_usize).max(1);
    let turns = argument(2, 5_u32).max(1);
    let checked = env::args().nth(3).as_deref() == Some("checked");
    let (velocity, angular_velocity, bearing) = inputs(instances);
    let mut state = vec![0.0_f32; instances * 3];
    let mut covariance = vec![0.0_f32; instances * 9];
    reset(&mut state, &mut covariance);
    let mut scratch = Scratch::default();
    dispatch::<false>(
        &mut state,
        &mut covariance,
        &velocity,
        &angular_velocity,
        &bearing,
        5,
        &mut scratch,
    );
    reset(&mut state, &mut covariance);
    let started = Instant::now();
    let faults = if checked {
        dispatch::<true>(
            &mut state,
            &mut covariance,
            &velocity,
            &angular_velocity,
            &bearing,
            turns,
            &mut scratch,
        )
    } else {
        dispatch::<false>(
            &mut state,
            &mut covariance,
            &velocity,
            &angular_velocity,
            &bearing,
            turns,
            &mut scratch,
        )
    };
    let elapsed = started.elapsed();
    let seconds = elapsed.as_secs_f64();
    let throughput = instances as f64 * turns as f64 / seconds;
    let checksum = state
        .iter()
        .chain(&covariance)
        .map(|value| *value as f64)
        .sum::<f64>();
    black_box((&state, &covariance));
    println!("lane: Rust optimized fixed-shape");
    println!("instances: {instances}");
    println!("turns: {turns}");
    println!("validation: {}", if checked { "checked" } else { "unchecked" });
    println!("faults: {faults}");
    println!("elapsed_s: {seconds:.9}");
    println!("throughput: {throughput:.3}");
    println!("checksum: {checksum:.9}");
}
fn argument<T: std::str::FromStr>(index: usize, default: T) -> T {
    env::args()
        .nth(index)
        .and_then(|argument| argument.parse().ok())
        .unwrap_or(default)
}
fn inputs(instances: usize) -> (Vec<f32>, Vec<f32>, Vec<f32>) {
    let denominator = instances as f32;
    let mut velocity = Vec::with_capacity(instances);
    let mut angular_velocity = Vec::with_capacity(instances);
    let mut bearing = Vec::with_capacity(instances);
    for index in 0..instances {
        let phase = std::f32::consts::TAU * index as f32 / denominator;
        velocity.push(1.0 + 0.05 * (phase * 3.0).sin());
        angular_velocity.push(0.015 * (1.0 + 0.1 * (phase * 2.0).sin()));
        bearing.push(-0.55 + 0.01 * (phase * 7.0).sin() + 0.005 * (phase * 11.0).sin());
    }
    (velocity, angular_velocity, bearing)
}
fn reset(state: &mut [f32], covariance: &mut [f32]) {
    for lane in 0..state.len() / 3 {
        state[lane * 3..lane * 3 + 3].copy_from_slice(&[55.0, 25.0, 0.4]);
        covariance[lane * 9..lane * 9 + 9]
            .copy_from_slice(&[100.0, 0.0, 0.0, 0.0, 100.0, 0.0, 0.0, 0.0, 0.15]);
    }
}
fn dispatch<const CHECKED: bool>(
    state: &mut [f32],
    covariance: &mut [f32],
    velocity: &[f32],
    angular_velocity: &[f32],
    bearing: &[f32],
    turns: u32,
    scratch: &mut Scratch,
) -> usize {
    let mut faults = 0;
    for _ in 0..turns {
        for lane in 0..velocity.len() {
            if !step::<CHECKED>(
                &mut state[lane * 3..lane * 3 + 3],
                &mut covariance[lane * 9..lane * 9 + 9],
                velocity[lane],
                angular_velocity[lane],
                bearing[lane],
                scratch,
            ) {
                faults += 1;
            }
        }
    }
    faults
}
#[inline(always)]
fn step<const CHECKED: bool>(
    state: &mut [f32],
    covariance: &mut [f32],
    velocity: f32,
    angular_velocity: f32,
    bearing: f32,
    s: &mut Scratch,
) -> bool {
    let dt = 0.1_f32;
    let sin_theta = state[2].sin();
    let cos_theta = state[2].cos();
    let distance = velocity * dt;
    let predicted_state = [
        state[0] + distance * cos_theta,
        state[1] + distance * sin_theta,
        state[2] + angular_velocity * dt,
    ];
    s.f = [
        1.0,
        0.0,
        0.0,
        0.0,
        1.0,
        0.0,
        -distance * sin_theta,
        distance * cos_theta,
        1.0,
    ];
    s.g = [cos_theta * dt, sin_theta * dt, 0.0, 0.0, 0.0, dt];
    transpose(&s.f, 3, 3, &mut s.ft);
    matmul(&s.f, 3, 3, covariance, 3, &mut s.left);
    matmul(&s.left, 3, 3, &s.ft, 3, &mut s.predicted_p);
    matmul(
        &s.g,
        3,
        2,
        &[0.01, 0.0, 0.0, 0.0025],
        2,
        &mut s.process_left,
    );
    transpose(&s.g, 3, 2, &mut s.gt);
    matmul(&s.process_left, 3, 2, &s.gt, 3, &mut s.process_p);
    for index in 0..9 {
        s.predicted_p[index] += s.process_p[index];
    }
    let delta_x = 140.0 - predicted_state[0];
    let delta_y = 12.0 - predicted_state[1];
    let squared_range = delta_x * delta_x + delta_y * delta_y;
    let predicted_bearing = delta_y.atan2(delta_x) - predicted_state[2];
    let raw_innovation = bearing - predicted_bearing;
    let innovation = raw_innovation.sin().atan2(raw_innovation.cos());
    let h = [delta_y / squared_range, -delta_x / squared_range, -1.0];
    for row in 0..3 {
        s.pht[row] = (0..3)
            .map(|inner| s.predicted_p[row + inner * 3] * h[inner])
            .sum();
    }
    let innovation_variance = h
        .iter()
        .zip(s.pht)
        .map(|(left, right)| left * right)
        .sum::<f32>()
        + 0.25;
    let gain = s.pht.map(|value| value / innovation_variance);
    let candidate_state = [
        predicted_state[0] + gain[0] * innovation,
        predicted_state[1] + gain[1] * innovation,
        predicted_state[2] + gain[2] * innovation,
    ];
    for column in 0..3 {
        for row in 0..3 {
            s.a[row + column * 3] = f32::from(row == column) - gain[row] * h[column];
        }
    }
    transpose(&s.a, 3, 3, &mut s.at);
    matmul(&s.a, 3, 3, &s.predicted_p, 3, &mut s.ap);
    matmul(&s.ap, 3, 3, &s.at, 3, &mut s.corrected_p);
    for column in 0..3 {
        for row in 0..3 {
            s.corrected_p[row + column * 3] =
                s.corrected_p[row + column * 3] + gain[row] * gain[column] * 0.25;
        }
    }
    if CHECKED && !valid_candidate(&candidate_state, &s.corrected_p) {
        return false;
    }
    state.copy_from_slice(&candidate_state);
    covariance.copy_from_slice(&s.corrected_p);
    true
}
#[inline(always)]
fn valid_candidate(state: &[f32; 3], covariance: &[f32; 9]) -> bool {
    state.iter().all(|value| value.is_finite())
        && covariance.iter().all(|value| value.is_finite())
        && covariance[0] > 0.0
        && covariance[4] > 0.0
        && covariance[8] > 0.0
        && (covariance[1] - covariance[3]).abs() <= 1.0e-4
        && (covariance[2] - covariance[6]).abs() <= 1.0e-4
        && (covariance[5] - covariance[7]).abs() <= 1.0e-4
}
#[inline(always)]
fn matmul(a: &[f32], rows: usize, inner: usize, b: &[f32], columns: usize, out: &mut [f32]) {
    for column in 0..columns {
        for row in 0..rows {
            out[row + column * rows] = (0..inner)
                .map(|index| a[row + index * rows] * b[index + column * inner])
                .sum();
        }
    }
}
#[inline(always)]
fn transpose(input: &[f32], rows: usize, columns: usize, out: &mut [f32]) {
    for column in 0..columns {
        for row in 0..rows {
            out[column + row * columns] = input[row + column * rows];
        }
    }
}
