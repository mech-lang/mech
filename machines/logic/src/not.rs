use crate::*;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::Not;

// Not ------------------------------------------------------------------------

// NotS -----------------------------------------------------------------------

#[derive(Debug)]
pub(crate) struct NotS<T> {
    pub arg: Ref<T>,
    pub out: Ref<T>,
    pub _marker: PhantomData<T>,
}
impl<T> MechFunctionFactory for NotS<T>
where
    T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static + FunctionRuntimeType + Not<Output = T>,
    #[cfg(feature = "semantic-compiler")]
    T: CompileConst + ConstElem,
    T: FunctionStateBacking,
{
    const SIGNATURE: RuntimeFunctionSignature =
        RuntimeFunctionSignature::unary(T::REPRESENTATION, T::REPRESENTATION);

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, arg) = invocation.expect_unary()?;
        let arg: Ref<T> = arg.try_ref()?;
        let out: Ref<T> = out.try_ref()?;
        Ok(Box::new(Self {
            arg,
            out,
            _marker: PhantomData::default(),
        }))
    }

}
impl<T> MechFunctionImpl for NotS<T>
where
    T: Copy
        + Debug
        + Clone
        + Sync
        + Send
        + PartialEq
        + 'static
        + Not<Output = T>
        + FunctionStateBacking,
{
    fn solve_result(&self) -> MResult<()> {
        let arg_ptr = self.arg.as_ptr();
        let out_ptr = self.out.as_mut_ptr();
        unsafe {
            *out_ptr = !*arg_ptr;
        };
        Ok(())
    }
    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        Some(FunctionStatePort::from_ref(&self.out))
    }
    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(crate::logic_unary_full_write_contract(T::REPRESENTATION))
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        Ok(Some(vec![FunctionStatePort::from_ref(&self.out)]))
    }
}
#[cfg(feature = "semantic-compiler")]
impl<T> MechFunctionCompiler for NotS<T>
where
    T: CompileConst + ConstElem + FunctionRuntimeType,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!("NotS<{}>", <T as FunctionRuntimeType>::REPRESENTATION);
        compile_unop!(name, self.out, self.arg, ctx);
    }
}
// NotV -----------------------------------------------------------------------

#[derive(Debug)]
pub struct NotV<T, MatA> {
    pub arg: Ref<MatA>,
    pub out: Ref<MatA>,
    pub _marker: PhantomData<T>,
}
impl<T, MatA> MechFunctionFactory for NotV<T, MatA>
where
    T: Debug + Clone + Sync + Send + 'static + FunctionRuntimeType + Not<Output = T>,
    #[cfg(feature = "semantic-compiler")]
    T: CompileConst + ConstElem,
    for<'a> &'a MatA: IntoIterator<Item = &'a T>,
    for<'a> &'a mut MatA: IntoIterator<Item = &'a mut T>,
    MatA: Debug + FunctionRuntimeType + 'static,
    #[cfg(feature = "semantic-compiler")]
    MatA: CompileConst + ConstElem,
    MatA: FunctionStateBacking,
{
    const SIGNATURE: RuntimeFunctionSignature =
        RuntimeFunctionSignature::unary(MatA::REPRESENTATION, MatA::REPRESENTATION);

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, arg) = invocation.expect_unary()?;
        let arg: Ref<MatA> = arg.try_ref()?;
        let out: Ref<MatA> = out.try_ref()?;
        Ok(Box::new(Self {
            arg,
            out,
            _marker: PhantomData::default(),
        }))
    }

}
impl<T, MatA> MechFunctionImpl for NotV<T, MatA>
where
    T: Debug + Clone + Sync + Send + 'static + FunctionRuntimeType + Not<Output = T>,
    for<'a> &'a MatA: IntoIterator<Item = &'a T>,
    for<'a> &'a mut MatA: IntoIterator<Item = &'a mut T>,
    MatA: Debug + FunctionStateBacking,
{
    fn solve_result(&self) -> MResult<()> {
        unsafe {
            let sink_ptr = self.out.as_mut_ptr();
            let source_ptr = self.arg.as_ptr();
            let sink_ref: &mut MatA = &mut *sink_ptr;
            let source_ref: &MatA = &*source_ptr;
            for (dst, src) in sink_ref.into_iter().zip(source_ref.into_iter()) {
                *dst = !src.clone();
            }
        };
        Ok(())
    }
    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        Some(FunctionStatePort::from_ref(&self.out))
    }
    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(crate::logic_unary_full_write_contract(MatA::REPRESENTATION))
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        Ok(Some(vec![FunctionStatePort::from_ref(&self.out)]))
    }
}
#[cfg(feature = "semantic-compiler")]
impl<T, MatA> MechFunctionCompiler for NotV<T, MatA>
where
    T: CompileConst + ConstElem + FunctionRuntimeType,
    MatA: CompileConst + ConstElem + FunctionRuntimeType,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!("NotV<{}{}>", <T as FunctionRuntimeType>::REPRESENTATION, <MatA as FunctionRuntimeType>::REPRESENTATION);
        compile_unop!(name, self.out, self.arg, ctx);
    }
}

#[cfg(feature = "source")]
fn specialize_not_factory<F>(input: &SpecializationInput) -> MResult<SpecializedFunction>
where
    F: MechFunctionFactory,
{
    let output = input.cell()?.detached_clone()?;
    let invocation = FunctionInvocation::unary(output, input.cell()?.clone());
    let implementation = F::new_invocation(invocation.clone())?;
    Ok(SpecializedFunction::new(FunctionInstance::new(
        implementation,
        invocation,
    )))
}

#[cfg(feature = "source")]
pub struct LogicNot;

#[cfg(feature = "source")]
impl CanonicalFunctionSpecializer for LogicNot {
    fn specialize_invocation(
        &self,
        specialization: &SpecializationInvocation,
        _context: &mut SpecializationContext<'_>,
    ) -> MResult<SpecializedFunction> {
        if specialization.len() != 1 {
            return Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 1,
                    found: specialization.len(),
                },
                None,
            )
            .with_compiler_loc());
        }
        let input = specialization.input(0).expect("validated unary input");
        match input.representation() {
            #[cfg(feature = "bool")]
            Some(FunctionValueRepresentation::Bool) => {
                specialize_not_factory::<NotS<bool>>(input)
            }
            #[cfg(all(feature = "bool", feature = "matrix1"))]
            Some(FunctionValueRepresentation::Matrix {
                element: FunctionMatrixElement::Bool,
                storage: FunctionMatrixStoragePattern::Exact(
                    FunctionMatrixRepresentation::Matrix1,
                ),
            }) => specialize_not_factory::<NotV<bool, nalgebra::Matrix1<bool>>>(input),
            #[cfg(all(feature = "bool", feature = "matrix2"))]
            Some(FunctionValueRepresentation::Matrix {
                element: FunctionMatrixElement::Bool,
                storage: FunctionMatrixStoragePattern::Exact(
                    FunctionMatrixRepresentation::Matrix2,
                ),
            }) => specialize_not_factory::<NotV<bool, nalgebra::Matrix2<bool>>>(input),
            #[cfg(all(feature = "bool", feature = "matrix3"))]
            Some(FunctionValueRepresentation::Matrix {
                element: FunctionMatrixElement::Bool,
                storage: FunctionMatrixStoragePattern::Exact(
                    FunctionMatrixRepresentation::Matrix3,
                ),
            }) => specialize_not_factory::<NotV<bool, nalgebra::Matrix3<bool>>>(input),
            #[cfg(all(feature = "bool", feature = "matrix4"))]
            Some(FunctionValueRepresentation::Matrix {
                element: FunctionMatrixElement::Bool,
                storage: FunctionMatrixStoragePattern::Exact(
                    FunctionMatrixRepresentation::Matrix4,
                ),
            }) => specialize_not_factory::<NotV<bool, nalgebra::Matrix4<bool>>>(input),
            #[cfg(all(feature = "bool", feature = "matrix2x3"))]
            Some(FunctionValueRepresentation::Matrix {
                element: FunctionMatrixElement::Bool,
                storage: FunctionMatrixStoragePattern::Exact(
                    FunctionMatrixRepresentation::Matrix2x3,
                ),
            }) => specialize_not_factory::<NotV<bool, nalgebra::Matrix2x3<bool>>>(input),
            #[cfg(all(feature = "bool", feature = "matrix3x2"))]
            Some(FunctionValueRepresentation::Matrix {
                element: FunctionMatrixElement::Bool,
                storage: FunctionMatrixStoragePattern::Exact(
                    FunctionMatrixRepresentation::Matrix3x2,
                ),
            }) => specialize_not_factory::<NotV<bool, nalgebra::Matrix3x2<bool>>>(input),
            #[cfg(all(feature = "bool", feature = "row_vector2"))]
            Some(FunctionValueRepresentation::Matrix {
                element: FunctionMatrixElement::Bool,
                storage: FunctionMatrixStoragePattern::Exact(
                    FunctionMatrixRepresentation::RowVector2,
                ),
            }) => specialize_not_factory::<NotV<bool, nalgebra::RowVector2<bool>>>(input),
            #[cfg(all(feature = "bool", feature = "row_vector3"))]
            Some(FunctionValueRepresentation::Matrix {
                element: FunctionMatrixElement::Bool,
                storage: FunctionMatrixStoragePattern::Exact(
                    FunctionMatrixRepresentation::RowVector3,
                ),
            }) => specialize_not_factory::<NotV<bool, nalgebra::RowVector3<bool>>>(input),
            #[cfg(all(feature = "bool", feature = "row_vector4"))]
            Some(FunctionValueRepresentation::Matrix {
                element: FunctionMatrixElement::Bool,
                storage: FunctionMatrixStoragePattern::Exact(
                    FunctionMatrixRepresentation::RowVector4,
                ),
            }) => specialize_not_factory::<NotV<bool, nalgebra::RowVector4<bool>>>(input),
            #[cfg(all(feature = "bool", feature = "row_vectord"))]
            Some(FunctionValueRepresentation::Matrix {
                element: FunctionMatrixElement::Bool,
                storage: FunctionMatrixStoragePattern::Exact(
                    FunctionMatrixRepresentation::RowVectorD,
                ),
            }) => specialize_not_factory::<NotV<bool, nalgebra::RowDVector<bool>>>(input),
            #[cfg(all(feature = "bool", feature = "vector2"))]
            Some(FunctionValueRepresentation::Matrix {
                element: FunctionMatrixElement::Bool,
                storage: FunctionMatrixStoragePattern::Exact(
                    FunctionMatrixRepresentation::Vector2,
                ),
            }) => specialize_not_factory::<NotV<bool, nalgebra::Vector2<bool>>>(input),
            #[cfg(all(feature = "bool", feature = "vector3"))]
            Some(FunctionValueRepresentation::Matrix {
                element: FunctionMatrixElement::Bool,
                storage: FunctionMatrixStoragePattern::Exact(
                    FunctionMatrixRepresentation::Vector3,
                ),
            }) => specialize_not_factory::<NotV<bool, nalgebra::Vector3<bool>>>(input),
            #[cfg(all(feature = "bool", feature = "vector4"))]
            Some(FunctionValueRepresentation::Matrix {
                element: FunctionMatrixElement::Bool,
                storage: FunctionMatrixStoragePattern::Exact(
                    FunctionMatrixRepresentation::Vector4,
                ),
            }) => specialize_not_factory::<NotV<bool, nalgebra::Vector4<bool>>>(input),
            #[cfg(all(feature = "bool", feature = "vectord"))]
            Some(FunctionValueRepresentation::Matrix {
                element: FunctionMatrixElement::Bool,
                storage: FunctionMatrixStoragePattern::Exact(
                    FunctionMatrixRepresentation::VectorD,
                ),
            }) => specialize_not_factory::<NotV<bool, nalgebra::DVector<bool>>>(input),
            #[cfg(all(feature = "bool", feature = "matrixd"))]
            Some(FunctionValueRepresentation::Matrix {
                element: FunctionMatrixElement::Bool,
                storage: FunctionMatrixStoragePattern::Exact(
                    FunctionMatrixRepresentation::MatrixD,
                ),
            }) => specialize_not_factory::<NotV<bool, nalgebra::DMatrix<bool>>>(input),
            found => Err(MechError::new(
                FunctionArgumentTypeMismatch {
                    role: FunctionArgumentRole::Input(0),
                    expected: "Bool scalar or exact Bool matrix".into(),
                    found: format!("{found:?}"),
                },
                None,
            )
            .with_compiler_loc()),
        }
    }
}
