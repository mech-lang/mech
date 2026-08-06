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
extern crate paste;

use mech_core::*;

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

use paste::paste;
use std::fmt::Debug;
use std::ops::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatsArithmeticOverflow {
    pub operation: &'static str,
    pub operand_type: &'static str,
}

impl MechErrorKind for StatsArithmeticOverflow {
    fn name(&self) -> &str {
        "StatsArithmeticOverflow"
    }

    fn message(&self) -> String {
        format!(
            "{} overflows operand type {}",
            self.operation, self.operand_type,
        )
    }
}

pub trait StatsCheckedAdd: Copy {
    fn stats_checked_add(self, rhs: Self) -> Option<Self>;
}

macro_rules! impl_checked_integer_sum {
    ($($type:ty),+ $(,)?) => {
        $(
            impl StatsCheckedAdd for $type {
                fn stats_checked_add(self, rhs: Self) -> Option<Self> { self.checked_add(rhs) }
            }
        )+
    };
}

impl_checked_integer_sum!(i8, i16, i32, i64, i128, u8, u16, u32, u64, u128);

macro_rules! impl_unbounded_sum {
    ($($type:ty),+ $(,)?) => {
        $(
            impl StatsCheckedAdd for $type {
                fn stats_checked_add(self, rhs: Self) -> Option<Self> { Some(self + rhs) }
            }
        )+
    };
}

impl_unbounded_sum!(f32, f64);
#[cfg(feature = "complex")]
impl_unbounded_sum!(C64);

#[cfg(feature = "rational")]
impl StatsCheckedAdd for R64 {
    fn stats_checked_add(self, rhs: Self) -> Option<Self> { self.checked_add(rhs) }
}

fn checked_sum_add<T: StatsCheckedAdd>(lhs: T, rhs: T) -> MResult<T> {
    lhs.stats_checked_add(rhs).ok_or_else(|| {
        MechError::new(
            StatsArithmeticOverflow {
                operation: "statistics sum",
                operand_type: std::any::type_name::<T>(),
            },
            None,
        )
        .with_compiler_loc()
    })
}

#[cfg(feature = "runtime")]
pub mod catalog;
#[cfg(feature = "runtime")]
pub use self::catalog::*;

#[cfg(feature = "sum")]
pub mod sum_column;
#[cfg(feature = "sum")]
pub mod sum_row;

#[cfg(feature = "sum")]
pub use self::sum_column::*;
#[cfg(feature = "sum")]
pub use self::sum_row::*;

#[macro_export]
macro_rules! impl_stats_unop {
    ($struct_name:ident, $arg_type:ty, $out_type:ty, $op:ident) => {
        #[derive(Debug)]
        pub(crate) struct $struct_name<T> {
            arg: Ref<$arg_type>,
            out: Ref<$out_type>,
        }
        impl<T> MechFunctionFactory for $struct_name<T>
        where
            T: Copy
                + Debug
                + Clone
                + Sync
                + Send
                + 'static
                + Add<Output = T>
                + AddAssign
                + AsValueKind
                + Zero
                + One
                + PartialEq
                + PartialOrd,
            T: StatsCheckedAdd,
            #[cfg(feature = "compiler")]
            T: CompileConst + ConstElem,
            Ref<$out_type>: ToValue,
            $arg_type: FunctionRuntimeType,
            $out_type: FunctionRuntimeType,
        {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::unary(
                <$out_type as FunctionRuntimeType>::REPRESENTATION,
                <$arg_type as FunctionRuntimeType>::REPRESENTATION,
            );

            fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
                match args {
                    FunctionArgs::Unary(out, arg) => {
                        let arg = arg.try_function_ref(FunctionArgumentRole::Input(0))?;
                        let out = out.try_function_ref(FunctionArgumentRole::Output)?;
                        Ok(Box::new($struct_name { arg, out }))
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
            T: Copy
                + Debug
                + Clone
                + Sync
                + Send
                + 'static
                + Add<Output = T>
                + AddAssign
                + Zero
                + One
                + PartialEq
                + PartialOrd,
            T: StatsCheckedAdd,
            Ref<$out_type>: ToValue,
        {
            fn solve_result(&self) -> MResult<()> {
                let mut next = self.out.borrow().clone();
                {
                    let arg = self.arg.borrow();
                    $op!(&*arg, &mut next)?;
                }
                *self.out.borrow_mut() = next;
                Ok(())
            }
            fn out(&self) -> Value {
                self.out.to_value()
            }
            fn to_string(&self) -> String {
                format!("{:#?}", self)
            }

            fn transaction_state_values(&self) -> MResult<Vec<Value>> {
                Ok(self.reactive_output_values())
            }
        }
        #[cfg(feature = "compiler")]
        impl<T> MechFunctionCompiler for $struct_name<T>
        where
            T: CompileConst + ConstElem + AsValueKind,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!("{}<{}>", stringify!($struct_name), T::as_value_kind());
                compile_unop!(name, self.out, self.arg, ctx);
            }
        }
    };
}

#[macro_export]
macro_rules! impls_stas {
    ($name:ident, $arg_type:ty, $out_type:ty, $op:ident) => {
        impl_stats_unop!($name, $arg_type, $out_type, $op);
    };
}
