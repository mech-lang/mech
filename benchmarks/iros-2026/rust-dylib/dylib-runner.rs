//! Minimal common loader used to compare Mech AOT and Rust dynamic libraries.

use std::{
    env,
    ffi::{CStr, CString, c_char, c_int, c_void},
    hint::black_box,
    path::Path,
    time::Instant,
};

type NativeTurn =
    unsafe extern "C" fn(*const *const f32, *const *const f32, *const *mut f32, usize) -> u64;

const WARMUP_TURNS: u32 = 100;

#[cfg(target_os = "macos")]
const RTLD_NOW: c_int = 0x2;
#[cfg(all(unix, not(target_os = "macos")))]
const RTLD_NOW: c_int = 0x2;

unsafe extern "C" {
    fn dlopen(path: *const c_char, mode: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    fn dlclose(handle: *mut c_void) -> c_int;
    fn dlerror() -> *const c_char;
}

struct Library {
    handle: *mut c_void,
    turn: NativeTurn,
}

impl Library {
    fn open(path: &Path) -> Self {
        let path = CString::new(path.as_os_str().to_string_lossy().as_bytes()).unwrap();
        let handle = unsafe { dlopen(path.as_ptr(), RTLD_NOW) };
        if handle.is_null() {
            panic!("dlopen failed: {}", dynamic_error());
        }
        let symbol = b"mech_fixed_numeric_turn\0";
        let address = unsafe { dlsym(handle, symbol.as_ptr().cast()) };
        if address.is_null() {
            unsafe { dlclose(handle) };
            panic!("dlsym failed: {}", dynamic_error());
        }
        let turn = unsafe { std::mem::transmute::<*mut c_void, NativeTurn>(address) };
        Self { handle, turn }
    }
}

impl Drop for Library {
    fn drop(&mut self) {
        unsafe { dlclose(self.handle) };
    }
}

fn dynamic_error() -> String {
    let error = unsafe { dlerror() };
    if error.is_null() {
        "unknown dynamic-loader error".to_owned()
    } else {
        unsafe { CStr::from_ptr(error) }
            .to_string_lossy()
            .into_owned()
    }
}

struct Workload {
    inputs: [Vec<f32>; 5],
    state: [Vec<f32>; 2],
    next_state: [Vec<f32>; 2],
}

impl Workload {
    fn new(instances: usize) -> Self {
        let denominator = instances as f32;
        let mut linear_velocity = vec![0.0; instances];
        let mut angular_velocity = vec![0.0; instances];
        let mut bearing = vec![0.0; instances];
        for instance in 0..instances {
            let phase = std::f32::consts::TAU * instance as f32 / denominator;
            linear_velocity[instance] = 1.0 + 0.05 * (phase * 3.0).sin();
            angular_velocity[instance] = 0.015 * (1.0 + 0.1 * (phase * 2.0).sin());
            bearing[instance] = -0.55 + 0.01 * (phase * 7.0).sin() + 0.005 * (phase * 11.0).sin();
        }
        let inputs = [
            vec![0.1; instances],
            linear_velocity,
            angular_velocity,
            bearing,
            vec![0.25; instances],
        ];
        let mut workload = Self {
            inputs,
            state: [vec![0.0; instances * 3], vec![0.0; instances * 9]],
            next_state: [vec![0.0; instances * 3], vec![0.0; instances * 9]],
        };
        workload.reset();
        workload
    }

    fn reset(&mut self) {
        for instance in 0..self.state[0].len() / 3 {
            self.state[0][instance * 3..instance * 3 + 3].copy_from_slice(&[55.0, 25.0, 0.4]);
            self.state[1][instance * 9..instance * 9 + 9]
                .copy_from_slice(&[100.0, 0.0, 0.0, 0.0, 100.0, 0.0, 0.0, 0.0, 0.15]);
        }
        self.next_state[0].fill(0.0);
        self.next_state[1].fill(0.0);
    }

    fn dispatch(&mut self, turn: NativeTurn, turns: u32) -> u64 {
        let input_pointers = self.inputs.each_ref().map(|values| values.as_ptr());
        for _ in 0..turns {
            let state_pointers = self.state.each_ref().map(|values| values.as_ptr());
            let next_state_pointers = self.next_state.each_mut().map(|values| values.as_mut_ptr());
            let fault = unsafe {
                turn(
                    input_pointers.as_ptr(),
                    state_pointers.as_ptr(),
                    next_state_pointers.as_ptr(),
                    self.inputs[0].len(),
                )
            };
            if fault != 0 {
                return fault;
            }
            std::mem::swap(&mut self.state, &mut self.next_state);
        }
        0
    }

    fn checksum(&self) -> f64 {
        self.state
            .iter()
            .flat_map(|values| values.iter())
            .map(|value| *value as f64)
            .sum()
    }

    fn maximum_error(&self, other: &Self) -> f32 {
        self.state
            .iter()
            .flatten()
            .zip(other.state.iter().flatten())
            .map(|(left, right)| (left - right).abs())
            .fold(0.0, f32::max)
    }
}

fn argument<T: std::str::FromStr>(index: usize, default: T) -> T {
    env::args()
        .nth(index)
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn main() {
    let path = env::args()
        .nth(1)
        .expect("usage: dylib-runner LIBRARY [INSTANCES] [TURNS] [REFERENCE]");
    let instances = argument(2, 10_000_usize).max(1);
    let turns = argument(3, 200_u32).max(1);
    let library = Library::open(Path::new(&path));
    let mut workload = Workload::new(instances);
    let warmup_fault = workload.dispatch(library.turn, WARMUP_TURNS);
    assert_eq!(warmup_fault, 0, "warmup must not reject a candidate");
    workload.reset();
    let started = Instant::now();
    let fault = workload.dispatch(library.turn, turns);
    let elapsed = started.elapsed().as_secs_f64();
    assert_eq!(fault, 0, "timed run must not reject a candidate");
    let throughput = instances as f64 * turns as f64 / elapsed / 1_000_000.0;
    let checksum = workload.checksum();
    black_box(&workload);

    println!("library: {path}");
    println!("instances: {instances}");
    println!("turns: {turns}");
    println!("elapsed_s: {elapsed:.9}");
    println!("throughput_million_ekf_turns_per_second: {throughput:.6}");
    println!("checksum: {checksum:.9}");
    println!("faults: {}", u64::from(fault != 0));

    if let Some(reference_path) = env::args().nth(4) {
        let reference = Library::open(Path::new(&reference_path));
        let mut reference_workload = Workload::new(instances);
        let reference_fault = reference_workload.dispatch(reference.turn, turns);
        assert_eq!(reference_fault, 0, "reference must not reject a candidate");
        println!(
            "maximum_reference_absolute_error: {:.9e}",
            workload.maximum_error(&reference_workload)
        );
    }
}
