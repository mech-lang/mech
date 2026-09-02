#![cfg_attr(not(test), no_main)]
#![feature(where_clause_attrs)]

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(feature = "matrix")]
extern crate nalgebra as na;

#[cfg(all(not(feature = "dynamic-module"), feature = "runtime"))]
use mech_core::*;

#[cfg(all(not(feature = "dynamic-module"), feature = "math"))]
use paste::paste;

#[cfg(feature = "matrixd")]
use na::DMatrix;
#[cfg(feature = "vectord")]
use na::DVector;
#[cfg(any(feature = "matrix1", feature = "matrix1_interop"))]
use na::Matrix1;
#[cfg(feature = "matrix2")]
use na::Matrix2;
#[cfg(feature = "matrix2x3")]
use na::Matrix2x3;
#[cfg(feature = "matrix3")]
use na::Matrix3;
#[cfg(feature = "matrix3x2")]
use na::Matrix3x2;
#[cfg(feature = "matrix4")]
use na::Matrix4;
#[cfg(feature = "row_vectord")]
use na::RowDVector;
#[cfg(feature = "row_vector2")]
use na::RowVector2;
#[cfg(feature = "row_vector3")]
use na::RowVector3;
#[cfg(feature = "row_vector4")]
use na::RowVector4;
#[cfg(feature = "vector2")]
use na::Vector2;
#[cfg(feature = "vector3")]
use na::Vector3;
#[cfg(feature = "vector4")]
use na::Vector4;

#[cfg(any(feature = "ops", feature = "op_assign"))]
use std::fmt::{Debug, Display};
#[cfg(any(feature = "neg", feature = "op_assign"))]
use std::marker::PhantomData;
#[cfg(any(feature = "ops", feature = "op_assign"))]
use std::ops::*;

#[cfg(all(feature = "runtime", not(feature = "dynamic-module")))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MathArithmeticOverflow {
    pub operation: &'static str,
    pub operand_type: &'static str,
}

#[cfg(all(feature = "runtime", not(feature = "dynamic-module")))]
impl MechErrorKind for MathArithmeticOverflow {
    fn name(&self) -> &str {
        "MathArithmeticOverflow"
    }

    fn message(&self) -> String {
        format!(
            "{} overflows operand type {}",
            self.operation, self.operand_type,
        )
    }
}

#[cfg(all(feature = "runtime", not(feature = "dynamic-module")))]
pub(crate) fn arithmetic_overflow<T>(operation: &'static str) -> MechError {
    MechError::new(
        MathArithmeticOverflow {
            operation,
            operand_type: std::any::type_name::<T>(),
        },
        None,
    )
    .with_compiler_loc()
}

#[cfg(any(feature = "round", feature = "dynamic-module"))]
pub mod kernels;

#[cfg(feature = "dynamic-module")]
mod dynamic_module;

#[cfg(feature = "arithmetic")]
pub mod arithmetic;
#[cfg(feature = "bessel")]
pub mod bessel;
#[cfg(feature = "gamma")]
pub mod gamma;
#[cfg(feature = "logarithm")]
pub mod logarithm;
#[cfg(feature = "op_assign")]
pub mod op_assign;
#[cfg(feature = "ops")]
pub mod ops;
#[cfg(feature = "root")]
pub mod root;
#[cfg(feature = "rounding")]
pub mod rounding;
#[cfg(feature = "stat_error")]
pub mod stat_error;
#[cfg(feature = "trig")]
pub mod trig;

#[cfg(all(feature = "runtime", not(feature = "dynamic-module")))]
pub mod catalog;

#[cfg(all(feature = "arithmetic", feature = "source"))]
pub use self::arithmetic::*;
#[cfg(all(feature = "bessel", feature = "source"))]
pub use self::bessel::*;
#[cfg(all(feature = "gamma", feature = "source"))]
pub use self::gamma::*;
#[cfg(all(feature = "logarithm", feature = "source"))]
pub use self::logarithm::*;
#[cfg(all(feature = "op_assign", feature = "source"))]
pub use self::op_assign::*;
#[cfg(all(feature = "ops", feature = "source"))]
pub use self::ops::*;
#[cfg(all(feature = "ops", feature = "runtime", not(feature = "source")))]
pub(crate) use self::ops::*;
#[cfg(all(feature = "root", feature = "source"))]
pub use self::root::*;
#[cfg(all(feature = "rounding", feature = "source"))]
pub use self::rounding::*;
#[cfg(all(feature = "stat_error", feature = "source"))]
pub use self::stat_error::*;
#[cfg(all(feature = "trig", feature = "source"))]
pub use self::trig::*;

#[cfg(all(feature = "runtime", not(feature = "dynamic-module")))]
pub use self::catalog::*;

#[doc(hidden)]
#[cfg(feature = "native-link")]
pub mod __mech_native {
    #[cfg(any(
        feature = "add_assign", feature = "div_assign", feature = "mul_assign",
        feature = "sub_assign", feature = "abs", feature = "neg", feature = "atan2",
        feature = "div", feature = "mod", feature = "mul", feature = "pow", feature = "sub",
        feature = "j0", feature = "j1", feature = "y0", feature = "y1",
        feature = "lgamma", feature = "tgamma",
        feature = "log", feature = "log10", feature = "log1p", feature = "log2",
        feature = "cbrt", feature = "sqrt",
        feature = "ceil", feature = "floor", feature = "rint", feature = "round",
        feature = "roundeven", feature = "trunc", feature = "erf", feature = "erfc",
        feature = "acos", feature = "acosh", feature = "acot", feature = "acsc",
        feature = "asec", feature = "asin", feature = "asinh", feature = "atan",
        feature = "atanh", feature = "cos", feature = "cosh", feature = "cot",
        feature = "csc", feature = "sec", feature = "sin", feature = "sinh",
        feature = "tan", feature = "tanh"
    ))]
    pub use crate::catalog::__mech_native::*;
    #[cfg(feature = "add")]
    pub use crate::ops::add::__mech_native::*;
}

// ----------------------------------------------------------------------------
// Math Library
// ----------------------------------------------------------------------------

#[macro_export]
macro_rules! impl_math_fxns {
    ($lib:ident) => {
        impl_fxns!($lib, T, T, impl_binop);
    };
}

#[macro_export]
macro_rules! impl_canonical_registered_math_unop_specializer {
    ($specializer:ident, $factory_prefix:literal) => {
        #[cfg(feature = "source")]
        pub struct $specializer;

        #[cfg(feature = "source")]
        impl CanonicalFunctionSpecializer for $specializer {
            fn specialize_invocation(
                &self,
                invocation: &SpecializationInvocation,
                context: &mut SpecializationContext<'_>,
            ) -> MResult<SpecializedFunction> {
                if invocation.len() != 1 {
                    return Err(MechError::new(
                        IncorrectNumberOfArguments {
                            expected: 1,
                            found: invocation.len(),
                        },
                        None,
                    )
                    .with_compiler_loc());
                }
                let input = invocation.input(0).expect("validated unary math input");
                context.bind_runtime_factory_derived_output(
                    $factory_prefix,
                    None,
                    &[input],
                )
            }
        }
    };
}

#[macro_export]
macro_rules! impl_canonical_registered_math_binop_specializer {
    ($specializer:ident, $factory_prefix:literal) => {
        #[cfg(feature = "source")]
        pub struct $specializer;

        #[cfg(feature = "source")]
        impl CanonicalFunctionSpecializer for $specializer {
            fn specialize_invocation(
                &self,
                invocation: &SpecializationInvocation,
                context: &mut SpecializationContext<'_>,
            ) -> MResult<SpecializedFunction> {
                if invocation.len() != 2 {
                    return Err(MechError::new(
                        IncorrectNumberOfArguments {
                            expected: 2,
                            found: invocation.len(),
                        },
                        None,
                    )
                    .with_compiler_loc());
                }
                let first = invocation.input(0).expect("validated binary math lhs");
                let second = invocation.input(1).expect("validated binary math rhs");
                context.bind_runtime_factory_derived_output(
                    $factory_prefix,
                    None,
                    &[first, second],
                )
            }
        }
    };
}

#[macro_export]
macro_rules! impl_math_unop {
  ($fxn_name:ident, $type:ident, $op_fxn:ident) => {
    paste!{
      impl_unop!([<$fxn_name $type:camel S>], $type, $type, [<$op_fxn _op>], crate::ops::unary_full_write_contract);
      #[cfg(feature = "matrix1")]
      impl_unop!([<$fxn_name $type:camel M1>], Matrix1<$type>, Matrix1<$type>, [<$op_fxn _vec_op>], crate::ops::unary_full_write_contract);
      #[cfg(feature = "matrix2")]
      impl_unop!([<$fxn_name $type:camel M2>], Matrix2<$type>, Matrix2<$type>, [<$op_fxn _vec_op>], crate::ops::unary_full_write_contract);
      #[cfg(feature = "matrix3")]
      impl_unop!([<$fxn_name $type:camel M3>], Matrix3<$type>, Matrix3<$type>, [<$op_fxn _vec_op>], crate::ops::unary_full_write_contract);
      #[cfg(feature = "matrix4")]
      impl_unop!([<$fxn_name $type:camel M4>], Matrix4<$type>, Matrix4<$type>, [<$op_fxn _vec_op>], crate::ops::unary_full_write_contract);
      #[cfg(feature = "matrix2x3")]
      impl_unop!([<$fxn_name $type:camel M2x3>], Matrix2x3<$type>, Matrix2x3<$type>, [<$op_fxn _vec_op>], crate::ops::unary_full_write_contract);
      #[cfg(feature = "matrix3x2")]
      impl_unop!([<$fxn_name $type:camel M3x2>], Matrix3x2<$type>, Matrix3x2<$type>, [<$op_fxn _vec_op>], crate::ops::unary_full_write_contract);
      #[cfg(feature = "matrixd")]
      impl_unop!([<$fxn_name $type:camel MD>], DMatrix<$type>, DMatrix<$type>, [<$op_fxn _vec_op>], crate::ops::unary_full_write_contract);
      #[cfg(feature = "row_vector2")]
      impl_unop!([<$fxn_name $type:camel R2>], RowVector2<$type>, RowVector2<$type>, [<$op_fxn _vec_op>], crate::ops::unary_full_write_contract);
      #[cfg(feature = "row_vector3")]
      impl_unop!([<$fxn_name $type:camel R3>], RowVector3<$type>, RowVector3<$type>, [<$op_fxn _vec_op>], crate::ops::unary_full_write_contract);
      #[cfg(feature = "row_vector4")]
      impl_unop!([<$fxn_name $type:camel R4>], RowVector4<$type>, RowVector4<$type>, [<$op_fxn _vec_op>], crate::ops::unary_full_write_contract);
      #[cfg(feature = "row_vectord")]
      impl_unop!([<$fxn_name $type:camel RD>], RowDVector<$type>, RowDVector<$type>, [<$op_fxn _vec_op>], crate::ops::unary_full_write_contract);
      #[cfg(feature = "vector2")]
      impl_unop!([<$fxn_name $type:camel V2>], Vector2<$type>, Vector2<$type>, [<$op_fxn _vec_op>], crate::ops::unary_full_write_contract);
      #[cfg(feature = "vector3")]
      impl_unop!([<$fxn_name $type:camel V3>], Vector3<$type>, Vector3<$type>, [<$op_fxn _vec_op>], crate::ops::unary_full_write_contract);
      #[cfg(feature = "vector4")]
      impl_unop!([<$fxn_name $type:camel V4>], Vector4<$type>, Vector4<$type>, [<$op_fxn _vec_op>], crate::ops::unary_full_write_contract);
      #[cfg(feature = "vectord")]
      impl_unop!([<$fxn_name $type:camel VD>], DVector<$type>, DVector<$type>, [<$op_fxn _vec_op>], crate::ops::unary_full_write_contract);
    }}}

#[cfg(all(
    feature = "source",
    any(
        feature = "acos",
        feature = "acosh",
        feature = "acot",
        feature = "acsc",
        feature = "asec",
        feature = "asin",
        feature = "asinh",
        feature = "atan",
        feature = "atanh",
        feature = "cbrt",
        feature = "ceil",
        feature = "cos",
        feature = "cosh",
        feature = "cot",
        feature = "csc",
        feature = "erf",
        feature = "erfc",
        feature = "floor",
        feature = "j0",
        feature = "j1",
        feature = "lgamma",
        feature = "log",
        feature = "log1p",
        feature = "log10",
        feature = "log2",
        feature = "rint",
        feature = "round",
        feature = "roundeven",
        feature = "sec",
        feature = "sin",
        feature = "sinh",
        feature = "sqrt",
        feature = "tan",
        feature = "tanh",
        feature = "tgamma",
        feature = "trunc",
        feature = "y0",
        feature = "y1"
    )
))]
fn specialize_math_unary_factory<F>(input: &SpecializationInput) -> MResult<SpecializedFunction>
where
    F: MechFunctionFactory,
{
    let invocation = FunctionInvocation::unary(
        input.cell()?.detached_clone()?,
        input.cell()?.clone(),
    );
    let implementation = F::new_invocation(invocation.clone())?;
    Ok(SpecializedFunction::new(FunctionInstance::new(
        implementation,
        invocation,
    )))
}

#[doc(hidden)]
#[macro_export]
macro_rules! __try_math_float_unary_scalar {
    ($input:ident, $lib:ident, $scalar:ident) => {{
        macro_rules! try_suffix {
            ($suffix:ident) => {
                mech_core::paste::paste! {
                    if let RuntimeFunctionInputs::Unary(expected) =
                        <[<$lib $scalar:camel $suffix>] as MechFunctionFactory>::SIGNATURE.inputs
                        && $input.representation() == Some(expected)
                    {
                        return $crate::specialize_math_unary_factory::<
                            [<$lib $scalar:camel $suffix>]
                        >($input);
                    }
                }
            };
        }
        try_suffix!(S);
        #[cfg(feature = "matrix1")]
        try_suffix!(M1);
        #[cfg(feature = "matrix2")]
        try_suffix!(M2);
        #[cfg(feature = "matrix3")]
        try_suffix!(M3);
        #[cfg(feature = "matrix4")]
        try_suffix!(M4);
        #[cfg(feature = "matrix2x3")]
        try_suffix!(M2x3);
        #[cfg(feature = "matrix3x2")]
        try_suffix!(M3x2);
        #[cfg(feature = "matrixd")]
        try_suffix!(MD);
        #[cfg(feature = "row_vector2")]
        try_suffix!(R2);
        #[cfg(feature = "row_vector3")]
        try_suffix!(R3);
        #[cfg(feature = "row_vector4")]
        try_suffix!(R4);
        #[cfg(feature = "row_vectord")]
        try_suffix!(RD);
        #[cfg(feature = "vector2")]
        try_suffix!(V2);
        #[cfg(feature = "vector3")]
        try_suffix!(V3);
        #[cfg(feature = "vector4")]
        try_suffix!(V4);
        #[cfg(feature = "vectord")]
        try_suffix!(VD);
    }};
}

#[macro_export]
macro_rules! impl_canonical_math_float_unop_specializer {
    ($specializer:ident, $lib:ident, $operation:literal) => {
        #[cfg(feature = "source")]
        pub struct $specializer {}

        #[cfg(feature = "source")]
        impl CanonicalFunctionSpecializer for $specializer {
            fn specialize_invocation(
                &self,
                specialization: &SpecializationInvocation,
                _context: &mut SpecializationContext<'_>,
            ) -> MResult<SpecializedFunction> {
                if specialization.len() != 1 {
                    return Err(MechError::new(
                        IncorrectNumberOfArguments {
                            expected: 1,
                            found: specialization.len(),
                        },
                        None,
                    )
                    .with_compiler_loc());
                }
                let input = specialization.input(0).expect("validated unary input");
                #[cfg(feature = "f32")]
                $crate::__try_math_float_unary_scalar!(input, $lib, f32);
                #[cfg(feature = "f64")]
                $crate::__try_math_float_unary_scalar!(input, $lib, f64);
                Err(MechError::new(
                    FunctionArgumentTypeMismatch {
                        role: FunctionArgumentRole::Input(0),
                        expected: concat!("supported floating input for ", $operation).into(),
                        found: format!("{:?}", input.representation()),
                    },
                    None,
                )
                .with_compiler_loc())
            }
        }
    };
}

#[cfg(all(
    feature = "source",
    any(
        feature = "copysign",
        feature = "fdim",
        feature = "fmod",
        feature = "nextafter",
        feature = "remainder",
        feature = "atan2",
        feature = "jn",
        feature = "yn"
    )
))]
fn specialize_math_binary_factory<F>(
    first: &SpecializationInput,
    second: &SpecializationInput,
) -> MResult<SpecializedFunction>
where
    F: MechFunctionFactory,
{
    let invocation = FunctionInvocation::binary(
        first.cell()?.detached_clone()?,
        first.cell()?.clone(),
        second.cell()?.clone(),
    );
    let implementation = F::new_invocation(invocation.clone())?;
    Ok(SpecializedFunction::new(FunctionInstance::new(
        implementation,
        invocation,
    )))
}

#[doc(hidden)]
#[macro_export]
macro_rules! __try_math_same_type_binary_scalar {
    ($first:ident, $second:ident, $prefix:ident, $scalar:ident) => {{
        macro_rules! try_suffix {
            () => {
                mech_core::paste::paste! {
                    if let RuntimeFunctionInputs::Binary(expected_first, expected_second) =
                        <[<$prefix $scalar:camel>] as MechFunctionFactory>::SIGNATURE.inputs
                        && $first.representation() == Some(expected_first)
                        && $second.representation() == Some(expected_second)
                    {
                        return $crate::specialize_math_binary_factory::<
                            [<$prefix $scalar:camel>]
                        >($first, $second);
                    }
                }
            };
            ($suffix:ident) => {
                mech_core::paste::paste! {
                    if let RuntimeFunctionInputs::Binary(expected_first, expected_second) =
                        <[<$prefix $suffix $scalar:camel>] as MechFunctionFactory>::SIGNATURE.inputs
                        && $first.representation() == Some(expected_first)
                        && $second.representation() == Some(expected_second)
                    {
                        return $crate::specialize_math_binary_factory::<
                            [<$prefix $suffix $scalar:camel>]
                        >($first, $second);
                    }
                }
            };
        }
        try_suffix!();
        #[cfg(feature = "matrix1")]
        try_suffix!(M1);
        #[cfg(feature = "matrix2")]
        try_suffix!(M2);
        #[cfg(feature = "matrix3")]
        try_suffix!(M3);
        #[cfg(feature = "matrix4")]
        try_suffix!(M4);
        #[cfg(feature = "matrix2x3")]
        try_suffix!(M2x3);
        #[cfg(feature = "matrix3x2")]
        try_suffix!(M3x2);
        #[cfg(feature = "matrixd")]
        try_suffix!(MD);
        #[cfg(feature = "row_vector2")]
        try_suffix!(R2);
        #[cfg(feature = "row_vector3")]
        try_suffix!(R3);
        #[cfg(feature = "row_vector4")]
        try_suffix!(R4);
        #[cfg(feature = "row_vectord")]
        try_suffix!(RD);
        #[cfg(feature = "vector2")]
        try_suffix!(V2);
        #[cfg(feature = "vector3")]
        try_suffix!(V3);
        #[cfg(feature = "vector4")]
        try_suffix!(V4);
        #[cfg(feature = "vectord")]
        try_suffix!(VD);
    }};
}

#[macro_export]
macro_rules! impl_canonical_math_same_type_binop_specializer {
    ($specializer:ident, $prefix:ident, $operation:literal) => {
        #[cfg(feature = "source")]
        pub struct $specializer {}

        #[cfg(feature = "source")]
        impl CanonicalFunctionSpecializer for $specializer {
            fn specialize_invocation(
                &self,
                specialization: &SpecializationInvocation,
                _context: &mut SpecializationContext<'_>,
            ) -> MResult<SpecializedFunction> {
                if specialization.len() != 2 {
                    return Err(MechError::new(
                        IncorrectNumberOfArguments {
                            expected: 2,
                            found: specialization.len(),
                        },
                        None,
                    )
                    .with_compiler_loc());
                }
                let first = specialization.input(0).expect("validated first input");
                let second = specialization.input(1).expect("validated second input");
                #[cfg(feature = "f32")]
                $crate::__try_math_same_type_binary_scalar!(first, second, $prefix, f32);
                #[cfg(feature = "f64")]
                $crate::__try_math_same_type_binary_scalar!(first, second, $prefix, f64);
                Err(MechError::new(
                    FunctionArgumentTypeMismatch {
                        role: FunctionArgumentRole::Input(0),
                        expected: concat!("matching floating inputs for ", $operation).into(),
                        found: format!(
                            "{:?} and {:?}",
                            first.representation(),
                            second.representation(),
                        ),
                    },
                    None,
                )
                .with_compiler_loc())
            }
        }
    };
}
