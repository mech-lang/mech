use std::path::PathBuf;
use std::sync::Arc;

use mech_build::{
    NativeApplicationBuilder, NativeBuildEnvironment, NativeBuildProfile, NativeBuildRequest,
    NativeDependencySource, NativeEmit, NativeHostCatalog, NativeNumericOpcode,
    NativeNumericSlotStorage,
};
use mech_runtime::RuntimeBuilder;

fn compile(source: &str) -> (NativeApplicationBuilder, NativeBuildRequest) {
    let catalog = mech_stdlib::source_catalog();
    let mut runtime = RuntimeBuilder::new()
        .planning()
        .function_catalog(catalog.clone())
        .build()
        .unwrap();
    runtime.run_string(source).unwrap();
    let bytecode = runtime.compile_program_bytecode().unwrap();
    let builder = NativeApplicationBuilder::new(NativeBuildEnvironment {
        function_catalog: catalog,
        host_catalog: Arc::new(NativeHostCatalog::new()),
        dependency_source: NativeDependencySource::Registry {
            version: "0.3.5".to_owned(),
        },
    });
    let request = NativeBuildRequest {
        bytecode,
        aot: false,
        runtime_config: None,
        target: None,
        profile: NativeBuildProfile::Release,
        binary_name: "numeric_region_test".to_owned(),
        output: PathBuf::from("ignored"),
        emit: NativeEmit::Plan,
        keep_project: false,
        offline: true,
    };
    (builder, request)
}

#[test]
fn build_analysis_finds_one_native_region_for_a_pure_recurrence() {
    let (builder, request) = compile(
        r#"~state := 1.0
next := state * 0.9 + 0.1
state = next
state"#,
    );

    let analysis = builder.analyze_numeric_regions(&request).unwrap();
    assert!(analysis.rejections.is_empty(), "{:?}", analysis.rejections);
    assert_eq!(analysis.regions.len(), 1);
    let region = &analysis.regions[0];
    assert!(region.nodes.len() >= 3);
    assert_eq!(region.instructions.len(), region.nodes.len());
    assert_eq!(
        region
            .instructions
            .iter()
            .map(|instruction| instruction.opcode)
            .collect::<Vec<_>>(),
        [
            NativeNumericOpcode::Multiply,
            NativeNumericOpcode::Add,
            NativeNumericOpcode::Assign,
        ],
    );
    assert!(
        !region.live_inputs.is_empty(),
        "state must be an explicit region live-in"
    );
    assert_eq!(region.constants.len(), 2);
    assert!(
        region
            .constants
            .iter()
            .all(|constant| constant.shape.len() == Some(constant.elements.len()))
    );
    assert!(region.slots.iter().any(|slot| {
        region.live_inputs.contains(&slot.slot) && slot.storage == NativeNumericSlotStorage::State
    }));
    assert_eq!(region.live_outputs.len(), 1);
}

#[test]
fn build_analysis_keeps_an_unsupported_operation_as_a_fallback_boundary() {
    let (builder, request) = compile(
        r#"~state := [2.0 3.0; 4.0 5.0]
state = state[:,[1,2]]
state"#,
    );

    let analysis = builder.analyze_numeric_regions(&request).unwrap();
    assert_eq!(analysis.rejections.len(), 1);
    assert!(analysis.rejections[0].operation.contains("Access2D"));
    assert!(
        analysis.rejections[0]
            .reason
            .contains("no native numeric opcode")
    );
    assert_eq!(analysis.regions.len(), 1);
    assert_eq!(analysis.regions[0].nodes.len(), 1);
}

#[test]
fn build_analysis_never_fuses_across_a_fallback_dependency() {
    let (builder, request) = compile(
        r#"~state := [2.0 3.0; 4.0 5.0]
before := state + [1.0 1.0; 1.0 1.0]
fallback := before[:,[1,2]]
after := before + fallback
state = after
state"#,
    );

    let analysis = builder.analyze_numeric_regions(&request).unwrap();
    assert_eq!(analysis.rejections.len(), 1);
    assert!(analysis.rejections[0].operation.contains("Access2D"));
    assert_eq!(analysis.regions.len(), 2, "{analysis:#?}");
    assert_eq!(analysis.regions[0].instructions.len(), 1);
    assert_eq!(
        analysis.regions[0].instructions[0].opcode,
        NativeNumericOpcode::Add,
    );
    assert_eq!(analysis.regions[1].instructions.len(), 2);
    assert_eq!(
        analysis.regions[1]
            .instructions
            .iter()
            .map(|instruction| instruction.opcode)
            .collect::<Vec<_>>(),
        [NativeNumericOpcode::Add, NativeNumericOpcode::Assign],
    );
    let fallback_output = analysis.regions[1]
        .instructions
        .first()
        .and_then(|instruction| instruction.inputs.get(1))
        .and_then(|source| match source {
            mech_build::NativeNumericSource::Slot(slot) => Some(*slot),
            mech_build::NativeNumericSource::Constant(_) => None,
        })
        .unwrap();
    assert!(analysis.regions[1].live_inputs.contains(&fallback_output));
}

#[test]
fn aot_is_all_or_nothing_while_the_ordinary_build_keeps_its_fallback() {
    let (builder, request) = compile(
        r#"~state := [2.0 3.0; 4.0 5.0]
state = state[:,[1,2]]
state"#,
    );

    let plan = builder.plan(&request).unwrap();
    assert!(!plan.aot);

    let mut aot_request = request;
    aot_request.aot = true;
    let error = builder.plan(&aot_request).unwrap_err();
    assert!(
        error
            .display_message()
            .contains("AOT numeric lowering rejected the program"),
        "{error:?}",
    );
}

#[test]
fn build_analysis_lowers_the_canonical_n_body_turn_as_one_region() {
    let source =
        include_str!("../../../tests/architecture/resident-activation/n-body-source-v1.mec");
    let catalog = mech_stdlib::source_catalog();
    let mut program = mech_engine::MechProgram::with_function_catalog(
        mech_engine::MechProgramConfig::default(),
        catalog.clone(),
    );
    program.run_string(source).unwrap();
    let (artifact, _) = program.compile_program_product().unwrap().into_parts();
    let instance = mech_engine::__resident::activate(
        mech_core::ReactiveInstanceId::new(0, 0),
        &artifact,
        &catalog,
        &mech_engine::__resident::ActivationFacts::default(),
    )
    .unwrap();
    let analysis = mech_build::analyze_activated_artifact(&artifact, &instance.plan);
    assert!(analysis.rejections.is_empty(), "{:#?}", analysis.rejections);
    assert_eq!(analysis.regions.len(), 1, "{analysis:#?}");
    let opcodes = analysis.regions[0]
        .instructions
        .iter()
        .map(|instruction| instruction.opcode)
        .collect::<Vec<_>>();
    assert!(opcodes.contains(&NativeNumericOpcode::RowsAllColumns));
    assert!(opcodes.contains(&NativeNumericOpcode::SumColumns));
    assert!(opcodes.contains(&NativeNumericOpcode::Power));
    assert!(opcodes.contains(&NativeNumericOpcode::MultiplyRows));
    assert!(opcodes.contains(&NativeNumericOpcode::SubtractIndexedRows));
    assert!(opcodes.contains(&NativeNumericOpcode::AddIndexedRows));
    assert!(opcodes.contains(&NativeNumericOpcode::AddAssign));
}

#[test]
fn canonical_n_body_lowers_to_standalone_rust_source() {
    let source =
        include_str!("../../../tests/architecture/resident-activation/n-body-source-v1.mec");
    let catalog = mech_stdlib::source_catalog();
    let mut program = mech_engine::MechProgram::with_function_catalog(
        mech_engine::MechProgramConfig::default(),
        catalog.clone(),
    );
    program.run_string(source).unwrap();
    let (_, bytecode) = program.compile_program_product().unwrap().into_parts();
    let aot = mech_build::aot::lower_bytecode(&bytecode, &catalog).unwrap();
    assert_eq!(aot.input_len, 0);
    assert!(aot.state_len > 0);
    assert!(aot.instruction_count > 0);
    assert!(aot.source.contains("pub fn turn_in_place"));
    let (strict, fast) = aot.source.split_once("pub fn turn_in_place").unwrap();
    assert!(strict.contains(".powf("));
    assert!(!fast.contains(".powf("));
    assert!(fast.contains(".sqrt()"));
}

#[test]
fn standalone_n_body_lowers_to_c_callable_mlir() {
    let source = include_str!("../../../examples/aot-n-body/n-body.mec");
    let catalog = mech_stdlib::source_catalog();
    let mut program = mech_engine::MechProgram::with_function_catalog(
        mech_engine::MechProgramConfig::default(),
        catalog.clone(),
    );
    program.run_string(source).unwrap();
    let (_, bytecode) = program.compile_program_product().unwrap().into_parts();

    let mlir = mech_build::aot::lower_bytecode_mlir(&bytecode, &catalog).unwrap();

    assert_eq!(mlir.input_len, 0);
    assert_eq!(mlir.state_len, 30);
    assert!(mlir.instruction_count > 0);
    assert!(mlir.source.contains("func.func @mech_initialize"));
    assert!(mlir.source.contains("func.func @mech_run_fast"));
    assert!(mlir.source.contains("llvm.emit_c_interface"));
    assert!(mlir.source.contains("math.sqrt"));
    assert!(!mlir.source.contains("math.powf"));
}

#[test]
fn materialized_particle_lanes_lower_to_gpu_mlir() {
    let source = r#"~position := [1.0 1.0 1.0 1.0 1.0 1.0 1.0 1.0]
~velocity := [0.5 0.5 0.5 0.5 0.5 0.5 0.5 0.5]
dt := 0.25
acceleration := position * -0.125
next-velocity := velocity + acceleration * dt
velocity = next-velocity
position = position + next-velocity * dt
position"#;
    let catalog = mech_stdlib::source_catalog();
    let mut program = mech_engine::MechProgram::with_function_catalog(
        mech_engine::MechProgramConfig::default(),
        catalog.clone(),
    );
    program.run_string(source).unwrap();
    let (_, bytecode) = program.compile_program_product().unwrap().into_parts();

    let mlir = mech_build::aot::lower_bytecode_mlir_gpu(&bytecode, &catalog).unwrap();

    assert_eq!(mlir.input_len, 0);
    assert_eq!(mlir.state_len, 16);
    assert!(mlir.instruction_count > 0);
    assert!(
        mlir.source
            .contains("module attributes {gpu.container_module}")
    );
    assert!(mlir.source.contains("gpu.func @mech_turn"));
    assert!(mlir.source.contains("gpu.thread_id x"));
    assert!(
        mlir.source
            .contains("gpu.launch_func @mech_kernels::@mech_turn")
    );
    assert!(mlir.source.contains("memref<16xf64>"));
    assert!(mlir.source.contains("arith.mulf"));
}

#[test]
fn gpu_mlir_rejection_identifies_an_unsupported_operation() {
    let source = r#"+> math/*

~position := [1.0 1.0 1.0 1.0]
next := sin(position)
position = next
position"#;
    let catalog = mech_stdlib::source_catalog();
    let mut program = mech_engine::MechProgram::with_function_catalog(
        mech_engine::MechProgramConfig::default(),
        catalog.clone(),
    );
    program.run_string(source).unwrap();
    let (_, bytecode) = program.compile_program_product().unwrap().into_parts();

    let error = mech_build::aot::lower_bytecode_mlir_gpu(&bytecode, &catalog).unwrap_err();

    assert!(error.contains("unsupported operation"), "{error}");
    assert!(error.contains("Sin"), "{error}");
    assert!(error.contains("node"), "{error}");
}

#[test]
fn materialized_particle_lanes_lower_to_relaxed_f32_gpu_mlir() {
    let source = r#"~position := [1.0 1.0 1.0 1.0]
~velocity := [0.5 0.5 0.5 0.5]
next-velocity := velocity + position * -0.03125
velocity = next-velocity
position = position + next-velocity * 0.25
position"#;
    let catalog = mech_stdlib::source_catalog();
    let mut program = mech_engine::MechProgram::with_function_catalog(
        mech_engine::MechProgramConfig::default(),
        catalog.clone(),
    );
    program.run_string(source).unwrap();
    let (_, bytecode) = program.compile_program_product().unwrap().into_parts();

    let mlir = mech_build::aot::lower_bytecode_mlir_gpu_f32(&bytecode, &catalog).unwrap();

    assert_eq!(mlir.state_len, 8);
    assert!(mlir.source.contains("memref<8xf32>"));
    assert!(mlir.source.contains("0x3F000000 : f32"));
    assert!(mlir.source.contains("spirv.entry_point_abi"));
    assert!(!mlir.source.contains("memref<8xf64>"));
}

#[test]
fn materialized_particle_lanes_lower_to_f32_spirv_mlir() {
    let source = r#"~position := [1.0 1.0 1.0 1.0]
~velocity := [0.5 0.5 0.5 0.5]
next-velocity := velocity + position * -0.03125
velocity = next-velocity
position = position + next-velocity * 0.25
position"#;
    let catalog = mech_stdlib::source_catalog();
    let mut program = mech_engine::MechProgram::with_function_catalog(
        mech_engine::MechProgramConfig::default(),
        catalog.clone(),
    );
    program.run_string(source).unwrap();
    let (_, bytecode) = program.compile_program_product().unwrap().into_parts();

    let mlir = mech_build::aot::lower_bytecode_mlir_spirv_f32(&bytecode, &catalog).unwrap();

    assert_eq!(mlir.state_len, 8);
    assert!(mlir.source.contains("spirv.module @mech_kernels"));
    assert!(mlir.source.contains("@mech_state bind(0, 0)"));
    assert!(mlir.source.contains("spirv.func @mech_initialize"));
    assert!(mlir.source.contains("spirv.func @mech_turn"));
    assert!(mlir.source.contains("spirv.FMul"));
    assert!(mlir.source.contains("!spirv.array<8 x f32"));
    assert!(!mlir.source.contains(": f64"));
    assert!(!mlir.source.contains("x f64"));
}

#[test]
fn shaped_particle_state_finalizes_for_aot_lowering() {
    let source = r#"~position<[f64]:1,1024> := 0.25
~velocity<[f64]:1,1024> := 0.5
next-velocity := velocity + position * -0.03125
velocity = next-velocity
position = position + next-velocity * 0.25
position"#;
    let catalog = mech_stdlib::source_catalog();
    let mut program = mech_engine::MechProgram::with_function_catalog(
        mech_engine::MechProgramConfig::default(),
        catalog.clone(),
    );
    program.run_string(source).unwrap();
    let (_, bytecode) = program.compile_program_product().unwrap().into_parts();

    let mlir = mech_build::aot::lower_bytecode_mlir_spirv_f32(&bytecode, &catalog).unwrap();
    let rust = mech_build::aot::lower_bytecode(&bytecode, &catalog).unwrap();
    let rust_f32 = mech_build::aot::lower_bytecode_rust_f32(&bytecode, &catalog).unwrap();

    assert_eq!(mlir.state_len, 2048);
    assert!(mlir.source.contains("// mech.batch_len = 1024"));
    assert!(mlir.source.contains("!spirv.array<2048 x f32"));
    assert!(rust.source.contains("state[0..1024].fill("));
    assert!(!rust.source.contains("[f64; 1024]"));
    assert!(rust.source.len() < 100_000, "{}", rust.source.len());
    assert!(rust_f32.source.contains("state: &mut [f32]"));
    assert!(rust_f32.source.contains("f32::from_bits("));
    assert!(!rust_f32.source.contains("state: &mut [f64]"));
}

#[test]
fn host_initialized_metal_accepts_nonuniform_particles_and_scalar_controls() {
    let source = r#"+> math/*
lane := 0..=63
angle := lane * 0.17
radius := 0.1 + lane / 63 * 0.8
~x := cos(angle) * radius
~y := sin(angle) * radius
~vx<[f64]:1,64> := 0.0
~vy<[f64]:1,64> := 0.0
~pointer-x := 0.0
~pointer-y := 0.0
~pointer-down := 0.0
~dt := 0.008
pointer-x = pointer-x
pointer-y = pointer-y
pointer-down = pointer-down
dt = dt
dx := pointer-x - x
dy := pointer-y - y
distance2 := dx * dx + dy * dy + 0.02
force := pointer-down * 0.0005 / distance2
next-vx := (vx + dx * force * dt) * 0.999
next-vy := (vy + dy * force * dt) * 0.999
vx = next-vx
vy = next-vy
x = x + next-vx * dt
y = y + next-vy * dt
x"#;
    let catalog = mech_stdlib::source_catalog();
    let mut program = mech_engine::MechProgram::with_function_catalog(
        mech_engine::MechProgramConfig::default(),
        catalog.clone(),
    );
    program.run_string(source).unwrap();
    let (_, bytecode) = program.compile_program_product().unwrap().into_parts();

    let device_error =
        mech_build::aot::lower_bytecode_mlir_spirv_f32(&bytecode, &catalog).unwrap_err();
    assert!(
        device_error.contains("host-initialized Metal target"),
        "{device_error}"
    );

    let mlir = mech_build::aot::lower_bytecode_mlir_spirv_f32_host_initialized(&bytecode, &catalog)
        .unwrap();
    assert_eq!(mlir.state_len, 260);
    assert!(mlir.source.contains("// mech.batch_len = 64"));
    assert!(mlir.source.contains("// mech.initialization = host"));
    assert!(mlir.source.contains("// mech.lane_state_offsets = "));
    assert!(mlir.source.contains("// mech.scalar_state_offsets = "));
    assert!(!mlir.source.contains("spirv.func @mech_initialize"));
    assert!(mlir.source.contains("spirv.func @mech_turn"));

    let rust = mech_build::aot::lower_bytecode_rust_f32(&bytecode, &catalog).unwrap();
    assert!(rust.source.contains("pub fn initialize(state: &mut [f32])"));

    let initial = mech_build::aot::lower_bytecode_initial_state_f32(&bytecode, &catalog).unwrap();
    assert_eq!(initial.len(), 260 * std::mem::size_of::<f32>());
    let value = |offset: usize| {
        f32::from_le_bytes(
            initial[offset * std::mem::size_of::<f32>()..(offset + 1) * std::mem::size_of::<f32>()]
                .try_into()
                .unwrap(),
        )
    };
    assert_eq!(value(256), 0.0);
    assert_eq!(value(257), 0.0);
    assert_eq!(value(258), 0.0);
    assert_eq!(value(259), 0.008_f32);
}

#[test]
fn standalone_n_body_example_plans_as_aot_from_bytecode() {
    let source = include_str!("../../../examples/aot-n-body/n-body.mec");
    let (builder, mut request) = compile(source);
    request.aot = true;

    let plan = builder.plan(&request).unwrap();

    assert!(plan.aot);
}
