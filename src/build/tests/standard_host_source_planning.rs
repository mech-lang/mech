#![cfg(feature = "standard-hosts")]

use std::collections::BTreeMap;

use mech_build::standard_planning_host_factory;
use mech_core::{ApplicationRequirement, ParsedProgram, Ref, ResourceIntent, Value};
use mech_runtime::{
    ConfigValue, HostInstanceConfig, PlannedPureHostFunction, RunResourceGrantConfig,
    RuntimeBuilder, RuntimeValueSnapshot,
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
