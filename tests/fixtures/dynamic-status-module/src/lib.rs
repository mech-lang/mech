use mech_abi::{MechF64ViewMutV1, MechF64ViewV1, MechStatusV1};

mech_abi::mech_dynamic_module_v1! {
    module: b"status-test",
    exports: [
        unary_f64_to_f64 {
            name: b"status-test/unary",
            function: status_test_unary,
        },
        binary_f64_f64_to_f64 {
            name: b"status-test/binary",
            function: status_test_binary,
        },
        unary_f64_view_to_f64_view {
            name: b"status-test/view",
            function: status_test_view,
        },
    ],
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn status_test_unary(input: f64, out: *mut f64) -> MechStatusV1 {
    if out.is_null() {
        return MechStatusV1::NullPointer;
    }

    unsafe {
        *out = if input == 2.0 { 999.0 } else { input * 10.0 };
    }

    if input == 2.0 {
        MechStatusV1::WrongShape
    } else {
        MechStatusV1::Ok
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn status_test_binary(lhs: f64, rhs: f64, out: *mut f64) -> MechStatusV1 {
    if out.is_null() {
        return MechStatusV1::NullPointer;
    }

    unsafe {
        *out = if lhs == 2.0 { 999.0 } else { lhs + rhs };
    }

    if lhs == 2.0 {
        MechStatusV1::Unsupported
    } else {
        MechStatusV1::Ok
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn status_test_view(
    input: MechF64ViewV1,
    out: MechF64ViewMutV1,
) -> MechStatusV1 {
    let Some(expected_len) = input.rows.checked_mul(input.cols) else {
        return MechStatusV1::WrongShape;
    };

    if input.len != out.len
        || input.rows != out.rows
        || input.cols != out.cols
        || input.len != expected_len
    {
        return MechStatusV1::WrongShape;
    }

    if input.len > 0 && (input.ptr.is_null() || out.ptr.is_null()) {
        return MechStatusV1::NullPointer;
    }

    if input.len == 0 {
        return MechStatusV1::Ok;
    }

    let input_values = unsafe { std::slice::from_raw_parts(input.ptr, input.len) };
    let output_values = unsafe { std::slice::from_raw_parts_mut(out.ptr, out.len) };
    let should_fail = input_values.iter().any(|value| *value == 2.0);

    for (output, value) in output_values.iter_mut().zip(input_values) {
        *output = if should_fail { 999.0 } else { value * 10.0 };
    }

    if should_fail {
        MechStatusV1::PANIC
    } else {
        MechStatusV1::Ok
    }
}
