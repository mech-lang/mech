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

#[repr(C)]
#[derive(Clone, Copy)]
pub struct MechF64SliceV1 {
    pub ptr: *const f64,
    pub len: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct MechF64SliceMutV1 {
    pub ptr: *mut f64,
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

/// Scalar kernel shape exported by a dynamic module.
///
/// V1 supports scalar typed function pointers through `MechKernelFnV1`, a
/// tagged union keyed by this enum. The host must read only the union field
/// corresponding to `kind`.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MechKernelKindV1 {
    UnaryF64ToF64 = 1,
    BinaryF64F64ToF64 = 2,
    UnaryF64SliceToF64Slice = 3,
}

/// Kernel for a unary scalar f64 function.
///
/// The host owns all Mech runtime values. The module receives one copied
/// scalar input and writes one scalar result into `out`.
/// The module must not retain `out` after returning.
pub type MechUnaryF64ToF64KernelV1 =
    unsafe extern "C" fn(input: f64, out: *mut f64) -> MechStatusV1;

pub type MechUnaryF64SliceToF64SliceKernelV1 =
    unsafe extern "C" fn(input: MechF64SliceV1, out: MechF64SliceMutV1) -> MechStatusV1;

/// Kernel for a binary scalar f64 function.
///
/// The host owns all Mech runtime values. The module receives copied scalar
/// inputs and writes one scalar result into `out`.
/// The module must not retain `out` after returning.
pub type MechBinaryF64F64ToF64KernelV1 =
    unsafe extern "C" fn(n: f64, k: f64, out: *mut f64) -> MechStatusV1;

#[repr(C)]
#[derive(Clone, Copy)]
pub union MechKernelFnV1 {
    pub unary_f64_to_f64: MechUnaryF64ToF64KernelV1,
    pub binary_f64_f64_to_f64: MechBinaryF64F64ToF64KernelV1,
    pub unary_f64_slice_to_f64_slice: MechUnaryF64SliceToF64SliceKernelV1,
}

/// One exported Mech function/kernel.
///
/// V1 supports scalar typed function pointers through a tagged union. The host
/// must read only the `function` union field corresponding to `kind`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct MechExportV1 {
    pub name: MechStrV1,
    pub kind: MechKernelKindV1,
    pub function: MechKernelFnV1,
}

pub type MechModuleAbiVersionFnV1 = unsafe extern "C" fn() -> u32;
pub type MechModuleNameFnV1 = unsafe extern "C" fn(out: *mut MechStrV1) -> MechStatusV1;
pub type MechModuleExportCountFnV1 = unsafe extern "C" fn() -> usize;
pub type MechModuleGetExportFnV1 =
    unsafe extern "C" fn(index: usize, out: *mut MechExportV1) -> MechStatusV1;

/// Generates the standard V1 dynamic module metadata and export symbols.
///
/// The generated ABI exposes only module metadata and typed scalar kernel
/// pointers. Export order is the declaration order in the macro invocation.
#[macro_export]
macro_rules! mech_dynamic_module_v1 {
    (
        module: $module_name:expr,
        exports: [
            $(
                $kind:ident {
                    name: $export_name:expr,
                    function: $function:path $(,)?
                }
            ),* $(,)?
        ],
    ) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn mech_module_abi_version_v1() -> u32 {
            $crate::MECH_MODULE_ABI_VERSION_V1
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn mech_module_name_v1(
            out: *mut $crate::MechStrV1,
        ) -> $crate::MechStatusV1 {
            if out.is_null() {
                return $crate::MechStatusV1::NullPointer;
            }

            unsafe {
                *out = $crate::MechStrV1::from_static($module_name);
            }

            $crate::MechStatusV1::Ok
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn mech_module_export_count_v1() -> usize {
            <[()]>::len(&[$($crate::mech_dynamic_module_v1!(@unit $kind)),*])
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn mech_module_get_export_v1(
            index: usize,
            out: *mut $crate::MechExportV1,
        ) -> $crate::MechStatusV1 {
            if out.is_null() {
                return $crate::MechStatusV1::NullPointer;
            }

            let exports: &[$crate::MechExportV1] = &[
                $(
                    $crate::mech_dynamic_module_v1!(
                        @export $kind, $export_name, $function
                    )
                ),*
            ];

            let Some(export) = exports.get(index).copied() else {
                return $crate::MechStatusV1::InvalidIndex;
            };

            unsafe {
                *out = export;
            }

            $crate::MechStatusV1::Ok
        }
    };

    (@unit $kind:ident) => { () };

    (@export unary_f64_to_f64, $export_name:expr, $function:path) => {
        $crate::MechExportV1 {
            name: $crate::MechStrV1::from_static($export_name),
            kind: $crate::MechKernelKindV1::UnaryF64ToF64,
            function: $crate::MechKernelFnV1 {
                unary_f64_to_f64: $function,
            },
        }
    };

    (@export binary_f64_f64_to_f64, $export_name:expr, $function:path) => {
        $crate::MechExportV1 {
            name: $crate::MechStrV1::from_static($export_name),
            kind: $crate::MechKernelKindV1::BinaryF64F64ToF64,
            function: $crate::MechKernelFnV1 {
                binary_f64_f64_to_f64: $function,
            },
        }
    };

    (@export unary_f64_slice_to_f64_slice, $export_name:expr, $function:path) => {
        $crate::MechExportV1 {
            name: $crate::MechStrV1::from_static($export_name),
            kind: $crate::MechKernelKindV1::UnaryF64SliceToF64Slice,
            function: $crate::MechKernelFnV1 {
                unary_f64_slice_to_f64_slice: $function,
            },
        }
    };
}
