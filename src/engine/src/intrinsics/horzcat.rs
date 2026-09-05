use crate::intrinsics::constructors::ValueMatrixConcatenation;
use crate::intrinsics::*;
use std::marker::PhantomData;
use std::sync::LazyLock;

fn horizontal_concatenation_contract(inputs: InputPortLayout) -> OperationContractDeclaration {
    OperationContractDeclaration {
        inputs,
        outputs: vec![OutputPortPolicy {
            access: AccessMode::Write,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::Build {
                postcondition: ShapeContractReference {
                    module_path: vec!["matrix".to_owned(), "concatenate".to_owned()]
                        .into_boxed_slice(),
                    contract_name: "horizontal-output".to_owned(),
                },
            },
            alias: AliasPolicy::NoAlias,
            change_detection: ChangeDetectionPolicy::KernelReported,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    }
}

static PURE_HORIZONTAL_VARIADIC_BUILD_CONTRACT: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| {
        horizontal_concatenation_contract(InputPortLayout::Variadic {
            prefix: Box::new([]),
            repeated: InputPortPolicy {
                access: AccessMode::Read,
                delivery: DeliveryMode::Signal,
            },
            min_repetitions: 1,
        })
    });
static PURE_HORIZONTAL_UNARY_BUILD_CONTRACT: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| {
        horizontal_concatenation_contract(InputPortLayout::Fixed(
            vec![InputPortPolicy {
                access: AccessMode::Read,
                delivery: DeliveryMode::Signal,
            }]
            .into_boxed_slice(),
        ))
    });

// Horizontal Concatenate -----------------------------------------------------

#[cfg(any(
    feature = "row_vector2",
    feature = "row_vector3",
    feature = "row_vector4"
))]
macro_rules! horizontal_concatenate {
  ($name:ident, $vec_size:expr) => {
    paste!{
      #[derive(Debug)]
      struct $name<T> {
        out: Ref<[<RowVector $vec_size>]<T>>,
      }
      impl<T> MechFunctionFactory for $name<T>
      where
        T: Debug + Clone + Sync + Send + PartialEq + 'static +
        ConstElem + FunctionRuntimeType,
        #[cfg(feature = "semantic-compiler")]
        T: CompileConst + CanonicalMatrixElementBacking,
        [<RowVector $vec_size>]<T>: FunctionStateBacking,
      {
        const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::nullary(
          <[<RowVector $vec_size>]<T> as FunctionRuntimeType>::REPRESENTATION,
        );

            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                mech_core::ImplementationMemoryClass::NoAdditionalScratch
            }

        fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
          let out: Ref<[<RowVector $vec_size>]<T>> = invocation.expect_nullary()?.try_ref()?;
          Ok(Box::new(Self { out }))
        }

      }
      impl<T> MechFunctionImpl for $name<T>
      where
        T: Debug + Clone + Sync + Send + PartialEq + 'static,
        [<RowVector $vec_size>]<T>: FunctionStateBacking,
      {
        fn solve_result(&self) -> MResult<()> {
            Ok(())
        }
        fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
          Some(FunctionStatePort::from_ref(&self.out))
        }
        fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
          Ok(Some(vec![FunctionStatePort::from_ref(&self.out)]))
        }
        fn to_string(&self) -> String { format!("{:#?}", self) }

      }

      #[cfg(feature = "semantic-compiler")]
      impl<T> MechFunctionCompiler for $name<T>
      where
        T: ConstElem + CompileConst + FunctionRuntimeType + CanonicalMatrixElementBacking
      {
        fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
          let name = format!("{}<{}{}>", stringify!($name), <T as FunctionRuntimeType>::REPRESENTATION, stringify!([<RowVector $vec_size>]));
          compile_nullop!(name, self.out, ctx);
        }
      }
    }
  };
}

#[cfg(any(
    all(feature = "matrix1", feature = "row_vector2"),
    all(feature = "row_vector2", feature = "row_vector4"),
    all(feature = "matrix1", feature = "row_vector3", feature = "row_vector4"),
    all(feature = "vector2", feature = "matrix2"),
    all(feature = "vector3", feature = "matrix3x2"),
    all(feature = "matrixd", feature = "matrix4")
))]
macro_rules! horzcat_two_args {
    ($fxn:ident, $e0:ident, $e1:ident, $out:ident, $opt:ident) => {
        #[derive(Debug)]
        struct $fxn<T> {
            e0: Ref<$e0<T>>,
            e1: Ref<$e1<T>>,
            out: Ref<$out<T>>,
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
            $e0<T>: FunctionPortBacking,
            $e1<T>: FunctionPortBacking,
            $out<T>: FunctionStateBacking,
        {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
                <$out<T> as FunctionRuntimeType>::REPRESENTATION,
                <$e0<T> as FunctionRuntimeType>::REPRESENTATION,
                <$e1<T> as FunctionRuntimeType>::REPRESENTATION,
            );

            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                mech_core::ImplementationMemoryClass::NoAdditionalScratch
            }

            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                let (out, e0, e1) = invocation.expect_binary()?;
                let e0: Ref<$e0<T>> = e0.try_ref()?;
                let e1: Ref<$e1<T>> = e1.try_ref()?;
                let out: Ref<$out<T>> = out.try_ref()?;
                Ok(Box::new(Self { e0, e1, out }))
            }
        }
        impl<T> MechFunctionImpl for $fxn<T>
        where
            T: Debug + Clone + Sync + Send + PartialEq + 'static,
            $out<T>: FunctionStateBacking,
        {
            fn solve_result(&self) -> MResult<()> {
                unsafe {
                    let e0_ptr = (*(self.e0.as_ptr())).clone();
                    let e1_ptr = (*(self.e1.as_ptr())).clone();
                    let out_ptr = (&mut *(self.out.as_mut_ptr()));
                    $opt!(out_ptr, e0_ptr, e1_ptr);
                };
                Ok(())
            }
            fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
                Some(FunctionStatePort::from_ref(&self.out))
            }
            fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
                Ok(Some(vec![FunctionStatePort::from_ref(&self.out)]))
            }
            fn to_string(&self) -> String {
                format!("{:#?}", self)
            }
        }
        #[cfg(feature = "semantic-compiler")]
        impl<T> MechFunctionCompiler for $fxn<T>
        where
            T: ConstElem + CompileConst + FunctionRuntimeType + CanonicalMatrixElementBacking,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!(
                    "{}<{}{}{}{}>",
                    stringify!($fxn),
                    <T as FunctionRuntimeType>::REPRESENTATION,
                    stringify!($out),
                    stringify!($e0),
                    stringify!($e1)
                );
                compile_binop!(name, self.out, self.e0, self.e1, ctx);
            }
        }
    };
}

#[cfg(any(
    all(feature = "row_vector3", feature = "matrix1"),
    all(feature = "row_vector4", feature = "matrix1", feature = "row_vector2"),
    all(feature = "vector2", feature = "matrix2x3"),
    all(feature = "vector3", feature = "matrix3"),
    all(feature = "matrixd", feature = "vector4", feature = "matrix4")
))]
macro_rules! horzcat_three_args {
    ($fxn:ident, $e0:ident, $e1:ident, $e2:ident, $out:ident, $opt:ident) => {
        #[derive(Debug)]
        struct $fxn<T> {
            e0: Ref<$e0<T>>,
            e1: Ref<$e1<T>>,
            e2: Ref<$e2<T>>,
            out: Ref<$out<T>>,
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
            $e0<T>: FunctionPortBacking,
            $e1<T>: FunctionPortBacking,
            $e2<T>: FunctionPortBacking,
            $out<T>: FunctionStateBacking,
        {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::ternary(
                <$out<T> as FunctionRuntimeType>::REPRESENTATION,
                <$e0<T> as FunctionRuntimeType>::REPRESENTATION,
                <$e1<T> as FunctionRuntimeType>::REPRESENTATION,
                <$e2<T> as FunctionRuntimeType>::REPRESENTATION,
            );

            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                mech_core::ImplementationMemoryClass::NoAdditionalScratch
            }

            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                let (out, e0, e1, e2) = invocation.expect_ternary()?;
                let e0: Ref<$e0<T>> = e0.try_ref()?;
                let e1: Ref<$e1<T>> = e1.try_ref()?;
                let e2: Ref<$e2<T>> = e2.try_ref()?;
                let out: Ref<$out<T>> = out.try_ref()?;
                Ok(Box::new(Self { e0, e1, e2, out }))
            }
        }
        impl<T> MechFunctionImpl for $fxn<T>
        where
            T: Debug + Clone + Sync + Send + PartialEq + 'static,
            $out<T>: FunctionStateBacking,
        {
            fn solve_result(&self) -> MResult<()> {
                unsafe {
                    let e0_ptr = (*(self.e0.as_ptr())).clone();
                    let e1_ptr = (*(self.e1.as_ptr())).clone();
                    let e2_ptr = (*(self.e2.as_ptr())).clone();
                    let out_ptr = (&mut *(self.out.as_mut_ptr()));
                    $opt!(out_ptr, e0_ptr, e1_ptr, e2_ptr);
                };
                Ok(())
            }
            fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
                Some(FunctionStatePort::from_ref(&self.out))
            }
            fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
                Ok(Some(vec![FunctionStatePort::from_ref(&self.out)]))
            }
            fn to_string(&self) -> String {
                format!("{:#?}", self)
            }
        }
        #[cfg(feature = "semantic-compiler")]
        impl<T> MechFunctionCompiler for $fxn<T>
        where
            T: ConstElem + CompileConst + FunctionRuntimeType + CanonicalMatrixElementBacking,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!(
                    "{}<{}{}{}{}{}>",
                    stringify!($fxn),
                    <T as FunctionRuntimeType>::REPRESENTATION,
                    stringify!($out),
                    stringify!($e0),
                    stringify!($e1),
                    stringify!($e2)
                );
                compile_ternop!(name, self.out, self.e0, self.e1, self.e2, ctx);
            }
        }
    };
}

#[cfg(all(feature = "matrix4", feature = "vector4"))]
macro_rules! horzcat_four_args {
    ($fxn:ident, $e0:ident, $e1:ident, $e2:ident, $e3:ident, $out:ident, $opt:ident) => {
        #[derive(Debug)]
        struct $fxn<T> {
            e0: Ref<$e0<T>>,
            e1: Ref<$e1<T>>,
            e2: Ref<$e2<T>>,
            e3: Ref<$e3<T>>,
            out: Ref<$out<T>>,
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
            $e0<T>: FunctionPortBacking,
            $e1<T>: FunctionPortBacking,
            $e2<T>: FunctionPortBacking,
            $e3<T>: FunctionPortBacking,
            $out<T>: FunctionStateBacking,
        {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::quaternary(
                <$out<T> as FunctionRuntimeType>::REPRESENTATION,
                <$e0<T> as FunctionRuntimeType>::REPRESENTATION,
                <$e1<T> as FunctionRuntimeType>::REPRESENTATION,
                <$e2<T> as FunctionRuntimeType>::REPRESENTATION,
                <$e3<T> as FunctionRuntimeType>::REPRESENTATION,
            );

            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                mech_core::ImplementationMemoryClass::NoAdditionalScratch
            }

            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                let (out, e0, e1, e2, e3) = invocation.expect_quaternary()?;
                let e0: Ref<$e0<T>> = e0.try_ref()?;
                let e1: Ref<$e1<T>> = e1.try_ref()?;
                let e2: Ref<$e2<T>> = e2.try_ref()?;
                let e3: Ref<$e3<T>> = e3.try_ref()?;
                let out: Ref<$out<T>> = out.try_ref()?;
                Ok(Box::new(Self {
                    e0,
                    e1,
                    e2,
                    e3,
                    out,
                }))
            }
        }
        impl<T> MechFunctionImpl for $fxn<T>
        where
            T: Debug + Clone + Sync + Send + PartialEq + 'static,
            $out<T>: FunctionStateBacking,
        {
            fn solve_result(&self) -> MResult<()> {
                unsafe {
                    let e0_ptr = (*(self.e0.as_ptr())).clone();
                    let e1_ptr = (*(self.e1.as_ptr())).clone();
                    let e2_ptr = (*(self.e2.as_ptr())).clone();
                    let e3_ptr = (*(self.e3.as_ptr())).clone();
                    let out_ptr = (&mut *(self.out.as_mut_ptr()));
                    $opt!(out_ptr, e0_ptr, e1_ptr, e2_ptr, e3_ptr);
                };
                Ok(())
            }
            fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
                Some(FunctionStatePort::from_ref(&self.out))
            }
            fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
                Ok(Some(vec![FunctionStatePort::from_ref(&self.out)]))
            }
            fn to_string(&self) -> String {
                format!("{:#?}", self)
            }
        }
        #[cfg(feature = "semantic-compiler")]
        impl<T> MechFunctionCompiler for $fxn<T>
        where
            T: ConstElem + CompileConst + FunctionRuntimeType + CanonicalMatrixElementBacking,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!(
                    "{}<{}{}{}{}{}{}>",
                    stringify!($fxn),
                    <T as FunctionRuntimeType>::REPRESENTATION,
                    stringify!($out),
                    stringify!($e0),
                    stringify!($e1),
                    stringify!($e2),
                    stringify!($e3)
                );
                compile_quadop!(name, self.out, self.e0, self.e1, self.e2, self.e3, ctx);
            }
        }
    };
}

// HorizontalConcatenateTwoArgs -----------------------------------------------

#[cfg(feature = "matrixd")]
struct HorizontalConcatenateTwoArgs<T> {
    e0: Box<dyn CopyMat<T>>,
    e1: Box<dyn CopyMat<T>>,
    out: Ref<DMatrix<T>>,
}
#[cfg(feature = "matrixd")]
impl<T> MechFunctionFactory for HorizontalConcatenateTwoArgs<T>
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
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, arg0, arg1) = invocation.expect_binary()?;
        let e0: Box<dyn CopyMat<T>> = arg0.try_copyable_matrix()?;
        let e1: Box<dyn CopyMat<T>> = arg1.try_copyable_matrix()?;
        let out: Ref<DMatrix<T>> = out.try_ref()?;
        Ok(Box::new(Self { e0, e1, out }))
    }

    fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_HORIZONTAL_VARIADIC_BUILD_CONTRACT)
    }
}
#[cfg(feature = "matrixd")]
impl<T> MechFunctionImpl for HorizontalConcatenateTwoArgs<T>
where
    T: Debug + Clone + Sync + Send + PartialEq + 'static,
{
    fn solve_result(&self) -> MResult<()> {
        let offset = self.e0.copy_into(&self.out, 0);
        self.e1.copy_into(&self.out, offset);
        Ok(())
    }

    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_HORIZONTAL_VARIADIC_BUILD_CONTRACT)
    }
    fn to_string(&self) -> String {
        format!("HorizontalConcatenateTwoArgs\n{:#?}", self.out)
    }
}
#[cfg(feature = "matrixd")]
#[cfg(feature = "semantic-compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateTwoArgs<T>
where
    T: ConstElem + CompileConst + FunctionRuntimeType + CanonicalMatrixElementBacking,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let mut registers = [0, 0, 0];

        registers[0] = compile_register!(self.out, ctx);
        registers[1] = compile_register_mat!(self.e0, ctx);
        registers[2] = compile_register_mat!(self.e1, ctx);

        ctx.emit_binop(
            hash_str(&format!(
                "HorizontalConcatenateTwoArgs<{}>",
                <T as FunctionRuntimeType>::REPRESENTATION
            )),
            registers[0],
            registers[1],
            registers[2],
        );

        Ok(registers[0])
    }
}

// HorizontalConcatenateThreeArgs ---------------------------------------------

#[cfg(feature = "matrixd")]
struct HorizontalConcatenateThreeArgs<T> {
    e0: Box<dyn CopyMat<T>>,
    e1: Box<dyn CopyMat<T>>,
    e2: Box<dyn CopyMat<T>>,
    out: Ref<DMatrix<T>>,
}
#[cfg(feature = "matrixd")]
impl<T> MechFunctionFactory for HorizontalConcatenateThreeArgs<T>
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
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, arg0, arg1, arg2) = invocation.expect_ternary()?;
        let e0: Box<dyn CopyMat<T>> = arg0.try_copyable_matrix()?;
        let e1: Box<dyn CopyMat<T>> = arg1.try_copyable_matrix()?;
        let e2: Box<dyn CopyMat<T>> = arg2.try_copyable_matrix()?;
        let out: Ref<DMatrix<T>> = out.try_ref()?;
        Ok(Box::new(Self { e0, e1, e2, out }))
    }

    fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_HORIZONTAL_VARIADIC_BUILD_CONTRACT)
    }
}
#[cfg(feature = "matrixd")]
impl<T> MechFunctionImpl for HorizontalConcatenateThreeArgs<T>
where
    T: Debug + Clone + Sync + Send + PartialEq + 'static,
{
    fn solve_result(&self) -> MResult<()> {
        let mut offset = self.e0.copy_into(&self.out, 0);
        offset += self.e1.copy_into(&self.out, offset);
        self.e2.copy_into(&self.out, offset);
        Ok(())
    }

    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_HORIZONTAL_VARIADIC_BUILD_CONTRACT)
    }
    fn to_string(&self) -> String {
        format!("HorizontalConcatenateThreeArgs\n{:#?}", self.out)
    }
}
#[cfg(feature = "matrixd")]
#[cfg(feature = "semantic-compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateThreeArgs<T>
where
    T: ConstElem + CompileConst + FunctionRuntimeType + CanonicalMatrixElementBacking,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let mut registers = [0, 0, 0, 0];

        registers[0] = compile_register!(self.out, ctx);
        registers[1] = compile_register_mat!(self.e0, ctx);
        registers[2] = compile_register_mat!(self.e1, ctx);
        registers[3] = compile_register_mat!(self.e2, ctx);

        ctx.emit_ternop(
            hash_str(&format!(
                "HorizontalConcatenateThreeArgs<{}>",
                <T as FunctionRuntimeType>::REPRESENTATION
            )),
            registers[0],
            registers[1],
            registers[2],
            registers[3],
        );
        Ok(registers[0])
    }
}

// HorizontalConcatenateFourArgs ----------------------------------------------

#[cfg(feature = "matrixd")]
#[allow(
    dead_code,
    reason = "the four-input compatibility factory is selected only by native-plan and focused tests"
)]
struct HorizontalConcatenateFourArgs<T> {
    e0: Box<dyn CopyMat<T>>,
    e1: Box<dyn CopyMat<T>>,
    e2: Box<dyn CopyMat<T>>,
    e3: Box<dyn CopyMat<T>>,
    out: Ref<DMatrix<T>>,
}
#[cfg(feature = "matrixd")]
impl<T> MechFunctionFactory for HorizontalConcatenateFourArgs<T>
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
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, arg0, arg1, arg2, arg3) = invocation.expect_quaternary()?;
        let e0: Box<dyn CopyMat<T>> = arg0.try_copyable_matrix()?;
        let e1: Box<dyn CopyMat<T>> = arg1.try_copyable_matrix()?;
        let e2: Box<dyn CopyMat<T>> = arg2.try_copyable_matrix()?;
        let e3: Box<dyn CopyMat<T>> = arg3.try_copyable_matrix()?;
        let out: Ref<DMatrix<T>> = out.try_ref()?;
        Ok(Box::new(Self {
            e0,
            e1,
            e2,
            e3,
            out,
        }))
    }

    fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_HORIZONTAL_VARIADIC_BUILD_CONTRACT)
    }
}
#[cfg(feature = "matrixd")]
impl<T> MechFunctionImpl for HorizontalConcatenateFourArgs<T>
where
    T: Debug + Clone + Sync + Send + PartialEq + 'static,
{
    fn solve_result(&self) -> MResult<()> {
        let mut offset = self.e0.copy_into(&self.out, 0);
        offset += self.e1.copy_into(&self.out, offset);
        offset += self.e2.copy_into(&self.out, offset);
        self.e3.copy_into(&self.out, offset);
        Ok(())
    }

    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_HORIZONTAL_VARIADIC_BUILD_CONTRACT)
    }
    fn to_string(&self) -> String {
        format!("HorizontalConcatenateFourArgs\n{:#?}", self.out)
    }
}
#[cfg(feature = "matrixd")]
#[cfg(feature = "semantic-compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateFourArgs<T>
where
    T: ConstElem + CompileConst + FunctionRuntimeType + CanonicalMatrixElementBacking,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let registers = [
            compile_register!(self.out, ctx),
            compile_register_mat!(self.e0, ctx),
            compile_register_mat!(self.e1, ctx),
            compile_register_mat!(self.e2, ctx),
            compile_register_mat!(self.e3, ctx),
        ];

        ctx.emit_quadop(
            hash_str(&format!(
                "HorizontalConcatenateFourArgs<{}>",
                <T as FunctionRuntimeType>::REPRESENTATION
            )),
            registers[0],
            registers[1],
            registers[2],
            registers[3],
            registers[4],
        );
        Ok(registers[0])
    }
}

// HorizontalConcatenateNArgs -------------------------------------------------

#[cfg(feature = "matrixd")]
#[allow(
    dead_code,
    reason = "retained as a frozen bytecode factory outside the standard linked runtime catalog"
)]
enum HorizontalConcatenateInput<T> {
    Scalar(Ref<T>),
    Matrix(Box<dyn CopyMat<T>>),
}

#[cfg(feature = "matrixd")]
#[allow(
    dead_code,
    reason = "retained as a frozen bytecode factory outside the standard linked runtime catalog"
)]
struct HorizontalConcatenateNArgs<T> {
    e0: Vec<HorizontalConcatenateInput<T>>,
    out: Ref<DMatrix<T>>,
}
#[cfg(feature = "matrixd")]
impl<T> MechFunctionFactory for HorizontalConcatenateNArgs<T>
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
        FunctionValueRepresentation::AnyValue,
    );

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, inputs) = invocation.expect_variadic()?;
        let mut e0 = Vec::with_capacity(inputs.len());
        for arg in inputs {
            if matches!(
                arg.value().representation(),
                FunctionValueRepresentation::Matrix { .. }
            ) {
                e0.push(HorizontalConcatenateInput::Matrix(
                    arg.try_copyable_matrix()?,
                ));
            } else {
                e0.push(HorizontalConcatenateInput::Scalar(arg.try_ref()?));
            }
        }
        let out: Ref<DMatrix<T>> = out.try_ref()?;
        Ok(Box::new(Self { e0, out }))
    }

    fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_HORIZONTAL_VARIADIC_BUILD_CONTRACT)
    }
}
#[cfg(feature = "matrixd")]
impl<T> MechFunctionImpl for HorizontalConcatenateNArgs<T>
where
    T: Debug + Clone + PartialEq + 'static,
{
    fn solve_result(&self) -> MResult<()> {
        let mut offset = 0;
        for e in &self.e0 {
            match e {
                HorizontalConcatenateInput::Scalar(value) => unsafe {
                    (&mut *self.out.as_mut_ptr())[offset] = value.borrow().clone();
                    offset += 1;
                },
                HorizontalConcatenateInput::Matrix(matrix) => {
                    offset += matrix.copy_into(&self.out, offset);
                }
            }
        }
        Ok(())
    }

    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_HORIZONTAL_VARIADIC_BUILD_CONTRACT)
    }
    fn to_string(&self) -> String {
        format!("HorizontalConcatenateNArgs\n{:#?}", self.out)
    }
}
#[cfg(feature = "matrixd")]
#[cfg(feature = "semantic-compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateNArgs<T>
where
    T: ConstElem + CompileConst + FunctionRuntimeType + CanonicalMatrixElementBacking,
{
    fn reserve_bytecode_registers(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<()> {
        Ok(())
    }

    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let mut registers = [0, 0];
        registers[0] = compile_register!(self.out, ctx);

        let mut input_registers = Vec::new();
        for e in &self.e0 {
            input_registers.push(match e {
                HorizontalConcatenateInput::Scalar(value) => {
                    compile_register_brrw!(value, ctx)
                }
                HorizontalConcatenateInput::Matrix(matrix) => {
                    compile_register_mat!(matrix, ctx)
                }
            });
        }
        ctx.emit_varop(
            hash_str(&format!(
                "HorizontalConcatenateNArgs<{}>",
                <T as FunctionRuntimeType>::REPRESENTATION
            )),
            registers[0],
            input_registers,
        );
        Ok(registers[0])
    }
}

// HorizontalConcatenateRD ----------------------------------------------------

#[cfg(feature = "row_vectord")]
#[derive(Debug)]
struct HorizontalConcatenateRD<T> {
    output: FunctionValueOutput,
    _marker: PhantomData<T>,
    #[cfg(feature = "semantic-compiler")]
    out: Ref<RowDVector<T>>,
}
#[cfg(feature = "row_vectord")]
impl<T> MechFunctionFactory for HorizontalConcatenateRD<T>
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
        RuntimeFunctionSignature::nullary(<RowDVector<T> as FunctionRuntimeType>::REPRESENTATION);

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let out = invocation.expect_nullary()?;
        let output = out.value();
        let out: Ref<RowDVector<T>> = out.try_ref()?;
        #[cfg(not(feature = "semantic-compiler"))]
        drop(out);
        Ok(Box::new(Self {
            output,
            _marker: PhantomData,
            #[cfg(feature = "semantic-compiler")]
            out,
        }))
    }
}
#[cfg(feature = "row_vectord")]
impl<T> MechFunctionImpl for HorizontalConcatenateRD<T>
where
    T: Debug + Clone + Sync + Send + PartialEq + 'static,
{
    fn solve_result(&self) -> MResult<()> {
        Ok(())
    }

    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }

    fn reactive_output_value_cells(&self) -> Vec<ValueCell> {
        vec![self.output.cell().clone()]
    }
}
#[cfg(feature = "row_vectord")]
#[cfg(feature = "semantic-compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateRD<T>
where
    T: ConstElem + CompileConst + FunctionRuntimeType + CanonicalMatrixElementBacking,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!(
            "HorizontalConcatenateRD<{}>",
            <T as FunctionRuntimeType>::REPRESENTATION
        );
        compile_nullop!(name, self.out, ctx);
    }
}

// HorizontalConcatenateRDN ---------------------------------------------------

#[cfg(feature = "row_vectord")]
struct HorizontalConcatenateRDN<T> {
    scalar: Vec<(Ref<T>, usize)>,
    matrix: Vec<(Box<dyn CopyMat<T>>, usize)>,
    out: Ref<RowDVector<T>>,
}
#[cfg(feature = "row_vectord")]
impl<T> MechFunctionFactory for HorizontalConcatenateRDN<T>
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
        <RowDVector<T> as FunctionRuntimeType>::REPRESENTATION,
        FunctionValueRepresentation::AnyValue,
    );

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, inputs) = invocation.expect_variadic()?;
        let mut scalar: Vec<(Ref<T>, usize)> = Vec::new();
        let mut matrix: Vec<(Box<dyn CopyMat<T>>, usize)> = Vec::new();
        for (i, arg) in inputs.enumerate() {
            if matches!(
                arg.value().representation(),
                FunctionValueRepresentation::Matrix { .. }
            ) {
                matrix.push((arg.try_copyable_matrix()?, i));
            } else {
                scalar.push((arg.try_ref()?, i));
            }
        }
        let out: Ref<RowDVector<T>> = out.try_ref()?;
        Ok(Box::new(Self {
            scalar,
            matrix,
            out,
        }))
    }

    fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_HORIZONTAL_VARIADIC_BUILD_CONTRACT)
    }
}
#[cfg(feature = "row_vectord")]
impl<T> MechFunctionImpl for HorizontalConcatenateRDN<T>
where
    T: Debug + Clone + Sync + Send + PartialEq + 'static,
{
    fn solve_result(&self) -> MResult<()> {
        unsafe {
            let out_ptr = &mut *(self.out.as_mut_ptr());
            for (e, i) in &self.matrix {
                e.copy_into_r(&self.out, *i);
            }
            for (e, i) in &self.scalar {
                out_ptr[*i] = e.borrow().clone();
            }
        };
        Ok(())
    }

    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_HORIZONTAL_VARIADIC_BUILD_CONTRACT)
    }
    fn to_string(&self) -> String {
        format!("HorizontalConcatenateRDN\n{:#?}", self.out)
    }
}
#[cfg(feature = "row_vectord")]
#[cfg(feature = "semantic-compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateRDN<T>
where
    T: CompileConst + ConstElem + FunctionRuntimeType + CanonicalMatrixElementBacking,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let mut registers = [0, 0];

        registers[0] = compile_register!(self.out, ctx);

        let mut mat_regs = Vec::new();
        for (e, _) in &self.matrix {
            mat_regs.push(compile_register_mat!(e, ctx));
        }
        let mut scalar_regs = Vec::new();
        for (e, _) in &self.scalar {
            let e_reg = compile_register_brrw!(e, ctx);
            scalar_regs.push(e_reg);
        }
        let mut all_regs = vec![];
        all_regs.push(registers[0]);
        all_regs.extend(mat_regs);
        all_regs.extend(scalar_regs);

        ctx.emit_varop(
            hash_str(&format!(
                "HorizontalConcatenateRDN<{}>",
                <T as FunctionRuntimeType>::REPRESENTATION
            )),
            registers[0],
            all_regs[1..].to_vec(),
        );

        Ok(registers[0])
    }
}

mech_core::declare_native_runtime_factory! {
    cfg: all(
        feature = "f64",
        feature = "matrix_horzcat",
        feature = "row_vectord"
    ),

    registration: register_horizontal_concatenate_rdn_f64,
    installer: install_horizontal_concatenate_rdn_f64,

    name: "HorizontalConcatenateRDN<f64>",
    factory_type: HorizontalConcatenateRDN<f64>,
    contract: RuntimeFunctionContract::horizontal_concatenation(RuntimeOutputAliasPolicy::DisallowInputAlias),
    compiler_family: mech_core::RuntimeFamilyId::from_name("HorizontalConcatenateRDN<f64>"),

    package: "mech-engine",
    crate_name: "mech_engine",
    installer_path: "mech_engine::__mech_native::install_horizontal_concatenate_rdn_f64",

    extra_cargo_features: ["matrix_horzcat"],
}

#[cfg(all(
    test,
    feature = "compiler",
    feature = "matrixd",
    feature = "row_vectord",
    feature = "f64",
))]
mod compiler_tests {
    use super::*;
    use crate::test_support::bytecode_compiler::RecordingBytecodeCompilerContext;

    fn matrix() -> Ref<DMatrix<f64>> {
        Ref::new(DMatrix::from_vec(1, 1, vec![7.0]))
    }

    fn assert_single_matrix_load(
        context: &RecordingBytecodeCompilerContext,
        matrix: &Ref<DMatrix<f64>>,
    ) -> Register {
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
        matrix_register
    }

    #[test]
    fn pointer_register_matrix_initializes_once() -> MResult<()> {
        let mut context = RecordingBytecodeCompilerContext::default();
        let context = &mut context;
        let matrix_a = matrix();
        let matrix_b = matrix();

        let register_a = compile_register_mat!(matrix_a, context);
        let register_a_again = compile_register_mat!(matrix_a, context);
        let register_b = compile_register_mat!(matrix_b, context);

        assert_eq!(register_a_again, register_a);
        assert_ne!(register_b, register_a);
        assert_eq!(context.const_count, 2);
        assert_eq!(
            context
                .instructions
                .iter()
                .filter(|instruction| {
                    matches!(instruction, BytecodeInstruction::ConstLoad { dst, .. } if *dst == register_a)
                })
                .count(),
            1,
        );
        assert_eq!(
            context
                .instructions
                .iter()
                .filter(|instruction| {
                    matches!(instruction, BytecodeInstruction::ConstLoad { dst, .. } if *dst == register_b)
                })
                .count(),
            1,
        );
        Ok(())
    }

    #[test]
    fn horizontal_concatenate_four_args_reuses_repeated_matrix_register() {
        let matrix = matrix();
        let function = HorizontalConcatenateFourArgs {
            e0: Box::new(matrix.clone()),
            e1: Box::new(matrix.clone()),
            e2: Box::new(matrix.clone()),
            e3: Box::new(matrix.clone()),
            out: Ref::new(DMatrix::from_element(1, 4, 0.0)),
        };
        let mut context = RecordingBytecodeCompilerContext::default();
        function.compile(&mut context).unwrap();

        let matrix_register = assert_single_matrix_load(&context, &matrix);
        assert!(matches!(
          context.instructions.last(),
          Some(BytecodeInstruction::RuntimeQuaternary { a, b, c, d, .. })
            if [*a, *b, *c, *d] == [matrix_register; 4]
        ));
    }

    #[test]
    fn horizontal_concatenate_n_args_reuses_repeated_matrix_register() {
        let matrix = matrix();
        let function = HorizontalConcatenateNArgs {
            e0: vec![
                HorizontalConcatenateInput::Matrix(Box::new(matrix.clone())),
                HorizontalConcatenateInput::Matrix(Box::new(matrix.clone())),
            ],
            out: Ref::new(DMatrix::from_element(1, 2, 0.0)),
        };
        let mut context = RecordingBytecodeCompilerContext::default();
        function.compile(&mut context).unwrap();

        let matrix_register = assert_single_matrix_load(&context, &matrix);
        assert!(matches!(
          context.instructions.last(),
          Some(BytecodeInstruction::RuntimeVariadic { arguments, .. })
            if arguments == &vec![matrix_register, matrix_register]
        ));
    }

    #[test]
    fn horizontal_concatenate_n_args_preserves_scalar_and_matrix_order() {
        let scalar = Ref::new(9.0);
        let scalar_cell = scalar.reactive_cell_id().get() as usize;
        let matrix = matrix();
        let function = HorizontalConcatenateNArgs {
            e0: vec![
                HorizontalConcatenateInput::Scalar(scalar),
                HorizontalConcatenateInput::Matrix(Box::new(matrix.clone())),
            ],
            out: Ref::new(DMatrix::from_element(1, 2, 0.0)),
        };
        let mut context = RecordingBytecodeCompilerContext::default();
        function.compile(&mut context).unwrap();

        let scalar_register = context.reg_map[&scalar_cell];
        let matrix_register = assert_single_matrix_load(&context, &matrix);
        assert!(matches!(
          context.instructions.last(),
          Some(BytecodeInstruction::RuntimeVariadic { arguments, .. })
            if arguments == &vec![scalar_register, matrix_register]
        ));

        function.solve_result().unwrap();
        assert_eq!(function.out.borrow().as_slice(), &[9.0, 7.0]);
    }

    #[test]
    fn horizontal_concatenate_rdn_reuses_repeated_matrix_register() {
        let matrix = matrix();
        let scalar = Ref::new(9.0);
        let scalar_cell = scalar.reactive_cell_id().get() as usize;
        let function = HorizontalConcatenateRDN {
            matrix: vec![(Box::new(matrix.clone()), 0), (Box::new(matrix.clone()), 1)],
            scalar: vec![(scalar, 2)],
            out: Ref::new(RowDVector::from_element(3, 0.0)),
        };
        let mut context = RecordingBytecodeCompilerContext::default();
        function.compile(&mut context).unwrap();

        let matrix_register = assert_single_matrix_load(&context, &matrix);
        let scalar_register = context.reg_map[&scalar_cell];
        assert!(matches!(
          context.instructions.last(),
          Some(BytecodeInstruction::RuntimeVariadic { arguments, .. })
            if arguments == &vec![matrix_register, matrix_register, scalar_register]
        ));
    }
}

// HorizontalConcatenateS1D ---------------------------------------------------

#[cfg(feature = "matrixd")]
#[derive(Debug)]
struct HorizontalConcatenateS1D<T> {
    arg: Ref<T>,
    out: Ref<DMatrix<T>>,
}
#[cfg(feature = "matrixd")]
impl<T> MechFunctionFactory for HorizontalConcatenateS1D<T>
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
    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::unary(
        <DMatrix<T> as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
    );

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, arg0) = invocation.expect_unary()?;
        let arg: Ref<T> = arg0.try_ref()?;
        let out: Ref<DMatrix<T>> = out.try_ref()?;
        Ok(Box::new(Self { arg, out }))
    }

    fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_HORIZONTAL_UNARY_BUILD_CONTRACT)
    }
}
#[cfg(feature = "matrixd")]
impl<T> MechFunctionImpl for HorizontalConcatenateS1D<T>
where
    T: Debug + Clone + Sync + Send + PartialEq + 'static,
{
    fn solve_result(&self) -> MResult<()> {
        unsafe {
            let out_ptr = &mut *(self.out.as_mut_ptr());
            out_ptr[0] = self.arg.borrow().clone();
        };
        Ok(())
    }

    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_HORIZONTAL_UNARY_BUILD_CONTRACT)
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}
#[cfg(feature = "matrixd")]
#[cfg(feature = "semantic-compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateS1D<T>
where
    T: ConstElem + CompileConst + FunctionRuntimeType + CanonicalMatrixElementBacking,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!(
            "HorizontalConcatenateS1D<{}>",
            <T as FunctionRuntimeType>::REPRESENTATION
        );
        compile_unop!(name, self.out, self.arg, ctx);
    }
}

// HorizontalConcatenateS1 ----------------------------------------------------

#[cfg(feature = "matrix1")]
#[derive(Debug)]
struct HorizontalConcatenateS1<T> {
    arg: Ref<T>,
    out: Ref<Matrix1<T>>,
}
#[cfg(feature = "matrix1")]
impl<T> MechFunctionFactory for HorizontalConcatenateS1<T>
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
    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::unary(
        <Matrix1<T> as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
    );

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, arg0) = invocation.expect_unary()?;
        let arg: Ref<T> = arg0.try_ref()?;
        let out: Ref<Matrix1<T>> = out.try_ref()?;
        Ok(Box::new(Self { arg, out }))
    }

    fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_HORIZONTAL_UNARY_BUILD_CONTRACT)
    }
}
#[cfg(feature = "matrix1")]
impl<T> MechFunctionImpl for HorizontalConcatenateS1<T>
where
    T: Debug + Clone + Sync + Send + PartialEq + 'static,
{
    fn solve_result(&self) -> MResult<()> {
        unsafe {
            let out_ptr = &mut *(self.out.as_mut_ptr());
            out_ptr[0] = self.arg.borrow().clone();
        };
        Ok(())
    }

    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }

    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_HORIZONTAL_UNARY_BUILD_CONTRACT)
    }
}
#[cfg(feature = "matrix1")]
#[cfg(feature = "semantic-compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateS1<T>
where
    T: ConstElem + CompileConst + FunctionRuntimeType + CanonicalMatrixElementBacking,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!(
            "HorizontalConcatenateS1<{}>",
            <T as FunctionRuntimeType>::REPRESENTATION
        );
        compile_unop!(name, self.out, self.arg, ctx);
    }
}

// HorizontalConcatenateS2 --------------------------------------------------

#[cfg(feature = "row_vector2")]
#[derive(Debug)]
struct HorizontalConcatenateS2<T> {
    e0: Ref<T>,
    e1: Ref<T>,
    out: Ref<RowVector2<T>>,
}
#[cfg(feature = "row_vector2")]
impl<T> MechFunctionFactory for HorizontalConcatenateS2<T>
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
        <RowVector2<T> as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
    );

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, arg0, arg1) = invocation.expect_binary()?;
        let e0: Ref<T> = arg0.try_ref()?;
        let e1: Ref<T> = arg1.try_ref()?;
        let out: Ref<RowVector2<T>> = out.try_ref()?;
        Ok(Box::new(Self { e0, e1, out }))
    }

    fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_HORIZONTAL_VARIADIC_BUILD_CONTRACT)
    }
}
#[cfg(feature = "row_vector2")]
impl<T> MechFunctionImpl for HorizontalConcatenateS2<T>
where
    T: Debug + Clone + Sync + Send + PartialEq + 'static,
{
    fn solve_result(&self) -> MResult<()> {
        unsafe {
            let out_ptr = &mut *(self.out.as_mut_ptr());
            out_ptr[0] = self.e0.borrow().clone();
            out_ptr[1] = self.e1.borrow().clone();
        };
        Ok(())
    }

    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }

    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_HORIZONTAL_VARIADIC_BUILD_CONTRACT)
    }
}
#[cfg(feature = "row_vector2")]
#[cfg(feature = "semantic-compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateS2<T>
where
    T: ConstElem + CompileConst + FunctionRuntimeType + CanonicalMatrixElementBacking,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!(
            "HorizontalConcatenateS2<{}>",
            <T as FunctionRuntimeType>::REPRESENTATION
        );
        compile_binop!(name, self.out, self.e0, self.e1, ctx);
    }
}

mech_core::declare_native_runtime_factory! {
    cfg: all(
        feature = "f64",
        feature = "matrix_horzcat",
        feature = "row_vector2"
    ),

    registration: register_horizontal_concatenate_s2_f64,
    installer: install_horizontal_concatenate_s2_f64,

    name: "HorizontalConcatenateS2<f64>",
    factory_type: HorizontalConcatenateS2<f64>,
    contract: RuntimeFunctionContract::horizontal_concatenation(RuntimeOutputAliasPolicy::DisallowInputAlias),
    compiler_family: mech_core::RuntimeFamilyId::from_name("HorizontalConcatenateS2<f64>"),

    package: "mech-engine",
    crate_name: "mech_engine",
    installer_path: "mech_engine::__mech_native::install_horizontal_concatenate_s2_f64",

    extra_cargo_features: ["matrix_horzcat"],
}

// HorizontalConcatenateS3 --------------------------------------------------

#[cfg(feature = "row_vector3")]
#[derive(Debug)]
struct HorizontalConcatenateS3<T> {
    e0: Ref<T>,
    e1: Ref<T>,
    e2: Ref<T>,
    out: Ref<RowVector3<T>>,
}
#[cfg(feature = "row_vector3")]
impl<T> MechFunctionFactory for HorizontalConcatenateS3<T>
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
        <RowVector3<T> as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
    );

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, arg0, arg1, arg2) = invocation.expect_ternary()?;
        let e0: Ref<T> = arg0.try_ref()?;
        let e1: Ref<T> = arg1.try_ref()?;
        let e2: Ref<T> = arg2.try_ref()?;
        let out: Ref<RowVector3<T>> = out.try_ref()?;
        Ok(Box::new(Self { e0, e1, e2, out }))
    }

    fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_HORIZONTAL_VARIADIC_BUILD_CONTRACT)
    }
}
#[cfg(feature = "row_vector3")]
impl<T> MechFunctionImpl for HorizontalConcatenateS3<T>
where
    T: Debug + Clone + Sync + Send + PartialEq + 'static,
{
    fn solve_result(&self) -> MResult<()> {
        unsafe {
            let out_ptr = &mut *(self.out.as_mut_ptr());
            out_ptr[0] = self.e0.borrow().clone();
            out_ptr[1] = self.e1.borrow().clone();
            out_ptr[2] = self.e2.borrow().clone();
        };
        Ok(())
    }

    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }

    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_HORIZONTAL_VARIADIC_BUILD_CONTRACT)
    }
}
#[cfg(feature = "row_vector3")]
#[cfg(feature = "semantic-compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateS3<T>
where
    T: ConstElem + CompileConst + FunctionRuntimeType + CanonicalMatrixElementBacking,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!(
            "HorizontalConcatenateS3<{}>",
            <T as FunctionRuntimeType>::REPRESENTATION
        );
        compile_ternop!(name, self.out, self.e0, self.e1, self.e2, ctx);
    }
}

// HorizontalConcatenateS4 --------------------------------------------------

#[cfg(feature = "row_vector4")]
#[derive(Debug)]
struct HorizontalConcatenateS4<T> {
    e0: Ref<T>,
    e1: Ref<T>,
    e2: Ref<T>,
    e3: Ref<T>,
    out: Ref<RowVector4<T>>,
}
#[cfg(feature = "row_vector4")]
impl<T> MechFunctionFactory for HorizontalConcatenateS4<T>
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
        <RowVector4<T> as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
    );

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, arg0, arg1, arg2, arg3) = invocation.expect_quaternary()?;
        let e0: Ref<T> = arg0.try_ref()?;
        let e1: Ref<T> = arg1.try_ref()?;
        let e2: Ref<T> = arg2.try_ref()?;
        let e3: Ref<T> = arg3.try_ref()?;
        let out: Ref<RowVector4<T>> = out.try_ref()?;
        Ok(Box::new(Self {
            e0,
            e1,
            e2,
            e3,
            out,
        }))
    }

    fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_HORIZONTAL_VARIADIC_BUILD_CONTRACT)
    }
}
#[cfg(feature = "row_vector4")]
impl<T> MechFunctionImpl for HorizontalConcatenateS4<T>
where
    T: Debug + Clone + Sync + Send + PartialEq + 'static,
{
    fn solve_result(&self) -> MResult<()> {
        unsafe {
            let out_ptr = &mut *(self.out.as_mut_ptr());
            out_ptr[0] = self.e0.borrow().clone();
            out_ptr[1] = self.e1.borrow().clone();
            out_ptr[2] = self.e2.borrow().clone();
            out_ptr[3] = self.e3.borrow().clone();
        };
        Ok(())
    }

    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }

    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_HORIZONTAL_VARIADIC_BUILD_CONTRACT)
    }
}
#[cfg(feature = "row_vector4")]
#[cfg(feature = "semantic-compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateS4<T>
where
    T: ConstElem + CompileConst + FunctionRuntimeType + CanonicalMatrixElementBacking,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!(
            "HorizontalConcatenateS4<{}>",
            <T as FunctionRuntimeType>::REPRESENTATION
        );
        compile_quadop!(name, self.out, self.e0, self.e1, self.e2, self.e3, ctx);
    }
}

// HorizontalConcatenateR2 ----------------------------------------------------

#[cfg(feature = "row_vector2")]
horizontal_concatenate!(HorizontalConcatenateR2, 2);

// HorizontalConcatenateR3 ----------------------------------------------------

#[cfg(feature = "row_vector3")]
horizontal_concatenate!(HorizontalConcatenateR3, 3);

// HorizontalConcatenateR4 ----------------------------------------------------

#[cfg(feature = "row_vector4")]
horizontal_concatenate!(HorizontalConcatenateR4, 4);

// HorizontalConcatenateSD ----------------------------------------------------

#[cfg(feature = "row_vectord")]
#[derive(Debug)]
struct HorizontalConcatenateSD<T> {
    output: FunctionValueOutput,
    _marker: PhantomData<T>,
    #[cfg(feature = "semantic-compiler")]
    out: Ref<RowDVector<T>>,
}
#[cfg(feature = "row_vectord")]
impl<T> MechFunctionFactory for HorizontalConcatenateSD<T>
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
        RuntimeFunctionSignature::nullary(<RowDVector<T> as FunctionRuntimeType>::REPRESENTATION);

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let out = invocation.expect_nullary()?;
        let output = out.value();
        let out: Ref<RowDVector<T>> = out.try_ref()?;
        #[cfg(not(feature = "semantic-compiler"))]
        drop(out);
        Ok(Box::new(Self {
            output,
            _marker: PhantomData,
            #[cfg(feature = "semantic-compiler")]
            out,
        }))
    }
}
#[cfg(feature = "row_vectord")]
impl<T> MechFunctionImpl for HorizontalConcatenateSD<T>
where
    T: Debug + Clone + Sync + Send + PartialEq + 'static,
{
    fn solve_result(&self) -> MResult<()> {
        Ok(())
    }

    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }

    fn reactive_output_value_cells(&self) -> Vec<ValueCell> {
        vec![self.output.cell().clone()]
    }
}
#[cfg(feature = "row_vectord")]
#[cfg(feature = "semantic-compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateSD<T>
where
    T: ConstElem + CompileConst + FunctionRuntimeType + CanonicalMatrixElementBacking,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!(
            "HorizontalConcatenateSD<{}>",
            <T as FunctionRuntimeType>::REPRESENTATION
        );
        compile_nullop!(name, self.out, ctx);
    }
}

// HorizontalConcatenate for single argument types ----------------------------

macro_rules! horzcat_single {
    ($name:ident,$shape:ident) => {
        #[derive(Debug)]
        struct $name<T> {
            output: FunctionValueOutput,
            _marker: PhantomData<T>,
            #[cfg(feature = "semantic-compiler")]
            out: Ref<$shape<T>>,
        }
        impl<T> MechFunctionFactory for $name<T>
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
            $shape<T>: FunctionRuntimeType,
        {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::nullary(
                <$shape<T> as FunctionRuntimeType>::REPRESENTATION,
            );

            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                mech_core::ImplementationMemoryClass::NoAdditionalScratch
            }

            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                let out = invocation.expect_nullary()?;
                let output = out.value();
                let out: Ref<$shape<T>> = out.try_ref()?;
                #[cfg(not(feature = "semantic-compiler"))]
                drop(out);
                Ok(Box::new(Self {
                    output,
                    _marker: PhantomData,
                    #[cfg(feature = "semantic-compiler")]
                    out,
                }))
            }
        }
        impl<T> MechFunctionImpl for $name<T>
        where
            T: Debug + Clone + Sync + Send + PartialEq + 'static,
        {
            fn solve_result(&self) -> MResult<()> {
                Ok(())
            }

            fn to_string(&self) -> String {
                format!("{:#?}", self)
            }

            fn reactive_output_value_cells(&self) -> Vec<ValueCell> {
                vec![self.output.cell().clone()]
            }
        }
        #[cfg(feature = "semantic-compiler")]
        impl<T> MechFunctionCompiler for $name<T>
        where
            T: ConstElem + CompileConst + FunctionRuntimeType + CanonicalMatrixElementBacking,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!(
                    "{}<{}>",
                    stringify!($name),
                    <T as FunctionRuntimeType>::REPRESENTATION
                );
                compile_nullop!(name, self.out, ctx);
            }
        }
    };
}

#[cfg(feature = "matrix1")]
horzcat_single!(HorizontalConcatenateM1, Matrix1);
#[cfg(feature = "matrix2")]
horzcat_single!(HorizontalConcatenateM2, Matrix2);
#[cfg(feature = "matrix3")]
horzcat_single!(HorizontalConcatenateM3, Matrix3);
#[cfg(feature = "matrix4")]
horzcat_single!(HorizontalConcatenateM4, Matrix4);
#[cfg(feature = "matrix2x3")]
horzcat_single!(HorizontalConcatenateM2x3, Matrix2x3);
#[cfg(feature = "matrix3x2")]
horzcat_single!(HorizontalConcatenateM3x2, Matrix3x2);
#[cfg(feature = "matrixd")]
horzcat_single!(HorizontalConcatenateMD, DMatrix);
#[cfg(feature = "vector2")]
horzcat_single!(HorizontalConcatenateV2, Vector2);
#[cfg(feature = "vector3")]
horzcat_single!(HorizontalConcatenateV3, Vector3);
#[cfg(feature = "vector4")]
horzcat_single!(HorizontalConcatenateV4, Vector4);
#[cfg(feature = "vectord")]
horzcat_single!(HorizontalConcatenateVD, DVector);

// HorizontalConcatenateSR2 --------------------------------------------------

#[cfg(all(feature = "row_vector2", feature = "row_vector3"))]
#[derive(Debug)]
struct HorizontalConcatenateSR2<T> {
    e0: Ref<T>,
    e1: Ref<RowVector2<T>>,
    out: Ref<RowVector3<T>>,
}
#[cfg(all(feature = "row_vector2", feature = "row_vector3"))]
impl<T> MechFunctionFactory for HorizontalConcatenateSR2<T>
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
        <RowVector3<T> as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
        <RowVector2<T> as FunctionRuntimeType>::REPRESENTATION,
    );

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, arg0, arg1) = invocation.expect_binary()?;
        let e0: Ref<T> = arg0.try_ref()?;
        let e1: Ref<RowVector2<T>> = arg1.try_ref()?;
        let out: Ref<RowVector3<T>> = out.try_ref()?;
        Ok(Box::new(Self { e0, e1, out }))
    }
}
#[cfg(all(feature = "row_vector2", feature = "row_vector3"))]
impl<T> MechFunctionImpl for HorizontalConcatenateSR2<T>
where
    T: Debug + Clone + Sync + Send + PartialEq + 'static,
{
    fn solve_result(&self) -> MResult<()> {
        unsafe {
            let e0_ptr = (*(self.e0.as_ptr())).clone();
            let e1_ptr = (*(self.e1.as_ptr())).clone();
            let out_ptr = &mut *(self.out.as_mut_ptr());
            out_ptr[0] = e0_ptr.clone();
            out_ptr[1] = e1_ptr[0].clone();
            out_ptr[2] = e1_ptr[1].clone();
        };
        Ok(())
    }

    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}
#[cfg(all(feature = "row_vector2", feature = "row_vector3"))]
#[cfg(feature = "semantic-compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateSR2<T>
where
    T: ConstElem + CompileConst + FunctionRuntimeType + CanonicalMatrixElementBacking,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!(
            "HorizontalConcatenateSR2<{}>",
            <T as FunctionRuntimeType>::REPRESENTATION
        );
        compile_binop!(name, self.out, self.e0, self.e1, ctx);
    }
}

// HorizontalConcatenateR2S --------------------------------------------------

#[cfg(all(feature = "row_vector2", feature = "row_vector3"))]
#[derive(Debug)]
struct HorizontalConcatenateR2S<T> {
    e0: Ref<RowVector2<T>>,
    e1: Ref<T>,
    out: Ref<RowVector3<T>>,
}
#[cfg(all(feature = "row_vector2", feature = "row_vector3"))]
impl<T> MechFunctionFactory for HorizontalConcatenateR2S<T>
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
        <RowVector3<T> as FunctionRuntimeType>::REPRESENTATION,
        <RowVector2<T> as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
    );

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, arg0, arg1) = invocation.expect_binary()?;
        let e0: Ref<RowVector2<T>> = arg0.try_ref()?;
        let e1: Ref<T> = arg1.try_ref()?;
        let out: Ref<RowVector3<T>> = out.try_ref()?;
        Ok(Box::new(Self { e0, e1, out }))
    }
}
#[cfg(all(feature = "row_vector2", feature = "row_vector3"))]
impl<T> MechFunctionImpl for HorizontalConcatenateR2S<T>
where
    T: Debug + Clone + Sync + Send + PartialEq + 'static,
{
    fn solve_result(&self) -> MResult<()> {
        unsafe {
            let e0_ptr = (*(self.e0.as_ptr())).clone();
            let out_ptr = &mut *(self.out.as_mut_ptr());
            out_ptr[0] = e0_ptr[0].clone();
            out_ptr[1] = e0_ptr[1].clone();
            out_ptr[2] = self.e1.borrow().clone();
        };
        Ok(())
    }

    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}
#[cfg(all(feature = "row_vector2", feature = "row_vector3"))]
#[cfg(feature = "semantic-compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateR2S<T>
where
    T: ConstElem + CompileConst + FunctionRuntimeType + CanonicalMatrixElementBacking,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!(
            "HorizontalConcatenateR2S<{}>",
            <T as FunctionRuntimeType>::REPRESENTATION
        );
        compile_binop!(name, self.out, self.e0, self.e1, ctx);
    }
}
// HorizontalConcatenateSM1 ---------------------------------------------------

#[cfg(all(feature = "matrix1", feature = "row_vector2"))]
#[derive(Debug)]
struct HorizontalConcatenateSM1<T> {
    e0: Ref<T>,
    e1: Ref<Matrix1<T>>,
    out: Ref<RowVector2<T>>,
}
#[cfg(all(feature = "matrix1", feature = "row_vector2"))]
impl<T> MechFunctionFactory for HorizontalConcatenateSM1<T>
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
        <RowVector2<T> as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
        <Matrix1<T> as FunctionRuntimeType>::REPRESENTATION,
    );

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, arg0, arg1) = invocation.expect_binary()?;
        let e0: Ref<T> = arg0.try_ref()?;
        let e1: Ref<Matrix1<T>> = arg1.try_ref()?;
        let out: Ref<RowVector2<T>> = out.try_ref()?;
        Ok(Box::new(Self { e0, e1, out }))
    }
}
#[cfg(all(feature = "matrix1", feature = "row_vector2"))]
impl<T> MechFunctionImpl for HorizontalConcatenateSM1<T>
where
    T: Debug + Clone + Sync + Send + PartialEq + 'static,
{
    fn solve_result(&self) -> MResult<()> {
        unsafe {
            let e0_val = self.e0.borrow().clone();
            let e1_ptr = (*(self.e1.as_ptr())).clone();
            let out_ptr = &mut *(self.out.as_mut_ptr());
            out_ptr[0] = e0_val;
            out_ptr[1] = e1_ptr[0].clone();
        };
        Ok(())
    }

    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}
#[cfg(all(feature = "matrix1", feature = "row_vector2"))]
#[cfg(feature = "semantic-compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateSM1<T>
where
    T: ConstElem + CompileConst + FunctionRuntimeType + CanonicalMatrixElementBacking,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!(
            "HorizontalConcatenateSM1<{}>",
            <T as FunctionRuntimeType>::REPRESENTATION
        );
        compile_binop!(name, self.out, self.e0, self.e1, ctx);
    }
}

// HorizontalConcatenateM1S ---------------------------------------------------

#[cfg(all(feature = "matrix1", feature = "row_vector2"))]
#[derive(Debug)]
struct HorizontalConcatenateM1S<T> {
    e0: Ref<Matrix1<T>>, // Matrix1
    e1: Ref<T>,          // scalar
    out: Ref<RowVector2<T>>,
}
#[cfg(all(feature = "matrix1", feature = "row_vector2"))]
impl<T> MechFunctionFactory for HorizontalConcatenateM1S<T>
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
        <RowVector2<T> as FunctionRuntimeType>::REPRESENTATION,
        <Matrix1<T> as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
    );

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, arg0, arg1) = invocation.expect_binary()?;
        let e0: Ref<Matrix1<T>> = arg0.try_ref()?;
        let e1: Ref<T> = arg1.try_ref()?;
        let out: Ref<RowVector2<T>> = out.try_ref()?;
        Ok(Box::new(Self { e0, e1, out }))
    }
}
#[cfg(all(feature = "matrix1", feature = "row_vector2"))]
impl<T> MechFunctionImpl for HorizontalConcatenateM1S<T>
where
    T: Debug + Clone + Sync + Send + PartialEq + 'static,
{
    fn solve_result(&self) -> MResult<()> {
        unsafe {
            let e0_ptr = (*(self.e0.as_ptr())).clone();
            let e1_val = self.e1.borrow().clone();
            let out_ptr = &mut *(self.out.as_mut_ptr());
            out_ptr[0] = e0_ptr[0].clone();
            out_ptr[1] = e1_val;
        };
        Ok(())
    }

    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}
#[cfg(all(feature = "matrix1", feature = "row_vector2"))]
#[cfg(feature = "semantic-compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateM1S<T>
where
    T: ConstElem + CompileConst + FunctionRuntimeType + CanonicalMatrixElementBacking,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!(
            "HorizontalConcatenateM1S<{}>",
            <T as FunctionRuntimeType>::REPRESENTATION
        );
        compile_binop!(name, self.out, self.e0, self.e1, ctx);
    }
}

// HorizontalConcatenateSSSM1 -------------------------------------------------

#[cfg(all(feature = "matrix1", feature = "row_vector4"))]
#[derive(Debug)]
struct HorizontalConcatenateSSSM1<T> {
    e0: Ref<T>,
    e1: Ref<T>,
    e2: Ref<T>,
    e3: Ref<Matrix1<T>>,
    out: Ref<RowVector4<T>>,
}
#[cfg(all(feature = "matrix1", feature = "row_vector4"))]
impl<T> MechFunctionFactory for HorizontalConcatenateSSSM1<T>
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
        <RowVector4<T> as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
        <Matrix1<T> as FunctionRuntimeType>::REPRESENTATION,
    );

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, arg0, arg1, arg2, arg3) = invocation.expect_quaternary()?;
        let e0: Ref<T> = arg0.try_ref()?;
        let e1: Ref<T> = arg1.try_ref()?;
        let e2: Ref<T> = arg2.try_ref()?;
        let e3: Ref<Matrix1<T>> = arg3.try_ref()?;
        let out: Ref<RowVector4<T>> = out.try_ref()?;
        Ok(Box::new(Self {
            e0,
            e1,
            e2,
            e3,
            out,
        }))
    }
}
#[cfg(all(feature = "matrix1", feature = "row_vector4"))]
impl<T> MechFunctionImpl for HorizontalConcatenateSSSM1<T>
where
    T: Debug + Clone + Sync + Send + PartialEq + 'static,
{
    fn solve_result(&self) -> MResult<()> {
        unsafe {
            let e0_val = self.e0.borrow().clone();
            let e1_val = self.e1.borrow().clone();
            let e2_val = self.e2.borrow().clone();
            let e3_ptr = (*(self.e3.as_ptr())).clone();
            let out_ptr = &mut *(self.out.as_mut_ptr());
            out_ptr[0] = e0_val;
            out_ptr[1] = e1_val;
            out_ptr[2] = e2_val;
            out_ptr[3] = e3_ptr[0].clone();
        };
        Ok(())
    }

    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}
#[cfg(all(feature = "matrix1", feature = "row_vector4"))]
#[cfg(feature = "semantic-compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateSSSM1<T>
where
    T: ConstElem + CompileConst + FunctionRuntimeType + CanonicalMatrixElementBacking,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!(
            "HorizontalConcatenateSSSM1<{}>",
            <T as FunctionRuntimeType>::REPRESENTATION
        );
        compile_quadop!(name, self.out, self.e0, self.e1, self.e2, self.e3, ctx);
    }
}

// HorizontalConcatenateSSM1S -------------------------------------------------

#[cfg(all(feature = "matrix1", feature = "row_vector4"))]
#[derive(Debug)]
struct HorizontalConcatenateSSM1S<T> {
    e0: Ref<T>,
    e1: Ref<T>,
    e2: Ref<Matrix1<T>>,
    e3: Ref<T>,
    out: Ref<RowVector4<T>>,
}
#[cfg(all(feature = "matrix1", feature = "row_vector4"))]
impl<T> MechFunctionFactory for HorizontalConcatenateSSM1S<T>
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
        <RowVector4<T> as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
        <Matrix1<T> as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
    );

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, arg0, arg1, arg2, arg3) = invocation.expect_quaternary()?;
        let e0: Ref<T> = arg0.try_ref()?;
        let e1: Ref<T> = arg1.try_ref()?;
        let e2: Ref<Matrix1<T>> = arg2.try_ref()?;
        let e3: Ref<T> = arg3.try_ref()?;
        let out: Ref<RowVector4<T>> = out.try_ref()?;
        Ok(Box::new(Self {
            e0,
            e1,
            e2,
            e3,
            out,
        }))
    }
}
#[cfg(all(feature = "matrix1", feature = "row_vector4"))]
impl<T> MechFunctionImpl for HorizontalConcatenateSSM1S<T>
where
    T: Debug + Clone + Sync + Send + PartialEq + 'static,
{
    fn solve_result(&self) -> MResult<()> {
        unsafe {
            let e0_val = self.e0.borrow().clone();
            let e1_val = self.e1.borrow().clone();
            let e2_ptr = (*(self.e2.as_ptr())).clone();
            let e3_val = self.e3.borrow().clone();
            let out_ptr = &mut *(self.out.as_mut_ptr());
            out_ptr[0] = e0_val;
            out_ptr[1] = e1_val;
            out_ptr[2] = e2_ptr[0].clone();
            out_ptr[3] = e3_val;
        };
        Ok(())
    }

    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}
#[cfg(all(feature = "matrix1", feature = "row_vector4"))]
#[cfg(feature = "semantic-compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateSSM1S<T>
where
    T: ConstElem + CompileConst + FunctionRuntimeType + CanonicalMatrixElementBacking,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!(
            "HorizontalConcatenateSSM1S<{}>",
            <T as FunctionRuntimeType>::REPRESENTATION
        );
        compile_quadop!(name, self.out, self.e0, self.e1, self.e2, self.e3, ctx);
    }
}

// HorizontalConcatenateSM1SS -------------------------------------------------

#[cfg(all(feature = "matrix1", feature = "row_vector4"))]
#[derive(Debug)]
struct HorizontalConcatenateSM1SS<T> {
    e0: Ref<T>,
    e1: Ref<Matrix1<T>>,
    e2: Ref<T>,
    e3: Ref<T>,
    out: Ref<RowVector4<T>>,
}
#[cfg(all(feature = "matrix1", feature = "row_vector4"))]
impl<T> MechFunctionFactory for HorizontalConcatenateSM1SS<T>
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
        <RowVector4<T> as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
        <Matrix1<T> as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
    );

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, arg0, arg1, arg2, arg3) = invocation.expect_quaternary()?;
        let e0: Ref<T> = arg0.try_ref()?;
        let e1: Ref<Matrix1<T>> = arg1.try_ref()?;
        let e2: Ref<T> = arg2.try_ref()?;
        let e3: Ref<T> = arg3.try_ref()?;
        let out: Ref<RowVector4<T>> = out.try_ref()?;
        Ok(Box::new(Self {
            e0,
            e1,
            e2,
            e3,
            out,
        }))
    }
}
#[cfg(all(feature = "matrix1", feature = "row_vector4"))]
impl<T> MechFunctionImpl for HorizontalConcatenateSM1SS<T>
where
    T: Debug + Clone + Sync + Send + PartialEq + 'static,
{
    fn solve_result(&self) -> MResult<()> {
        unsafe {
            let e0_val = self.e0.borrow().clone();
            let e1_ptr = (*(self.e1.as_ptr())).clone();
            let e2_val = self.e2.borrow().clone();
            let e3_val = self.e3.borrow().clone();
            let out_ptr = &mut *(self.out.as_mut_ptr());
            out_ptr[0] = e0_val;
            out_ptr[1] = e1_ptr[0].clone();
            out_ptr[2] = e2_val;
            out_ptr[3] = e3_val;
        };
        Ok(())
    }

    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}
#[cfg(all(feature = "matrix1", feature = "row_vector4"))]
#[cfg(feature = "semantic-compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateSM1SS<T>
where
    T: ConstElem + CompileConst + FunctionRuntimeType + CanonicalMatrixElementBacking,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!(
            "HorizontalConcatenateSM1SS<{}>",
            <T as FunctionRuntimeType>::REPRESENTATION
        );
        compile_quadop!(name, self.out, self.e0, self.e1, self.e2, self.e3, ctx);
    }
}

// HorizontalConcatenateM1SSS -------------------------------------------------

#[cfg(all(feature = "matrix1", feature = "row_vector4"))]
#[derive(Debug)]
struct HorizontalConcatenateM1SSS<T> {
    e0: Ref<Matrix1<T>>,
    e1: Ref<T>,
    e2: Ref<T>,
    e3: Ref<T>,
    out: Ref<RowVector4<T>>,
}
#[cfg(all(feature = "matrix1", feature = "row_vector4"))]
impl<T> MechFunctionFactory for HorizontalConcatenateM1SSS<T>
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
        <RowVector4<T> as FunctionRuntimeType>::REPRESENTATION,
        <Matrix1<T> as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
    );

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, arg0, arg1, arg2, arg3) = invocation.expect_quaternary()?;
        let e0: Ref<Matrix1<T>> = arg0.try_ref()?;
        let e1: Ref<T> = arg1.try_ref()?;
        let e2: Ref<T> = arg2.try_ref()?;
        let e3: Ref<T> = arg3.try_ref()?;
        let out: Ref<RowVector4<T>> = out.try_ref()?;
        Ok(Box::new(Self {
            e0,
            e1,
            e2,
            e3,
            out,
        }))
    }
}
#[cfg(all(feature = "matrix1", feature = "row_vector4"))]
impl<T> MechFunctionImpl for HorizontalConcatenateM1SSS<T>
where
    T: Debug + Clone + Sync + Send + PartialEq + 'static,
{
    fn solve_result(&self) -> MResult<()> {
        unsafe {
            let e0_ptr = (*(self.e0.as_ptr())).clone();
            let e1_val = self.e1.borrow().clone();
            let e2_val = self.e2.borrow().clone();
            let e3_val = self.e3.borrow().clone();
            let out_ptr = &mut *(self.out.as_mut_ptr());
            out_ptr[0] = e0_ptr[0].clone();
            out_ptr[1] = e1_val;
            out_ptr[2] = e2_val;
            out_ptr[3] = e3_val;
        };
        Ok(())
    }

    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}
#[cfg(all(feature = "matrix1", feature = "row_vector4"))]
#[cfg(feature = "semantic-compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateM1SSS<T>
where
    T: ConstElem + CompileConst + FunctionRuntimeType + CanonicalMatrixElementBacking,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!(
            "HorizontalConcatenateM1SSS<{}>",
            <T as FunctionRuntimeType>::REPRESENTATION
        );
        compile_quadop!(name, self.out, self.e0, self.e1, self.e2, self.e3, ctx);
    }
}

// HorizontalConcatenateSR3 -------------------------------------------------

#[cfg(all(feature = "row_vector3", feature = "row_vector4"))]
#[derive(Debug)]
struct HorizontalConcatenateSR3<T> {
    e0: Ref<T>,
    e1: Ref<RowVector3<T>>,
    out: Ref<RowVector4<T>>,
}
#[cfg(all(feature = "row_vector3", feature = "row_vector4"))]
impl<T> MechFunctionFactory for HorizontalConcatenateSR3<T>
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
        <RowVector4<T> as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
        <RowVector3<T> as FunctionRuntimeType>::REPRESENTATION,
    );

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, arg0, arg1) = invocation.expect_binary()?;
        let e0: Ref<T> = arg0.try_ref()?;
        let e1: Ref<RowVector3<T>> = arg1.try_ref()?;
        let out: Ref<RowVector4<T>> = out.try_ref()?;
        Ok(Box::new(Self { e0, e1, out }))
    }
}
#[cfg(all(feature = "row_vector3", feature = "row_vector4"))]
impl<T> MechFunctionImpl for HorizontalConcatenateSR3<T>
where
    T: Debug + Clone + Sync + Send + PartialEq + 'static,
{
    fn solve_result(&self) -> MResult<()> {
        unsafe {
            let e0_ptr = (*(self.e0.as_ptr())).clone();
            let e1_ptr = (*(self.e1.as_ptr())).clone();
            let out_ptr = &mut *(self.out.as_mut_ptr());
            out_ptr[0] = e0_ptr.clone();
            out_ptr[1] = e1_ptr[0].clone();
            out_ptr[2] = e1_ptr[1].clone();
            out_ptr[3] = e1_ptr[2].clone();
        };
        Ok(())
    }

    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}
#[cfg(all(feature = "row_vector3", feature = "row_vector4"))]
#[cfg(feature = "semantic-compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateSR3<T>
where
    T: ConstElem + CompileConst + FunctionRuntimeType + CanonicalMatrixElementBacking,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!(
            "HorizontalConcatenateSR3<{}>",
            <T as FunctionRuntimeType>::REPRESENTATION
        );
        compile_binop!(name, self.out, self.e0, self.e1, ctx);
    }
}

// HorizontalConcatenateR3S -------------------------------------------------

#[cfg(all(feature = "row_vector3", feature = "row_vector4"))]
#[derive(Debug)]
struct HorizontalConcatenateR3S<T> {
    e0: Ref<RowVector3<T>>,
    e1: Ref<T>,
    out: Ref<RowVector4<T>>,
}
#[cfg(all(feature = "row_vector3", feature = "row_vector4"))]
impl<T> MechFunctionFactory for HorizontalConcatenateR3S<T>
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
        <RowVector4<T> as FunctionRuntimeType>::REPRESENTATION,
        <RowVector3<T> as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
    );

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, arg0, arg1) = invocation.expect_binary()?;
        let e0: Ref<RowVector3<T>> = arg0.try_ref()?;
        let e1: Ref<T> = arg1.try_ref()?;
        let out: Ref<RowVector4<T>> = out.try_ref()?;
        Ok(Box::new(Self { e0, e1, out }))
    }
}
#[cfg(all(feature = "row_vector3", feature = "row_vector4"))]
impl<T> MechFunctionImpl for HorizontalConcatenateR3S<T>
where
    T: Debug + Clone + Sync + Send + PartialEq + 'static,
{
    fn solve_result(&self) -> MResult<()> {
        unsafe {
            let e0_ptr = (*(self.e0.as_ptr())).clone();
            let e1_ptr = self.e1.borrow().clone();
            let out_ptr = &mut *(self.out.as_mut_ptr());
            out_ptr[0] = e0_ptr[0].clone();
            out_ptr[1] = e0_ptr[1].clone();
            out_ptr[2] = e0_ptr[2].clone();
            out_ptr[3] = e1_ptr.clone();
        };
        Ok(())
    }

    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}
#[cfg(all(feature = "row_vector3", feature = "row_vector4"))]
#[cfg(feature = "semantic-compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateR3S<T>
where
    T: ConstElem + CompileConst + FunctionRuntimeType + CanonicalMatrixElementBacking,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!(
            "HorizontalConcatenateR3S<{}>",
            <T as FunctionRuntimeType>::REPRESENTATION
        );
        compile_binop!(name, self.out, self.e0, self.e1, ctx);
    }
}

// HorizontalConcatenateSSM1 -------------------------------------------------

#[cfg(all(feature = "matrix1", feature = "row_vector3"))]
#[derive(Debug)]
struct HorizontalConcatenateSSM1<T> {
    e0: Ref<T>,          // scalar
    e1: Ref<T>,          // scalar
    e2: Ref<Matrix1<T>>, // Matrix1
    out: Ref<RowVector3<T>>,
}
#[cfg(all(feature = "matrix1", feature = "row_vector3"))]
impl<T> MechFunctionFactory for HorizontalConcatenateSSM1<T>
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
        <RowVector3<T> as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
        <Matrix1<T> as FunctionRuntimeType>::REPRESENTATION,
    );

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, arg0, arg1, arg2) = invocation.expect_ternary()?;
        let e0: Ref<T> = arg0.try_ref()?;
        let e1: Ref<T> = arg1.try_ref()?;
        let e2: Ref<Matrix1<T>> = arg2.try_ref()?;
        let out: Ref<RowVector3<T>> = out.try_ref()?;
        Ok(Box::new(Self { e0, e1, e2, out }))
    }
}
#[cfg(all(feature = "matrix1", feature = "row_vector3"))]
impl<T> MechFunctionImpl for HorizontalConcatenateSSM1<T>
where
    T: Debug + Clone + Sync + Send + PartialEq + 'static,
{
    fn solve_result(&self) -> MResult<()> {
        unsafe {
            let e0_val = self.e0.borrow().clone();
            let e1_val = self.e1.borrow().clone();
            let e2_ptr = (*(self.e2.as_ptr())).clone();
            let out_ptr = &mut *(self.out.as_mut_ptr());
            out_ptr[0] = e0_val;
            out_ptr[1] = e1_val;
            out_ptr[2] = e2_ptr[0].clone();
        };
        Ok(())
    }

    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}
#[cfg(all(feature = "matrix1", feature = "row_vector3"))]
#[cfg(feature = "semantic-compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateSSM1<T>
where
    T: ConstElem + CompileConst + FunctionRuntimeType + CanonicalMatrixElementBacking,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!(
            "HorizontalConcatenateSSM1<{}>",
            <T as FunctionRuntimeType>::REPRESENTATION
        );
        compile_ternop!(name, self.out, self.e0, self.e1, self.e2, ctx);
    }
}

// HorizontalConcatenateSM1S -------------------------------------------------

#[cfg(all(feature = "matrix1", feature = "row_vector3"))]
#[derive(Debug)]
struct HorizontalConcatenateSM1S<T> {
    e0: Ref<T>,          // scalar
    e1: Ref<Matrix1<T>>, // Matrix1
    e2: Ref<T>,          // scalar
    out: Ref<RowVector3<T>>,
}
#[cfg(all(feature = "matrix1", feature = "row_vector3"))]
impl<T> MechFunctionFactory for HorizontalConcatenateSM1S<T>
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
        <RowVector3<T> as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
        <Matrix1<T> as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
    );

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, arg0, arg1, arg2) = invocation.expect_ternary()?;
        let e0: Ref<T> = arg0.try_ref()?;
        let e1: Ref<Matrix1<T>> = arg1.try_ref()?;
        let e2: Ref<T> = arg2.try_ref()?;
        let out: Ref<RowVector3<T>> = out.try_ref()?;
        Ok(Box::new(Self { e0, e1, e2, out }))
    }
}
#[cfg(all(feature = "matrix1", feature = "row_vector3"))]
impl<T> MechFunctionImpl for HorizontalConcatenateSM1S<T>
where
    T: Debug + Clone + Sync + Send + PartialEq + 'static,
{
    fn solve_result(&self) -> MResult<()> {
        unsafe {
            let e0_val = self.e0.borrow().clone();
            let e1_ptr = (*(self.e1.as_ptr())).clone();
            let e2_val = self.e2.borrow().clone();
            let out_ptr = &mut *(self.out.as_mut_ptr());
            out_ptr[0] = e0_val;
            out_ptr[1] = e1_ptr[0].clone();
            out_ptr[2] = e2_val;
        };
        Ok(())
    }

    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}
#[cfg(all(feature = "matrix1", feature = "row_vector3"))]
#[cfg(feature = "semantic-compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateSM1S<T>
where
    T: ConstElem + CompileConst + FunctionRuntimeType + CanonicalMatrixElementBacking,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!(
            "HorizontalConcatenateSM1S<{}>",
            <T as FunctionRuntimeType>::REPRESENTATION
        );
        compile_ternop!(name, self.out, self.e0, self.e1, self.e2, ctx);
    }
}

// HorizontalConcatenateM1SS -------------------------------------------------

#[cfg(all(feature = "matrix1", feature = "row_vector3"))]
#[derive(Debug)]
struct HorizontalConcatenateM1SS<T> {
    e0: Ref<Matrix1<T>>, // Matrix1
    e1: Ref<T>,          // scalar
    e2: Ref<T>,          // scalar
    out: Ref<RowVector3<T>>,
}
#[cfg(all(feature = "matrix1", feature = "row_vector3"))]
impl<T> MechFunctionFactory for HorizontalConcatenateM1SS<T>
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
        <RowVector3<T> as FunctionRuntimeType>::REPRESENTATION,
        <Matrix1<T> as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
    );

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, arg0, arg1, arg2) = invocation.expect_ternary()?;
        let e0: Ref<Matrix1<T>> = arg0.try_ref()?;
        let e1: Ref<T> = arg1.try_ref()?;
        let e2: Ref<T> = arg2.try_ref()?;
        let out: Ref<RowVector3<T>> = out.try_ref()?;
        Ok(Box::new(Self { e0, e1, e2, out }))
    }
}
#[cfg(all(feature = "matrix1", feature = "row_vector3"))]
impl<T> MechFunctionImpl for HorizontalConcatenateM1SS<T>
where
    T: Debug + Clone + Sync + Send + PartialEq + 'static,
{
    fn solve_result(&self) -> MResult<()> {
        unsafe {
            let e0_ptr = (*(self.e0.as_ptr())).clone();
            let e1_val = self.e1.borrow().clone();
            let e2_val = self.e2.borrow().clone();
            let out_ptr = &mut *(self.out.as_mut_ptr());
            out_ptr[0] = e0_ptr[0].clone();
            out_ptr[1] = e1_val;
            out_ptr[2] = e2_val;
        };
        Ok(())
    }

    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}
#[cfg(all(feature = "matrix1", feature = "row_vector3"))]
#[cfg(feature = "semantic-compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateM1SS<T>
where
    T: ConstElem + CompileConst + FunctionRuntimeType + CanonicalMatrixElementBacking,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!(
            "HorizontalConcatenateM1SS<{}>",
            <T as FunctionRuntimeType>::REPRESENTATION
        );
        compile_ternop!(name, self.out, self.e0, self.e1, self.e2, ctx);
    }
}

// HorizontalConcatenateSSR2 -------------------------------------------------

#[cfg(all(feature = "row_vector2", feature = "row_vector4"))]
#[derive(Debug)]
struct HorizontalConcatenateSSR2<T> {
    e0: Ref<T>,
    e1: Ref<T>,
    e2: Ref<RowVector2<T>>,
    out: Ref<RowVector4<T>>,
}
#[cfg(all(feature = "row_vector2", feature = "row_vector4"))]
impl<T> MechFunctionFactory for HorizontalConcatenateSSR2<T>
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
        <RowVector4<T> as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
        <RowVector2<T> as FunctionRuntimeType>::REPRESENTATION,
    );

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, arg0, arg1, arg2) = invocation.expect_ternary()?;
        let e0: Ref<T> = arg0.try_ref()?;
        let e1: Ref<T> = arg1.try_ref()?;
        let e2: Ref<RowVector2<T>> = arg2.try_ref()?;
        let out: Ref<RowVector4<T>> = out.try_ref()?;
        Ok(Box::new(Self { e0, e1, e2, out }))
    }
}
#[cfg(all(feature = "row_vector2", feature = "row_vector4"))]
impl<T> MechFunctionImpl for HorizontalConcatenateSSR2<T>
where
    T: Debug + Clone + Sync + Send + PartialEq + 'static,
{
    fn solve_result(&self) -> MResult<()> {
        unsafe {
            let e0_val = self.e0.borrow().clone();
            let e1_val = self.e1.borrow().clone();
            let e2_ptr = (*(self.e2.as_ptr())).clone();
            let out_ptr = &mut *(self.out.as_mut_ptr());
            out_ptr[0] = e0_val;
            out_ptr[1] = e1_val;
            out_ptr[2] = e2_ptr[0].clone();
            out_ptr[3] = e2_ptr[1].clone();
        };
        Ok(())
    }

    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}
#[cfg(all(feature = "row_vector2", feature = "row_vector4"))]
#[cfg(feature = "semantic-compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateSSR2<T>
where
    T: ConstElem + CompileConst + FunctionRuntimeType + CanonicalMatrixElementBacking,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!(
            "HorizontalConcatenateSSR2<{}>",
            <T as FunctionRuntimeType>::REPRESENTATION
        );
        compile_ternop!(name, self.out, self.e0, self.e1, self.e2, ctx);
    }
}

// HorizontalConcatenateSR2S -------------------------------------------------

#[cfg(all(feature = "row_vector2", feature = "row_vector4"))]
#[derive(Debug)]
struct HorizontalConcatenateSR2S<T> {
    e0: Ref<T>,
    e1: Ref<RowVector2<T>>,
    e2: Ref<T>,
    out: Ref<RowVector4<T>>,
}
#[cfg(all(feature = "row_vector2", feature = "row_vector4"))]
impl<T> MechFunctionFactory for HorizontalConcatenateSR2S<T>
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
        <RowVector4<T> as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
        <RowVector2<T> as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
    );

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, arg0, arg1, arg2) = invocation.expect_ternary()?;
        let e0: Ref<T> = arg0.try_ref()?;
        let e1: Ref<RowVector2<T>> = arg1.try_ref()?;
        let e2: Ref<T> = arg2.try_ref()?;
        let out: Ref<RowVector4<T>> = out.try_ref()?;
        Ok(Box::new(Self { e0, e1, e2, out }))
    }
}
#[cfg(all(feature = "row_vector2", feature = "row_vector4"))]
impl<T> MechFunctionImpl for HorizontalConcatenateSR2S<T>
where
    T: Debug + Clone + Sync + Send + PartialEq + 'static,
{
    fn solve_result(&self) -> MResult<()> {
        unsafe {
            let e0_val = self.e0.borrow().clone();
            let e1_ptr = (*(self.e1.as_ptr())).clone();
            let e2_val = self.e2.borrow().clone();
            let out_ptr = &mut *(self.out.as_mut_ptr());
            out_ptr[0] = e0_val;
            out_ptr[1] = e1_ptr[0].clone();
            out_ptr[2] = e1_ptr[1].clone();
            out_ptr[3] = e2_val;
        };
        Ok(())
    }

    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}
#[cfg(all(feature = "row_vector2", feature = "row_vector4"))]
#[cfg(feature = "semantic-compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateSR2S<T>
where
    T: ConstElem + CompileConst + FunctionRuntimeType + CanonicalMatrixElementBacking,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!(
            "HorizontalConcatenateSR2S<{}>",
            <T as FunctionRuntimeType>::REPRESENTATION
        );
        compile_ternop!(name, self.out, self.e0, self.e1, self.e2, ctx);
    }
}

// HorizontalConcatenateR2SS -------------------------------------------------

#[cfg(all(feature = "row_vector2", feature = "row_vector4"))]
#[derive(Debug)]
struct HorizontalConcatenateR2SS<T> {
    e0: Ref<RowVector2<T>>,
    e1: Ref<T>,
    e2: Ref<T>,
    out: Ref<RowVector4<T>>,
}
#[cfg(all(feature = "row_vector2", feature = "row_vector4"))]
impl<T> MechFunctionFactory for HorizontalConcatenateR2SS<T>
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
        <RowVector4<T> as FunctionRuntimeType>::REPRESENTATION,
        <RowVector2<T> as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
    );

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, arg0, arg1, arg2) = invocation.expect_ternary()?;
        let e0: Ref<RowVector2<T>> = arg0.try_ref()?;
        let e1: Ref<T> = arg1.try_ref()?;
        let e2: Ref<T> = arg2.try_ref()?;
        let out: Ref<RowVector4<T>> = out.try_ref()?;
        Ok(Box::new(Self { e0, e1, e2, out }))
    }
}
#[cfg(all(feature = "row_vector2", feature = "row_vector4"))]
impl<T> MechFunctionImpl for HorizontalConcatenateR2SS<T>
where
    T: Debug + Clone + Sync + Send + PartialEq + 'static,
{
    fn solve_result(&self) -> MResult<()> {
        unsafe {
            let e0_ptr = (*(self.e0.as_ptr())).clone();
            let e1_val = self.e1.borrow().clone();
            let e2_val = self.e2.borrow().clone();
            let out_ptr = &mut *(self.out.as_mut_ptr());
            out_ptr[0] = e0_ptr[0].clone();
            out_ptr[1] = e0_ptr[1].clone();
            out_ptr[2] = e1_val;
            out_ptr[3] = e2_val;
        };
        Ok(())
    }

    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}
#[cfg(all(feature = "row_vector2", feature = "row_vector4"))]
#[cfg(feature = "semantic-compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateR2SS<T>
where
    T: ConstElem + CompileConst + FunctionRuntimeType + CanonicalMatrixElementBacking,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!(
            "HorizontalConcatenateR2SS<{}>",
            <T as FunctionRuntimeType>::REPRESENTATION
        );
        compile_ternop!(name, self.out, self.e0, self.e1, self.e2, ctx);
    }
}

// HorizontalConcatenateM1M1S -------------------------------------------------

#[cfg(all(feature = "matrix1", feature = "row_vector3"))]
#[derive(Debug)]
struct HorizontalConcatenateM1M1S<T> {
    e0: Ref<Matrix1<T>>,
    e1: Ref<Matrix1<T>>,
    e2: Ref<T>,
    out: Ref<RowVector3<T>>,
}
#[cfg(all(feature = "matrix1", feature = "row_vector3"))]
impl<T> MechFunctionFactory for HorizontalConcatenateM1M1S<T>
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
        <RowVector3<T> as FunctionRuntimeType>::REPRESENTATION,
        <Matrix1<T> as FunctionRuntimeType>::REPRESENTATION,
        <Matrix1<T> as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
    );

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, arg0, arg1, arg2) = invocation.expect_ternary()?;
        let e0: Ref<Matrix1<T>> = arg0.try_ref()?;
        let e1: Ref<Matrix1<T>> = arg1.try_ref()?;
        let e2: Ref<T> = arg2.try_ref()?;
        let out: Ref<RowVector3<T>> = out.try_ref()?;
        Ok(Box::new(Self { e0, e1, e2, out }))
    }
}
#[cfg(all(feature = "matrix1", feature = "row_vector3"))]
impl<T> MechFunctionImpl for HorizontalConcatenateM1M1S<T>
where
    T: Debug + Clone + Sync + Send + PartialEq + 'static,
{
    fn solve_result(&self) -> MResult<()> {
        unsafe {
            let e0_ptr = (*(self.e0.as_ptr())).clone();
            let e1_ptr = (*(self.e1.as_ptr())).clone();
            let e2_val = self.e2.borrow().clone();
            let out_ptr = &mut *(self.out.as_mut_ptr());
            out_ptr[0] = e0_ptr[0].clone();
            out_ptr[1] = e1_ptr[0].clone();
            out_ptr[2] = e2_val;
        };
        Ok(())
    }

    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}
#[cfg(all(feature = "matrix1", feature = "row_vector3"))]
#[cfg(feature = "semantic-compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateM1M1S<T>
where
    T: ConstElem + CompileConst + FunctionRuntimeType + CanonicalMatrixElementBacking,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!(
            "HorizontalConcatenateM1M1S<{}>",
            <T as FunctionRuntimeType>::REPRESENTATION
        );
        compile_ternop!(name, self.out, self.e0, self.e1, self.e2, ctx);
    }
}

// HorizontalConcatenateM1M1 -------------------------------------------------

#[cfg(all(feature = "matrix1", feature = "row_vector2"))]
macro_rules! horzcat_m1m1 {
    ($out:expr, $e0:expr, $e1:expr) => {
        $out[0] = $e0[0].clone();
        $out[1] = $e1[0].clone();
    };
}
#[cfg(all(feature = "matrix1", feature = "row_vector2"))]
horzcat_two_args!(
    HorizontalConcatenateM1M1,
    Matrix1,
    Matrix1,
    RowVector2,
    horzcat_m1m1
);

// HorizontalConcatenateM1SM1 -------------------------------------------------

#[cfg(all(feature = "matrix1", feature = "row_vector3"))]
#[derive(Debug)]
struct HorizontalConcatenateM1SM1<T> {
    e0: Ref<Matrix1<T>>,
    e1: Ref<T>,
    e2: Ref<Matrix1<T>>,
    out: Ref<RowVector3<T>>,
}
#[cfg(all(feature = "matrix1", feature = "row_vector3"))]
impl<T> MechFunctionFactory for HorizontalConcatenateM1SM1<T>
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
        <RowVector3<T> as FunctionRuntimeType>::REPRESENTATION,
        <Matrix1<T> as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
        <Matrix1<T> as FunctionRuntimeType>::REPRESENTATION,
    );

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, arg0, arg1, arg2) = invocation.expect_ternary()?;
        let e0: Ref<Matrix1<T>> = arg0.try_ref()?;
        let e1: Ref<T> = arg1.try_ref()?;
        let e2: Ref<Matrix1<T>> = arg2.try_ref()?;
        let out: Ref<RowVector3<T>> = out.try_ref()?;
        Ok(Box::new(Self { e0, e1, e2, out }))
    }
}
#[cfg(all(feature = "matrix1", feature = "row_vector3"))]
impl<T> MechFunctionImpl for HorizontalConcatenateM1SM1<T>
where
    T: Debug + Clone + Sync + Send + PartialEq + 'static,
{
    fn solve_result(&self) -> MResult<()> {
        unsafe {
            let e0_ptr = (*(self.e0.as_ptr())).clone();
            let e1_val = self.e1.borrow().clone();
            let e2_ptr = (*(self.e2.as_ptr())).clone();
            let out_ptr = &mut *(self.out.as_mut_ptr());
            out_ptr[0] = e0_ptr[0].clone();
            out_ptr[1] = e1_val;
            out_ptr[2] = e2_ptr[0].clone();
        };
        Ok(())
    }

    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}
#[cfg(all(feature = "matrix1", feature = "row_vector3"))]
#[cfg(feature = "semantic-compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateM1SM1<T>
where
    T: ConstElem + CompileConst + FunctionRuntimeType + CanonicalMatrixElementBacking,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!(
            "HorizontalConcatenateM1SM1<{}>",
            <T as FunctionRuntimeType>::REPRESENTATION
        );
        compile_ternop!(name, self.out, self.e0, self.e1, self.e2, ctx);
    }
}

// HorizontalConcatenateSM1M1 -------------------------------------------------

#[cfg(all(feature = "matrix1", feature = "row_vector3"))]
#[derive(Debug)]
struct HorizontalConcatenateSM1M1<T> {
    e0: Ref<T>,
    e1: Ref<Matrix1<T>>,
    e2: Ref<Matrix1<T>>,
    out: Ref<RowVector3<T>>,
}
#[cfg(all(feature = "matrix1", feature = "row_vector3"))]
impl<T> MechFunctionFactory for HorizontalConcatenateSM1M1<T>
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
        <RowVector3<T> as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
        <Matrix1<T> as FunctionRuntimeType>::REPRESENTATION,
        <Matrix1<T> as FunctionRuntimeType>::REPRESENTATION,
    );

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, arg0, arg1, arg2) = invocation.expect_ternary()?;
        let e0: Ref<T> = arg0.try_ref()?;
        let e1: Ref<Matrix1<T>> = arg1.try_ref()?;
        let e2: Ref<Matrix1<T>> = arg2.try_ref()?;
        let out: Ref<RowVector3<T>> = out.try_ref()?;
        Ok(Box::new(Self { e0, e1, e2, out }))
    }
}
#[cfg(all(feature = "matrix1", feature = "row_vector3"))]
impl<T> MechFunctionImpl for HorizontalConcatenateSM1M1<T>
where
    T: Debug + Clone + Sync + Send + PartialEq + 'static,
{
    fn solve_result(&self) -> MResult<()> {
        unsafe {
            let e0_val = self.e0.borrow().clone();
            let e1_ptr = (*(self.e1.as_ptr())).clone();
            let e2_ptr = (*(self.e2.as_ptr())).clone();
            let out_ptr = &mut *(self.out.as_mut_ptr());
            out_ptr[0] = e0_val;
            out_ptr[1] = e1_ptr[0].clone();
            out_ptr[2] = e2_ptr[0].clone();
        };
        Ok(())
    }

    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}
#[cfg(all(feature = "matrix1", feature = "row_vector3"))]
#[cfg(feature = "semantic-compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateSM1M1<T>
where
    T: ConstElem + CompileConst + FunctionRuntimeType + CanonicalMatrixElementBacking,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!(
            "HorizontalConcatenateSM1M1<{}>",
            <T as FunctionRuntimeType>::REPRESENTATION
        );
        compile_ternop!(name, self.out, self.e0, self.e1, self.e2, ctx);
    }
}

// HorizontalConcatenateR2R2 -------------------------------------------------

#[cfg(all(feature = "row_vector2", feature = "row_vector4"))]
macro_rules! horzcat_r2r2 {
    ($out:expr, $e0:expr, $e1:expr) => {
        $out[0] = $e0[0].clone();
        $out[1] = $e0[1].clone();
        $out[2] = $e1[0].clone();
        $out[3] = $e1[1].clone();
    };
}
#[cfg(all(feature = "row_vector2", feature = "row_vector4"))]
horzcat_two_args!(
    HorizontalConcatenateR2R2,
    RowVector2,
    RowVector2,
    RowVector4,
    horzcat_r2r2
);

// HorizontalConcatenateM1R3 -------------------------------------------------

#[cfg(all(feature = "matrix1", feature = "row_vector3", feature = "row_vector4"))]
macro_rules! horzcat_m1r3 {
    ($out:expr, $e0:expr, $e1:expr) => {
        $out[0] = $e0[0].clone();
        $out[1] = $e1[0].clone();
        $out[2] = $e1[1].clone();
        $out[3] = $e1[2].clone();
    };
}
#[cfg(all(feature = "matrix1", feature = "row_vector3", feature = "row_vector4"))]
horzcat_two_args!(
    HorizontalConcatenateM1R3,
    Matrix1,
    RowVector3,
    RowVector4,
    horzcat_m1r3
);

// HorizontalConcatenateR3M1 -------------------------------------------------

#[cfg(all(feature = "matrix1", feature = "row_vector3", feature = "row_vector4"))]
macro_rules! horzcat_r3m1 {
    ($out:expr, $e0:expr, $e1:expr) => {
        $out[0] = $e0[0].clone();
        $out[1] = $e0[1].clone();
        $out[2] = $e0[2].clone();
        $out[3] = $e1[0].clone();
    };
}
#[cfg(all(feature = "matrix1", feature = "row_vector3", feature = "row_vector4"))]
horzcat_two_args!(
    HorizontalConcatenateR3M1,
    RowVector3,
    Matrix1,
    RowVector4,
    horzcat_r3m1
);

// HorizontalConcatenateSM1R2 -------------------------------------------------

#[cfg(all(feature = "matrix1", feature = "row_vector2", feature = "row_vector4"))]
#[derive(Debug)]
struct HorizontalConcatenateSM1R2<T> {
    e0: Ref<T>,
    e1: Ref<Matrix1<T>>,
    e2: Ref<RowVector2<T>>,
    out: Ref<RowVector4<T>>,
}
#[cfg(all(feature = "matrix1", feature = "row_vector2", feature = "row_vector4"))]
impl<T> MechFunctionFactory for HorizontalConcatenateSM1R2<T>
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
        <RowVector4<T> as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
        <Matrix1<T> as FunctionRuntimeType>::REPRESENTATION,
        <RowVector2<T> as FunctionRuntimeType>::REPRESENTATION,
    );

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, arg0, arg1, arg2) = invocation.expect_ternary()?;
        let e0: Ref<T> = arg0.try_ref()?;
        let e1: Ref<Matrix1<T>> = arg1.try_ref()?;
        let e2: Ref<RowVector2<T>> = arg2.try_ref()?;
        let out: Ref<RowVector4<T>> = out.try_ref()?;
        Ok(Box::new(Self { e0, e1, e2, out }))
    }
}
#[cfg(all(feature = "matrix1", feature = "row_vector2", feature = "row_vector4"))]
impl<T> MechFunctionImpl for HorizontalConcatenateSM1R2<T>
where
    T: Debug + Clone + Sync + Send + PartialEq + 'static,
{
    fn solve_result(&self) -> MResult<()> {
        unsafe {
            let e0_val = self.e0.borrow().clone();
            let e1_ptr = (*(self.e1.as_ptr())).clone();
            let e2_ptr = (*(self.e2.as_ptr())).clone();
            let out_ptr = &mut *(self.out.as_mut_ptr());
            out_ptr[0] = e0_val;
            out_ptr[1] = e1_ptr[0].clone();
            out_ptr[2] = e2_ptr[0].clone();
            out_ptr[3] = e2_ptr[1].clone();
        };
        Ok(())
    }

    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}
#[cfg(all(feature = "matrix1", feature = "row_vector2", feature = "row_vector4"))]
#[cfg(feature = "semantic-compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateSM1R2<T>
where
    T: ConstElem + CompileConst + FunctionRuntimeType + CanonicalMatrixElementBacking,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!(
            "HorizontalConcatenateSM1R2<{}>",
            <T as FunctionRuntimeType>::REPRESENTATION
        );
        compile_ternop!(name, self.out, self.e0, self.e1, self.e2, ctx);
    }
}

// HorizontalConcatenateM1SR2 -------------------------------------------------

#[cfg(all(feature = "matrix1", feature = "row_vector2", feature = "row_vector4"))]
#[derive(Debug)]
struct HorizontalConcatenateM1SR2<T> {
    e0: Ref<Matrix1<T>>,
    e1: Ref<T>,
    e2: Ref<RowVector2<T>>,
    out: Ref<RowVector4<T>>,
}
#[cfg(all(feature = "matrix1", feature = "row_vector2", feature = "row_vector4"))]
impl<T> MechFunctionFactory for HorizontalConcatenateM1SR2<T>
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
        <RowVector4<T> as FunctionRuntimeType>::REPRESENTATION,
        <Matrix1<T> as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
        <RowVector2<T> as FunctionRuntimeType>::REPRESENTATION,
    );

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, arg0, arg1, arg2) = invocation.expect_ternary()?;
        let e0: Ref<Matrix1<T>> = arg0.try_ref()?;
        let e1: Ref<T> = arg1.try_ref()?;
        let e2: Ref<RowVector2<T>> = arg2.try_ref()?;
        let out: Ref<RowVector4<T>> = out.try_ref()?;
        Ok(Box::new(Self { e0, e1, e2, out }))
    }
}
#[cfg(all(feature = "matrix1", feature = "row_vector2", feature = "row_vector4"))]
impl<T> MechFunctionImpl for HorizontalConcatenateM1SR2<T>
where
    T: Debug + Clone + Sync + Send + PartialEq + 'static,
{
    fn solve_result(&self) -> MResult<()> {
        unsafe {
            let e0_ptr = (*(self.e0.as_ptr())).clone();
            let e1_val = self.e1.borrow().clone();
            let e2_ptr = (*(self.e2.as_ptr())).clone();
            let out_ptr = &mut *(self.out.as_mut_ptr());
            out_ptr[0] = e0_ptr[0].clone();
            out_ptr[1] = e1_val;
            out_ptr[2] = e2_ptr[0].clone();
            out_ptr[3] = e2_ptr[1].clone();
        };
        Ok(())
    }

    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}
#[cfg(all(feature = "matrix1", feature = "row_vector2", feature = "row_vector4"))]
#[cfg(feature = "semantic-compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateM1SR2<T>
where
    T: ConstElem + CompileConst + FunctionRuntimeType + CanonicalMatrixElementBacking,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!(
            "HorizontalConcatenateM1SR2<{}>",
            <T as FunctionRuntimeType>::REPRESENTATION
        );
        compile_ternop!(name, self.out, self.e0, self.e1, self.e2, ctx);
    }
}

// HorizontalConcatenateSM1SM1 -------------------------------------------------

#[cfg(all(feature = "row_vector4", feature = "matrix1"))]
#[derive(Debug)]
struct HorizontalConcatenateSM1SM1<T> {
    e0: Ref<T>,
    e1: Ref<Matrix1<T>>,
    e2: Ref<T>,
    e3: Ref<Matrix1<T>>,
    out: Ref<RowVector4<T>>,
}
#[cfg(all(feature = "row_vector4", feature = "matrix1"))]
impl<T> MechFunctionFactory for HorizontalConcatenateSM1SM1<T>
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
        <RowVector4<T> as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
        <Matrix1<T> as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
        <Matrix1<T> as FunctionRuntimeType>::REPRESENTATION,
    );

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, arg0, arg1, arg2, arg3) = invocation.expect_quaternary()?;
        let e0: Ref<T> = arg0.try_ref()?;
        let e1: Ref<Matrix1<T>> = arg1.try_ref()?;
        let e2: Ref<T> = arg2.try_ref()?;
        let e3: Ref<Matrix1<T>> = arg3.try_ref()?;
        let out: Ref<RowVector4<T>> = out.try_ref()?;
        Ok(Box::new(Self {
            e0,
            e1,
            e2,
            e3,
            out,
        }))
    }
}
#[cfg(all(feature = "row_vector4", feature = "matrix1"))]
impl<T> MechFunctionImpl for HorizontalConcatenateSM1SM1<T>
where
    T: Debug + Clone + Sync + Send + PartialEq + 'static,
{
    fn solve_result(&self) -> MResult<()> {
        unsafe {
            let e0_val = self.e0.borrow().clone();
            let e1_ptr = (*(self.e1.as_ptr())).clone();
            let e2_val = self.e2.borrow().clone();
            let e3_ptr = (*(self.e3.as_ptr())).clone();
            let out_ptr = &mut *(self.out.as_mut_ptr());
            out_ptr[0] = e0_val;
            out_ptr[1] = e1_ptr[0].clone();
            out_ptr[2] = e2_val;
            out_ptr[3] = e3_ptr[0].clone();
        };
        Ok(())
    }

    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}
#[cfg(all(feature = "row_vector4", feature = "matrix1"))]
#[cfg(feature = "semantic-compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateSM1SM1<T>
where
    T: ConstElem + CompileConst + FunctionRuntimeType + CanonicalMatrixElementBacking,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!(
            "HorizontalConcatenateSM1SM1<{}>",
            <T as FunctionRuntimeType>::REPRESENTATION
        );
        compile_quadop!(name, self.out, self.e0, self.e1, self.e2, self.e3, ctx);
    }
}

// HorizontalConcatenateM1R2S -------------------------------------------------

#[cfg(all(feature = "row_vector4", feature = "row_vector2", feature = "matrix1"))]
#[derive(Debug)]
struct HorizontalConcatenateM1R2S<T> {
    e0: Ref<Matrix1<T>>,
    e1: Ref<RowVector2<T>>,
    e2: Ref<T>,
    out: Ref<RowVector4<T>>,
}
#[cfg(all(feature = "row_vector4", feature = "row_vector2", feature = "matrix1"))]
impl<T> MechFunctionFactory for HorizontalConcatenateM1R2S<T>
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
        <RowVector4<T> as FunctionRuntimeType>::REPRESENTATION,
        <Matrix1<T> as FunctionRuntimeType>::REPRESENTATION,
        <RowVector2<T> as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
    );

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, arg0, arg1, arg2) = invocation.expect_ternary()?;
        let e0: Ref<Matrix1<T>> = arg0.try_ref()?;
        let e1: Ref<RowVector2<T>> = arg1.try_ref()?;
        let e2: Ref<T> = arg2.try_ref()?;
        let out: Ref<RowVector4<T>> = out.try_ref()?;
        Ok(Box::new(Self { e0, e1, e2, out }))
    }
}
#[cfg(all(feature = "row_vector4", feature = "row_vector2", feature = "matrix1"))]
impl<T> MechFunctionImpl for HorizontalConcatenateM1R2S<T>
where
    T: Debug + Clone + Sync + Send + PartialEq + 'static,
{
    fn solve_result(&self) -> MResult<()> {
        unsafe {
            let e0_ptr = (*(self.e0.as_ptr())).clone();
            let e1_ptr = (*(self.e1.as_ptr())).clone();
            let e2_val = self.e2.borrow().clone();
            let out_ptr = &mut *(self.out.as_mut_ptr());
            out_ptr[0] = e0_ptr[0].clone();
            out_ptr[1] = e1_ptr[0].clone();
            out_ptr[2] = e1_ptr[1].clone();
            out_ptr[3] = e2_val;
        };
        Ok(())
    }

    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}
#[cfg(all(feature = "row_vector4", feature = "row_vector2", feature = "matrix1"))]
#[cfg(feature = "semantic-compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateM1R2S<T>
where
    T: ConstElem + CompileConst + FunctionRuntimeType + CanonicalMatrixElementBacking,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!(
            "HorizontalConcatenateM1R2S<{}>",
            <T as FunctionRuntimeType>::REPRESENTATION
        );
        compile_ternop!(name, self.out, self.e0, self.e1, self.e2, ctx);
    }
}

// HorizontalConcatenateR2M1S -------------------------------------------------

#[cfg(all(feature = "row_vector4", feature = "row_vector2", feature = "matrix1"))]
#[derive(Debug)]
struct HorizontalConcatenateR2M1S<T> {
    e0: Ref<RowVector2<T>>,
    e1: Ref<Matrix1<T>>,
    e2: Ref<T>,
    out: Ref<RowVector4<T>>,
}
#[cfg(all(feature = "row_vector4", feature = "row_vector2", feature = "matrix1"))]
impl<T> MechFunctionFactory for HorizontalConcatenateR2M1S<T>
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
        <RowVector4<T> as FunctionRuntimeType>::REPRESENTATION,
        <RowVector2<T> as FunctionRuntimeType>::REPRESENTATION,
        <Matrix1<T> as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
    );

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, arg0, arg1, arg2) = invocation.expect_ternary()?;
        let e0: Ref<RowVector2<T>> = arg0.try_ref()?;
        let e1: Ref<Matrix1<T>> = arg1.try_ref()?;
        let e2: Ref<T> = arg2.try_ref()?;
        let out: Ref<RowVector4<T>> = out.try_ref()?;
        Ok(Box::new(Self { e0, e1, e2, out }))
    }
}
#[cfg(all(feature = "row_vector4", feature = "row_vector2", feature = "matrix1"))]
impl<T> MechFunctionImpl for HorizontalConcatenateR2M1S<T>
where
    T: Debug + Clone + Sync + Send + PartialEq + 'static,
{
    fn solve_result(&self) -> MResult<()> {
        unsafe {
            let e0_ptr = (*(self.e0.as_ptr())).clone();
            let e1_ptr = (*(self.e1.as_ptr())).clone();
            let e2_val = self.e2.borrow().clone();
            let out_ptr = &mut *(self.out.as_mut_ptr());
            out_ptr[0] = e0_ptr[0].clone();
            out_ptr[1] = e0_ptr[1].clone();
            out_ptr[2] = e1_ptr[0].clone();
            out_ptr[3] = e2_val;
        };
        Ok(())
    }

    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}
#[cfg(all(feature = "row_vector4", feature = "row_vector2", feature = "matrix1"))]
#[cfg(feature = "semantic-compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateR2M1S<T>
where
    T: ConstElem + CompileConst + FunctionRuntimeType + CanonicalMatrixElementBacking,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!(
            "HorizontalConcatenateR2M1S<{}>",
            <T as FunctionRuntimeType>::REPRESENTATION
        );
        compile_ternop!(name, self.out, self.e0, self.e1, self.e2, ctx);
    }
}

// HorizontalConcatenateR2SM1 -------------------------------------------------

#[cfg(all(feature = "row_vector4", feature = "row_vector2", feature = "matrix1"))]
#[derive(Debug)]
struct HorizontalConcatenateR2SM1<T> {
    e0: Ref<RowVector2<T>>,
    e1: Ref<T>,
    e2: Ref<Matrix1<T>>,
    out: Ref<RowVector4<T>>,
}
#[cfg(all(feature = "row_vector4", feature = "row_vector2", feature = "matrix1"))]
impl<T> MechFunctionFactory for HorizontalConcatenateR2SM1<T>
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
        <RowVector4<T> as FunctionRuntimeType>::REPRESENTATION,
        <RowVector2<T> as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
        <Matrix1<T> as FunctionRuntimeType>::REPRESENTATION,
    );

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, arg0, arg1, arg2) = invocation.expect_ternary()?;
        let e0: Ref<RowVector2<T>> = arg0.try_ref()?;
        let e1: Ref<T> = arg1.try_ref()?;
        let e2: Ref<Matrix1<T>> = arg2.try_ref()?;
        let out: Ref<RowVector4<T>> = out.try_ref()?;
        Ok(Box::new(Self { e0, e1, e2, out }))
    }
}
#[cfg(all(feature = "row_vector4", feature = "row_vector2", feature = "matrix1"))]
impl<T> MechFunctionImpl for HorizontalConcatenateR2SM1<T>
where
    T: Debug + Clone + Sync + Send + PartialEq + 'static,
{
    fn solve_result(&self) -> MResult<()> {
        unsafe {
            let e0_ptr = (*(self.e0.as_ptr())).clone();
            let e1_val = self.e1.borrow().clone();
            let e2_ptr = (*(self.e2.as_ptr())).clone();
            let out_ptr = &mut *(self.out.as_mut_ptr());
            out_ptr[0] = e0_ptr[0].clone();
            out_ptr[1] = e0_ptr[1].clone();
            out_ptr[2] = e1_val;
            out_ptr[3] = e2_ptr[0].clone();
        };
        Ok(())
    }

    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}
#[cfg(all(feature = "row_vector4", feature = "row_vector2", feature = "matrix1"))]
#[cfg(feature = "semantic-compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateR2SM1<T>
where
    T: ConstElem + CompileConst + FunctionRuntimeType + CanonicalMatrixElementBacking,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!(
            "HorizontalConcatenateR2SM1<{}>",
            <T as FunctionRuntimeType>::REPRESENTATION
        );
        compile_ternop!(name, self.out, self.e0, self.e1, self.e2, ctx);
    }
}

// HorizontalConcatenateSR2M1 -------------------------------------------------

#[cfg(all(feature = "row_vector4", feature = "row_vector2", feature = "matrix1"))]
#[derive(Debug)]
struct HorizontalConcatenateSR2M1<T> {
    e0: Ref<T>,
    e1: Ref<RowVector2<T>>,
    e2: Ref<Matrix1<T>>,
    out: Ref<RowVector4<T>>,
}
#[cfg(all(feature = "row_vector4", feature = "row_vector2", feature = "matrix1"))]
impl<T> MechFunctionFactory for HorizontalConcatenateSR2M1<T>
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
        <RowVector4<T> as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
        <RowVector2<T> as FunctionRuntimeType>::REPRESENTATION,
        <Matrix1<T> as FunctionRuntimeType>::REPRESENTATION,
    );

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, arg0, arg1, arg2) = invocation.expect_ternary()?;
        let e0: Ref<T> = arg0.try_ref()?;
        let e1: Ref<RowVector2<T>> = arg1.try_ref()?;
        let e2: Ref<Matrix1<T>> = arg2.try_ref()?;
        let out: Ref<RowVector4<T>> = out.try_ref()?;
        Ok(Box::new(Self { e0, e1, e2, out }))
    }
}
#[cfg(all(feature = "row_vector4", feature = "row_vector2", feature = "matrix1"))]
impl<T> MechFunctionImpl for HorizontalConcatenateSR2M1<T>
where
    T: Debug + Clone + Sync + Send + PartialEq + 'static,
{
    fn solve_result(&self) -> MResult<()> {
        unsafe {
            let e0_val = self.e0.borrow().clone();
            let e1_ptr = (*(self.e1.as_ptr())).clone();
            let e2_ptr = (*(self.e2.as_ptr())).clone();
            let out_ptr = &mut *(self.out.as_mut_ptr());
            out_ptr[0] = e0_val;
            out_ptr[1] = e1_ptr[0].clone();
            out_ptr[2] = e1_ptr[1].clone();
            out_ptr[3] = e2_ptr[0].clone();
        };
        Ok(())
    }

    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}
#[cfg(all(feature = "row_vector4", feature = "row_vector2", feature = "matrix1"))]
#[cfg(feature = "semantic-compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateSR2M1<T>
where
    T: ConstElem + CompileConst + FunctionRuntimeType + CanonicalMatrixElementBacking,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!(
            "HorizontalConcatenateSR2M1<{}>",
            <T as FunctionRuntimeType>::REPRESENTATION
        );
        compile_ternop!(name, self.out, self.e0, self.e1, self.e2, ctx);
    }
}

// HorizontalConcatenateSSM1M1 ------------------------------------------------

#[cfg(all(feature = "row_vector4", feature = "matrix1"))]
#[derive(Debug)]
struct HorizontalConcatenateSSM1M1<T> {
    e0: Ref<T>,
    e1: Ref<T>,
    e2: Ref<Matrix1<T>>,
    e3: Ref<Matrix1<T>>,
    out: Ref<RowVector4<T>>,
}
#[cfg(all(feature = "row_vector4", feature = "matrix1"))]
impl<T> MechFunctionFactory for HorizontalConcatenateSSM1M1<T>
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
        <RowVector4<T> as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
        <Matrix1<T> as FunctionRuntimeType>::REPRESENTATION,
        <Matrix1<T> as FunctionRuntimeType>::REPRESENTATION,
    );

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, arg0, arg1, arg2, arg3) = invocation.expect_quaternary()?;
        let e0: Ref<T> = arg0.try_ref()?;
        let e1: Ref<T> = arg1.try_ref()?;
        let e2: Ref<Matrix1<T>> = arg2.try_ref()?;
        let e3: Ref<Matrix1<T>> = arg3.try_ref()?;
        let out: Ref<RowVector4<T>> = out.try_ref()?;
        Ok(Box::new(Self {
            e0,
            e1,
            e2,
            e3,
            out,
        }))
    }
}
#[cfg(all(feature = "row_vector4", feature = "matrix1"))]
impl<T> MechFunctionImpl for HorizontalConcatenateSSM1M1<T>
where
    T: Debug + Clone + Sync + Send + PartialEq + 'static,
{
    fn solve_result(&self) -> MResult<()> {
        unsafe {
            let e0_val = self.e0.borrow().clone();
            let e1_val = self.e1.borrow().clone();
            let e2_ptr = (*(self.e2.as_ptr())).clone();
            let e3_ptr = (*(self.e3.as_ptr())).clone();
            let out_ptr = &mut *(self.out.as_mut_ptr());
            out_ptr[0] = e0_val;
            out_ptr[1] = e1_val;
            out_ptr[2] = e2_ptr[0].clone();
            out_ptr[3] = e3_ptr[0].clone();
        };
        Ok(())
    }

    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}
#[cfg(all(feature = "row_vector4", feature = "matrix1"))]
#[cfg(feature = "semantic-compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateSSM1M1<T>
where
    T: ConstElem + CompileConst + FunctionRuntimeType + CanonicalMatrixElementBacking,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!(
            "HorizontalConcatenateSSM1M1<{}>",
            <T as FunctionRuntimeType>::REPRESENTATION
        );
        compile_quadop!(name, self.out, self.e0, self.e1, self.e2, self.e3, ctx);
    }
}

// HorizontalConcatenateM1M1SS ------------------------------------------------

#[cfg(all(feature = "row_vector4", feature = "matrix1"))]
#[derive(Debug)]
struct HorizontalConcatenateM1M1SS<T> {
    e0: Ref<Matrix1<T>>,
    e1: Ref<Matrix1<T>>,
    e2: Ref<T>,
    e3: Ref<T>,
    out: Ref<RowVector4<T>>,
}
#[cfg(all(feature = "row_vector4", feature = "matrix1"))]
impl<T> MechFunctionFactory for HorizontalConcatenateM1M1SS<T>
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
        <RowVector4<T> as FunctionRuntimeType>::REPRESENTATION,
        <Matrix1<T> as FunctionRuntimeType>::REPRESENTATION,
        <Matrix1<T> as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
    );

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, arg0, arg1, arg2, arg3) = invocation.expect_quaternary()?;
        let e0: Ref<Matrix1<T>> = arg0.try_ref()?;
        let e1: Ref<Matrix1<T>> = arg1.try_ref()?;
        let e2: Ref<T> = arg2.try_ref()?;
        let e3: Ref<T> = arg3.try_ref()?;
        let out: Ref<RowVector4<T>> = out.try_ref()?;
        Ok(Box::new(Self {
            e0,
            e1,
            e2,
            e3,
            out,
        }))
    }
}
#[cfg(all(feature = "row_vector4", feature = "matrix1"))]
impl<T> MechFunctionImpl for HorizontalConcatenateM1M1SS<T>
where
    T: Debug + Clone + Sync + Send + PartialEq + 'static,
{
    fn solve_result(&self) -> MResult<()> {
        unsafe {
            let e0_ptr = (*(self.e0.as_ptr())).clone();
            let e1_ptr = (*(self.e1.as_ptr())).clone();
            let e2_val = self.e2.borrow().clone();
            let e3_val = self.e3.borrow().clone();
            let out_ptr = &mut *(self.out.as_mut_ptr());
            out_ptr[0] = e0_ptr[0].clone();
            out_ptr[1] = e1_ptr[0].clone();
            out_ptr[2] = e2_val;
            out_ptr[3] = e3_val;
        };
        Ok(())
    }

    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}
#[cfg(all(feature = "row_vector4", feature = "matrix1"))]
#[cfg(feature = "semantic-compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateM1M1SS<T>
where
    T: ConstElem + CompileConst + FunctionRuntimeType + CanonicalMatrixElementBacking,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!(
            "HorizontalConcatenateM1M1SS<{}>",
            <T as FunctionRuntimeType>::REPRESENTATION
        );
        compile_quadop!(name, self.out, self.e0, self.e1, self.e2, self.e3, ctx);
    }
}

// HorizontalConcatenateSM1M1S ------------------------------------------------

#[cfg(all(feature = "row_vector4", feature = "matrix1"))]
#[derive(Debug)]
struct HorizontalConcatenateSM1M1S<T> {
    e0: Ref<T>,
    e1: Ref<Matrix1<T>>,
    e2: Ref<Matrix1<T>>,
    e3: Ref<T>,
    out: Ref<RowVector4<T>>,
}
#[cfg(all(feature = "row_vector4", feature = "matrix1"))]
impl<T> MechFunctionFactory for HorizontalConcatenateSM1M1S<T>
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
        <RowVector4<T> as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
        <Matrix1<T> as FunctionRuntimeType>::REPRESENTATION,
        <Matrix1<T> as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
    );

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, arg0, arg1, arg2, arg3) = invocation.expect_quaternary()?;
        let e0: Ref<T> = arg0.try_ref()?;
        let e1: Ref<Matrix1<T>> = arg1.try_ref()?;
        let e2: Ref<Matrix1<T>> = arg2.try_ref()?;
        let e3: Ref<T> = arg3.try_ref()?;
        let out: Ref<RowVector4<T>> = out.try_ref()?;
        Ok(Box::new(Self {
            e0,
            e1,
            e2,
            e3,
            out,
        }))
    }
}
#[cfg(all(feature = "row_vector4", feature = "matrix1"))]
impl<T> MechFunctionImpl for HorizontalConcatenateSM1M1S<T>
where
    T: Debug + Clone + Sync + Send + PartialEq + 'static,
{
    fn solve_result(&self) -> MResult<()> {
        unsafe {
            let e0_val = self.e0.borrow().clone();
            let e1_ptr = (*(self.e1.as_ptr())).clone();
            let e2_ptr = (*(self.e2.as_ptr())).clone();
            let e3_val = self.e3.borrow().clone();
            let out_ptr = &mut *(self.out.as_mut_ptr());
            out_ptr[0] = e0_val;
            out_ptr[1] = e1_ptr[0].clone();
            out_ptr[2] = e2_ptr[0].clone();
            out_ptr[3] = e3_val;
        };
        Ok(())
    }

    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}
#[cfg(all(feature = "row_vector4", feature = "matrix1"))]
#[cfg(feature = "semantic-compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateSM1M1S<T>
where
    T: ConstElem + CompileConst + FunctionRuntimeType + CanonicalMatrixElementBacking,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!(
            "HorizontalConcatenateSM1M1S<{}>",
            <T as FunctionRuntimeType>::REPRESENTATION
        );
        compile_quadop!(name, self.out, self.e0, self.e1, self.e2, self.e3, ctx);
    }
}

// HorizontalConcatenateM1SSM1 ------------------------------------------------

#[cfg(all(feature = "row_vector4", feature = "matrix1"))]
#[derive(Debug)]
struct HorizontalConcatenateM1SSM1<T> {
    e0: Ref<Matrix1<T>>,
    e1: Ref<T>,
    e2: Ref<T>,
    e3: Ref<Matrix1<T>>,
    out: Ref<RowVector4<T>>,
}
#[cfg(all(feature = "row_vector4", feature = "matrix1"))]
impl<T> MechFunctionFactory for HorizontalConcatenateM1SSM1<T>
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
        <RowVector4<T> as FunctionRuntimeType>::REPRESENTATION,
        <Matrix1<T> as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
        <Matrix1<T> as FunctionRuntimeType>::REPRESENTATION,
    );

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, arg0, arg1, arg2, arg3) = invocation.expect_quaternary()?;
        let e0: Ref<Matrix1<T>> = arg0.try_ref()?;
        let e1: Ref<T> = arg1.try_ref()?;
        let e2: Ref<T> = arg2.try_ref()?;
        let e3: Ref<Matrix1<T>> = arg3.try_ref()?;
        let out: Ref<RowVector4<T>> = out.try_ref()?;
        Ok(Box::new(Self {
            e0,
            e1,
            e2,
            e3,
            out,
        }))
    }
}
#[cfg(all(feature = "row_vector4", feature = "matrix1"))]
impl<T> MechFunctionImpl for HorizontalConcatenateM1SSM1<T>
where
    T: Debug + Clone + Sync + Send + PartialEq + 'static,
{
    fn solve_result(&self) -> MResult<()> {
        unsafe {
            let e0_ptr = (*(self.e0.as_ptr())).clone();
            let e1_val = self.e1.borrow().clone();
            let e2_val = self.e2.borrow().clone();
            let e3_ptr = (*(self.e3.as_ptr())).clone();
            let out_ptr = &mut *(self.out.as_mut_ptr());
            out_ptr[0] = e0_ptr[0].clone();
            out_ptr[1] = e1_val;
            out_ptr[2] = e2_val;
            out_ptr[3] = e3_ptr[0].clone();
        };
        Ok(())
    }

    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}
#[cfg(all(feature = "row_vector4", feature = "matrix1"))]
#[cfg(feature = "semantic-compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateM1SSM1<T>
where
    T: ConstElem + CompileConst + FunctionRuntimeType + CanonicalMatrixElementBacking,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!(
            "HorizontalConcatenateM1SSM1<{}>",
            <T as FunctionRuntimeType>::REPRESENTATION
        );
        compile_quadop!(name, self.out, self.e0, self.e1, self.e2, self.e3, ctx);
    }
}

// HorizontalConcatenateM1SM1S ------------------------------------------------

#[cfg(all(feature = "row_vector4", feature = "matrix1"))]
#[derive(Debug)]
struct HorizontalConcatenateM1SM1S<T> {
    e0: Ref<Matrix1<T>>,
    e1: Ref<T>,
    e2: Ref<Matrix1<T>>,
    e3: Ref<T>,
    out: Ref<RowVector4<T>>,
}
#[cfg(all(feature = "row_vector4", feature = "matrix1"))]
impl<T> MechFunctionFactory for HorizontalConcatenateM1SM1S<T>
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
        <RowVector4<T> as FunctionRuntimeType>::REPRESENTATION,
        <Matrix1<T> as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
        <Matrix1<T> as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
    );

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, arg0, arg1, arg2, arg3) = invocation.expect_quaternary()?;
        let e0: Ref<Matrix1<T>> = arg0.try_ref()?;
        let e1: Ref<T> = arg1.try_ref()?;
        let e2: Ref<Matrix1<T>> = arg2.try_ref()?;
        let e3: Ref<T> = arg3.try_ref()?;
        let out: Ref<RowVector4<T>> = out.try_ref()?;
        Ok(Box::new(Self {
            e0,
            e1,
            e2,
            e3,
            out,
        }))
    }
}
#[cfg(all(feature = "row_vector4", feature = "matrix1"))]
impl<T> MechFunctionImpl for HorizontalConcatenateM1SM1S<T>
where
    T: Debug + Clone + Sync + Send + PartialEq + 'static,
{
    fn solve_result(&self) -> MResult<()> {
        unsafe {
            let e0_ptr = (*(self.e0.as_ptr())).clone();
            let e1_val = self.e1.borrow().clone();
            let e2_ptr = (*(self.e2.as_ptr())).clone();
            let e3_val = self.e3.borrow().clone();
            let out_ptr = &mut *(self.out.as_mut_ptr());
            out_ptr[0] = e0_ptr[0].clone();
            out_ptr[1] = e1_val;
            out_ptr[2] = e2_ptr[0].clone();
            out_ptr[3] = e3_val;
        };
        Ok(())
    }

    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}
#[cfg(all(feature = "row_vector4", feature = "matrix1"))]
#[cfg(feature = "semantic-compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateM1SM1S<T>
where
    T: ConstElem + CompileConst + FunctionRuntimeType + CanonicalMatrixElementBacking,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!(
            "HorizontalConcatenateM1SM1S<{}>",
            <T as FunctionRuntimeType>::REPRESENTATION
        );
        compile_quadop!(name, self.out, self.e0, self.e1, self.e2, self.e3, ctx);
    }
}

// HorizontalConcatenateM1R2 --------------------------------------------------

#[cfg(all(feature = "row_vector3", feature = "matrix1", feature = "row_vector2"))]
macro_rules! horzcat_m1r2 {
    ($out:expr, $e0:expr, $e1:expr) => {
        $out[0] = $e0[0].clone();
        $out[1] = $e1[0].clone();
        $out[2] = $e1[1].clone();
    };
}
#[cfg(all(feature = "row_vector3", feature = "matrix1", feature = "row_vector2"))]
horzcat_two_args!(
    HorizontalConcatenateM1R2,
    Matrix1,
    RowVector2,
    RowVector3,
    horzcat_m1r2
);

// HorizontalConcatenateR2M1 --------------------------------------------------

#[cfg(all(feature = "row_vector3", feature = "matrix1", feature = "row_vector2"))]
macro_rules! horzcat_r2m1 {
    ($out:expr, $e0:expr, $e1:expr) => {
        $out[0] = $e0[0].clone();
        $out[1] = $e0[1].clone();
        $out[2] = $e1[0].clone();
    };
}
#[cfg(all(feature = "row_vector3", feature = "matrix1", feature = "row_vector2"))]
horzcat_two_args!(
    HorizontalConcatenateR2M1,
    RowVector2,
    Matrix1,
    RowVector3,
    horzcat_r2m1
);

// HorizontalConcatenateM1M1M1 ------------------------------------------------

#[cfg(all(feature = "row_vector3", feature = "matrix1"))]
macro_rules! horzcat_m1m1m1 {
    ($out:expr, $e0:expr,$e1:expr,$e2:expr) => {
        $out[0] = $e0[0].clone();
        $out[1] = $e1[0].clone();
        $out[2] = $e2[0].clone();
    };
}
#[cfg(all(feature = "row_vector3", feature = "matrix1"))]
horzcat_three_args!(
    HorizontalConcatenateM1M1M1,
    Matrix1,
    Matrix1,
    Matrix1,
    RowVector3,
    horzcat_m1m1m1
);

// HorizontalConcatenateM1M1R2 ------------------------------------------------

#[cfg(all(feature = "row_vector4", feature = "matrix1", feature = "row_vector2"))]
macro_rules! horzcat_m1m1r2 {
    ($out:expr, $e0:expr, $e1:expr, $e2:expr) => {
        $out[0] = $e0[0].clone();
        $out[1] = $e1[0].clone();
        $out[2] = $e2[0].clone();
        $out[3] = $e2[1].clone();
    };
}
#[cfg(all(feature = "row_vector4", feature = "matrix1", feature = "row_vector2"))]
horzcat_three_args!(
    HorizontalConcatenateM1M1R2,
    Matrix1,
    Matrix1,
    RowVector2,
    RowVector4,
    horzcat_m1m1r2
);

// HorizontalConcatenateM1R2M1 ------------------------------------------------

#[cfg(all(feature = "row_vector4", feature = "matrix1", feature = "row_vector2"))]
macro_rules! horzcat_m1r2m1 {
    ($out:expr, $e0:expr, $e1:expr, $e2:expr) => {
        $out[0] = $e0[0].clone();
        $out[1] = $e1[0].clone();
        $out[2] = $e1[1].clone();
        $out[3] = $e2[0].clone();
    };
}
#[cfg(all(feature = "row_vector4", feature = "matrix1", feature = "row_vector2"))]
horzcat_three_args!(
    HorizontalConcatenateM1R2M1,
    Matrix1,
    RowVector2,
    Matrix1,
    RowVector4,
    horzcat_m1r2m1
);

#[cfg(all(feature = "row_vector4", feature = "matrix1", feature = "row_vector2"))]
macro_rules! horzcat_r2m1m1 {
    ($out:expr, $e0:expr, $e1:expr, $e2:expr) => {
        $out[0] = $e0[0].clone();
        $out[1] = $e0[1].clone();
        $out[2] = $e1[0].clone();
        $out[3] = $e2[0].clone();
    };
}
#[cfg(all(feature = "row_vector4", feature = "matrix1", feature = "row_vector2"))]
horzcat_three_args!(
    HorizontalConcatenateR2M1M1,
    RowVector2,
    Matrix1,
    Matrix1,
    RowVector4,
    horzcat_r2m1m1
);

// HorizontalConcatenateSM1M1M1 -----------------------------------------------

#[cfg(all(feature = "row_vector4", feature = "matrix1"))]
#[derive(Debug)]
struct HorizontalConcatenateSM1M1M1<T> {
    e0: Ref<T>,
    e1: Ref<Matrix1<T>>,
    e2: Ref<Matrix1<T>>,
    e3: Ref<Matrix1<T>>,
    out: Ref<RowVector4<T>>,
}
#[cfg(all(feature = "row_vector4", feature = "matrix1"))]
impl<T> MechFunctionFactory for HorizontalConcatenateSM1M1M1<T>
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
        <RowVector4<T> as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
        <Matrix1<T> as FunctionRuntimeType>::REPRESENTATION,
        <Matrix1<T> as FunctionRuntimeType>::REPRESENTATION,
        <Matrix1<T> as FunctionRuntimeType>::REPRESENTATION,
    );

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, arg0, arg1, arg2, arg3) = invocation.expect_quaternary()?;
        let e0: Ref<T> = arg0.try_ref()?;
        let e1: Ref<Matrix1<T>> = arg1.try_ref()?;
        let e2: Ref<Matrix1<T>> = arg2.try_ref()?;
        let e3: Ref<Matrix1<T>> = arg3.try_ref()?;
        let out: Ref<RowVector4<T>> = out.try_ref()?;
        Ok(Box::new(Self {
            e0,
            e1,
            e2,
            e3,
            out,
        }))
    }
}
#[cfg(all(feature = "row_vector4", feature = "matrix1"))]
impl<T> MechFunctionImpl for HorizontalConcatenateSM1M1M1<T>
where
    T: Debug + Clone + Sync + Send + PartialEq + 'static,
{
    fn solve_result(&self) -> MResult<()> {
        unsafe {
            let e0_val = self.e0.borrow().clone();
            let e1_ptr = (*(self.e1.as_ptr())).clone();
            let e2_ptr = (*(self.e2.as_ptr())).clone();
            let e3_ptr = (*(self.e3.as_ptr())).clone();
            let out_ptr = &mut *(self.out.as_mut_ptr());
            out_ptr[0] = e0_val;
            out_ptr[1] = e1_ptr[0].clone();
            out_ptr[2] = e2_ptr[0].clone();
            out_ptr[3] = e3_ptr[0].clone();
        };
        Ok(())
    }

    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}
#[cfg(all(feature = "row_vector4", feature = "matrix1"))]
#[cfg(feature = "semantic-compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateSM1M1M1<T>
where
    T: ConstElem + CompileConst + FunctionRuntimeType + CanonicalMatrixElementBacking,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!(
            "HorizontalConcatenateSM1M1M1<{}>",
            <T as FunctionRuntimeType>::REPRESENTATION
        );
        compile_quadop!(name, self.out, self.e0, self.e1, self.e2, self.e3, ctx);
    }
}

// HorizontalConcatenateM1SM1M1 -----------------------------------------------

#[cfg(all(feature = "row_vector4", feature = "matrix1"))]
#[derive(Debug)]
struct HorizontalConcatenateM1SM1M1<T> {
    e0: Ref<Matrix1<T>>,
    e1: Ref<T>,
    e2: Ref<Matrix1<T>>,
    e3: Ref<Matrix1<T>>,
    out: Ref<RowVector4<T>>,
}
#[cfg(all(feature = "row_vector4", feature = "matrix1"))]
impl<T> MechFunctionFactory for HorizontalConcatenateM1SM1M1<T>
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
        <RowVector4<T> as FunctionRuntimeType>::REPRESENTATION,
        <Matrix1<T> as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
        <Matrix1<T> as FunctionRuntimeType>::REPRESENTATION,
        <Matrix1<T> as FunctionRuntimeType>::REPRESENTATION,
    );

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, arg0, arg1, arg2, arg3) = invocation.expect_quaternary()?;
        let e0: Ref<Matrix1<T>> = arg0.try_ref()?;
        let e1: Ref<T> = arg1.try_ref()?;
        let e2: Ref<Matrix1<T>> = arg2.try_ref()?;
        let e3: Ref<Matrix1<T>> = arg3.try_ref()?;
        let out: Ref<RowVector4<T>> = out.try_ref()?;
        Ok(Box::new(Self {
            e0,
            e1,
            e2,
            e3,
            out,
        }))
    }
}
#[cfg(all(feature = "row_vector4", feature = "matrix1"))]
impl<T> MechFunctionImpl for HorizontalConcatenateM1SM1M1<T>
where
    T: Debug + Clone + Sync + Send + PartialEq + 'static,
{
    fn solve_result(&self) -> MResult<()> {
        unsafe {
            let e0_ptr = (*(self.e0.as_ptr())).clone();
            let e1_val = self.e1.borrow().clone();
            let e2_ptr = (*(self.e2.as_ptr())).clone();
            let e3_ptr = (*(self.e3.as_ptr())).clone();
            let out_ptr = &mut *(self.out.as_mut_ptr());
            out_ptr[0] = e0_ptr[0].clone();
            out_ptr[1] = e1_val;
            out_ptr[2] = e2_ptr[0].clone();
            out_ptr[3] = e3_ptr[0].clone();
        };
        Ok(())
    }

    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}
#[cfg(all(feature = "row_vector4", feature = "matrix1"))]
#[cfg(feature = "semantic-compiler")]
impl<T> MechFunctionCompiler for HorizontalConcatenateM1SM1M1<T>
where
    T: ConstElem + CompileConst + FunctionRuntimeType + CanonicalMatrixElementBacking,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!(
            "HorizontalConcatenateM1SM1M1<{}>",
            <T as FunctionRuntimeType>::REPRESENTATION
        );
        compile_quadop!(name, self.out, self.e0, self.e1, self.e2, self.e3, ctx);
    }
}

// HorizontalConcatenateM1M1SM1 -----------------------------------------------

#[cfg(all(feature = "row_vector4", feature = "matrix1"))]
#[derive(Debug)]
struct HorizontalConcatenateM1M1SM1<T> {
    e0: Ref<Matrix1<T>>,
    e1: Ref<Matrix1<T>>,
    e2: Ref<T>,
    e3: Ref<Matrix1<T>>,
    out: Ref<RowVector4<T>>,
}
#[cfg(all(feature = "row_vector4", feature = "matrix1"))]
impl<T> MechFunctionFactory for HorizontalConcatenateM1M1SM1<T>
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
        <RowVector4<T> as FunctionRuntimeType>::REPRESENTATION,
        <Matrix1<T> as FunctionRuntimeType>::REPRESENTATION,
        <Matrix1<T> as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
        <Matrix1<T> as FunctionRuntimeType>::REPRESENTATION,
    );

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, arg0, arg1, arg2, arg3) = invocation.expect_quaternary()?;
        let e0: Ref<Matrix1<T>> = arg0.try_ref()?;
        let e1: Ref<Matrix1<T>> = arg1.try_ref()?;
        let e2: Ref<T> = arg2.try_ref()?;
        let e3: Ref<Matrix1<T>> = arg3.try_ref()?;
        let out: Ref<RowVector4<T>> = out.try_ref()?;
        Ok(Box::new(Self {
            e0,
            e1,
            e2,
            e3,
            out,
        }))
    }
}
#[cfg(all(feature = "row_vector4", feature = "matrix1"))]
impl<T> MechFunctionImpl for HorizontalConcatenateM1M1SM1<T>
where
    T: Debug + Clone + Sync + Send + PartialEq + 'static,
{
    fn solve_result(&self) -> MResult<()> {
        unsafe {
            let e0_ptr = (*(self.e0.as_ptr())).clone();
            let e1_ptr = (*(self.e1.as_ptr())).clone();
            let e2_val = self.e2.borrow().clone();
            let e3_ptr = (*(self.e3.as_ptr())).clone();
            let out_ptr = &mut *(self.out.as_mut_ptr());
            out_ptr[0] = e0_ptr[0].clone();
            out_ptr[1] = e1_ptr[0].clone();
            out_ptr[2] = e2_val;
            out_ptr[3] = e3_ptr[0].clone();
        };
        Ok(())
    }

    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}
#[cfg(all(
    feature = "row_vector4",
    feature = "matrix1",
    feature = "semantic-compiler"
))]
impl<T> MechFunctionCompiler for HorizontalConcatenateM1M1SM1<T>
where
    T: ConstElem + CompileConst + FunctionRuntimeType + CanonicalMatrixElementBacking,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!(
            "HorizontalConcatenateM1M1SM1<{}>",
            <T as FunctionRuntimeType>::REPRESENTATION
        );
        compile_quadop!(name, self.out, self.e0, self.e1, self.e2, self.e3, ctx);
    }
}

// HorizontalConcatenateM1M1M1S -----------------------------------------------

#[cfg(all(feature = "row_vector4", feature = "matrix1"))]
#[derive(Debug)]
struct HorizontalConcatenateM1M1M1S<T> {
    e0: Ref<Matrix1<T>>,
    e1: Ref<Matrix1<T>>,
    e2: Ref<Matrix1<T>>,
    e3: Ref<T>,
    out: Ref<RowVector4<T>>,
}
#[cfg(all(feature = "row_vector4", feature = "matrix1"))]
impl<T> MechFunctionFactory for HorizontalConcatenateM1M1M1S<T>
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
        <RowVector4<T> as FunctionRuntimeType>::REPRESENTATION,
        <Matrix1<T> as FunctionRuntimeType>::REPRESENTATION,
        <Matrix1<T> as FunctionRuntimeType>::REPRESENTATION,
        <Matrix1<T> as FunctionRuntimeType>::REPRESENTATION,
        <T as FunctionRuntimeType>::REPRESENTATION,
    );

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, arg0, arg1, arg2, arg3) = invocation.expect_quaternary()?;
        let e0: Ref<Matrix1<T>> = arg0.try_ref()?;
        let e1: Ref<Matrix1<T>> = arg1.try_ref()?;
        let e2: Ref<Matrix1<T>> = arg2.try_ref()?;
        let e3: Ref<T> = arg3.try_ref()?;
        let out: Ref<RowVector4<T>> = out.try_ref()?;
        Ok(Box::new(Self {
            e0,
            e1,
            e2,
            e3,
            out,
        }))
    }
}
#[cfg(all(feature = "row_vector4", feature = "matrix1"))]
impl<T> MechFunctionImpl for HorizontalConcatenateM1M1M1S<T>
where
    T: Debug + Clone + Sync + Send + PartialEq + 'static,
{
    fn solve_result(&self) -> MResult<()> {
        unsafe {
            let e0_ptr = (*(self.e0.as_ptr())).clone();
            let e1_ptr = (*(self.e1.as_ptr())).clone();
            let e2_ptr = (*(self.e2.as_ptr())).clone();
            let e3_val = self.e3.borrow().clone();
            let out_ptr = &mut *(self.out.as_mut_ptr());
            out_ptr[0] = e0_ptr[0].clone();
            out_ptr[1] = e1_ptr[0].clone();
            out_ptr[2] = e2_ptr[0].clone();
            out_ptr[3] = e3_val;
        };
        Ok(())
    }

    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}
#[cfg(all(
    feature = "row_vector4",
    feature = "matrix1",
    feature = "semantic-compiler"
))]
impl<T> MechFunctionCompiler for HorizontalConcatenateM1M1M1S<T>
where
    T: ConstElem + CompileConst + FunctionRuntimeType + CanonicalMatrixElementBacking,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!(
            "HorizontalConcatenateM1M1M1S<{}>",
            <T as FunctionRuntimeType>::REPRESENTATION
        );
        compile_quadop!(name, self.out, self.e0, self.e1, self.e2, self.e3, ctx);
    }
}

// HorizontalConcatenateM1M1M1S -----------------------------------------------

#[cfg(all(feature = "row_vector4", feature = "matrix1"))]
#[derive(Debug)]
struct HorizontalConcatenateM1M1M1M1<T> {
    e0: Ref<Matrix1<T>>,
    e1: Ref<Matrix1<T>>,
    e2: Ref<Matrix1<T>>,
    e3: Ref<Matrix1<T>>,
    out: Ref<RowVector4<T>>,
}
#[cfg(all(feature = "row_vector4", feature = "matrix1"))]
impl<T> MechFunctionFactory for HorizontalConcatenateM1M1M1M1<T>
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
        <RowVector4<T> as FunctionRuntimeType>::REPRESENTATION,
        <Matrix1<T> as FunctionRuntimeType>::REPRESENTATION,
        <Matrix1<T> as FunctionRuntimeType>::REPRESENTATION,
        <Matrix1<T> as FunctionRuntimeType>::REPRESENTATION,
        <Matrix1<T> as FunctionRuntimeType>::REPRESENTATION,
    );

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, arg0, arg1, arg2, arg3) = invocation.expect_quaternary()?;
        let e0: Ref<Matrix1<T>> = arg0.try_ref()?;
        let e1: Ref<Matrix1<T>> = arg1.try_ref()?;
        let e2: Ref<Matrix1<T>> = arg2.try_ref()?;
        let e3: Ref<Matrix1<T>> = arg3.try_ref()?;
        let out: Ref<RowVector4<T>> = out.try_ref()?;
        Ok(Box::new(Self {
            e0,
            e1,
            e2,
            e3,
            out,
        }))
    }
}
#[cfg(all(feature = "row_vector4", feature = "matrix1"))]
impl<T> MechFunctionImpl for HorizontalConcatenateM1M1M1M1<T>
where
    T: Debug + Clone + Sync + Send + PartialEq + 'static,
{
    fn solve_result(&self) -> MResult<()> {
        unsafe {
            let e0_ptr = (*(self.e0.as_ptr())).clone();
            let e1_ptr = (*(self.e1.as_ptr())).clone();
            let e2_ptr = (*(self.e2.as_ptr())).clone();
            let e3_ptr = (*(self.e3.as_ptr())).clone();
            let out_ptr = &mut *(self.out.as_mut_ptr());
            out_ptr[0] = e0_ptr[0].clone();
            out_ptr[1] = e1_ptr[0].clone();
            out_ptr[2] = e2_ptr[0].clone();
            out_ptr[3] = e3_ptr[0].clone();
        };
        Ok(())
    }

    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}
#[cfg(all(
    feature = "row_vector4",
    feature = "matrix1",
    feature = "semantic-compiler"
))]
impl<T> MechFunctionCompiler for HorizontalConcatenateM1M1M1M1<T>
where
    T: ConstElem + CompileConst + FunctionRuntimeType + CanonicalMatrixElementBacking,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!(
            "HorizontalConcatenateM1M1M1M1<{}>",
            <T as FunctionRuntimeType>::REPRESENTATION
        );
        compile_quadop!(name, self.out, self.e0, self.e1, self.e2, self.e3, ctx);
    }
}

// HorizontalConcatenateV2V2 -------------------------------------------------

#[cfg(all(feature = "vector2", feature = "matrix2"))]
macro_rules! horzcat_v2v2 {
    ($out:expr, $e0:expr, $e1:expr) => {
        $out[0] = $e0[0].clone();
        $out[1] = $e0[1].clone();
        $out[2] = $e1[0].clone();
        $out[3] = $e1[1].clone();
    };
}
#[cfg(all(feature = "vector2", feature = "matrix2"))]
horzcat_two_args!(
    HorizontalConcatenateV2V2,
    Vector2,
    Vector2,
    Matrix2,
    horzcat_v2v2
);

#[cfg(all(feature = "vector3", feature = "matrix3x2"))]
macro_rules! horzcat_v3v3 {
    ($out:expr, $e0:expr, $e1:expr) => {
        $out[0] = $e0[0].clone();
        $out[1] = $e0[1].clone();
        $out[2] = $e0[2].clone();
        $out[3] = $e1[0].clone();
        $out[4] = $e1[1].clone();
        $out[5] = $e1[2].clone();
    };
}
#[cfg(all(feature = "vector3", feature = "matrix3x2"))]
horzcat_two_args!(
    HorizontalConcatenateV3V3,
    Vector3,
    Vector3,
    Matrix3x2,
    horzcat_v3v3
);

// HorizontalConcatenateV2M2 --------------------------------------------------

#[cfg(all(feature = "vector2", feature = "matrix2", feature = "matrix2x3"))]
macro_rules! horzcat_v2m2 {
    ($out:expr, $e0:expr, $e1:expr) => {
        $out[0] = $e0[0].clone();
        $out[1] = $e0[1].clone();
        $out[2] = $e1[0].clone();
        $out[3] = $e1[1].clone();
        $out[4] = $e1[2].clone();
        $out[5] = $e1[3].clone();
    };
}
#[cfg(all(feature = "vector2", feature = "matrix2", feature = "matrix2x3"))]
horzcat_two_args!(
    HorizontalConcatenateV2M2,
    Vector2,
    Matrix2,
    Matrix2x3,
    horzcat_v2m2
);

// HorizontalConcatenateM2V2 --------------------------------------------------

#[cfg(all(feature = "vector2", feature = "matrix2", feature = "matrix2x3"))]
macro_rules! horzcat_m2v2 {
    ($out:expr, $e0:expr, $e1:expr) => {
        $out[0] = $e0[0].clone();
        $out[1] = $e0[1].clone();
        $out[2] = $e0[2].clone();
        $out[3] = $e0[3].clone();
        $out[4] = $e1[0].clone();
        $out[5] = $e1[1].clone();
    };
}
#[cfg(all(feature = "vector2", feature = "matrix2", feature = "matrix2x3"))]
horzcat_two_args!(
    HorizontalConcatenateM2V2,
    Matrix2,
    Vector2,
    Matrix2x3,
    horzcat_m2v2
);

// HorizontalConcatenateM3x2V3 ------------------------------------------------

#[cfg(all(feature = "vector3", feature = "matrix3x2", feature = "matrix3"))]
macro_rules! horzcat_m3x2v3 {
    ($out:expr, $e0:expr, $e1:expr) => {
        $out[0] = $e0[0].clone();
        $out[1] = $e0[1].clone();
        $out[2] = $e0[2].clone();
        $out[3] = $e0[3].clone();
        $out[4] = $e0[4].clone();
        $out[5] = $e0[5].clone();
        $out[6] = $e1[0].clone();
        $out[7] = $e1[1].clone();
        $out[8] = $e1[2].clone();
    };
}
#[cfg(all(feature = "vector3", feature = "matrix3x2", feature = "matrix3"))]
horzcat_two_args!(
    HorizontalConcatenateM3x2V3,
    Matrix3x2,
    Vector3,
    Matrix3,
    horzcat_m3x2v3
);

// HorizontalConcatenateV3M3x2 ------------------------------------------------

#[cfg(all(feature = "vector3", feature = "matrix3x2", feature = "matrix3"))]
macro_rules! horzcat_v3m3x2 {
    ($out:expr, $e0:expr, $e1:expr) => {
        $out[0] = $e0[0].clone();
        $out[1] = $e0[1].clone();
        $out[2] = $e0[2].clone();
        $out[3] = $e1[0].clone();
        $out[4] = $e1[1].clone();
        $out[5] = $e1[2].clone();
        $out[6] = $e1[3].clone();
        $out[7] = $e1[4].clone();
        $out[8] = $e1[5].clone();
    };
}
#[cfg(all(feature = "vector3", feature = "matrix3x2", feature = "matrix3"))]
horzcat_two_args!(
    HorizontalConcatenateV3M3x2,
    Vector3,
    Matrix3x2,
    Matrix3,
    horzcat_v3m3x2
);

// HorizontalConcatenateV4V4 --------------------------------------------------

#[cfg(all(feature = "matrixd", feature = "matrix4", feature = "vector4"))]
macro_rules! horzcat_v4md {
    ($out:expr, $e0:expr, $e1:expr) => {
        $out[0] = $e0[0].clone();
        $out[1] = $e0[1].clone();
        $out[2] = $e0[2].clone();
        $out[3] = $e0[3].clone();
        let offset = 4;
        for i in 0..$e1.len() {
            $out[i + offset] = $e1[i].clone();
        }
    };
}
#[cfg(all(feature = "matrixd", feature = "matrix4", feature = "vector4"))]
horzcat_two_args!(
    HorizontalConcatenateV4MD,
    Vector4,
    DMatrix,
    Matrix4,
    horzcat_v4md
);

// HorizontalConcatenateMDV4 --------------------------------------------------

#[cfg(all(feature = "matrixd", feature = "matrix4", feature = "vector4"))]
macro_rules! horzcat_mdv4 {
    ($out:expr, $e0:expr, $e1:expr) => {
        let e0_len = $e0.len();
        for i in 0..e0_len {
            $out[i] = $e0[i].clone();
        }
        let offset = e0_len;
        $out[offset] = $e1[0].clone();
        $out[offset + 1] = $e1[1].clone();
        $out[offset + 2] = $e1[2].clone();
        $out[offset + 3] = $e1[3].clone();
    };
}
#[cfg(all(feature = "matrixd", feature = "matrix4", feature = "vector4"))]
horzcat_two_args!(
    HorizontalConcatenateMDV4,
    DMatrix,
    Vector4,
    Matrix4,
    horzcat_mdv4
);

// HorizontalConcatenateMDV4 --------------------------------------------------

#[cfg(all(feature = "matrixd", feature = "matrix4"))]
macro_rules! horzcat_mdmd {
    ($out:expr, $e0:expr, $e1:expr) => {
        let e0_len = $e0.len();
        for i in 0..e0_len {
            $out[i] = $e0[i].clone();
        }
        let offset = e0_len;
        for i in 0..$e1.len() {
            $out[i + offset] = $e1[i].clone();
        }
    };
}
#[cfg(all(feature = "matrixd", feature = "matrix4"))]
horzcat_two_args!(
    HorizontalConcatenateMDMD,
    DMatrix,
    DMatrix,
    Matrix4,
    horzcat_mdmd
);

// HorizontalConcatenateMDMDMD ------------------------------------------------

#[cfg(any(
    all(feature = "vector2", feature = "matrix2x3"),
    all(feature = "vector3", feature = "matrix3"),
    all(feature = "matrixd", feature = "vector4", feature = "matrix4")
))]
macro_rules! horzcat_mdmdmd {
    ($out:expr, $e0:expr, $e1:expr, $e2:expr) => {
        let e0_len = $e0.len();
        for i in 0..e0_len {
            $out[i] = $e0[i].clone();
        }
        let offset = e0_len;
        for i in 0..$e1.len() {
            $out[i + offset] = $e1[i].clone();
        }
        let offset = offset + $e1.len();
        for i in 0..$e2.len() {
            $out[i + offset] = $e2[i].clone();
        }
    };
}

// HorizontalConcatenateV2V2V2 ------------------------------------------------

#[cfg(all(feature = "vector2", feature = "matrix2x3"))]
horzcat_three_args!(
    HorizontalConcatenateV2V2V2,
    Vector2,
    Vector2,
    Vector2,
    Matrix2x3,
    horzcat_mdmdmd
);

// HorizontalConcatenateV3V3V3 ------------------------------------------------

#[cfg(all(feature = "vector3", feature = "matrix3"))]
horzcat_three_args!(
    HorizontalConcatenateV3V3V3,
    Vector3,
    Vector3,
    Vector3,
    Matrix3,
    horzcat_mdmdmd
);

// HorizontalConcatenateV2V2MD ------------------------------------------------

#[cfg(all(feature = "matrixd", feature = "vector4", feature = "matrix4"))]
horzcat_three_args!(
    HorizontalConcatenateV4V4MD,
    Vector4,
    Vector4,
    DMatrix,
    Matrix4,
    horzcat_mdmdmd
);

// HorizontalConcatenateV2MDV2 ------------------------------------------------

#[cfg(all(feature = "matrixd", feature = "vector4", feature = "matrix4"))]
horzcat_three_args!(
    HorizontalConcatenateV4MDV4,
    Vector4,
    DMatrix,
    Vector4,
    Matrix4,
    horzcat_mdmdmd
);

// HorizontalConcatenateMDV2V2 ------------------------------------------------

#[cfg(all(feature = "matrixd", feature = "vector4", feature = "matrix4"))]
horzcat_three_args!(
    HorizontalConcatenateMDV4V4,
    DMatrix,
    Vector4,
    Vector4,
    Matrix4,
    horzcat_mdmdmd
);

// HorizontalConcatenateV4V4V4V4 ------------------------------------------------

#[cfg(all(feature = "matrix4", feature = "vector4"))]
macro_rules! horzcat_mdmdmdmd {
    ($out:expr, $e0:expr, $e1:expr, $e2:expr, $e3:expr) => {
        let e0_len = $e0.len();
        for i in 0..e0_len {
            $out[i] = $e0[i].clone();
        }
        let offset = e0_len;
        for i in 0..$e1.len() {
            $out[i + offset] = $e1[i].clone();
        }
        let offset = offset + $e1.len();
        for i in 0..$e2.len() {
            $out[i + offset] = $e2[i].clone();
        }
        let offset = offset + $e2.len();
        for i in 0..$e3.len() {
            $out[i + offset] = $e3[i].clone();
        }
    };
}

#[cfg(all(feature = "matrix4", feature = "vector4"))]
horzcat_four_args!(
    HorizontalConcatenateV4V4V4V4,
    Vector4,
    Vector4,
    Vector4,
    Vector4,
    Matrix4,
    horzcat_mdmdmdmd
);

#[cfg(any(
    feature = "matrix1",
    feature = "matrix2",
    feature = "matrix3",
    feature = "matrix4",
    feature = "matrix2x3",
    feature = "matrix3x2",
    feature = "row_vector2",
    feature = "row_vector3",
    feature = "row_vector4",
    feature = "vector2",
    feature = "vector3",
    feature = "vector4"
))]
macro_rules! install_horzcat_factories {
    ($builder:ident, $factory:ident) => {
        install_horzcat_linked_factories!($builder, $factory)?;
    };
}

macro_rules! for_each_horzcat_scalar {
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

#[cfg(feature = "row_vectord")]
fn validate_nullary_horizontal_concatenation_canonical(
    output: &ValueCell,
    inputs: &[ValueCell],
) -> MResult<()> {
    let contract = "horizontal_concatenation_nullary";
    if !inputs.is_empty() {
        return Err(function_shape_contract_violation(
            contract,
            format!("expected no inputs, found {}", inputs.len()),
        ));
    }
    match output.closed_schema_body()? {
        SchemaBody::Matrix { .. } => Ok(()),
        _ => Err(function_shape_contract_violation(
            contract,
            "output must be matrix-backed",
        )),
    }
}

macro_rules! declare_horzcat_scalar {
    (HorizontalConcatenateRD, [$($feature:literal),+]; $token:ident, $scalar:ty, $name:literal, $cargo:literal) => {
        paste! {
            mech_core::declare_native_runtime_factory! {
                cfg: all(feature = "matrix_horzcat", feature = $cargo, $(feature = $feature),+),
                registration: [<register_horizontal_concatenate_r_d_ $token>],
                installer: [<install_horizontal_concatenate_r_d_ $token>],
                name: concat!("HorizontalConcatenateRD<", $name, ">"),
                factory_type: HorizontalConcatenateRD<$scalar>,
                contract: RuntimeFunctionContract::canonical_custom(
                    "horizontal_concatenation_nullary",
                    RuntimeOutputAliasPolicy::DisallowInputAlias,
                    validate_nullary_horizontal_concatenation_canonical,
                ),
                compiler_family: mech_core::RuntimeFamilyId::from_name(concat!("HorizontalConcatenateRD<", $name, ">")),
                package: "mech-engine", crate_name: "mech_engine",
                installer_path: concat!("mech_engine::__mech_native::install_horizontal_concatenate_r_d_", stringify!($token)),
                extra_cargo_features: ["matrix_horzcat"],
            }
        }
    };
    ($factory:ident, [$($feature:literal),+]; $token:ident, $scalar:ty, $name:literal, $cargo:literal) => {
        paste! {
            mech_core::declare_native_runtime_factory! {
                cfg: all(feature = "matrix_horzcat", feature = $cargo, $(feature = $feature),+),
                registration: [<register_ $factory:snake _ $token>],
                installer: [<install_ $factory:snake _ $token>],
                name: concat!(stringify!($factory), "<", $name, ">"),
                factory_type: $factory<$scalar>,
                contract: RuntimeFunctionContract::horizontal_concatenation(RuntimeOutputAliasPolicy::DisallowInputAlias),
                compiler_family: mech_core::RuntimeFamilyId::from_name(concat!(stringify!($factory), "<", $name, ">")),
                package: "mech-engine", crate_name: "mech_engine",
                installer_path: concat!("mech_engine::__mech_native::install_", stringify!([<$factory:snake _ $token>])),
                extra_cargo_features: ["matrix_horzcat"],
            }
        }
    };
}

macro_rules! declare_horzcat_family {
    ($factory:ident, [$($feature:literal),+]) => {
        for_each_horzcat_scalar!(declare_horzcat_scalar, ($factory, [$($feature),+]));
    };
}

macro_rules! register_horzcat_scalar {
    ($builder:ident, $factory:ident; $token:ident, $_scalar:ty, $_name:literal, $_cargo:literal) => {
        paste! { [<register_ $factory:snake _ $token>]($builder)?; }
    };
}

macro_rules! install_horzcat_linked_factories {
    ($builder:ident, $factory:ident) => {{
        for_each_horzcat_scalar!(register_horzcat_scalar, ($builder, $factory));
        Ok::<(), MechError>(())
    }};
}

#[cfg(any(feature = "row_vectord", feature = "row_vector2"))]
macro_rules! register_horzcat_scalar_except_f64 {
    ($builder:ident, $factory:ident; f64, $_scalar:ty, $_name:literal, $_cargo:literal) => {};
    ($builder:ident, $factory:ident; $token:ident, $_scalar:ty, $_name:literal, $_cargo:literal) => {
        paste! { [<register_ $factory:snake _ $token>]($builder)?; }
    };
}

#[cfg(any(feature = "row_vectord", feature = "row_vector2"))]
macro_rules! install_horzcat_linked_factories_except_f64 {
    ($builder:ident, $factory:ident) => {{
        for_each_horzcat_scalar!(register_horzcat_scalar_except_f64, ($builder, $factory));
        Ok::<(), MechError>(())
    }};
}

declare_horzcat_family!(HorizontalConcatenateTwoArgs, ["matrixd"]);
declare_horzcat_family!(HorizontalConcatenateThreeArgs, ["matrixd"]);
declare_horzcat_family!(HorizontalConcatenateFourArgs, ["matrixd"]);
declare_horzcat_family!(HorizontalConcatenateNArgs, ["matrixd"]);
declare_horzcat_family!(HorizontalConcatenateS1D, ["matrixd"]);
declare_horzcat_family!(HorizontalConcatenateMD, ["matrixd"]);
declare_horzcat_family!(HorizontalConcatenateRD, ["row_vectord"]);
declare_horzcat_family!(HorizontalConcatenateRDN, ["row_vectord"]);
declare_horzcat_family!(HorizontalConcatenateSD, ["row_vectord"]);
declare_horzcat_family!(HorizontalConcatenateVD, ["vectord"]);

// Fixed-shape constructors use the same concrete scalar traversal as the
// dynamic families above.  Keep their feature requirements beside each
// factory family so a native plan selects exactly the storage needed by the
// retained concrete implementation.
macro_rules! for_each_horzcat_legacy_family {
    ($callback:ident) => {
        #[cfg(feature = "matrix1")] $callback!(normal; HorizontalConcatenateS1; ["matrix1"]);
        #[cfg(feature = "matrix1")] $callback!(normal; HorizontalConcatenateM1; ["matrix1"]);
        #[cfg(feature = "matrix2")] $callback!(normal; HorizontalConcatenateM2; ["matrix2"]);
        #[cfg(feature = "matrix3")] $callback!(normal; HorizontalConcatenateM3; ["matrix3"]);
        #[cfg(feature = "matrix4")] $callback!(normal; HorizontalConcatenateM4; ["matrix4"]);
        #[cfg(feature = "matrix2x3")] $callback!(normal; HorizontalConcatenateM2x3; ["matrix2x3"]);
        #[cfg(feature = "matrix3x2")] $callback!(normal; HorizontalConcatenateM3x2; ["matrix3x2"]);
        #[cfg(feature = "vector2")] $callback!(normal; HorizontalConcatenateV2; ["vector2"]);
        #[cfg(feature = "vector3")] $callback!(normal; HorizontalConcatenateV3; ["vector3"]);
        #[cfg(feature = "vector4")] $callback!(normal; HorizontalConcatenateV4; ["vector4"]);
        #[cfg(feature = "row_vector2")] $callback!(except_f64; HorizontalConcatenateS2; ["row_vector2"]);
        #[cfg(feature = "row_vector2")] $callback!(normal; HorizontalConcatenateR2; ["row_vector2"]);
        #[cfg(feature = "row_vector3")] $callback!(normal; HorizontalConcatenateS3; ["row_vector3"]);
        #[cfg(feature = "row_vector3")] $callback!(normal; HorizontalConcatenateR3; ["row_vector3"]);
        #[cfg(feature = "row_vector4")] $callback!(normal; HorizontalConcatenateS4; ["row_vector4"]);
        #[cfg(feature = "row_vector4")] $callback!(normal; HorizontalConcatenateR4; ["row_vector4"]);
        #[cfg(all(feature = "row_vector2", feature = "row_vector3"))] $callback!(normal; HorizontalConcatenateSR2; ["row_vector2", "row_vector3"]);
        #[cfg(all(feature = "row_vector2", feature = "row_vector3"))] $callback!(normal; HorizontalConcatenateR2S; ["row_vector2", "row_vector3"]);
        #[cfg(all(feature = "matrix1", feature = "row_vector2"))] $callback!(normal; HorizontalConcatenateSM1; ["matrix1", "row_vector2"]);
        #[cfg(all(feature = "matrix1", feature = "row_vector2"))] $callback!(normal; HorizontalConcatenateM1S; ["matrix1", "row_vector2"]);
        #[cfg(all(feature = "matrix1", feature = "row_vector2"))] $callback!(normal; HorizontalConcatenateM1M1; ["matrix1", "row_vector2"]);
        #[cfg(all(feature = "matrix1", feature = "row_vector3"))] $callback!(normal; HorizontalConcatenateSSM1; ["matrix1", "row_vector3"]);
        #[cfg(all(feature = "matrix1", feature = "row_vector3"))] $callback!(normal; HorizontalConcatenateSM1S; ["matrix1", "row_vector3"]);
        #[cfg(all(feature = "matrix1", feature = "row_vector3"))] $callback!(normal; HorizontalConcatenateM1SS; ["matrix1", "row_vector3"]);
        #[cfg(all(feature = "matrix1", feature = "row_vector3"))] $callback!(normal; HorizontalConcatenateM1M1S; ["matrix1", "row_vector3"]);
        #[cfg(all(feature = "matrix1", feature = "row_vector3"))] $callback!(normal; HorizontalConcatenateM1SM1; ["matrix1", "row_vector3"]);
        #[cfg(all(feature = "matrix1", feature = "row_vector3"))] $callback!(normal; HorizontalConcatenateSM1M1; ["matrix1", "row_vector3"]);
        #[cfg(all(feature = "matrix1", feature = "row_vector3"))] $callback!(normal; HorizontalConcatenateM1M1M1; ["matrix1", "row_vector3"]);
        #[cfg(all(feature = "matrix1", feature = "row_vector4"))] $callback!(normal; HorizontalConcatenateSSSM1; ["matrix1", "row_vector4"]);
        #[cfg(all(feature = "matrix1", feature = "row_vector4"))] $callback!(normal; HorizontalConcatenateSSM1S; ["matrix1", "row_vector4"]);
        #[cfg(all(feature = "matrix1", feature = "row_vector4"))] $callback!(normal; HorizontalConcatenateSM1SS; ["matrix1", "row_vector4"]);
        #[cfg(all(feature = "matrix1", feature = "row_vector4"))] $callback!(normal; HorizontalConcatenateM1SSS; ["matrix1", "row_vector4"]);
        #[cfg(all(feature = "matrix1", feature = "row_vector4"))] $callback!(normal; HorizontalConcatenateSM1SM1; ["matrix1", "row_vector4"]);
        #[cfg(all(feature = "matrix1", feature = "row_vector4"))] $callback!(normal; HorizontalConcatenateSSM1M1; ["matrix1", "row_vector4"]);
        #[cfg(all(feature = "matrix1", feature = "row_vector4"))] $callback!(normal; HorizontalConcatenateM1M1SS; ["matrix1", "row_vector4"]);
        #[cfg(all(feature = "matrix1", feature = "row_vector4"))] $callback!(normal; HorizontalConcatenateSM1M1S; ["matrix1", "row_vector4"]);
        #[cfg(all(feature = "matrix1", feature = "row_vector4"))] $callback!(normal; HorizontalConcatenateM1M1M1S; ["matrix1", "row_vector4"]);
        #[cfg(all(feature = "matrix1", feature = "row_vector4"))] $callback!(normal; HorizontalConcatenateM1SSM1; ["matrix1", "row_vector4"]);
        #[cfg(all(feature = "matrix1", feature = "row_vector4"))] $callback!(normal; HorizontalConcatenateM1SM1S; ["matrix1", "row_vector4"]);
        #[cfg(all(feature = "matrix1", feature = "row_vector4"))] $callback!(normal; HorizontalConcatenateSM1M1M1; ["matrix1", "row_vector4"]);
        #[cfg(all(feature = "matrix1", feature = "row_vector4"))] $callback!(normal; HorizontalConcatenateM1SM1M1; ["matrix1", "row_vector4"]);
        #[cfg(all(feature = "matrix1", feature = "row_vector4"))] $callback!(normal; HorizontalConcatenateM1M1SM1; ["matrix1", "row_vector4"]);
        #[cfg(all(feature = "matrix1", feature = "row_vector4"))] $callback!(normal; HorizontalConcatenateM1M1M1M1; ["matrix1", "row_vector4"]);
        #[cfg(all(feature = "row_vector3", feature = "row_vector4"))] $callback!(normal; HorizontalConcatenateSR3; ["row_vector3", "row_vector4"]);
        #[cfg(all(feature = "row_vector3", feature = "row_vector4"))] $callback!(normal; HorizontalConcatenateR3S; ["row_vector3", "row_vector4"]);
        #[cfg(all(feature = "row_vector2", feature = "row_vector4"))] $callback!(normal; HorizontalConcatenateSSR2; ["row_vector2", "row_vector4"]);
        #[cfg(all(feature = "row_vector2", feature = "row_vector4"))] $callback!(normal; HorizontalConcatenateSR2S; ["row_vector2", "row_vector4"]);
        #[cfg(all(feature = "row_vector2", feature = "row_vector4"))] $callback!(normal; HorizontalConcatenateR2SS; ["row_vector2", "row_vector4"]);
        #[cfg(all(feature = "row_vector2", feature = "row_vector4"))] $callback!(normal; HorizontalConcatenateR2R2; ["row_vector2", "row_vector4"]);
        #[cfg(all(feature = "matrix1", feature = "row_vector3", feature = "row_vector4"))] $callback!(normal; HorizontalConcatenateM1R3; ["matrix1", "row_vector3", "row_vector4"]);
        #[cfg(all(feature = "matrix1", feature = "row_vector3", feature = "row_vector4"))] $callback!(normal; HorizontalConcatenateR3M1; ["matrix1", "row_vector3", "row_vector4"]);
        #[cfg(all(feature = "matrix1", feature = "row_vector2", feature = "row_vector4"))] $callback!(normal; HorizontalConcatenateSM1R2; ["matrix1", "row_vector2", "row_vector4"]);
        #[cfg(all(feature = "matrix1", feature = "row_vector2", feature = "row_vector4"))] $callback!(normal; HorizontalConcatenateM1SR2; ["matrix1", "row_vector2", "row_vector4"]);
        #[cfg(all(feature = "matrix1", feature = "row_vector2", feature = "row_vector4"))] $callback!(normal; HorizontalConcatenateM1R2S; ["matrix1", "row_vector2", "row_vector4"]);
        #[cfg(all(feature = "matrix1", feature = "row_vector2", feature = "row_vector4"))] $callback!(normal; HorizontalConcatenateR2M1S; ["matrix1", "row_vector2", "row_vector4"]);
        #[cfg(all(feature = "matrix1", feature = "row_vector2", feature = "row_vector4"))] $callback!(normal; HorizontalConcatenateR2SM1; ["matrix1", "row_vector2", "row_vector4"]);
        #[cfg(all(feature = "matrix1", feature = "row_vector2", feature = "row_vector4"))] $callback!(normal; HorizontalConcatenateSR2M1; ["matrix1", "row_vector2", "row_vector4"]);
        #[cfg(all(feature = "matrix1", feature = "row_vector2", feature = "row_vector4"))] $callback!(normal; HorizontalConcatenateM1M1R2; ["matrix1", "row_vector2", "row_vector4"]);
        #[cfg(all(feature = "matrix1", feature = "row_vector2", feature = "row_vector4"))] $callback!(normal; HorizontalConcatenateM1R2M1; ["matrix1", "row_vector2", "row_vector4"]);
        #[cfg(all(feature = "matrix1", feature = "row_vector2", feature = "row_vector4"))] $callback!(normal; HorizontalConcatenateR2M1M1; ["matrix1", "row_vector2", "row_vector4"]);
        #[cfg(all(feature = "matrix1", feature = "row_vector2", feature = "row_vector3"))] $callback!(normal; HorizontalConcatenateM1R2; ["matrix1", "row_vector2", "row_vector3"]);
        #[cfg(all(feature = "matrix1", feature = "row_vector2", feature = "row_vector3"))] $callback!(normal; HorizontalConcatenateR2M1; ["matrix1", "row_vector2", "row_vector3"]);
        #[cfg(all(feature = "vector2", feature = "matrix2"))] $callback!(normal; HorizontalConcatenateV2V2; ["vector2", "matrix2"]);
        #[cfg(all(feature = "vector3", feature = "matrix3x2"))] $callback!(normal; HorizontalConcatenateV3V3; ["vector3", "matrix3x2"]);
        #[cfg(all(feature = "vector2", feature = "matrix2", feature = "matrix2x3"))] $callback!(normal; HorizontalConcatenateV2M2; ["vector2", "matrix2", "matrix2x3"]);
        #[cfg(all(feature = "vector2", feature = "matrix2", feature = "matrix2x3"))] $callback!(normal; HorizontalConcatenateM2V2; ["vector2", "matrix2", "matrix2x3"]);
        #[cfg(all(feature = "vector3", feature = "matrix3x2", feature = "matrix3"))] $callback!(normal; HorizontalConcatenateM3x2V3; ["vector3", "matrix3x2", "matrix3"]);
        #[cfg(all(feature = "vector3", feature = "matrix3x2", feature = "matrix3"))] $callback!(normal; HorizontalConcatenateV3M3x2; ["vector3", "matrix3x2", "matrix3"]);
        #[cfg(all(feature = "matrixd", feature = "matrix4", feature = "vector4"))] $callback!(normal; HorizontalConcatenateV4MD; ["matrixd", "matrix4", "vector4"]);
        #[cfg(all(feature = "matrixd", feature = "matrix4", feature = "vector4"))] $callback!(normal; HorizontalConcatenateMDV4; ["matrixd", "matrix4", "vector4"]);
        #[cfg(all(feature = "matrixd", feature = "matrix4", feature = "vector4"))] $callback!(normal; HorizontalConcatenateV4V4MD; ["matrixd", "matrix4", "vector4"]);
        #[cfg(all(feature = "matrixd", feature = "matrix4", feature = "vector4"))] $callback!(normal; HorizontalConcatenateV4MDV4; ["matrixd", "matrix4", "vector4"]);
        #[cfg(all(feature = "matrixd", feature = "matrix4", feature = "vector4"))] $callback!(normal; HorizontalConcatenateMDV4V4; ["matrixd", "matrix4", "vector4"]);
        #[cfg(all(feature = "matrixd", feature = "matrix4"))] $callback!(normal; HorizontalConcatenateMDMD; ["matrixd", "matrix4"]);
        #[cfg(all(feature = "vector2", feature = "matrix2x3"))] $callback!(normal; HorizontalConcatenateV2V2V2; ["vector2", "matrix2x3"]);
        #[cfg(all(feature = "vector3", feature = "matrix3"))] $callback!(normal; HorizontalConcatenateV3V3V3; ["vector3", "matrix3"]);
        #[cfg(all(feature = "matrix4", feature = "vector4"))] $callback!(normal; HorizontalConcatenateV4V4V4V4; ["matrix4", "vector4"]);
    };
}

#[cfg(feature = "row_vector2")]
macro_rules! declare_horzcat_scalar_except_f64 {
    ($factory:ident, [$($feature:literal),+]; f64, $_scalar:ty, $_name:literal, $_cargo:literal) => {};
    ($factory:ident, [$($feature:literal),+]; $token:ident, $scalar:ty, $name:literal, $cargo:literal) => {
        declare_horzcat_scalar!($factory, [$($feature),+]; $token, $scalar, $name, $cargo);
    };
}

#[cfg(any(
    feature = "matrix1",
    feature = "matrix2",
    feature = "matrix3",
    feature = "matrix4",
    feature = "matrix2x3",
    feature = "matrix3x2",
    feature = "row_vector2",
    feature = "row_vector3",
    feature = "row_vector4",
    feature = "vector2",
    feature = "vector3",
    feature = "vector4"
))]
macro_rules! declare_horzcat_legacy_family {
    (normal; $factory:ident; [$($feature:literal),+]) => {
        declare_horzcat_family!($factory, [$($feature),+]);
    };
    (except_f64; $factory:ident; [$($feature:literal),+]) => {
        for_each_horzcat_scalar!(declare_horzcat_scalar_except_f64, ($factory, [$($feature),+]));
    };
}

for_each_horzcat_legacy_family!(declare_horzcat_legacy_family);

#[cfg(feature = "row_vector2")]
macro_rules! install_horzcat_factories_except_f64 {
    ($builder:ident, $factory:ident) => {
        install_horzcat_linked_factories_except_f64!($builder, $factory)?;
    };
}

/// Installs every enabled legacy runtime factory emitted by this module.
pub(super) fn install_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    #[cfg(feature = "matrixd")]
    {
        install_horzcat_linked_factories!(builder, HorizontalConcatenateTwoArgs)?;
        install_horzcat_linked_factories!(builder, HorizontalConcatenateThreeArgs)?;
        install_horzcat_linked_factories!(builder, HorizontalConcatenateS1D)?;
        install_horzcat_linked_factories!(builder, HorizontalConcatenateMD)?;
    }
    #[cfg(feature = "row_vectord")]
    {
        install_horzcat_linked_factories!(builder, HorizontalConcatenateRD)?;
        #[cfg(feature = "f64")]
        register_horizontal_concatenate_rdn_f64(builder)?;
        install_horzcat_linked_factories_except_f64!(builder, HorizontalConcatenateRDN)?;
        install_horzcat_linked_factories!(builder, HorizontalConcatenateSD)?;
    }
    #[cfg(feature = "vectord")]
    install_horzcat_linked_factories!(builder, HorizontalConcatenateVD)?;

    #[cfg(feature = "matrix1")]
    {
        install_horzcat_factories!(builder, HorizontalConcatenateS1);
        install_horzcat_factories!(builder, HorizontalConcatenateM1);
    }
    #[cfg(feature = "matrix2")]
    install_horzcat_factories!(builder, HorizontalConcatenateM2);
    #[cfg(feature = "matrix3")]
    install_horzcat_factories!(builder, HorizontalConcatenateM3);
    #[cfg(feature = "matrix4")]
    install_horzcat_factories!(builder, HorizontalConcatenateM4);
    #[cfg(feature = "matrix2x3")]
    install_horzcat_factories!(builder, HorizontalConcatenateM2x3);
    #[cfg(feature = "matrix3x2")]
    install_horzcat_factories!(builder, HorizontalConcatenateM3x2);
    #[cfg(feature = "vector2")]
    install_horzcat_factories!(builder, HorizontalConcatenateV2);
    #[cfg(feature = "vector3")]
    install_horzcat_factories!(builder, HorizontalConcatenateV3);
    #[cfg(feature = "vector4")]
    install_horzcat_factories!(builder, HorizontalConcatenateV4);
    #[cfg(feature = "row_vector2")]
    {
        #[cfg(feature = "f64")]
        register_horizontal_concatenate_s2_f64(builder)?;
        install_horzcat_factories_except_f64!(builder, HorizontalConcatenateS2);
        install_horzcat_factories!(builder, HorizontalConcatenateR2);
    }
    #[cfg(feature = "row_vector3")]
    {
        install_horzcat_factories!(builder, HorizontalConcatenateS3);
        install_horzcat_factories!(builder, HorizontalConcatenateR3);
    }
    #[cfg(feature = "row_vector4")]
    {
        install_horzcat_factories!(builder, HorizontalConcatenateS4);
        install_horzcat_factories!(builder, HorizontalConcatenateR4);
    }

    #[cfg(all(feature = "row_vector2", feature = "row_vector3"))]
    {
        install_horzcat_factories!(builder, HorizontalConcatenateSR2);
        install_horzcat_factories!(builder, HorizontalConcatenateR2S);
    }
    #[cfg(all(feature = "matrix1", feature = "row_vector2"))]
    {
        install_horzcat_factories!(builder, HorizontalConcatenateSM1);
        install_horzcat_factories!(builder, HorizontalConcatenateM1S);
        install_horzcat_factories!(builder, HorizontalConcatenateM1M1);
    }
    #[cfg(all(feature = "matrix1", feature = "row_vector3"))]
    {
        install_horzcat_factories!(builder, HorizontalConcatenateSSM1);
        install_horzcat_factories!(builder, HorizontalConcatenateSM1S);
        install_horzcat_factories!(builder, HorizontalConcatenateM1SS);
        install_horzcat_factories!(builder, HorizontalConcatenateM1M1S);
        install_horzcat_factories!(builder, HorizontalConcatenateM1SM1);
        install_horzcat_factories!(builder, HorizontalConcatenateSM1M1);
        install_horzcat_factories!(builder, HorizontalConcatenateM1M1M1);
    }
    #[cfg(all(feature = "matrix1", feature = "row_vector4"))]
    {
        install_horzcat_factories!(builder, HorizontalConcatenateSSSM1);
        install_horzcat_factories!(builder, HorizontalConcatenateSSM1S);
        install_horzcat_factories!(builder, HorizontalConcatenateSM1SS);
        install_horzcat_factories!(builder, HorizontalConcatenateM1SSS);
        install_horzcat_factories!(builder, HorizontalConcatenateSM1SM1);
        install_horzcat_factories!(builder, HorizontalConcatenateSSM1M1);
        install_horzcat_factories!(builder, HorizontalConcatenateM1M1SS);
        install_horzcat_factories!(builder, HorizontalConcatenateSM1M1S);
        install_horzcat_factories!(builder, HorizontalConcatenateM1SSM1);
        install_horzcat_factories!(builder, HorizontalConcatenateM1SM1S);
        install_horzcat_factories!(builder, HorizontalConcatenateSM1M1M1);
        install_horzcat_factories!(builder, HorizontalConcatenateM1SM1M1);
        install_horzcat_factories!(builder, HorizontalConcatenateM1M1SM1);
        install_horzcat_factories!(builder, HorizontalConcatenateM1M1M1S);
        install_horzcat_factories!(builder, HorizontalConcatenateM1M1M1M1);
    }
    #[cfg(all(feature = "row_vector3", feature = "row_vector4"))]
    {
        install_horzcat_factories!(builder, HorizontalConcatenateSR3);
        install_horzcat_factories!(builder, HorizontalConcatenateR3S);
    }
    #[cfg(all(feature = "row_vector2", feature = "row_vector4"))]
    {
        install_horzcat_factories!(builder, HorizontalConcatenateSSR2);
        install_horzcat_factories!(builder, HorizontalConcatenateSR2S);
        install_horzcat_factories!(builder, HorizontalConcatenateR2SS);
        install_horzcat_factories!(builder, HorizontalConcatenateR2R2);
    }
    #[cfg(all(feature = "matrix1", feature = "row_vector3", feature = "row_vector4"))]
    {
        install_horzcat_factories!(builder, HorizontalConcatenateM1R3);
        install_horzcat_factories!(builder, HorizontalConcatenateR3M1);
    }
    #[cfg(all(feature = "matrix1", feature = "row_vector2", feature = "row_vector4"))]
    {
        install_horzcat_factories!(builder, HorizontalConcatenateSM1R2);
        install_horzcat_factories!(builder, HorizontalConcatenateM1SR2);
        install_horzcat_factories!(builder, HorizontalConcatenateM1R2S);
        install_horzcat_factories!(builder, HorizontalConcatenateR2M1S);
        install_horzcat_factories!(builder, HorizontalConcatenateR2SM1);
        install_horzcat_factories!(builder, HorizontalConcatenateSR2M1);
        install_horzcat_factories!(builder, HorizontalConcatenateM1M1R2);
        install_horzcat_factories!(builder, HorizontalConcatenateM1R2M1);
        install_horzcat_factories!(builder, HorizontalConcatenateR2M1M1);
    }
    #[cfg(all(feature = "matrix1", feature = "row_vector2", feature = "row_vector3"))]
    {
        install_horzcat_factories!(builder, HorizontalConcatenateM1R2);
        install_horzcat_factories!(builder, HorizontalConcatenateR2M1);
    }

    #[cfg(all(feature = "vector2", feature = "matrix2"))]
    install_horzcat_factories!(builder, HorizontalConcatenateV2V2);
    #[cfg(all(feature = "vector3", feature = "matrix3x2"))]
    install_horzcat_factories!(builder, HorizontalConcatenateV3V3);
    #[cfg(all(feature = "vector2", feature = "matrix2", feature = "matrix2x3"))]
    {
        install_horzcat_factories!(builder, HorizontalConcatenateV2M2);
        install_horzcat_factories!(builder, HorizontalConcatenateM2V2);
    }
    #[cfg(all(feature = "vector3", feature = "matrix3x2", feature = "matrix3"))]
    {
        install_horzcat_factories!(builder, HorizontalConcatenateM3x2V3);
        install_horzcat_factories!(builder, HorizontalConcatenateV3M3x2);
    }
    #[cfg(all(feature = "matrixd", feature = "matrix4", feature = "vector4"))]
    {
        install_horzcat_factories!(builder, HorizontalConcatenateV4MD);
        install_horzcat_factories!(builder, HorizontalConcatenateMDV4);
        install_horzcat_factories!(builder, HorizontalConcatenateV4V4MD);
        install_horzcat_factories!(builder, HorizontalConcatenateV4MDV4);
        install_horzcat_factories!(builder, HorizontalConcatenateMDV4V4);
    }
    #[cfg(all(feature = "matrixd", feature = "matrix4"))]
    install_horzcat_factories!(builder, HorizontalConcatenateMDMD);
    #[cfg(all(feature = "vector2", feature = "matrix2x3"))]
    install_horzcat_factories!(builder, HorizontalConcatenateV2V2V2);
    #[cfg(all(feature = "vector3", feature = "matrix3"))]
    install_horzcat_factories!(builder, HorizontalConcatenateV3V3V3);
    #[cfg(all(feature = "matrix4", feature = "vector4"))]
    install_horzcat_factories!(builder, HorizontalConcatenateV4V4V4V4);

    Ok(())
}

/// Installs the variadic f64 factory needed to execute and compile ordinary
/// dynamic-matrix source without expanding the frozen runtime-only catalog.
#[cfg(feature = "semantic-compiler")]
pub(super) fn install_source_runtime(
    #[cfg(all(feature = "f64", feature = "matrixd"))] builder: &mut FunctionCatalogBuilder,
    #[cfg(not(all(feature = "f64", feature = "matrixd")))] _: &mut FunctionCatalogBuilder,
) -> MResult<()> {
    #[cfg(all(feature = "f64", feature = "matrixd"))]
    if !builder.contains_runtime_factory(mech_core::RuntimeFunctionId::from_name(
        "HorizontalConcatenateNArgs<f64>",
    )) {
        register_horizontal_concatenate_n_args_f64(builder)?;
    }
    Ok(())
}

/// Installs compiler-emitted factories that belong only to native planning.
///
/// The standard runtime catalog intentionally excludes these dynamic f64
/// horizontal-concatenation specializations to preserve its frozen surface.
/// Native application planning must still resolve bytecode that the source
/// compiler emits for four-or-more dynamic matrix inputs.
#[cfg(feature = "native-plan")]
pub(super) fn install_native_plan_runtime(
    #[cfg(all(feature = "f64", feature = "matrixd"))] builder: &mut FunctionCatalogBuilder,
    #[cfg(not(all(feature = "f64", feature = "matrixd")))] _: &mut FunctionCatalogBuilder,
) -> MResult<()> {
    #[cfg(all(feature = "f64", feature = "matrixd"))]
    {
        register_horizontal_concatenate_four_args_f64(builder)?;
        register_horizontal_concatenate_n_args_f64(builder)?;
    }

    Ok(())
}

#[cfg(feature = "native-link")]
macro_rules! export_horzcat_scalar {
    ($factory:ident, [$($feature:literal),+]; $token:ident, $_scalar:ty, $_name:literal, $cargo:literal) => {
        #[cfg(all(feature = "matrix_horzcat", feature = $cargo, $(feature = $feature),+))]
        mech_core::paste::paste! { pub use super::[<install_ $factory:snake _ $token>]; }
    };
}

#[cfg(feature = "native-link")]
macro_rules! export_horzcat_family {
    ($factory:ident, [$($feature:literal),+]) => {
        for_each_horzcat_scalar!(export_horzcat_scalar, ($factory, [$($feature),+]));
    };
}

#[cfg(all(feature = "native-link", feature = "row_vector2"))]
macro_rules! export_horzcat_scalar_except_f64 {
    ($factory:ident, [$($feature:literal),+]; f64, $_scalar:ty, $_name:literal, $_cargo:literal) => {};
    ($factory:ident, [$($feature:literal),+]; $token:ident, $_scalar:ty, $_name:literal, $cargo:literal) => {
        #[cfg(all(feature = "matrix_horzcat", feature = $cargo, $(feature = $feature),+))]
        mech_core::paste::paste! { pub use super::[<install_ $factory:snake _ $token>]; }
    };
}

#[cfg(all(
    feature = "native-link",
    any(
        feature = "matrix1",
        feature = "matrix2",
        feature = "matrix3",
        feature = "matrix4",
        feature = "matrix2x3",
        feature = "matrix3x2",
        feature = "row_vector2",
        feature = "row_vector3",
        feature = "row_vector4",
        feature = "vector2",
        feature = "vector3",
        feature = "vector4"
    )
))]
macro_rules! export_horzcat_legacy_family {
    (normal; $factory:ident; [$($feature:literal),+]) => {
        export_horzcat_family!($factory, [$($feature),+]);
    };
    (except_f64; $factory:ident; [$($feature:literal),+]) => {
        for_each_horzcat_scalar!(export_horzcat_scalar_except_f64, ($factory, [$($feature),+]));
    };
}

#[doc(hidden)]
#[cfg(feature = "native-link")]
pub mod __mech_native {
    export_horzcat_family!(HorizontalConcatenateTwoArgs, ["matrixd"]);
    export_horzcat_family!(HorizontalConcatenateThreeArgs, ["matrixd"]);
    export_horzcat_family!(HorizontalConcatenateFourArgs, ["matrixd"]);
    export_horzcat_family!(HorizontalConcatenateNArgs, ["matrixd"]);
    export_horzcat_family!(HorizontalConcatenateS1D, ["matrixd"]);
    export_horzcat_family!(HorizontalConcatenateMD, ["matrixd"]);
    export_horzcat_family!(HorizontalConcatenateRD, ["row_vectord"]);
    export_horzcat_family!(HorizontalConcatenateRDN, ["row_vectord"]);
    export_horzcat_family!(HorizontalConcatenateSD, ["row_vectord"]);
    export_horzcat_family!(HorizontalConcatenateVD, ["vectord"]);
    for_each_horzcat_legacy_family!(export_horzcat_legacy_family);

    #[cfg(all(feature = "f64", feature = "row_vectord"))]
    pub use super::install_horizontal_concatenate_rdn_f64;
    #[cfg(all(feature = "f64", feature = "row_vector2"))]
    pub use super::install_horizontal_concatenate_s2_f64;
}

pub struct MatrixHorzCat {}
impl CanonicalFunctionSpecializer for MatrixHorzCat {
    fn specialize_invocation(
        &self,
        invocation: &SpecializationInvocation,
        context: &mut SpecializationContext<'_>,
    ) -> MResult<SpecializedFunction> {
        ValueMatrixConcatenation::<false>::specialize(invocation, context)
    }
}

#[derive(Debug, Clone)]
pub struct HorizontalConcatenateDimensionMismatchError {}
impl MechErrorKind for HorizontalConcatenateDimensionMismatchError {
    fn name(&self) -> &str {
        "HorizontalConcatenateDimensionMismatch"
    }
    fn message(&self) -> String {
        format!(
            "Cannot horizontally concatenate matrices/vectors with dimensions that do not align."
        )
    }
}
