use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    time::Instant,
};

use mech_core::{Body, LegacyValue, MechCode, Program, Ref, Section, SectionElement, hash_str};
use mech_engine::{ArtifactComputeRegion, MechProgram, MechProgramConfig, ProgramArtifact};
use mech_gpu::GpuHost;

const SOURCE: &str = include_str!("../fixtures/ekf-kernel.mec");
const COVARIANCE_SYMMETRY_TOLERANCE: f32 = 1.0e-4;

fn main() {
    let requested_instances = argument(1, 100_000_usize).max(1);
    let cpu_turns = argument(2, 3_u32).max(1);
    let single_gpu_turns = argument(3, 20_u32).max(1);
    let checked_gpu_turns = argument(4, 120_u32).max(1);
    let validation_turns = 4;

    let compile_started = Instant::now();
    let tree = source_tree(requested_instances);
    let driver = evaluate_driver(&tree);
    let (artifact, regions) = compile_artifact(&tree, &driver);
    let inputs = source_inputs(&driver, &artifact);
    let program = GpuHost
        .compile_broadcast_with_regions(&artifact, &regions, &inputs)
        .unwrap_or_else(|error| panic!("generic EKF source must be admitted: {error}"));
    let covariance_slot = program
        .state_shapes()
        .find_map(|(slot, rows, columns)| (rows == 3 && columns == 3).then_some(slot))
        .expect("EKF artifact must contain one 3x3 covariance state");
    let program = program
        .with_robot_state_integrity(covariance_slot, COVARIANCE_SYMMETRY_TOLERANCE)
        .expect("robot-state integrity policy must match the EKF artifact");
    let compile_time = compile_started.elapsed();
    let instances = program.instances() as usize;
    assert_eq!(
        instances, requested_instances,
        "the accelerator must infer the Mech array extent"
    );

    let mut cpu_validation = program.prepare_cpu(&inputs).unwrap();
    cpu_validation.dispatch_turns(validation_turns).unwrap();
    let expected = cpu_validation.state().clone();
    let mut simd_validation = program.prepare_simd_cpu(&inputs).unwrap();
    simd_validation.dispatch_turns(validation_turns).unwrap();
    let simd_max_error = maximum_error(&expected, simd_validation.state());
    assert!(
        simd_max_error <= 1.0e-4,
        "SIMD result differs from scalar CPU lowering by {simd_max_error}"
    );
    let mut jit_validation = program.prepare_jit_cpu(&inputs).unwrap();
    jit_validation.dispatch_turns(validation_turns).unwrap();
    let jit_max_error = maximum_error(&expected, jit_validation.state());
    assert!(
        jit_max_error <= 1.0e-4,
        "JIT result differs from scalar CPU lowering by {jit_max_error}"
    );
    let mut gpu_validation = program.prepare_resident(&inputs).unwrap();
    let actual = gpu_validation.run_turns(validation_turns).unwrap();
    let max_error = maximum_error(&expected, &actual.state);
    assert!(
        max_error <= 1.0e-4,
        "GPU result differs from generic CPU lowering by {max_error}"
    );

    let mut cpu_warmup = program.prepare_cpu(&inputs).unwrap();
    cpu_warmup.dispatch_turns(5).unwrap();
    let mut cpu = program.prepare_cpu(&inputs).unwrap();
    let cpu_started = Instant::now();
    cpu.dispatch_turns(cpu_turns).unwrap();
    let cpu_per_turn = cpu_started.elapsed() / cpu_turns;
    let cpu_checksum = state_checksum(cpu.state());

    let mut simd_warmup = program.prepare_simd_cpu(&inputs).unwrap();
    simd_warmup.dispatch_turns(5).unwrap();
    let mut simd = program.prepare_simd_cpu(&inputs).unwrap();
    let simd_started = Instant::now();
    simd.dispatch_turns(cpu_turns).unwrap();
    let simd_per_turn = simd_started.elapsed() / cpu_turns;
    let simd_checksum = state_checksum(simd.state());

    let mut jit_warmup = program.prepare_jit_cpu(&inputs).unwrap();
    jit_warmup.dispatch_turns(5).unwrap();
    let jit_prepare_started = Instant::now();
    let mut jit = program.prepare_jit_cpu(&inputs).unwrap();
    let jit_prepare = jit_prepare_started.elapsed();
    let jit_started = Instant::now();
    jit.dispatch_turns(cpu_turns).unwrap();
    let jit_per_turn = jit_started.elapsed() / cpu_turns;
    let jit_checksum = state_checksum(jit.state());

    let prepare_started = Instant::now();
    let mut gpu = program.prepare_resident(&inputs).unwrap();
    let resident_prepare = prepare_started.elapsed();
    gpu.dispatch_turns(1).unwrap();
    let single_started = Instant::now();
    for _ in 0..single_gpu_turns {
        gpu.dispatch_turns(1).unwrap();
    }
    let single_per_turn = single_started.elapsed() / single_gpu_turns;
    let checked_repeated = gpu.dispatch_turns(checked_gpu_turns).unwrap() / checked_gpu_turns;
    let (readback, state) = gpu.read_state().unwrap();
    std::hint::black_box(state);

    println!("EKF instances: {instances}");
    println!("batch extent authority: Mech input arrays");
    println!("source artifact nodes: {}", artifact.nodes().len());
    println!("generated WGSL bytes: {}", program.wgsl().len());
    println!("GPU workgroups: {}", program.workgroup_count());
    println!("CPU SIMD width: {} f32 lanes", program.simd_lanes());
    println!(
        "robot integrity: all state finite; covariance diagonal > 0; symmetry tolerance {:.1e}",
        COVARIANCE_SYMMETRY_TOLERANCE
    );
    println!(
        "integrity failure: reject candidate, retain previous published estimate, record latest fault + count"
    );
    println!("adapter: {}", actual.adapter);
    println!(
        "source + artifact + scalarization: {:.3} ms",
        millis(compile_time)
    );
    println!("resident GPU prepare: {:.3} ms", millis(resident_prepare));
    println!("Cranelift JIT prepare: {:.3} ms", millis(jit_prepare));
    println!(
        "Mech scalar CPU: {:.3} ms/turn ({cpu_turns} turns)",
        millis(cpu_per_turn)
    );
    println!(
        "Mech SIMD CPU: {:.3} ms/turn ({cpu_turns} turns)",
        millis(simd_per_turn)
    );
    println!(
        "Mech Cranelift JIT CPU: {:.3} ms/turn ({cpu_turns} turns)",
        millis(jit_per_turn)
    );
    println!(
        "resident GPU, one submission per turn: {:.3} ms/turn ({single_gpu_turns} turns)",
        millis(single_per_turn)
    );
    println!(
        "resident GPU, checked repeated turns: {:.3} ms/turn ({checked_gpu_turns} turns)",
        millis(checked_repeated)
    );
    println!(
        "Mech scalar throughput: {:.3} million EKF-turns/s",
        throughput(instances, cpu_per_turn)
    );
    println!(
        "Mech SIMD throughput: {:.3} million EKF-turns/s",
        throughput(instances, simd_per_turn)
    );
    println!(
        "Mech Cranelift JIT throughput: {:.3} million EKF-turns/s",
        throughput(instances, jit_per_turn)
    );
    println!(
        "GPU single-submit throughput: {:.3} million EKF-turns/s",
        throughput(instances, single_per_turn)
    );
    println!(
        "GPU checked repeated throughput: {:.3} million EKF-turns/s",
        throughput(instances, checked_repeated)
    );
    println!("final state readback: {:.3} ms", millis(readback));
    println!("maximum CPU/GPU absolute error: {max_error:.3e}");
    println!("maximum scalar/SIMD absolute error: {simd_max_error:.3e}");
    println!("maximum scalar/JIT absolute error: {jit_max_error:.3e}");
    println!("Mech scalar checksum: {cpu_checksum:.9}");
    println!("Mech SIMD checksum: {simd_checksum:.9}");
    println!("Mech Cranelift JIT checksum: {jit_checksum:.9}");
}

fn argument<T: std::str::FromStr>(index: usize, default: T) -> T {
    env::args()
        .nth(index)
        .map(|argument| {
            argument
                .parse()
                .ok()
                .expect("benchmark argument must parse")
        })
        .unwrap_or(default)
}

fn source_tree(instances: usize) -> Program {
    let source = SOURCE.replacen("100000f32", &format!("{instances}f32"), 1);
    mech_syntax::parse(&source).expect("EKF array source must parse")
}

fn evaluate_driver(tree: &Program) -> MechProgram {
    let mut program = MechProgram::with_function_catalog(
        MechProgramConfig::default(),
        mech_stdlib::source_native_plan_catalog(),
    );
    program
        .run_tree(&projected_tree(tree, false))
        .expect("Mech EKF array inputs must evaluate");
    program
}

fn compile_artifact(
    tree: &Program,
    driver: &MechProgram,
) -> (ProgramArtifact, Box<[ArtifactComputeRegion]>) {
    let mut program = MechProgram::with_function_catalog(
        MechProgramConfig::default(),
        mech_stdlib::source_native_plan_catalog(),
    );
    seed_lane_inputs(&program, driver, tree);
    program
        .run_tree(&projected_tree(tree, true))
        .expect("single-EKF compute region must run");
    program
        .compile_program_artifact_with_regions()
        .expect("EKF source must compile to a typed artifact")
}

fn seed_lane_inputs(target: &MechProgram, driver: &MechProgram, tree: &Program) {
    let compute_definitions = tree
        .body
        .sections
        .iter()
        .filter(|section| section.compute.is_some())
        .flat_map(|section| &section.elements)
        .filter_map(|element| match element {
            SectionElement::MechCode(code) => Some(code),
            _ => None,
        })
        .flatten()
        .filter_map(|(code, _)| match code {
            MechCode::Statement(mech_core::Statement::VariableDefine(definition)) => {
                Some(definition.var.name.hash())
            }
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let driver_symbols = driver.interpreter().symbols();
    let lane_values = {
        let symbols = driver_symbols.borrow();
        let dictionary = symbols.dictionary.borrow();
        symbols
            .symbols
            .iter()
            .filter(|(id, _)| !compute_definitions.contains(id))
            .filter_map(|(id, cell)| {
                let values = cell.borrow().as_vecf32().ok()?;
                Some((*id, dictionary.get(id)?.clone(), *values.first()?))
            })
            .collect::<Vec<_>>()
    };
    let target_symbols = target.interpreter().symbols();
    for (id, name, value) in lane_values {
        target_symbols
            .borrow_mut()
            .insert(id, LegacyValue::F32(Ref::new(value)), false);
        target_symbols
            .borrow()
            .dictionary
            .borrow_mut()
            .insert(id, name);
    }
}

fn source_inputs(program: &MechProgram, artifact: &ProgramArtifact) -> BTreeMap<String, Vec<f32>> {
    let symbols = program.interpreter().symbols();
    let symbols = symbols.borrow();
    artifact
        .inputs()
        .iter()
        .filter_map(|input| {
            symbols.get(hash_str(&input.name)).map(|cell| {
                let values = cell.borrow().as_vecf32().unwrap_or_else(|error| {
                    panic!("Mech array input `{}` must be f32: {error:?}", input.name)
                });
                (input.name.clone(), values)
            })
        })
        .collect()
}

fn projected_tree(tree: &Program, compute: bool) -> Program {
    let imports = tree
        .body
        .sections
        .iter()
        .flat_map(|section| &section.elements)
        .filter_map(|element| {
            let SectionElement::MechCode(code) = element else {
                return None;
            };
            let imports = code
                .iter()
                .filter(|(code, _)| matches!(code, MechCode::Import(_)))
                .cloned()
                .collect::<Vec<_>>();
            (!imports.is_empty()).then_some(SectionElement::MechCode(imports))
        })
        .collect::<Vec<_>>();
    let selected = tree
        .body
        .sections
        .iter()
        .find(|section| section.compute.is_some() == compute)
        .unwrap_or_else(|| panic!("EKF source must contain the selected section"))
        .clone();
    Program {
        title: tree.title.clone(),
        body: Body {
            sections: vec![
                Section {
                    subtitle: None,
                    compute: None,
                    elements: imports,
                },
                selected,
            ],
        },
    }
}

fn maximum_error(
    expected: &BTreeMap<mech_core::CellSlotId, Vec<f32>>,
    actual: &BTreeMap<mech_core::CellSlotId, Vec<f32>>,
) -> f32 {
    expected
        .iter()
        .flat_map(|(slot, expected)| {
            expected
                .iter()
                .zip(&actual[slot])
                .map(|(left, right)| (left - right).abs())
        })
        .fold(0.0_f32, f32::max)
}

fn state_checksum(state: &BTreeMap<mech_core::CellSlotId, Vec<f32>>) -> f64 {
    state
        .values()
        .flatten()
        .map(|value| f64::from(*value))
        .sum()
}

fn millis(duration: std::time::Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}

fn throughput(instances: usize, duration: std::time::Duration) -> f64 {
    instances as f64 / duration.as_secs_f64() / 1_000_000.0
}
