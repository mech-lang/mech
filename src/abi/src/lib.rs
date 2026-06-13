#![allow(non_camel_case_types)]

pub const MECH_MODULE_ABI_VERSION_V1: u32 = 1;

#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MechStatusV1 {
    Ok = 0,
    InvalidIndex = 1,
    NullPointer = 2,
    WrongType = 3,
    WrongShape = 4,
    Unsupported = 5,
    Panic = 6,
}

/// Borrowed UTF-8 string view owned by the dynamic module.
///
/// The pointer must be non-null when `len > 0`.
/// The pointed-to bytes must remain valid for at least as long as the
/// dynamic library remains loaded.
/// Ownership is never transferred across the ABI boundary.
/// The host must copy the bytes if it needs to retain the string.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct MechStrV1 {
    pub ptr: *const u8,
    pub len: usize,
}

impl MechStrV1 {
    pub const fn from_static(bytes: &'static [u8]) -> Self {
        Self {
            ptr: bytes.as_ptr(),
            len: bytes.len(),
        }
    }
}

// This v1 prototype intentionally contains one kernel kind.
// Add a #[repr(C)] union of typed kernel function pointers when the second
// kernel kind is introduced.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MechKernelKindV1 {
    BinaryF64F64ToF64 = 1,
}

/// Kernel for a binary scalar f64 function.
///
/// The host owns all Mech runtime values. The module receives copied scalar
/// inputs and writes one scalar result into `out`.
/// The module must not retain `out` after returning.
pub type MechBinaryF64F64ToF64KernelV1 =
    unsafe extern "C" fn(n: f64, k: f64, out: *mut f64) -> MechStatusV1;

/// One exported Mech function/kernel.
///
/// V1 intentionally supports a single typed kernel function pointer.
/// When a second kernel kind is added, replace the typed function field with
/// a `#[repr(C)]` union of typed function pointers keyed by `kind`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct MechExportV1 {
    pub name: MechStrV1,
    pub kind: MechKernelKindV1,

    // In v1 there is only one kernel kind, so keep this typed.
    // When more kinds are added, replace this with a #[repr(C)] union.
    pub binary_f64_f64_to_f64: MechBinaryF64F64ToF64KernelV1,
}

pub type MechModuleAbiVersionFnV1 = unsafe extern "C" fn() -> u32;
pub type MechModuleNameFnV1 = unsafe extern "C" fn(out: *mut MechStrV1) -> MechStatusV1;
pub type MechModuleExportCountFnV1 = unsafe extern "C" fn() -> usize;
pub type MechModuleGetExportFnV1 =
    unsafe extern "C" fn(index: usize, out: *mut MechExportV1) -> MechStatusV1;
