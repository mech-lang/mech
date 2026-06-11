use mech_core::*;
// Pull in mech-math so inventory descriptors are present in this dylib.
use mech_math as _;

/// Experimental same-build Rust ABI registration symbol for the `math` module.
///
/// This is intentionally a Rust `dylib` boundary for modules built from the
/// same source tree, same Rust toolchain, same `mech-core` version, and same
/// lockfile as the host. It is not a stable external plugin ABI.
#[unsafe(no_mangle)]
pub unsafe fn mech_module_register_v1(_fxns: &mut Functions, module: &str) -> MResult<Vec<String>> {
    if module != "math" {
        return Ok(Vec::new());
    }

    let mut items = Vec::<String>::new();

    for item_desc in inventory::iter::<ModuleItemDescriptor> {
        if item_desc.module == module {
            let item = item_desc.item.to_string();
            if !items.iter().any(|existing| existing == &item) {
                items.push(item);
            }
        }
    }

    Ok(items)
}

#[unsafe(no_mangle)]
pub unsafe fn mech_module_compilers_v1(module: &str) -> MResult<Vec<DynamicModuleCompilerV1>> {
    if module != "math" {
        return Ok(Vec::new());
    }

    let items = module_items(module);
    let module_prefix = format!("{module}/");
    let mut compilers = Vec::new();

    for fxn_comp in inventory::iter::<FunctionCompilerDescriptor> {
        let name = fxn_comp.name;

        if let Some(item) = name.strip_prefix(&module_prefix) {
            if items.iter().any(|manifest_item| manifest_item == item) {
                compilers.push(DynamicModuleCompilerV1 {
                    name,
                    ptr: fxn_comp.ptr,
                });
            }
        }
    }

    Ok(compilers)
}

fn module_items(module: &str) -> Vec<String> {
    let mut items = Vec::<String>::new();

    for item_desc in inventory::iter::<ModuleItemDescriptor> {
        if item_desc.module == module {
            let item = item_desc.item.to_string();
            if !items.iter().any(|existing| existing == &item) {
                items.push(item);
            }
        }
    }

    items
}
