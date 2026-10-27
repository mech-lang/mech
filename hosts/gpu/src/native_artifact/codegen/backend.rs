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
    Ok(JITModule::new(builder))
}

pub(in crate::native_artifact) fn object(
    name: &str,
    options: &[(&str, &str)],
) -> Result<ObjectModule, NativeCompileError> {
    let mut flags = settings::builder();
    for &(key, value) in options {
        flags.set(key, value).map_err(backend)?;
    }
    let isa = cranelift_native::builder()
        .map_err(backend)?
        .finish(settings::Flags::new(flags))
        .map_err(backend)?;
    Ok(ObjectModule::new(
        ObjectBuilder::new(isa, name, default_libcall_names()).map_err(backend)?,
    ))
}
