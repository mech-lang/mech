use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::error::Error;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use std::time::{SystemTime, UNIX_EPOCH};

use mech_core::{
    ApplicationRequirement, BytecodeInstruction, BytecodeProgram, EncodedConstant,
    ExecutionResourceRequest, MatrixStorage, ParsedProgram, ResourceDelivery, ResourceIntent,
    RuntimeType, hash_str, write_bytecode,
};
use serde_json::{Value as JsonValue, json};
use sha2::{Digest, Sha256};

type AppResult<T> = Result<T, Box<dyn Error>>;

const FIXTURE_FILES: [&str; 7] = [
    "literal-f64.mecb",
    "scalar-add-f64.mecb",
    "fixed-matrix-add-f64.mecb",
    "dynamic-matrix-add-f64.mecb",
    "variadic-horzcat-f64.mecb",
    "cli-stdout.mecb",
    "synthetic-live-read.mecb",
];
const CORPUS_FILES: [&str; 8] = [
    "literal-f64.mecb",
    "scalar-add-f64.mecb",
    "fixed-matrix-add-f64.mecb",
    "dynamic-matrix-add-f64.mecb",
    "variadic-horzcat-f64.mecb",
    "cli-stdout.mecb",
    "synthetic-live-read.mecb",
    "manifest.json",
];
const DETERMINISM_RUNS: usize = 5;

struct Fixture {
    file: &'static str,
    source: &'static str,
    bytes: Vec<u8>,
    runtime_functions: &'static [&'static str],
    expected_result: JsonValue,
}

fn main() -> AppResult<()> {
    let args = env::args_os().skip(1).collect::<Vec<_>>();
    match args.as_slice() {
        [mode] if mode == "--write" => {
            let output = corpus_directory();
            write_generated_corpus(&output)?;
            println!("wrote bytecode v1 Phase 1 corpus to {}", output.display());
            Ok(())
        }
        [mode] if mode == "--check" => check_corpus(),
        [mode, output] if mode == "--emit" => write_generated_corpus(Path::new(output)),
        _ => Err(io::Error::other("usage: bytecode-v1-generator (--write | --check)").into()),
    }
}

fn corpus_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../architecture/bytecode-v1/phase1")
}

fn fixtures() -> AppResult<Vec<Fixture>> {
    let scalar_add = hash_str("AddSS<f64>");
    let fixed_add = hash_str("AddM2M2<f64>");
    let dynamic_add = hash_str("AddMDMD<f64>");
    let variadic_horzcat = hash_str("HorizontalConcatenateNArgs");

    let fixed_left = vec![1.0, 2.0, 3.0, 4.0];
    let fixed_right = vec![5.0, 6.0, 7.0, 8.0];
    let fixed_output = vec![6.0, 8.0, 10.0, 12.0];

    let dynamic_left = (1..=25).map(f64::from).collect::<Vec<_>>();
    let dynamic_right = (1..=25).rev().map(f64::from).collect::<Vec<_>>();
    let dynamic_output = vec![26.0; 25];

    Ok(vec![
        Fixture {
            file: "literal-f64.mecb",
            source: "42.0",
            bytes: compile_program(
                vec![f64_constant(42.0)],
                vec![BytecodeInstruction::ConstLoad {
                    dst: 0,
                    constant: 0,
                }],
                Vec::new(),
                0,
            )?,
            runtime_functions: &[],
            expected_result: json!(42.0),
        },
        Fixture {
            file: "scalar-add-f64.mecb",
            source: "1.0 + 2.0",
            bytes: compile_program(
                vec![f64_constant(3.0), f64_constant(1.0), f64_constant(2.0)],
                vec![
                    const_load(0, 0),
                    const_load(1, 1),
                    const_load(2, 2),
                    BytecodeInstruction::RuntimeBinary {
                        function: scalar_add,
                        dst: 0,
                        lhs: 1,
                        rhs: 2,
                    },
                ],
                Vec::new(),
                0,
            )?,
            runtime_functions: &["AddSS<f64>"],
            expected_result: json!(3.0),
        },
        Fixture {
            file: "fixed-matrix-add-f64.mecb",
            source: "[1.0 2.0; 3.0 4.0] + [5.0 6.0; 7.0 8.0]",
            bytes: compile_program(
                vec![
                    matrix_constant(MatrixStorage::Matrix2, 2, 2, &fixed_output)?,
                    matrix_constant(MatrixStorage::Matrix2, 2, 2, &fixed_left)?,
                    matrix_constant(MatrixStorage::Matrix2, 2, 2, &fixed_right)?,
                ],
                vec![
                    const_load(0, 0),
                    const_load(1, 1),
                    const_load(2, 2),
                    BytecodeInstruction::RuntimeBinary {
                        function: fixed_add,
                        dst: 0,
                        lhs: 1,
                        rhs: 2,
                    },
                ],
                Vec::new(),
                0,
            )?,
            runtime_functions: &["AddM2M2<f64>"],
            expected_result: matrix_json(2, 2, &fixed_output),
        },
        Fixture {
            file: "dynamic-matrix-add-f64.mecb",
            source: concat!(
                "left := [1.0 2.0 3.0 4.0 5.0; 6.0 7.0 8.0 9.0 10.0; ",
                "11.0 12.0 13.0 14.0 15.0; 16.0 17.0 18.0 19.0 20.0; ",
                "21.0 22.0 23.0 24.0 25.0]\n",
                "right := [25.0 24.0 23.0 22.0 21.0; 20.0 19.0 18.0 17.0 16.0; ",
                "15.0 14.0 13.0 12.0 11.0; 10.0 9.0 8.0 7.0 6.0; ",
                "5.0 4.0 3.0 2.0 1.0]\n",
                "left + right",
            ),
            bytes: compile_program(
                vec![
                    matrix_constant(MatrixStorage::MatrixD, 5, 5, &dynamic_output)?,
                    matrix_constant(MatrixStorage::MatrixD, 5, 5, &dynamic_left)?,
                    matrix_constant(MatrixStorage::MatrixD, 5, 5, &dynamic_right)?,
                ],
                vec![
                    const_load(0, 0),
                    const_load(1, 1),
                    const_load(2, 2),
                    BytecodeInstruction::RuntimeBinary {
                        function: dynamic_add,
                        dst: 0,
                        lhs: 1,
                        rhs: 2,
                    },
                ],
                Vec::new(),
                0,
            )?,
            runtime_functions: &["AddMDMD<f64>"],
            expected_result: matrix_json(5, 5, &dynamic_output),
        },
        Fixture {
            file: "variadic-horzcat-f64.mecb",
            source: "[1.0 2.0 3.0 4.0 5.0]",
            bytes: compile_program(
                vec![
                    matrix_constant(MatrixStorage::MatrixD, 1, 5, &[1.0, 2.0, 3.0, 4.0, 5.0])?,
                    f64_constant(1.0),
                    f64_constant(2.0),
                    f64_constant(3.0),
                    f64_constant(4.0),
                    f64_constant(5.0),
                ],
                vec![
                    const_load(0, 0),
                    const_load(1, 1),
                    const_load(2, 2),
                    const_load(3, 3),
                    const_load(4, 4),
                    const_load(5, 5),
                    BytecodeInstruction::RuntimeVariadic {
                        function: variadic_horzcat,
                        dst: 0,
                        arguments: vec![1, 2, 3, 4, 5],
                    },
                ],
                Vec::new(),
                0,
            )?,
            runtime_functions: &["HorizontalConcatenateNArgs"],
            expected_result: matrix_json(1, 5, &[1.0, 2.0, 3.0, 4.0, 5.0]),
        },
        Fixture {
            file: "cli-stdout.mecb",
            source: "+> @out := cli/stdout\n\n@out/line <- \"phase1-hosted-ok\"\n\n\"done\"",
            bytes: compile_program(
                vec![
                    empty_constant(),
                    string_constant("phase1-hosted-ok"),
                    string_constant("done"),
                ],
                vec![
                    const_load(0, 0),
                    const_load(1, 1),
                    const_load(2, 2),
                    BytecodeInstruction::ResourceSend {
                        requirement: 0,
                        dst: 0,
                        src: 1,
                    },
                ],
                vec![ApplicationRequirement::Resource(ExecutionResourceRequest {
                    base_uri: "cli://cli/stdout".to_string(),
                    path: "line".to_string(),
                    context_name: "out".to_string(),
                    operation: "write".to_string(),
                    intent: ResourceIntent::Send,
                    delivery: ResourceDelivery::Snapshot,
                })],
                2,
            )?,
            runtime_functions: &[],
            expected_result: json!("done"),
        },
        Fixture {
            file: "synthetic-live-read.mecb",
            source: concat!(
                "+> @clock := test-live/clock\n\n",
                "value := @clock/value\n",
                "doubled := value + value\n\n",
                "doubled",
            ),
            bytes: compile_program(
                vec![f64_constant(0.0), f64_constant(0.0)],
                vec![
                    const_load(0, 0),
                    const_load(1, 1),
                    BytecodeInstruction::ResourceRead {
                        requirement: 0,
                        dst: 0,
                    },
                    BytecodeInstruction::RuntimeBinary {
                        function: scalar_add,
                        dst: 1,
                        lhs: 0,
                        rhs: 0,
                    },
                ],
                vec![ApplicationRequirement::Resource(ExecutionResourceRequest {
                    base_uri: "test-live://clock/clock".to_string(),
                    path: "value".to_string(),
                    context_name: "clock".to_string(),
                    operation: "read".to_string(),
                    intent: ResourceIntent::Read,
                    delivery: ResourceDelivery::Live,
                })],
                1,
            )?,
            runtime_functions: &["AddSS<f64>"],
            expected_result: json!(0.0),
        },
    ])
}

fn compile_program(
    constants: Vec<EncodedConstant>,
    mut instructions: Vec<BytecodeInstruction>,
    requirements: Vec<ApplicationRequirement>,
    return_register: u32,
) -> AppResult<Vec<u8>> {
    let register_count = constants
        .len()
        .try_into()
        .map_err(|_| io::Error::other("fixture has too many registers"))?;
    instructions.push(BytecodeInstruction::Return {
        src: return_register,
    });
    let program = BytecodeProgram {
        register_count,
        constants,
        symbols: BTreeMap::new(),
        mutable_symbols: BTreeSet::new(),
        instructions,
        dictionary: BTreeMap::new(),
        requirements,
    };
    let bytes = write_bytecode(&program)
        .map_err(|error| io::Error::other(format!("bytecode writer failed: {error:?}")))?;
    ParsedProgram::from_bytes(&bytes)
        .map_err(|error| io::Error::other(format!("bytecode reader rejected output: {error:?}")))?;
    Ok(bytes)
}

fn const_load(dst: u32, constant: u32) -> BytecodeInstruction {
    BytecodeInstruction::ConstLoad { dst, constant }
}

fn f64_constant(value: f64) -> EncodedConstant {
    EncodedConstant {
        runtime_type: RuntimeType::F64,
        alignment: 8,
        bytes: value.to_bits().to_le_bytes().to_vec(),
    }
}

fn string_constant(value: &str) -> EncodedConstant {
    EncodedConstant {
        runtime_type: RuntimeType::String,
        alignment: 1,
        bytes: value.as_bytes().to_vec(),
    }
}

fn empty_constant() -> EncodedConstant {
    EncodedConstant {
        runtime_type: RuntimeType::Empty,
        alignment: 1,
        bytes: Vec::new(),
    }
}

fn matrix_constant(
    storage: MatrixStorage,
    rows: u32,
    cols: u32,
    elements: &[f64],
) -> AppResult<EncodedConstant> {
    let expected = usize::try_from(rows)?
        .checked_mul(usize::try_from(cols)?)
        .ok_or_else(|| io::Error::other("matrix fixture dimensions overflow"))?;
    if elements.len() != expected || !storage.validate_dimensions(rows, cols) {
        return Err(io::Error::other("invalid matrix fixture shape").into());
    }
    let mut bytes = Vec::with_capacity(8 + elements.len() * 8);
    bytes.extend_from_slice(&rows.to_le_bytes());
    bytes.extend_from_slice(&cols.to_le_bytes());
    for element in elements {
        bytes.extend_from_slice(&element.to_bits().to_le_bytes());
    }
    Ok(EncodedConstant {
        runtime_type: RuntimeType::Matrix {
            element: Box::new(RuntimeType::F64),
            storage,
            rows,
            cols,
        },
        alignment: 8,
        bytes,
    })
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
        "phase": 1,
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

    Ok(json!({
        "file": fixture.file,
        "source": fixture.source,
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
        "expected_result": fixture.expected_result,
    }))
}

fn runtime_type_json(runtime_type: &RuntimeType) -> JsonValue {
    match runtime_type {
        RuntimeType::F64 => json!({ "kind": "f64" }),
        RuntimeType::String => json!({ "kind": "string" }),
        RuntimeType::Empty => json!({ "kind": "empty" }),
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
        other => json!({
            "kind": "other",
            "debug": format!("{other:?}"),
        }),
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
        for run in 0..DETERMINISM_RUNS {
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
    println!("checked seven bytecode v1 fixtures across {DETERMINISM_RUNS} fresh child processes",);
    Ok(())
}

fn compare_corpora(expected: &Path, actual: &Path) -> AppResult<()> {
    ensure_exact_file_set(expected)?;
    ensure_exact_file_set(actual)?;
    for file in CORPUS_FILES {
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
    Ok(())
}

fn ensure_exact_file_set(directory: &Path) -> AppResult<()> {
    let mut actual = fs::read_dir(directory)?
        .map(|entry| {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
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
    let mut expected = CORPUS_FILES.map(str::to_string).to_vec();
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
