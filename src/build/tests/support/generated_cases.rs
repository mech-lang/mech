use std::collections::BTreeMap;

use mech_build::selected_planning_host_factory;
use mech_core::{LegacyValue, Ref};
use mech_runtime::{
    ConfigValue, HostInstanceConfig, PlannedPureHostFunction, RunResourceGrantConfig,
    RuntimeBuilder, RuntimeValueSnapshot,
};

use super::{OwnerProfile, fixture_path};

pub struct GeneratedCase {
    pub profile: OwnerProfile,
    pub case: &'static str,
    pub binary_name: &'static str,
    pub bytecode: Vec<u8>,
    pub expected_stdout: &'static str,
}

pub fn generated_cases() -> Vec<GeneratedCase> {
    let mut cases = vec![
        frozen(
            "literal",
            "generated_native_literal",
            "literal-f64.mecb",
            "42",
        ),
        frozen(
            "scalar",
            "generated_native_scalar",
            "scalar-add-f64.mecb",
            "3",
        ),
        compiled("unary", "generated_native_unary", "[9.0]", "[9]"),
        compiled(
            "ternary",
            "generated_native_ternary",
            "1.0..1.0..=4.0",
            "[1 2 3 4]",
        ),
        compiled(
            "quaternary",
            "generated_native_quaternary",
            concat!(
                "a := [1.0 2.0 3.0 4.0 5.0]; b := [6.0 7.0 8.0 9.0 10.0]; ",
                "c := [11.0 12.0 13.0 14.0 15.0]; ",
                "d := [16.0 17.0 18.0 19.0 20.0]; [a; b; c; d]",
            ),
            "[1 2 3 4 5; 6 7 8 9 10; 11 12 13 14 15; 16 17 18 19 20]",
        ),
        frozen(
            "variadic",
            "generated_native_variadic",
            "variadic-horzcat-f64.mecb",
            "[1 2 3 4 5]",
        ),
        compiled(
            "integrity",
            "generated_native_integrity",
            "x := 1.0\nsafe! := x <= 2.0\n\"integrity-done\"",
            "\"integrity-done\"",
        ),
        GeneratedCase {
            profile: OwnerProfile::Fixed,
            case: "fixed-matrix",
            binary_name: "generated_native_fixed_matrix",
            bytecode: std::fs::read(fixture_path("fixed-matrix-add-f64.mecb")).unwrap(),
            expected_stdout: "[6 8; 10 12]",
        },
        frozen(
            "dynamic-matrix",
            "generated_native_dynamic_matrix",
            "dynamic-matrix-add-f64.mecb",
            "[26 26 26 26 26; 26 26 26 26 26; 26 26 26 26 26; 26 26 26 26 26; 26 26 26 26 26]",
        ),
        generated_cli_alias_case(),
        hosted(
            "console",
            "generated_native_console",
            "console",
            "@out := console://output{:write(line)}\n@out/line <- \"generated-console-ok\"\n\"console-done\"",
            "generated-console-ok\n\"console-done\"",
        ),
        hosted(
            "time-once",
            "generated_native_time",
            "time",
            "@clock := time://clock/clock{:read(second)}\nsecond := @clock/second\n\"time-done\"",
            "\"time-done\"",
        ),
        hosted(
            "timer-once",
            "generated_native_timer",
            "timer",
            "@timer := timer://timer/tick{:read(tick)}\ntick := @timer/tick\n\"timer-done\"",
            "\"timer-done\"",
        ),
        hosted(
            "scene",
            "generated_native_scene",
            "scene",
            concat!(
                "@frame := scene://scene/frame{:write(replace)}\n",
                "frame := {width: 640.0, height: 480.0, background: \"black\"}\n",
                "@frame/replace <- frame\n\"scene-done\"",
            ),
            "\"scene-done\"",
        ),
        hosted(
            "robot-arm",
            "generated_native_robot_arm",
            "robot-arm",
            "@arm := robot://arm/commands{:move(move)}\n@arm/move <- true\n\"robot-done\"",
            "\"robot-done\"",
        ),
    ];
    #[cfg(feature = "experimental-actors")]
    cases.extend([
        actor_case(
            "actor-alpha",
            "generated_native_actor_alpha",
            "\"payload-a\"",
        ),
        actor_case("actor-beta", "generated_native_actor_beta", "\"payload-b\""),
    ]);
    cases
}

pub fn generated_cli_alias_case() -> GeneratedCase {
    hosted(
        "cli",
        "generated_native_cli",
        "cli",
        "@out := cli://stdout{:write(line)}\n@out/line <- \"phase1-hosted-ok\"\n\"done\"",
        "phase1-hosted-ok\n\"done\"",
    )
}

fn frozen(
    case: &'static str,
    binary_name: &'static str,
    fixture: &str,
    expected_stdout: &'static str,
) -> GeneratedCase {
    GeneratedCase {
        profile: OwnerProfile::Standard,
        case,
        binary_name,
        bytecode: std::fs::read(fixture_path(fixture)).unwrap(),
        expected_stdout,
    }
}

fn compiled(
    case: &'static str,
    binary_name: &'static str,
    source: &str,
    expected_stdout: &'static str,
) -> GeneratedCase {
    let mut runtime = RuntimeBuilder::new()
        .planning()
        .function_catalog(mech_stdlib::source_catalog())
        .build()
        .unwrap();
    runtime.run_string(source).unwrap();
    GeneratedCase {
        profile: OwnerProfile::Standard,
        case,
        binary_name,
        bytecode: runtime.compile_program_bytecode().unwrap(),
        expected_stdout,
    }
}

fn hosted(
    case: &'static str,
    binary_name: &'static str,
    provider: &'static str,
    source: &str,
    expected_stdout: &'static str,
) -> GeneratedCase {
    let (instance, target, operations, paths, settings) = host_configuration(provider);
    let mut runtime = RuntimeBuilder::new()
        .planning()
        .function_catalog(mech_stdlib::source_catalog())
        .host_factory(selected_planning_host_factory(provider).unwrap())
        .unwrap()
        .host_instance(HostInstanceConfig {
            name: instance.to_owned(),
            provider: provider.to_owned(),
            settings,
        })
        .run_resource_grant(RunResourceGrantConfig {
            target: target.to_owned(),
            operations: operations.into_iter().map(str::to_owned).collect(),
            paths: paths.into_iter().map(str::to_owned).collect(),
        })
        .build()
        .unwrap();
    runtime.run_string(source).unwrap();
    GeneratedCase {
        profile: if provider == "robot-arm" {
            OwnerProfile::Full
        } else {
            OwnerProfile::Standard
        },
        case,
        binary_name,
        bytecode: runtime.compile_program_bytecode().unwrap(),
        expected_stdout,
    }
}

#[cfg(feature = "experimental-actors")]
fn actor_case(
    case: &'static str,
    binary_name: &'static str,
    expected: &'static str,
) -> GeneratedCase {
    let mut builder = RuntimeBuilder::new()
        .planning()
        .function_catalog(mech_stdlib::source_catalog());
    for name in [
        "actor/message/kind",
        "actor/message/payload",
        "actor/state/id",
        "actor/state/get",
        "actor/state/put",
    ] {
        builder = builder
            .host_function(PlannedPureHostFunction::new(
                name,
                |_context, _arguments| {
                    RuntimeValueSnapshot::try_capture(&LegacyValue::String(Ref::new(String::new())))
                },
                move |_context, _arguments| panic!("{name} executed while planning"),
            ))
            .unwrap();
    }
    let mut runtime = builder.build().unwrap();
    runtime
        .run_string(concat!(
            "kind := actor/message/kind()\n",
            "payload := actor/message/payload()\n",
            "state-id := actor/state/id()\n",
            "state := actor/state/get()\n",
            "updated := actor/state/put(kind)\n",
            "payload",
        ))
        .unwrap();
    GeneratedCase {
        profile: OwnerProfile::Standard,
        case,
        binary_name,
        bytecode: runtime.compile_program_bytecode().unwrap(),
        expected_stdout: expected,
    }
}

fn host_configuration(
    provider: &str,
) -> (
    &'static str,
    &'static str,
    Vec<&'static str>,
    Vec<&'static str>,
    ConfigValue,
) {
    let empty = || ConfigValue::Map(BTreeMap::new());
    match provider {
        "cli" => ("cli", "cli/stdout", vec!["write"], vec!["line"], empty()),
        "console" => (
            "console",
            "console/output",
            vec!["write"],
            vec!["line"],
            empty(),
        ),
        "time" => (
            "clock",
            "clock/clock",
            vec!["read"],
            vec!["second"],
            empty(),
        ),
        "timer" => ("timer", "timer/tick", vec!["read"], vec!["tick"], empty()),
        "scene" => (
            "scene",
            "scene/frame",
            vec!["write"],
            vec!["replace"],
            ConfigValue::Map(BTreeMap::from([
                (
                    "renderer".to_owned(),
                    ConfigValue::String("canvas".to_owned()),
                ),
                (
                    "selector".to_owned(),
                    ConfigValue::String("#scene".to_owned()),
                ),
            ])),
        ),
        "robot-arm" => ("arm", "arm/commands", vec!["move"], vec!["move"], empty()),
        _ => panic!("unsupported generated host provider {provider}"),
    }
}
use mech_runtime::legacy_interpreter::LegacyInterpreterTestExt as _;
