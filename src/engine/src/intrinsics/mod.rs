#[cfg(any(
    feature = "access",
    feature = "assign",
    feature = "set",
    feature = "set_comprehensions",
    feature = "matrix_comprehensions",
    feature = "convert",
    feature = "variable_define",
    feature = "matrix_horzcat",
    feature = "table",
    feature = "matrix_vertcat"
))]
use crate::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

#[cfg(any(
    feature = "access",
    feature = "convert",
    feature = "variable_define",
    feature = "matrix_horzcat",
    feature = "matrix_vertcat",
    feature = "table"
))]
use mech_core::paste::paste;
#[cfg(feature = "matrix")]
use na::DMatrix;
#[cfg(feature = "vectord")]
use na::DVector;
#[cfg(any(feature = "matrix1", feature = "variable_define_matrix1"))]
use na::Matrix1;
#[cfg(feature = "matrix2")]
use na::Matrix2;
#[cfg(feature = "matrix2x3")]
use na::Matrix2x3;
#[cfg(feature = "matrix3")]
use na::Matrix3;
#[cfg(feature = "matrix3x2")]
use na::Matrix3x2;
#[cfg(feature = "matrix4")]
use na::Matrix4;
#[cfg(feature = "row_vectord")]
use na::RowDVector;
#[cfg(feature = "row_vector2")]
use na::RowVector2;
#[cfg(feature = "row_vector3")]
use na::RowVector3;
#[cfg(feature = "row_vector4")]
use na::RowVector4;
#[cfg(feature = "vector2")]
use na::Vector2;
#[cfg(feature = "vector3")]
use na::Vector3;
#[cfg(feature = "vector4")]
use na::Vector4;
#[cfg(any(feature = "access", feature = "assign", feature = "variable_define"))]
use std::fmt::Debug;

#[cfg(feature = "semantic-compiler")]
#[derive(Debug, Clone)]
pub(crate) struct IndexOutOfBoundsError;

#[cfg(feature = "semantic-compiler")]
impl MechErrorKind for IndexOutOfBoundsError {
    fn name(&self) -> &str {
        "IndexOutOfBounds"
    }

    fn message(&self) -> String {
        "Index out of bounds".to_string()
    }
}

#[cfg(feature = "functions")]
pub mod catalog;

#[cfg(feature = "access")]
pub mod access;
#[cfg(feature = "assign")]
pub mod assign;
#[cfg(all(
    feature = "semantic-compiler",
    any(feature = "access", feature = "assign")
))]
pub(crate) mod canonical_access;
#[cfg(any(
    feature = "set",
    feature = "set_comprehensions",
    feature = "matrix_comprehensions",
    feature = "matrix_horzcat",
    feature = "matrix_vertcat"
))]
pub mod constructors;
#[cfg(feature = "convert")]
pub mod convert;
#[cfg(feature = "variable_define")]
pub mod define;
#[cfg(feature = "matrix_horzcat")]
pub mod horzcat;
#[cfg(feature = "table")]
pub mod table_ops;
#[cfg(feature = "matrix_vertcat")]
pub mod vertcat;

pub trait LosslessInto<T> {
    fn lossless_into(self) -> T;
}

pub trait LossyFrom<T> {
    fn lossy_from(value: T) -> Self;
}

#[macro_export]
macro_rules! impl_range_range_fxn_v {
    ($struct_name:ident, $op:ident, $ix1:ty, $ix2:ty) => {
        #[derive(Debug)]
        pub struct $struct_name<T, MatA, MatB, IxVec1, IxVec2> {
            pub source: FunctionValueInput,
            pub ixes: (FunctionValueInput, FunctionValueInput),
            pub sink: FunctionValueOutput,
            pub invocation: FunctionInvocation,
            pub _marker: std::marker::PhantomData<fn() -> (T, MatA, MatB, IxVec1, IxVec2)>,
        }
        impl<
            T,
            R1: 'static,
            C1: 'static,
            S1: 'static,
            R2: 'static,
            C2: 'static,
            S2: 'static,
            IxVec1: 'static,
            IxVec2: 'static,
        > MechFunctionFactory
            for $struct_name<T, naMatrix<T, R1, C1, S1>, naMatrix<T, R2, C2, S2>, IxVec1, IxVec2>
        where
            T: Debug
                + Clone
                + Sync
                + Send
                + 'static
                + PartialEq
                + PartialOrd
                + ConstElem
                + FunctionRuntimeType
                + ManagedAccessElement,
            #[cfg(feature = "semantic-compiler")]
            T: CompileConst + CanonicalMatrixElementBacking,
            IxVec1: ConstElem
                + Debug
                + AsRef<[$ix1]>
                + FunctionPortBacking
                + ManagedAccessSelectorBacking,
            #[cfg(feature = "semantic-compiler")]
            IxVec1: CompileConst,
            IxVec2: ConstElem
                + Debug
                + AsRef<[$ix2]>
                + FunctionPortBacking
                + ManagedAccessSelectorBacking,
            #[cfg(feature = "semantic-compiler")]
            IxVec2: CompileConst,
            R1: Dim,
            C1: Dim,
            S1: StorageMut<T, R1, C1> + Clone + Debug,
            R2: Dim,
            C2: Dim,
            S2: Storage<T, R2, C2> + Clone + Debug,
            naMatrix<T, R1, C1, S1>: ConstElem + Debug + FunctionStateBacking,
            #[cfg(feature = "semantic-compiler")]
            naMatrix<T, R1, C1, S1>: CompileConst,
            naMatrix<T, R2, C2, S2>: ConstElem + Debug + FunctionPortBacking,
            #[cfg(feature = "semantic-compiler")]
            naMatrix<T, R2, C2, S2>: CompileConst,
        {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::ternary(
                <naMatrix<T, R1, C1, S1> as FunctionRuntimeType>::REPRESENTATION,
                <naMatrix<T, R2, C2, S2> as FunctionRuntimeType>::REPRESENTATION,
                <IxVec1 as FunctionRuntimeType>::REPRESENTATION,
                <IxVec2 as FunctionRuntimeType>::REPRESENTATION,
            );

            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                T::MEMORY_CLASS
            }

            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                let (sink, source, ix1, ix2) = invocation.expect_ternary()?;
                T::validate_input(source)?;
                <IxVec1 as ManagedAccessSelectorBacking>::validate(ix1)?;
                <IxVec2 as ManagedAccessSelectorBacking>::validate(ix2)?;
                T::validate_output(sink)?;
                Ok(Box::new(Self {
                    sink: sink.value(),
                    source: source.value(),
                    ixes: (ix1.value(), ix2.value()),
                    invocation,
                    _marker: std::marker::PhantomData::default(),
                }))
            }

            fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
                Some(&managed_access_contract!($op))
            }
        }
        impl<T, R1, C1, S1, R2, C2, S2, IxVec1, IxVec2> MechFunctionImpl
            for $struct_name<T, naMatrix<T, R1, C1, S1>, naMatrix<T, R2, C2, S2>, IxVec1, IxVec2>
        where
            T: ManagedAccessElement,
            IxVec1: AsRef<[$ix1]> + Debug + ManagedAccessSelectorBacking,
            IxVec2: AsRef<[$ix2]> + Debug + ManagedAccessSelectorBacking,
            R1: Dim,
            C1: Dim,
            S1: StorageMut<T, R1, C1> + Clone + Debug,
            naMatrix<T, R1, C1, S1>: FunctionStateBacking,
            R2: Dim,
            C2: Dim,
            S2: Storage<T, R2, C2> + Clone + Debug,
        {
            fn planned_output_shapes(&self) -> MResult<Option<Box<[ShapeInstance]>>> {
                Ok(planned_matrix_access_output_shape(
                    self.invocation.output_cell(),
                    self.invocation.input_cells(),
                )?
                .map(|shape| vec![shape].into_boxed_slice()))
            }

            fn solve_managed(
                &self,
                frame: &mut mech_core::KernelMemoryFrame<'_>,
                _services: &mut dyn mech_core::MechExecutionServices,
            ) -> MResult<mech_core::ReactiveSolveStatus> {
                validate_matrix_access_contract(
                    self.invocation.output_cell(),
                    self.invocation.input_cells(),
                )?;
                T::solve_ternary::<
                    <IxVec1 as ManagedAccessSelectorBacking>::Element,
                    <IxVec2 as ManagedAccessSelectorBacking>::Element,
                >(
                    frame,
                    &self.source,
                    &self.ixes.0,
                    &self.ixes.1,
                    &self.sink,
                    managed_access_kernel!($op),
                )?;
                Ok(mech_core::ReactiveSolveStatus::Changed)
            }
            fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
                Some(FunctionStatePort::from_cell(self.sink.cell()))
            }
            fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
                Ok(Some(vec![FunctionStatePort::from_cell(self.sink.cell())]))
            }
            fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
                Some(&managed_access_contract!($op))
            }
            fn to_string(&self) -> String {
                format!("{:#?}", self)
            }
        }
        #[cfg(feature = "semantic-compiler")]
        impl<T, R1, C1, S1, R2, C2, S2, IxVec1, IxVec2> MechFunctionCompiler
            for $struct_name<T, naMatrix<T, R1, C1, S1>, naMatrix<T, R2, C2, S2>, IxVec1, IxVec2>
        where
            T: CompileConst + ConstElem + FunctionRuntimeType + CanonicalMatrixElementBacking,
            IxVec1: CompileConst + ConstElem,
            IxVec2: CompileConst + ConstElem,
            naMatrix<T, R1, C1, S1>: CompileConst + ConstElem,
            naMatrix<T, R2, C2, S2>: CompileConst + ConstElem,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!(
                    "{}<{}{}{}{}{}>",
                    stringify!($struct_name),
                    <T as FunctionRuntimeType>::REPRESENTATION,
                    function_matrix_storage_name::<naMatrix<T, R1, C1, S1>>(),
                    function_matrix_storage_name::<naMatrix<T, R2, C2, S2>>(),
                    function_matrix_storage_name::<IxVec1>(),
                    function_matrix_storage_name::<IxVec2>()
                );
                let sink = compile_value_cell_register(self.sink.cell(), ctx)?;
                let source = compile_value_cell_register(self.source.cell(), ctx)?;
                let ix1 = compile_value_cell_register(self.ixes.0.cell(), ctx)?;
                let ix2 = compile_value_cell_register(self.ixes.1.cell(), ctx)?;
                let function = ctx.function_id(&name)?;
                ctx.emit_ternop(function, sink, source, ix1, ix2);
                Ok(sink)
            }
        }
    };
}

#[macro_export]
macro_rules! impl_all_fxn_v {
    ($struct_name:ident, $op:ident, $ix:ty $(, $semantic_contract:path)?) => {
        #[derive(Debug)]
        pub struct $struct_name<T, MatA, MatB, IxVec> {
            pub source: FunctionValueInput,
            pub ixes: FunctionValueInput,
            pub sink: FunctionValueOutput,
            pub invocation: FunctionInvocation,
            pub _marker: std::marker::PhantomData<fn() -> (T, MatA, MatB, IxVec)>,
        }
        impl<
            T,
            R1: 'static,
            C1: 'static,
            S1: 'static,
            R2: 'static,
            C2: 'static,
            S2: 'static,
            IxVec: 'static,
        > MechFunctionFactory
            for $struct_name<T, naMatrix<T, R1, C1, S1>, naMatrix<T, R2, C2, S2>, IxVec>
        where
            T: Debug
                + Clone
                + Sync
                + Send
                + 'static
                + PartialEq
                + PartialOrd
                + ConstElem
                + FunctionRuntimeType
                + ManagedAccessElement,
            #[cfg(feature = "semantic-compiler")]
            T: CompileConst + CanonicalMatrixElementBacking,
            IxVec: ConstElem
                + Debug
                + AsRef<[$ix]>
                + FunctionPortBacking
                + ManagedAccessSelectorBacking,
            #[cfg(feature = "semantic-compiler")]
            IxVec: CompileConst,
            R1: Dim,
            C1: Dim,
            S1: StorageMut<T, R1, C1> + Clone + Debug,
            R2: Dim,
            C2: Dim,
            S2: Storage<T, R2, C2> + Clone + Debug,
            naMatrix<T, R1, C1, S1>: ConstElem + Debug + FunctionStateBacking,
            #[cfg(feature = "semantic-compiler")]
            naMatrix<T, R1, C1, S1>: CompileConst,
            naMatrix<T, R2, C2, S2>: ConstElem + Debug + FunctionPortBacking,
            #[cfg(feature = "semantic-compiler")]
            naMatrix<T, R2, C2, S2>: CompileConst,
        {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
                <naMatrix<T, R1, C1, S1> as FunctionRuntimeType>::REPRESENTATION,
                <naMatrix<T, R2, C2, S2> as FunctionRuntimeType>::REPRESENTATION,
                <IxVec as FunctionRuntimeType>::REPRESENTATION,
            );

            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                T::MEMORY_CLASS
            }

            fn new_invocation(
                invocation: FunctionInvocation,
            ) -> MResult<Box<dyn MechFunction>> {
                let (sink, source, ixes) = invocation.expect_binary()?;
                T::validate_input(source)?;
                <IxVec as ManagedAccessSelectorBacking>::validate(ixes)?;
                T::validate_output(sink)?;
                Ok(Box::new(Self {
                    sink: sink.value(),
                    source: source.value(),
                    ixes: ixes.value(),
                    invocation,
                    _marker: std::marker::PhantomData::default(),
                }))
            }

        }
        impl<T, R1, C1, S1, R2, C2, S2, IxVec> MechFunctionImpl
            for $struct_name<T, naMatrix<T, R1, C1, S1>, naMatrix<T, R2, C2, S2>, IxVec>
        where
            T: ManagedAccessElement,
            IxVec: AsRef<[$ix]> + Debug + ManagedAccessSelectorBacking,
            R1: Dim,
            C1: Dim,
            S1: StorageMut<T, R1, C1> + Clone + Debug,
            naMatrix<T, R1, C1, S1>: FunctionStateBacking,
            R2: Dim,
            C2: Dim,
            S2: Storage<T, R2, C2> + Clone + Debug,
        {
            fn planned_output_shapes(&self) -> MResult<Option<Box<[ShapeInstance]>>> {
                Ok(planned_matrix_access_all_range_output_shape(
                    self.invocation.output_cell(),
                    self.invocation.input_cells(),
                )?
                .map(|shape| vec![shape].into_boxed_slice()))
            }

            fn solve_managed(
                &self,
                frame: &mut mech_core::KernelMemoryFrame<'_>,
                _services: &mut dyn mech_core::MechExecutionServices,
            ) -> MResult<mech_core::ReactiveSolveStatus> {
                validate_matrix_access_all_range_contract(
                    self.invocation.output_cell(),
                    self.invocation.input_cells(),
                )?;
                T::solve_binary::<<IxVec as ManagedAccessSelectorBacking>::Element>(
                    frame,
                    &self.source,
                    &self.ixes,
                    &self.sink,
                    managed_access_kernel!($op),
                )?;
                Ok(mech_core::ReactiveSolveStatus::Changed)
            }
            fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
                Some(FunctionStatePort::from_cell(self.sink.cell()))
            }
            fn transaction_state_ports(
                &self,
            ) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
                Ok(Some(vec![FunctionStatePort::from_cell(self.sink.cell())]))
            }
            fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
                optional_operation_contract!($($semantic_contract)?)
            }
            fn to_string(&self) -> String {
                format!("{:#?}", self)
            }

        }
        #[cfg(feature = "semantic-compiler")]
        impl<T, R1, C1, S1, R2, C2, S2, IxVec> MechFunctionCompiler
            for $struct_name<T, naMatrix<T, R1, C1, S1>, naMatrix<T, R2, C2, S2>, IxVec>
        where
            T: CompileConst + ConstElem + FunctionRuntimeType + CanonicalMatrixElementBacking,
            IxVec: CompileConst + ConstElem,
            naMatrix<T, R1, C1, S1>: CompileConst + ConstElem,
            naMatrix<T, R2, C2, S2>: CompileConst + ConstElem,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!(
                    "{}<{}{}{}{}>",
                    stringify!($struct_name),
                    <T as FunctionRuntimeType>::REPRESENTATION,
                    function_matrix_storage_name::<naMatrix<T, R1, C1, S1>>(),
                    function_matrix_storage_name::<naMatrix<T, R2, C2, S2>>(),
                    function_matrix_storage_name::<IxVec>()
                );
                let sink = compile_value_cell_register(self.sink.cell(), ctx)?;
                let source = compile_value_cell_register(self.source.cell(), ctx)?;
                let ixes = compile_value_cell_register(self.ixes.cell(), ctx)?;
                let function = ctx.function_id(&name)?;
                ctx.emit_binop(function, sink, source, ixes);
                Ok(sink)
            }
        }
    };
}
