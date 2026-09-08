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

#[cfg(test)]
fn test_managed_factory<F: MechFunctionFactory>(
    invocation: FunctionInvocation,
    operation: &'static str,
) -> SpecializedFunction {
    let implementation = F::new_invocation(invocation.clone()).unwrap();
    let contract = F::declared_operation_contract()
        .or_else(|| implementation.semantic_operation_contract())
        .expect("managed statistics fixture requires an operation contract");
    SpecializedFunction::syntax_directed(
        (implementation, invocation),
        ResolvedOperationDescriptor::from_name(operation, contract.clone()).unwrap(),
        RuntimeFunctionId::from_name(operation),
        ExecutionTarget::DirectRuntime,
        F::implementation_memory_class(),
    )
    .unwrap()
}

#[cfg(test)]
fn assert_test_value(actual: &ValueCell, expected: ValueCell) {
    let actual = actual.snapshot().unwrap();
    let expected = expected.snapshot().unwrap();
    let actual_schemas = actual.schemas().unwrap();
    let expected_schemas = expected.schemas().unwrap();
    assert!(
        actual
            .language_eq(&actual_schemas, &expected, &expected_schemas)
            .unwrap()
    );
}

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
            arg: ManagedPort<T>,
            out: ManagedPort<T>,
            marker: core::marker::PhantomData<($arg_type, $out_type)>,
        }
        impl<T> MechFunctionFactory for $struct_name<T>
        where
            T: ManagedElement
                + CanonicalMatrixElementBacking
                + FunctionPortBacking
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

            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                mech_core::ImplementationMemoryClass::NoAdditionalScratch
            }

            fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
                Some(&PURE_STATS_REDUCTION_CONTRACT)
            }

            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                let (out, arg) = invocation.expect_unary()?;
                let _ = arg.try_managed::<$arg_type>()?;
                let _ = out.try_managed::<$out_type>()?;
                Ok(Box::new($struct_name {
                    arg: arg.try_managed_element::<T>()?,
                    out: out.try_managed_element::<T>()?,
                    marker: core::marker::PhantomData,
                }))
            }
        }
        impl<T> MechFunctionImpl for $struct_name<T>
        where
            T: ManagedElement
                + CanonicalMatrixElementBacking
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
            fn solve_managed(
                &self,
                frame: &mut mech_core::KernelMemoryFrame<'_>,
                _services: &mut dyn mech_core::MechExecutionServices,
            ) -> MResult<mech_core::ReactiveSolveStatus> {
                frame.with_unary_port_views(&self.arg, &self.out, |arg, out| $op!(arg, out))?;
                Ok(mech_core::ReactiveSolveStatus::Changed)
            }
            fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
                Some(FunctionStatePort::from_cell(self.out.cell()))
            }
            fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
                Ok(Some(vec![FunctionStatePort::from_cell(self.out.cell())]))
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
                let output = compile_value_cell_register(self.out.cell(), ctx)?;
                let input = compile_value_cell_register(self.arg.cell(), ctx)?;
                let function = ctx.function_id(&name)?;
                ctx.emit_unop(function, output, input);
                Ok(output)
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
