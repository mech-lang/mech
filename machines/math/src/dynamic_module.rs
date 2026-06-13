#![cfg(feature = "dynamic-module")]

use mech_abi::{
    MechExportV1, MechKernelFnV1, MechKernelKindV1, MechStatusV1, MechStrV1,
    MECH_MODULE_ABI_VERSION_V1,
};

const MODULE_NAME: &[u8] = b"math";
const EXPORT_NAME: &[u8] = b"math/round";

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mech_module_abi_version_v1() -> u32 {
    MECH_MODULE_ABI_VERSION_V1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mech_module_name_v1(out: *mut MechStrV1) -> MechStatusV1 {
    if out.is_null() {
        return MechStatusV1::NullPointer;
    }

    unsafe {
        *out = MechStrV1::from_static(MODULE_NAME);
    }

    MechStatusV1::Ok
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mech_module_export_count_v1() -> usize {
    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mech_module_get_export_v1(
    index: usize,
    out: *mut MechExportV1,
) -> MechStatusV1 {
    if out.is_null() {
        return MechStatusV1::NullPointer;
    }

    if index != 0 {
        return MechStatusV1::InvalidIndex;
    }

    unsafe {
        *out = MechExportV1 {
            name: MechStrV1::from_static(EXPORT_NAME),
            kind: MechKernelKindV1::UnaryF64ToF64,
            function: MechKernelFnV1 {
                unary_f64_to_f64: math_round_f64_v1,
            },
        };
    }

    MechStatusV1::Ok
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn math_round_f64_v1(input: f64, out: *mut f64) -> MechStatusV1 {
    if out.is_null() {
        return MechStatusV1::NullPointer;
    }

    unsafe {
        *out = crate::kernels::round::scalar_f64(input);
    }

    MechStatusV1::Ok
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_metadata_reports_module_name() {
        let mut module_name = MechStrV1 { ptr: core::ptr::null(), len: 0 };
        let status = unsafe { mech_module_name_v1(&mut module_name) };
        let name = unsafe { core::slice::from_raw_parts(module_name.ptr, module_name.len) };
        assert_eq!(status, MechStatusV1::Ok);
        assert_eq!(name, MODULE_NAME);
    }

    #[test]
    fn module_export_count_is_one() {
        assert_eq!(unsafe { mech_module_export_count_v1() }, 1);
    }

    #[test]
    fn export_metadata_describes_round_kernel() {
        let mut export = MechExportV1 {
            name: MechStrV1 { ptr: core::ptr::null(), len: 0 },
            kind: MechKernelKindV1::UnaryF64ToF64,
            function: MechKernelFnV1 { unary_f64_to_f64: math_round_f64_v1 },
        };
        let status = unsafe { mech_module_get_export_v1(0, &mut export) };
        let name = unsafe { core::slice::from_raw_parts(export.name.ptr, export.name.len) };
        assert_eq!(status, MechStatusV1::Ok);
        assert_eq!(name, EXPORT_NAME);
        assert_eq!(export.kind, MechKernelKindV1::UnaryF64ToF64);
    }

    #[test]
    fn math_round_f64_returns_expected_result() {
        let mut out = 0.0;
        let status = unsafe { math_round_f64_v1(1.23, &mut out) };
        assert_eq!(status, MechStatusV1::Ok);
        assert_eq!(out, 1.0);
    }

    #[test]
    fn math_round_f64_rejects_null_output() {
        let status = unsafe { math_round_f64_v1(1.23, core::ptr::null_mut()) };
        assert_eq!(status, MechStatusV1::NullPointer);
    }
}
