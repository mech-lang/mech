//! General source runner for JIT and loaded interpretation of admitted graphs.
//! Program admission selects a typed language graph, never a benchmark name.
use mech_engine::{
    __resident::{ActivationFacts, ResidentValueBorrow, activate},
    SlotRole,
};
use mech_runtime::RuntimeBuilder;
use rayon::prelude::*;
use std::{collections::BTreeMap, io::Write, time::Instant};

pub fn run_cli() -> Result<(), String> {
    let start = Instant::now();
    let mut args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() < 2 {
        return Err("usage: mech-native-source SOURCE jit|resident [arguments] [numeric optimization flags]".into());
    }
    let path = args.remove(0);
    let mode = args.remove(0);
    if !matches!(mode.as_str(), "jit" | "resident") {
        return Err("mode must be jit or resident".into());
    }
    let mut flags = vec![];
    args.retain(|arg| {
        if arg.starts_with("--") {
            flags.push(arg.clone());
            false
        } else {
            true
        }
    });
    let source = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let arguments = args
        .iter()
        .map(|s| {
            s.parse::<u64>()
                .map_err(|_| "expected unsigned argument".to_owned())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let parsed = mech_engine::native_sequence::ParsedSource::parse(&source)?;
    let simd = !flags.iter().any(|s| s == "--scalar");
    if let Ok(program) = crate::sequence::compile_parsed(&parsed, 4096) {
        drop(parsed);
        if !flags.is_empty() && flags != ["--scalar"] {
            return Err("unsupported sequence runner flags".into());
        }
        let program = program.scheduled()?.fuse_record_encoding();
        let parse_env = |key: &str, default: usize| {
            std::env::var(key)
                .unwrap_or_else(|_| default.to_string())
                .parse::<usize>()
                .map_err(|_| format!("invalid {key}"))
        };
        let mut context = crate::sequence::Context::new(&program.constants, None, true)
            .with_arguments(std::iter::once(path).chain(args).collect())
            .with_histogram_policy(
                parse_env("MECH_SEQUENCE_PARTITIONS", 4)?,
                std::env::var("MECH_SEQUENCE_NARROW").as_deref() != Ok("0"),
                false,
            )?
            .with_standard_map(std::env::var("MECH_SEQUENCE_STD_MAP").as_deref() != Ok("0"))
            .with_array_grain(parse_env("MECH_ARRAY_GRAIN", 16)?)?
            .with_independent_tasks(std::env::var("MECH_SEQUENCE_ASYNC").as_deref() == Ok("1"))
            .with_byte_buffer_reuse(
                std::env::var("MECH_SEQUENCE_BUFFER_REUSE").as_deref() == Ok("1"),
            );
        let jit = if mode == "jit" {
            Some(crate::sequence::JitProgram::compile(&program, 4096, simd)?)
        } else {
            None
        };
        let prepared = Instant::now();
        if let Some(jit) = jit {
            jit.run(&mut context)?;
        } else {
            context.run_resident(&program)?;
        }
        diagnostic(start, prepared);
        return Ok(());
    }
    if let Ok(program) = mech_engine::native_sequence::control::compile_parsed(
        &parsed,
        mech_stdlib::native_control_catalog(),
    ) {
        drop(parsed);
        if !flags.is_empty() && flags != ["--scalar"] {
            return Err("unsupported control runner flags".into());
        }
        if mode == "jit" {
            let program = crate::control::JitProgram::from_program(program, simd)?;
            let prepared = Instant::now();
            program.run(&arguments)?;
            diagnostic(start, prepared);
        } else {
            let program = crate::control::ResidentProgram::lower(&program)?;
            let prepared = Instant::now();
            program.run(&arguments)?;
            diagnostic(start, prepared);
        }
        return Ok(());
    }
    if let Ok(grid) = mech_engine::native_sequence::grid::compile_parsed(
        &parsed,
        mech_stdlib::native_grid_catalog(),
    ) {
        drop(parsed);
        let catalog = mech_stdlib::source_catalog();
        let (artifact, _) = RuntimeBuilder::new()
            .function_catalog(catalog.clone())
            .build_compiler()
            .map_err(|e| format!("{e:?}"))?
            .compile_source(&grid.kernel_source)
            .map_err(|e| format!("{e:?}"))?
            .into_parts();
        let instance = activate(
            mech_core::ReactiveInstanceId::new(502, 0),
            &artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .map_err(|e| format!("{e:?}"))?;
        let mut initial = BTreeMap::new();
        for slot in artifact
            .slots()
            .iter()
            .filter(|s| s.role == SlotRole::State)
        {
            let Some(ResidentValueBorrow::F64 { values, .. }) = instance.state_borrow(slot.slot)
            else {
                return Err("grid requires f64 state".into());
            };
            initial.insert(slot.slot, values.to_vec());
        }
        let mut program = crate::fused_f64::FusedProgram::lower(&artifact, &initial)?;
        for flag in flags {
            match flag.as_str() {
                "--scalar" => {}
                "--fp-contract-loop" => program.contract_loop_fma(),
                "--fp-contract" => program.contract_fma(),
                "--schedule" => program.schedule_dependencies(),
                "--pack-state-tails" => program.pack_state_tails(),
                "--reassociate-constants" => program.reassociate_constant_products(),
                _ => return Err(format!("unsupported grid flag {flag}")),
            }
        }
        let lanes = program.output_len("inside")?;
        if lanes != grid.lanes {
            return Err("grid lane declaration mismatch".into());
        }
        let mut offsets = [usize::MAX; 3];
        let mut packed_offsets = [usize::MAX; 3];
        for (i, name) in grid.inputs.iter().enumerate() {
            let mut packed = 0;
            for input in &program.inputs {
                if &input.name == name {
                    offsets[i] = input.offset;
                    packed_offsets[i] = packed;
                    break;
                }
                packed += input.rows * input.columns;
            }
            if offsets[i] == usize::MAX {
                return Err("missing grid input".into());
            }
        }
        let dimension = |d| -> Result<usize, String> {
            use mech_engine::native_sequence::grid::DimensionSource;
            usize::try_from(match d {
                DimensionSource::Constant(n) => n,
                DimensionSource::Argument(i) => {
                    *arguments.get(i - 1).ok_or("missing grid argument")?
                }
            })
            .map_err(|_| "invalid grid dimension".into())
        };
        let width = dimension(grid.width)?;
        let height = dimension(grid.height)?;
        enum Kernel {
            Jit(crate::fused_f64::GridKernel),
            Resident(crate::fused_f64::ResidentGrid),
        }
        let kernel = if mode == "jit" {
            Kernel::Jit(crate::fused_f64::GridKernel::compile(
                &program,
                "done",
                "inside",
                simd,
                packed_offsets[0],
                packed_offsets[1],
                packed_offsets[2],
            )?)
        } else {
            Kernel::Resident(crate::fused_f64::ResidentGrid::load(
                &program, "done", "inside", offsets[0], offsets[1], offsets[2],
            )?)
        };
        let prepared = Instant::now();
        let bytes = match kernel {
            Kernel::Jit(k) => k.rows(width, height)?,
            Kernel::Resident(k) => {
                let stride = width.div_ceil(8);
                let size = stride
                    .checked_mul(height)
                    .filter(|&n| n <= isize::MAX as usize)
                    .ok_or("grid too large")?;
                let mut bytes = vec![0; size];
                if stride > 0 {
                    bytes
                        .par_chunks_mut(stride)
                        .enumerate()
                        .try_for_each(|(i, row)| k.row(width, i, row))?;
                }
                bytes
            }
        };
        let stdout = std::io::stdout();
        let mut writer = stdout.lock();
        write!(writer, "P4\n{width} {height}\n").map_err(|e| e.to_string())?;
        writer.write_all(&bytes).map_err(|e| e.to_string())?;
        writer.flush().map_err(|e| e.to_string())?;
        diagnostic(start, prepared);
        return Ok(());
    }
    // Autonomous canonical artifacts use the same fixed-shape graph as AOT.
    drop(parsed);
    if arguments.len() != 1 {
        return Err("autonomous source requires one turn count".into());
    }
    let mut native = vec![
        arguments[0].to_string(),
        path,
        if mode == "jit" {
            if simd { "jit-simd" } else { "jit" }
        } else {
            "resident-tape"
        }
        .into(),
    ];
    native.extend(flags.into_iter().filter(|s| s != "--scalar"));
    crate::fused_f64::runner::run_loaded(native, &source, start)
}
fn diagnostic(start: Instant, prepared: Instant) {
    if std::env::var_os("MECH_TIMINGS").is_some() {
        eprintln!(
            "TIMING {{\"prepare_seconds\":{},\"kernel_seconds\":{}}}",
            prepared.duration_since(start).as_secs_f64(),
            prepared.elapsed().as_secs_f64()
        );
    }
}
