fn main() {
    if let Err(error) = mech_gpu::execution::run_cli() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
