use std::path::PathBuf;

use mech_build::{
    NativeApplicationBuilder, NativeBuildEnvironment, NativeBuildProfile, NativeBuildRequest,
    NativeDependencySource, NativeEmit, NativeHostCatalog,
};
use mech_runtime::RuntimeBuilder;

#[test]
fn canonical_map_artifacts_retain_their_resident_family_in_generated_dependencies() {
    for source in [
        "+> map\nmap/histogram([\"A\" \"C\" \"A\"])\n",
        "+> map\nmap/keys({\"C\": 1u64, \"A\": 2u64})\n",
        "+> map\nmap/values({\"C\": 1u64, \"A\": 2u64})\n",
    ] {
        let mut compiler = RuntimeBuilder::new()
            .function_catalog(mech_stdlib::source_catalog())
            .build_compiler()
            .unwrap();
        let product = compiler.compile_canonical_source(source).unwrap();
        let request = NativeBuildRequest {
            bytecode: product.bytecode().to_vec(),
            instruction_type_bindings: None,
            instruction_type_binding_requirements: None,
            runtime_config: None,
            target: None,
            profile: NativeBuildProfile::Debug,
            binary_name: "canonical-map-features".into(),
            output: PathBuf::from("ignored"),
            emit: NativeEmit::Plan,
            keep_project: false,
            offline: true,
        };
        let plan = NativeApplicationBuilder::new(NativeBuildEnvironment {
            function_catalog: mech_stdlib::source_native_plan_catalog(),
            host_catalog: NativeHostCatalog::new().into(),
            dependency_source: NativeDependencySource::Registry {
                version: mech_build::MECH_COMPONENT_VERSION.into(),
            },
        })
        .plan(&request)
        .unwrap();
        assert!(plan.runtime_functions.is_empty(), "{source}");
        assert!(plan.hosts.is_empty(), "{source}");
        assert!(
            plan.engine_features
                .iter()
                .any(|feature| feature == "map_ops"),
            "{source}: {:?}",
            plan.engine_features
        );
        assert!(
            !plan
                .engine_features
                .iter()
                .any(|feature| feature == "dynamic-modules"),
            "{source}: {:?}",
            plan.engine_features
        );
    }
}
