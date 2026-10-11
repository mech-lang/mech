use std::{env, fs, path::PathBuf, process::Command};
fn main() {
    println!("cargo:rerun-if-env-changed=MECH_SEQUENCE_SOURCE");
    println!("cargo:rerun-if-env-changed=MECH_SEQUENCE_BATCH");
    println!("cargo:rerun-if-env-changed=MECH_SEQUENCE_SCHEDULE");
    println!("cargo:rerun-if-env-changed=MECH_SEQUENCE_PACK8");
    println!("cargo:rerun-if-env-changed=MECH_SEQUENCE_SIMD");
    println!("cargo:rerun-if-env-changed=MECH_SEQUENCE_FUSE_ENCODE");
    let source = env::var("MECH_SEQUENCE_SOURCE").unwrap_or_else(|_| "../knucleotide.mec".into());
    println!("cargo:rerun-if-changed={source}");
    let batch = env::var("MECH_SEQUENCE_BATCH")
        .unwrap_or_else(|_| "4096".into())
        .parse()
        .unwrap();
    let flag = |name: &str| match env::var(name).as_deref().unwrap_or("0") {
        "0" => false,
        "1" => true,
        _ => panic!("{name} must be 0 or 1"),
    };
    let schedule = match env::var("MECH_SEQUENCE_SCHEDULE")
        .as_deref()
        .unwrap_or("source")
    {
        "source" => false,
        "dag" => true,
        _ => panic!("MECH_SEQUENCE_SCHEDULE must be source or dag"),
    };
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    mech_gpu::sequence::emit_program(
        &fs::read_to_string(source).unwrap(),
        &out,
        batch,
        flag("MECH_SEQUENCE_PACK8"),
        flag("MECH_SEQUENCE_SIMD"),
        schedule,
        flag("MECH_SEQUENCE_FUSE_ENCODE"),
    )
    .unwrap();
    let object = out.join("mech-sequence.o");
    let status = Command::new("ar")
        .args(["crs"])
        .arg(out.join("libmech_sequence_program.a"))
        .arg(object)
        .status()
        .unwrap();
    assert!(status.success());
    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-lib=static=mech_sequence_program");
}
