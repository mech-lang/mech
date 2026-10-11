//! Audit harness: emit the same general typed programs as the production AOT path.
use std::path::Path;
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
fn run() -> Result<(), String> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() < 3 {
        return Err("KIND SOURCE DIRECTORY [numeric flags]".into());
    }
    let kind = &args[0];
    let path = Path::new(&args[1]);
    let directory = Path::new(&args[2]);
    let source = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(directory).map_err(|e| e.to_string())?;
    let simd = !args.iter().any(|s| s == "--scalar");
    match kind.as_str() {
        "sequence" => {
            let program = mech_gpu::sequence::compile(&source, 4096)?
                .scheduled()?
                .fuse_record_encoding();
            std::fs::write(
                directory.join("mech-sequence.o"),
                program.object_with_array_simd(4096, true, false, simd)?,
            )
            .map_err(|e| e.to_string())?;
            std::fs::write(
                directory.join("constants.json"),
                serde_json::to_string(&program.constants).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
        }
        "control" => mech_gpu::control::emit(&source, directory, simd)?,
        "grid" => mech_gpu::fused_f64::map_runner::emit_source_program(
            path,
            &directory.join("generated.c"),
            &directory.join("mech-grid.o"),
            simd,
            args.iter().any(|s| s == "--fp-contract-loop"),
        )?,
        "numeric" => {
            let mut arguments = vec![
                "0".into(),
                path.display().to_string(),
                if simd {
                    "emit-cranelift-simd"
                } else {
                    "emit-cranelift"
                }
                .into(),
                directory.join("generated.c").display().to_string(),
                directory.join("mech-numeric.o").display().to_string(),
            ];
            arguments.extend(
                args[3..]
                    .iter()
                    .filter(|s| s.as_str() != "--scalar")
                    .cloned(),
            );
            mech_gpu::fused_f64::runner::run_args(arguments)?;
        }
        _ => return Err("unknown typed graph kind".into()),
    }
    Ok(())
}
