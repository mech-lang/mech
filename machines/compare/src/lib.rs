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

use std::sync::LazyLock;

#[cfg(feature = "source")]
pub(crate) fn semantic_compare_extents(inputs: &[&SpecializationInput]) -> MResult<Box<[u64]>> {
    let mut result: Option<[u64; 2]> = None;
    for input in inputs {
        let extents = input
            .cell()?
            .resolved_descriptor()?
            .current_extents()
            .map_err(MechError::from)?;
        if !extents.is_empty() {
            let [rows, columns] = extents.as_ref() else {
                return Err(MechError::new(
                    GenericError {
                        msg: "comparison requires scalar or rank-two inputs".into(),
                    },
                    None,
                )
                .with_compiler_loc());
            };
            result = Some(match result {
                None => [*rows, *columns],
                Some([left_rows, left_columns]) => {
                    let axis = |left: u64, right: u64| {
                        if left == right {
                            Some(left)
                        } else if left == 1 {
                            Some(right)
                        } else if right == 1 {
                            Some(left)
                        } else {
                            None
                        }
                    };
                    [
                        axis(left_rows, *rows).ok_or_else(|| {
                            MechError::new(
                                DimensionMismatch {
                                    dims: vec![
                                        left_rows as usize,
                                        left_columns as usize,
                                        *rows as usize,
                                        *columns as usize,
                                    ],
                                },
                                None,
                            )
                            .with_compiler_loc()
                        })?,
                        axis(left_columns, *columns).ok_or_else(|| {
                            MechError::new(
                                DimensionMismatch {
                                    dims: vec![
                                        left_rows as usize,
                                        left_columns as usize,
                                        *rows as usize,
                                        *columns as usize,
                                    ],
                                },
                                None,
                            )
                            .with_compiler_loc()
                        })?,
                    ]
                }
            });
        }
    }
    Ok(result.map_or_else(
        || Vec::<u64>::new().into_boxed_slice(),
        |shape| shape.into_iter().collect::<Vec<_>>().into_boxed_slice(),
    ))
}

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

#[macro_export]
macro_rules! impl_canonical_numeric_compare_specializer {
    ($specializer:ident, $module:ident, $lib:ident, $operation:literal) => {
        #[cfg(feature = "source")]
        pub struct $specializer;

        #[cfg(feature = "source")]
        impl CanonicalFunctionSpecializer for $specializer {
            fn specialize_invocation(
                &self,
                specialization: &SpecializationInvocation,
                context: &mut SpecializationContext<'_>,
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
                let first = specialization.input(0).expect("validated comparison lhs");
                let second = specialization.input(1).expect("validated comparison rhs");
                let extents = $crate::semantic_compare_extents(&[first, second])?;
                context.bind_resolved_runtime(
                    RuntimeBindingSelector::Operation(context.resolved_call()?.operation.id),
                    ExecutionTarget::DirectRuntime,
                    vec![extents].into_boxed_slice(),
                    &[first, second],
                )
            }
        }
    };
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

#[cfg(all(feature = "eq", feature = "source"))]
pub use self::eq::*;
#[cfg(all(feature = "eq", feature = "runtime", not(feature = "source")))]
pub(crate) use self::eq::*;
#[cfg(all(feature = "gt", feature = "source"))]
pub use self::gt::*;
#[cfg(all(feature = "gt", feature = "runtime", not(feature = "source")))]
pub(crate) use self::gt::*;
#[cfg(all(feature = "gte", feature = "source"))]
pub use self::gte::*;
#[cfg(all(feature = "gte", feature = "runtime", not(feature = "source")))]
pub(crate) use self::gte::*;
#[cfg(all(feature = "lt", feature = "source"))]
pub use self::lt::*;
#[cfg(all(feature = "lt", feature = "runtime", not(feature = "source")))]
pub(crate) use self::lt::*;
#[cfg(all(feature = "lte", feature = "source"))]
pub use self::lte::*;
#[cfg(all(feature = "lte", feature = "runtime", not(feature = "source")))]
pub(crate) use self::lte::*;
#[cfg(all(feature = "max", feature = "source"))]
pub use self::max::*;
#[cfg(all(feature = "max", feature = "runtime", not(feature = "source")))]
pub(crate) use self::max::*;
#[cfg(all(feature = "min", feature = "source"))]
pub use self::min::*;
#[cfg(all(feature = "min", feature = "runtime", not(feature = "source")))]
pub(crate) use self::min::*;
#[cfg(all(feature = "neq", feature = "source"))]
pub use self::neq::*;
#[cfg(all(feature = "neq", feature = "runtime", not(feature = "source")))]
pub(crate) use self::neq::*;
#[cfg(all(feature = "seq", feature = "source"))]
pub use self::seq::*;
#[cfg(all(feature = "seq", feature = "runtime", not(feature = "source")))]
pub(crate) use self::seq::*;
#[cfg(all(feature = "sneq", feature = "source"))]
pub use self::sneq::*;
#[cfg(all(feature = "sneq", feature = "runtime", not(feature = "source")))]
pub(crate) use self::sneq::*;

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
            T: std::fmt::Debug + Clone + 'static + FunctionRuntimeType + PartialEq + PartialOrd,
            #[cfg(feature = "semantic-compiler")]
            T: CanonicalMatrixElementBacking + ConstElem + CompileConst,
            $arg1_type: FunctionRuntimeType + FunctionPortBacking,
            $arg2_type: FunctionRuntimeType + FunctionPortBacking,
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
                Some(compare_full_write_contract(
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
            T: std::fmt::Debug + Clone + 'static + PartialEq + PartialOrd,
            #[cfg(feature = "semantic-compiler")]
            T: CanonicalMatrixElementBacking,
            $out_type: FunctionStateBacking,
        {
            fn solve_result(&self) -> MResult<()> {
                let lhs_ptr = self.lhs.as_ptr();
                let rhs_ptr = self.rhs.as_ptr();
                let out_ptr = self.out.as_mut_ptr();
                $op!(lhs_ptr, rhs_ptr, out_ptr);
                Ok(())
            }
            fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
                Some(FunctionStatePort::from_ref(&self.out))
            }
            fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
                Some(compare_full_write_contract(
                    <$out_type as FunctionRuntimeType>::REPRESENTATION,
                ))
            }
            fn to_string(&self) -> String {
                format!("{:#?}", self)
            }

            fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
                Ok(Some(vec![FunctionStatePort::from_ref(&self.out)]))
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
