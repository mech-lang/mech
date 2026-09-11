use crate::*;

pub(crate) trait RuntimeNextafter: Copy {
    fn runtime_nextafter(self, rhs: Self) -> Self;
}

#[cfg(feature = "f32")]
impl RuntimeNextafter for f32 {
    fn runtime_nextafter(self, rhs: Self) -> Self {
        libm::nextafterf(self, rhs)
    }
}

#[cfg(feature = "f64")]
impl RuntimeNextafter for f64 {
    fn runtime_nextafter(self, rhs: Self) -> Self {
        libm::nextafter(self, rhs)
    }
}

macro_rules! nextafter_op {
    ($arg1:expr, $arg2:expr) => {
        $arg1.runtime_nextafter($arg2)
    };
}

macro_rules! impl_nextafter_binop {
    ($struct_name:ident, $arg1_type:ty, $arg2_type:ty, $out_type:ty, $_op:ident) => {
        impl_managed_math_broadcast_binary_full_write!(
            $struct_name,
            $arg1_type,
            $arg2_type,
            $out_type,
            RuntimeNextafter,
            nextafter_op,
            "math/nextafter",
            Nextafter
        );
    };
}

impl_fxns!(Nextafter, T, T, impl_nextafter_binop);

impl_canonical_math_same_type_binop_specializer!(MathNextafter, Nextafter, "math/nextafter");
