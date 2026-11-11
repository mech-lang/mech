//! Same-Object image/symbol binding and compiled launcher dispatch.
use mech_gpu::native_artifact::{NativeProgramModule, launch::{BuildOwnedProgramBinding, run_program_application}};
unsafe extern "C" {
    #[link_name = "mech_canonical_program_identity_v1_7d1b5b9df8106a284391bca3b5c5028c28292c0400916d931991cf64cd79ecde_compiled_module_v1"]
    static LINKED_MODULE: NativeProgramModule;
}
fn catalog() -> Result<mech_core::FunctionCatalog, String> {
    let builder = mech_core::FunctionCatalogBuilder::new();

    builder.build().map_err(|error| format!("{error:?}"))
}
fn main() {
    // SAFETY: build.rs verifies the unchanged Object/images and all generated
    // inputs. Entry, identity and images belong to that SAME checked build.
    let binding = unsafe { BuildOwnedProgramBinding::from_compiled_module(
        &LINKED_MODULE, catalog,
    ) };
    if let Err(error) = run_program_application(binding, std::env::args_os().skip(1).collect()) {
        eprintln!("{error}"); std::process::exit(1);
    }
}
