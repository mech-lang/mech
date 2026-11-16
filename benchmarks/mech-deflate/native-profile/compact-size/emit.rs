use mech_engine::{decode_program_artifact_bytecode_v1, encode_program_artifact_bytecode_v1};
use mech_gpu::native_artifact::emit_compact_closed_object_for_diagnostic;
use std::{fs, path::Path};

fn emit(input: &Path, output: &Path) -> Result<(), String> {
    let bytes = fs::read(input).map_err(|error| error.to_string())?;
    let artifact =
        decode_program_artifact_bytecode_v1(&bytes).map_err(|error| format!("{error:?}"))?;
    let encoded =
        encode_program_artifact_bytecode_v1(&artifact).map_err(|error| format!("{error:?}"))?;
    if encoded != bytes {
        return Err("input is not canonical Program bytecode".into());
    }
    let catalog = mech_stdlib::source_catalog();
    let (object, metadata) = emit_compact_closed_object_for_diagnostic(&artifact, &catalog)
        .map_err(|error| format!("{error:?}"))?;
    fs::create_dir_all(output).map_err(|error| error.to_string())?;
    fs::write(output.join("program.o"), object).map_err(|error| error.to_string())?;
    let metadata = serde_json::to_vec(&metadata).map_err(|error| error.to_string())?;
    fs::write(output.join("metadata.json"), metadata).map_err(|error| error.to_string())?;
    Ok(())
}

fn main() -> Result<(), String> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let [input, output] = args.as_slice() else {
        return Err("usage: emit_compact_size PROGRAM.mecb OUTPUT_DIRECTORY".into());
    };
    let input = input.clone();
    let output = output.clone();
    // The compact diagnostic compiler still lowers a large canonical Plan.
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(move || emit(Path::new(&input), Path::new(&output)))
        .map_err(|error| error.to_string())?
        .join()
        .map_err(|_| "compact emitter panicked".to_owned())?
}
