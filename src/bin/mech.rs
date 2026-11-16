#[cfg(all(feature = "compiler", not(target_arch = "wasm32")))]
#[path = "../compiler_allocator.rs"]
mod compiler_allocator;

fn main() -> Result<(), mech::MechError> {
    mech::cli::app::run()
}
