use crate::*;

pub(crate) trait RuntimeCopysign: Copy {
    fn runtime_copysign(self, rhs: Self) -> Self;
}

#[cfg(feature = "f32")]
impl RuntimeCopysign for f32 {
    fn runtime_copysign(self, rhs: Self) -> Self {
        libm::copysignf(self, rhs)
    }
}

#[cfg(feature = "f64")]
impl RuntimeCopysign for f64 {
    fn runtime_copysign(self, rhs: Self) -> Self {
        libm::copysign(self, rhs)
    }
}

macro_rules! copysign_op {
    ($arg1:expr, $arg2:expr) => {
        $arg1.runtime_copysign($arg2)
    };
}

macro_rules! impl_copysign_binop {
    ($struct_name:ident, $arg1_type:ty, $arg2_type:ty, $out_type:ty, $_op:ident) => {
        impl_managed_math_broadcast_binary_full_write!(
            $struct_name,
            $arg1_type,
            $arg2_type,
            $out_type,
            RuntimeCopysign,
            copysign_op,
            "math/copysign",
            Copysign
        );
    };
}

impl_fxns!(Copysign, T, T, impl_copysign_binop);

impl_canonical_math_same_type_binop_specializer!(MathCopysign, Copysign, "math/copysign");
