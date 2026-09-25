use mech::kernel::{Backend, Kernel};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bundle = std::env::args_os()
        .nth(1)
        .unwrap_or_else(|| "ekf.bundle".into());
    let kernel = Kernel::from_source(include_str!("ekf.mec"))
        .input("bearing", [-0.55; 4])
        .export("state")
        .compile(Backend::AotSimd)?;
    kernel.save_bundle(&bundle)?;
    println!(
        "Saved AOT bundle: {}",
        std::path::Path::new(&bundle).display()
    );
    Ok(())
}
