use mech_core::*;
use std::ffi::c_void;

// Pull in mech-math so inventory descriptors are present in this dylib.
use mech_math as _;

fn math_dynamic_trace(message: impl std::fmt::Display) {
    if std::env::var_os("MECH_DYNAMIC_TRACE").is_some() {
        eprintln!("[math-dylib] {message}");
    }
}

struct DynamicFunctionBox {
    inner: Box<dyn MechFunction>,
}

unsafe fn dynamic_function_solve(instance: *mut c_void) {
    let function = unsafe { &*(instance as *mut DynamicFunctionBox) };
    function.inner.solve();
}

unsafe fn dynamic_function_out(instance: *mut c_void) -> Value {
    let function = unsafe { &*(instance as *mut DynamicFunctionBox) };
    function.inner.out()
}

unsafe fn dynamic_function_compile(
    instance: *mut c_void,
    ctx: &mut CompileCtx,
) -> MResult<Register> {
    let function = unsafe { &*(instance as *mut DynamicFunctionBox) };
    function.inner.compile(ctx)
}

unsafe fn dynamic_function_drop(instance: *mut c_void) {
    drop(unsafe { Box::from_raw(instance as *mut DynamicFunctionBox) });
}

static DYNAMIC_FUNCTION_VTABLE: DynamicFunctionVTableV1 = DynamicFunctionVTableV1 {
    solve: dynamic_function_solve,
    out: dynamic_function_out,

    compile: dynamic_function_compile,

    drop: dynamic_function_drop,
};

unsafe fn dynamic_compile_native_compiler(
    compiler: *const c_void,
    arguments: &Vec<Value>,
) -> MResult<DynamicFunctionHandleV1> {
    math_dynamic_trace("dynamic compile thunk called");
    let compiler_desc = unsafe { &*(compiler as *const FunctionCompilerDescriptor) };
    math_dynamic_trace(format!("dynamic compile descriptor `{}`", compiler_desc.name));
    let inner = compiler_desc.ptr.compile(arguments)?;
    math_dynamic_trace("native compiler returned function");

    let boxed = Box::new(DynamicFunctionBox { inner });

    Ok(DynamicFunctionHandleV1 {
        instance: Box::into_raw(boxed) as *mut c_void,
        vtable: &DYNAMIC_FUNCTION_VTABLE,
    })
}

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
                registrar.register_compiler(
                    fxn_comp.name,
                    fxn_comp as *const FunctionCompilerDescriptor as *const c_void,
                    dynamic_compile_native_compiler,
                );
            }
        }
    }

    Ok(())
}
