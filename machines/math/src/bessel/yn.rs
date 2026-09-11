use crate::*;

pub(crate) trait RuntimeYn: Copy {
    fn runtime_yn(self, rhs: Self) -> Self;
}

#[cfg(feature = "f32")]
impl RuntimeYn for f32 {
    fn runtime_yn(self, rhs: Self) -> Self {
        libm::ynf(self as i32, rhs)
    }
}

#[cfg(feature = "f64")]
impl RuntimeYn for f64 {
    fn runtime_yn(self, rhs: Self) -> Self {
        libm::yn(self as i32, rhs)
    }
}

macro_rules! yn_op {
    ($arg1:expr, $arg2:expr) => {
        $arg1.runtime_yn($arg2)
    };
}

macro_rules! impl_yn_binop {
    ($struct_name:ident, $arg1_type:ty, $arg2_type:ty, $out_type:ty, $_op:ident) => {
        impl_managed_math_broadcast_binary_full_write!(
            $struct_name,
            $arg1_type,
            $arg2_type,
            $out_type,
            RuntimeYn,
            yn_op,
            "math/bessel/yn"
        );
    };
}

impl_fxns!(Yn, T, T, impl_yn_binop);

impl_canonical_math_same_type_binop_specializer!(MathYn, Yn, "math/bessel/yn");
