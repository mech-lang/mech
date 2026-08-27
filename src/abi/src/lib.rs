pub const MECH_MODULE_ABI_VERSION_V1: u32 = 1;

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MechStatusV1(pub i32);

impl MechStatusV1 {
    pub const OK: Self = Self(0);
    pub const INVALID_INDEX: Self = Self(1);
    pub const NULL_POINTER: Self = Self(2);
    pub const WRONG_TYPE: Self = Self(3);
    pub const WRONG_SHAPE: Self = Self(4);
    pub const UNSUPPORTED: Self = Self(5);
    pub const PANIC: Self = Self(6);
}

#[expect(
    non_upper_case_globals,
    reason = "Mech module ABI V1 source names are frozen compatibility aliases"
)]
impl MechStatusV1 {
    pub const Ok: Self = Self::OK;
    pub const InvalidIndex: Self = Self::INVALID_INDEX;
    pub const NullPointer: Self = Self::NULL_POINTER;
    pub const WrongType: Self = Self::WRONG_TYPE;
    pub const WrongShape: Self = Self::WRONG_SHAPE;
    pub const Unsupported: Self = Self::UNSUPPORTED;
    pub const Panic: Self = Self::PANIC;
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

/// Borrowed contiguous f64 view owned by the host.
///
/// The view is valid only for the duration of the ABI call.
/// `len` must equal `rows * cols`.
/// The dynamic module must not retain the pointer.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct MechF64ViewV1 {
    pub ptr: *const f64,
    pub len: usize,
    pub rows: usize,
    pub cols: usize,
}

/// Mutable contiguous f64 view owned by the host.
///
/// The view is valid only for the duration of the ABI call.
/// `len` must equal `rows * cols`.
/// The dynamic module must not retain the pointer.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct MechF64ViewMutV1 {
    pub ptr: *mut f64,
    pub len: usize,
    pub rows: usize,
    pub cols: usize,
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
/// tagged union keyed by this raw integer newtype. The host must read only the union field
/// corresponding to `kind`.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MechKernelKindV1(pub u32);

impl MechKernelKindV1 {
    pub const UNARY_F64_TO_F64: Self = Self(1);
    pub const BINARY_F64_F64_TO_F64: Self = Self(2);
    pub const UNARY_F64_VIEW_TO_F64_VIEW: Self = Self(3);
}

#[expect(
    non_upper_case_globals,
    reason = "Mech module ABI V1 source names are frozen compatibility aliases"
)]
impl MechKernelKindV1 {
    pub const UnaryF64ToF64: Self = Self::UNARY_F64_TO_F64;
    pub const BinaryF64F64ToF64: Self = Self::BINARY_F64_F64_TO_F64;
    pub const UnaryF64ViewToF64View: Self = Self::UNARY_F64_VIEW_TO_F64_VIEW;
}

/// Kernel for a unary scalar f64 function.
///
/// The host owns all Mech runtime values. The module receives one copied
/// scalar input and writes one scalar result into `out`.
/// The module must not retain `out` after returning.
pub type MechUnaryF64ToF64KernelV1 =
    unsafe extern "C" fn(input: f64, out: *mut f64) -> MechStatusV1;

pub type MechUnaryF64ViewToF64ViewKernelV1 =
    unsafe extern "C" fn(input: MechF64ViewV1, out: MechF64ViewMutV1) -> MechStatusV1;

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
    pub unary_f64_view_to_f64_view: MechUnaryF64ViewToF64ViewKernelV1,
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
                return $crate::MechStatusV1::NULL_POINTER;
            }

            unsafe {
                *out = $crate::MechStrV1::from_static($module_name);
            }

            $crate::MechStatusV1::OK
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
                return $crate::MechStatusV1::NULL_POINTER;
            }

            let exports: &[$crate::MechExportV1] = &[
                $(
                    $crate::mech_dynamic_module_v1!(
                        @export $kind, $export_name, $function
                    )
                ),*
            ];

            let Some(export) = exports.get(index).copied() else {
                return $crate::MechStatusV1::INVALID_INDEX;
            };

            unsafe {
                *out = export;
            }

            $crate::MechStatusV1::OK
        }
    };

    (@unit $kind:ident) => { () };

    (@export unary_f64_to_f64, $export_name:expr, $function:path) => {
        $crate::MechExportV1 {
            name: $crate::MechStrV1::from_static($export_name),
            kind: $crate::MechKernelKindV1::UNARY_F64_TO_F64,
            function: $crate::MechKernelFnV1 {
                unary_f64_to_f64: $function,
            },
        }
    };

    (@export binary_f64_f64_to_f64, $export_name:expr, $function:path) => {
        $crate::MechExportV1 {
            name: $crate::MechStrV1::from_static($export_name),
            kind: $crate::MechKernelKindV1::BINARY_F64_F64_TO_F64,
            function: $crate::MechKernelFnV1 {
                binary_f64_f64_to_f64: $function,
            },
        }
    };

    (@export unary_f64_view_to_f64_view, $export_name:expr, $function:path) => {
        $crate::MechExportV1 {
            name: $crate::MechStrV1::from_static($export_name),
            kind: $crate::MechKernelKindV1::UNARY_F64_VIEW_TO_F64_VIEW,
            function: $crate::MechKernelFnV1 {
                unary_f64_view_to_f64_view: $function,
            },
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dynamic_abi_constants_retain_numeric_values() {
        assert_eq!(MechStatusV1::OK.0, 0);
        assert_eq!(MechStatusV1::INVALID_INDEX.0, 1);
        assert_eq!(MechStatusV1::NULL_POINTER.0, 2);
        assert_eq!(MechStatusV1::WRONG_TYPE.0, 3);
        assert_eq!(MechStatusV1::WRONG_SHAPE.0, 4);
        assert_eq!(MechStatusV1::UNSUPPORTED.0, 5);
        assert_eq!(MechStatusV1::PANIC.0, 6);
        assert_eq!(MechKernelKindV1::UNARY_F64_TO_F64.0, 1);
        assert_eq!(MechKernelKindV1::BINARY_F64_F64_TO_F64.0, 2);
        assert_eq!(MechKernelKindV1::UNARY_F64_VIEW_TO_F64_VIEW.0, 3);
    }

    #[test]
    fn v1_source_compatibility_aliases_are_stable() {
        assert_eq!(MechStatusV1::Ok, MechStatusV1::OK);
        assert_eq!(MechStatusV1::InvalidIndex, MechStatusV1::INVALID_INDEX);
        assert_eq!(MechStatusV1::NullPointer, MechStatusV1::NULL_POINTER);
        assert_eq!(MechStatusV1::WrongType, MechStatusV1::WRONG_TYPE);
        assert_eq!(MechStatusV1::WrongShape, MechStatusV1::WRONG_SHAPE);
        assert_eq!(MechStatusV1::Unsupported, MechStatusV1::UNSUPPORTED);
        assert_eq!(MechStatusV1::Panic, MechStatusV1::PANIC);
        assert_eq!(
            MechKernelKindV1::UnaryF64ToF64,
            MechKernelKindV1::UNARY_F64_TO_F64
        );
        assert_eq!(
            MechKernelKindV1::BinaryF64F64ToF64,
            MechKernelKindV1::BINARY_F64_F64_TO_F64
        );
        assert_eq!(
            MechKernelKindV1::UnaryF64ViewToF64View,
            MechKernelKindV1::UNARY_F64_VIEW_TO_F64_VIEW
        );
    }
}
