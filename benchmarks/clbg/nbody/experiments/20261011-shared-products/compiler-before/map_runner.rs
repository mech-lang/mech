//! Generic Boolean grid map CLI. Coordinates are indices; the Mech
//! program supplies all mapping, numerical recurrence and termination logic.
use super::FusedProgram;
use mech_core::ReactiveInstanceId;
use mech_engine::{
    __resident::{ActivationFacts, ResidentValueBorrow, activate},
    SlotRole,
};
use mech_runtime::RuntimeBuilder;
use std::{collections::BTreeMap, fmt::Write};

pub fn run_cli() -> Result<(), String> {
    run_args(std::env::args().skip(1).collect())
}

/// Emit a complete source-defined grid program without a CLI subprocess.
pub fn emit_source_program(
    source: &std::path::Path,
    c: &std::path::Path,
    object: &std::path::Path,
    simd: bool,
    loop_fma: bool,
) -> Result<(), String> {
    let mut args = vec![
        source.display().to_string(),
        if simd {
            "emit-cranelift-simd"
        } else {
            "emit-cranelift"
        }
        .into(),
        c.display().to_string(),
        object.display().to_string(),
        "--source-program".into(),
    ];
    if loop_fma {
        args.push("--fp-contract-loop".into());
    }
    run_args(args)
}

fn run_args(mut args: Vec<String>) -> Result<(), String> {
    let mut take_flag = |flag: &str| {
        if let Some(i) = args.iter().position(|arg| arg == flag) {
            args.remove(i);
            true
        } else {
            false
        }
    };
    let source_program = take_flag("--source-program");
    let contract = take_flag("--fp-contract");
    let contract_loop = take_flag("--fp-contract-loop");
    let schedule = take_flag("--schedule");
    let pack_state_tails = take_flag("--pack-state-tails");
    let square_seeds = take_flag("--simplify-square-seeds");
    let reassociate = take_flag("--reassociate-constants");
    let packed_bits = take_flag("--packed-bits");
    let region_size = if let Some(index) = args.iter().position(|arg| arg == "--schedule-regions") {
        if index + 1 >= args.len() {
            return Err("--schedule-regions requires an instruction count".into());
        }
        let count = args
            .remove(index + 1)
            .parse::<usize>()
            .map_err(|e| e.to_string())?;
        args.remove(index);
        if !(8..=4096).contains(&count) {
            return Err("region instruction count must be 8 through 4096".into());
        }
        count
    } else {
        0
    };
    if contract && contract_loop {
        return Err("choose either full or loop-only FMA contraction".into());
    }
    if args.len() != if source_program { 4 } else { 8 } {
        return Err("usage: mech-native-map SOURCE emit-cranelift|emit-cranelift-simd C_PATH OBJECT_PATH [--source-program | STOP_OUTPUT BOOL_OUTPUT WIDTH_INPUT COLUMN_INPUT,ROW_INPUT] [--fp-contract|--fp-contract-loop] [--reassociate-constants] [--schedule] [--pack-state-tails] [--packed-bits] [--simplify-square-seeds] [--schedule-regions COUNT]".into());
    }
    let simd = match args[1].as_str() {
        "emit-cranelift" => false,
        "emit-cranelift-simd" => true,
        _ => return Err("unknown map emission mode".into()),
    };
    let mut source = std::fs::read_to_string(&args[0]).map_err(|e| e.to_string())?;
    let grid = if source_program {
        Some(mech_engine::native_sequence::grid::compile(
            &source,
            mech_stdlib::native_grid_catalog(),
        )?)
    } else {
        None
    };
    if let Some(grid) = &grid {
        source = grid.kernel_source.clone();
        args.extend([
            "done".into(),
            "inside".into(),
            grid.inputs[0].clone(),
            format!("{},{}", grid.inputs[1], grid.inputs[2]),
        ]);
    }
    let catalog = mech_stdlib::source_catalog();
    let (artifact, _) = RuntimeBuilder::new()
        .function_catalog(catalog.clone())
        .build_compiler()
        .map_err(|e| format!("{e:?}"))?
        .compile_source(&source)
        .map_err(|e| format!("{e:?}"))?
        .into_parts();
    if std::env::var_os("MECH_DUMP_IR").is_some() {
        for node in artifact.nodes() {
            if let Some(op) = node.as_operation() {
                eprintln!("node {} {}", op.node.get(), op.operation.canonical_name());
            }
        }
    }
    let resident = activate(
        ReactiveInstanceId::new(501, 0),
        &artifact,
        &catalog,
        &ActivationFacts::default(),
    )
    .map_err(|e| format!("map activation: {e:?}"))?;
    let mut initial = BTreeMap::new();
    for slot in artifact
        .slots()
        .iter()
        .filter(|slot| slot.role == SlotRole::State)
    {
        let Some(ResidentValueBorrow::F64 { values, .. }) = resident.state_borrow(slot.slot) else {
            return Err("native map requires f64 state".into());
        };
        initial.insert(slot.slot, values.to_vec());
    }
    let mut program = FusedProgram::lower(&artifact, &initial)?;
    if contract {
        program.contract_fma();
    }
    if contract_loop {
        program.contract_loop_fma();
    }
    if square_seeds {
        program.simplify_square_seeds();
    }
    if region_size > 0 {
        program.schedule_regions(region_size);
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
    if !program.output_is_boolean(&args[5]) {
        return Err("bitmap map output must be Boolean".into());
    }
    let output_count = program.output_len(&args[5])?;
    let coordinates = args[7].split(',').collect::<Vec<_>>();
    if coordinates.len() != 2 {
        return Err("column and row input names must be comma separated".into());
    }
    let mut offset = 0;
    let mut width = None;
    let mut column = None;
    let mut row = None;
    for input in &program.inputs {
        let length = input.rows * input.columns;
        if input.name == args[6] && length == 1 {
            width = Some(offset);
        } else if input.name == coordinates[0] && length == output_count {
            column = Some(offset);
        } else if input.name == coordinates[1] && length == 1 {
            row = Some(offset);
        } else {
            return Err(format!("grid map cannot supply input {}", input.name));
        }
        offset += length;
    }
    let (Some(width), Some(column), Some(row)) = (width, column, row) else {
        return Err("grid input names or shapes do not match".into());
    };
    if output_count > 64 || output_count == 0 {
        return Err("grid map supports 1 through 64 output lanes".into());
    }
    let object = if grid.is_some() {
        program.aot_grid_object(&args[4], &args[5], simd, width, column, row)?
    } else if packed_bits {
        program.aot_bitmap_map_object(&args[4], &args[5], simd)?
    } else {
        program.aot_map_object(&args[4], &args[5], simd)?
    };
    std::fs::write(&args[3], object).map_err(|e| e.to_string())?;
    let mut code = String::new();
    writeln!(code,"#define MAP_INPUT_COUNT {offset}\n#define MAP_LANES {output_count}\n#define MAP_PACKED_BITS {}\n#define GRID_WIDTH_INPUT {width}\n#define GRID_COLUMN_INPUT {column}\n#define GRID_ROW_INPUT {row}", u8::from(packed_bits)).unwrap();
    if let Some(grid) = &grid {
        use mech_engine::native_sequence::grid::DimensionSource;
        let mut last_argument = 0;
        for (name, dimension) in [("WIDTH", grid.width), ("HEIGHT", grid.height)] {
            let (argument, constant) = match dimension {
                DimensionSource::Argument(i) => (i, 0),
                DimensionSource::Constant(n) => (0, n),
            };
            last_argument = last_argument.max(argument);
            writeln!(
                code,
                "#define GRID_{name}_ARG {argument}\n#define GRID_{name}_CONST {constant}"
            )
            .unwrap();
        }
        writeln!(
            code,
            "#define GRID_RUNTIME_THREADS_ARG {}",
            last_argument + 1
        )
        .unwrap();
        code.push_str(include_str!("grid_runner.c"));
    } else {
        code.push_str(include_str!("map_runner.c"));
    }
    std::fs::write(&args[2], code).map_err(|e| e.to_string())?;
    Ok(())
}
