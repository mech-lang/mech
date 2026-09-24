//! Hand-written Rust control for the Mech fixed-shape native turn ABI.
//!
//! Build with:
//! rustc --crate-type cdylib -C opt-level=3 -C target-cpu=native \
//!   -o target/iros-rust-dylib/librust_ekf.dylib rust-ekf-dylib.rs

use std::slice;

const SYMMETRY_TOLERANCE: f32 = 1.0e-4;

/// Advances one checked EKF turn for every instance.
///
/// The pointer tables match the ABI exported by Mech's Cranelift AOT backend:
/// five structure-of-arrays inputs (`dt`, linear velocity, angular velocity,
/// bearing, and measurement noise) followed by two array-of-structures state
/// buffers (three state components and nine column-major covariance values).
/// A nonzero return packs the first failing instance in the high bits and the
/// one-based constraint code in the low byte.
///
/// # Safety
///
/// The caller must provide the pointer tables and buffer extents described
/// above for at least `instances` elements.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mech_fixed_numeric_turn(
    input_pointers: *const *const f32,
    state_pointers: *const *const f32,
    next_state_pointers: *const *mut f32,
    instances: usize,
) -> u64 {
    let inputs = unsafe { slice::from_raw_parts(input_pointers, 5) };
    let states = unsafe { slice::from_raw_parts(state_pointers, 2) };
    let next_states = unsafe { slice::from_raw_parts(next_state_pointers, 2) };

    for instance in 0..instances {
        let input = |index: usize| unsafe { *inputs[index].add(instance) };
        let state = unsafe { slice::from_raw_parts(states[0].add(instance * 3), 3) };
        let covariance = unsafe { slice::from_raw_parts(states[1].add(instance * 9), 9) };

        let dt = input(0);
        let velocity = input(1);
        let angular_velocity = input(2);
        let bearing = input(3);
        let measurement_noise = input(4);
        let theta = state[2];
        let sin_theta = theta.sin();
        let cos_theta = theta.cos();
        let distance = velocity * dt;
        let predicted_x0 = state[0] + distance * cos_theta;
        let predicted_x1 = state[1] + distance * sin_theta;
        let predicted_x2 = theta + angular_velocity * dt;
        let f02 = -distance * sin_theta;
        let f12 = distance * cos_theta;

        // The native ABI stores fixed-shape matrices in column-major order.
        let p00 = covariance[0];
        let p10 = covariance[1];
        let p20 = covariance[2];
        let p01 = covariance[3];
        let p11 = covariance[4];
        let p21 = covariance[5];
        let p02 = covariance[6];
        let p12 = covariance[7];
        let p22 = covariance[8];

        let ap00 = p00 + f02 * p20;
        let ap01 = p01 + f02 * p21;
        let ap02 = p02 + f02 * p22;
        let ap10 = p10 + f12 * p20;
        let ap11 = p11 + f12 * p21;
        let ap12 = p12 + f12 * p22;
        let dt2 = dt * dt;
        let process00 = cos_theta * cos_theta * dt2 * 0.01;
        let process01 = cos_theta * sin_theta * dt2 * 0.01;
        let process11 = sin_theta * sin_theta * dt2 * 0.01;
        let predicted_p00 = ap00 + ap02 * f02 + process00;
        let predicted_p01 = ap01 + ap02 * f12 + process01;
        let predicted_p02 = ap02;
        let predicted_p10 = ap10 + ap12 * f02 + process01;
        let predicted_p11 = ap11 + ap12 * f12 + process11;
        let predicted_p12 = ap12;
        let predicted_p20 = p20 + p22 * f02;
        let predicted_p21 = p21 + p22 * f12;
        let predicted_p22 = p22 + dt2 * 0.0025;

        let delta_x = 140.0 - predicted_x0;
        let delta_y = 12.0 - predicted_x1;
        let squared_range = delta_x * delta_x + delta_y * delta_y;
        let predicted_bearing = delta_y.atan2(delta_x) - predicted_x2;
        let raw_innovation = bearing - predicted_bearing;
        let innovation = raw_innovation.sin().atan2(raw_innovation.cos());
        let h0 = delta_y / squared_range;
        let h1 = -delta_x / squared_range;
        let h2 = -1.0;
        let pht0 = predicted_p00 * h0 + predicted_p01 * h1 + predicted_p02 * h2;
        let pht1 = predicted_p10 * h0 + predicted_p11 * h1 + predicted_p12 * h2;
        let pht2 = predicted_p20 * h0 + predicted_p21 * h1 + predicted_p22 * h2;
        let variance = h0 * pht0 + h1 * pht1 + h2 * pht2 + measurement_noise;
        let k0 = pht0 / variance;
        let k1 = pht1 / variance;
        let k2 = pht2 / variance;
        let candidate_state = [
            predicted_x0 + k0 * innovation,
            predicted_x1 + k1 * innovation,
            predicted_x2 + k2 * innovation,
        ];

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
        let candidate_covariance = [
            b00 * a00 + b01 * a01 + b02 * a02 + k0 * k0 * measurement_noise,
            b10 * a00 + b11 * a01 + b12 * a02 + k1 * k0 * measurement_noise,
            b20 * a00 + b21 * a01 + b22 * a02 + k2 * k0 * measurement_noise,
            b00 * a10 + b01 * a11 + b02 * a12 + k0 * k1 * measurement_noise,
            b10 * a10 + b11 * a11 + b12 * a12 + k1 * k1 * measurement_noise,
            b20 * a10 + b21 * a11 + b22 * a12 + k2 * k1 * measurement_noise,
            b00 * a20 + b01 * a21 + b02 * a22 + k0 * k2 * measurement_noise,
            b10 * a20 + b11 * a21 + b12 * a22 + k1 * k2 * measurement_noise,
            b20 * a20 + b21 * a21 + b22 * a22 + k2 * k2 * measurement_noise,
        ];

        let next_state = unsafe { slice::from_raw_parts_mut(next_states[0].add(instance * 3), 3) };
        let next_covariance =
            unsafe { slice::from_raw_parts_mut(next_states[1].add(instance * 9), 9) };
        next_state.copy_from_slice(&candidate_state);
        next_covariance.copy_from_slice(&candidate_covariance);

        let constraint = if !candidate_state.iter().copied().all(f32::is_finite)
            || !candidate_covariance.iter().copied().all(f32::is_finite)
        {
            1
        } else if candidate_covariance[0] <= 0.0
            || candidate_covariance[4] <= 0.0
            || candidate_covariance[8] <= 0.0
        {
            2
        } else if (candidate_covariance[3] - candidate_covariance[1]).abs() > SYMMETRY_TOLERANCE
            || (candidate_covariance[6] - candidate_covariance[2]).abs() > SYMMETRY_TOLERANCE
            || (candidate_covariance[7] - candidate_covariance[5]).abs() > SYMMETRY_TOLERANCE
        {
            3
        } else {
            0
        };
        if constraint != 0 {
            return ((instance as u64) << 8) | constraint;
        }
    }
    0
}
