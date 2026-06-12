use mech_core::*;

// Pull in mech-math so inventory descriptors are present in this dylib.
use mech_math as _;

/// Experimental same-build Rust ABI declaration for the `math` module.
///
/// This is intentionally a Rust `dylib` boundary for modules built from the
/// same source tree, same Rust toolchain, same `mech-core` version, and same
/// lockfile as the host. It is not a stable external plugin ABI.
#[unsafe(no_mangle)]
pub static mech_module_math: DynamicModuleDeclaration = DynamicModuleDeclaration {
    abi_version: MECH_DYNAMIC_MODULE_ABI_VERSION,
    module: "math",
    register: register_math,
};

unsafe fn register_math(registrar: &mut DynamicModuleRegistrar) -> MResult<()> {
    if registrar.module() != "math" {
        return Ok(());
    }

    for item_desc in inventory::iter::<ModuleItemDescriptor> {
        if item_desc.module == "math" {
            registrar.register_item(item_desc.item);
        }
    }

    let module_prefix = "math/";

    for fxn_comp in inventory::iter::<FunctionCompilerDescriptor> {
        if let Some(item) = fxn_comp.name.strip_prefix(module_prefix) {
            if registrar
                .items()
                .iter()
                .any(|manifest_item| manifest_item == item)
            {
                registrar.register_compiler(fxn_comp.name, fxn_comp.ptr);
            }
        }
    }

    Ok(())
}
