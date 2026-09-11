use crate::intrinsics::constructors::ValueMatrixConcatenation;
use crate::intrinsics::*;
use std::marker::PhantomData;
use std::sync::LazyLock;

static PURE_VERTICAL_VARIADIC_BUILD_CONTRACT: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| OperationContractDeclaration {
        inputs: InputPortLayout::Variadic {
            prefix: Box::new([]),
            repeated: InputPortPolicy {
                access: AccessMode::Read,
                delivery: DeliveryMode::Signal,
            },
            min_repetitions: 1,
        },
        outputs: vec![OutputPortPolicy {
            access: AccessMode::Write,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::Build {
                postcondition: ShapeContractReference {
                    module_path: vec!["matrix".to_owned(), "concatenate".to_owned()]
                        .into_boxed_slice(),
                    contract_name: "vertical-output".to_owned(),
                },
            },
            alias: AliasPolicy::NoAlias,
            change_detection: ChangeDetectionPolicy::KernelReported,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    });

// Vertical Concatenate -----------------------------------------------------

#[cfg(any(
    all(feature = "matrix1", feature = "vector2"),
    all(feature = "vector2", feature = "vector4"),
    all(feature = "matrix1", feature = "vector3", feature = "vector4"),
    all(feature = "row_vector2", feature = "matrix2"),
    all(feature = "row_vector3", feature = "matrix2x3"),
    all(feature = "matrixd", feature = "matrix4")
))]
macro_rules! vertcat_two_args {
    ($fxn:ident, $e0:ident, $e1:ident, $out:ident) => {
        #[derive(Debug)]
        struct $fxn<T> {
            _marker: PhantomData<T>,
        }
        impl<T> MechFunctionFactory for $fxn<T>
        where
            T: Debug
                + Clone
                + Sync
                + Send
                + PartialEq
                + 'static
                + ConstElem
                + FunctionRuntimeType
                + FunctionRuntimeType
                + FunctionPortBacking,
            #[cfg(feature = "semantic-compiler")]
            T: CompileConst + CanonicalMatrixElementBacking,
            $e0<T>: FunctionRuntimeType,
            $e1<T>: FunctionRuntimeType,
            $out<T>: FunctionRuntimeType,
        {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
                <$out<T> as FunctionRuntimeType>::REPRESENTATION,
                <$e0<T> as FunctionRuntimeType>::REPRESENTATION,
                <$e1<T> as FunctionRuntimeType>::REPRESENTATION,
            );

            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                mech_core::ImplementationMemoryClass::CanonicalFinalize
            }

            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                invocation.expect_binary()?;
                crate::intrinsics::constructors::managed_legacy_matrix_concatenation::<true>(
                    invocation,
                )
            }
        }
    };
}

#[cfg(any(
    all(feature = "matrix1", feature = "vector3"),
    all(feature = "matrix1", feature = "vector2", feature = "vector4"),
    all(feature = "row_vector2", feature = "matrix3x2"),
    all(feature = "row_vector3", feature = "matrix3"),
    all(feature = "row_vector4", feature = "matrixd", feature = "matrix4")
))]
macro_rules! vertcat_three_args {
    ($fxn:ident, $e0:ident, $e1:ident, $e2:ident, $out:ident) => {
        #[derive(Debug)]
        struct $fxn<T> {
            _marker: PhantomData<T>,
        }
        impl<T> MechFunctionFactory for $fxn<T>
        where
            T: Debug
                + Clone
                + Sync
                + Send
                + PartialEq
                + 'static
                + ConstElem
                + FunctionRuntimeType
                + FunctionRuntimeType
                + FunctionPortBacking,
            #[cfg(feature = "semantic-compiler")]
            T: CompileConst + CanonicalMatrixElementBacking,
            $e0<T>: FunctionRuntimeType,
            $e1<T>: FunctionRuntimeType,
            $e2<T>: FunctionRuntimeType,
            $out<T>: FunctionRuntimeType,
        {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::ternary(
                <$out<T> as FunctionRuntimeType>::REPRESENTATION,
                <$e0<T> as FunctionRuntimeType>::REPRESENTATION,
                <$e1<T> as FunctionRuntimeType>::REPRESENTATION,
                <$e2<T> as FunctionRuntimeType>::REPRESENTATION,
            );

            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                mech_core::ImplementationMemoryClass::CanonicalFinalize
            }

            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                invocation.expect_ternary()?;
                crate::intrinsics::constructors::managed_legacy_matrix_concatenation::<true>(
                    invocation,
                )
            }
        }
    };
}

#[cfg(all(feature = "matrix4", feature = "row_vector4"))]
macro_rules! vertcat_four_args {
    ($fxn:ident, $e0:ident, $e1:ident, $e2:ident, $e3:ident, $out:ident) => {
        #[derive(Debug)]
        struct $fxn<T> {
            _marker: PhantomData<T>,
        }
        impl<T> MechFunctionFactory for $fxn<T>
        where
            T: Debug
                + Clone
                + Sync
                + Send
                + PartialEq
                + 'static
                + ConstElem
                + FunctionRuntimeType
                + FunctionRuntimeType
                + FunctionPortBacking,
            #[cfg(feature = "semantic-compiler")]
            T: CompileConst + CanonicalMatrixElementBacking,
            $e0<T>: FunctionRuntimeType,
            $e1<T>: FunctionRuntimeType,
            $e2<T>: FunctionRuntimeType,
            $e3<T>: FunctionRuntimeType,
            $out<T>: FunctionRuntimeType,
        {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::quaternary(
                <$out<T> as FunctionRuntimeType>::REPRESENTATION,
                <$e0<T> as FunctionRuntimeType>::REPRESENTATION,
                <$e1<T> as FunctionRuntimeType>::REPRESENTATION,
                <$e2<T> as FunctionRuntimeType>::REPRESENTATION,
                <$e3<T> as FunctionRuntimeType>::REPRESENTATION,
            );

            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                mech_core::ImplementationMemoryClass::CanonicalFinalize
            }

            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                invocation.expect_quaternary()?;
                crate::intrinsics::constructors::managed_legacy_matrix_concatenation::<true>(
                    invocation,
                )
            }
        }
    };
}

// VerticalConcatenateTwoArgs -------------------------------------------------

#[cfg(feature = "matrixd")]
struct VerticalConcatenateTwoArgs<T> {
    _marker: PhantomData<T>,
}
#[cfg(feature = "matrixd")]
impl<T> MechFunctionFactory for VerticalConcatenateTwoArgs<T>
where
    T: Debug
        + Clone
        + Sync
        + Send
        + PartialEq
        + 'static
        + ConstElem
        + FunctionRuntimeType
        + FunctionRuntimeType
        + FunctionPortBacking,
    #[cfg(feature = "semantic-compiler")]
    T: CompileConst + CanonicalMatrixElementBacking,
{
    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
        <DMatrix<T> as FunctionRuntimeType>::REPRESENTATION,
        <Matrix<T> as FunctionRuntimeType>::REPRESENTATION,
        <Matrix<T> as FunctionRuntimeType>::REPRESENTATION,
    );

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::CanonicalFinalize
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        invocation.expect_binary()?;
        crate::intrinsics::constructors::managed_legacy_matrix_concatenation::<true>(invocation)
    }

    fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_VERTICAL_VARIADIC_BUILD_CONTRACT)
    }
}

// VerticalConcatenateThreeArgs -----------------------------------------------

#[cfg(feature = "matrixd")]
struct VerticalConcatenateThreeArgs<T> {
    _marker: PhantomData<T>,
}
#[cfg(feature = "matrixd")]
impl<T> MechFunctionFactory for VerticalConcatenateThreeArgs<T>
where
    T: Debug
        + Clone
        + Sync
        + Send
        + PartialEq
        + 'static
        + ConstElem
        + FunctionRuntimeType
        + FunctionRuntimeType
        + FunctionPortBacking,
    #[cfg(feature = "semantic-compiler")]
    T: CompileConst + CanonicalMatrixElementBacking,
{
    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::ternary(
        <DMatrix<T> as FunctionRuntimeType>::REPRESENTATION,
        <Matrix<T> as FunctionRuntimeType>::REPRESENTATION,
        <Matrix<T> as FunctionRuntimeType>::REPRESENTATION,
        <Matrix<T> as FunctionRuntimeType>::REPRESENTATION,
    );

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::CanonicalFinalize
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        invocation.expect_ternary()?;
        crate::intrinsics::constructors::managed_legacy_matrix_concatenation::<true>(invocation)
    }

    fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_VERTICAL_VARIADIC_BUILD_CONTRACT)
    }
}

// VerticalConcatenateFourArgs ------------------------------------------------

#[cfg(feature = "matrixd")]
struct VerticalConcatenateFourArgs<T> {
    _marker: PhantomData<T>,
}
#[cfg(feature = "matrixd")]
impl<T> MechFunctionFactory for VerticalConcatenateFourArgs<T>
where
    T: Debug
        + Clone
        + Sync
        + Send
        + PartialEq
        + 'static
        + ConstElem
        + FunctionRuntimeType
        + FunctionRuntimeType
        + FunctionPortBacking,
    #[cfg(feature = "semantic-compiler")]
    T: CompileConst + CanonicalMatrixElementBacking,
{
    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::quaternary(
        <DMatrix<T> as FunctionRuntimeType>::REPRESENTATION,
        <Matrix<T> as FunctionRuntimeType>::REPRESENTATION,
        <Matrix<T> as FunctionRuntimeType>::REPRESENTATION,
        <Matrix<T> as FunctionRuntimeType>::REPRESENTATION,
        <Matrix<T> as FunctionRuntimeType>::REPRESENTATION,
    );

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::CanonicalFinalize
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        invocation.expect_quaternary()?;
        crate::intrinsics::constructors::managed_legacy_matrix_concatenation::<true>(invocation)
    }

    fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_VERTICAL_VARIADIC_BUILD_CONTRACT)
    }
}

// VerticalConcatenateNArgs ---------------------------------------------------

#[cfg(feature = "matrixd")]
struct VerticalConcatenateNArgs<T> {
    _marker: PhantomData<T>,
}
#[cfg(feature = "matrixd")]
impl<T> MechFunctionFactory for VerticalConcatenateNArgs<T>
where
    T: Debug
        + Clone
        + Sync
        + Send
        + PartialEq
        + 'static
        + ConstElem
        + FunctionRuntimeType
        + FunctionRuntimeType
        + FunctionPortBacking,
    #[cfg(feature = "semantic-compiler")]
    T: CompileConst + CanonicalMatrixElementBacking,
{
    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::variadic(
        <DMatrix<T> as FunctionRuntimeType>::REPRESENTATION,
        <Matrix<T> as FunctionRuntimeType>::REPRESENTATION,
    );

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::CanonicalFinalize
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        invocation.expect_variadic()?;
        crate::intrinsics::constructors::managed_legacy_matrix_concatenation::<true>(invocation)
    }

    fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_VERTICAL_VARIADIC_BUILD_CONTRACT)
    }
}

mech_core::declare_native_runtime_factory! {
    cfg: all(
        feature = "f64",
        feature = "matrix_vertcat",
        feature = "matrixd"
    ),

    registration: register_vertical_concatenate_n_args_f64,
    installer: install_vertical_concatenate_n_args_f64,

    name: "VerticalConcatenateNArgs<f64>",
    factory_type: VerticalConcatenateNArgs<f64>,
    contract: RuntimeFunctionContract::vertical_concatenation(RuntimeOutputAliasPolicy::DisallowInputAlias),
    compiler_family: mech_core::RuntimeFamilyId::from_name("VerticalConcatenateNArgs<f64>"),

    package: "mech-engine",
    crate_name: "mech_engine",
    installer_path: "mech_engine::__mech_native::install_vertical_concatenate_n_args_f64",

    extra_cargo_features: ["matrix_vertcat"],
}

#[cfg(all(test, feature = "compiler", feature = "matrixd", feature = "f64"))]
mod compiler_tests {
    use super::*;
    use crate::test_support::bytecode_compiler::RecordingBytecodeCompilerContext;

    #[test]
    fn managed_vertical_concatenation_reuses_repeated_matrix_register() {
        let matrix = ValueCell::from_exact(DMatrix::from_vec(1, 1, vec![7.0])).unwrap();
        let output = ValueCell::from_exact(DMatrix::from_element(2, 1, 0.0)).unwrap();
        let function =
            ValueMatrixConcatenation::<true>::new_invocation(FunctionInvocation::variadic(
                output,
                vec![matrix.clone(), matrix.clone()].into_boxed_slice(),
            ))
            .unwrap();
        let mut context = RecordingBytecodeCompilerContext::default();

        function.compile(&mut context).unwrap();

        let matrix_register = context.reg_map[&(matrix.reactive_cell_id().get() as usize)];
        assert_eq!(
            context
                .instructions
                .iter()
                .filter(|instruction| {
                    matches!(
                      instruction,
                      BytecodeInstruction::ConstLoad { dst, .. } if *dst == matrix_register
                    )
                })
                .count(),
            1,
        );
        assert!(matches!(
          context.instructions.last(),
          Some(BytecodeInstruction::RuntimeVariadic { arguments, .. })
            if arguments == &vec![matrix_register, matrix_register]
        ));
    }
}

// VerticalConcatenateVec -----------------------------------------------------

macro_rules! vertical_concatenate {
    ($name:ident, $vec_size:expr) => {
        paste! {
          #[derive(Debug)]
          struct $name<T> {
              _marker: PhantomData<T>,
          }
          impl<T> MechFunctionFactory for $name<T>
          where
            T: Debug + Clone + Sync + Send + PartialEq + 'static +
            ConstElem + FunctionRuntimeType
            + FunctionPortBacking,
            #[cfg(feature = "semantic-compiler")]
            T: CompileConst + CanonicalMatrixElementBacking,
            [<$vec_size>]<T>: FunctionRuntimeType,
          {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::unary(
              <[<$vec_size>]<T> as FunctionRuntimeType>::REPRESENTATION,
              FunctionValueRepresentation::AnyValue,
            );

            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                mech_core::ImplementationMemoryClass::CanonicalFinalize
            }

            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                invocation.expect_unary()?;
                crate::intrinsics::constructors::managed_legacy_matrix_concatenation::<true>(invocation)
            }

          }


        }
    };
}

// VerticalConcatenateVD2 -----------------------------------------------------

#[cfg(feature = "vectord")]
struct VerticalConcatenateVD2<T> {
    _marker: PhantomData<T>,
}
#[cfg(feature = "vectord")]
impl<T> MechFunctionFactory for VerticalConcatenateVD2<T>
where
    T: Debug
        + Clone
        + Sync
        + Send
        + PartialEq
        + 'static
        + ConstElem
        + FunctionRuntimeType
        + FunctionRuntimeType
        + FunctionPortBacking,
    #[cfg(feature = "semantic-compiler")]
    T: CompileConst + CanonicalMatrixElementBacking,
{
    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
        <DVector<T> as FunctionRuntimeType>::REPRESENTATION,
        <Matrix<T> as FunctionRuntimeType>::REPRESENTATION,
        <Matrix<T> as FunctionRuntimeType>::REPRESENTATION,
    );

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::CanonicalFinalize
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        invocation.expect_binary()?;
        crate::intrinsics::constructors::managed_legacy_matrix_concatenation::<true>(invocation)
    }

    fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_VERTICAL_VARIADIC_BUILD_CONTRACT)
    }
}

// VerticalConcatenateVD3 -----------------------------------------------------

#[cfg(feature = "vectord")]
struct VerticalConcatenateVD3<T> {
    _marker: PhantomData<T>,
}
#[cfg(feature = "vectord")]
impl<T> MechFunctionFactory for VerticalConcatenateVD3<T>
where
    T: Debug
        + Clone
        + Sync
        + Send
        + PartialEq
        + 'static
        + ConstElem
        + FunctionRuntimeType
        + FunctionRuntimeType
        + FunctionPortBacking,
    #[cfg(feature = "semantic-compiler")]
    T: CompileConst + CanonicalMatrixElementBacking,
{
    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::ternary(
        <DVector<T> as FunctionRuntimeType>::REPRESENTATION,
        <Matrix<T> as FunctionRuntimeType>::REPRESENTATION,
        <Matrix<T> as FunctionRuntimeType>::REPRESENTATION,
        <Matrix<T> as FunctionRuntimeType>::REPRESENTATION,
    );

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::CanonicalFinalize
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        invocation.expect_ternary()?;
        crate::intrinsics::constructors::managed_legacy_matrix_concatenation::<true>(invocation)
    }

    fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_VERTICAL_VARIADIC_BUILD_CONTRACT)
    }
}

// VerticalConcatenateVD4 -----------------------------------------------------

#[cfg(feature = "vectord")]
struct VerticalConcatenateVD4<T> {
    _marker: PhantomData<T>,
}
#[cfg(feature = "vectord")]
impl<T> MechFunctionFactory for VerticalConcatenateVD4<T>
where
    T: Debug
        + Clone
        + Sync
        + Send
        + PartialEq
        + 'static
        + ConstElem
        + FunctionRuntimeType
        + FunctionRuntimeType
        + FunctionPortBacking,
    #[cfg(feature = "semantic-compiler")]
    T: CompileConst + CanonicalMatrixElementBacking,
{
    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::quaternary(
        <DVector<T> as FunctionRuntimeType>::REPRESENTATION,
        <Matrix<T> as FunctionRuntimeType>::REPRESENTATION,
        <Matrix<T> as FunctionRuntimeType>::REPRESENTATION,
        <Matrix<T> as FunctionRuntimeType>::REPRESENTATION,
        <Matrix<T> as FunctionRuntimeType>::REPRESENTATION,
    );

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::CanonicalFinalize
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        invocation.expect_quaternary()?;
        crate::intrinsics::constructors::managed_legacy_matrix_concatenation::<true>(invocation)
    }

    fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_VERTICAL_VARIADIC_BUILD_CONTRACT)
    }
}

// VerticalConcatenateVDN -----------------------------------------------------

#[cfg(feature = "vectord")]
struct VerticalConcatenateVDN<T> {
    _marker: PhantomData<T>,
}
#[cfg(feature = "vectord")]
impl<T> MechFunctionFactory for VerticalConcatenateVDN<T>
where
    T: Debug
        + Clone
        + Sync
        + Send
        + PartialEq
        + 'static
        + ConstElem
        + FunctionRuntimeType
        + FunctionRuntimeType
        + FunctionPortBacking,
    #[cfg(feature = "semantic-compiler")]
    T: CompileConst + CanonicalMatrixElementBacking,
{
    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::variadic(
        <DVector<T> as FunctionRuntimeType>::REPRESENTATION,
        FunctionValueRepresentation::AnyValue,
    );

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::CanonicalFinalize
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        invocation.expect_variadic()?;
        crate::intrinsics::constructors::managed_legacy_matrix_concatenation::<true>(invocation)
    }

    fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_VERTICAL_VARIADIC_BUILD_CONTRACT)
    }
}

// VerticalConcatenateS1 ------------------------------------------------------

#[cfg(feature = "matrix1")]
#[derive(Debug)]
struct VerticalConcatenateS1<T> {
    _marker: PhantomData<T>,
}
#[cfg(feature = "matrix1")]
impl<T> MechFunctionFactory for VerticalConcatenateS1<T>
where
    T: Debug
        + Clone
        + Sync
        + Send
        + PartialEq
        + 'static
        + ConstElem
        + FunctionRuntimeType
        + FunctionRuntimeType
        + FunctionPortBacking,
    #[cfg(feature = "semantic-compiler")]
    T: CompileConst + CanonicalMatrixElementBacking,
{
    const SIGNATURE: RuntimeFunctionSignature =
        RuntimeFunctionSignature::nullary(<Matrix1<T> as FunctionRuntimeType>::REPRESENTATION);

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::CanonicalFinalize
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        invocation.expect_nullary()?;
        crate::intrinsics::constructors::managed_legacy_matrix_concatenation::<true>(invocation)
    }

    fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_VERTICAL_VARIADIC_BUILD_CONTRACT)
    }
}

// VerticalConcatenateMD ------------------------------------------------------

#[cfg(feature = "matrixd")]
vertical_concatenate!(VerticalConcatenateMD, DMatrix);

// VerticalConcatenateVD ------------------------------------------------------

#[cfg(feature = "vectord")]
vertical_concatenate!(VerticalConcatenateVD, DVector);

// VerticalConcatenateSD ------------------------------------------------------

#[cfg(feature = "vectord")]
#[derive(Debug)]
struct VerticalConcatenateSD<T> {
    _marker: PhantomData<T>,
}
#[cfg(feature = "vectord")]
impl<T> MechFunctionFactory for VerticalConcatenateSD<T>
where
    T: Debug
        + Clone
        + Sync
        + Send
        + PartialEq
        + 'static
        + ConstElem
        + FunctionRuntimeType
        + FunctionRuntimeType
        + FunctionPortBacking,
    #[cfg(feature = "semantic-compiler")]
    T: CompileConst + CanonicalMatrixElementBacking,
{
    const SIGNATURE: RuntimeFunctionSignature =
        RuntimeFunctionSignature::nullary(<DVector<T> as FunctionRuntimeType>::REPRESENTATION);

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::CanonicalFinalize
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        invocation.expect_nullary()?;
        crate::intrinsics::constructors::managed_legacy_matrix_concatenation::<true>(invocation)
    }
}

// VerticalConcatenateM1M1 ----------------------------------------------------

#[cfg(all(feature = "matrix1", feature = "vector2"))]
vertcat_two_args!(VerticalConcatenateM1M1, Matrix1, Matrix1, Vector2);

// VerticalConcatenateV2V2 ----------------------------------------------------

#[cfg(all(feature = "vector2", feature = "vector4"))]
vertcat_two_args!(VerticalConcatenateV2V2, Vector2, Vector2, Vector4);

// VerticalConcatenateM1V3 ----------------------------------------------------

#[cfg(all(feature = "matrix1", feature = "vector3", feature = "vector4"))]
vertcat_two_args!(VerticalConcatenateM1V3, Matrix1, Vector3, Vector4);

// VerticalConcatenateV3M1 ----------------------------------------------------

#[cfg(all(feature = "vector3", feature = "matrix1", feature = "vector4"))]
vertcat_two_args!(VerticalConcatenateV3M1, Vector3, Matrix1, Vector4);

// VerticalConcatenateM1V2 ----------------------------------------------------

#[cfg(all(feature = "matrix1", feature = "vector2", feature = "vector3"))]
vertcat_two_args!(VerticalConcatenateM1V2, Matrix1, Vector2, Vector3);

// VerticalConcatenateV2M1 ----------------------------------------------------

#[cfg(all(feature = "vector2", feature = "matrix1", feature = "vector3"))]
vertcat_two_args!(VerticalConcatenateV2M1, Vector2, Matrix1, Vector3);

// VerticalConcatenateM1M1M1 --------------------------------------------------

#[cfg(all(feature = "matrix1", feature = "vector3"))]
vertcat_three_args!(
    VerticalConcatenateM1M1M1,
    Matrix1,
    Matrix1,
    Matrix1,
    Vector3
);

// VerticalConcatenateM1M1V2 --------------------------------------------------

#[cfg(all(feature = "matrix1", feature = "vector2", feature = "vector4"))]
vertcat_three_args!(
    VerticalConcatenateM1M1V2,
    Matrix1,
    Matrix1,
    Vector2,
    Vector4
);

// VerticalConcatenateM1V2M1 --------------------------------------------------

#[cfg(all(feature = "matrix1", feature = "vector2", feature = "vector4"))]
vertcat_three_args!(
    VerticalConcatenateM1V2M1,
    Matrix1,
    Vector2,
    Matrix1,
    Vector4
);

// VerticalConcatenateV2M1M1 --------------------------------------------------

#[cfg(all(feature = "vector2", feature = "matrix1", feature = "vector4"))]
vertcat_three_args!(
    VerticalConcatenateV2M1M1,
    Vector2,
    Matrix1,
    Matrix1,
    Vector4
);

// VerticalConcatenateM1M1M1M1 ------------------------------------------------

#[cfg(all(feature = "matrix1", feature = "vector4"))]
#[derive(Debug)]
struct VerticalConcatenateM1M1M1M1<T> {
    _marker: PhantomData<T>,
}
#[cfg(all(feature = "matrix1", feature = "vector4"))]
impl<T> MechFunctionFactory for VerticalConcatenateM1M1M1M1<T>
where
    T: Debug
        + Clone
        + Sync
        + Send
        + PartialEq
        + 'static
        + ConstElem
        + FunctionRuntimeType
        + FunctionRuntimeType
        + FunctionPortBacking,
    #[cfg(feature = "semantic-compiler")]
    T: CompileConst + CanonicalMatrixElementBacking,
{
    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::quaternary(
        <Vector4<T> as FunctionRuntimeType>::REPRESENTATION,
        <Matrix1<T> as FunctionRuntimeType>::REPRESENTATION,
        <Matrix1<T> as FunctionRuntimeType>::REPRESENTATION,
        <Matrix1<T> as FunctionRuntimeType>::REPRESENTATION,
        <Matrix1<T> as FunctionRuntimeType>::REPRESENTATION,
    );

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::CanonicalFinalize
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        invocation.expect_quaternary()?;
        crate::intrinsics::constructors::managed_legacy_matrix_concatenation::<true>(invocation)
    }
}

// Mixed Type Vertical Concatenations -----------------------------------------

#[cfg(all(feature = "row_vector2", feature = "matrix2"))]
vertcat_two_args!(VerticalConcatenateR2R2, RowVector2, RowVector2, Matrix2);

mech_core::declare_native_runtime_factory! {
    cfg: all(
        feature = "f64",
        feature = "matrix2",
        feature = "matrix_vertcat",
        feature = "row_vector2"
    ),

    registration: register_vertical_concatenate_r2_r2_f64,
    installer: install_vertical_concatenate_r2_r2_f64,

    name: "VerticalConcatenateR2R2<f64Matrix2RowVector2RowVector2>",
    factory_type: VerticalConcatenateR2R2<f64>,
    contract: RuntimeFunctionContract::vertical_concatenation(RuntimeOutputAliasPolicy::DisallowInputAlias),
    compiler_family: mech_core::RuntimeFamilyId::from_name("VerticalConcatenateR2R2<f64Matrix2RowVector2RowVector2>"),

    package: "mech-engine",
    crate_name: "mech_engine",
    installer_path: "mech_engine::__mech_native::install_vertical_concatenate_r2_r2_f64",

    extra_cargo_features: ["matrix_vertcat"],
}

// VerticalConcatenateR3R3 ----------------------------------------------------

#[cfg(all(feature = "row_vector3", feature = "matrix2x3"))]
vertcat_two_args!(VerticalConcatenateR3R3, RowVector3, RowVector3, Matrix2x3);

// VerticalConcatenateR2M2 ----------------------------------------------------

#[cfg(all(feature = "row_vector2", feature = "matrix2", feature = "matrix3x2"))]
vertcat_two_args!(VerticalConcatenateR2M2, RowVector2, Matrix2, Matrix3x2);

// VerticalConcatenateM2R2 ----------------------------------------------------

#[cfg(all(feature = "matrix2", feature = "row_vector2", feature = "matrix3x2"))]
vertcat_two_args!(VerticalConcatenateM2R2, Matrix2, RowVector2, Matrix3x2);

// VerticalConcatenateM2x3R3 --------------------------------------------------

#[cfg(all(feature = "matrix2x3", feature = "row_vector3", feature = "matrix3"))]
vertcat_two_args!(VerticalConcatenateM2x3R3, Matrix2x3, RowVector3, Matrix3);

// VerticalConcatenateR3M2x3 --------------------------------------------------

#[cfg(all(feature = "row_vector3", feature = "matrix2x3", feature = "matrix3"))]
vertcat_two_args!(VerticalConcatenateR3M2x3, RowVector3, Matrix2x3, Matrix3);

// VerticalConcatenateMDR4 ----------------------------------------------------

#[cfg(all(feature = "matrixd", feature = "row_vector4", feature = "matrix4"))]
vertcat_two_args!(VerticalConcatenateMDR4, DMatrix, RowVector4, Matrix4);

// VerticalConcatenateMDMD ----------------------------------------------------

#[cfg(all(feature = "matrixd", feature = "matrix4"))]
vertcat_two_args!(VerticalConcatenateMDMD, DMatrix, DMatrix, Matrix4);

// VerticalConcatenateR4MD ----------------------------------------------------

#[cfg(all(feature = "matrixd", feature = "matrix4", feature = "row_vector4"))]
vertcat_two_args!(VerticalConcatenateR4MD, RowVector4, DMatrix, Matrix4);

// VerticalConcatenateR2R2R2 ----------------------------------------------------

#[cfg(all(feature = "row_vector2", feature = "matrix3x2"))]
vertcat_three_args!(
    VerticalConcatenateR2R2R2,
    RowVector2,
    RowVector2,
    RowVector2,
    Matrix3x2
);

// VerticalConcatenateR3R3R3 --------------------------------------------------

#[cfg(all(feature = "row_vector3", feature = "matrix3"))]
vertcat_three_args!(
    VerticalConcatenateR3R3R3,
    RowVector3,
    RowVector3,
    RowVector3,
    Matrix3
);

// VerticalConcatenateR4R4MD --------------------------------------------------

#[cfg(all(feature = "row_vector4", feature = "matrixd", feature = "matrix4"))]
vertcat_three_args!(
    VerticalConcatenateR4R4MD,
    RowVector4,
    RowVector4,
    DMatrix,
    Matrix4
);

// VerticalConcatenateR4MDR4 --------------------------------------------------

#[cfg(all(
    feature = "row_vector4",
    feature = "matrixd",
    feature = "row_vector4",
    feature = "matrix4"
))]
vertcat_three_args!(
    VerticalConcatenateR4MDR4,
    RowVector4,
    DMatrix,
    RowVector4,
    Matrix4
);

// VerticalConcatenateMDR4R4 --------------------------------------------------

#[cfg(all(
    feature = "matrixd",
    feature = "row_vector4",
    feature = "row_vector4",
    feature = "matrix4"
))]
vertcat_three_args!(
    VerticalConcatenateMDR4R4,
    DMatrix,
    RowVector4,
    RowVector4,
    Matrix4
);

// VerticalConcatenateR4R4R4R4 ------------------------------------------------

#[cfg(all(feature = "matrix4", feature = "row_vector4"))]
vertcat_four_args!(
    VerticalConcatenateR4R4R4R4,
    RowVector4,
    RowVector4,
    RowVector4,
    RowVector4,
    Matrix4
);

macro_rules! for_each_vertcat_scalar {
    ($callback:ident, ($($context:tt)*)) => {
        #[cfg(feature = "bool")] $callback!($($context)*; bool, bool, "bool", "bool");
        #[cfg(feature = "string")] $callback!($($context)*; string, String, "string", "string");
        #[cfg(feature = "u8")] $callback!($($context)*; u8, u8, "u8", "u8");
        #[cfg(feature = "u16")] $callback!($($context)*; u16, u16, "u16", "u16");
        #[cfg(feature = "u32")] $callback!($($context)*; u32, u32, "u32", "u32");
        #[cfg(feature = "u64")] $callback!($($context)*; u64, u64, "u64", "u64");
        #[cfg(feature = "u128")] $callback!($($context)*; u128, u128, "u128", "u128");
        #[cfg(feature = "i8")] $callback!($($context)*; i8, i8, "i8", "i8");
        #[cfg(feature = "i16")] $callback!($($context)*; i16, i16, "i16", "i16");
        #[cfg(feature = "i32")] $callback!($($context)*; i32, i32, "i32", "i32");
        #[cfg(feature = "i64")] $callback!($($context)*; i64, i64, "i64", "i64");
        #[cfg(feature = "i128")] $callback!($($context)*; i128, i128, "i128", "i128");
        #[cfg(feature = "f32")] $callback!($($context)*; f32, f32, "f32", "f32");
        #[cfg(feature = "f64")] $callback!($($context)*; f64, f64, "f64", "f64");
        #[cfg(feature = "c64")] $callback!($($context)*; c64, C64, "c64", "c64");
        #[cfg(feature = "r64")] $callback!($($context)*; r64, R64, "r64", "r64");
    };
}

macro_rules! declare_vertcat_scalar {
    ($factory:ident, [$($feature:literal),+]; $token:ident, $scalar:ty, $name:literal, $cargo:literal) => {
        paste! { mech_core::declare_native_runtime_factory! {
            cfg: all(feature = "matrix_vertcat", feature = $cargo, $(feature = $feature),+),
            registration: [<register_ $factory:snake _ $token>],
            installer: [<install_ $factory:snake _ $token>],
            name: concat!(stringify!($factory), "<", $name, ">"),
            factory_type: $factory<$scalar>,
            contract: RuntimeFunctionContract::vertical_concatenation(RuntimeOutputAliasPolicy::DisallowInputAlias),
            compiler_family: mech_core::RuntimeFamilyId::from_name(concat!(stringify!($factory), "<", $name, ">")),
            package: "mech-engine", crate_name: "mech_engine",
            installer_path: concat!("mech_engine::__mech_native::install_", stringify!([<$factory:snake _ $token>])),
            extra_cargo_features: ["matrix_vertcat"],
        }}
    };
}

macro_rules! declare_vertcat_family {
    ($factory:ident, [$($feature:literal),+]) => {
        for_each_vertcat_scalar!(declare_vertcat_scalar, ($factory, [$($feature),+]));
    };
}

macro_rules! declare_vertcat_scalar_except_f64 {
    ($factory:ident, [$($feature:literal),+]; f64, $_scalar:ty, $_name:literal, $_cargo:literal) => {};
    ($factory:ident, [$($feature:literal),+]; $token:ident, $scalar:ty, $name:literal, $cargo:literal) => {
        declare_vertcat_scalar!($factory, [$($feature),+]; $token, $scalar, $name, $cargo);
    };
}

macro_rules! declare_vertcat_family_except_f64 {
    ($factory:ident, [$($feature:literal),+]) => {
        for_each_vertcat_scalar!(declare_vertcat_scalar_except_f64, ($factory, [$($feature),+]));
    };
}

macro_rules! register_vertcat_scalar {
    ($builder:ident, $factory:ident; $token:ident, $_scalar:ty, $_name:literal, $_cargo:literal) => {
        paste! { [<register_ $factory:snake _ $token>]($builder)?; }
    };
}

macro_rules! install_vertcat_linked_factories {
    ($builder:ident, $factory:ident) => {{
        for_each_vertcat_scalar!(register_vertcat_scalar, ($builder, $factory));
        Ok::<(), MechError>(())
    }};
}

macro_rules! register_vertcat_scalar_except_f64 {
    ($builder:ident, $factory:ident; f64, $_scalar:ty, $_name:literal, $_cargo:literal) => {};
    ($builder:ident, $factory:ident; $token:ident, $_scalar:ty, $_name:literal, $_cargo:literal) => {
        paste! { [<register_ $factory:snake _ $token>]($builder)?; }
    };
}

macro_rules! install_vertcat_linked_factories_except_f64 {
    ($builder:ident, $factory:ident) => {{
        for_each_vertcat_scalar!(register_vertcat_scalar_except_f64, ($builder, $factory));
        Ok::<(), MechError>(())
    }};
}

declare_vertcat_family!(VerticalConcatenateMD, ["matrixd"]);
declare_vertcat_family!(VerticalConcatenateTwoArgs, ["matrixd"]);
declare_vertcat_family!(VerticalConcatenateThreeArgs, ["matrixd"]);
declare_vertcat_family!(VerticalConcatenateFourArgs, ["matrixd"]);
declare_vertcat_family_except_f64!(VerticalConcatenateNArgs, ["matrixd"]);
declare_vertcat_family!(VerticalConcatenateVD, ["vectord"]);
declare_vertcat_family!(VerticalConcatenateVD2, ["vectord"]);
declare_vertcat_family!(VerticalConcatenateVD3, ["vectord"]);
declare_vertcat_family!(VerticalConcatenateVD4, ["vectord"]);
declare_vertcat_family!(VerticalConcatenateVDN, ["vectord"]);
declare_vertcat_family!(VerticalConcatenateSD, ["vectord"]);

// Fixed-shape families share the same scalar traversal as their runtime
// registration and generated-application exports. Keep the exact storage
// requirements beside the family so all three consumers stay in lockstep.
macro_rules! for_each_vertcat_typed_family {
    ($callback:ident, ($($context:tt)*)) => {
        #[cfg(feature = "matrix1")] $callback!($($context)*; VerticalConcatenateS1; ["matrix1"]);
        #[cfg(all(feature = "matrix1", feature = "vector3"))] $callback!($($context)*; VerticalConcatenateM1M1M1; ["matrix1", "vector3"]);
        #[cfg(all(feature = "matrix1", feature = "vector2", feature = "vector4"))] $callback!($($context)*; VerticalConcatenateM1M1V2; ["matrix1", "vector2", "vector4"]);
        #[cfg(all(feature = "matrix1", feature = "vector2", feature = "vector4"))] $callback!($($context)*; VerticalConcatenateM1V2M1; ["matrix1", "vector2", "vector4"]);
        #[cfg(all(feature = "matrix1", feature = "vector2", feature = "vector4"))] $callback!($($context)*; VerticalConcatenateV2M1M1; ["matrix1", "vector2", "vector4"]);
        #[cfg(all(feature = "matrix1", feature = "vector4"))] $callback!($($context)*; VerticalConcatenateM1M1M1M1; ["matrix1", "vector4"]);
        #[cfg(all(feature = "row_vector2", feature = "matrix3x2"))] $callback!($($context)*; VerticalConcatenateR2R2R2; ["row_vector2", "matrix3x2"]);
        #[cfg(all(feature = "row_vector3", feature = "matrix3"))] $callback!($($context)*; VerticalConcatenateR3R3R3; ["row_vector3", "matrix3"]);
        #[cfg(all(feature = "row_vector4", feature = "matrixd", feature = "matrix4"))] $callback!($($context)*; VerticalConcatenateR4R4MD; ["row_vector4", "matrixd", "matrix4"]);
        #[cfg(all(feature = "row_vector4", feature = "matrixd", feature = "matrix4"))] $callback!($($context)*; VerticalConcatenateR4MDR4; ["row_vector4", "matrixd", "matrix4"]);
        #[cfg(all(feature = "row_vector4", feature = "matrixd", feature = "matrix4"))] $callback!($($context)*; VerticalConcatenateMDR4R4; ["row_vector4", "matrixd", "matrix4"]);
        #[cfg(all(feature = "matrix4", feature = "row_vector4"))] $callback!($($context)*; VerticalConcatenateR4R4R4R4; ["matrix4", "row_vector4"]);
    };
}

#[cfg(any(
    feature = "matrix1",
    all(feature = "row_vector2", feature = "matrix3x2"),
    all(feature = "row_vector3", feature = "matrix3"),
    all(feature = "matrix4", feature = "row_vector4")
))]
macro_rules! declare_vertcat_typed_family {
    (; $factory:ident; [$($feature:literal),+]) => {
        declare_vertcat_family!($factory, [$($feature),+]);
    };
}

for_each_vertcat_typed_family!(declare_vertcat_typed_family, ());

macro_rules! for_each_vertcat_binary_family {
    ($callback:ident, ($($context:tt)*)) => {
        #[cfg(all(feature = "matrix1", feature = "vector2"))] $callback!($($context)*; normal; VerticalConcatenateM1M1, Matrix1, Matrix1, Vector2; ["matrix1", "vector2"]);
        #[cfg(all(feature = "vector2", feature = "vector4"))] $callback!($($context)*; normal; VerticalConcatenateV2V2, Vector2, Vector2, Vector4; ["vector2", "vector4"]);
        #[cfg(all(feature = "matrix1", feature = "vector3", feature = "vector4"))] $callback!($($context)*; normal; VerticalConcatenateM1V3, Matrix1, Vector3, Vector4; ["matrix1", "vector3", "vector4"]);
        #[cfg(all(feature = "vector3", feature = "matrix1", feature = "vector4"))] $callback!($($context)*; normal; VerticalConcatenateV3M1, Vector3, Matrix1, Vector4; ["matrix1", "vector3", "vector4"]);
        #[cfg(all(feature = "matrix1", feature = "vector2", feature = "vector3"))] $callback!($($context)*; normal; VerticalConcatenateM1V2, Matrix1, Vector2, Vector3; ["matrix1", "vector2", "vector3"]);
        #[cfg(all(feature = "vector2", feature = "matrix1", feature = "vector3"))] $callback!($($context)*; normal; VerticalConcatenateV2M1, Vector2, Matrix1, Vector3; ["matrix1", "vector2", "vector3"]);
        #[cfg(all(feature = "row_vector2", feature = "matrix2"))] $callback!($($context)*; except_f64; VerticalConcatenateR2R2, RowVector2, RowVector2, Matrix2; ["row_vector2", "matrix2"]);
        #[cfg(all(feature = "row_vector3", feature = "matrix2x3"))] $callback!($($context)*; normal; VerticalConcatenateR3R3, RowVector3, RowVector3, Matrix2x3; ["row_vector3", "matrix2x3"]);
        #[cfg(all(feature = "row_vector2", feature = "matrix2", feature = "matrix3x2"))] $callback!($($context)*; normal; VerticalConcatenateR2M2, RowVector2, Matrix2, Matrix3x2; ["row_vector2", "matrix2", "matrix3x2"]);
        #[cfg(all(feature = "matrix2", feature = "row_vector2", feature = "matrix3x2"))] $callback!($($context)*; normal; VerticalConcatenateM2R2, Matrix2, RowVector2, Matrix3x2; ["matrix2", "row_vector2", "matrix3x2"]);
        #[cfg(all(feature = "matrix2x3", feature = "row_vector3", feature = "matrix3"))] $callback!($($context)*; normal; VerticalConcatenateM2x3R3, Matrix2x3, RowVector3, Matrix3; ["matrix2x3", "row_vector3", "matrix3"]);
        #[cfg(all(feature = "row_vector3", feature = "matrix2x3", feature = "matrix3"))] $callback!($($context)*; normal; VerticalConcatenateR3M2x3, RowVector3, Matrix2x3, Matrix3; ["row_vector3", "matrix2x3", "matrix3"]);
        #[cfg(all(feature = "matrixd", feature = "row_vector4", feature = "matrix4"))] $callback!($($context)*; normal; VerticalConcatenateMDR4, DMatrix, RowVector4, Matrix4; ["matrixd", "row_vector4", "matrix4"]);
        #[cfg(all(feature = "matrixd", feature = "matrix4"))] $callback!($($context)*; normal; VerticalConcatenateMDMD, DMatrix, DMatrix, Matrix4; ["matrixd", "matrix4"]);
        #[cfg(all(feature = "matrixd", feature = "matrix4", feature = "row_vector4"))] $callback!($($context)*; normal; VerticalConcatenateR4MD, RowVector4, DMatrix, Matrix4; ["row_vector4", "matrixd", "matrix4"]);
    };
}

#[cfg(any(
    all(feature = "matrix1", feature = "vector2"),
    all(feature = "vector2", feature = "vector4"),
    all(feature = "matrix1", feature = "vector3", feature = "vector4"),
    all(feature = "row_vector2", feature = "matrix2"),
    all(feature = "row_vector3", feature = "matrix2x3"),
    all(feature = "matrixd", feature = "matrix4")
))]
macro_rules! declare_vertcat_binary_scalar {
    ($factory:ident, $e0:ident, $e1:ident, $out:ident, [$($feature:literal),+]; $token:ident, $scalar:ty, $name:literal, $cargo:literal) => {
        paste! { mech_core::declare_native_runtime_factory! {
            cfg: all(feature = "matrix_vertcat", feature = $cargo, $(feature = $feature),+),
            registration: [<register_ $factory:snake _ $token _ $out:lower _ $e0:lower _ $e1:lower>],
            installer: [<install_ $factory:snake _ $token _ $out:lower _ $e0:lower _ $e1:lower>],
            name: concat!(stringify!($factory), "<", $name, stringify!($out), stringify!($e0), stringify!($e1), ">"),
            factory_type: $factory<$scalar>,
            contract: RuntimeFunctionContract::vertical_concatenation(RuntimeOutputAliasPolicy::DisallowInputAlias),
            compiler_family: mech_core::RuntimeFamilyId::from_name(concat!(stringify!($factory), "<", $name, stringify!($out), stringify!($e0), stringify!($e1), ">")),
            package: "mech-engine", crate_name: "mech_engine",
            installer_path: concat!("mech_engine::__mech_native::install_", stringify!([<$factory:snake _ $token _ $out:lower _ $e0:lower _ $e1:lower>])),
            extra_cargo_features: ["matrix_vertcat"],
        }}
    };
}

#[cfg(all(feature = "row_vector2", feature = "matrix2"))]
macro_rules! declare_vertcat_binary_scalar_except_f64 {
    ($factory:ident, $e0:ident, $e1:ident, $out:ident, [$($feature:literal),+]; f64, $_scalar:ty, $_name:literal, $_cargo:literal) => {};
    ($factory:ident, $e0:ident, $e1:ident, $out:ident, [$($feature:literal),+]; $token:ident, $scalar:ty, $name:literal, $cargo:literal) => {
        declare_vertcat_binary_scalar!($factory, $e0, $e1, $out, [$($feature),+]; $token, $scalar, $name, $cargo);
    };
}

#[cfg(any(
    all(feature = "matrix1", feature = "vector2"),
    all(feature = "vector2", feature = "vector4"),
    all(feature = "matrix1", feature = "vector3", feature = "vector4"),
    all(feature = "row_vector2", feature = "matrix2"),
    all(feature = "row_vector3", feature = "matrix2x3"),
    all(feature = "matrixd", feature = "matrix4")
))]
macro_rules! declare_vertcat_binary_family {
    (; normal; $factory:ident, $e0:ident, $e1:ident, $out:ident; [$($feature:literal),+]) => {
        for_each_vertcat_scalar!(declare_vertcat_binary_scalar, ($factory, $e0, $e1, $out, [$($feature),+]));
    };
    (; except_f64; $factory:ident, $e0:ident, $e1:ident, $out:ident; [$($feature:literal),+]) => {
        for_each_vertcat_scalar!(declare_vertcat_binary_scalar_except_f64, ($factory, $e0, $e1, $out, [$($feature),+]));
    };
}

for_each_vertcat_binary_family!(declare_vertcat_binary_family, ());

#[cfg(any(
    all(feature = "matrix1", feature = "vector2"),
    all(feature = "vector2", feature = "vector4"),
    all(feature = "matrix1", feature = "vector3", feature = "vector4"),
    all(feature = "row_vector2", feature = "matrix2"),
    all(feature = "row_vector3", feature = "matrix2x3"),
    all(feature = "matrixd", feature = "matrix4")
))]
macro_rules! register_vertcat_binary_scalar {
    ($builder:ident, $factory:ident, $e0:ident, $e1:ident, $out:ident; $token:ident, $_scalar:ty, $_name:literal, $_cargo:literal) => {
        paste! { [<register_ $factory:snake _ $token _ $out:lower _ $e0:lower _ $e1:lower>]($builder)?; }
    };
}

#[cfg(all(feature = "row_vector2", feature = "matrix2"))]
macro_rules! register_vertcat_binary_scalar_except_f64 {
    ($builder:ident, $_factory:ident, $_e0:ident, $_e1:ident, $_out:ident; f64, $_scalar:ty, $_name:literal, $_cargo:literal) => {};
    ($builder:ident, $factory:ident, $e0:ident, $e1:ident, $out:ident; $token:ident, $_scalar:ty, $_name:literal, $_cargo:literal) => {
        register_vertcat_binary_scalar!($builder, $factory, $e0, $e1, $out; $token, $_scalar, $_name, $_cargo);
    };
}

#[cfg(any(
    all(feature = "matrix1", feature = "vector2"),
    all(feature = "vector2", feature = "vector4"),
    all(feature = "matrix1", feature = "vector3", feature = "vector4"),
    all(feature = "row_vector2", feature = "matrix2", feature = "matrix3x2"),
    all(feature = "row_vector3", feature = "matrix2x3"),
    all(feature = "matrixd", feature = "matrix4")
))]
macro_rules! install_vertcat_binary_factories {
    ($builder:ident, $factory:ident, $e0:ident, $e1:ident, $out:ident) => {
        for_each_vertcat_scalar!(
            register_vertcat_binary_scalar,
            ($builder, $factory, $e0, $e1, $out)
        );
    };
}

#[cfg(all(feature = "row_vector2", feature = "matrix2"))]
macro_rules! install_vertcat_binary_factories_except_f64 {
    ($builder:ident, $factory:ident, $e0:ident, $e1:ident, $out:ident) => {
        for_each_vertcat_scalar!(
            register_vertcat_binary_scalar_except_f64,
            ($builder, $factory, $e0, $e1, $out)
        );
    };
}

#[cfg(any(
    all(feature = "matrix1", feature = "vector2"),
    all(feature = "vector2", feature = "vector4"),
    all(feature = "matrix1", feature = "vector3", feature = "vector4"),
    all(feature = "row_vector2", feature = "matrix2"),
    all(feature = "row_vector3", feature = "matrix2x3"),
    all(feature = "matrixd", feature = "matrix4")
))]
macro_rules! install_vertcat_binary_family {
    ($builder:ident; normal; $factory:ident, $e0:ident, $e1:ident, $out:ident; [$($_feature:literal),+]) => {
        install_vertcat_binary_factories!($builder, $factory, $e0, $e1, $out);
    };
    ($builder:ident; except_f64; $factory:ident, $e0:ident, $e1:ident, $out:ident; [$($_feature:literal),+]) => {
        #[cfg(feature = "f64")]
        paste! { [<register_ $factory:snake _f64>]($builder)?; }
        install_vertcat_binary_factories_except_f64!($builder, $factory, $e0, $e1, $out);
    };
}

#[cfg(any(
    feature = "matrix1",
    all(feature = "row_vector2", feature = "matrix3x2"),
    all(feature = "row_vector3", feature = "matrix3"),
    all(feature = "matrix4", feature = "row_vector4")
))]
macro_rules! install_vertcat_typed_family {
    ($builder:ident; $factory:ident; [$($_feature:literal),+]) => {
        install_vertcat_linked_factories!($builder, $factory)?;
    };
}

/// Installs every enabled legacy runtime factory emitted by this module.
pub(super) fn install_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    #[cfg(feature = "matrixd")]
    {
        install_vertcat_linked_factories!(builder, VerticalConcatenateMD)?;
        install_vertcat_linked_factories!(builder, VerticalConcatenateTwoArgs)?;
        install_vertcat_linked_factories!(builder, VerticalConcatenateThreeArgs)?;
        install_vertcat_linked_factories!(builder, VerticalConcatenateFourArgs)?;
        #[cfg(feature = "f64")]
        register_vertical_concatenate_n_args_f64(builder)?;
        install_vertcat_linked_factories_except_f64!(builder, VerticalConcatenateNArgs)?;
    }
    #[cfg(feature = "vectord")]
    {
        install_vertcat_linked_factories!(builder, VerticalConcatenateVD)?;
        install_vertcat_linked_factories!(builder, VerticalConcatenateVD2)?;
        install_vertcat_linked_factories!(builder, VerticalConcatenateVD3)?;
        install_vertcat_linked_factories!(builder, VerticalConcatenateVD4)?;
        install_vertcat_linked_factories!(builder, VerticalConcatenateVDN)?;
        install_vertcat_linked_factories!(builder, VerticalConcatenateSD)?;
    }
    for_each_vertcat_binary_family!(install_vertcat_binary_family, (builder));
    for_each_vertcat_typed_family!(install_vertcat_typed_family, (builder));

    Ok(())
}

#[cfg(feature = "native-link")]
macro_rules! export_vertcat_scalar {
    ($factory:ident, [$($feature:literal),+]; $token:ident, $_scalar:ty, $_name:literal, $cargo:literal) => {
        #[cfg(all(feature = "matrix_vertcat", feature = $cargo, $(feature = $feature),+))]
        mech_core::paste::paste! { pub use super::[<install_ $factory:snake _ $token>]; }
    };
}

#[cfg(feature = "native-link")]
macro_rules! export_vertcat_family {
    ($factory:ident, [$($feature:literal),+]) => {
        for_each_vertcat_scalar!(export_vertcat_scalar, ($factory, [$($feature),+]));
    };
}

#[cfg(feature = "native-link")]
macro_rules! export_vertcat_scalar_except_f64 {
    ($factory:ident, [$($feature:literal),+]; f64, $_scalar:ty, $_name:literal, $_cargo:literal) => {};
    ($factory:ident, [$($feature:literal),+]; $token:ident, $_scalar:ty, $_name:literal, $cargo:literal) => {
        #[cfg(all(feature = "matrix_vertcat", feature = $cargo, $(feature = $feature),+))]
        mech_core::paste::paste! { pub use super::[<install_ $factory:snake _ $token>]; }
    };
}

#[cfg(feature = "native-link")]
macro_rules! export_vertcat_family_except_f64 {
    ($factory:ident, [$($feature:literal),+]) => {
        for_each_vertcat_scalar!(export_vertcat_scalar_except_f64, ($factory, [$($feature),+]));
    };
}

#[cfg(all(
    feature = "native-link",
    any(
        feature = "matrix1",
        all(feature = "row_vector2", feature = "matrix3x2"),
        all(feature = "row_vector3", feature = "matrix3"),
        all(feature = "matrix4", feature = "row_vector4")
    )
))]
macro_rules! export_vertcat_typed_family {
    (; $factory:ident; [$($feature:literal),+]) => {
        export_vertcat_family!($factory, [$($feature),+]);
    };
}

#[cfg(all(
    feature = "native-link",
    any(
        all(feature = "matrix1", feature = "vector2"),
        all(feature = "vector2", feature = "vector4"),
        all(feature = "matrix1", feature = "vector3", feature = "vector4"),
        all(feature = "row_vector2", feature = "matrix2"),
        all(feature = "row_vector3", feature = "matrix2x3"),
        all(feature = "matrixd", feature = "matrix4")
    )
))]
macro_rules! export_vertcat_binary_scalar {
    ($factory:ident, $e0:ident, $e1:ident, $out:ident, [$($feature:literal),+]; $token:ident, $_scalar:ty, $_name:literal, $cargo:literal) => {
        #[cfg(all(feature = "matrix_vertcat", feature = $cargo, $(feature = $feature),+))]
        mech_core::paste::paste! { pub use super::[<install_ $factory:snake _ $token _ $out:lower _ $e0:lower _ $e1:lower>]; }
    };
}

#[cfg(all(feature = "native-link", feature = "row_vector2", feature = "matrix2"))]
macro_rules! export_vertcat_binary_scalar_except_f64 {
    ($factory:ident, $e0:ident, $e1:ident, $out:ident, [$($feature:literal),+]; f64, $_scalar:ty, $_name:literal, $_cargo:literal) => {};
    ($factory:ident, $e0:ident, $e1:ident, $out:ident, [$($feature:literal),+]; $token:ident, $scalar:ty, $name:literal, $cargo:literal) => {
        export_vertcat_binary_scalar!($factory, $e0, $e1, $out, [$($feature),+]; $token, $scalar, $name, $cargo);
    };
}

#[cfg(all(
    feature = "native-link",
    any(
        all(feature = "matrix1", feature = "vector2"),
        all(feature = "vector2", feature = "vector4"),
        all(feature = "matrix1", feature = "vector3", feature = "vector4"),
        all(feature = "row_vector2", feature = "matrix2"),
        all(feature = "row_vector3", feature = "matrix2x3"),
        all(feature = "matrixd", feature = "matrix4")
    )
))]
macro_rules! export_vertcat_binary_family {
    (; normal; $factory:ident, $e0:ident, $e1:ident, $out:ident; [$($feature:literal),+]) => {
        for_each_vertcat_scalar!(export_vertcat_binary_scalar, ($factory, $e0, $e1, $out, [$($feature),+]));
    };
    (; except_f64; $factory:ident, $e0:ident, $e1:ident, $out:ident; [$($feature:literal),+]) => {
        for_each_vertcat_scalar!(export_vertcat_binary_scalar_except_f64, ($factory, $e0, $e1, $out, [$($feature),+]));
    };
}

#[doc(hidden)]
#[cfg(feature = "native-link")]
pub mod __mech_native {
    export_vertcat_family!(VerticalConcatenateMD, ["matrixd"]);
    export_vertcat_family!(VerticalConcatenateTwoArgs, ["matrixd"]);
    export_vertcat_family!(VerticalConcatenateThreeArgs, ["matrixd"]);
    export_vertcat_family!(VerticalConcatenateFourArgs, ["matrixd"]);
    export_vertcat_family_except_f64!(VerticalConcatenateNArgs, ["matrixd"]);
    export_vertcat_family!(VerticalConcatenateVD, ["vectord"]);
    export_vertcat_family!(VerticalConcatenateVD2, ["vectord"]);
    export_vertcat_family!(VerticalConcatenateVD3, ["vectord"]);
    export_vertcat_family!(VerticalConcatenateVD4, ["vectord"]);
    export_vertcat_family!(VerticalConcatenateVDN, ["vectord"]);
    export_vertcat_family!(VerticalConcatenateSD, ["vectord"]);
    for_each_vertcat_typed_family!(export_vertcat_typed_family, ());
    for_each_vertcat_binary_family!(export_vertcat_binary_family, ());

    #[cfg(all(feature = "f64", feature = "matrixd"))]
    pub use super::install_vertical_concatenate_n_args_f64;
    #[cfg(all(feature = "f64", feature = "matrix2", feature = "row_vector2"))]
    pub use super::install_vertical_concatenate_r2_r2_f64;
}

pub struct MatrixVertCat {}
impl CanonicalFunctionSpecializer for MatrixVertCat {
    fn specialize_invocation(
        &self,
        invocation: &SpecializationInvocation,
        context: &mut SpecializationContext<'_>,
    ) -> MResult<SpecializedFunction> {
        ValueMatrixConcatenation::<true>::specialize(invocation, context)
    }
}

#[derive(Debug, Clone)]
pub struct VerticalConcatenateDimensionMismatch {
    pub rows: usize,
    pub cols: usize,
}
impl MechErrorKind for VerticalConcatenateDimensionMismatch {
    fn name(&self) -> &str {
        "VerticalConcatenateDimensionMismatch"
    }
    fn message(&self) -> String {
        format!(
            "Cannot vertically concatenate matrices/vectors with dimensions ({}, {})",
            self.rows, self.cols
        )
    }
}
