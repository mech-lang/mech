use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::error::Error;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use mech_build::{
    NativeApplicationBuilder, NativeBuildEnvironment, NativeBuildPlan, NativeBuildProfile,
    NativeBuildRequest, NativeDependencySource, NativeEmit, NativeRuntimeConfig,
};
use mech_core::{
    BytecodeInstruction, BytecodeProgram, EncodedConstant, FunctionCatalog, ParsedProgram,
    RuntimeType, write_bytecode,
};
#[cfg(feature = "fixed")]
use mech_core::FunctionCatalogBuilder;
use mech_runtime::{ConfigValue, HostInstanceConfig, RunResourceGrantConfig, RuntimeConfig};
use serde::Serialize;

type AppResult<T> = Result<T, Box<dyn Error>>;

#[derive(Serialize)]
struct RunnerResult {
    plan: NativeBuildPlan,
    project_root: Option<PathBuf>,
    cargo_manifest: Option<String>,
    build_plan_json: Option<String>,
    catalog_source: Option<String>,
    runtime_source: Option<String>,
    executable: Option<PathBuf>,
    stdout: Option<String>,
    poisoned_output_seed: bool,
    poisoned_output_seed_count: usize,
}

fn main() -> AppResult<()> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    let [action, case, bytecode_path, binary_name, poison] = arguments.as_slice() else {
        return Err("usage: native-build-owner-runner <plan|generate|build|build-only> <case> <bytecode> <binary> <raw|poison>".into());
    };
    if action != "plan" && action != "generate" && action != "build" && action != "build-only" {
        return Err(format!("unknown runner action `{action}`").into());
    }
    if poison != "raw" && poison != "poison" {
        return Err(format!("unknown seed mode `{poison}`").into());
    }

    let workspace = workspace_root()?;
    let mut bytecode = fs::read(bytecode_path)?;
    let poisoned_output_seed = poison == "poison";
    let poisoned_output_seed_count = if poisoned_output_seed {
        let (poisoned, count) = poison_runtime_output_seeds(bytecode)?;
        bytecode = poisoned;
        count
    } else {
        0
    };
    let request = request(&workspace, case, binary_name, bytecode);
    let builder = NativeApplicationBuilder::new(NativeBuildEnvironment {
        function_catalog: owner_catalog()?,
        host_catalog: mech_build::standard_native_host_catalog()
            .map_err(|error| mech_error("native host catalog", error))?,
        dependency_source: NativeDependencySource::Workspace {
            root: workspace.clone(),
        },
    });
    let plan = builder
        .plan(&request)
        .map_err(|error| mech_error("native plan", error))?;

    let mut result = RunnerResult {
        plan,
        project_root: None,
        cargo_manifest: None,
        build_plan_json: None,
        catalog_source: None,
        runtime_source: None,
        executable: None,
        stdout: None,
        poisoned_output_seed,
        poisoned_output_seed_count,
    };
    if action == "generate" || action == "build" || action == "build-only" {
        let project = builder
            .generate(&request, &result.plan)
            .map_err(|error| mech_error("native project generation", error))?;
        result.project_root = Some(project.root.clone());
        result.cargo_manifest = Some(project.cargo_manifest.clone());
        result.build_plan_json = Some(project.build_plan_json.clone());
        result.catalog_source = project.sources.get("src/catalog.rs").cloned();
        result.runtime_source = project.sources.get("src/runtime.rs").cloned();
    }
    if action == "build" || action == "build-only" {
        let artifact = builder
            .build(&request, &result.plan)
            .map_err(|error| mech_error("native project build", error))?;
        result.executable = Some(artifact.executable().to_owned());
        if action == "build-only" {
            serde_json::to_writer(std::io::stdout(), &result)?;
            return Ok(());
        }
        let mut command = Command::new(artifact.executable());
        if case.ends_with("-once") {
            command.arg("--once");
        }
        let output = command.output()?;
        if !output.status.success() {
            return Err(format!(
                "generated binary failed with {}: stdout={} stderr={}",
                output.status,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
            )
            .into());
        }
        result.stdout = Some(String::from_utf8(output.stdout)?);
    }
    serde_json::to_writer(std::io::stdout(), &result)?;
    Ok(())
}

fn workspace_root() -> AppResult<PathBuf> {
    Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()?)
}

fn request(
    workspace: &Path,
    case: &str,
    binary_name: &str,
    bytecode: Vec<u8>,
) -> NativeBuildRequest {
    NativeBuildRequest {
        bytecode,
        runtime_config: runtime_config(case),
        target: None,
        profile: NativeBuildProfile::Debug,
        binary_name: binary_name.to_owned(),
        output: workspace
            .join("target/mech-native/test-output")
            .join(binary_name),
        emit: NativeEmit::Native,
        keep_project: true,
        offline: true,
    }
}

fn runtime_config(case: &str) -> Option<NativeRuntimeConfig> {
    match case {
        "cli" => Some(cli_runtime_config()),
        "console" => Some(single_host_runtime_config(
            "console",
            "console",
            ConfigValue::Map(BTreeMap::new()),
            "console/output",
            &["write"],
            &["line"],
        )),
        "time-once" => Some(single_host_runtime_config(
            "clock",
            "time",
            ConfigValue::Map(BTreeMap::new()),
            "clock/clock",
            &["read"],
            &["second"],
        )),
        "timer-once" => Some(single_host_runtime_config(
            "timer",
            "timer",
            ConfigValue::Map(BTreeMap::new()),
            "timer/tick",
            &["read"],
            &["tick"],
        )),
        "scene" => Some(single_host_runtime_config(
            "scene",
            "scene",
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
            "scene/frame",
            &["write"],
            &["replace"],
        )),
        "robot-arm" => Some(single_host_runtime_config(
            "arm",
            "robot-arm",
            ConfigValue::Map(BTreeMap::new()),
            "arm/commands",
            &["move"],
            &["move"],
        )),
        _ => None,
    }
}

fn single_host_runtime_config(
    instance: &str,
    provider: &str,
    settings: ConfigValue,
    target: &str,
    operations: &[&str],
    paths: &[&str],
) -> NativeRuntimeConfig {
    NativeRuntimeConfig {
        runtime: RuntimeConfig::new(format!("generated-{provider}-runtime")),
        hosts: vec![HostInstanceConfig {
            name: instance.to_owned(),
            provider: provider.to_owned(),
            settings,
        }],
        run_grants: vec![RunResourceGrantConfig {
            target: target.to_owned(),
            operations: operations.iter().map(|value| (*value).to_owned()).collect(),
            paths: paths.iter().map(|value| (*value).to_owned()).collect(),
        }],
    }
}

fn cli_runtime_config() -> NativeRuntimeConfig {
    let mut runtime = RuntimeConfig::new("native-generated-runtime");
    runtime.limits.max_steps_per_turn = Some(321);
    runtime.diagnostics.trace_enabled = true;
    runtime.diagnostics.log_level = mech_runtime::LogLevel::Debug;
    NativeRuntimeConfig {
        runtime,
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

#[cfg(all(feature = "standard", not(feature = "fixed")))]
fn owner_catalog() -> AppResult<Arc<FunctionCatalog>> {
    Ok(mech_stdlib::native_plan_catalog())
}

#[cfg(all(feature = "fixed", not(feature = "standard")))]
fn owner_catalog() -> AppResult<Arc<FunctionCatalog>> {
    let mut builder = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_runtime(&mut builder)
        .map_err(|error| mech_error("engine owner catalog", error))?;
    mech_math::install_runtime(&mut builder)
        .map_err(|error| mech_error("math owner catalog", error))?;
    Ok(Arc::new(
        builder
            .build()
            .map_err(|error| mech_error("fixed owner catalog", error))?,
    ))
}

#[cfg(any(
    all(feature = "standard", feature = "fixed"),
    all(not(feature = "standard"), not(feature = "fixed")),
))]
fn owner_catalog() -> AppResult<Arc<FunctionCatalog>> {
    Err("enable exactly one owner profile: `standard` or `fixed`".into())
}

fn poison_runtime_output_seeds(bytes: Vec<u8>) -> AppResult<(Vec<u8>, usize)> {
    let parsed = ParsedProgram::from_bytes(&bytes)
        .map_err(|error| mech_error("output-seed bytecode parse", error))?;
    let runtime_destinations = parsed
        .instructions
        .iter()
        .filter_map(instruction_destination)
        .collect::<BTreeSet<_>>();
    if runtime_destinations.is_empty() {
        return Err("compiled source has no runtime output seed to poison".into());
    }
    let mut constant_ids = BTreeSet::new();
    for destination in runtime_destinations {
        let constant = parsed
            .instructions
            .iter()
            .find_map(|instruction| match instruction {
                BytecodeInstruction::ConstLoad { dst, constant } if *dst == destination => {
                    Some(*constant)
                }
                _ => None,
            })
            .ok_or_else(|| {
                format!(
                    "runtime output register {destination} has no compiler-emitted seed constant"
                )
            })?;
        constant_ids.insert(constant);
    }

    let mut constants = parsed
        .constants
        .iter()
        .map(|entry| -> AppResult<EncodedConstant> {
            let start = usize::try_from(entry.offset)?;
            let length = usize::try_from(entry.length)?;
            let end = start.checked_add(length).ok_or("constant range overflow")?;
            let bytes = parsed
                .constant_blob
                .get(start..end)
                .ok_or("constant range is outside the blob")?
                .to_vec();
            let runtime_type = parsed
                .types
                .get(usize::try_from(entry.type_id)?)
                .ok_or("constant type ID is outside the type table")?
                .clone();
            Ok(EncodedConstant {
                runtime_type,
                alignment: entry.alignment,
                bytes,
            })
        })
        .collect::<AppResult<Vec<_>>>()?;
    for constant_id in &constant_ids {
        let output = constants
            .get_mut(usize::try_from(*constant_id)?)
            .ok_or("output seed constant is outside the constant table")?;
        if matches!(
            &output.runtime_type,
            RuntimeType::Set { element, max_len: Some(0) }
                if **element == RuntimeType::Empty
        ) {
            // An empty comprehension has no element from which the compiler
            // can infer a narrower Set element type. Seed it with a valid,
            // nonempty Set<f64>; the runtime nullary factory accepts the same
            // collection kind and must replace it with the calculated empty
            // set.
            output.runtime_type = RuntimeType::Set {
                element: Box::new(RuntimeType::F64),
                max_len: Some(1),
            };
            output.bytes = Vec::with_capacity(16);
            output.bytes.extend_from_slice(&1_u32.to_le_bytes());
            output.bytes.extend_from_slice(&8_u32.to_le_bytes());
            output.bytes.extend_from_slice(&0.0_f64.to_le_bytes());
            continue;
        }
        match &output.runtime_type {
            RuntimeType::F64 if output.bytes.len() == 8 => {
                if output.bytes.iter().all(|byte| *byte == 0) {
                    return Err(format!("compiler output seed {constant_id} was already zero").into());
                }
                output.bytes.fill(0);
            }
            RuntimeType::Matrix { element, .. } if **element == RuntimeType::F64 => {
                if output.bytes.len() < 8 || (output.bytes.len() - 8) % 8 != 0 {
                    return Err("invalid compiler-emitted matrix seed".into());
                }
                if output.bytes.len() == 8 {
                    let rows = u32::from_le_bytes(output.bytes[0..4].try_into()?);
                    let columns = u32::from_le_bytes(output.bytes[4..8].try_into()?);
                    if rows != 0 || columns != 0 {
                        return Err(format!(
                            "matrix output seed {constant_id} has a non-empty shape but no elements"
                        )
                        .into());
                    }
                    // A 1x0 dynamic matrix is a valid, kind-compatible but
                    // deliberately incorrect seed for the calculated 0x0
                    // nullary-comprehension result.
                    output.bytes[0..4].copy_from_slice(&1_u32.to_le_bytes());
                    continue;
                }
                if output.bytes[8..].iter().all(|byte| *byte == 0) {
                    return Err(
                        format!("compiler matrix output seed {constant_id} was already zero").into(),
                    );
                }
                output.bytes[8..].fill(0);
            }
            RuntimeType::Set { element, max_len }
                if **element == RuntimeType::F64 && max_len.is_none_or(|limit| limit >= 1) =>
            {
                match output.bytes.as_mut_slice() {
                    bytes if bytes == 0_u32.to_le_bytes() => {
                        // One f64 element is a valid set with the same runtime
                        // type, but differs from the empty result the nullary
                        // comprehension must calculate.
                        output.bytes = Vec::with_capacity(16);
                        output.bytes.extend_from_slice(&1_u32.to_le_bytes());
                        output.bytes.extend_from_slice(&8_u32.to_le_bytes());
                        output.bytes.extend_from_slice(&0.0_f64.to_le_bytes());
                    }
                    bytes
                        if bytes.len() == 16
                            && bytes[0..4] == 1_u32.to_le_bytes()
                            && bytes[4..8] == 8_u32.to_le_bytes() =>
                    {
                        if bytes[8..].iter().all(|byte| *byte == 0) {
                            return Err(format!(
                                "compiler set output seed {constant_id} was already zero"
                            )
                            .into());
                        }
                        bytes[8..].fill(0);
                    }
                    bytes => {
                        return Err(format!(
                            "cannot poison f64 set output seed {constant_id} with {} bytes",
                            bytes.len()
                        )
                        .into());
                    }
                }
            }
            runtime_type => {
                return Err(
                    format!("cannot poison output seed of type {runtime_type:?}").into(),
                );
            }
        }
    }

    let poisoned = write_bytecode(&BytecodeProgram {
        register_count: parsed.header.register_count,
        constants,
        symbols: parsed.symbols,
        mutable_symbols: parsed.mutable_symbols,
        instructions: parsed.instructions,
        dictionary: parsed.dictionary,
        requirements: parsed.requirements,
    })
    .map_err(|error| mech_error("output-seed bytecode rewrite", error))?;
    ParsedProgram::from_bytes(&poisoned)
        .map_err(|error| mech_error("poisoned bytecode validation", error))?;
    Ok((poisoned, constant_ids.len()))
}

fn mech_error(context: &str, error: impl std::fmt::Debug) -> io::Error {
    io::Error::other(format!("{context} failed: {error:?}"))
}

fn instruction_destination(instruction: &BytecodeInstruction) -> Option<u32> {
    match instruction {
        BytecodeInstruction::RuntimeNullary { dst, .. }
        | BytecodeInstruction::RuntimeUnary { dst, .. }
        | BytecodeInstruction::RuntimeBinary { dst, .. }
        | BytecodeInstruction::RuntimeTernary { dst, .. }
        | BytecodeInstruction::RuntimeQuaternary { dst, .. }
        | BytecodeInstruction::RuntimeVariadic { dst, .. } => Some(*dst),
        _ => None,
    }
}
