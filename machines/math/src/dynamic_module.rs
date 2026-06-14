#![cfg(feature = "dynamic-module")]

use mech_abi::{
    MechExportV1, MechF64ViewMutV1, MechF64ViewV1, MechKernelFnV1, MechKernelKindV1, MechStatusV1,
    MechStrV1,
};

const MODULE_NAME: &[u8] = b"math";
const EXPORT_NAME: &[u8] = b"math/round";
const SQRT_EXPORT_NAME: &[u8] = b"math/sqrt";
const FLOOR_EXPORT_NAME: &[u8] = b"math/floor";
const CEIL_EXPORT_NAME: &[u8] = b"math/ceil";
const ATAN2_EXPORT_NAME: &[u8] = b"math/atan2";

macro_rules! define_unary_f64_dynamic_kernels {
    ($scalar_symbol:ident, $view_symbol:ident, $kernel_mod:ident) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $scalar_symbol(input: f64, out: *mut f64) -> MechStatusV1 {
            unsafe { call_unary_f64(input, out, crate::kernels::$kernel_mod::scalar_f64) }
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $view_symbol(
            input: MechF64ViewV1,
            out: MechF64ViewMutV1,
        ) -> MechStatusV1 {
            unsafe { call_unary_f64_view(input, out, crate::kernels::$kernel_mod::view_f64) }
        }
    };
}

macro_rules! define_binary_f64_dynamic_kernel {
    ($symbol:ident, $kernel_mod:ident) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $symbol(lhs: f64, rhs: f64, out: *mut f64) -> MechStatusV1 {
            unsafe { call_binary_f64(lhs, rhs, out, crate::kernels::$kernel_mod::scalar_f64) }
        }
    };
}

macro_rules! math_dynamic_module_v1 {
    (
        unary: [
            $(
                $unary_name:expr => ($unary_scalar:path, $unary_view:path)
            ),* $(,)?
        ],
        binary: [
            $(
                $binary_name:expr => $binary_scalar:path
            ),* $(,)?
        ],
    ) => {
        mech_abi::mech_dynamic_module_v1! {
            module: b"math",
            exports: [
                $(
                    unary_f64_to_f64 {
                        name: $unary_name,
                        function: $unary_scalar,
                    },
                    unary_f64_view_to_f64_view {
                        name: $unary_name,
                        function: $unary_view,
                    },
                )*
                $(
                    binary_f64_f64_to_f64 {
                        name: $binary_name,
                        function: $binary_scalar,
                    },
                )*
            ],
        }
    };
}

math_dynamic_module_v1! {
    unary: [
        b"math/round" => (math_round_f64_v1, math_round_f64_view_v1),
        b"math/sqrt" => (math_sqrt_f64_v1, math_sqrt_f64_view_v1),
        b"math/floor" => (math_floor_f64_v1, math_floor_f64_view_v1),
        b"math/ceil" => (math_ceil_f64_v1, math_ceil_f64_view_v1),
    ],
    binary: [
        b"math/atan2" => math_atan2_f64_v1,
    ],
}

unsafe fn call_unary_f64(input: f64, out: *mut f64, kernel: fn(f64) -> f64) -> MechStatusV1 {
    if out.is_null() {
        return MechStatusV1::NullPointer;
    }

    unsafe {
        *out = kernel(input);
    }

    MechStatusV1::Ok
}

unsafe fn call_binary_f64(
    lhs: f64,
    rhs: f64,
    out: *mut f64,
    kernel: fn(f64, f64) -> f64,
) -> MechStatusV1 {
    if out.is_null() {
        return MechStatusV1::NullPointer;
    }

    unsafe {
        *out = kernel(lhs, rhs);
    }

    MechStatusV1::Ok
}

unsafe fn call_unary_f64_view(
    input: MechF64ViewV1,
    out: MechF64ViewMutV1,
    kernel: fn(&[f64], &mut [f64]),
) -> MechStatusV1 {
    if input.len != out.len {
        return MechStatusV1::WrongShape;
    }

    if input.rows != out.rows || input.cols != out.cols {
        return MechStatusV1::WrongShape;
    }

    let Some(input_cells) = input.rows.checked_mul(input.cols) else {
        return MechStatusV1::WrongShape;
    };

    let Some(out_cells) = out.rows.checked_mul(out.cols) else {
        return MechStatusV1::WrongShape;
    };

    if input.len != input_cells || out.len != out_cells {
        return MechStatusV1::WrongShape;
    }

    if input.len == 0 {
        return MechStatusV1::Ok;
    }

    if input.ptr.is_null() || out.ptr.is_null() {
        return MechStatusV1::NullPointer;
    }

    let input_slice = unsafe { core::slice::from_raw_parts(input.ptr, input.len) };
    let out_slice = unsafe { core::slice::from_raw_parts_mut(out.ptr, out.len) };

    kernel(input_slice, out_slice);

    MechStatusV1::Ok
}

define_unary_f64_dynamic_kernels!(
    math_round_f64_v1,
    math_round_f64_view_v1,
    round
);

define_unary_f64_dynamic_kernels!(
    math_sqrt_f64_v1,
    math_sqrt_f64_view_v1,
    sqrt
);

define_unary_f64_dynamic_kernels!(
    math_floor_f64_v1,
    math_floor_f64_view_v1,
    floor
);

define_unary_f64_dynamic_kernels!(
    math_ceil_f64_v1,
    math_ceil_f64_view_v1,
    ceil
);

define_binary_f64_dynamic_kernel!(
    math_atan2_f64_v1,
    atan2
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_name_null_pointer_returns_null_pointer() {
        let status = unsafe { mech_module_name_v1(core::ptr::null_mut()) };
        assert_eq!(status, MechStatusV1::NullPointer);
    }

    #[test]
    fn module_metadata_reports_module_name() {
        let mut module_name = MechStrV1 {
            ptr: core::ptr::null(),
            len: 0,
        };
        let status = unsafe { mech_module_name_v1(&mut module_name) };
        let name = unsafe { core::slice::from_raw_parts(module_name.ptr, module_name.len) };
        assert_eq!(status, MechStatusV1::Ok);
        assert_eq!(name, MODULE_NAME);
    }

    #[test]
    fn module_export_count_is_nine() {
        assert_eq!(unsafe { mech_module_export_count_v1() }, 9);
    }

    #[test]
    fn module_get_export_rejects_null_output() {
        let status = unsafe { mech_module_get_export_v1(0, core::ptr::null_mut()) };
        assert_eq!(status, MechStatusV1::NullPointer);
    }

    #[test]
    fn module_get_export_rejects_invalid_index() {
        let mut export = MechExportV1 {
            name: MechStrV1 {
                ptr: core::ptr::null(),
                len: 0,
            },
            kind: MechKernelKindV1::UnaryF64ToF64,
            function: MechKernelFnV1 {
                unary_f64_to_f64: math_round_f64_v1,
            },
        };
        let status = unsafe { mech_module_get_export_v1(9, &mut export) };
        assert_eq!(status, MechStatusV1::InvalidIndex);
    }

    #[test]
    fn export_metadata_describes_round_kernel() {
        let mut export = MechExportV1 {
            name: MechStrV1 {
                ptr: core::ptr::null(),
                len: 0,
            },
            kind: MechKernelKindV1::UnaryF64ToF64,
            function: MechKernelFnV1 {
                unary_f64_to_f64: math_round_f64_v1,
            },
        };
        let status = unsafe { mech_module_get_export_v1(0, &mut export) };
        let name = unsafe { core::slice::from_raw_parts(export.name.ptr, export.name.len) };
        assert_eq!(status, MechStatusV1::Ok);
        assert_eq!(name, EXPORT_NAME);
        assert_eq!(export.kind, MechKernelKindV1::UnaryF64ToF64);
    }

    #[test]
    fn export_metadata_describes_round_view_kernel() {
        let mut export = MechExportV1 {
            name: MechStrV1 {
                ptr: core::ptr::null(),
                len: 0,
            },
            kind: MechKernelKindV1::UnaryF64ViewToF64View,
            function: MechKernelFnV1 {
                unary_f64_view_to_f64_view: math_round_f64_view_v1,
            },
        };
        let status = unsafe { mech_module_get_export_v1(1, &mut export) };
        let name = unsafe { core::slice::from_raw_parts(export.name.ptr, export.name.len) };
        assert_eq!(status, MechStatusV1::Ok);
        assert_eq!(name, EXPORT_NAME);
        assert_eq!(export.kind, MechKernelKindV1::UnaryF64ViewToF64View);
    }

    #[test]
    fn export_metadata_describes_sqrt_scalar_kernel() {
        let mut export = MechExportV1 {
            name: MechStrV1 {
                ptr: core::ptr::null(),
                len: 0,
            },
            kind: MechKernelKindV1::UnaryF64ToF64,
            function: MechKernelFnV1 {
                unary_f64_to_f64: math_sqrt_f64_v1,
            },
        };
        let status = unsafe { mech_module_get_export_v1(2, &mut export) };
        let name = unsafe { core::slice::from_raw_parts(export.name.ptr, export.name.len) };
        assert_eq!(status, MechStatusV1::Ok);
        assert_eq!(name, SQRT_EXPORT_NAME);
        assert_eq!(export.kind, MechKernelKindV1::UnaryF64ToF64);
    }

    #[test]
    fn export_metadata_describes_sqrt_view_kernel() {
        let mut export = MechExportV1 {
            name: MechStrV1 {
                ptr: core::ptr::null(),
                len: 0,
            },
            kind: MechKernelKindV1::UnaryF64ViewToF64View,
            function: MechKernelFnV1 {
                unary_f64_view_to_f64_view: math_sqrt_f64_view_v1,
            },
        };
        let status = unsafe { mech_module_get_export_v1(3, &mut export) };
        let name = unsafe { core::slice::from_raw_parts(export.name.ptr, export.name.len) };
        assert_eq!(status, MechStatusV1::Ok);
        assert_eq!(name, SQRT_EXPORT_NAME);
        assert_eq!(export.kind, MechKernelKindV1::UnaryF64ViewToF64View);
    }

    #[test]
    fn export_metadata_describes_atan2_kernel() {
        let mut export = MechExportV1 {
            name: MechStrV1 {
                ptr: core::ptr::null(),
                len: 0,
            },
            kind: MechKernelKindV1::BinaryF64F64ToF64,
            function: MechKernelFnV1 {
                binary_f64_f64_to_f64: math_atan2_f64_v1,
            },
        };
        let status = unsafe { mech_module_get_export_v1(8, &mut export) };
        let name = unsafe { core::slice::from_raw_parts(export.name.ptr, export.name.len) };
        assert_eq!(status, MechStatusV1::Ok);
        assert_eq!(name, ATAN2_EXPORT_NAME);
        assert_eq!(export.kind, MechKernelKindV1::BinaryF64F64ToF64);
    }

    #[test]
    fn math_round_f64_returns_expected_result() {
        let mut out = 0.0;
        let status = unsafe { math_round_f64_v1(1.23, &mut out) };
        assert_eq!(status, MechStatusV1::Ok);
        assert_eq!(out, 1.0);
    }

    #[test]
    fn math_sqrt_f64_returns_expected_result() {
        let mut out = 0.0;
        let status = unsafe { math_sqrt_f64_v1(9.0, &mut out) };
        assert_eq!(status, MechStatusV1::Ok);
        assert_eq!(out, 3.0);
    }

    #[test]
    fn math_floor_f64_returns_expected_result() {
        let mut out = 0.0;
        let status = unsafe { math_floor_f64_v1(4.56, &mut out) };
        assert_eq!(status, MechStatusV1::Ok);
        assert_eq!(out, 4.0);
    }

    #[test]
    fn math_ceil_f64_returns_expected_result() {
        let mut out = 0.0;
        let status = unsafe { math_ceil_f64_v1(4.56, &mut out) };
        assert_eq!(status, MechStatusV1::Ok);
        assert_eq!(out, 5.0);
    }

    #[test]
    fn math_atan2_f64_returns_expected_result() {
        let mut out = 1.0;
        let status = unsafe { math_atan2_f64_v1(0.0, 1.0, &mut out) };
        assert_eq!(status, MechStatusV1::Ok);
        assert_eq!(out, 0.0);
    }

    #[test]
    fn math_sqrt_f64_view_returns_expected_result() {
        let input = [1.0, 4.0, 9.0];
        let mut out = [0.0, 0.0, 0.0];
        let status = unsafe {
            math_sqrt_f64_view_v1(
                MechF64ViewV1 {
                    ptr: input.as_ptr(),
                    len: input.len(),
                    rows: 1,
                    cols: 3,
                },
                MechF64ViewMutV1 {
                    ptr: out.as_mut_ptr(),
                    len: out.len(),
                    rows: 1,
                    cols: 3,
                },
            )
        };
        assert_eq!(status, MechStatusV1::Ok);
        assert_eq!(out, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn math_round_f64_rejects_null_output() {
        let status = unsafe { math_round_f64_v1(1.23, core::ptr::null_mut()) };
        assert_eq!(status, MechStatusV1::NullPointer);
    }

    #[test]
    fn math_round_f64_view_returns_expected_result() {
        let input = [1.23, 2.7, 3.1];
        let mut out = [0.0, 0.0, 0.0];

        let status = unsafe {
            math_round_f64_view_v1(
                MechF64ViewV1 {
                    ptr: input.as_ptr(),
                    len: input.len(),
                    rows: 1,
                    cols: 3,
                },
                MechF64ViewMutV1 {
                    ptr: out.as_mut_ptr(),
                    len: out.len(),
                    rows: 1,
                    cols: 3,
                },
            )
        };

        assert_eq!(status, MechStatusV1::Ok);
        assert_eq!(out, [1.0, 3.0, 3.0]);
    }

    #[test]
    fn math_round_f64_view_rejects_null_input() {
        let mut out = [0.0, 0.0, 0.0];

        let status = unsafe {
            math_round_f64_view_v1(
                MechF64ViewV1 {
                    ptr: core::ptr::null(),
                    len: out.len(),
                    rows: 1,
                    cols: 3,
                },
                MechF64ViewMutV1 {
                    ptr: out.as_mut_ptr(),
                    len: out.len(),
                    rows: 1,
                    cols: 3,
                },
            )
        };

        assert_eq!(status, MechStatusV1::NullPointer);
    }

    #[test]
    fn math_round_f64_view_rejects_null_output() {
        let input = [1.23, 2.7, 3.1];

        let status = unsafe {
            math_round_f64_view_v1(
                MechF64ViewV1 {
                    ptr: input.as_ptr(),
                    len: input.len(),
                    rows: 1,
                    cols: 3,
                },
                MechF64ViewMutV1 {
                    ptr: core::ptr::null_mut(),
                    len: input.len(),
                    rows: 1,
                    cols: 3,
                },
            )
        };

        assert_eq!(status, MechStatusV1::NullPointer);
    }

    #[test]
    fn math_round_f64_view_rejects_wrong_len() {
        let input = [1.23, 2.7, 3.1];
        let mut out = [0.0, 0.0];

        let status = unsafe {
            math_round_f64_view_v1(
                MechF64ViewV1 {
                    ptr: input.as_ptr(),
                    len: input.len(),
                    rows: 1,
                    cols: 3,
                },
                MechF64ViewMutV1 {
                    ptr: out.as_mut_ptr(),
                    len: out.len(),
                    rows: 1,
                    cols: 2,
                },
            )
        };

        assert_eq!(status, MechStatusV1::WrongShape);
    }

    #[test]
    fn math_round_f64_view_rejects_mismatched_shape() {
        let input = [1.23, 2.7, 3.1, 4.2];
        let mut out = [0.0, 0.0, 0.0, 0.0];

        let status = unsafe {
            math_round_f64_view_v1(
                MechF64ViewV1 {
                    ptr: input.as_ptr(),
                    len: input.len(),
                    rows: 2,
                    cols: 2,
                },
                MechF64ViewMutV1 {
                    ptr: out.as_mut_ptr(),
                    len: out.len(),
                    rows: 1,
                    cols: 4,
                },
            )
        };

        assert_eq!(status, MechStatusV1::WrongShape);
    }

    #[test]
    fn math_round_f64_view_rejects_len_shape_mismatch() {
        let input = [1.23, 2.7, 3.1];
        let mut out = [0.0, 0.0, 0.0];

        let status = unsafe {
            math_round_f64_view_v1(
                MechF64ViewV1 {
                    ptr: input.as_ptr(),
                    len: input.len(),
                    rows: 3,
                    cols: 3,
                },
                MechF64ViewMutV1 {
                    ptr: out.as_mut_ptr(),
                    len: out.len(),
                    rows: 3,
                    cols: 3,
                },
            )
        };

        assert_eq!(status, MechStatusV1::WrongShape);
    }

    #[test]
    fn math_round_f64_view_accepts_empty_views() {
        let status = unsafe {
            math_round_f64_view_v1(
                MechF64ViewV1 {
                    ptr: core::ptr::null(),
                    len: 0,
                    rows: 0,
                    cols: 0,
                },
                MechF64ViewMutV1 {
                    ptr: core::ptr::null_mut(),
                    len: 0,
                    rows: 0,
                    cols: 0,
                },
            )
        };

        assert_eq!(status, MechStatusV1::Ok);
    }
}
