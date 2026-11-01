fn main() {
    if let Err(error) = mech_gpu::fused_f64::runner::run_cli() {
        eprintln!("{error}");
        std::process::exit(2);
    }
}
