use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use mech_core::{MResult, MechError, MechErrorKind, MechSourceCode, hash_str};
use mech_engine::{MechProgram, MechProgramConfig};
use mech_gpu::{GpuBindingRole, GpuHost, GpuProgram};
use mech_runtime::{FS_READ, MECH_TOOL_SUBJECT, RunExecutorConfig, check_fs_capability};

use crate::cli::outcome::CliOutcome;
use crate::cli::runtime_plan::RunExecutionPlan;

#[derive(Clone, Debug)]
struct ConfiguredExecutorError {
    operation: &'static str,
    reason: String,
}

impl MechErrorKind for ConfiguredExecutorError {
    fn name(&self) -> &str {
        "ConfiguredExecutorError"
    }

    fn message(&self) -> String {
        format!("{} failed: {}", self.operation, self.reason)
    }
}

fn executor_error(operation: &'static str, reason: impl Into<String>) -> MechError {
    MechError::new(
        ConfiguredExecutorError {
            operation,
            reason: reason.into(),
        },
        None,
    )
    .with_compiler_loc()
}

pub(crate) fn configured_executor(plan: &RunExecutionPlan) -> Option<&RunExecutorConfig> {
    plan.loaded_config
        .as_ref()?
        .document
        .run
        .as_ref()?
        .executor
        .as_ref()
}

pub(crate) fn run(plan: &RunExecutionPlan) -> MResult<CliOutcome> {
    let executor = configured_executor(plan).expect("caller checked configured executor");
    if !matches!(executor.provider.as_str(), "cpu" | "gpu") {
        return Err(executor_error(
            "select_executor",
            format!(
                "unknown executor provider `{}`; this build supports `cpu` and `gpu`",
                executor.provider
            ),
        ));
    }

    let source_path = one_mech_source(plan)?;
    let source = read_source(plan, &source_path)?;
    println!("[Mech Run] Compiling {}", source_path.display());

    let compile_started = Instant::now();
    let mut source_program = MechProgram::with_function_catalog(
        MechProgramConfig::default(),
        mech_stdlib::source_catalog(),
    );
    let catalog_elapsed = compile_started.elapsed();

    let parse_started = Instant::now();
    let tree = mech_syntax::parse(source.trim())?;
    let parse_elapsed = parse_started.elapsed();

    let source_started = Instant::now();
    source_program.run_tree(&tree)?;
    let source_elapsed = source_started.elapsed();

    let artifact_started = Instant::now();
    let artifact = source_program.compile_program_product()?.into_parts().0;
    let artifact_elapsed = artifact_started.elapsed();

    let lower_started = Instant::now();
    let placement = GpuHost.plan(&artifact);
    let program = GpuHost.compile(&artifact).map_err(|error| {
        executor_error(
            "admit_program",
            format!(
                "executor `{}` rejected the Mech program:\n{error}",
                executor.provider
            ),
        )
    })?;
    let lower_elapsed = lower_started.elapsed();
    let compile_elapsed = compile_started.elapsed();

    println!(
        "[Mech Run] Placement: {} GPU region(s), {} device state slot(s), {} transfer boundary/boundaries",
        placement.gpu_regions.len(),
        placement
            .slots
            .iter()
            .filter(|slot| slot.residence == mech_gpu::SlotResidence::DeviceState)
            .count(),
        placement.transfers.len(),
    );
    println!(
        "[Mech Run] Generated {} workgroup(s) in {:.3} ms",
        program.workgroup_count(),
        milliseconds(compile_elapsed),
    );
    println!(
        "[Mech Run] Compile phases: catalog {:.3} ms, parse {:.3} ms, source execution + initialization {:.3} ms, artifact {:.3} ms, GPU lowering {:.3} ms",
        milliseconds(catalog_elapsed),
        milliseconds(parse_elapsed),
        milliseconds(source_elapsed),
        milliseconds(artifact_elapsed),
        milliseconds(lower_elapsed),
    );

    let inputs_started = Instant::now();
    let inputs = source_input_values(&source_program, &program)?;
    println!(
        "[Mech Run] Captured executor inputs in {:.3} ms",
        milliseconds(inputs_started.elapsed())
    );
    match executor.provider.as_str() {
        "cpu" => {
            let mut session = program
                .prepare_cpu(&inputs)
                .map_err(|error| executor_error("prepare_cpu_executor", error.to_string()))?;
            let started = Instant::now();
            session
                .dispatch_turns(executor.turns)
                .map_err(|error| executor_error("dispatch_cpu_executor", error.to_string()))?;
            let elapsed = started.elapsed();
            let outputs = session
                .outputs()
                .map_err(|error| executor_error("read_cpu_outputs", error.to_string()))?;
            println!("[Mech Run] Executor: resident CPU");
            print_profile(executor.turns, elapsed);
            print_outputs(&outputs);
        }
        "gpu" => {
            let mut session = program
                .prepare_resident(&inputs)
                .map_err(|error| executor_error("prepare_gpu_executor", error.to_string()))?;
            let profile = session
                .run_turns(executor.turns)
                .map_err(|error| executor_error("dispatch_gpu_executor", error.to_string()))?;
            println!("[Mech Run] Executor: resident GPU");
            println!("[Mech Run] Adapter: {}", profile.adapter);
            print_profile(executor.turns, profile.dispatch);
            println!(
                "[Mech Run] Final readback: {:.3} ms",
                milliseconds(profile.readback)
            );
            print_outputs(&profile.outputs);
        }
        _ => unreachable!("provider validated above"),
    }

    Ok(CliOutcome::exit(0))
}

fn source_input_values(
    source_program: &MechProgram,
    program: &GpuProgram,
) -> MResult<BTreeMap<String, Vec<f32>>> {
    let symbols = source_program.interpreter().symbols();
    let symbols = symbols.borrow();
    let mut inputs = BTreeMap::new();
    for binding in program
        .bindings()
        .iter()
        .filter(|binding| binding.role() == GpuBindingRole::Input)
    {
        let cell = symbols.get(hash_str(&binding.name)).ok_or_else(|| {
            executor_error(
                "prepare_executor_inputs",
                format!("GPU input `{}` has no source value", binding.name),
            )
        })?;
        let values = cell.borrow().as_vecf32().map_err(|failure| {
            executor_error(
                "prepare_executor_inputs",
                format!("GPU input `{}` is not f32 data: {failure:?}", binding.name),
            )
        })?;
        if values.len() != binding.elements as usize {
            return Err(executor_error(
                "prepare_executor_inputs",
                format!(
                    "GPU input `{}` has {} source value(s), expected {}",
                    binding.name,
                    values.len(),
                    binding.elements,
                ),
            ));
        }
        inputs.insert(binding.name.clone(), values);
    }
    Ok(inputs)
}

fn one_mech_source(plan: &RunExecutionPlan) -> MResult<PathBuf> {
    let mut sources = Vec::new();
    for configured_path in &plan.run_paths {
        for target in super::commands::run::collect_run_targets_with_capabilities(
            Path::new(configured_path),
            &plan.filesystem_access.kernel,
        )? {
            if target.extension().and_then(|extension| extension.to_str()) == Some("mec") {
                sources.push(target);
            }
        }
    }
    match sources.as_slice() {
        [source] => Ok(source.clone()),
        [] => Err(executor_error(
            "select_executor_source",
            "configured executors require exactly one .mec source; none were found",
        )),
        _ => Err(executor_error(
            "select_executor_source",
            format!(
                "configured executors currently require exactly one .mec source; found {}",
                sources.len()
            ),
        )),
    }
}

fn read_source<'a>(plan: &RunExecutionPlan, source_path: &'a Path) -> MResult<String> {
    let mut kernel = plan.filesystem_access.kernel.clone();
    check_fs_capability(&mut kernel, MECH_TOOL_SUBJECT, FS_READ, source_path)?;
    let source = mech_runtime::read_runtime_source_file_with_capabilities(
        source_path,
        Some(&plan.filesystem_access.kernel),
        Some(MECH_TOOL_SUBJECT),
    )?;
    match source {
        MechSourceCode::String(source) => Ok(source),
        other => Err(executor_error(
            "read_executor_source",
            format!(
                "configured executors require textual Mech source, got {}",
                other.to_string()
            ),
        )),
    }
}

fn print_profile(turns: u32, elapsed: std::time::Duration) {
    let per_turn = elapsed.as_secs_f64() / f64::from(turns);
    println!(
        "[Mech Run] Turns: {turns} in {:.3} ms ({:.3} ms/turn)",
        milliseconds(elapsed),
        per_turn * 1_000.0,
    );
}

fn print_outputs(outputs: &BTreeMap<String, Vec<f32>>) {
    for (name, values) in outputs {
        let preview = values
            .iter()
            .take(8)
            .map(|value| format!("{value:.6}"))
            .collect::<Vec<_>>()
            .join(", ");
        let suffix = if values.len() > 8 { ", ..." } else { "" };
        println!(
            "[Mech Run] Output {name}: {} f32 value(s) [{preview}{suffix}]",
            values.len()
        );
    }
}

fn milliseconds(duration: std::time::Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}
