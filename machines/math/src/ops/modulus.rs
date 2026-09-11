use crate::*;
use num_traits::*;

// Mod ------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MathRemainderInvalid {
    pub operand_type: &'static str,
}

impl MechErrorKind for MathRemainderInvalid {
    fn name(&self) -> &str {
        "MathRemainderInvalid"
    }

    fn message(&self) -> String {
        format!(
            "remainder is undefined or overflows for operand type {}",
            self.operand_type,
        )
    }
}

pub trait RuntimeCheckedRem: Copy {
    fn runtime_checked_rem(self, rhs: Self) -> Option<Self>;
}

macro_rules! impl_checked_integer_rem {
    ($($type:ty),+ $(,)?) => {
        $(
            impl RuntimeCheckedRem for $type {
                fn runtime_checked_rem(self, rhs: Self) -> Option<Self> {
                    self.checked_rem(rhs)
                }
            }
        )+
    };
}

impl_checked_integer_rem!(i8, i16, i32, i64, i128, u8, u16, u32, u64, u128);

impl RuntimeCheckedRem for f32 {
    fn runtime_checked_rem(self, rhs: Self) -> Option<Self> {
        Some(self % rhs)
    }
}

impl RuntimeCheckedRem for f64 {
    fn runtime_checked_rem(self, rhs: Self) -> Option<Self> {
        Some(self % rhs)
    }
}

fn checked_runtime_rem<T: RuntimeCheckedRem>(lhs: T, rhs: T) -> MResult<T> {
    lhs.runtime_checked_rem(rhs).ok_or_else(|| {
        MechError::new(
            MathRemainderInvalid {
                operand_type: std::any::type_name::<T>(),
            },
            None,
        )
        .with_compiler_loc()
    })
}

macro_rules! managed_rem_op {
    (@managed $lhs:expr, $rhs:expr) => {
        checked_runtime_rem($lhs, $rhs)
    };
}

macro_rules! impl_binop2 {
    ($struct_name:ident, $arg1_type:ty, $arg2_type:ty, $out_type:ty, $op:ident) => {
        impl_checked_arithmetic_binop!(@bound RuntimeCheckedRem;
            $struct_name, $arg1_type, $arg2_type, $out_type, managed_rem_op,
            crate::managed_binary::arithmetic_full_write_contract);
    };
}

macro_rules! impl_math_fxns2 {
    ($lib:ident) => {
        impl_fxns!($lib, T, T, impl_binop2);
    };
}

impl_math_fxns2!(Mod);

#[cfg(all(test, feature = "i32"))]
mod tests {
    use super::*;

    #[test]
    fn integer_remainder_rejects_zero_and_signed_overflow_on_reactive_resolve() {
        let lhs = ValueCell::from_exact(i32::MIN).unwrap();
        let rhs = ValueCell::from_exact(2_i32).unwrap();
        let out = ValueCell::from_exact(17_i32).unwrap();
        let function = crate::catalog::bind_test_binary::<ModSS<i32>>(
            "math/mod",
            "ModSS<i32>",
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
            assert_eq!(error.kind_name(), "MathRemainderInvalid");
            assert_eq!(output(&out), previous);
        }
    }

    fn output(cell: &ValueCell) -> i32 {
        let value = cell.snapshot().unwrap();
        let ValueData::I32(value) = value.data() else {
            panic!("expected I32")
        };
        *value
    }
}

impl_canonical_registered_math_binop_specializer!(MathMod, "Mod");
