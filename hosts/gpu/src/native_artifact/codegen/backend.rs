//! Compiler-only module construction; each family retains its emission flags.
use crate::native_artifact::compiler::{NativeCompileError, backend};
use cranelift_codegen::settings::{self, Configurable};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::default_libcall_names;
use cranelift_object::{ObjectBuilder, ObjectModule};

pub(in crate::native_artifact) const SCALAR_FLAGS: &[(&str, &str)] = &[
    ("opt_level", "none"),
    ("enable_nan_canonicalization", "false"),
    ("is_pic", "true"),
    ("preserve_frame_pointers", "true"),
    ("unwind_info", "true"),
];
pub(in crate::native_artifact) const CONTROL_FLAGS: &[(&str, &str)] = &[
    ("opt_level", "none"),
    ("is_pic", "true"),
    ("preserve_frame_pointers", "true"),
    ("unwind_info", "true"),
];

pub(in crate::native_artifact) fn jit(
    flags: &[(&str, &str)],
) -> Result<JITModule, NativeCompileError> {
    let builder = JITBuilder::with_flags(flags, default_libcall_names()).map_err(backend)?;
    // x86 local calls and data references use signed 32-bit displacements.
    // Independent system mappings can be more than 2 GiB apart after a large
    // compilation. Reserve one address range for code and data instead. Unix
    // commits its pages only as the JIT uses them; Windows commits up front.
    #[cfg(all(target_arch = "x86_64", not(target_os = "windows")))]
    let builder = {
        let mut builder = builder;
        builder.memory_provider(Box::new(
            cranelift_jit::ArenaMemoryProvider::new_with_size(1 << 31).map_err(backend)?,
        ));
        builder
    };
    Ok(JITModule::new(builder))
}

pub(in crate::native_artifact) fn object(
    name: &str,
    options: &[(&str, &str)],
) -> Result<ObjectModule, NativeCompileError> {
    object_with_isa(name, options).map(|(module, _)| module)
}

pub(in crate::native_artifact) fn object_with_isa(
    name: &str,
    options: &[(&str, &str)],
) -> Result<(ObjectModule, cranelift_codegen::isa::OwnedTargetIsa), NativeCompileError> {
    let mut flags = settings::builder();
    for &(key, value) in options {
        flags.set(key, value).map_err(backend)?;
    }
    let isa = cranelift_native::builder()
        .map_err(backend)?
        .finish(settings::Flags::new(flags))
        .map_err(backend)?;
    let module = ObjectModule::new(
        ObjectBuilder::new(isa.clone(), name, default_libcall_names()).map_err(backend)?,
    );
    Ok((module, isa))
}

#[cfg(all(test, target_arch = "x86_64", not(target_os = "windows")))]
mod tests {
    use super::*;
    use cranelift_codegen::ir::{AbiParam, InstBuilder, MemFlags, types};
    use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
    use cranelift_module::{DataDescription, Linkage, Module};

    #[test]
    fn jit_code_and_data_stay_within_relative_addressing_range() {
        let mut module = jit(CONTROL_FLAGS).unwrap();
        let data = module
            .declare_data("payload", Linkage::Local, false, false)
            .unwrap();
        let mut description = DataDescription::new();
        description.define(vec![42; 8 * 1024 * 1024].into_boxed_slice());
        module.define_data(data, &description).unwrap();
        let mut context = module.make_context();
        context
            .func
            .signature
            .returns
            .push(AbiParam::new(types::I64));
        let function = module
            .declare_function("read_payload", Linkage::Local, &context.func.signature)
            .unwrap();
        let mut builder_context = FunctionBuilderContext::new();
        let mut builder = FunctionBuilder::new(&mut context.func, &mut builder_context);
        let block = builder.create_block();
        builder.switch_to_block(block);
        let global = module.declare_data_in_func(data, builder.func);
        let address = builder.ins().global_value(types::I64, global);
        let value = builder.ins().load(types::I64, MemFlags::new(), address, 0);
        builder.ins().return_(&[value]);
        builder.seal_all_blocks();
        builder.finalize();
        module.define_function(function, &mut context).unwrap();
        module.finalize_definitions().unwrap();
        let code = module.get_finalized_function(function).addr();
        let (payload, length) = module.get_finalized_data(data);
        assert_eq!(length, 8 * 1024 * 1024);
        assert!(code.abs_diff(payload.addr()) <= i32::MAX as usize);
        assert!(code.abs_diff(payload.addr() + length - 1) <= i32::MAX as usize);
    }
}
