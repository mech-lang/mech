use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::error::Error;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use std::time::{SystemTime, UNIX_EPOCH};

use mech_core::{
    ApplicationRequirement, BytecodeInstruction, ModuleManifestConfig, ModuleManifestExportConfig,
    ModuleManifestExportKind, ParsedProgram, RuntimeType, hash_str,
};
use mech_engine::{MechProgram, MechProgramConfig};
use mech_host_cli::CliHostFactory;
use mech_native_live_host_fixture::{
    TEST_LIVE_BASE_URI, TEST_LIVE_CONTEXT, TEST_LIVE_INSTANCE, TEST_LIVE_PATH, TEST_LIVE_PROVIDER,
    TestLiveHostFactory, empty_settings,
};
use mech_runtime::{ConfigValue, HostInstanceConfig, RunResourceGrantConfig, RuntimeBuilder};
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
const CLI_SOURCE: &str = "+> @out := cli/stdout\n\n@out/line <- \"phase1-hosted-ok\"\n\n\"done\"";
const LIVE_SOURCE: &str = concat!(
    "+> @clock := test-live/clock\n\n",
    "value := @clock/value\n",
    "doubled := value + value\n\n",
    "doubled",
);

struct Fixture {
    file: &'static str,
    source: &'static str,
    bytes: Vec<u8>,
    runtime_functions: Vec<String>,
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
    let fixed_output = vec![6.0, 8.0, 10.0, 12.0];
    let dynamic_output = vec![26.0; 25];

    let literal = compile_standard_source(LITERAL_SOURCE)?;
    let scalar = compile_standard_source(SCALAR_SOURCE)?;
    let (fixed, fixed_runtime_functions) = compile_fixed_source(FIXED_MATRIX_SOURCE)?;
    let dynamic = compile_standard_source(DYNAMIC_MATRIX_SOURCE)?;
    let variadic = compile_standard_source(VARIADIC_SOURCE)?;
    let cli = compile_cli_planning_source(CLI_SOURCE)?;
    let live = compile_live_planning_source(LIVE_SOURCE)?;

    Ok(vec![
        Fixture {
            file: "literal-f64.mecb",
            source: LITERAL_SOURCE,
            runtime_functions: runtime_function_names(&literal)?,
            bytes: literal,
            expected_result: json!(42.0),
        },
        Fixture {
            file: "scalar-add-f64.mecb",
            source: SCALAR_SOURCE,
            runtime_functions: runtime_function_names(&scalar)?,
            bytes: scalar,
            expected_result: json!(3.0),
        },
        Fixture {
            file: "fixed-matrix-add-f64.mecb",
            source: FIXED_MATRIX_SOURCE,
            runtime_functions: fixed_runtime_functions,
            bytes: fixed,
            expected_result: matrix_json(2, 2, &fixed_output),
        },
        Fixture {
            file: "dynamic-matrix-add-f64.mecb",
            source: DYNAMIC_MATRIX_SOURCE,
            runtime_functions: runtime_function_names(&dynamic)?,
            bytes: dynamic,
            expected_result: matrix_json(5, 5, &dynamic_output),
        },
        Fixture {
            file: "variadic-horzcat-f64.mecb",
            source: VARIADIC_SOURCE,
            runtime_functions: runtime_function_names(&variadic)?,
            bytes: variadic,
            expected_result: matrix_json(1, 5, &[1.0, 2.0, 3.0, 4.0, 5.0]),
        },
        Fixture {
            file: "cli-stdout.mecb",
            source: CLI_SOURCE,
            runtime_functions: runtime_function_names(&cli)?,
            bytes: cli,
            expected_result: json!("done"),
        },
        Fixture {
            file: "synthetic-live-read.mecb",
            source: LIVE_SOURCE,
            runtime_functions: runtime_function_names(&live)?,
            bytes: live,
            expected_result: json!(0.0),
        },
    ])
}

fn compile_standard_source(source: &str) -> AppResult<Vec<u8>> {
    let mut program = MechProgram::with_function_catalog(
        MechProgramConfig::default(),
        mech_stdlib::source_catalog(),
    );
    program
        .run_string(source)
        .map_err(|error| mech_error("standard source execution", error))?;
    program
        .compile_bytecode()
        .map_err(|error| mech_error("standard bytecode compilation", error).into())
}

fn compile_cli_planning_source(source: &str) -> AppResult<Vec<u8>> {
    let builder = RuntimeBuilder::new()
        .planning()
        .function_catalog(mech_stdlib::source_catalog())
        .host_factory(Box::new(
            CliHostFactory::new().map_err(|error| mech_error("CLI host construction", error))?,
        ))
        .map_err(|error| mech_error("CLI host registration", error))?
        .host_instance(HostInstanceConfig {
            name: "cli".to_owned(),
            provider: "cli".to_owned(),
            settings: ConfigValue::Map(BTreeMap::new()),
        })
        .run_resource_grant(RunResourceGrantConfig {
            target: "cli/stdout".to_owned(),
            operations: vec!["write".to_owned()],
            paths: vec!["line".to_owned()],
        });
    compile_planning_source(builder, source)
}

fn compile_live_planning_source(source: &str) -> AppResult<Vec<u8>> {
    let (factory, _driver) =
        TestLiveHostFactory::new().map_err(|error| mech_error("live host construction", error))?;
    let builder = RuntimeBuilder::new()
        .planning()
        .function_catalog(mech_stdlib::source_catalog())
        .module_manifest(ModuleManifestConfig {
            name: TEST_LIVE_PROVIDER.to_owned(),
            exports: vec![ModuleManifestExportConfig {
                name: TEST_LIVE_CONTEXT.to_owned(),
                kind: ModuleManifestExportKind::Context,
                base_uri: TEST_LIVE_BASE_URI.to_owned(),
                operations: vec!["read".to_owned()],
            }],
        })
        .map_err(|error| mech_error("live module manifest registration", error))?
        .host_factory(Box::new(factory))
        .map_err(|error| mech_error("live host registration", error))?
        .host_instance(HostInstanceConfig {
            name: TEST_LIVE_INSTANCE.to_owned(),
            provider: TEST_LIVE_PROVIDER.to_owned(),
            settings: empty_settings(),
        })
        .run_resource_grant(RunResourceGrantConfig {
            target: format!("{TEST_LIVE_INSTANCE}/{TEST_LIVE_CONTEXT}"),
            operations: vec!["read".to_owned()],
            paths: vec![TEST_LIVE_PATH.to_owned()],
        });
    compile_planning_source(builder, source)
}

fn compile_planning_source(builder: RuntimeBuilder, source: &str) -> AppResult<Vec<u8>> {
    let mut runtime = builder
        .build()
        .map_err(|error| mech_error("planning runtime construction", error))?;
    runtime
        .run_string(source)
        .map_err(|error| mech_error("planning source execution", error))?;
    runtime
        .compile_program_bytecode()
        .map_err(|error| mech_error("planning bytecode compilation", error).into())
}

fn compile_fixed_source(source: &str) -> AppResult<(Vec<u8>, Vec<String>)> {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let manifest = repository.join("tests/fixtures/bytecode-v1-fixed-generator/Cargo.toml");
    let temporary = tempfile::tempdir()?;
    let output = temporary.path().join("fixed-matrix-add-f64.mecb");
    let functions = temporary.path().join("functions.json");
    let cargo = env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let status = Command::new(cargo)
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
            repository.join("target/phase1-fixtures/cargo-target"),
        )
        .status()?;
    if !status.success() {
        return Err(io::Error::other(format!(
            "isolated fixed-matrix source compiler failed with {status}",
        ))
        .into());
    }
    let bytes = fs::read(&output)?;
    ParsedProgram::from_bytes(&bytes)
        .map_err(|error| mech_error("fixed-matrix bytecode validation", error))?;
    let runtime_functions = serde_json::from_slice(&fs::read(functions)?)?;
    Ok((bytes, runtime_functions))
}

fn runtime_function_names(bytes: &[u8]) -> AppResult<Vec<String>> {
    let parsed = ParsedProgram::from_bytes(bytes)
        .map_err(|error| mech_error("runtime-function inspection", error))?;
    let catalog = mech_stdlib::runtime_catalog();
    parsed
        .instructions
        .iter()
        .filter_map(BytecodeInstruction::runtime_function)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|id| {
            catalog
                .runtime_entries()
                .find(|entry| entry.id.raw() == id)
                .map(|entry| entry.name.clone())
                .ok_or_else(|| {
                    io::Error::other(format!(
                        "real source compiler emitted ID {id:016x} absent from the ordinary runtime catalog",
                    ))
                    .into()
                })
        })
        .collect()
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
        "origin": "source-compiler",
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
