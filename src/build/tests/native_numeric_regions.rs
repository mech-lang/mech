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
        r#"~state := 2.0
next := state ^ 2.0
state = next
state"#,
    );

    let analysis = builder.analyze_numeric_regions(&request).unwrap();
    assert_eq!(analysis.rejections.len(), 1);
    assert!(analysis.rejections[0].operation.contains("Pow"));
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
        r#"~state := 2.0
before := state + 1.0
fallback := before ^ 2.0
after := before + fallback
state = after
state"#,
    );

    let analysis = builder.analyze_numeric_regions(&request).unwrap();
    assert_eq!(analysis.rejections.len(), 1);
    assert!(analysis.rejections[0].operation.contains("Pow"));
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
