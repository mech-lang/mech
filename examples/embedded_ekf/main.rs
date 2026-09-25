fn main() -> Result<(), Box<dyn std::error::Error>> {
    // POSTER-BEGIN
    use mech::kernel::{Backend, Kernel};
    let source = include_str!("ekf.mec");
    let kernel = Kernel::from_source(source)
        .input("bearing", [-0.55; 4])
        .export("state")
        .compile(Backend::AotSimd)?;
    let mut ekf = kernel.start()?;
    ekf.turn([("bearing", [-0.54; 4])])?;
    let state = ekf.state("state")?;
    // POSTER-END

    if let Some(path) = kernel.library_path() {
        println!("AOT library: {}", path.display());
    }
    println!("{} filter instances", kernel.instances());
    for (instance, pose) in state.chunks_exact(3).enumerate() {
        println!(
            "filter {instance}: x={:.6}, y={:.6}, heading={:.6}",
            pose[0], pose[1], pose[2]
        );
    }
    Ok(())
}
