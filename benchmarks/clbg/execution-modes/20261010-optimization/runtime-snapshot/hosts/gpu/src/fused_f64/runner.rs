//! Generic CLI support for autonomous fixed-shape f64 programs. Program logic,
//! state layout, and selected output come entirely from the canonical artifact.
use super::{FusedProgram, JitKernel, Operand};
use mech_core::ReactiveInstanceId;
use mech_engine::__resident::{ActivationFacts, ReactiveInstance, ResidentValueBorrow, activate};
use mech_engine::{ProgramArtifact, SlotRole};
use mech_runtime::RuntimeBuilder;
use std::{collections::BTreeMap, fmt::Write, time::Instant};

fn error(error: impl std::fmt::Debug) -> String {
    format!("{error:?}")
}

fn state(
    instance: &ReactiveInstance,
    artifact: &ProgramArtifact,
) -> Result<BTreeMap<mech_core::CellSlotId, Vec<f64>>, String> {
    artifact
        .slots()
        .iter()
        .filter(|slot| slot.role == SlotRole::State)
        .map(|slot| {
            let Some(ResidentValueBorrow::F64 { values, .. }) = instance.state_borrow(slot.slot)
            else {
                return Err("native runner requires f64 state".into());
            };
            Ok((slot.slot, values.to_vec()))
        })
        .collect()
}

fn resident_output(instance: &mut ReactiveInstance, index: usize) -> Result<Vec<f64>, String> {
    // Read the candidate's pure output without committing its state writes.
    let candidate = instance.prepare_turn(&[]).map_err(error)?;
    let Some(ResidentValueBorrow::F64 { values, .. }) = candidate.output_borrow(index) else {
        return Err("selected output is not f64".into());
    };
    Ok(values.to_vec())
}

fn print_output(output: &[f64]) {
    if output.len() == 1 {
        println!("{:.9}", output[0]);
    } else {
        println!("{output:?}");
    }
}

/// Run the generic native CLI. Syntax: N SOURCE MODE [C_PATH OBJECT_PATH]
/// [--output NAME]. The final statement is the default program result.
/// AOT output includes the same runtime for every program.
pub fn run_cli() -> Result<(), String> {
    run_args(std::env::args().skip(1).collect())
}
pub fn run_args(args: Vec<String>) -> Result<(), String> {
    run(args, Instant::now(), None)
}
/// Retain the caller's admission timing and already-loaded source document.
pub fn run_loaded(args: Vec<String>, source: &str, start: Instant) -> Result<(), String> {
    run(args, start, Some(source))
}
fn run(mut args: Vec<String>, start: Instant, loaded_source: Option<&str>) -> Result<(), String> {
    let output = if let Some(index) = args.iter().position(|arg| arg == "--output") {
        if index + 1 >= args.len() {
            return Err("--output requires a name".into());
        }
        let output = args.remove(index + 1);
        args.remove(index);
        output
    } else {
        "result".to_owned()
    };
    let mut take_flag = |flag: &str| {
        if let Some(index) = args.iter().position(|arg| arg == flag) {
            args.remove(index);
            true
        } else {
            false
        }
    };
    let contract = take_flag("--fp-contract");
    let refined = take_flag("--refined-reciprocal");
    let schedule = take_flag("--schedule");
    let pack_state_tails = take_flag("--pack-state-tails");
    let reassociate = take_flag("--reassociate-constants");
    let algebraic = take_flag("--algebraic-powers");
    if args.len() < 2 {
        return Err("turn count and source path are required".into());
    }
    let turns = args[0].parse::<usize>().map_err(error)?;
    let owned_source;
    let source = if let Some(source) = loaded_source {
        source
    } else {
        owned_source = std::fs::read_to_string(&args[1]).map_err(error)?;
        &owned_source
    };
    let mode = args.get(2).map(String::as_str).unwrap_or("jit-simd");
    let catalog = mech_stdlib::source_catalog();
    let (artifact, _) = RuntimeBuilder::new()
        .function_catalog(catalog.clone())
        .build_compiler()
        .map_err(error)?
        .compile_source(source)
        .map_err(error)?
        .into_parts();
    if !artifact.inputs().is_empty() {
        return Err("autonomous runner requires a program without inputs; use the native map runner for explicit grid inputs".into());
    }
    let mut instance = activate(
        ReactiveInstanceId::new(500, 0),
        &artifact,
        &catalog,
        &ActivationFacts::default(),
    )
    .map_err(error)?;
    let initial = state(&instance, &artifact)?;
    if mode == "resident" {
        if contract || refined || schedule || algebraic || pack_state_tails || reassociate {
            return Err("native optimization flags require a compiled execution mode".into());
        }
        let index = artifact
            .outputs()
            .iter()
            .position(|decl| {
                decl.name == output
                    || decl
                        .interactive_binding
                        .as_ref()
                        .is_some_and(|binding| binding.lexical_name == output)
            })
            .ok_or("unknown output")?;
        print_output(&resident_output(&mut instance, index)?);
        let prepared = Instant::now();
        for _ in 0..turns {
            instance.turn_without_summary(&[]).map_err(error)?;
        }
        let elapsed = prepared.elapsed().as_secs_f64();
        print_output(&resident_output(&mut instance, index)?);
        diagnostic(
            &state(&instance, &artifact)?
                .into_values()
                .flatten()
                .collect::<Vec<_>>(),
            prepared.duration_since(start).as_secs_f64(),
            elapsed,
        );
        return Ok(());
    }
    let mut program = FusedProgram::lower(&artifact, &initial)?;
    if algebraic {
        program.lower_algebraic_powers();
    }
    if refined {
        program.refine_reciprocal();
    }
    if contract {
        program.contract_fma();
    }
    if reassociate {
        program.reassociate_constant_products();
    }
    if schedule {
        program.schedule_dependencies();
    }
    if pack_state_tails {
        program.pack_state_tails();
    }
    if mode == "resident-tape" {
        use std::io::Write as _;
        let kernel = super::ResidentKernel::load(&program);
        let mut state = program.initial.clone();
        let prepared = Instant::now();
        print_output(&kernel.observe(&state, &output)?);
        kernel.run(&mut state, turns)?;
        print_output(&kernel.observe(&state, &output)?);
        std::io::stdout().flush().map_err(error)?;
        let elapsed = prepared.elapsed().as_secs_f64();
        diagnostic(
            &state,
            prepared.duration_since(start).as_secs_f64(),
            elapsed,
        );
        return Ok(());
    }
    let output_count = program.output_len(&output)?;
    if matches!(mode, "emit-c" | "emit-cranelift" | "emit-cranelift-simd") {
        let path = args.get(3).ok_or("generated C path is required")?;
        let mut code = if mode == "emit-c" {
            program.c_kernel() + &program.c_observer(&output)?
        } else {
            let object = args.get(4).ok_or("native object path is required")?;
            std::fs::write(
                object,
                program.aot_object_with_output(&output, mode.ends_with("-simd"))?,
            )
            .map_err(error)?;
            String::from(
                "#include <stddef.h>\n#include <math.h>\nvoid mech_turns(double *state, size_t turns);\nvoid mech_observe(const double *state, double *output);\n",
            )
        };
        writeln!(
            code,
            "#define STATE_COUNT {}\n#define OUTPUT_COUNT {output_count}",
            program.initial.len()
        )
        .unwrap();
        writeln!(
            code,
            "static const double initial[{}] = {{",
            program.initial.len().max(1)
        )
        .unwrap();
        for value in &program.initial {
            writeln!(code, "{},", program.c_operand(Operand::Constant(*value))).unwrap();
        }
        if program.initial.is_empty() {
            code.push_str("0.0,\n");
        }
        code.push_str("};\n");
        code.push_str(include_str!("runner.c"));
        std::fs::write(path, code).map_err(error)?;
        return Ok(());
    }
    if !matches!(mode, "jit" | "jit-simd") {
        return Err(format!("unknown execution mode: {mode}"));
    }
    let kernel = JitKernel::compile_with_output(&program, &output, mode == "jit-simd")?;
    let mut state = program.initial.clone();
    print_output(&kernel.observe(&state)?);
    let prepared = Instant::now();
    kernel.run(&mut state, turns);
    let elapsed = prepared.elapsed().as_secs_f64();
    print_output(&kernel.observe(&state)?);
    diagnostic(
        &state,
        prepared.duration_since(start).as_secs_f64(),
        elapsed,
    );
    Ok(())
}

fn diagnostic(state: &[f64], prepare: f64, kernel: f64) {
    if std::env::var_os("MECH_DUMP_STATE").is_some() {
        eprintln!("STATE {state:?}");
    }
    if std::env::var_os("MECH_TIMINGS").is_some() {
        eprintln!("TIMING {{\"prepare_seconds\":{prepare},\"kernel_seconds\":{kernel}}}");
    }
}
