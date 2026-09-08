use crate::*;
use num_traits::*;

// Div ------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MathDivisionInvalid {
    pub operand_type: &'static str,
}

impl MechErrorKind for MathDivisionInvalid {
    fn name(&self) -> &str {
        "MathDivisionInvalid"
    }

    fn message(&self) -> String {
        format!(
            "division is undefined or overflows for operand type {}",
            self.operand_type,
        )
    }
}

pub trait RuntimeCheckedDiv: Copy {
    fn runtime_checked_div(self, rhs: Self) -> Option<Self>;
}

macro_rules! impl_checked_integer_div {
    ($($type:ty),+ $(,)?) => {
        $(
            impl RuntimeCheckedDiv for $type {
                fn runtime_checked_div(self, rhs: Self) -> Option<Self> {
                    self.checked_div(rhs)
                }
            }
        )+
    };
}

impl_checked_integer_div!(i8, i16, i32, i64, i128, u8, u16, u32, u64, u128);

impl RuntimeCheckedDiv for f32 {
    fn runtime_checked_div(self, rhs: Self) -> Option<Self> {
        Some(self / rhs)
    }
}

impl RuntimeCheckedDiv for f64 {
    fn runtime_checked_div(self, rhs: Self) -> Option<Self> {
        Some(self / rhs)
    }
}

#[cfg(feature = "rational")]
impl RuntimeCheckedDiv for R64 {
    fn runtime_checked_div(self, rhs: Self) -> Option<Self> {
        self.checked_div(rhs)
    }
}

#[cfg(feature = "complex")]
impl RuntimeCheckedDiv for C64 {
    fn runtime_checked_div(self, rhs: Self) -> Option<Self> {
        Some(self / rhs)
    }
}

fn checked_runtime_div<T: RuntimeCheckedDiv>(lhs: T, rhs: T) -> MResult<T> {
    lhs.runtime_checked_div(rhs).ok_or_else(|| {
        MechError::new(
            MathDivisionInvalid {
                operand_type: std::any::type_name::<T>(),
            },
            None,
        )
        .with_compiler_loc()
    })
}

macro_rules! managed_div_op {
    (@managed $lhs:expr, $rhs:expr) => {
        checked_runtime_div($lhs, $rhs)
    };
}

macro_rules! impl_checked_div_binop {
    ($struct_name:ident, $arg1_type:ty, $arg2_type:ty, $out_type:ty, $op:ident) => {
        impl_checked_arithmetic_binop!(@bound RuntimeCheckedDiv;
            $struct_name, $arg1_type, $arg2_type, $out_type, managed_div_op,
            crate::ops::arithmetic_full_write_contract);
    };
}

impl_fxns!(Div, T, T, impl_checked_div_binop);

#[cfg(all(test, feature = "i32"))]
mod tests {
    use super::*;

    #[test]
    fn integer_division_rejects_zero_and_signed_overflow_on_reactive_resolve() {
        let lhs = ValueCell::from_exact(i32::MIN).unwrap();
        let rhs = ValueCell::from_exact(2_i32).unwrap();
        let out = ValueCell::from_exact(17_i32).unwrap();
        let function = crate::catalog::bind_test_binary::<DivSS<i32>>(
            "math/div",
            "DivSS<i32>",
            lhs,
            rhs.clone(),
            out.clone(),
        );
        function.instance().solve_result().unwrap();
        let previous = output(&out);
        for invalid in [-1, 0] {
            rhs.replace(
                &rhs.rebuild_data_draft(ValueDataDraft::I32(invalid))
                    .unwrap(),
            )
            .unwrap();
            let error = function.instance().solve_result().unwrap_err();
            assert_eq!(error.kind_name(), "MathDivisionInvalid");
            assert_eq!(output(&out), previous);
        }
    }

    fn output(cell: &ValueCell) -> i32 {
        let snapshot = cell.snapshot().unwrap();
        let ValueData::I32(value) = snapshot.data() else {
            panic!("expected I32")
        };
        *value
    }
}

impl_canonical_registered_math_binop_specializer!(MathDiv, "Div");
