use crate::*;
use num_traits::*;

fn checked_runtime_mul<T: RuntimeCheckedArithmetic>(lhs: T, rhs: T) -> MResult<T> {
    lhs.runtime_checked_mul(rhs)
        .ok_or_else(|| arithmetic_overflow::<T>("multiplication"))
}

// Mul ------------------------------------------------------------------------

macro_rules! mul_op {
    (@managed $lhs:expr, $rhs:expr) => {
        checked_runtime_mul($lhs, $rhs)
    };
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let next = checked_runtime_mul(*$lhs, *$rhs)?;
            *$out = next;
        }
    };
}

macro_rules! impl_checked_mul_binop {
    ($struct_name:ident, $arg1_type:ty, $arg2_type:ty, $out_type:ty, $op:ident) => {
        impl_checked_arithmetic_binop!(
            $struct_name,
            $arg1_type,
            $arg2_type,
            $out_type,
            mul_op,
            crate::ops::arithmetic_full_write_contract
        );
    };
}

impl_fxns!(Mul, T, T, impl_checked_mul_binop);

#[cfg(all(test, feature = "u8", feature = "source"))]
mod checked_arithmetic_tests {
    use super::*;

    #[test]
    fn integer_multiplication_rejects_reactive_overflow_and_retains_output() {
        let lhs = ValueCell::from_exact(20_u8).unwrap();
        let rhs = ValueCell::from_exact(2_u8).unwrap();
        let function = specialize(lhs, rhs.clone());
        function.instance().solve_result().unwrap();
        assert_eq!(output(&function), 40);
        let overflow = rhs.rebuild_data_draft(ValueDataDraft::U8(20)).unwrap();
        rhs.replace(&overflow).unwrap();
        let error = function.instance().solve_result().unwrap_err();
        assert_eq!(error.kind_name(), "MathArithmeticOverflow");
        assert_eq!(output(&function), 40);
    }

    fn specialize(lhs: ValueCell, rhs: ValueCell) -> SpecializedFunction {
        let mut builder = FunctionCatalogBuilder::new();
        crate::catalog::install_runtime(&mut builder).unwrap();
        crate::catalog::install_source(&mut builder).unwrap();
        let catalog = builder.build().unwrap();
        crate::catalog::specialize_test_operation(&catalog, "math/mul", vec![lhs, rhs])
    }

    fn output(function: &SpecializedFunction) -> u8 {
        let snapshot = function.output().snapshot().unwrap();
        let ValueData::U8(value) = snapshot.data() else {
            panic!("expected U8 mul output")
        };
        *value
    }
}

impl_canonical_registered_math_binop_specializer!(MathMul, "Mul");
