use core::ffi::c_void;
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
            function: combinatorics_n_choose_k_f64_v1 as *const c_void,
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

    if !n.is_finite() || !k.is_finite() {
        return MechStatusV1::WrongType;
    }

    if k < 0.0 || n < 0.0 {
        unsafe {
            *out = 0.0;
        }
        return MechStatusV1::Ok;
    }

    if k > n {
        unsafe {
            *out = 0.0;
        }
        return MechStatusV1::Ok;
    }

    let mut result = 1.0;
    let mut i = 0.0;

    while i < k {
        let numerator = n - i;
        let denominator = i + 1.0;
        result = result * numerator / denominator;
        i += 1.0;
    }

    unsafe {
        *out = result;
    }

    MechStatusV1::Ok
}
