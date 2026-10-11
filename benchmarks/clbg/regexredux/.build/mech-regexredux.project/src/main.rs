use mech_gpu::sequence::Context;
unsafe extern "C" { fn mech_sequence_entry(context: *mut std::ffi::c_void); }
fn main() {
    let constants: Vec<_> = serde_json::from_str(include_str!("../constants.json")).unwrap();
    let partitions = std::env::var("MECH_SEQUENCE_PARTITIONS").unwrap_or_else(|_| "4".into()).parse().unwrap();
    let grain = std::env::var("MECH_ARRAY_GRAIN").unwrap_or_else(|_| "16".into()).parse().unwrap();
    let compact = std::env::var("MECH_SEQUENCE_NARROW").as_deref() != Ok("0");
    let standard = std::env::var("MECH_SEQUENCE_STD_MAP").as_deref() != Ok("0");
    let independent = std::env::var("MECH_SEQUENCE_ASYNC").as_deref() == Ok("1");
    let mut context = Context::new(&constants, None, true).with_histogram_policy(partitions, compact, false).unwrap().with_standard_map(standard).with_array_grain(grain).unwrap().with_independent_tasks(independent);
    // SAFETY: this application links the source object and constant table emitted
    // together by the checked native sequence compiler; the ABI is library-owned.
    if let Err(error) = unsafe { context.run(mech_sequence_entry) } { eprintln!("{error}"); std::process::exit(1); }
}
