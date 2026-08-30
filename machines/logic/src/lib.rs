#![cfg_attr(not(test), no_main)]
#![feature(where_clause_attrs)]
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[doc(hidden)]
#[cfg(feature = "native-link")]
pub mod __mech_native {
    pub use crate::catalog::__mech_native::*;
}

#[cfg(all(
    any(feature = "and", feature = "or", feature = "xor"),
    feature = "matrix"
))]
extern crate nalgebra as na;
extern crate paste;

use mech_core::*;

#[cfg(feature = "runtime")]
pub mod catalog;
#[cfg(feature = "runtime")]
pub use self::catalog::*;

#[cfg(all(
    any(feature = "and", feature = "or", feature = "xor"),
    feature = "matrixd"
))]
use nalgebra::DMatrix;
#[cfg(all(
    any(feature = "and", feature = "or", feature = "xor"),
    feature = "vectord"
))]
use nalgebra::DVector;
#[cfg(all(
    any(feature = "and", feature = "or", feature = "xor"),
    feature = "matrix1"
))]
use nalgebra::Matrix1;
#[cfg(all(
    any(feature = "and", feature = "or", feature = "xor"),
    feature = "matrix2"
))]
use nalgebra::Matrix2;
#[cfg(all(
    any(feature = "and", feature = "or", feature = "xor"),
    feature = "matrix2x3"
))]
use nalgebra::Matrix2x3;
#[cfg(all(
    any(feature = "and", feature = "or", feature = "xor"),
    feature = "matrix3"
))]
use nalgebra::Matrix3;
#[cfg(all(
    any(feature = "and", feature = "or", feature = "xor"),
    feature = "matrix3x2"
))]
use nalgebra::Matrix3x2;
#[cfg(all(
    any(feature = "and", feature = "or", feature = "xor"),
    feature = "matrix4"
))]
use nalgebra::Matrix4;
#[cfg(all(
    any(feature = "and", feature = "or", feature = "xor"),
    feature = "row_vectord"
))]
use nalgebra::RowDVector;
#[cfg(all(
    any(feature = "and", feature = "or", feature = "xor"),
    feature = "row_vector2"
))]
use nalgebra::RowVector2;
#[cfg(all(
    any(feature = "and", feature = "or", feature = "xor"),
    feature = "row_vector3"
))]
use nalgebra::RowVector3;
#[cfg(all(
    any(feature = "and", feature = "or", feature = "xor"),
    feature = "row_vector4"
))]
use nalgebra::RowVector4;
#[cfg(all(
    any(feature = "and", feature = "or", feature = "xor"),
    feature = "vector2"
))]
use nalgebra::Vector2;
#[cfg(all(
    any(feature = "and", feature = "or", feature = "xor"),
    feature = "vector3"
))]
use nalgebra::Vector3;
#[cfg(all(
    any(feature = "and", feature = "or", feature = "xor"),
    feature = "vector4"
))]
use nalgebra::Vector4;

#[cfg(any(
    feature = "and",
    feature = "not",
    feature = "or",
    feature = "xor"
))]
use std::sync::LazyLock;

#[cfg(any(feature = "and", feature = "or", feature = "xor"))]
static PURE_LOGIC_BINARY_EXACT_SCALAR: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| logic_full_write_contract(2, ChangeDetectionPolicy::ExactScalar));
#[cfg(any(feature = "and", feature = "or", feature = "xor"))]
static PURE_LOGIC_BINARY_KERNEL_REPORTED: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| logic_full_write_contract(2, ChangeDetectionPolicy::KernelReported));
#[cfg(feature = "not")]
static PURE_LOGIC_UNARY_EXACT_SCALAR: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| logic_full_write_contract(1, ChangeDetectionPolicy::ExactScalar));
#[cfg(feature = "not")]
static PURE_LOGIC_UNARY_KERNEL_REPORTED: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| logic_full_write_contract(1, ChangeDetectionPolicy::KernelReported));

fn logic_full_write_contract(
    input_count: usize,
    change_detection: ChangeDetectionPolicy,
) -> OperationContractDeclaration {
    OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(
            (0..input_count)
                .map(|_| InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                })
                .collect::<Vec<_>>()
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

#[cfg(any(feature = "and", feature = "or", feature = "xor"))]
fn logic_binary_full_write_contract(
    output: FunctionValueRepresentation,
) -> &'static OperationContractDeclaration {
    match output {
        FunctionValueRepresentation::Matrix { .. } => &PURE_LOGIC_BINARY_KERNEL_REPORTED,
        _ => &PURE_LOGIC_BINARY_EXACT_SCALAR,
    }
}

#[cfg(feature = "not")]
fn logic_unary_full_write_contract(
    output: FunctionValueRepresentation,
) -> &'static OperationContractDeclaration {
    match output {
        FunctionValueRepresentation::Matrix { .. } => &PURE_LOGIC_UNARY_KERNEL_REPORTED,
        _ => &PURE_LOGIC_UNARY_EXACT_SCALAR,
    }
}

#[cfg(feature = "and")]
pub mod and;
#[cfg(feature = "not")]
pub mod not;
#[cfg(feature = "or")]
pub mod or;
#[cfg(feature = "xor")]
pub mod xor;

#[cfg(all(feature = "and", feature = "source"))]
pub use self::and::*;
#[cfg(all(feature = "not", feature = "source"))]
pub use self::not::*;
#[cfg(all(feature = "or", feature = "source"))]
pub use self::or::*;
#[cfg(all(feature = "xor", feature = "source"))]
pub use self::xor::*;

// ----------------------------------------------------------------------------
// Logic Library
// ----------------------------------------------------------------------------

#[macro_export]
macro_rules! impl_logic_binop {
    ($struct_name:ident, $arg1_type:ty, $arg2_type:ty, $out_type:ty, $op:ident) => {
        #[derive(Debug)]
        pub(crate) struct $struct_name {
            lhs: Ref<$arg1_type>,
            rhs: Ref<$arg2_type>,
            out: Ref<$out_type>,
        }
        impl MechFunctionFactory for $struct_name {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
                <$out_type as FunctionRuntimeType>::REPRESENTATION,
                <$arg1_type as FunctionRuntimeType>::REPRESENTATION,
                <$arg2_type as FunctionRuntimeType>::REPRESENTATION,
            );

            fn new_invocation(
                invocation: FunctionInvocation,
            ) -> MResult<Box<dyn MechFunction>> {
                let (out, lhs, rhs) = invocation.expect_binary()?;
                let lhs: Ref<$arg1_type> = lhs.try_ref()?;
                let rhs: Ref<$arg2_type> = rhs.try_ref()?;
                let out: Ref<$out_type> = out.try_ref()?;
                Ok(Box::new(Self { lhs, rhs, out }))
            }

        }
        impl MechFunctionImpl for $struct_name {
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
                Some($crate::logic_binary_full_write_contract(
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
        impl MechFunctionCompiler for $struct_name {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!("{}<bool>", stringify!($struct_name));
                compile_binop!(name, self.out, self.lhs, self.rhs, ctx);
            }
        }
    };
}

#[macro_export]
macro_rules! impl_logic_fxns {
    ($lib:ident) => {
        impl_fxns!($lib, bool, bool, impl_logic_binop);
    };
}

#[cfg(feature = "source")]
fn specialize_logic_binary_factory<F>(
    first: &SpecializationInput,
    second: &SpecializationInput,
) -> MResult<SpecializedFunction>
where
    F: MechFunctionFactory,
{
    let output_representation = F::SIGNATURE.output;
    let template = if first.representation() == Some(output_representation) {
        first
    } else if second.representation() == Some(output_representation) {
        second
    } else {
        return Err(MechError::new(
            FunctionArgumentTypeMismatch {
                role: FunctionArgumentRole::Output,
                expected: format!("{output_representation:?}"),
                found: format!(
                    "inputs {:?} and {:?}",
                    first.representation(),
                    second.representation(),
                ),
            },
            None,
        )
        .with_compiler_loc());
    };
    let invocation = FunctionInvocation::binary(
        template.cell()?.detached_clone()?,
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
macro_rules! __try_logic_binary_factory {
    (($module:ident, $first:ident, $second:ident), $lib:ident, $suffix:ident, $shape_features:tt, $scalar:ty, $scalar_name:literal, $scalar_token:ident) => {
        mech_core::paste::paste! {
            if let RuntimeFunctionInputs::Binary(expected_first, expected_second) =
                <$crate::$module::[<$lib $suffix>] as MechFunctionFactory>::SIGNATURE.inputs
                && $first.representation() == Some(expected_first)
                && $second.representation() == Some(expected_second)
            {
                return $crate::specialize_logic_binary_factory::<
                    $crate::$module::[<$lib $suffix>]
                >($first, $second);
            }
        }
    };
}

#[macro_export]
macro_rules! impl_canonical_logic_binop_specializer {
    ($specializer:ident, $module:ident, $lib:ident, $operation:literal) => {
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
                mech_core::__mech_for_each_exact_binop_runtime_factory_for_type!(
                    $crate::__try_logic_binary_factory,
                    ($module, first, second),
                    $lib,
                    bool,
                    "bool",
                    bool
                );
                Err(MechError::new(
                    FunctionArgumentTypeMismatch {
                        role: FunctionArgumentRole::Input(0),
                        expected: concat!("supported Bool inputs for ", $operation).into(),
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

#[cfg(all(
    test,
    feature = "runtime",
    feature = "and",
    feature = "not",
    feature = "bool",
    feature = "matrix2",
    feature = "matrixd"
))]
mod invocation_port_tests {
    use super::*;
    use nalgebra::{DMatrix, Matrix2};

    #[test]
    fn scalar_binary_and_unary_factories_use_canonical_ports() {
        let binary_out = ValueCell::from_exact(true).unwrap();
        let binary = crate::and::AndSS::new_invocation(FunctionInvocation::binary(
            binary_out.clone(),
            ValueCell::from_exact(true).unwrap(),
            ValueCell::from_exact(false).unwrap(),
        ))
        .unwrap();
        binary.solve_result().unwrap();
        assert!(matches!(
            binary_out.snapshot().unwrap().data(),
            ValueData::Bool(false)
        ));
        assert_eq!(
            binary.reactive_output_cell_ids(),
            vec![binary_out.reactive_cell_id()]
        );

        let unary_out = ValueCell::from_exact(false).unwrap();
        let unary = crate::not::NotS::<bool>::new_invocation(FunctionInvocation::unary(
            unary_out.clone(),
            ValueCell::from_exact(true).unwrap(),
        ))
        .unwrap();
        unary.solve_result().unwrap();

        with_reactive_journal_participant(|mut participant| -> MResult<()> {
            participant.capture_function_state(binary.as_ref())?;
            participant.capture_function_state(unary.as_ref())?;
            binary_out.replace(&ValueCell::from_exact(true)?.snapshot()?)?;
            unary_out.replace(&ValueCell::from_exact(true)?.snapshot()?)?;
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();
        assert!(matches!(
            binary_out.snapshot().unwrap().data(),
            ValueData::Bool(false)
        ));
        assert!(matches!(
            unary_out.snapshot().unwrap().data(),
            ValueData::Bool(false)
        ));
    }

    #[test]
    fn fixed_and_dynamic_logic_factories_preserve_exact_storage() {
        let fixed_lhs = Ref::new(Matrix2::new(true, true, false, false));
        let fixed_rhs = Ref::new(Matrix2::new(true, false, true, false));
        let fixed_out = Ref::new(Matrix2::from_element(false));
        crate::and::AndM2M2::new_invocation(FunctionInvocation::binary(
            ValueCell::from_exact_matrix_ref(fixed_out.clone(), 2, 2).unwrap(),
            ValueCell::from_exact_matrix_ref(fixed_lhs, 2, 2).unwrap(),
            ValueCell::from_exact_matrix_ref(fixed_rhs, 2, 2).unwrap(),
        ))
        .unwrap()
        .solve_result()
        .unwrap();
        assert_eq!(
            *fixed_out.borrow(),
            Matrix2::new(true, false, false, false)
        );

        let dynamic_lhs = Ref::new(DMatrix::from_row_slice(
            2,
            2,
            &[true, true, false, false],
        ));
        let dynamic_rhs = Ref::new(DMatrix::from_row_slice(
            2,
            2,
            &[true, false, true, false],
        ));
        let dynamic_out = Ref::new(DMatrix::from_element(2, 2, false));
        let function = crate::and::AndMDMD::new_invocation(FunctionInvocation::binary(
            ValueCell::from_exact_matrix_ref(dynamic_out.clone(), 2, 2).unwrap(),
            ValueCell::from_exact_matrix_ref(dynamic_lhs, 2, 2).unwrap(),
            ValueCell::from_exact_matrix_ref(dynamic_rhs, 2, 2).unwrap(),
        ))
        .unwrap();
        function.solve_result().unwrap();
        with_reactive_journal_participant(|mut participant| -> MResult<()> {
            participant.capture_function_state(function.as_ref())?;
            *dynamic_out.borrow_mut() = DMatrix::from_element(1, 1, true);
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();
        assert_eq!(
            *dynamic_out.borrow(),
            DMatrix::from_row_slice(2, 2, &[true, false, false, false])
        );
    }

    #[test]
    fn logic_ports_reject_wrong_types_and_layouts() {
        assert!(crate::and::AndSS::new_invocation(FunctionInvocation::binary(
            ValueCell::from_exact(false).unwrap(),
            ValueCell::from_exact(1_usize).unwrap(),
            ValueCell::from_exact(true).unwrap(),
        ))
        .is_err());
        assert!(crate::and::AndSS::new_invocation(FunctionInvocation::unary(
            ValueCell::from_exact(false).unwrap(),
            ValueCell::from_exact(true).unwrap(),
        ))
        .is_err());
    }
}
