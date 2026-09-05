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
use std::sync::LazyLock;

static PURE_STRING_BINARY_EXACT_SCALAR: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| string_binary_contract(ChangeDetectionPolicy::ExactScalar));
static PURE_STRING_BINARY_KERNEL_REPORTED: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| string_binary_contract(ChangeDetectionPolicy::KernelReported));

fn string_binary_contract(change_detection: ChangeDetectionPolicy) -> OperationContractDeclaration {
    OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(
            vec![
                InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                },
                InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                },
            ]
            .into_boxed_slice(),
        ),
        outputs: vec![OutputPortPolicy {
            access: AccessMode::Write,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
            },
            alias: AliasPolicy::NoAlias,
            change_detection,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    }
}

fn string_binary_full_write_contract(
    output: FunctionValueRepresentation,
) -> &'static OperationContractDeclaration {
    match output {
        FunctionValueRepresentation::Matrix { .. } => &PURE_STRING_BINARY_KERNEL_REPORTED,
        _ => &PURE_STRING_BINARY_EXACT_SCALAR,
    }
}

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

#[cfg(feature = "runtime")]
pub mod catalog;
#[cfg(feature = "runtime")]
pub use self::catalog::*;

#[cfg(feature = "concat")]
pub mod concat;

#[cfg(all(feature = "concat", feature = "source"))]
pub use self::concat::*;

// ----------------------------------------------------------------------------
// String Library
// ----------------------------------------------------------------------------

pub trait Concat {
    fn concat(&self, rhs: &Self) -> Self;
}

impl Concat for String {
    fn concat(&self, rhs: &Self) -> Self {
        let mut s = self.clone();
        s.push_str(rhs);
        s
    }
}

#[macro_export]
macro_rules! impl_string_binop {
    ($struct_name:ident, $arg1_type:ty, $arg2_type:ty, $out_type:ty, $op:ident) => {
        #[derive(Debug)]
        pub(crate) struct $struct_name<T> {
            lhs: Ref<$arg1_type>,
            rhs: Ref<$arg2_type>,
            out: Ref<$out_type>,
        }
        impl<T> MechFunctionFactory for $struct_name<T>
        where
            T: std::fmt::Debug + Clone + Sync + Send + 'static + FunctionRuntimeType + Concat,
            #[cfg(feature = "semantic-compiler")]
            T: CanonicalMatrixElementBacking + ConstElem + CompileConst,
            $arg1_type: FunctionPortBacking,
            $arg2_type: FunctionPortBacking,
            $out_type: FunctionStateBacking,
        {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
                <$out_type as FunctionRuntimeType>::REPRESENTATION,
                <$arg1_type as FunctionRuntimeType>::REPRESENTATION,
                <$arg2_type as FunctionRuntimeType>::REPRESENTATION,
            );

            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                mech_core::ImplementationMemoryClass::NoAdditionalScratch
            }

            fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
                Some($crate::string_binary_full_write_contract(
                    <$out_type as FunctionRuntimeType>::REPRESENTATION,
                ))
            }

            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                let (out, lhs, rhs) = invocation.expect_binary()?;
                let lhs: Ref<$arg1_type> = lhs.try_ref()?;
                let rhs: Ref<$arg2_type> = rhs.try_ref()?;
                let out: Ref<$out_type> = out.try_ref()?;
                Ok(Box::new(Self { lhs, rhs, out }))
            }
        }
        impl<T> MechFunctionImpl for $struct_name<T>
        where
            T: std::fmt::Debug + Clone + Sync + Send + 'static + Concat,
            #[cfg(feature = "semantic-compiler")]
            T: CanonicalMatrixElementBacking,
            $out_type: FunctionStateBacking,
        {
            fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
                Some(FunctionStatePort::from_ref(&self.out))
            }
            fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
                Ok(Some(vec![FunctionStatePort::from_ref(&self.out)]))
            }
            fn solve_result(&self) -> MResult<()> {
                let lhs_ptr = self.lhs.as_ptr();
                let rhs_ptr = self.rhs.as_ptr();
                let out_ptr = self.out.as_mut_ptr();
                $op!(lhs_ptr, rhs_ptr, out_ptr);
                Ok(())
            }
            fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
                Some($crate::string_binary_full_write_contract(
                    <$out_type as FunctionRuntimeType>::REPRESENTATION,
                ))
            }
            fn to_string(&self) -> String {
                format!("{:#?}", self)
            }
        }
        #[cfg(feature = "semantic-compiler")]
        impl<T> MechFunctionCompiler for $struct_name<T>
        where
            T: CanonicalMatrixElementBacking + ConstElem + CompileConst + FunctionRuntimeType,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!(
                    "{}<{}>",
                    stringify!($struct_name),
                    <T as FunctionRuntimeType>::REPRESENTATION
                );
                compile_binop!(name, self.out, self.lhs, self.rhs, ctx);
            }
        }
    };
}

#[macro_export]
macro_rules! impl_string_fxns {
    ($lib:ident) => {
        impl_fxns!($lib, T, T, impl_string_binop);
    };
}
