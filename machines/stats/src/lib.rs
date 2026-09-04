#![cfg_attr(not(test), no_main)]
#![feature(where_clause_attrs)]

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[doc(hidden)]
#[cfg(feature = "native-link")]
pub mod __mech_native {
    pub use crate::catalog::__mech_native::*;
}

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

use std::fmt::Debug;
use std::ops::*;
use std::sync::LazyLock;

static PURE_STATS_REDUCTION_CONTRACT: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(
            vec![InputPortPolicy {
                access: AccessMode::Read,
                delivery: DeliveryMode::Signal,
            }]
            .into_boxed_slice(),
        ),
        outputs: vec![OutputPortPolicy {
            access: AccessMode::Write,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
            },
            alias: AliasPolicy::NoAlias,
            change_detection: ChangeDetectionPolicy::KernelReported,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    });

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
    fn stats_checked_add(self, rhs: Self) -> Option<Self> {
        self.checked_add(rhs)
    }
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

#[cfg(all(feature = "sum", feature = "source"))]
pub use self::sum_column::*;
#[cfg(all(feature = "sum", feature = "source"))]
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
                + FunctionRuntimeType
                + Zero
                + One
                + PartialEq
                + PartialOrd,
            T: StatsCheckedAdd,
            #[cfg(feature = "semantic-compiler")]
            T: CanonicalMatrixElementBacking + CompileConst + ConstElem,
            $arg_type: FunctionPortBacking,
            $out_type: FunctionStateBacking,
        {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::unary(
                <$out_type as FunctionRuntimeType>::REPRESENTATION,
                <$arg_type as FunctionRuntimeType>::REPRESENTATION,
            );

            fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
                Some(&PURE_STATS_REDUCTION_CONTRACT)
            }

            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                let (out, arg) = invocation.expect_unary()?;
                let arg: Ref<$arg_type> = arg.try_ref()?;
                let out: Ref<$out_type> = out.try_ref()?;
                Ok(Box::new($struct_name { arg, out }))
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
            #[cfg(feature = "semantic-compiler")]
            T: CanonicalMatrixElementBacking,
            $out_type: FunctionStateBacking,
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
            fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
                Some(FunctionStatePort::from_ref(&self.out))
            }
            fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
                Ok(Some(vec![FunctionStatePort::from_ref(&self.out)]))
            }
            fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
                Some(&PURE_STATS_REDUCTION_CONTRACT)
            }
            fn to_string(&self) -> String {
                format!("{:#?}", self)
            }
        }
        #[cfg(feature = "semantic-compiler")]
        impl<T> MechFunctionCompiler for $struct_name<T>
        where
            T: CanonicalMatrixElementBacking + CompileConst + ConstElem + FunctionRuntimeType,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!(
                    "{}<{}>",
                    stringify!($struct_name),
                    <T as FunctionRuntimeType>::REPRESENTATION
                );
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
