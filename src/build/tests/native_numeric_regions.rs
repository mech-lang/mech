use std::path::PathBuf;
use std::sync::Arc;

use mech_build::{
    NativeApplicationBuilder, NativeBuildEnvironment, NativeBuildProfile, NativeBuildRequest,
    NativeDependencySource, NativeEmit, NativeHostCatalog, NativeNumericSource,
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
    assert!(
        region
            .live_inputs
            .iter()
            .any(|source| matches!(source, NativeNumericSource::Slot(_)))
    );
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
