use crate::*;
use num_traits::*;

fn checked_runtime_sub<T: RuntimeCheckedArithmetic>(lhs: T, rhs: T) -> MResult<T> {
    lhs.runtime_checked_sub(rhs)
        .ok_or_else(|| arithmetic_overflow::<T>("subtraction"))
}

// Sub ------------------------------------------------------------------------

macro_rules! sub_op {
    (@managed $lhs:expr, $rhs:expr) => {
        checked_runtime_sub($lhs, $rhs)
    };
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let next = checked_runtime_sub(*$lhs, *$rhs)?;
            *$out = next;
        }
    };
}

macro_rules! impl_checked_sub_binop {
    ($struct_name:ident, $arg1_type:ty, $arg2_type:ty, $out_type:ty, $op:ident) => {
        impl_checked_arithmetic_binop!(
            $struct_name,
            $arg1_type,
            $arg2_type,
            $out_type,
            sub_op,
            crate::ops::arithmetic_full_write_contract
        );
    };
}

impl_fxns!(Sub, T, T, impl_checked_sub_binop);

#[cfg(all(test, feature = "u8", feature = "source"))]
mod checked_arithmetic_tests {
    use super::*;

    #[test]
    fn integer_subtraction_rejects_reactive_overflow_and_retains_output() {
        let lhs = ValueCell::from_exact(40_u8).unwrap();
        let rhs = ValueCell::from_exact(1_u8).unwrap();
        let function = specialize(lhs, rhs.clone());
        function.instance().solve_result().unwrap();
        assert_eq!(output(&function), 39);
        let underflow = rhs.rebuild_data_draft(ValueDataDraft::U8(41)).unwrap();
        rhs.replace(&underflow).unwrap();
        let error = function.instance().solve_result().unwrap_err();
        assert_eq!(error.kind_name(), "MathArithmeticOverflow");
        assert_eq!(output(&function), 39);
    }

    fn specialize(lhs: ValueCell, rhs: ValueCell) -> SpecializedFunction {
        let mut builder = FunctionCatalogBuilder::new();
        crate::catalog::install_runtime(&mut builder).unwrap();
        crate::catalog::install_source(&mut builder).unwrap();
        let catalog = builder.build().unwrap();
        crate::catalog::specialize_test_operation(&catalog, "math/sub", vec![lhs, rhs])
    }

    fn output(function: &SpecializedFunction) -> u8 {
        let snapshot = function.output().snapshot().unwrap();
        let ValueData::U8(value) = snapshot.data() else {
            panic!("expected U8 sub output")
        };
        *value
    }
}

impl_canonical_registered_math_binop_specializer!(MathSub, "Sub");
