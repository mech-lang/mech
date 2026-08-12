#![cfg(feature = "full-hosts")]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use mech_build::{
    NativeApplicationBuilder, NativeBuildEnvironment, NativeBuildProfile, NativeBuildRequest,
    NativeDependencySource, NativeEmit, NativeRuntimeConfig, PlannedResourceGrantKey,
    render_generated_native_project, standard_native_host_catalog,
};
use mech_runtime::{ConfigValue, HostInstanceConfig, RunResourceGrantConfig, RuntimeConfig};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .to_path_buf()
}

fn empty_settings() -> ConfigValue {
    ConfigValue::Map(BTreeMap::new())
}

fn host(name: &str, provider: &str, settings: ConfigValue) -> HostInstanceConfig {
    HostInstanceConfig {
        name: name.to_owned(),
        provider: provider.to_owned(),
        settings,
    }
}

fn grant(target: &str, operations: &[&str], paths: &[&str]) -> RunResourceGrantConfig {
    RunResourceGrantConfig {
        target: target.to_owned(),
        operations: operations.iter().map(|value| (*value).to_owned()).collect(),
        paths: paths.iter().map(|value| (*value).to_owned()).collect(),
    }
}

#[test]
fn configured_standard_hosts_are_pruned_to_exact_bytecode_owners() {
    let scene_settings = ConfigValue::Map(BTreeMap::from([
        (
            "renderer".to_owned(),
            ConfigValue::String("canvas".to_owned()),
        ),
        (
            "selector".to_owned(),
            ConfigValue::String("#scene".to_owned()),
        ),
    ]));
    let runtime_config = NativeRuntimeConfig {
        runtime: RuntimeConfig::new("pruned-host-runtime"),
        actor_bootstrap: None,
        hosts: vec![
            host("cli", "cli", empty_settings()),
            host("console", "console", empty_settings()),
            host("arm", "robot-arm", empty_settings()),
            host("scene", "scene", scene_settings),
            host("clock", "time", empty_settings()),
            host("timer", "timer", empty_settings()),
        ],
        run_grants: vec![
            grant("cli/stdout", &["write"], &["*"]),
            grant("console/output", &["read", "write"], &["*"]),
            grant("arm/commands", &["move"], &["*"]),
            grant("scene/frame", &["write"], &["*"]),
            grant("clock/clock", &["read"], &["*"]),
            grant("timer/tick", &["read"], &["*"]),
        ],
    };
    let request = NativeBuildRequest {
        bytecode: std::fs::read(
            workspace_root().join("tests/architecture/bytecode-v1/console.mecb"),
        )
        .unwrap(),
        aot: false,
        runtime_config: Some(runtime_config),
        target: None,
        profile: NativeBuildProfile::Debug,
        binary_name: "pruned_standard_hosts".to_owned(),
        output: PathBuf::from("ignored"),
        emit: NativeEmit::CargoProject,
        keep_project: false,
        offline: true,
    };
    let builder = NativeApplicationBuilder::new(NativeBuildEnvironment {
        function_catalog: mech_stdlib::native_plan_catalog(),
        host_catalog: standard_native_host_catalog().unwrap(),
        dependency_source: NativeDependencySource::Registry {
            version: mech_build::MECH_COMPONENT_VERSION.to_owned(),
        },
    });

    let plan = builder.plan(&request).unwrap();
    assert_eq!(
        plan.hosts
            .iter()
            .map(|host| (host.name.as_str(), host.provider.as_str()))
            .collect::<Vec<_>>(),
        [("console", "console")]
    );
    assert_eq!(
        plan.run_grants,
        [PlannedResourceGrantKey {
            host_instance: "console".to_owned(),
            host_context: "output".to_owned(),
            operation: "write".to_owned(),
            path: "line".to_owned(),
        }]
    );
    assert_eq!(
        plan.packages
            .iter()
            .map(|package| package.package.as_str())
            .collect::<Vec<_>>(),
        ["mech-core", "mech-engine", "mech-console", "mech-runtime"]
    );

    let project =
        render_generated_native_project("ignored-project", &request, &plan, None).unwrap();
    assert!(project.cargo_manifest.contains("mech-console"));
    for unused in [
        "mech-terminal",
        "mech-robot-arm",
        "mech-scene",
        "mech-time",
        "mech-timer",
    ] {
        assert!(!project.cargo_manifest.contains(unused), "linked {unused}");
    }
    let runtime = project.sources.get("src/runtime.rs").unwrap();
    assert!(runtime.contains("NativeConsoleHostFactory::new"));
    assert_eq!(runtime.matches("HostInstanceConfig {").count(), 1);
    assert_eq!(runtime.matches("RunResourceGrantConfig {").count(), 1);
}
