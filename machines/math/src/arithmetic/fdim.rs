use crate::*;

pub(crate) trait RuntimeFdim: Copy {
    fn runtime_fdim(self, rhs: Self) -> Self;
}

#[cfg(feature = "f32")]
impl RuntimeFdim for f32 {
    fn runtime_fdim(self, rhs: Self) -> Self {
        libm::fdimf(self, rhs)
    }
}

#[cfg(feature = "f64")]
impl RuntimeFdim for f64 {
    fn runtime_fdim(self, rhs: Self) -> Self {
        libm::fdim(self, rhs)
    }
}

macro_rules! fdim_op {
    ($arg1:expr, $arg2:expr) => {
        $arg1.runtime_fdim($arg2)
    };
}

macro_rules! impl_fdim_binop {
    ($struct_name:ident, $arg1_type:ty, $arg2_type:ty, $out_type:ty, $_op:ident) => {
        impl_managed_math_broadcast_binary_full_write!(
            $struct_name,
            $arg1_type,
            $arg2_type,
            $out_type,
            RuntimeFdim,
            fdim_op,
            "math/fdim"
        );
    };
}

impl_fxns!(Fdim, T, T, impl_fdim_binop);

impl_canonical_math_same_type_binop_specializer!(MathFdim, Fdim, "math/fdim");
