use mech_gpu::sequence::{CliHostIo, Constant, Context, NativeResourceRequirement};

unsafe extern "C" { fn mech_sequence_entry(context: *mut std::ffi::c_void); }

fn run() -> Result<(), String> {
    let constants: Vec<Constant> = serde_json::from_str(include_str!("../constants.json"))
        .map_err(|error| error.to_string())?;
    let requirements: Vec<NativeResourceRequirement> =
        serde_json::from_str(include_str!("../resource-requirements.json"))
            .map_err(|error| error.to_string())?;
    let partitions = std::env::var("MECH_SEQUENCE_PARTITIONS")
        .unwrap_or_else(|_| "4".into()).parse().map_err(|_| "invalid partition count")?;
    let grain = std::env::var("MECH_ARRAY_GRAIN")
        .unwrap_or_else(|_| "16".into()).parse().map_err(|_| "invalid array grain")?;
    let compact = std::env::var("MECH_SEQUENCE_NARROW").as_deref() != Ok("0");
    let standard = std::env::var("MECH_SEQUENCE_STD_MAP").as_deref() != Ok("0");
    let independent = std::env::var("MECH_SEQUENCE_ASYNC").as_deref() == Ok("1");
    let reuse = std::env::var("MECH_SEQUENCE_BUFFER_REUSE").as_deref() == Ok("1");
    let host = CliHostIo::from_environment(&requirements)?;
    let mut context = Context::activated(&constants, Box::new(host))?
        .with_histogram_policy(partitions, compact, false)?
        .with_standard_map(standard)
        .with_array_grain(grain)?
        .with_independent_tasks(independent)
        .with_byte_buffer_reuse(reuse);
    // SAFETY: object, constants and resource identities are emitted together by
    // the checked compiler. Every host operation resolves an activated port.
    unsafe { context.run(mech_sequence_entry) }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
