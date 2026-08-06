#![cfg_attr(not(test), no_main)]
#![allow(warnings)]
#![feature(where_clause_attrs)]

#[doc(hidden)]
#[cfg(feature = "native-link")]
pub mod __mech_native {
    pub use crate::catalog::__mech_native::*;
}

#[macro_use]
extern crate mech_core;
#[cfg(feature = "matrix")]
extern crate nalgebra as na;
extern crate paste;

use mech_core::*;

use paste::paste;

#[cfg(feature = "matrixd")]
use nalgebra::DMatrix;
#[cfg(feature = "vectord")]
use nalgebra::DVector;
#[cfg(feature = "matrix1")]
use nalgebra::Matrix1;
#[cfg(feature = "matrix2")]
use nalgebra::Matrix2;
#[cfg(feature = "matrix2x3")]
use nalgebra::Matrix2x3;
#[cfg(feature = "matrix3")]
use nalgebra::Matrix3;
#[cfg(feature = "matrix3x2")]
use nalgebra::Matrix3x2;
#[cfg(feature = "matrix4")]
use nalgebra::Matrix4;
#[cfg(feature = "rowdvector")]
use nalgebra::RowDVector;
#[cfg(feature = "row_vectord")]
use nalgebra::RowDVector;
#[cfg(feature = "row_vector2")]
use nalgebra::RowVector2;
#[cfg(feature = "row_vector3")]
use nalgebra::RowVector3;
#[cfg(feature = "row_vector4")]
use nalgebra::RowVector4;
#[cfg(feature = "vector2")]
use nalgebra::Vector2;
#[cfg(feature = "vector3")]
use nalgebra::Vector3;
#[cfg(feature = "vector4")]
use nalgebra::Vector4;

#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;
#[cfg(any(feature = "dot", feature = "matmul"))]
use num_traits::*;
use std::fmt::Debug;
use std::ops::*;

use std::fmt::Display;

#[cfg(any(feature = "dot", feature = "matmul"))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatrixArithmeticOverflow {
    pub operation: &'static str,
    pub operand_type: &'static str,
}

#[cfg(any(feature = "dot", feature = "matmul"))]
impl MechErrorKind for MatrixArithmeticOverflow {
    fn name(&self) -> &str {
        "MatrixArithmeticOverflow"
    }

    fn message(&self) -> String {
        format!(
            "{} overflows operand type {}",
            self.operation, self.operand_type,
        )
    }
}

/// Arithmetic shared by every dot-product and matrix-product factory. Integer
/// implementations are checked so debug and release applications have the
/// same behavior; IEEE and exact non-primitive numeric types retain their
/// established unbounded/IEEE operations.
#[cfg(any(feature = "dot", feature = "matmul"))]
pub trait RuntimeMatrixArithmetic:
    Copy
    + Debug
    + Display
    + Clone
    + Sync
    + Send
    + 'static
    + PartialEq
    + PartialOrd
    + AsValueKind
    + Add<Output = Self>
    + AddAssign
    + Sub<Output = Self>
    + SubAssign
    + Mul<Output = Self>
    + MulAssign
    + Div<Output = Self>
    + DivAssign
    + Zero
    + One
{
    fn runtime_checked_add(self, rhs: Self) -> Option<Self>;
    fn runtime_checked_mul(self, rhs: Self) -> Option<Self>;
}

#[cfg(any(feature = "dot", feature = "matmul"))]
macro_rules! impl_checked_matrix_arithmetic {
    ($($type:ty),+ $(,)?) => {
        $(
            impl RuntimeMatrixArithmetic for $type {
                fn runtime_checked_add(self, rhs: Self) -> Option<Self> {
                    self.checked_add(rhs)
                }

                fn runtime_checked_mul(self, rhs: Self) -> Option<Self> {
                    self.checked_mul(rhs)
                }
            }
        )+
    };
}

#[cfg(all(any(feature = "dot", feature = "matmul"), feature = "i8"))]
impl_checked_matrix_arithmetic!(i8);
#[cfg(all(any(feature = "dot", feature = "matmul"), feature = "i16"))]
impl_checked_matrix_arithmetic!(i16);
#[cfg(all(any(feature = "dot", feature = "matmul"), feature = "i32"))]
impl_checked_matrix_arithmetic!(i32);
#[cfg(all(any(feature = "dot", feature = "matmul"), feature = "i64"))]
impl_checked_matrix_arithmetic!(i64);
#[cfg(all(any(feature = "dot", feature = "matmul"), feature = "i128"))]
impl_checked_matrix_arithmetic!(i128);
#[cfg(all(any(feature = "dot", feature = "matmul"), feature = "u8"))]
impl_checked_matrix_arithmetic!(u8);
#[cfg(all(any(feature = "dot", feature = "matmul"), feature = "u16"))]
impl_checked_matrix_arithmetic!(u16);
#[cfg(all(any(feature = "dot", feature = "matmul"), feature = "u32"))]
impl_checked_matrix_arithmetic!(u32);
#[cfg(all(any(feature = "dot", feature = "matmul"), feature = "u64"))]
impl_checked_matrix_arithmetic!(u64);
#[cfg(all(any(feature = "dot", feature = "matmul"), feature = "u128"))]
impl_checked_matrix_arithmetic!(u128);

#[cfg(any(feature = "dot", feature = "matmul"))]
macro_rules! impl_unchecked_matrix_arithmetic {
    ($($type:ty),+ $(,)?) => {
        $(
            impl RuntimeMatrixArithmetic for $type {
                fn runtime_checked_add(self, rhs: Self) -> Option<Self> {
                    Some(self + rhs)
                }

                fn runtime_checked_mul(self, rhs: Self) -> Option<Self> {
                    Some(self * rhs)
                }
            }
        )+
    };
}

#[cfg(all(any(feature = "dot", feature = "matmul"), feature = "f32"))]
impl_unchecked_matrix_arithmetic!(f32);
#[cfg(all(any(feature = "dot", feature = "matmul"), feature = "f64"))]
impl_unchecked_matrix_arithmetic!(f64);
#[cfg(all(
    any(feature = "dot", feature = "matmul"),
    feature = "rational"
))]
impl_unchecked_matrix_arithmetic!(mech_core::R64);
#[cfg(all(
    any(feature = "dot", feature = "matmul"),
    feature = "complex"
))]
impl_unchecked_matrix_arithmetic!(mech_core::C64);

#[cfg(any(feature = "dot", feature = "matmul"))]
fn checked_matrix_add<T: RuntimeMatrixArithmetic>(
    lhs: T,
    rhs: T,
    operation: &'static str,
) -> MResult<T> {
    lhs.runtime_checked_add(rhs).ok_or_else(|| {
        MechError::new(
            MatrixArithmeticOverflow {
                operation,
                operand_type: std::any::type_name::<T>(),
            },
            None,
        )
        .with_compiler_loc()
    })
}

#[cfg(any(feature = "dot", feature = "matmul"))]
fn checked_matrix_mul<T: RuntimeMatrixArithmetic>(
    lhs: T,
    rhs: T,
    operation: &'static str,
) -> MResult<T> {
    lhs.runtime_checked_mul(rhs).ok_or_else(|| {
        MechError::new(
            MatrixArithmeticOverflow {
                operation,
                operand_type: std::any::type_name::<T>(),
            },
            None,
        )
        .with_compiler_loc()
    })
}

/// Fallible counterpart to `impl_binop!` for reduction kernels. The operation
/// macro computes a complete staged result and may use `?`; it publishes only
/// after every multiplication and accumulation succeeds.
#[cfg(any(feature = "dot", feature = "matmul"))]
macro_rules! impl_checked_matrix_binop {
    ($struct_name:ident, $arg1_type:ty, $arg2_type:ty, $out_type:ty, $op:ident) => {
        #[derive(Debug)]
        pub struct $struct_name<T> {
            pub lhs: Ref<$arg1_type>,
            pub rhs: Ref<$arg2_type>,
            pub out: Ref<$out_type>,
        }

        impl<T> MechFunctionFactory for $struct_name<T>
        where
            T: RuntimeMatrixArithmetic,
            #[cfg(feature = "compiler")]
            T: ConstElem + CompileConst,
            Ref<$out_type>: ToValue,
            $arg1_type: FunctionRuntimeType,
            $arg2_type: FunctionRuntimeType,
            $out_type: FunctionRuntimeType,
        {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
                <$out_type as FunctionRuntimeType>::REPRESENTATION,
                <$arg1_type as FunctionRuntimeType>::REPRESENTATION,
                <$arg2_type as FunctionRuntimeType>::REPRESENTATION,
            );

            fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
                match args {
                    FunctionArgs::Binary(out, arg1, arg2) => {
                        let lhs = arg1.try_function_ref(FunctionArgumentRole::Input(0))?;
                        let rhs = arg2.try_function_ref(FunctionArgumentRole::Input(1))?;
                        let out = out.try_function_ref(FunctionArgumentRole::Output)?;
                        Ok(Box::new(Self { lhs, rhs, out }))
                    }
                    _ => Err(MechError::new(
                        IncorrectNumberOfArguments {
                            expected: 2,
                            found: args.len(),
                        },
                        None,
                    )
                    .with_compiler_loc()),
                }
            }
        }

        impl<T> MechFunctionImpl for $struct_name<T>
        where
            T: RuntimeMatrixArithmetic,
            Ref<$out_type>: ToValue,
        {
            fn solve_result(&self) -> MResult<()> {
                let lhs_ptr = self.lhs.as_ptr();
                let rhs_ptr = self.rhs.as_ptr();
                let out_ptr = self.out.as_mut_ptr();
                $op!(lhs_ptr, rhs_ptr, out_ptr);
                Ok(())
            }

            fn out(&self) -> Value {
                self.out.to_value()
            }

            fn to_string(&self) -> String {
                format!("{self:#?}")
            }

            fn transaction_state_values(&self) -> MResult<Vec<Value>> {
                Ok(self.reactive_output_values())
            }
        }

        #[cfg(feature = "compiler")]
        impl<T> MechFunctionCompiler for $struct_name<T>
        where
            T: RuntimeMatrixArithmetic + ConstElem + CompileConst,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!("{}<{}>", stringify!($struct_name), T::as_value_kind());
                compile_binop!(name, self.out, self.lhs, self.rhs, ctx);
            }
        }
    };
}

#[cfg(feature = "runtime")]
pub mod catalog;
#[cfg(feature = "runtime")]
pub use self::catalog::*;

#[cfg(feature = "dot")]
pub mod dot;
#[cfg(feature = "matmul")]
pub mod matmul;
#[cfg(feature = "solve")]
pub mod solve;
#[cfg(feature = "transpose")]
pub mod transpose;
//pub mod cross;

#[cfg(feature = "dot")]
pub use self::dot::*;
#[cfg(feature = "matmul")]
pub use self::matmul::*;
#[cfg(feature = "solve")]
pub use self::solve::*;
#[cfg(feature = "transpose")]
pub use self::transpose::*;
//pub use self::cross::*;

// ----------------------------------------------------------------------------
// Matrix Library
// ----------------------------------------------------------------------------
