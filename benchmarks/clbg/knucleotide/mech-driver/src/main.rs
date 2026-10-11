use mech_gpu::sequence::Context;
unsafe extern "C" {
    fn mech_sequence_entry(context: *mut std::ffi::c_void);
}
fn main() {
    let constants: Vec<_> =
        serde_json::from_str(include_str!(concat!(env!("OUT_DIR"), "/constants.json"))).unwrap();
    let partitions = std::env::var("MECH_SEQUENCE_PARTITIONS")
        .unwrap_or_else(|_| "1".into())
        .parse()
        .unwrap();
    let compact = std::env::var("MECH_SEQUENCE_NARROW").as_deref() == Ok("1");
    let standard = std::env::var("MECH_SEQUENCE_STD_MAP").as_deref() == Ok("1");
    let profile = std::env::var("MECH_SEQUENCE_PROFILE").as_deref() == Ok("1");
    let mut context = Context::new(&constants, None, true)
        .with_histogram_policy(partitions, compact, profile)
        .unwrap()
        .with_standard_map(standard);
    // SAFETY: build.rs emits and links this entry from the very same constants,
    // using the collection backend's checked native ABI and library services.
    if let Err(error) = unsafe { context.run(mech_sequence_entry) } {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
