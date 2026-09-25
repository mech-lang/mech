use mech::kernel::Kernel;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // SAFETY: Load only a trusted bundle produced for this host by the matching
    // Mech build. Loading a native library can execute its initialization code.
    // POSTER-AOT-BEGIN
    let kernel = unsafe { Kernel::load_bundle("ekf.bundle")? };
    let mut ekf = kernel.start()?;
    ekf.turn([("bearing", [-0.54; 4])])?;
    // POSTER-AOT-END
    for (instance, pose) in ekf.state("state")?.chunks_exact(3).enumerate() {
        println!(
            "filter {instance}: x={:.6}, y={:.6}, heading={:.6}",
            pose[0], pose[1], pose[2]
        );
    }
    Ok(())
}
