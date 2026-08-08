use std::sync::OnceLock;

use sha2::{Digest, Sha256};

use super::raw_kernel;

pub const EPISODE_LENGTH: usize = 4_096;
pub const SCALED_INSTANCES: [usize; 3] = [1, 8, 64];
pub const DT: f64 = 0.05;
pub const LANDMARK: [f64; 2] = [25.0, -10.0];
pub const PROCESS_COVARIANCE: [f64; 4] = [0.04, 0.0, 0.0, 0.0025];
pub const MEASUREMENT_COVARIANCE: [f64; 4] = [0.25, 0.0, 0.0, 0.0009];
pub const ABSOLUTE_TOLERANCE: f64 = 1.0e-10;
pub const RELATIVE_TOLERANCE: f64 = 1.0e-10;
pub const QUANTIZATION: f64 = 1.0e-10;
pub const TRACE_SHA256: &str = "ab901e1d115aa92166dc2a6d45a28732e6a548363b829997aa410ae4c2d77c8b";
pub const REFERENCE_TRAJECTORY_SHA256: &str =
    "ddca8ab17cb390839d4c77e7cecc5203122f249685f5a28c36fd342cf303a758";

const TRACE_BYTES: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../benchmarks/runtime/gate-b/ekf-input-v1.bin"
));

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EkfInput {
    pub velocity: f64,
    pub angular_velocity: f64,
    pub measured_range: f64,
    pub measured_bearing: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EkfState {
    pub state: [f64; 3],
    pub covariance: [f64; 9],
}

impl EkfState {
    pub const INITIAL: Self = Self {
        state: [2.0, 1.0, 0.15],
        covariance: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.05],
    };

    pub const REFERENCE_FINAL: Self = Self {
        state: [18.169827258925427, 4.339708695271022, 0.2557219366745068],
        covariance: [
            0.3270953723043491,
            0.1509754472729972,
            -0.022618166436367253,
            0.1509754472729972,
            0.07105284175378412,
            -0.010486015657880304,
            -0.022618166436367253,
            -0.010486015657880304,
            0.0016395600302299483,
        ],
    };

    pub fn values(self) -> [f64; 12] {
        let mut values = [0.0; 12];
        values[..3].copy_from_slice(&self.state);
        values[3..].copy_from_slice(&self.covariance);
        values
    }

    pub fn from_values(values: &[f64]) -> Self {
        assert_eq!(values.len(), 12);
        let mut state = [0.0; 3];
        let mut covariance = [0.0; 9];
        state.copy_from_slice(&values[..3]);
        covariance.copy_from_slice(&values[3..]);
        Self { state, covariance }
    }
}

fn decode_f64(bytes: &[u8]) -> f64 {
    f64::from_le_bytes(bytes.try_into().expect("one little-endian f64"))
}

pub fn trace() -> &'static [EkfInput] {
    static TRACE: OnceLock<Vec<EkfInput>> = OnceLock::new();
    TRACE.get_or_init(|| {
        assert_eq!(TRACE_BYTES.len(), EPISODE_LENGTH * 4 * 8);
        TRACE_BYTES
            .chunks_exact(32)
            .map(|row| EkfInput {
                velocity: decode_f64(&row[0..8]),
                angular_velocity: decode_f64(&row[8..16]),
                measured_range: decode_f64(&row[16..24]),
                measured_bearing: decode_f64(&row[24..32]),
            })
            .collect()
    })
}

pub fn trace_sha256() -> String {
    format!("{:x}", Sha256::digest(TRACE_BYTES))
}

pub fn tolerance(expected: f64) -> f64 {
    ABSOLUTE_TOLERANCE + RELATIVE_TOLERANCE * expected.abs()
}

pub fn assert_state_close(actual: EkfState, expected: EkfState, turn: usize) {
    for (index, (actual, expected)) in actual
        .values()
        .into_iter()
        .zip(expected.values())
        .enumerate()
    {
        assert!(
            (actual - expected).abs() <= tolerance(expected),
            "Gate B EKF mismatch at turn {turn}, value {index}: {actual} != {expected}",
        );
    }
}

pub fn reference_trajectory() -> &'static [EkfState] {
    static REFERENCE: OnceLock<Vec<EkfState>> = OnceLock::new();
    REFERENCE.get_or_init(|| {
        let mut current = EkfState::INITIAL;
        let mut states = Vec::with_capacity(EPISODE_LENGTH);
        for input in trace() {
            current = raw_kernel::step(current, *input).expect("frozen EKF trace must be valid");
            states.push(current);
        }
        assert_state_close(current, EkfState::REFERENCE_FINAL, EPISODE_LENGTH);
        assert_eq!(
            quantized_trajectory_hash(&states),
            REFERENCE_TRAJECTORY_SHA256
        );
        states
    })
}

fn quantize(value: f64) -> i64 {
    (value / QUANTIZATION).round() as i64
}

pub fn quantized_trajectory_hash(states: &[EkfState]) -> String {
    let mut hasher = Sha256::new();
    for state in states {
        for value in state.values() {
            hasher.update(quantize(value).to_le_bytes());
        }
    }
    format!("{:x}", hasher.finalize())
}

pub fn state_hash64(state: EkfState) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for value in state.values() {
        for byte in value.to_bits().to_le_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
    }
    hash
}
