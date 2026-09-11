use crate::*;

pub(crate) trait RuntimeJn: Copy {
    fn runtime_jn(self, rhs: Self) -> Self;
}

#[cfg(feature = "f32")]
impl RuntimeJn for f32 {
    fn runtime_jn(self, rhs: Self) -> Self {
        libm::jnf(self as i32, rhs)
    }
}

#[cfg(feature = "f64")]
impl RuntimeJn for f64 {
    fn runtime_jn(self, rhs: Self) -> Self {
        libm::jn(self as i32, rhs)
    }
}

macro_rules! jn_op {
    ($arg1:expr, $arg2:expr) => {
        $arg1.runtime_jn($arg2)
    };
}

macro_rules! impl_jn_binop {
    ($struct_name:ident, $arg1_type:ty, $arg2_type:ty, $out_type:ty, $_op:ident) => {
        impl_managed_math_broadcast_binary_full_write!(
            $struct_name,
            $arg1_type,
            $arg2_type,
            $out_type,
            RuntimeJn,
            jn_op,
            "math/bessel/jn"
        );
    };
}

impl_fxns!(Jn, T, T, impl_jn_binop);

impl_canonical_math_same_type_binop_specializer!(MathJn, Jn, "math/bessel/jn");
