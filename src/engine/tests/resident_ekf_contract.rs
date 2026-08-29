#![cfg(feature = "resident-ekf")]

use mech_engine::__gate_b_resident::{FULL_WRITE_ELEMENTS, ResidentEkfBatch, ResidentFullWrite};
use sha2::{Digest, Sha256};

const TRACE: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../benchmarks/runtime/gate-b/ekf-input-v1.bin"
));
const TURNS: usize = 4_096;
const EXPECTED_HASH: &str = "ddca8ab17cb390839d4c77e7cecc5203122f249685f5a28c36fd342cf303a758";

fn inputs() -> impl Iterator<Item = [f64; 4]> {
    assert_eq!(TRACE.len(), TURNS * 32);
    TRACE.chunks_exact(32).map(|row| {
        let number = |start| f64::from_le_bytes(row[start..start + 8].try_into().unwrap());
        [number(0), number(8), number(16), number(24)]
    })
}

fn quantized_hash(states: impl IntoIterator<Item = [f64; 12]>) -> String {
    let mut hash = Sha256::new();
    for state in states {
        for value in state {
            hash.update(((value / 1.0e-10).round() as i64).to_le_bytes());
        }
    }
    format!("{:x}", hash.finalize())
}

fn state_values(resident: &ResidentEkfBatch, instance: usize) -> [f64; 12] {
    let state = resident.state(instance);
    let mut values = [0.0; 12];
    values[..3].copy_from_slice(&state.state);
    values[3..].copy_from_slice(&state.covariance);
    values
}

#[test]
fn frozen_trace_matches_every_scaled_resident_instance() {
    for instances in [1, 8, 64] {
        let mut resident = ResidentEkfBatch::new(instances);
        let mut trajectory = Vec::with_capacity(TURNS);
        for input in inputs() {
            resident
                .turn(input)
                .expect("frozen trace must pass integrity");
            let first = state_values(&resident, 0);
            for instance in 1..instances {
                assert_eq!(state_values(&resident, instance), first);
            }
            trajectory.push(first);
        }
        assert_eq!(quantized_hash(trajectory), EXPECTED_HASH);
        assert_eq!(resident.published_epoch(), TURNS as u64);
    }
}

#[test]
fn rejected_batches_never_escape_and_repeated_aborts_stay_bounded() {
    let input = inputs().next().unwrap();
    let mut resident = ResidentEkfBatch::new(8);
    resident.turn(input).unwrap();
    let published: Vec<_> = (0..8).map(|instance| resident.state(instance)).collect();
    let epoch = resident.published_epoch();
    for _ in 0..10_000 {
        resident.execute_then_abort(input).unwrap();
        assert_eq!(resident.published_epoch(), epoch);
        for (instance, expected) in published.iter().enumerate() {
            assert_eq!(resident.state(instance), *expected);
        }
    }
}

#[test]
fn full_write_uses_the_exact_frozen_recurrence_and_abort_is_invisible() {
    let mut resident = ResidentFullWrite::new();
    let mut expected: Vec<_> = (0..FULL_WRITE_ELEMENTS)
        .map(|index| (index as f64 + 1.0) * 0.0001)
        .collect();
    for input in inputs() {
        resident.turn(input[0]).unwrap();
        for (index, value) in expected.iter_mut().enumerate() {
            let coefficient = ((index % 127) as f64 + 1.0) * 0.000001;
            *value = *value * 1.000001 + coefficient * input[0];
        }
    }
    assert_eq!(resident.published(), expected);
    let published = resident.published().to_vec();
    let epoch = resident.published_epoch();
    resident.execute_then_abort(1.0).unwrap();
    assert_eq!(resident.published_epoch(), epoch);
    assert_eq!(resident.published(), published);
}

#[test]
fn resident_source_has_no_erased_or_legacy_turn_dependencies() {
    let resident_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/resident");
    let mut pending = vec![resident_root];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
                continue;
            }
            if path.extension().and_then(|value| value.to_str()) != Some("rs") {
                continue;
            }
            let source = std::fs::read_to_string(&path).unwrap();
            for forbidden in [
                "plan.slot(",
                "raw_kernel::step",
                "ekf_step",
                "RuntimeExecutionTransaction",
                "CanonicalStateJournal",
                "CanonicalTurnJournal",
                "ReactiveCellId",
                "ValueCell",
                "MutableReference",
            ] {
                assert!(
                    !source.contains(forbidden),
                    "{} contains forbidden {forbidden}",
                    path.display()
                );
            }
            let typed_general_arena = path.ends_with("resident/general/mod.rs");
            if path.file_name().and_then(|value| value.to_str()) != Some("full_write.rs")
                && !typed_general_arena
            {
                assert!(
                    !source.contains("Box<[f64]>"),
                    "{} erases typed resident storage",
                    path.display()
                );
            }
        }
    }
}
