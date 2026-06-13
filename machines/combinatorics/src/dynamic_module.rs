#![cfg(feature = "dynamic-module")]

use mech_abi::{
    MechExportV1, MechKernelKindV1, MechStatusV1, MechStrV1, MECH_MODULE_ABI_VERSION_V1,
};

const MODULE_NAME: &[u8] = b"combinatorics";
const EXPORT_NAME: &[u8] = b"combinatorics/n-choose-k";

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
            kind: MechKernelKindV1::BinaryF64F64ToF64,
            binary_f64_f64_to_f64: combinatorics_n_choose_k_f64_v1,
        };
    }

    MechStatusV1::Ok
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn combinatorics_n_choose_k_f64_v1(
    n: f64,
    k: f64,
    out: *mut f64,
) -> MechStatusV1 {
    if out.is_null() {
        return MechStatusV1::NullPointer;
    }

    unsafe {
        *out = crate::kernels::n_choose_k::scalar(n, k);
    }

    MechStatusV1::Ok
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_name_null_pointer_returns_null_pointer() {
        let status = unsafe { mech_module_name_v1(core::ptr::null_mut()) };

        assert_eq!(status, MechStatusV1::NullPointer);
    }

    #[test]
    fn export_metadata_describes_n_choose_k_kernel() {
        let mut export = MechExportV1 {
            name: MechStrV1 {
                ptr: core::ptr::null(),
                len: 0,
            },
            kind: MechKernelKindV1::BinaryF64F64ToF64,
            binary_f64_f64_to_f64: combinatorics_n_choose_k_f64_v1,
        };
        let status = unsafe { mech_module_get_export_v1(0, &mut export) };
        let name = unsafe { core::slice::from_raw_parts(export.name.ptr, export.name.len) };

        assert_eq!(status, MechStatusV1::Ok);
        assert_eq!(name, EXPORT_NAME);
        assert_eq!(export.kind, MechKernelKindV1::BinaryF64F64ToF64);
    }

    #[test]
    fn n_choose_k_f64_returns_expected_result() {
        let mut out = 0.0;
        let status = unsafe { combinatorics_n_choose_k_f64_v1(10.0, 2.0, &mut out) };

        assert_eq!(status, MechStatusV1::Ok);
        assert_eq!(out, crate::kernels::n_choose_k::scalar(10.0_f64, 2.0_f64));
    }

    #[test]
    fn n_choose_k_returns_zero_when_k_exceeds_n() {
        let mut out = 123.0;
        let status = unsafe { combinatorics_n_choose_k_f64_v1(2.0, 10.0, &mut out) };

        assert_eq!(status, MechStatusV1::Ok);
        assert_eq!(out, crate::kernels::n_choose_k::scalar(2.0_f64, 10.0_f64));
    }
}
