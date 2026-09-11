use crate::*;

pub(crate) trait RuntimeRemainder: Copy {
    fn runtime_remainder(self, rhs: Self) -> Self;
}

#[cfg(feature = "f32")]
impl RuntimeRemainder for f32 {
    fn runtime_remainder(self, rhs: Self) -> Self {
        libm::remainderf(self, rhs)
    }
}

#[cfg(feature = "f64")]
impl RuntimeRemainder for f64 {
    fn runtime_remainder(self, rhs: Self) -> Self {
        libm::remainder(self, rhs)
    }
}

macro_rules! remainder_op {
    ($arg1:expr, $arg2:expr) => {
        $arg1.runtime_remainder($arg2)
    };
}

macro_rules! impl_remainder_binop {
    ($struct_name:ident, $arg1_type:ty, $arg2_type:ty, $out_type:ty, $_op:ident) => {
        impl_managed_math_broadcast_binary_full_write!(
            $struct_name,
            $arg1_type,
            $arg2_type,
            $out_type,
            RuntimeRemainder,
            remainder_op,
            "math/remainder",
            Remainder
        );
    };
}

impl_fxns!(Remainder, T, T, impl_remainder_binop);

impl_canonical_math_same_type_binop_specializer!(MathRemainder, Remainder, "math/remainder");
