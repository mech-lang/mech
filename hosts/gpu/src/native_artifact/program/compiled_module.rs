//! Compiler-owned linkage for immutable native program data and entry points.
use crate::native_artifact::{
    NATIVE_PROGRAM_IMAGE_BINDING_BYTES, NATIVE_PROGRAM_LINK_IDENTITY_BYTES, NativeLinkError,
};
use std::ffi::c_void;

type Entry = unsafe extern "C" fn(*mut c_void) -> u64;
const MAGIC: [u8; 16] = *b"MECHMODULEPIN\0\0\0";
const VERSION: u64 = 1;

/// Read-only compiler-emitted module record. There is no public constructor or
/// wire decoder. Its relocations name data and code in one checked Object.
/// Merely creating matching bytes does not establish checked-code provenance.
#[repr(C)]
pub struct NativeProgramModule {
    magic: [u8; 16],
    version: u64,
    pointer_bits: u64,
    image: *const u8,
    image_len: u64,
    identity: *const [u8; NATIVE_PROGRAM_LINK_IDENTITY_BYTES],
    binding: *const [u8; NATIVE_PROGRAM_IMAGE_BINDING_BYTES],
    entry: Entry,
}
// SAFETY: fields are private and immutable. The only public consumer is unsafe
// and requires genuine process-lifetime read-only data and code from one Object.
unsafe impl Sync for NativeProgramModule {}

pub(super) struct ModuleParts {
    pub image: &'static [u8],
    pub identity: &'static [u8; NATIVE_PROGRAM_LINK_IDENTITY_BYTES],
    pub binding: &'static [u8; NATIVE_PROGRAM_IMAGE_BINDING_BYTES],
    pub entry: Entry,
}
impl NativeProgramModule {
    fn valid_header(&self) -> bool {
        self.magic == MAGIC
            && self.version == VERSION
            && self.pointer_bits == usize::BITS as u64
            && self.pointer_bits == 64
            && self.image_len > 0
            && self.image_len <= isize::MAX as u64
            && !self.image.is_null()
            && !self.identity.is_null()
            && !self.binding.is_null()
    }
    /// # Safety
    /// Self and every relocation must be genuine immutable exports from the
    /// same unchanged checked compiler Object, with process lifetime. No caller
    /// bytes, digest, memory-permission test or static lifetime grants authority.
    pub(super) unsafe fn parts(&'static self) -> Result<ModuleParts, NativeLinkError> {
        if !self.valid_header() {
            return Err(NativeLinkError::new(
                "unsupported checked native module record",
            ));
        }
        // SAFETY: the caller establishes every pointer's immutable same-Object
        // provenance and extent; the checks above reject incompatible headers.
        Ok(unsafe {
            ModuleParts {
                image: std::slice::from_raw_parts(self.image, self.image_len as usize),
                identity: &*self.identity,
                binding: &*self.binding,
                entry: self.entry,
            }
        })
    }
}

#[cfg(feature = "canonical-native-program")]
pub(super) fn define<M: cranelift_module::Module>(
    module: &mut M,
    symbol: &str,
    image: cranelift_module::DataId,
    image_len: usize,
    identity: cranelift_module::DataId,
    binding: cranelift_module::DataId,
    entry: cranelift_module::FuncId,
) -> Result<(), crate::native_artifact::NativeCompileError> {
    use crate::native_artifact::compiler::backend;
    use cranelift_module::{DataDescription, Linkage};
    use std::mem::{offset_of, size_of};
    if size_of::<NativeProgramModule>() != 72
        || module.target_config().pointer_type() != cranelift_codegen::ir::types::I64
    {
        return Err(crate::native_artifact::NativeCompileError::UnsupportedHost);
    }
    let id = module
        .declare_data(symbol, Linkage::Export, false, false)
        .map_err(backend)?;
    let mut bytes = vec![0; size_of::<NativeProgramModule>()];
    bytes[..16].copy_from_slice(&MAGIC);
    bytes[16..24].copy_from_slice(&VERSION.to_le_bytes());
    bytes[24..32].copy_from_slice(&64u64.to_le_bytes());
    bytes[40..48].copy_from_slice(&(image_len as u64).to_le_bytes());
    let mut data = DataDescription::new();
    data.define(bytes.into_boxed_slice());
    data.set_align(8);
    for (offset, target) in [
        (offset_of!(NativeProgramModule, image), image),
        (offset_of!(NativeProgramModule, identity), identity),
        (offset_of!(NativeProgramModule, binding), binding),
    ] {
        let target = module.declare_data_in_data(target, &mut data);
        data.write_data_addr(offset as u32, target, 0);
    }
    let entry = module.declare_func_in_data(entry, &mut data);
    data.write_function_addr(offset_of!(NativeProgramModule, entry) as u32, entry);
    module.define_data(id, &data).map_err(backend)
}

#[cfg(all(test, feature = "canonical-native-program"))]
#[path = "compiled_module_tests.rs"]
mod tests;
