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

#[cfg(any(feature = "and", feature = "or", feature = "xor"))]
use paste::paste;
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

            fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
                Self::new_invocation(args.into())
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
            fn out(&self) -> LegacyValue {
                self.out.to_value()
            }
            fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
                Some($crate::logic_binary_full_write_contract(
                    <$out_type as FunctionRuntimeType>::REPRESENTATION,
                ))
            }
            fn to_string(&self) -> String {
                format!("{:#?}", self)
            }

            fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
                Ok(self.reactive_output_values())
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

    fn binary_args<T>(out: &Ref<T>, lhs: &Ref<T>, rhs: &Ref<T>) -> FunctionArgs
    where
        Ref<T>: ToValue,
    {
        FunctionArgs::Binary(out.to_value(), lhs.to_value(), rhs.to_value())
    }

    #[test]
    fn scalar_binary_and_unary_factories_use_invocation_ports() {
        let lhs = Ref::new(true);
        let rhs = Ref::new(false);
        let binary_out = Ref::new(true);
        let binary = crate::and::AndSS::new_invocation(
            binary_args(&binary_out, &lhs, &rhs).into(),
        )
        .unwrap();
        binary.solve_result().unwrap();
        assert!(!*binary_out.borrow());
        assert_eq!(
            binary.reactive_output_cell_ids(),
            binary.out().reactive_root_cell_ids(),
        );

        let unary_out = Ref::new(false);
        let unary = crate::not::NotS::<bool>::new_invocation(
            FunctionArgs::Unary(unary_out.to_value(), lhs.to_value()).into(),
        )
        .unwrap();
        unary.solve_result().unwrap();
        assert!(!*unary_out.borrow());

        with_reactive_journal_participant(|mut participant| {
            participant.capture_function_state(&*binary)?;
            participant.capture_function_state(&*unary)?;
            *binary_out.borrow_mut() = true;
            *unary_out.borrow_mut() = true;
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();
        assert!(!*binary_out.borrow());
        assert!(!*unary_out.borrow());
    }

    #[test]
    fn fixed_and_dynamic_matrix_factories_preserve_storage() {
        let fixed_lhs = Ref::new(Matrix2::new(true, true, false, false));
        let fixed_rhs = Ref::new(Matrix2::new(true, false, true, false));
        let fixed_out = Ref::new(Matrix2::from_element(false));
        let fixed = crate::and::AndM2M2::new_invocation(
            binary_args(&fixed_out, &fixed_lhs, &fixed_rhs).into(),
        )
        .unwrap();
        fixed.solve_result().unwrap();
        assert_eq!(
            *fixed_out.borrow(),
            Matrix2::new(true, false, false, false)
        );
        with_reactive_journal_participant(|mut participant| {
            participant.capture_function_state(&*fixed)?;
            *fixed_out.borrow_mut() = Matrix2::from_element(true);
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();
        assert_eq!(
            *fixed_out.borrow(),
            Matrix2::new(true, false, false, false),
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
        let dynamic = crate::and::AndMDMD::new_invocation(
            binary_args(&dynamic_out, &dynamic_lhs, &dynamic_rhs).into(),
        )
        .unwrap();
        dynamic.solve_result().unwrap();
        assert_eq!(
            *dynamic_out.borrow(),
            DMatrix::from_row_slice(2, 2, &[true, false, false, false])
        );
        with_reactive_journal_participant(|mut participant| {
            participant.capture_function_state(&*dynamic)?;
            *dynamic_out.borrow_mut() = DMatrix::from_element(1, 1, true);
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();
        assert_eq!(
            *dynamic_out.borrow(),
            DMatrix::from_row_slice(2, 2, &[true, false, false, false]),
        );
    }

    #[test]
    fn logic_ports_reject_wrong_types_and_layouts() {
        let out = Ref::new(false);
        let scalar = Ref::new(true);
        let matrix = Ref::new(Matrix2::from_element(true));
        let type_error = crate::and::AndSS::new_invocation(
            FunctionArgs::Binary(out.to_value(), matrix.to_value(), scalar.to_value()).into(),
        )
        .err()
        .expect("wrong exact input representation must fail");
        assert_eq!(type_error.kind_name(), "FunctionArgumentTypeMismatch");

        let arity_error = crate::and::AndSS::new_invocation(
            FunctionArgs::Unary(out.to_value(), scalar.to_value()).into(),
        )
        .err()
        .expect("wrong layout must fail");
        assert_eq!(arity_error.kind_name(), "IncorrectNumberOfArguments");
    }
}
