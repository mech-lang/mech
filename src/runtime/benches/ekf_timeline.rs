use std::env;
use std::hint::black_box;
use std::time::Instant;

mod support;

use support::gate_b::contract::{EPISODE_LENGTH, EkfState, assert_state_close};
use support::gate_b::legacy_atomic::LegacyEkfFixture;
use support::gate_b::raw_kernel::KernelFixture;
use support::gate_b::resident_turn::ResidentTurnFixture;

const DEFAULT_SAMPLES: usize = 60;
const WARMUP_SAMPLES: usize = 3;

fn samples_from_args() -> usize {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    let mut samples = DEFAULT_SAMPLES;
    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--bench" => {}
            "--samples" => {
                index += 1;
                samples = arguments
                    .get(index)
                    .expect("--samples requires a value")
                    .parse()
                    .expect("--samples must be a positive integer");
            }
            argument => panic!("unknown argument: {argument}"),
        }
        index += 1;
    }
    assert!(samples > 0, "--samples must be positive");
    samples
}

fn emit(lane: &str, sample: usize, elapsed_ns: u128) {
    println!(
        "{{\"lane\":\"{lane}\",\"sample\":{sample},\"turns\":{EPISODE_LENGTH},\"elapsed_ns\":{elapsed_ns},\"gc_ns\":null}}"
    );
}

fn measure_raw(samples: usize) {
    for sample in 0..(WARMUP_SAMPLES + samples) {
        let mut fixture = KernelFixture::new(1);
        let started = Instant::now();
        fixture.run_episode();
        let elapsed = started.elapsed().as_nanos();
        assert_state_close(
            fixture.states()[0],
            EkfState::REFERENCE_FINAL,
            EPISODE_LENGTH,
        );
        black_box(fixture.states());
        if sample >= WARMUP_SAMPLES {
            emit("rust-raw", sample - WARMUP_SAMPLES, elapsed);
        }
    }
}

fn measure_resident(samples: usize) {
    for sample in 0..(WARMUP_SAMPLES + samples) {
        let mut fixture = ResidentTurnFixture::new(1, 0, 1);
        let started = Instant::now();
        fixture.run_episode();
        let elapsed = started.elapsed().as_nanos();
        fixture.validate_final();
        black_box(fixture.state(0));
        if sample >= WARMUP_SAMPLES {
            emit("mech-resident-complete", sample - WARMUP_SAMPLES, elapsed);
        }
    }
}

fn measure_atomic(samples: usize) {
    for sample in 0..(WARMUP_SAMPLES + samples) {
        let mut fixture = LegacyEkfFixture::new(1);
        let started = Instant::now();
        fixture.run_episode();
        let elapsed = started.elapsed().as_nanos();
        let states = fixture.states();
        assert_state_close(states[0], EkfState::REFERENCE_FINAL, EPISODE_LENGTH);
        black_box(states);
        if sample >= WARMUP_SAMPLES {
            emit("mech-current-atomic", sample - WARMUP_SAMPLES, elapsed);
        }
    }
}

fn main() {
    let samples = samples_from_args();
    eprintln!(
        "Gate B EKF timeline: {samples} ordered samples, {EPISODE_LENGTH} turns/sample, setup excluded"
    );
    measure_raw(samples);
    measure_resident(samples);
    measure_atomic(samples);
}
