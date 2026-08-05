#![cfg(feature = "standard-hosts")]

use std::collections::BTreeMap;
use std::path::PathBuf;

use mech_build::{
    NativeApplicationBuilder, NativeBuildEnvironment, NativeBuildProfile, NativeBuildRequest,
    NativeDependencySource, NativeEmit, NativeRuntimeConfig, standard_native_host_catalog,
    standard_planning_host_factory,
};
use mech_core::{
    ApplicationRequirement, BytecodeInstruction, ParsedProgram, Ref, ResourceIntent, Value,
};
use mech_runtime::{
    ConfigValue, HostInstanceConfig, PlannedPureHostFunction, RunResourceGrantConfig,
    RuntimeBuilder, RuntimeConfig, RuntimeValueSnapshot,
};

struct ProviderCase {
    provider: &'static str,
    instance: &'static str,
    target: &'static str,
    operations: &'static [&'static str],
    paths: &'static [&'static str],
    source: &'static str,
    base_uri: &'static str,
    path: &'static str,
    intent: ResourceIntent,
}

const PROVIDER_CASES: &[ProviderCase] = &[
    ProviderCase {
        provider: "cli",
        instance: "cli",
        target: "cli/env",
        operations: &["read"],
        paths: &["HOME"],
        source: "@env := cli://cli/env{:read(HOME)}\nhome := @env/HOME",
        base_uri: "cli://cli/env",
        path: "HOME",
        intent: ResourceIntent::Read,
    },
    ProviderCase {
        provider: "cli",
        instance: "cli",
        target: "cli/stdout",
        operations: &["write"],
        paths: &["line"],
        source: "@out := cli://stdout{:write(line)}\n@out/line <- \"planned-alias\"",
        base_uri: "cli://stdout",
        path: "line",
        intent: ResourceIntent::Send,
    },
    ProviderCase {
        provider: "console",
        instance: "console",
        target: "console/output",
        operations: &["write"],
        paths: &["line"],
        source: "@out := console://console/output{:write(line)}\n@out/line <- \"planned\"",
        base_uri: "console://console/output",
        path: "line",
        intent: ResourceIntent::Send,
    },
    ProviderCase {
        provider: "console",
        instance: "console",
        target: "console/output",
        operations: &["write"],
        paths: &["line"],
        source: "@out := console://output{:write(line)}\n@out/line <- \"planned-alias\"",
        base_uri: "console://output",
        path: "line",
        intent: ResourceIntent::Send,
    },
    ProviderCase {
        provider: "time",
        instance: "clock",
        target: "clock/clock",
        operations: &["read"],
        paths: &["second"],
        source: "@clock := time://clock/clock{:read(second)}\nsecond := @clock/second",
        base_uri: "time://clock/clock",
        path: "second",
        intent: ResourceIntent::Read,
    },
    ProviderCase {
        provider: "timer",
        instance: "timer",
        target: "timer/tick",
        operations: &["read"],
        paths: &["tick"],
        source: "@timer := timer://timer/tick{:read(tick)}\ntick := @timer/tick",
        base_uri: "timer://timer/tick",
        path: "tick",
        intent: ResourceIntent::Read,
    },
    ProviderCase {
        provider: "scene",
        instance: "scene",
        target: "scene/frame",
        operations: &["write"],
        paths: &["replace"],
        source: "@frame := scene://scene/frame{:write(replace)}\n@frame/replace <- \"planned\"",
        base_uri: "scene://scene/frame",
        path: "replace",
        intent: ResourceIntent::Send,
    },
    ProviderCase {
        provider: "robot-arm",
        instance: "arm",
        target: "arm/commands",
        operations: &["move"],
        paths: &["move"],
        source: "@arm := robot://arm/commands{:move(move)}\n@arm/move <- true",
        base_uri: "robot://arm/commands",
        path: "move",
        intent: ResourceIntent::Send,
    },
];

fn compile_provider(case: &ProviderCase) -> ParsedProgram {
    let settings = if case.provider == "scene" {
        ConfigValue::Map(BTreeMap::from([
            (
                "renderer".to_owned(),
                ConfigValue::String("canvas".to_owned()),
            ),
            (
                "selector".to_owned(),
                ConfigValue::String("#scene".to_owned()),
            ),
        ]))
    } else {
        ConfigValue::Map(BTreeMap::new())
    };
    let mut runtime = RuntimeBuilder::new()
        .planning()
        .function_catalog(mech_stdlib::source_catalog())
        .host_factory(standard_planning_host_factory(case.provider).unwrap())
        .unwrap()
        .host_instance(HostInstanceConfig {
            name: case.instance.to_owned(),
            provider: case.provider.to_owned(),
            settings,
        })
        .run_resource_grant(RunResourceGrantConfig {
            target: case.target.to_owned(),
            operations: case
                .operations
                .iter()
                .map(|operation| (*operation).to_owned())
                .collect(),
            paths: case.paths.iter().map(|path| (*path).to_owned()).collect(),
        })
        .build()
        .unwrap();
    runtime.run_string(case.source).unwrap();
    ParsedProgram::from_bytes(&runtime.compile_program_bytecode().unwrap()).unwrap()
}

#[test]
fn every_standard_provider_plans_source_to_bytecode() {
    for case in PROVIDER_CASES {
        let parsed = compile_provider(case);
        assert!(
            parsed.requirements.iter().any(|requirement| {
                matches!(
                    requirement,
                    ApplicationRequirement::Resource(request)
                        if request.base_uri == case.base_uri
                            && request.path == case.path
                            && request.intent == case.intent
                )
            }),
            "missing planned requirement for {}",
            case.provider
        );
    }
}

#[test]
fn computed_resource_send_reuses_its_runtime_producer_in_native_planning() {
    let case = ProviderCase {
        provider: "cli",
        instance: "cli",
        target: "cli/stdout",
        operations: &["write"],
        paths: &["line"],
        source: "@out := cli://stdout{:write(line)}\nx := 1\n@out/line <- x + 1",
        base_uri: "cli://stdout",
        path: "line",
        intent: ResourceIntent::Send,
    };
    let parsed = compile_provider(&case);
    let source = parsed
        .instructions
        .iter()
        .find_map(|instruction| match instruction {
            BytecodeInstruction::ResourceSend { src, .. } => Some(*src),
            _ => None,
        })
        .expect("computed source emits a resource send");
    assert!(parsed.instructions.iter().any(|instruction| {
        matches!(instruction, BytecodeInstruction::RuntimeBinary { dst, .. } if *dst == source)
    }));

    let mut planning_runtime = RuntimeBuilder::new()
        .planning()
        .function_catalog(mech_stdlib::source_catalog())
        .host_factory(standard_planning_host_factory("cli").unwrap())
        .unwrap()
        .host_instance(HostInstanceConfig {
            name: "cli".to_owned(),
            provider: "cli".to_owned(),
            settings: ConfigValue::Map(BTreeMap::new()),
        })
        .run_resource_grant(RunResourceGrantConfig {
            target: "cli/stdout".to_owned(),
            operations: vec!["write".to_owned()],
            paths: vec!["line".to_owned()],
        })
        .build()
        .unwrap();
    planning_runtime.run_string(case.source).unwrap();
    let request = NativeBuildRequest {
        bytecode: planning_runtime.compile_program_bytecode().unwrap(),
        runtime_config: Some(NativeRuntimeConfig {
            runtime: RuntimeConfig::default(),
            actor_bootstrap: None,
            hosts: vec![HostInstanceConfig {
                name: "cli".to_owned(),
                provider: "cli".to_owned(),
                settings: ConfigValue::Map(BTreeMap::new()),
            }],
            run_grants: vec![RunResourceGrantConfig {
                target: "cli/stdout".to_owned(),
                operations: vec!["write".to_owned()],
                paths: vec!["line".to_owned()],
            }],
        }),
        target: None,
        profile: NativeBuildProfile::Debug,
        binary_name: "computed-resource-send".to_owned(),
        output: PathBuf::from("ignored"),
        emit: NativeEmit::Plan,
        keep_project: false,
        offline: true,
    };
    NativeApplicationBuilder::new(NativeBuildEnvironment {
        function_catalog: mech_stdlib::native_plan_catalog(),
        host_catalog: standard_native_host_catalog().unwrap(),
        dependency_source: NativeDependencySource::Registry {
            version: mech_build::MECH_COMPONENT_VERSION.to_owned(),
        },
    })
    .plan(&request)
    .unwrap();
}

const ACTOR_FUNCTIONS: &[(&str, &str)] = &[
    ("actor/message/kind", "actor/message/kind()"),
    ("actor/message/payload", "actor/message/payload()"),
    ("actor/state/get", "actor/state/get()"),
    ("actor/state/id", "actor/state/id()"),
    ("actor/state/put", "actor/state/put(\"planned\")"),
];

#[test]
fn every_trusted_actor_function_plans_source_to_bytecode() {
    for (name, source) in ACTOR_FUNCTIONS {
        let planned_name = (*name).to_owned();
        let mut runtime = RuntimeBuilder::new()
            .planning()
            .function_catalog(mech_stdlib::source_catalog())
            .host_function(PlannedPureHostFunction::new(
                *name,
                move |_context, _arguments| {
                    RuntimeValueSnapshot::try_capture(&Value::String(Ref::new(String::new())))
                },
                move |_context, _arguments| panic!("{planned_name} executed while source planning"),
            ))
            .unwrap()
            .build()
            .unwrap();
        runtime.run_string(source).unwrap();
        let parsed =
            ParsedProgram::from_bytes(&runtime.compile_program_bytecode().unwrap()).unwrap();
        assert_eq!(
            parsed
                .requirements
                .iter()
                .filter_map(|requirement| match requirement {
                    ApplicationRequirement::HostFunction(request) => Some(request.name.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>(),
            [*name],
        );
    }
}
