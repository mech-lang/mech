use mech_core::{DecodedInstr, ParsedProgram, Value, hash_str};
use mech_interpreter as _;
use mech_math as _;
use mech_engine::{MechProgram, MechProgramConfig};
use mech_range as _;
use mech_string as _;
use serde::Deserialize;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

const MANIFEST_SCHEMA: u64 = 1;
const BYTECODE_VERSION: u64 = 1;
const BASE_COMMIT: &str = "f7768e0c6bbde69d27410be6d1ecacbd08c238c5";

#[derive(Deserialize)]
struct Manifest {
    schema: u64,
    base_commit: String,
    bytecode_version: u64,
    cases: Vec<Case>,
}

#[derive(Deserialize)]
struct Case {
    name: String,
    file: String,
    expected: Expected,
    runtime_factory_ids: Vec<RuntimeFactory>,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
struct RuntimeFactory {
    name: String,
    id_hex: String,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Expected {
    F64 {
        value: f64,
    },
    Bool {
        value: bool,
    },
    String {
        value: String,
    },
    MatrixF64 {
        rows: usize,
        cols: usize,
        values: Vec<f64>,
    },
}

fn main() {
    if let Err(error) = run() {
        eprintln!("function-system bytecode consumer failed: {error}");
        process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let corpus_dir =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../architecture/legacy-bytecode");
    let manifest_path = corpus_dir.join("manifest.json");
    let manifest_bytes = fs::read(&manifest_path)
        .map_err(|error| format!("could not read {}: {error}", manifest_path.display()))?;
    let manifest: Manifest = serde_json::from_slice(&manifest_bytes)
        .map_err(|error| format!("could not parse {}: {error}", manifest_path.display()))?;

    if manifest.schema != MANIFEST_SCHEMA {
        return Err(format!(
            "manifest schema mismatch: expected {MANIFEST_SCHEMA}, found {}",
            manifest.schema,
        ));
    }
    if manifest.base_commit != BASE_COMMIT {
        return Err(format!(
            "manifest base commit mismatch: expected {BASE_COMMIT}, found {}",
            manifest.base_commit,
        ));
    }
    if manifest.bytecode_version != BYTECODE_VERSION {
        return Err(format!(
            "manifest bytecode version mismatch: expected {BYTECODE_VERSION}, found {}",
            manifest.bytecode_version,
        ));
    }
    if manifest.cases.is_empty() {
        return Err("manifest contains no bytecode cases".to_string());
    }

    validate_corpus_files(&corpus_dir, &manifest.cases)?;
    for case in manifest.cases {
        let bytecode_path = corpus_dir.join(&case.file);
        let bytecode = fs::read(&bytecode_path)
            .map_err(|error| format!("{}: could not read {}: {error}", case.name, case.file))?;
        let parsed = ParsedProgram::from_bytes(&bytecode)
            .map_err(|error| format!("{}: invalid bytecode: {error:?}", case.name))?;
        let mut program = MechProgram::new(MechProgramConfig::default());
        program.load_full_stdlib();
        validate_runtime_factories(&case, &parsed, &program)?;

        let actual = program
            .run_bytecode_program(&parsed)
            .map_err(|error| format!("{}: bytecode execution failed: {error:?}", case.name))?;

        compare_result(&case.name, &actual, &case.expected)?;
    }

    Ok(())
}

fn validate_runtime_factories(
    case: &Case,
    parsed: &ParsedProgram,
    program: &MechProgram,
) -> Result<(), String> {
    if u64::from(parsed.header.version) != BYTECODE_VERSION {
        return Err(format!(
            "{}: bytecode version mismatch: expected {BYTECODE_VERSION}, found {}",
            case.name, parsed.header.version,
        ));
    }

    let functions_ref = program.interpreter().functions();
    let functions = functions_ref.borrow();
    let dictionary = functions.dictionary.borrow();
    let mut actual = Vec::new();
    for instruction in &parsed.instrs {
        let id = match instruction {
            DecodedInstr::NullOp { fxn_id, .. }
            | DecodedInstr::UnOp { fxn_id, .. }
            | DecodedInstr::BinOp { fxn_id, .. }
            | DecodedInstr::TernOp { fxn_id, .. }
            | DecodedInstr::QuadOp { fxn_id, .. }
            | DecodedInstr::VarArg { fxn_id, .. } => *fxn_id,
            DecodedInstr::ConstLoad { .. } | DecodedInstr::Ret { .. } => continue,
            DecodedInstr::Unknown { opcode, .. } => {
                return Err(format!(
                    "{}: bytecode contains unknown opcode {opcode:#04x}",
                    case.name,
                ));
            }
        };
        if !functions.functions.contains_key(&id) {
            return Err(format!(
                "{}: bytecode references runtime factory ID {} that is not loaded",
                case.name,
                format_id(id),
            ));
        }
        let name = dictionary.get(&id).ok_or_else(|| {
            format!(
                "{}: loaded runtime factory ID {} has no dictionary name",
                case.name,
                format_id(id),
            )
        })?;
        if hash_str(name) != id {
            return Err(format!(
                "{}: runtime factory name {:?} does not hash to ID {}",
                case.name,
                name,
                format_id(id),
            ));
        }
        actual.push(RuntimeFactory {
            name: name.clone(),
            id_hex: format_id(id),
        });
    }
    if actual.is_empty() {
        return Err(format!(
            "{}: bytecode contains no operation instruction",
            case.name,
        ));
    }
    actual.sort();

    if actual != case.runtime_factory_ids {
        return Err(format!(
            "{}: runtime factory manifest mismatch: expected {:?}, found {:?}",
            case.name, case.runtime_factory_ids, actual,
        ));
    }

    Ok(())
}

fn format_id(id: u64) -> String {
    format!("{id:016x}")
}

fn validate_corpus_files(corpus_dir: &Path, cases: &[Case]) -> Result<(), String> {
    let mut manifest_files = BTreeSet::new();
    let mut case_names = BTreeSet::new();
    for case in cases {
        if !case_names.insert(case.name.as_str()) {
            return Err(format!(
                "manifest contains duplicate case name {:?}",
                case.name
            ));
        }

        let file_path = Path::new(&case.file);
        if file_path.file_name().and_then(|name| name.to_str()) != Some(case.file.as_str())
            || file_path
                .extension()
                .and_then(|extension| extension.to_str())
                != Some("mecb")
        {
            return Err(format!(
                "{}: bytecode file must be a .mecb basename, found {:?}",
                case.name, case.file,
            ));
        }
        if !manifest_files.insert(case.file.clone()) {
            return Err(format!(
                "manifest lists bytecode file {:?} more than once",
                case.file
            ));
        }
    }

    let directory_files = fs::read_dir(corpus_dir)
        .map_err(|error| format!("could not read {}: {error}", corpus_dir.display()))?
        .map(|entry| {
            entry
                .map_err(|error| format!("could not read corpus directory entry: {error}"))
                .map(|entry| entry.path())
        })
        .collect::<Result<Vec<PathBuf>, String>>()?
        .into_iter()
        .filter(|path| path.extension().and_then(|extension| extension.to_str()) == Some("mecb"))
        .map(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(str::to_owned)
                .ok_or_else(|| format!("bytecode file name is not valid UTF-8: {}", path.display()))
        })
        .collect::<Result<BTreeSet<_>, String>>()?;

    if directory_files != manifest_files {
        let unlisted = directory_files
            .difference(&manifest_files)
            .cloned()
            .collect::<Vec<_>>();
        let missing = manifest_files
            .difference(&directory_files)
            .cloned()
            .collect::<Vec<_>>();
        return Err(format!(
            "manifest/corpus file mismatch (unlisted: {unlisted:?}, missing: {missing:?})",
        ));
    }

    Ok(())
}

fn compare_result(name: &str, actual: &Value, expected: &Expected) -> Result<(), String> {
    match expected {
        Expected::F64 { value } => {
            let Value::F64(found) = actual else {
                return Err(format!("{name}: expected f64, found {:?}", actual.kind()));
            };
            let found = *found.borrow();
            if found != *value {
                return Err(format!("{name}: expected f64 {value:?}, found {found:?}"));
            }
        }
        Expected::Bool { value } => {
            let Value::Bool(found) = actual else {
                return Err(format!("{name}: expected bool, found {:?}", actual.kind()));
            };
            let found = *found.borrow();
            if found != *value {
                return Err(format!("{name}: expected bool {value:?}, found {found:?}"));
            }
        }
        Expected::String { value } => {
            let Value::String(found) = actual else {
                return Err(format!(
                    "{name}: expected string, found {:?}",
                    actual.kind()
                ));
            };
            let found = found.borrow().clone();
            if found != *value {
                return Err(format!(
                    "{name}: expected string {value:?}, found {found:?}"
                ));
            }
        }
        Expected::MatrixF64 { rows, cols, values } => {
            let Value::MatrixF64(found) = actual else {
                return Err(format!(
                    "{name}: expected matrix_f64, found {:?}",
                    actual.kind(),
                ));
            };
            let (found_rows, found_cols, found_values) =
                (found.rows(), found.cols(), found.as_vec());
            if (found_rows, found_cols) != (*rows, *cols) || found_values != *values {
                return Err(format!(
                    "{name}: expected matrix_f64 {rows}x{cols} {values:?}, found {found_rows}x{found_cols} {found_values:?}",
                ));
            }
        }
    }

    Ok(())
}
