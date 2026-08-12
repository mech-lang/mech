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
    assert!(aot.source.contains(".powf("));
}

#[test]
fn standalone_n_body_example_plans_as_aot_from_bytecode() {
    let source = include_str!("../../../examples/aot-n-body/n-body.mec");
    let (builder, mut request) = compile(source);
    request.aot = true;

    let plan = builder.plan(&request).unwrap();

    assert!(plan.aot);
}
