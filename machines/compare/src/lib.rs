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

#[cfg(feature = "source")]
fn compare_boolean_output(
    representation: FunctionValueRepresentation,
    first: &SpecializationInput,
    second: &SpecializationInput,
) -> MResult<ValueCell> {
    let FunctionValueRepresentation::Matrix {
        element: FunctionMatrixElement::Bool,
        storage: FunctionMatrixStoragePattern::Exact(storage),
    } = representation
    else {
        if representation == FunctionValueRepresentation::Bool {
            return ValueCell::from_exact(false);
        }
        return Err(MechError::new(
            FunctionArgumentTypeMismatch {
                role: FunctionArgumentRole::Output,
                expected: "Boolean scalar or exact Boolean matrix output".into(),
                found: format!("{representation:?}"),
            },
            None,
        )
        .with_compiler_loc());
    };

    let template = [first, second]
        .into_iter()
        .find(|input| {
            matches!(
                input.representation(),
                Some(FunctionValueRepresentation::Matrix {
                    storage: FunctionMatrixStoragePattern::Exact(found),
                    ..
                }) if found == storage
            )
        })
        .ok_or_else(|| {
            MechError::new(
                FunctionArgumentTypeMismatch {
                    role: FunctionArgumentRole::Output,
                    expected: format!("matrix template with {storage:?} storage"),
                    found: format!(
                        "inputs {:?} and {:?}",
                        first.representation(),
                        second.representation(),
                    ),
                },
                None,
            )
            .with_compiler_loc()
        })?;
    let descriptor = template.matrix_descriptor()?.ok_or_else(|| {
        MechError::new(
            FunctionArgumentTypeMismatch {
                role: FunctionArgumentRole::Output,
                expected: format!("matrix dimensions for {storage:?}"),
                found: format!("{:?}", template.representation()),
            },
            None,
        )
        .with_compiler_loc()
    })?;
    let (rows, columns) = (descriptor.rows, descriptor.cols);

    #[allow(
        unused_macros,
        reason = "fixed matrix constructors are feature-selected below"
    )]
    macro_rules! fixed {
        ($matrix:ident) => {
            ValueCell::from_exact_matrix_ref(
                Ref::new($matrix::<bool>::from_element(false)),
                rows,
                columns,
            )
        };
    }
    #[allow(
        unreachable_patterns,
        reason = "the fallback is reachable only in narrow matrix feature profiles"
    )]
    match storage {
        #[cfg(feature = "matrix1")]
        FunctionMatrixRepresentation::Matrix1 => fixed!(Matrix1),
        #[cfg(feature = "matrix2")]
        FunctionMatrixRepresentation::Matrix2 => fixed!(Matrix2),
        #[cfg(feature = "matrix3")]
        FunctionMatrixRepresentation::Matrix3 => fixed!(Matrix3),
        #[cfg(feature = "matrix4")]
        FunctionMatrixRepresentation::Matrix4 => fixed!(Matrix4),
        #[cfg(feature = "matrix2x3")]
        FunctionMatrixRepresentation::Matrix2x3 => fixed!(Matrix2x3),
        #[cfg(feature = "matrix3x2")]
        FunctionMatrixRepresentation::Matrix3x2 => fixed!(Matrix3x2),
        #[cfg(feature = "row_vector2")]
        FunctionMatrixRepresentation::RowVector2 => fixed!(RowVector2),
        #[cfg(feature = "row_vector3")]
        FunctionMatrixRepresentation::RowVector3 => fixed!(RowVector3),
        #[cfg(feature = "row_vector4")]
        FunctionMatrixRepresentation::RowVector4 => fixed!(RowVector4),
        #[cfg(feature = "vector2")]
        FunctionMatrixRepresentation::Vector2 => fixed!(Vector2),
        #[cfg(feature = "vector3")]
        FunctionMatrixRepresentation::Vector3 => fixed!(Vector3),
        #[cfg(feature = "vector4")]
        FunctionMatrixRepresentation::Vector4 => fixed!(Vector4),
        #[cfg(feature = "row_vectord")]
        FunctionMatrixRepresentation::RowVectorD => ValueCell::from_exact_matrix_ref(
            Ref::new(RowDVector::<bool>::from_element(columns, false)),
            rows,
            columns,
        ),
        #[cfg(feature = "vectord")]
        FunctionMatrixRepresentation::VectorD => ValueCell::from_exact_matrix_ref(
            Ref::new(DVector::<bool>::from_element(rows, false)),
            rows,
            columns,
        ),
        #[cfg(feature = "matrixd")]
        FunctionMatrixRepresentation::MatrixD => ValueCell::from_exact_matrix_ref(
            Ref::new(DMatrix::<bool>::from_element(rows, columns, false)),
            rows,
            columns,
        ),
        _ => Err(MechError::new(
            FunctionArgumentTypeMismatch {
                role: FunctionArgumentRole::Output,
                expected: "enabled exact Boolean matrix storage".into(),
                found: format!("{storage:?}"),
            },
            None,
        )
        .with_compiler_loc()),
    }
}

#[cfg(feature = "source")]
fn specialize_compare_binary_factory<F>(
    first: &SpecializationInput,
    second: &SpecializationInput,
    resolved_output: &ResolvedType,
) -> MResult<SpecializedFunction>
where
    F: MechFunctionFactory,
{
    let output = match F::SIGNATURE.output {
        FunctionValueRepresentation::Bool
        | FunctionValueRepresentation::Matrix {
            element: FunctionMatrixElement::Bool,
            ..
        } => compare_boolean_output(F::SIGNATURE.output, first, second)?,
        representation => [first, second]
            .into_iter()
            .find(|input| input.representation() == Some(representation))
            .ok_or_else(|| {
                MechError::new(
                    FunctionArgumentTypeMismatch {
                        role: FunctionArgumentRole::Output,
                        expected: format!("input template matching {representation:?}"),
                        found: format!(
                            "inputs {:?} and {:?}",
                            first.representation(),
                            second.representation(),
                        ),
                    },
                    None,
                )
                .with_compiler_loc()
            })?
            .cell()?
            .detached_clone()?,
    }
    .with_resolved_output_type(resolved_output)?;
    SpecializedFunction::bind_factory::<F>(
        output,
        vec![first.cell()?.clone(), second.cell()?.clone()].into_boxed_slice(),
    )
}

#[doc(hidden)]
#[macro_export]
macro_rules! __try_compare_binary_factory {
    (($module:ident, $first:ident, $second:ident, $context:ident), $lib:ident, $suffix:ident, $scalar:ty, $scalar_name:literal, $scalar_token:ident) => {
        mech_core::paste::paste! {
            if let RuntimeFunctionInputs::Binary(expected_first, expected_second) =
                <$crate::$module::[<$lib $suffix>]<$scalar> as MechFunctionFactory>::SIGNATURE.inputs
                && $first.representation() == Some(expected_first)
                && $second.representation() == Some(expected_second)
            {
                return $crate::specialize_compare_binary_factory::<
                    $crate::$module::[<$lib $suffix>]<$scalar>
                >($first, $second, $context.resolved_output(0)?);
            }
        }
    };
}

#[macro_export]
macro_rules! try_compare_binary_factories {
    ($module:ident, $first:ident, $second:ident, $context:ident, $lib:ident) => {{
        #[cfg(feature = "bool")]
        mech_core::for_each_canonical_binop_factory!($crate::__try_compare_binary_factory, ($module, $first, $second, $context), $lib, bool, "bool", bool);
        #[cfg(feature = "string")]
        mech_core::for_each_canonical_binop_factory!($crate::__try_compare_binary_factory, ($module, $first, $second, $context), $lib, String, "string", string);
        #[cfg(feature = "u8")]
        mech_core::for_each_canonical_binop_factory!($crate::__try_compare_binary_factory, ($module, $first, $second, $context), $lib, u8, "u8", u8);
        #[cfg(feature = "u16")]
        mech_core::for_each_canonical_binop_factory!($crate::__try_compare_binary_factory, ($module, $first, $second, $context), $lib, u16, "u16", u16);
        #[cfg(feature = "u32")]
        mech_core::for_each_canonical_binop_factory!($crate::__try_compare_binary_factory, ($module, $first, $second, $context), $lib, u32, "u32", u32);
        #[cfg(feature = "u64")]
        mech_core::for_each_canonical_binop_factory!($crate::__try_compare_binary_factory, ($module, $first, $second, $context), $lib, u64, "u64", u64);
        #[cfg(feature = "u128")]
        mech_core::for_each_canonical_binop_factory!($crate::__try_compare_binary_factory, ($module, $first, $second, $context), $lib, u128, "u128", u128);
        #[cfg(feature = "i8")]
        mech_core::for_each_canonical_binop_factory!($crate::__try_compare_binary_factory, ($module, $first, $second, $context), $lib, i8, "i8", i8);
        #[cfg(feature = "i16")]
        mech_core::for_each_canonical_binop_factory!($crate::__try_compare_binary_factory, ($module, $first, $second, $context), $lib, i16, "i16", i16);
        #[cfg(feature = "i32")]
        mech_core::for_each_canonical_binop_factory!($crate::__try_compare_binary_factory, ($module, $first, $second, $context), $lib, i32, "i32", i32);
        #[cfg(feature = "i64")]
        mech_core::for_each_canonical_binop_factory!($crate::__try_compare_binary_factory, ($module, $first, $second, $context), $lib, i64, "i64", i64);
        #[cfg(feature = "i128")]
        mech_core::for_each_canonical_binop_factory!($crate::__try_compare_binary_factory, ($module, $first, $second, $context), $lib, i128, "i128", i128);
        #[cfg(feature = "f32")]
        mech_core::for_each_canonical_binop_factory!($crate::__try_compare_binary_factory, ($module, $first, $second, $context), $lib, f32, "f32", f32);
        #[cfg(feature = "f64")]
        mech_core::for_each_canonical_binop_factory!($crate::__try_compare_binary_factory, ($module, $first, $second, $context), $lib, f64, "f64", f64);
        #[cfg(feature = "rational")]
        mech_core::for_each_canonical_binop_factory!($crate::__try_compare_binary_factory, ($module, $first, $second, $context), $lib, R64, "r64", r64);
        #[cfg(feature = "complex")]
        mech_core::for_each_canonical_binop_factory!($crate::__try_compare_binary_factory, ($module, $first, $second, $context), $lib, C64, "c64", c64);
    }};
}

#[macro_export]
macro_rules! try_numeric_compare_binary_factories {
    ($module:ident, $first:ident, $second:ident, $context:ident, $lib:ident) => {{
        #[cfg(feature = "u8")]
        mech_core::for_each_canonical_binop_factory!($crate::__try_compare_binary_factory, ($module, $first, $second, $context), $lib, u8, "u8", u8);
        #[cfg(feature = "u16")]
        mech_core::for_each_canonical_binop_factory!($crate::__try_compare_binary_factory, ($module, $first, $second, $context), $lib, u16, "u16", u16);
        #[cfg(feature = "u32")]
        mech_core::for_each_canonical_binop_factory!($crate::__try_compare_binary_factory, ($module, $first, $second, $context), $lib, u32, "u32", u32);
        #[cfg(feature = "u64")]
        mech_core::for_each_canonical_binop_factory!($crate::__try_compare_binary_factory, ($module, $first, $second, $context), $lib, u64, "u64", u64);
        #[cfg(feature = "u128")]
        mech_core::for_each_canonical_binop_factory!($crate::__try_compare_binary_factory, ($module, $first, $second, $context), $lib, u128, "u128", u128);
        #[cfg(feature = "i8")]
        mech_core::for_each_canonical_binop_factory!($crate::__try_compare_binary_factory, ($module, $first, $second, $context), $lib, i8, "i8", i8);
        #[cfg(feature = "i16")]
        mech_core::for_each_canonical_binop_factory!($crate::__try_compare_binary_factory, ($module, $first, $second, $context), $lib, i16, "i16", i16);
        #[cfg(feature = "i32")]
        mech_core::for_each_canonical_binop_factory!($crate::__try_compare_binary_factory, ($module, $first, $second, $context), $lib, i32, "i32", i32);
        #[cfg(feature = "i64")]
        mech_core::for_each_canonical_binop_factory!($crate::__try_compare_binary_factory, ($module, $first, $second, $context), $lib, i64, "i64", i64);
        #[cfg(feature = "i128")]
        mech_core::for_each_canonical_binop_factory!($crate::__try_compare_binary_factory, ($module, $first, $second, $context), $lib, i128, "i128", i128);
        #[cfg(feature = "f32")]
        mech_core::for_each_canonical_binop_factory!($crate::__try_compare_binary_factory, ($module, $first, $second, $context), $lib, f32, "f32", f32);
        #[cfg(feature = "f64")]
        mech_core::for_each_canonical_binop_factory!($crate::__try_compare_binary_factory, ($module, $first, $second, $context), $lib, f64, "f64", f64);
        #[cfg(feature = "rational")]
        mech_core::for_each_canonical_binop_factory!($crate::__try_compare_binary_factory, ($module, $first, $second, $context), $lib, R64, "r64", r64);
        #[cfg(feature = "complex")]
        mech_core::for_each_canonical_binop_factory!($crate::__try_compare_binary_factory, ($module, $first, $second, $context), $lib, C64, "c64", c64);
    }};
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
                $crate::try_numeric_compare_binary_factories!($module, first, second, context, $lib);
                Err(MechError::new(
                    FunctionArgumentTypeMismatch {
                        role: FunctionArgumentRole::Input(0),
                        expected: concat!("matching numeric inputs for ", $operation).into(),
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
                let name = format!("{}<{}>", stringify!($struct_name), <T as FunctionRuntimeType>::REPRESENTATION);
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
