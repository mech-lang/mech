use std::{env, hint::black_box, mem::size_of, time::Instant};

use metal::{CompileOptions, Device, MTLResourceOptions, MTLSize};

const COMPONENTS: usize = 12;
const THREADGROUP_SIZE: u64 = 64;
const WARMUP_TURNS: u32 = 5;

fn buffer_from_f32(device: &Device, values: &[f32]) -> metal::Buffer {
    device.new_buffer_with_data(
        values.as_ptr().cast(),
        std::mem::size_of_val(values) as u64,
        MTLResourceOptions::StorageModeShared,
    )
}

fn buffer_from_u32(device: &Device, values: &[u32]) -> metal::Buffer {
    device.new_buffer_with_data(
        values.as_ptr().cast(),
        std::mem::size_of_val(values) as u64,
        MTLResourceOptions::StorageModeShared,
    )
}

fn write_f32(buffer: &metal::Buffer, values: &[f32]) {
    let target =
        unsafe { std::slice::from_raw_parts_mut(buffer.contents().cast::<f32>(), values.len()) };
    target.copy_from_slice(values);
}

fn clear_fault(buffer: &metal::Buffer) {
    let words = unsafe { std::slice::from_raw_parts_mut(buffer.contents().cast::<u32>(), 2) };
    words.copy_from_slice(&[0, u32::MAX]);
}

fn read_fault(buffer: &metal::Buffer) -> [u32; 2] {
    let words = unsafe { std::slice::from_raw_parts(buffer.contents().cast::<u32>(), 2) };
    [words[0], words[1]]
}

fn initial_state(instances: usize) -> Vec<f32> {
    let initial = [
        55.0, 25.0, 0.4, 100.0, 0.0, 0.0, 0.0, 100.0, 0.0, 0.0, 0.0, 0.15,
    ];
    let mut state = vec![0.0; COMPONENTS * instances];
    for (component, value) in initial.into_iter().enumerate() {
        state[component * instances..(component + 1) * instances].fill(value);
    }
    state
}

fn input(instances: usize) -> [Vec<f32>; 3] {
    let mut velocity = vec![0.0; instances];
    let mut angular_velocity = vec![0.0; instances];
    let mut bearing = vec![0.0; instances];
    for i in 0..instances {
        let phase = std::f32::consts::TAU * i as f32 / instances as f32;
        velocity[i] = 1.0 + 0.05 * (phase * 3.0).sin();
        angular_velocity[i] = 0.015 * (1.0 + 0.1 * (phase * 2.0).sin());
        bearing[i] = -0.55 + 0.01 * (phase * 7.0).sin() + 0.005 * (phase * 11.0).sin();
    }
    [velocity, angular_velocity, bearing]
}

fn argument<T: std::str::FromStr>(index: usize, default: T) -> T {
    env::args()
        .nth(index)
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn main() {
    let instances = argument(1, 500_000_usize).max(1);
    let turns = argument(2, 40_u32).max(1);
    let mode = env::args().nth(3).unwrap_or_else(|| "checked".to_owned());
    assert!(matches!(mode.as_str(), "checked" | "unchecked"));
    let checked = mode == "checked";
    let initial = initial_state(instances);
    let inputs = input(instances);

    let device = Device::system_default().expect("Metal device unavailable");
    let options = CompileOptions::new();
    options.set_fast_math_enabled(false);
    let library = device
        .new_library_with_source(include_str!("ekf.metal"), &options)
        .expect("Metal kernel compilation failed");
    let function = library
        .get_function(
            if checked {
                "ekf_checked"
            } else {
                "ekf_unchecked"
            },
            None,
        )
        .expect("Metal entry point missing");
    let pipeline = device
        .new_compute_pipeline_state_with_function(&function)
        .expect("Metal pipeline creation failed");
    let queue = device.new_command_queue();
    let state = [
        buffer_from_f32(&device, &initial),
        buffer_from_f32(&device, &initial),
    ];
    let input_buffers = inputs
        .each_ref()
        .map(|values| buffer_from_f32(&device, values));
    let fault = buffer_from_u32(&device, &[0, u32::MAX]);
    let count = buffer_from_u32(&device, &[instances as u32]);

    let dispatch = |source: usize| -> [u32; 2] {
        if checked {
            clear_fault(&fault);
        }
        let command = queue.new_command_buffer();
        let encoder = command.new_compute_command_encoder();
        encoder.set_compute_pipeline_state(&pipeline);
        encoder.set_buffer(0, Some(&state[source]), 0);
        encoder.set_buffer(1, Some(&state[1 - source]), 0);
        for (index, buffer) in input_buffers.iter().enumerate() {
            encoder.set_buffer((index + 2) as u64, Some(buffer), 0);
        }
        if checked {
            encoder.set_buffer(5, Some(&fault), 0);
            encoder.set_buffer(6, Some(&count), 0);
        } else {
            encoder.set_buffer(5, Some(&count), 0);
        }
        encoder.dispatch_threads(
            MTLSize::new(instances as u64, 1, 1),
            MTLSize::new(THREADGROUP_SIZE, 1, 1),
        );
        encoder.end_encoding();
        command.commit();
        command.wait_until_completed();
        if checked {
            read_fault(&fault)
        } else {
            [0, u32::MAX]
        }
    };

    let mut source = 0;
    for _ in 0..WARMUP_TURNS {
        let status = dispatch(source);
        assert_eq!(status[0], 0, "warmup candidate failed: {status:?}");
        source = 1 - source;
    }
    write_f32(&state[0], &initial);
    write_f32(&state[1], &initial);
    source = 0;

    let started = Instant::now();
    for _ in 0..turns {
        let status = dispatch(source);
        assert_eq!(status[0], 0, "timed candidate failed: {status:?}");
        source = 1 - source;
    }
    let elapsed = started.elapsed().as_secs_f64();
    let output = unsafe {
        std::slice::from_raw_parts(
            state[source].contents().cast::<f32>(),
            COMPONENTS * instances,
        )
    };
    let checksum = output.iter().map(|value| *value as f64).sum::<f64>();
    black_box(output);
    let throughput = instances as f64 * turns as f64 / elapsed / 1_000_000.0;

    println!("implementation: Rust host + hand-written MSL");
    println!("mode: {mode}");
    println!("instances: {instances}");
    println!("turns: {turns}");
    println!("threadgroup_size: {THREADGROUP_SIZE}");
    println!("elapsed_s: {elapsed:.9}");
    println!("throughput_million_ekf_turns_per_second: {throughput:.9}");
    println!("checksum: {checksum:.9}");
    println!("faults: 0");
    println!("state_bytes: {}", COMPONENTS * instances * size_of::<f32>());
}
