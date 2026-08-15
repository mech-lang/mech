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
use std::sync::LazyLock;

static PURE_COMPARE_SCALAR_CONTRACT: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| pure_compare_contract(ChangeDetectionPolicy::ExactScalar));
static PURE_COMPARE_MATRIX_CONTRACT: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| pure_compare_contract(ChangeDetectionPolicy::KernelReported));

fn pure_compare_contract(change_detection: ChangeDetectionPolicy) -> OperationContractDeclaration {
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

fn compare_full_write_contract(
    output: FunctionValueRepresentation,
) -> &'static OperationContractDeclaration {
    match output {
        FunctionValueRepresentation::Matrix { .. } => &PURE_COMPARE_MATRIX_CONTRACT,
        _ => &PURE_COMPARE_SCALAR_CONTRACT,
    }
}

#[cfg(feature = "runtime")]
pub mod catalog;
#[cfg(feature = "runtime")]
pub use self::catalog::*;

#[cfg(feature = "eq")]
pub mod eq;
#[cfg(feature = "gt")]
pub mod gt;
#[cfg(feature = "gte")]
pub mod gte;
#[cfg(feature = "lt")]
pub mod lt;
#[cfg(feature = "lte")]
pub mod lte;
#[cfg(feature = "max")]
pub mod max;
#[cfg(feature = "min")]
pub mod min;
#[cfg(feature = "neq")]
pub mod neq;
#[cfg(feature = "seq")]
pub mod seq;
#[cfg(feature = "sneq")]
pub mod sneq;

#[cfg(feature = "eq")]
pub use self::eq::*;
#[cfg(feature = "gt")]
pub use self::gt::*;
#[cfg(feature = "gte")]
pub use self::gte::*;
#[cfg(feature = "lt")]
pub use self::lt::*;
#[cfg(feature = "lte")]
pub use self::lte::*;
#[cfg(feature = "max")]
pub use self::max::*;
#[cfg(feature = "min")]
pub use self::min::*;
#[cfg(feature = "neq")]
pub use self::neq::*;
#[cfg(feature = "seq")]
pub use self::seq::*;
#[cfg(feature = "sneq")]
pub use self::sneq::*;

// ----------------------------------------------------------------------------
// Compare Library
// ----------------------------------------------------------------------------

#[macro_export]
macro_rules! impl_compare_binop {
    ($struct_name:ident, $arg1_type:ty, $arg2_type:ty, $out_type:ty, $op:ident) => {
        #[derive(Debug)]
        pub(crate) struct $struct_name<T> {
            lhs: Ref<$arg1_type>,
            rhs: Ref<$arg2_type>,
            out: Ref<$out_type>,
        }
        impl<T> MechFunctionFactory for $struct_name<T>
        where
            T: std::fmt::Debug + Clone + 'static + AsValueKind + PartialEq + PartialOrd,
            #[cfg(feature = "semantic-compiler")]
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
                        let lhs: Ref<$arg1_type> =
                            arg1.try_function_ref(FunctionArgumentRole::Input(0))?;
                        let rhs: Ref<$arg2_type> =
                            arg2.try_function_ref(FunctionArgumentRole::Input(1))?;
                        let out: Ref<$out_type> =
                            out.try_function_ref(FunctionArgumentRole::Output)?;
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
            T: std::fmt::Debug + Clone + 'static + PartialEq + PartialOrd,
            Ref<$out_type>: ToValue,
            $out_type: FunctionRuntimeType,
        {
            fn solve_result(&self) -> MResult<()> {
                let lhs_ptr = self.lhs.as_ptr();
                let rhs_ptr = self.rhs.as_ptr();
                let out_ptr = self.out.as_mut_ptr();
                $op!(lhs_ptr, rhs_ptr, out_ptr);
                Ok(())
            }
            fn out(&self) -> LegacyValue {
                self.out.to_value()
            }
            fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
                Some(compare_full_write_contract(
                    <$out_type as FunctionRuntimeType>::REPRESENTATION,
                ))
            }
            fn to_string(&self) -> String {
                format!("{:#?}", self)
            }

            fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
                Ok(self.reactive_output_values())
            }
        }
        #[cfg(feature = "semantic-compiler")]
        impl<T> MechFunctionCompiler for $struct_name<T>
        where
            T: ConstElem + CompileConst + AsValueKind,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!("{}<{}>", stringify!($struct_name), T::as_value_kind());
                compile_binop!(name, self.out, self.lhs, self.rhs, ctx);
            }
        }
    };
}

#[macro_export]
macro_rules! impl_compare_fxns {
    ($lib:ident) => {
        impl_fxns!($lib, T, bool, impl_compare_binop);
    };
}

#[macro_export]
macro_rules! impl_compare_fxns2 {
    ($lib:ident) => {
        impl_fxns!($lib, T, T, impl_compare_binop);
    };
}
