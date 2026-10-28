use mech_gpu::native_artifact::{NativeProgramLimitMode, emit_program_object_with_limits};
fn inspect() {
    let bytes = std::fs::read("/private/tmp/mech-linear-restack-20261026/complete-compressor-choice-qualified.mbc").unwrap();
    let artifact = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    let started = std::time::Instant::now();
    let object = emit_program_object_with_limits(&artifact, &mech_stdlib::source_catalog(), NativeProgramLimitMode::NoLimits.compilation()).unwrap();
    let root = "/private/tmp/mech-deflate-profiling-20261028";
    std::fs::write(format!("{root}/mech-compressor.o"), object.bytes()).unwrap();
    std::fs::write(format!("{root}/mech-object-metadata.json"), object.metadata().to_string()).unwrap();
    println!("Object emitted: {} bytes in {:.3} seconds; no compression executed", object.bytes().len(), started.elapsed().as_secs_f64());
}
fn main() {
    std::thread::Builder::new().stack_size(32*1024*1024).spawn(inspect).unwrap().join().unwrap();
}
