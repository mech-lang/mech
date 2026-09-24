use std::{env, hint::black_box, time::Instant};

const DT: f32 = 0.1;
const DT2: f32 = DT * DT;
const Q0: f32 = 0.01;
const Q1: f32 = 0.0025;
const R: f32 = 0.25;
const SYMMETRY_TOLERANCE: f32 = 1.0e-4;

fn main() {
    let instances = argument(1, 100_000_usize).max(1);
    let turns = argument(2, 5_u32).max(1);
    let checked = env::args().nth(3).as_deref() == Some("checked");
    let (velocity, angular_velocity, bearing) = inputs(instances);
    let mut state = vec![[55.0_f32, 25.0, 0.4]; instances];
    let mut covariance = vec![[0.0_f32; 9]; instances];
    reset(&mut covariance);
    dispatch(&mut state, &mut covariance, &velocity, &angular_velocity, &bearing, 5, false);
    reset_state(&mut state, &mut covariance);
    let started = Instant::now();
    let faults = dispatch(&mut state, &mut covariance, &velocity, &angular_velocity, &bearing, turns, checked);
    let elapsed = started.elapsed().as_secs_f64();
    let throughput = instances as f64 * turns as f64 / elapsed;
    let checksum = state
        .iter()
        .flat_map(|value| value.iter())
        .chain(covariance.iter().flat_map(|value| value.iter()))
        .map(|value| *value as f64)
        .sum::<f64>();
    black_box((&state, &covariance));
    println!("lane: Rust optimized fixed-shape scalar");
    println!("instances: {instances}");
    println!("turns: {turns}");
    println!("validation: {}", if checked { "checked" } else { "unchecked" });
    println!("faults: {faults}");
    println!("elapsed_s: {elapsed:.9}");
    println!("throughput: {throughput:.3}");
    println!("checksum: {checksum:.9}");
}

fn argument<T: std::str::FromStr>(index: usize, default: T) -> T {
    env::args().nth(index).and_then(|value| value.parse().ok()).unwrap_or(default)
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

fn reset(state: &mut [[f32; 9]]) {
    for value in state {
        *value = [100.0, 0.0, 0.0, 0.0, 100.0, 0.0, 0.0, 0.0, 0.15];
    }
}

fn reset_state(state: &mut [[f32; 3]], covariance: &mut [[f32; 9]]) {
    for value in state {
        *value = [55.0, 25.0, 0.4];
    }
    reset(covariance);
}

#[inline(always)]
fn dispatch(
    state: &mut [[f32; 3]],
    covariance: &mut [[f32; 9]],
    velocity: &[f32],
    angular_velocity: &[f32],
    bearing: &[f32],
    turns: u32,
    checked: bool,
) -> usize {
    let mut faults = 0;
    for _ in 0..turns {
        for lane in 0..velocity.len() {
            if !step(&mut state[lane], &mut covariance[lane], velocity[lane], angular_velocity[lane], bearing[lane], checked) {
                faults += 1;
            }
        }
    }
    faults
}

#[inline(always)]
fn step(
    state: &mut [f32; 3],
    covariance: &mut [f32; 9],
    velocity: f32,
    angular_velocity: f32,
    bearing: f32,
    checked: bool,
) -> bool {
    let theta = state[2];
    let (st, ct) = theta.sin_cos();
    let distance = velocity * DT;
    let predicted_x0 = state[0] + distance * ct;
    let predicted_x1 = state[1] + distance * st;
    let predicted_x2 = theta + angular_velocity * DT;
    let f02 = -distance * st;
    let f12 = distance * ct;
    let p00 = covariance[0];
    let p01 = covariance[1];
    let p02 = covariance[2];
    let p10 = covariance[3];
    let p11 = covariance[4];
    let p12 = covariance[5];
    let p20 = covariance[6];
    let p21 = covariance[7];
    let p22 = covariance[8];
    let ap00 = p00 + f02 * p20;
    let ap01 = p01 + f02 * p21;
    let ap02 = p02 + f02 * p22;
    let ap10 = p10 + f12 * p20;
    let ap11 = p11 + f12 * p21;
    let ap12 = p12 + f12 * p22;
    let process00 = ct * ct * DT2 * Q0;
    let process01 = ct * st * DT2 * Q0;
    let process11 = st * st * DT2 * Q0;
    let predicted_p00 = ap00 + ap02 * f02 + process00;
    let predicted_p01 = ap01 + ap02 * f12 + process01;
    let predicted_p02 = ap02;
    let predicted_p10 = ap10 + ap12 * f02 + process01;
    let predicted_p11 = ap11 + ap12 * f12 + process11;
    let predicted_p12 = ap12;
    let predicted_p20 = p20 + p22 * f02;
    let predicted_p21 = p21 + p22 * f12;
    let predicted_p22 = p22 + DT2 * Q1;
    let dx = 140.0 - predicted_x0;
    let dy = 12.0 - predicted_x1;
    let squared_range = dx * dx + dy * dy;
    let predicted_bearing = dy.atan2(dx) - predicted_x2;
    let raw_innovation = bearing - predicted_bearing;
    let innovation = raw_innovation.sin().atan2(raw_innovation.cos());
    let h0 = dy / squared_range;
    let h1 = -dx / squared_range;
    let h2 = -1.0;
    let pht0 = predicted_p00 * h0 + predicted_p01 * h1 + predicted_p02 * h2;
    let pht1 = predicted_p10 * h0 + predicted_p11 * h1 + predicted_p12 * h2;
    let pht2 = predicted_p20 * h0 + predicted_p21 * h1 + predicted_p22 * h2;
    let variance = h0 * pht0 + h1 * pht1 + h2 * pht2 + R;
    let k0 = pht0 / variance;
    let k1 = pht1 / variance;
    let k2 = pht2 / variance;
    let candidate_x0 = predicted_x0 + k0 * innovation;
    let candidate_x1 = predicted_x1 + k1 * innovation;
    let candidate_x2 = predicted_x2 + k2 * innovation;
    let a00 = 1.0 - k0 * h0;
    let a01 = -k0 * h1;
    let a02 = -k0 * h2;
    let a10 = -k1 * h0;
    let a11 = 1.0 - k1 * h1;
    let a12 = -k1 * h2;
    let a20 = -k2 * h0;
    let a21 = -k2 * h1;
    let a22 = 1.0 - k2 * h2;
    let b00 = a00 * predicted_p00 + a01 * predicted_p10 + a02 * predicted_p20;
    let b01 = a00 * predicted_p01 + a01 * predicted_p11 + a02 * predicted_p21;
    let b02 = a00 * predicted_p02 + a01 * predicted_p12 + a02 * predicted_p22;
    let b10 = a10 * predicted_p00 + a11 * predicted_p10 + a12 * predicted_p20;
    let b11 = a10 * predicted_p01 + a11 * predicted_p11 + a12 * predicted_p21;
    let b12 = a10 * predicted_p02 + a11 * predicted_p12 + a12 * predicted_p22;
    let b20 = a20 * predicted_p00 + a21 * predicted_p10 + a22 * predicted_p20;
    let b21 = a20 * predicted_p01 + a21 * predicted_p11 + a22 * predicted_p21;
    let b22 = a20 * predicted_p02 + a21 * predicted_p12 + a22 * predicted_p22;
    let candidate_p00 = b00 * a00 + b01 * a01 + b02 * a02 + k0 * k0 * R;
    let candidate_p01 = b00 * a10 + b01 * a11 + b02 * a12 + k0 * k1 * R;
    let candidate_p02 = b00 * a20 + b01 * a21 + b02 * a22 + k0 * k2 * R;
    let candidate_p10 = b10 * a00 + b11 * a01 + b12 * a02 + k1 * k0 * R;
    let candidate_p11 = b10 * a10 + b11 * a11 + b12 * a12 + k1 * k1 * R;
    let candidate_p12 = b10 * a20 + b11 * a21 + b12 * a22 + k1 * k2 * R;
    let candidate_p20 = b20 * a00 + b21 * a01 + b22 * a02 + k2 * k0 * R;
    let candidate_p21 = b20 * a10 + b21 * a11 + b22 * a12 + k2 * k1 * R;
    let candidate_p22 = b20 * a20 + b21 * a21 + b22 * a22 + k2 * k2 * R;
    if checked && !valid_candidate([
        candidate_x0,
        candidate_x1,
        candidate_x2,
    ], [
        candidate_p00,
        candidate_p01,
        candidate_p02,
        candidate_p10,
        candidate_p11,
        candidate_p12,
        candidate_p20,
        candidate_p21,
        candidate_p22,
    ]) {
        return false;
    }
    *state = [candidate_x0, candidate_x1, candidate_x2];
    *covariance = [
        candidate_p00,
        candidate_p01,
        candidate_p02,
        candidate_p10,
        candidate_p11,
        candidate_p12,
        candidate_p20,
        candidate_p21,
        candidate_p22,
    ];
    true
}

#[inline(always)]
fn valid_candidate(state: [f32; 3], covariance: [f32; 9]) -> bool {
    state.into_iter().all(f32::is_finite)
        && covariance.into_iter().all(f32::is_finite)
        && covariance[0] > 0.0
        && covariance[4] > 0.0
        && covariance[8] > 0.0
        && (covariance[1] - covariance[3]).abs() <= SYMMETRY_TOLERANCE
        && (covariance[2] - covariance[6]).abs() <= SYMMETRY_TOLERANCE
        && (covariance[5] - covariance[7]).abs() <= SYMMETRY_TOLERANCE
}
