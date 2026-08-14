use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::error::Error;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use mech_build::{
    MECH_COMPONENT_VERSION, NativeApplicationBuilder, NativeBuildEnvironment, NativeBuildProfile,
    NativeBuildRequest, NativeDependencySource, NativeEmit, NativeHostCatalog, NativeHostLinkage,
    NativeRuntimeConfig, NativeTargetFamily, selected_native_host_catalog,
};
use mech_core::{
    ApplicationRequirement, BytecodeInstruction, BytecodeProgram, EncodedConstant, MResult,
    MatrixStorage, ParsedProgram, RuntimeType, hash_str, write_bytecode,
};
use mech_native_live_host_fixture::{
    TEST_LIVE_CONTEXT, TEST_LIVE_INSTANCE, TEST_LIVE_OUTPUT_CONTEXT, TEST_LIVE_PROVIDER,
    TEST_LIVE_PATH, TEST_LIVE_RECORD_PATH, TEST_LIVE_TUPLE_PATH, TestLiveHostFactory,
    empty_settings,
};
use mech_runtime::{
    ConfigValue, HostInstanceConfig, RunResourceGrantConfig, RuntimeConfig, RuntimeHostFactory,
};
use serde_json::{Value as JsonValue, json};
use sha2::{Digest, Sha256};

type AppResult<T> = Result<T, Box<dyn Error>>;

const FIXTURE_FILES: [&str; 20] = [
    "canonical-scalars.mecb",
    "canonical-matrices.mecb",
    "canonical-composites.mecb",
    "literal-f64.mecb",
    "scalar-add-f64.mecb",
    "fixed-matrix-add-f64.mecb",
    "dynamic-matrix-add-f64.mecb",
    "variadic-horzcat-f64.mecb",
    "string.mecb",
    "unary.mecb",
    "ternary.mecb",
    "quaternary.mecb",
    "named-module-operation.mecb",
    "cli-stdout.mecb",
    "console.mecb",
    "time.mecb",
    "timer.mecb",
    "scene.mecb",
    "robot-arm.mecb",
    "synthetic-live-read.mecb",
];
const SOURCE_DIRECTORY: &str = "sources";
const SOURCE_FILES: [&str; 17] = [
    "literal-f64.mec",
    "scalar-add.mec",
    "fixed-matrix-add-f64.mec",
    "dynamic-matrix-add-f64.mec",
    "variadic-horzcat-f64.mec",
    "string.mec",
    "unary.mec",
    "ternary.mec",
    "quaternary.mec",
    "named-module-operation.mec",
    "cli-stdout.mec",
    "console.mec",
    "time.mec",
    "timer.mec",
    "scene.mec",
    "robot-arm.mec",
    "synthetic-live-read.mec",
];
const DEFAULT_DETERMINISM_RUNS: usize = 5;
const LITERAL_SOURCE: &str = "42.0";
const SCALAR_SOURCE: &str = "1.0 + 2.0";
const FIXED_MATRIX_SOURCE: &str = "[1.0 2.0; 3.0 4.0] + [5.0 6.0; 7.0 8.0]";
const DYNAMIC_MATRIX_SOURCE: &str = concat!(
    "[1.0 2.0 3.0 4.0 5.0; 6.0 7.0 8.0 9.0 10.0; ",
    "11.0 12.0 13.0 14.0 15.0; 16.0 17.0 18.0 19.0 20.0; ",
    "21.0 22.0 23.0 24.0 25.0] + ",
    "[25.0 24.0 23.0 22.0 21.0; 20.0 19.0 18.0 17.0 16.0; ",
    "15.0 14.0 13.0 12.0 11.0; 10.0 9.0 8.0 7.0 6.0; ",
    "5.0 4.0 3.0 2.0 1.0]",
);
const VARIADIC_SOURCE: &str = "[1.0 2.0 3.0 4.0 5.0]";
const STRING_SOURCE: &str = "\"bytecode-v1\"";
const UNARY_SOURCE: &str = "[9.0]";
const TERNARY_SOURCE: &str = "1.0..1.0..=4.0";
const QUATERNARY_SOURCE: &str = concat!(
    "a := [1.0 2.0 3.0 4.0 5.0]; b := [6.0 7.0 8.0 9.0 10.0]; ",
    "c := [11.0 12.0 13.0 14.0 15.0]; d := [16.0 17.0 18.0 19.0 20.0]; ",
    "[a; b; c; d]",
);
const NAMED_MODULE_SOURCE: &str = "+> math\nmath/sin(0.0)";
const CLI_SOURCE: &str = "+> @out := cli/stdout\n\n@out/line <- \"phase1-hosted-ok\"\n\n\"done\"";
const CONSOLE_SOURCE: &str = concat!(
    "@out := console://console/output{:write(line)}\n",
    "@out/line <- \"bytecode-v1-console\"\n\"console-done\"",
);
const TIME_SOURCE: &str = concat!(
    "@clock := time://clock/clock{:read(second)}\n",
    "second := @clock/second\n\"time-done\"",
);
const TIMER_SOURCE: &str = concat!(
    "@timer := timer://timer/tick{:read(tick)}\n",
    "tick := @timer/tick\n\"timer-done\"",
);
const SCENE_SOURCE: &str = concat!(
    "@frame := scene://scene/frame{:write(replace)}\n",
    "frame := {width: 640.0, height: 480.0, background: \"black\"}\n",
    "@frame/replace <- frame\n\"scene-done\"",
);
const ROBOT_ARM_SOURCE: &str = concat!(
    "@arm := robot://arm/commands{:move(move)}\n",
    "@arm/move <- true\n\"robot-done\"",
);
const LIVE_SOURCE: &str = concat!(
    "+> @clock := test-live/clock\n\n",
    "+> @out := test-live/output\n\n",
    "value := @clock/value\n",
    "doubled := value + value\n\n",
    "payload := (value, doubled)\n",
    "frame := {rotation: value}\n",
    "@out/tuple <- payload\n",
    "@out/frame <- frame\n\n",
    "payload",
);

#[derive(Clone, Copy)]
enum PlanCatalog {
    Standard,
    SyntheticLive,
}

struct Fixture {
    file: &'static str,
    source_file: Option<&'static str>,
    source: Option<&'static str>,
    construction: Option<&'static str>,
    bytes: Vec<u8>,
    runtime_functions: Vec<String>,
    runtime_config: Option<NativeRuntimeConfig>,
    plan_catalog: PlanCatalog,
    expected_output: JsonValue,
}

fn main() -> AppResult<()> {
    let args = env::args_os().skip(1).collect::<Vec<_>>();
    match args.as_slice() {
        [mode] if mode == "--write" => {
            let output = corpus_directory();
            write_generated_corpus(&output)?;
            println!("wrote bytecode v1 corpus to {}", output.display());
            Ok(())
        }
        [mode] if mode == "--check" => check_corpus(),
        [mode, output] if mode == "--emit" => write_generated_corpus(Path::new(output)),
        _ => Err(io::Error::other("usage: bytecode-v1-generator (--write | --check)").into()),
    }
}

fn corpus_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../architecture/bytecode-v1")
}

fn fixtures() -> AppResult<Vec<Fixture>> {
    let fixed_output = vec![6.0, 8.0, 10.0, 12.0];
    let dynamic_output = vec![26.0; 25];

    let scalars = canonical_scalars()?;
    let matrices = canonical_matrices()?;
    let composites = canonical_composites()?;
    let source_compiler = build_source_compiler()?;
    let (literal, literal_functions) =
        compile_source(&source_compiler, "standard", LITERAL_SOURCE)?;
    let (scalar, scalar_functions) = compile_source(&source_compiler, "standard", SCALAR_SOURCE)?;
    let (fixed, fixed_runtime_functions) = compile_fixed_source(FIXED_MATRIX_SOURCE)?;
    let (dynamic, dynamic_functions) =
        compile_source(&source_compiler, "standard", DYNAMIC_MATRIX_SOURCE)?;
    let (variadic, variadic_functions) =
        compile_source(&source_compiler, "standard", VARIADIC_SOURCE)?;
    let (string, string_functions) = compile_source(&source_compiler, "standard", STRING_SOURCE)?;
    let (unary, unary_functions) = compile_source(&source_compiler, "standard", UNARY_SOURCE)?;
    let (ternary, ternary_functions) =
        compile_source(&source_compiler, "standard", TERNARY_SOURCE)?;
    let (quaternary, quaternary_functions) =
        compile_source(&source_compiler, "standard", QUATERNARY_SOURCE)?;
    let (named_module, named_module_functions) =
        compile_source(&source_compiler, "standard", NAMED_MODULE_SOURCE)?;
    let (cli, cli_functions) = compile_source(&source_compiler, "cli", CLI_SOURCE)?;
    let (console, console_functions) = compile_source(&source_compiler, "console", CONSOLE_SOURCE)?;
    let (time, time_functions) = compile_source(&source_compiler, "time", TIME_SOURCE)?;
    let (timer, timer_functions) = compile_source(&source_compiler, "timer", TIMER_SOURCE)?;
    let (scene, scene_functions) = compile_source(&source_compiler, "scene", SCENE_SOURCE)?;
    let (robot_arm, robot_arm_functions) =
        compile_source(&source_compiler, "robot-arm", ROBOT_ARM_SOURCE)?;
    let (live, live_functions) = compile_source(&source_compiler, "synthetic-live", LIVE_SOURCE)?;

    Ok(vec![
        Fixture {
            file: "canonical-scalars.mecb",
            source_file: None,
            source: None,
            construction: Some(
                "authoritative BytecodeProgram containing every canonical scalar constant; returns the String constant",
            ),
            runtime_functions: Vec::new(),
            bytes: scalars,
            runtime_config: None,
            plan_catalog: PlanCatalog::Standard,
            expected_output: json!("bytecode-v1 🦀"),
        },
        Fixture {
            file: "canonical-matrices.mecb",
            source_file: None,
            source: None,
            construction: Some(
                "authoritative BytecodeProgram containing every MatrixStorage tag and every matrix element codec family; returns Matrix2<f64>",
            ),
            runtime_functions: Vec::new(),
            bytes: matrices,
            runtime_config: None,
            plan_catalog: PlanCatalog::Standard,
            expected_output: matrix_json(2, 2, &[0.0, 1.0, 2.0, 3.0]),
        },
        Fixture {
            file: "canonical-composites.mecb",
            source_file: None,
            source: None,
            construction: Some(
                "authoritative BytecodeProgram containing atom, enum, tuple, record, map, set, table, option-none, option-some, reference, and kind constants",
            ),
            runtime_functions: Vec::new(),
            bytes: composites,
            runtime_config: None,
            plan_catalog: PlanCatalog::Standard,
            expected_output: json!([7, "mech"]),
        },
        Fixture {
            file: "literal-f64.mecb",
            source_file: Some("literal-f64.mec"),
            source: Some(LITERAL_SOURCE),
            construction: None,
            runtime_functions: literal_functions,
            bytes: literal,
            runtime_config: None,
            plan_catalog: PlanCatalog::Standard,
            expected_output: json!(42.0),
        },
        Fixture {
            file: "scalar-add-f64.mecb",
            source_file: Some("scalar-add.mec"),
            source: Some(SCALAR_SOURCE),
            construction: None,
            runtime_functions: scalar_functions,
            bytes: scalar,
            runtime_config: None,
            plan_catalog: PlanCatalog::Standard,
            expected_output: json!(3.0),
        },
        Fixture {
            file: "fixed-matrix-add-f64.mecb",
            source_file: Some("fixed-matrix-add-f64.mec"),
            source: Some(FIXED_MATRIX_SOURCE),
            construction: None,
            runtime_functions: fixed_runtime_functions,
            bytes: fixed,
            runtime_config: None,
            plan_catalog: PlanCatalog::Standard,
            expected_output: matrix_json(2, 2, &fixed_output),
        },
        Fixture {
            file: "dynamic-matrix-add-f64.mecb",
            source_file: Some("dynamic-matrix-add-f64.mec"),
            source: Some(DYNAMIC_MATRIX_SOURCE),
            construction: None,
            runtime_functions: dynamic_functions,
            bytes: dynamic,
            runtime_config: None,
            plan_catalog: PlanCatalog::Standard,
            expected_output: matrix_json(5, 5, &dynamic_output),
        },
        Fixture {
            file: "variadic-horzcat-f64.mecb",
            source_file: Some("variadic-horzcat-f64.mec"),
            source: Some(VARIADIC_SOURCE),
            construction: None,
            runtime_functions: variadic_functions,
            bytes: variadic,
            runtime_config: None,
            plan_catalog: PlanCatalog::Standard,
            expected_output: matrix_json(1, 5, &[1.0, 2.0, 3.0, 4.0, 5.0]),
        },
        source_fixture(
            "string.mecb",
            "string.mec",
            STRING_SOURCE,
            string,
            string_functions,
            json!("bytecode-v1"),
        )?,
        source_fixture(
            "unary.mecb",
            "unary.mec",
            UNARY_SOURCE,
            unary,
            unary_functions,
            json!([9.0]),
        )?,
        source_fixture(
            "ternary.mecb",
            "ternary.mec",
            TERNARY_SOURCE,
            ternary,
            ternary_functions,
            json!([1.0, 2.0, 3.0, 4.0]),
        )?,
        source_fixture(
            "quaternary.mecb",
            "quaternary.mec",
            QUATERNARY_SOURCE,
            quaternary,
            quaternary_functions,
            matrix_json(
                4,
                5,
                &[
                    1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0,
                    15.0, 16.0, 17.0, 18.0, 19.0, 20.0,
                ],
            ),
        )?,
        source_fixture(
            "named-module-operation.mecb",
            "named-module-operation.mec",
            NAMED_MODULE_SOURCE,
            named_module,
            named_module_functions,
            json!(0.0),
        )?,
        Fixture {
            file: "cli-stdout.mecb",
            source_file: Some("cli-stdout.mec"),
            source: Some(CLI_SOURCE),
            construction: None,
            runtime_functions: cli_functions,
            bytes: cli,
            runtime_config: Some(cli_runtime_config()),
            plan_catalog: PlanCatalog::Standard,
            expected_output: json!("done"),
        },
        hosted_fixture(
            "console.mecb",
            "console.mec",
            "console",
            CONSOLE_SOURCE,
            console,
            console_functions,
            json!("console-done"),
        )?,
        hosted_fixture(
            "time.mecb",
            "time.mec",
            "time",
            TIME_SOURCE,
            time,
            time_functions,
            json!("time-done"),
        )?,
        hosted_fixture(
            "timer.mecb",
            "timer.mec",
            "timer",
            TIMER_SOURCE,
            timer,
            timer_functions,
            json!("timer-done"),
        )?,
        hosted_fixture(
            "scene.mecb",
            "scene.mec",
            "scene",
            SCENE_SOURCE,
            scene,
            scene_functions,
            json!("scene-done"),
        )?,
        hosted_fixture(
            "robot-arm.mecb",
            "robot-arm.mec",
            "robot-arm",
            ROBOT_ARM_SOURCE,
            robot_arm,
            robot_arm_functions,
            json!("robot-done"),
        )?,
        Fixture {
            file: "synthetic-live-read.mecb",
            source_file: Some("synthetic-live-read.mec"),
            source: Some(LIVE_SOURCE),
            construction: None,
            runtime_functions: live_functions,
            bytes: live,
            runtime_config: Some(synthetic_live_runtime_config()),
            plan_catalog: PlanCatalog::SyntheticLive,
            expected_output: json!([0.0, 0.0]),
        },
    ])
}

fn source_fixture(
    file: &'static str,
    source_file: &'static str,
    source: &'static str,
    bytes: Vec<u8>,
    runtime_functions: Vec<String>,
    expected_output: JsonValue,
) -> AppResult<Fixture> {
    Ok(Fixture {
        file,
        source_file: Some(source_file),
        source: Some(source),
        construction: None,
        runtime_functions,
        bytes,
        runtime_config: None,
        plan_catalog: PlanCatalog::Standard,
        expected_output,
    })
}

fn hosted_fixture(
    file: &'static str,
    source_file: &'static str,
    provider: &'static str,
    source: &'static str,
    bytes: Vec<u8>,
    runtime_functions: Vec<String>,
    expected_output: JsonValue,
) -> AppResult<Fixture> {
    Ok(Fixture {
        file,
        source_file: Some(source_file),
        source: Some(source),
        construction: None,
        runtime_functions,
        bytes,
        runtime_config: Some(host_runtime_config(provider)?),
        plan_catalog: PlanCatalog::Standard,
        expected_output,
    })
}

fn constructed_program(
    constants: Vec<EncodedConstant>,
    returned_constant: u32,
) -> AppResult<Vec<u8>> {
    let register_count = u32::try_from(constants.len())
        .map_err(|_| io::Error::other("constructed bytecode has too many constants"))?;
    if returned_constant >= register_count {
        return Err(io::Error::other(format!(
            "constructed bytecode return constant {returned_constant} exceeds constant count {register_count}",
        ))
        .into());
    }
    let mut instructions = (0..register_count)
        .map(|constant| BytecodeInstruction::ConstLoad {
            dst: constant,
            constant,
        })
        .collect::<Vec<_>>();
    instructions.push(BytecodeInstruction::Return {
        src: returned_constant,
    });
    let program = BytecodeProgram {
        register_count,
        constants,
        symbols: BTreeMap::new(),
        mutable_symbols: BTreeSet::new(),
        instructions,
        dictionary: BTreeMap::new(),
        requirements: Vec::new(),
    };
    write_bytecode(&program)
        .map_err(|error| mech_error("constructed bytecode serialization", error).into())
}

fn canonical_scalars() -> AppResult<Vec<u8>> {
    let constants = vec![
        EncodedConstant {
            runtime_type: RuntimeType::U8,
            alignment: 1,
            bytes: vec![u8::MAX],
        },
        EncodedConstant {
            runtime_type: RuntimeType::U16,
            alignment: 2,
            bytes: u16::MAX.to_le_bytes().to_vec(),
        },
        EncodedConstant {
            runtime_type: RuntimeType::U32,
            alignment: 4,
            bytes: u32::MAX.to_le_bytes().to_vec(),
        },
        EncodedConstant {
            runtime_type: RuntimeType::U64,
            alignment: 8,
            bytes: u64::MAX.to_le_bytes().to_vec(),
        },
        EncodedConstant {
            runtime_type: RuntimeType::U128,
            alignment: 16,
            bytes: u128::MAX.to_le_bytes().to_vec(),
        },
        EncodedConstant {
            runtime_type: RuntimeType::I8,
            alignment: 1,
            bytes: i8::MIN.to_le_bytes().to_vec(),
        },
        EncodedConstant {
            runtime_type: RuntimeType::I16,
            alignment: 2,
            bytes: i16::MIN.to_le_bytes().to_vec(),
        },
        EncodedConstant {
            runtime_type: RuntimeType::I32,
            alignment: 4,
            bytes: i32::MIN.to_le_bytes().to_vec(),
        },
        EncodedConstant {
            runtime_type: RuntimeType::I64,
            alignment: 8,
            bytes: i64::MIN.to_le_bytes().to_vec(),
        },
        EncodedConstant {
            runtime_type: RuntimeType::I128,
            alignment: 16,
            bytes: i128::MIN.to_le_bytes().to_vec(),
        },
        EncodedConstant {
            runtime_type: RuntimeType::F32,
            alignment: 4,
            bytes: 0x7fc0_1234_u32.to_le_bytes().to_vec(),
        },
        EncodedConstant {
            runtime_type: RuntimeType::F64,
            alignment: 8,
            bytes: 0x7ff8_0000_0000_1234_u64.to_le_bytes().to_vec(),
        },
        EncodedConstant {
            runtime_type: RuntimeType::C64,
            alignment: 8,
            bytes: [
                (-0.0_f64).to_bits().to_le_bytes(),
                1.5_f64.to_bits().to_le_bytes(),
            ]
            .concat(),
        },
        EncodedConstant {
            runtime_type: RuntimeType::R64,
            alignment: 8,
            bytes: [(-3_i64).to_le_bytes(), 7_i64.to_le_bytes()].concat(),
        },
        EncodedConstant {
            runtime_type: RuntimeType::String,
            alignment: 1,
            bytes: "bytecode-v1 🦀".as_bytes().to_vec(),
        },
        EncodedConstant {
            runtime_type: RuntimeType::Bool,
            alignment: 1,
            bytes: vec![1],
        },
        EncodedConstant {
            runtime_type: RuntimeType::Id,
            alignment: 8,
            bytes: 42_u64.to_le_bytes().to_vec(),
        },
        EncodedConstant {
            runtime_type: RuntimeType::Index,
            alignment: 8,
            bytes: 7_u64.to_le_bytes().to_vec(),
        },
        EncodedConstant {
            runtime_type: RuntimeType::Empty,
            alignment: 1,
            bytes: Vec::new(),
        },
    ];
    constructed_program(constants, 14)
}

fn matrix_constant(storage: MatrixStorage, rows: u32, cols: u32) -> EncodedConstant {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&rows.to_le_bytes());
    bytes.extend_from_slice(&cols.to_le_bytes());
    for value in 0..rows * cols {
        bytes.extend_from_slice(&f64::from(value).to_bits().to_le_bytes());
    }
    EncodedConstant {
        runtime_type: RuntimeType::Matrix {
            element: Box::new(RuntimeType::F64),
            storage,
            rows,
            cols,
        },
        alignment: 8,
        bytes,
    }
}

fn matrixd_constant(element: RuntimeType, element_bytes: Vec<u8>) -> EncodedConstant {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&1_u32.to_le_bytes());
    bytes.extend_from_slice(&1_u32.to_le_bytes());
    bytes.extend_from_slice(&element_bytes);
    EncodedConstant {
        runtime_type: RuntimeType::Matrix {
            element: Box::new(element),
            storage: MatrixStorage::MatrixD,
            rows: 1,
            cols: 1,
        },
        alignment: 8,
        bytes,
    }
}

fn canonical_matrices() -> AppResult<Vec<u8>> {
    let mut constants = [
        (MatrixStorage::Matrix1, 1, 1),
        (MatrixStorage::Matrix2, 2, 2),
        (MatrixStorage::Matrix3, 3, 3),
        (MatrixStorage::Matrix4, 4, 4),
        (MatrixStorage::Matrix2x3, 2, 3),
        (MatrixStorage::Matrix3x2, 3, 2),
        (MatrixStorage::RowVector2, 1, 2),
        (MatrixStorage::RowVector3, 1, 3),
        (MatrixStorage::RowVector4, 1, 4),
        (MatrixStorage::Vector2, 2, 1),
        (MatrixStorage::Vector3, 3, 1),
        (MatrixStorage::Vector4, 4, 1),
        (MatrixStorage::RowVectorD, 1, 5),
        (MatrixStorage::VectorD, 5, 1),
        (MatrixStorage::MatrixD, 2, 5),
    ]
    .into_iter()
    .map(|(storage, rows, cols)| matrix_constant(storage, rows, cols))
    .collect::<Vec<_>>();
    constants.extend([
        matrixd_constant(RuntimeType::Index, 5_u64.to_le_bytes().to_vec()),
        matrixd_constant(RuntimeType::Bool, vec![1]),
        matrixd_constant(RuntimeType::U8, vec![u8::MAX]),
        matrixd_constant(RuntimeType::U16, u16::MAX.to_le_bytes().to_vec()),
        matrixd_constant(RuntimeType::U32, u32::MAX.to_le_bytes().to_vec()),
        matrixd_constant(RuntimeType::U64, u64::MAX.to_le_bytes().to_vec()),
        matrixd_constant(RuntimeType::U128, u128::MAX.to_le_bytes().to_vec()),
        matrixd_constant(RuntimeType::I8, i8::MIN.to_le_bytes().to_vec()),
        matrixd_constant(RuntimeType::I16, i16::MIN.to_le_bytes().to_vec()),
        matrixd_constant(RuntimeType::I32, i32::MIN.to_le_bytes().to_vec()),
        matrixd_constant(RuntimeType::I64, i64::MIN.to_le_bytes().to_vec()),
        matrixd_constant(RuntimeType::I128, i128::MIN.to_le_bytes().to_vec()),
        matrixd_constant(RuntimeType::F32, 0x7fc0_1234_u32.to_le_bytes().to_vec()),
        matrixd_constant(
            RuntimeType::F64,
            0x7ff8_0000_0000_1234_u64.to_le_bytes().to_vec(),
        ),
        matrixd_constant(
            RuntimeType::C64,
            [
                (-0.0_f64).to_bits().to_le_bytes(),
                1.5_f64.to_bits().to_le_bytes(),
            ]
            .concat(),
        ),
        matrixd_constant(
            RuntimeType::R64,
            [(-3_i64).to_le_bytes(), 7_i64.to_le_bytes()].concat(),
        ),
        matrixd_constant(
            RuntimeType::String,
            [4_u32.to_le_bytes(), *b"mech"].concat(),
        ),
    ]);
    constructed_program(constants, 1)
}

fn append_child_payload(bytes: &mut Vec<u8>, child: &[u8]) {
    bytes.extend_from_slice(&(child.len() as u32).to_le_bytes());
    bytes.extend_from_slice(child);
}

fn canonical_composites() -> AppResult<Vec<u8>> {
    let mut tuple = 2_u32.to_le_bytes().to_vec();
    append_child_payload(&mut tuple, &[7]);
    append_child_payload(&mut tuple, b"mech");

    let mut record = 2_u32.to_le_bytes().to_vec();
    append_child_payload(&mut record, &[9]);
    append_child_payload(&mut record, &(-4_i16).to_le_bytes());

    let mut map = 1_u32.to_le_bytes().to_vec();
    append_child_payload(&mut map, &[3]);
    append_child_payload(&mut map, b"value");

    let mut set = 1_u32.to_le_bytes().to_vec();
    append_child_payload(&mut set, &[4]);

    let mut table = Vec::new();
    table.extend_from_slice(&1_u32.to_le_bytes());
    table.extend_from_slice(&2_u32.to_le_bytes());
    append_child_payload(&mut table, &[5]);
    append_child_payload(&mut table, b"row");

    let mut reference = Vec::new();
    append_child_payload(&mut reference, &[6]);

    let mut present_option = vec![1];
    append_child_payload(&mut present_option, &[8]);

    let enum_name = "status";
    let variant_name = "ready";
    let enum_id = hash_str(enum_name);
    let variant_id = hash_str(variant_name);
    let mut enumeration = 1_u32.to_le_bytes().to_vec();
    enumeration.extend_from_slice(&variant_id.to_le_bytes());
    enumeration.extend_from_slice(&(variant_name.len() as u32).to_le_bytes());
    enumeration.extend_from_slice(variant_name.as_bytes());
    enumeration.push(1);
    append_child_payload(&mut enumeration, &[1, 0]);
    append_child_payload(&mut enumeration, &[10]);

    let atom_name = "alpha";
    let atom_id = hash_str(atom_name);
    constructed_program(
        vec![
            EncodedConstant {
                runtime_type: RuntimeType::Tuple(vec![RuntimeType::U8, RuntimeType::String]),
                alignment: 4,
                bytes: tuple,
            },
            EncodedConstant {
                runtime_type: RuntimeType::Record(vec![
                    ("count".to_owned(), RuntimeType::U8),
                    ("delta".to_owned(), RuntimeType::I16),
                ]),
                alignment: 4,
                bytes: record,
            },
            EncodedConstant {
                runtime_type: RuntimeType::Map {
                    key: Box::new(RuntimeType::U8),
                    value: Box::new(RuntimeType::String),
                },
                alignment: 4,
                bytes: map,
            },
            EncodedConstant {
                runtime_type: RuntimeType::Set {
                    element: Box::new(RuntimeType::U8),
                    max_len: Some(1),
                },
                alignment: 4,
                bytes: set,
            },
            EncodedConstant {
                runtime_type: RuntimeType::Table {
                    columns: vec![
                        ("id".to_owned(), RuntimeType::U8),
                        ("name".to_owned(), RuntimeType::String),
                    ],
                    primary_key: 0,
                },
                alignment: 4,
                bytes: table,
            },
            EncodedConstant {
                runtime_type: RuntimeType::Reference(Box::new(RuntimeType::U8)),
                alignment: 4,
                bytes: reference,
            },
            EncodedConstant {
                runtime_type: RuntimeType::Option(Box::new(RuntimeType::U8)),
                alignment: 1,
                bytes: vec![0],
            },
            EncodedConstant {
                runtime_type: RuntimeType::Option(Box::new(RuntimeType::U8)),
                alignment: 4,
                bytes: present_option,
            },
            EncodedConstant {
                runtime_type: RuntimeType::Atom {
                    id: atom_id,
                    name: atom_name.to_owned(),
                },
                alignment: 1,
                bytes: Vec::new(),
            },
            EncodedConstant {
                runtime_type: RuntimeType::Enum {
                    id: enum_id,
                    name: enum_name.to_owned(),
                },
                alignment: 4,
                bytes: enumeration,
            },
            EncodedConstant {
                runtime_type: RuntimeType::Kind(mech_core::kind::Kind::Scalar(hash_str("u8"))),
                alignment: 1,
                bytes: Vec::new(),
            },
        ],
        0,
    )
}

fn build_source_compiler() -> AppResult<PathBuf> {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let manifest = repository.join("tests/fixtures/bytecode-v1-source-generator/Cargo.toml");
    let target = repository.join("target/bytecode-v1-fixtures/cargo-target");
    let cargo = env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let command_output = Command::new(cargo)
        .arg("build")
        .arg("--quiet")
        .arg("--locked")
        .arg("--offline")
        .arg("--manifest-path")
        .arg(&manifest)
        .env("CARGO_TARGET_DIR", &target)
        .output()?;
    if !command_output.status.success() {
        return Err(io::Error::other(format!(
            "isolated source compiler build failed with {}: stdout={} stderr={}",
            command_output.status,
            String::from_utf8_lossy(&command_output.stdout),
            String::from_utf8_lossy(&command_output.stderr),
        ))
        .into());
    }
    let executable = target.join("debug").join(format!(
        "bytecode-v1-source-generator{}",
        env::consts::EXE_SUFFIX
    ));
    if !executable.is_file() {
        return Err(io::Error::other(format!(
            "isolated source compiler was not written to {}",
            executable.display()
        ))
        .into());
    }
    Ok(executable)
}

fn compile_source(
    executable: &Path,
    mode: &str,
    source: &str,
) -> AppResult<(Vec<u8>, Vec<String>)> {
    let temporary = tempfile::tempdir()?;
    let output = temporary.path().join("fixture.mecb");
    let functions = temporary.path().join("functions.json");
    let command_output = Command::new(executable)
        .arg(&output)
        .arg(&functions)
        .arg(mode)
        .arg(source)
        .output()?;
    if !command_output.status.success() {
        return Err(io::Error::other(format!(
            "isolated {mode} source compiler failed with {}: stdout={} stderr={}",
            command_output.status,
            String::from_utf8_lossy(&command_output.stdout),
            String::from_utf8_lossy(&command_output.stderr),
        ))
        .into());
    }
    let bytes = fs::read(&output)?;
    ParsedProgram::from_bytes(&bytes)
        .map_err(|error| mech_error("source fixture bytecode validation", error))?;
    let runtime_functions = serde_json::from_slice(&fs::read(functions)?)?;
    Ok((bytes, runtime_functions))
}

fn cli_runtime_config() -> NativeRuntimeConfig {
    NativeRuntimeConfig {
        runtime: RuntimeConfig::new("bytecode-v1-cli"),
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
    }
}

fn host_runtime_config(provider: &str) -> AppResult<NativeRuntimeConfig> {
    let empty = || ConfigValue::Map(BTreeMap::new());
    let (instance, target, operations, paths, settings) = match provider {
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
        _ => return Err(io::Error::other(format!("unsupported host provider {provider}")).into()),
    };
    Ok(NativeRuntimeConfig {
        runtime: RuntimeConfig::new(format!("bytecode-v1-{provider}")),
        actor_bootstrap: None,
        hosts: vec![HostInstanceConfig {
            name: instance.to_owned(),
            provider: provider.to_owned(),
            settings,
        }],
        run_grants: vec![RunResourceGrantConfig {
            target: target.to_owned(),
            operations: operations.into_iter().map(str::to_owned).collect(),
            paths: paths.into_iter().map(str::to_owned).collect(),
        }],
    })
}

fn synthetic_live_runtime_config() -> NativeRuntimeConfig {
    NativeRuntimeConfig {
        runtime: RuntimeConfig::new("bytecode-v1-synthetic-live"),
        actor_bootstrap: None,
        hosts: vec![HostInstanceConfig {
            name: TEST_LIVE_INSTANCE.to_owned(),
            provider: TEST_LIVE_PROVIDER.to_owned(),
            settings: empty_settings(),
        }],
        run_grants: vec![
            RunResourceGrantConfig {
                target: format!("{TEST_LIVE_INSTANCE}/{TEST_LIVE_CONTEXT}"),
                operations: vec!["read".to_owned()],
                paths: vec![TEST_LIVE_PATH.to_owned()],
            },
            RunResourceGrantConfig {
                target: format!("{TEST_LIVE_INSTANCE}/{TEST_LIVE_OUTPUT_CONTEXT}"),
                operations: vec!["write".to_owned()],
                paths: vec![
                    TEST_LIVE_TUPLE_PATH.to_owned(),
                    TEST_LIVE_RECORD_PATH.to_owned(),
                ],
            },
        ],
    }
}

fn compile_fixed_source(source: &str) -> AppResult<(Vec<u8>, Vec<String>)> {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let manifest = repository.join("tests/fixtures/bytecode-v1-fixed-generator/Cargo.toml");
    let temporary = tempfile::tempdir()?;
    let output = temporary.path().join("fixed-matrix-add-f64.mecb");
    let functions = temporary.path().join("functions.json");
    let cargo = env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let command_output = Command::new(cargo)
        .arg("run")
        .arg("--quiet")
        .arg("--locked")
        .arg("--offline")
        .arg("--manifest-path")
        .arg(&manifest)
        .arg("--")
        .arg(&output)
        .arg(&functions)
        .arg(source)
        .env(
            "CARGO_TARGET_DIR",
            repository.join("target/bytecode-v1-fixtures/cargo-target"),
        )
        .output()?;
    if !command_output.status.success() {
        return Err(io::Error::other(format!(
            "isolated fixed-matrix source compiler failed with {}: stdout={} stderr={}",
            command_output.status,
            String::from_utf8_lossy(&command_output.stdout),
            String::from_utf8_lossy(&command_output.stderr),
        ))
        .into());
    }
    let bytes = fs::read(&output)?;
    ParsedProgram::from_bytes(&bytes)
        .map_err(|error| mech_error("fixed-matrix bytecode validation", error))?;
    let runtime_functions = serde_json::from_slice(&fs::read(functions)?)?;
    Ok((bytes, runtime_functions))
}

fn synthetic_live_host_catalog() -> AppResult<Arc<NativeHostCatalog>> {
    fn planning_factory() -> MResult<Box<dyn RuntimeHostFactory>> {
        Ok(Box::new(TestLiveHostFactory::native()?))
    }

    let mut catalog = NativeHostCatalog::new();
    catalog
        .insert_provider(NativeHostLinkage {
            provider: TEST_LIVE_PROVIDER,
            package: "mech-native-live-host-fixture",
            crate_name: "mech_native_live_host_fixture",
            cargo_features: &[],
            factory_path: "mech_native_live_host_fixture::TestLiveHostFactory::native",
            supported_targets: &[NativeTargetFamily::Unix, NativeTargetFamily::Windows],
            manifest: mech_native_live_host_fixture::test_live_manifest,
            validate_settings: mech_native_live_host_fixture::validate_settings,
            planning_factory,
        })
        .map_err(|error| mech_error("synthetic live native host linkage", error))?;
    Ok(Arc::new(catalog))
}

fn native_plan(fixture: &Fixture) -> AppResult<mech_build::NativeBuildPlan> {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()?;
    let function_catalog = match fixture.plan_catalog {
        PlanCatalog::Standard | PlanCatalog::SyntheticLive => mech_stdlib::native_plan_catalog(),
    };
    let host_catalog = match fixture.plan_catalog {
        PlanCatalog::SyntheticLive => synthetic_live_host_catalog()?,
        PlanCatalog::Standard => selected_native_host_catalog()
            .map_err(|error| mech_error("selected native host catalog", error))?,
    };
    let binary_name = fixture
        .file
        .strip_suffix(".mecb")
        .expect("fixture filenames end in .mecb")
        .replace('-', "_");
    let request = NativeBuildRequest {
        bytecode: fixture.bytes.clone(),
        runtime_config: fixture.runtime_config.clone(),
        target: None,
        profile: NativeBuildProfile::Release,
        binary_name: binary_name.clone(),
        output: repository
            .join("target/bytecode-v1-fixtures/native-plans")
            .join(binary_name),
        emit: NativeEmit::Plan,
        keep_project: false,
        offline: true,
    };
    let dependency_source = match fixture.plan_catalog {
        PlanCatalog::Standard => NativeDependencySource::Workspace { root: repository },
        PlanCatalog::SyntheticLive => NativeDependencySource::Registry {
            version: MECH_COMPONENT_VERSION.to_owned(),
        },
    };
    NativeApplicationBuilder::new(NativeBuildEnvironment {
        function_catalog,
        host_catalog,
        dependency_source,
    })
    .plan(&request)
    .map_err(|error| mech_error("fixture native planning", error).into())
}

fn mech_error(context: &str, error: impl std::fmt::Debug) -> io::Error {
    io::Error::other(format!("{context} failed: {error:?}"))
}

fn matrix_json(rows: usize, cols: usize, elements: &[f64]) -> JsonValue {
    debug_assert_eq!(elements.len(), rows * cols);
    JsonValue::Array(
        elements
            .chunks(cols)
            .map(|row| json!(row))
            .collect::<Vec<_>>(),
    )
}

fn write_generated_corpus(output: &Path) -> AppResult<()> {
    fs::create_dir_all(output)?;
    fs::create_dir_all(output.join(SOURCE_DIRECTORY))?;
    let fixtures = fixtures()?;
    if fixtures
        .iter()
        .map(|fixture| fixture.file)
        .collect::<Vec<_>>()
        != FIXTURE_FILES
    {
        return Err(io::Error::other("fixture declaration order or names changed").into());
    }

    let manifest = manifest(&fixtures)?;
    for fixture in &fixtures {
        fs::write(output.join(fixture.file), &fixture.bytes)?;
        if let (Some(source_file), Some(source)) = (fixture.source_file, fixture.source) {
            fs::write(output.join(SOURCE_DIRECTORY).join(source_file), source)?;
        }
    }
    fs::write(
        output.join("manifest.json"),
        serde_json::to_string_pretty(&manifest)? + "\n",
    )?;
    ensure_exact_file_set(output)
}

fn manifest(fixtures: &[Fixture]) -> AppResult<JsonValue> {
    let entries = fixtures
        .iter()
        .map(manifest_entry)
        .collect::<AppResult<Vec<_>>>()?;
    Ok(json!({
        "format": "mech-bytecode-v1",
        "fixtures": entries,
    }))
}

fn manifest_entry(fixture: &Fixture) -> AppResult<JsonValue> {
    let parsed = ParsedProgram::from_bytes(&fixture.bytes).map_err(|error| {
        io::Error::other(format!(
            "{} failed authoritative parsing: {error:?}",
            fixture.file
        ))
    })?;
    let actual_ids = parsed
        .instructions
        .iter()
        .filter_map(BytecodeInstruction::runtime_function)
        .collect::<BTreeSet<_>>();
    let expected_ids = fixture
        .runtime_functions
        .iter()
        .map(|name| hash_str(name))
        .collect::<BTreeSet<_>>();
    if actual_ids != expected_ids {
        return Err(io::Error::other(format!(
            "{} runtime function IDs do not match its declaration",
            fixture.file,
        ))
        .into());
    }

    let header = &parsed.header;
    let runtime_functions = fixture
        .runtime_functions
        .iter()
        .map(|name| {
            let id = hash_str(name);
            json!({
                "name": name,
                "id": id,
                "id_hex": format!("{id:016x}"),
            })
        })
        .collect::<Vec<_>>();
    let runtime_types = parsed
        .types
        .iter()
        .enumerate()
        .map(|(id, runtime_type)| {
            json!({
                "id": id,
                "type": runtime_type_json(runtime_type),
            })
        })
        .collect::<Vec<_>>();
    let plan = native_plan(fixture)?;
    if plan.bytecode_sha256 != sha256(&fixture.bytes) {
        return Err(io::Error::other(format!(
            "{} native plan bytecode digest drifted",
            fixture.file,
        ))
        .into());
    }

    Ok(json!({
        "file": fixture.file,
        "origin": if fixture.source.is_some() { "source-compiler" } else { "constructed-bytecode-program" },
        "source_file": fixture.source_file,
        "source": fixture.source,
        "construction": fixture.construction,
        "sha256": sha256(&fixture.bytes),
        "header": {
            "magic": String::from_utf8_lossy(&header.magic),
            "version": header.version,
            "header_size": header.header_size,
            "mech_major": header.mech_major,
            "mech_minor": header.mech_minor,
            "mech_patch": header.mech_patch,
            "flags": header.flags,
            "register_count": header.register_count,
            "instruction_count": header.instruction_count,
            "section_count": header.section_count,
            "reserved0": header.reserved0,
            "section_table_offset": header.section_table_offset,
            "file_len": header.file_len,
            "checksum_offset": header.checksum_offset,
            "reserved": header.reserved,
        },
        "sections": parsed.sections.iter().map(|section| json!({
            "kind": format!("{:?}", section.kind),
            "id": section.kind as u16,
            "flags": section.flags,
            "item_count": section.item_count,
            "offset": section.offset,
            "length": section.length,
            "reserved": section.reserved,
        })).collect::<Vec<_>>(),
        "runtime_function_ids": runtime_functions,
        "runtime_types": runtime_types,
        "application_requirements": parsed.requirements.iter().map(requirement_json).collect::<Vec<_>>(),
        "native_plan_sha256": plan.plan_sha256,
        "packages": plan.packages.iter().map(|package| json!({
            "package": package.package,
            "crate_name": package.crate_name,
            "source": package.source,
            "cargo_features": package.cargo_features,
        })).collect::<Vec<_>>(),
        "cargo_features": {
            "mech-core": plan.core_features,
            "mech-engine": plan.engine_features,
            "mech-runtime": plan.runtime_features,
        },
        "expected_output": fixture.expected_output,
    }))
}

fn runtime_type_json(runtime_type: &RuntimeType) -> JsonValue {
    match runtime_type {
        RuntimeType::U8 => json!({ "kind": "u8" }),
        RuntimeType::U16 => json!({ "kind": "u16" }),
        RuntimeType::U32 => json!({ "kind": "u32" }),
        RuntimeType::U64 => json!({ "kind": "u64" }),
        RuntimeType::U128 => json!({ "kind": "u128" }),
        RuntimeType::I8 => json!({ "kind": "i8" }),
        RuntimeType::I16 => json!({ "kind": "i16" }),
        RuntimeType::I32 => json!({ "kind": "i32" }),
        RuntimeType::I64 => json!({ "kind": "i64" }),
        RuntimeType::I128 => json!({ "kind": "i128" }),
        RuntimeType::F32 => json!({ "kind": "f32" }),
        RuntimeType::F64 => json!({ "kind": "f64" }),
        RuntimeType::C64 => json!({ "kind": "c64" }),
        RuntimeType::R64 => json!({ "kind": "r64" }),
        RuntimeType::String => json!({ "kind": "string" }),
        RuntimeType::Bool => json!({ "kind": "bool" }),
        RuntimeType::Id => json!({ "kind": "id" }),
        RuntimeType::Index => json!({ "kind": "index" }),
        RuntimeType::Empty => json!({ "kind": "empty" }),
        RuntimeType::Any => json!({ "kind": "any" }),
        RuntimeType::None => json!({ "kind": "none" }),
        RuntimeType::Matrix {
            element,
            storage,
            rows,
            cols,
        } => json!({
            "kind": "matrix",
            "element": runtime_type_json(element),
            "storage": format!("{:?}", storage),
            "storage_id": *storage as u8,
            "rows": rows,
            "cols": cols,
        }),
        RuntimeType::Enum { id, name } => json!({
            "kind": "enum",
            "id": id,
            "name": name,
        }),
        RuntimeType::Record(fields) => json!({
            "kind": "record",
            "fields": fields.iter().map(|(name, ty)| json!({
                "name": name,
                "type": runtime_type_json(ty),
            })).collect::<Vec<_>>(),
        }),
        RuntimeType::Map { key, value } => json!({
            "kind": "map",
            "key": runtime_type_json(key),
            "value": runtime_type_json(value),
        }),
        RuntimeType::Atom { id, name } => json!({
            "kind": "atom",
            "id": id,
            "name": name,
        }),
        RuntimeType::Table {
            columns,
            primary_key,
        } => json!({
            "kind": "table",
            "columns": columns.iter().map(|(name, ty)| json!({
                "name": name,
                "type": runtime_type_json(ty),
            })).collect::<Vec<_>>(),
            "primary_key": primary_key,
        }),
        RuntimeType::Tuple(types) => json!({
            "kind": "tuple",
            "elements": types.iter().map(runtime_type_json).collect::<Vec<_>>(),
        }),
        RuntimeType::Reference(child) => json!({
            "kind": "reference",
            "child": runtime_type_json(child),
        }),
        RuntimeType::Set { element, max_len } => json!({
            "kind": "set",
            "element": runtime_type_json(element),
            "max_len": max_len,
        }),
        RuntimeType::Option(child) => json!({
            "kind": "option",
            "child": runtime_type_json(child),
        }),
        RuntimeType::Kind(_) => json!({ "kind": "kind" }),
    }
}

fn requirement_json(requirement: &ApplicationRequirement) -> JsonValue {
    match requirement {
        ApplicationRequirement::HostFunction(request) => json!({
            "kind": "host-function",
            "name": request.name,
        }),
        ApplicationRequirement::Resource(request) => json!({
            "kind": "resource",
            "base_uri": request.base_uri,
            "path": request.path,
            "context_name": request.context_name,
            "operation": request.operation,
            "intent": format!("{:?}", request.intent),
            "intent_id": request.intent as u8,
            "delivery": format!("{:?}", request.delivery),
            "delivery_id": request.delivery as u8,
        }),
    }
}

fn sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn check_corpus() -> AppResult<()> {
    let determinism_runs = match env::var("MECH_BYTECODE_DETERMINISM_RUNS") {
        Ok(value) => {
            let runs = value.parse::<usize>().map_err(|error| {
                io::Error::other(format!(
                    "invalid MECH_BYTECODE_DETERMINISM_RUNS {value:?}: {error}",
                ))
            })?;
            if runs < 2 {
                return Err(io::Error::other(
                    "MECH_BYTECODE_DETERMINISM_RUNS must be at least 2",
                )
                .into());
            }
            runs
        }
        Err(env::VarError::NotPresent) => DEFAULT_DETERMINISM_RUNS,
        Err(error) => return Err(error.into()),
    };
    let committed = corpus_directory();
    ensure_exact_file_set(&committed)?;
    let executable = env::current_exe()?;
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let temporary = env::temp_dir().join(format!(
        "mech-bytecode-v1-generator-{}-{nonce}",
        process::id(),
    ));
    fs::create_dir(&temporary)?;

    let result: AppResult<()> = (|| -> AppResult<()> {
        let mut first_run: Option<PathBuf> = None;
        for run in 0..determinism_runs {
            let output = temporary.join(format!("run-{run}"));
            let status = Command::new(&executable)
                .arg("--emit")
                .arg(&output)
                .status()?;
            if !status.success() {
                return Err(io::Error::other(format!(
                    "fresh fixture process {run} failed with {status}",
                ))
                .into());
            }
            compare_corpora(&committed, &output)?;
            if let Some(first) = &first_run {
                compare_corpora(first, &output)?;
            } else {
                first_run = Some(output);
            }
        }
        Ok(())
    })();

    let cleanup = fs::remove_dir_all(&temporary);
    result?;
    cleanup?;
    println!(
        "checked {} bytecode v1 fixtures across {determinism_runs} fresh child processes",
        FIXTURE_FILES.len(),
    );
    Ok(())
}

fn compare_corpora(expected: &Path, actual: &Path) -> AppResult<()> {
    ensure_exact_file_set(expected)?;
    ensure_exact_file_set(actual)?;
    for file in FIXTURE_FILES.into_iter().chain(["manifest.json"]) {
        let expected_bytes = fs::read(expected.join(file))?;
        let actual_bytes = fs::read(actual.join(file))?;
        if actual_bytes != expected_bytes {
            return Err(io::Error::other(format!(
                "generated {file} differs between {} and {}",
                expected.display(),
                actual.display(),
            ))
            .into());
        }
    }
    for file in SOURCE_FILES {
        let expected_bytes = fs::read(expected.join(SOURCE_DIRECTORY).join(file))?;
        let actual_bytes = fs::read(actual.join(SOURCE_DIRECTORY).join(file))?;
        if actual_bytes != expected_bytes {
            return Err(io::Error::other(format!(
                "generated source fixture {file} differs between {} and {}",
                expected.display(),
                actual.display(),
            ))
            .into());
        }
    }
    Ok(())
}

fn ensure_exact_file_set(directory: &Path) -> AppResult<()> {
    let mut actual = fs::read_dir(directory)?
        .map(|entry| {
            let entry = entry?;
            let is_source_directory =
                entry.file_name() == SOURCE_DIRECTORY && entry.file_type()?.is_dir();
            if !entry.file_type()?.is_file() && !is_source_directory {
                return Err(io::Error::other(format!(
                    "unexpected non-file corpus entry {}",
                    entry.path().display(),
                )));
            }
            entry
                .file_name()
                .into_string()
                .map_err(|_| io::Error::other("corpus filename is not UTF-8"))
        })
        .collect::<Result<Vec<_>, io::Error>>()?;
    actual.sort();
    let mut expected = FIXTURE_FILES.map(str::to_string).to_vec();
    expected.push("manifest.json".to_owned());
    if directory.join(SOURCE_DIRECTORY).is_dir() {
        let mut source_files = fs::read_dir(directory.join(SOURCE_DIRECTORY))?
            .map(|entry| {
                let entry = entry?;
                if !entry.file_type()?.is_file() {
                    return Err(io::Error::other(format!(
                        "unexpected non-file source fixture {}",
                        entry.path().display(),
                    )));
                }
                entry
                    .file_name()
                    .into_string()
                    .map_err(|_| io::Error::other("source fixture filename is not UTF-8"))
            })
            .collect::<Result<Vec<_>, io::Error>>()?;
        source_files.sort();
        let mut expected_sources = SOURCE_FILES.map(str::to_string).to_vec();
        expected_sources.sort();
        if source_files != expected_sources {
            return Err(io::Error::other(format!(
                "{} must contain source fixtures {:?}, found {:?}",
                directory.join(SOURCE_DIRECTORY).display(),
                expected_sources,
                source_files,
            ))
            .into());
        }
        expected.push(SOURCE_DIRECTORY.to_owned());
    }
    expected.sort();
    if actual != expected {
        return Err(io::Error::other(format!(
            "{} must contain exactly {expected:?}, found {actual:?}",
            directory.display(),
        ))
        .into());
    }
    Ok(())
}
