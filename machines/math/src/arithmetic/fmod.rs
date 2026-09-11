use crate::*;

pub(crate) trait RuntimeFmod: Copy {
    fn runtime_fmod(self, rhs: Self) -> Self;
}

#[cfg(feature = "f32")]
impl RuntimeFmod for f32 {
    fn runtime_fmod(self, rhs: Self) -> Self {
        libm::fmodf(self, rhs)
    }
}

#[cfg(feature = "f64")]
impl RuntimeFmod for f64 {
    fn runtime_fmod(self, rhs: Self) -> Self {
        libm::fmod(self, rhs)
    }
}

macro_rules! fmod_op {
    ($arg1:expr, $arg2:expr) => {
        $arg1.runtime_fmod($arg2)
    };
}

macro_rules! impl_fmod_binop {
    ($struct_name:ident, $arg1_type:ty, $arg2_type:ty, $out_type:ty, $_op:ident) => {
        impl_managed_math_broadcast_binary_full_write!(
            $struct_name,
            $arg1_type,
            $arg2_type,
            $out_type,
            RuntimeFmod,
            fmod_op,
            "math/fmod",
            Fmod
        );
    };
}

impl_fxns!(Fmod, T, T, impl_fmod_binop);

impl_canonical_math_same_type_binop_specializer!(MathFmod, Fmod, "math/fmod");
