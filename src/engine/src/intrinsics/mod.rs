use crate::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;
use mech_core::*;

#[cfg(feature = "matrix")]
use na::{
    DMatrix, DVector, Matrix1, Matrix2, Matrix2x3, Matrix3, Matrix3x2, Matrix4, Matrix6, Rotation3,
    RowDVector, RowVector2, RowVector3, RowVector4, Vector2, Vector3, Vector4,
};
#[cfg(any(feature = "num-traits"))]
use num_traits::*;
use paste::paste;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::*;

#[derive(Debug, Clone)]
pub(crate) struct IndexOutOfBoundsError;

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
#[cfg(any(
    feature = "set",
    feature = "set_comprehensions",
    feature = "matrix_comprehensions"
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
            pub source: Ref<MatB>,
            pub ixes: (Ref<IxVec1>, Ref<IxVec2>),
            pub sink: Ref<MatA>,
            pub _marker: PhantomData<T>,
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
            Ref<naMatrix<T, R1, C1, S1>>: ToValue,
            Ref<naMatrix<T, R2, C2, S2>>: ToValue,
            T: Debug
                + Clone
                + Sync
                + Send
                + 'static
                + PartialEq
                + PartialOrd
                + ConstElem
                + AsValueKind,
            #[cfg(feature = "semantic-compiler")]
            T: CompileConst,
            IxVec1: ConstElem + AsNaKind + Debug + AsRef<[$ix1]> + FunctionRuntimeType,
            #[cfg(feature = "semantic-compiler")]
            IxVec1: CompileConst,
            IxVec2: ConstElem + AsNaKind + Debug + AsRef<[$ix2]> + FunctionRuntimeType,
            #[cfg(feature = "semantic-compiler")]
            IxVec2: CompileConst,
            R1: Dim,
            C1: Dim,
            S1: StorageMut<T, R1, C1> + Clone + Debug,
            R2: Dim,
            C2: Dim,
            S2: Storage<T, R2, C2> + Clone + Debug,
            naMatrix<T, R1, C1, S1>: ConstElem + Debug + AsNaKind + FunctionRuntimeType,
            #[cfg(feature = "semantic-compiler")]
            naMatrix<T, R1, C1, S1>: CompileConst,
            naMatrix<T, R2, C2, S2>: ConstElem + Debug + AsNaKind + FunctionRuntimeType,
            #[cfg(feature = "semantic-compiler")]
            naMatrix<T, R2, C2, S2>: CompileConst,
        {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::ternary(
                <naMatrix<T, R1, C1, S1> as FunctionRuntimeType>::REPRESENTATION,
                <naMatrix<T, R2, C2, S2> as FunctionRuntimeType>::REPRESENTATION,
                <IxVec1 as FunctionRuntimeType>::REPRESENTATION,
                <IxVec2 as FunctionRuntimeType>::REPRESENTATION,
            );

            fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
                match args {
                    FunctionArgs::Ternary(out, arg1, arg2, arg3) => {
                        let source: Ref<naMatrix<T, R2, C2, S2>> =
                            arg1.try_function_ref(FunctionArgumentRole::Input(0))?;
                        let ix1: Ref<IxVec1> =
                            arg2.try_function_ref(FunctionArgumentRole::Input(1))?;
                        let ix2: Ref<IxVec2> =
                            arg3.try_function_ref(FunctionArgumentRole::Input(2))?;
                        let sink: Ref<naMatrix<T, R1, C1, S1>> =
                            out.try_function_ref(FunctionArgumentRole::Output)?;
                        Ok(Box::new(Self {
                            sink,
                            source,
                            ixes: (ix1, ix2),
                            _marker: PhantomData::default(),
                        }))
                    }
                    _ => Err(MechError::new(
                        IncorrectNumberOfArguments {
                            expected: 3,
                            found: args.len(),
                        },
                        None,
                    )
                    .with_compiler_loc()),
                }
            }
        }
        impl<T, R1, C1, S1, R2, C2, S2, IxVec1, IxVec2> MechFunctionImpl
            for $struct_name<T, naMatrix<T, R1, C1, S1>, naMatrix<T, R2, C2, S2>, IxVec1, IxVec2>
        where
            Ref<naMatrix<T, R1, C1, S1>>: ToValue,
            T: Debug + Clone + Sync + Send + 'static + PartialEq + PartialOrd,
            IxVec1: AsRef<[$ix1]> + Debug,
            IxVec2: AsRef<[$ix2]> + Debug,
            R1: Dim,
            C1: Dim,
            S1: StorageMut<T, R1, C1> + Clone + Debug,
            R2: Dim,
            C2: Dim,
            S2: Storage<T, R2, C2> + Clone + Debug,
        {
            fn solve_result(&self) -> MResult<()> {
                unsafe {
                    let sink = &mut *self.sink.as_mut_ptr();
                    let source = &*self.source.as_ptr();
                    let ix1 = (*self.ixes.0.as_ptr()).as_ref();
                    let ix2 = (*self.ixes.1.as_ptr()).as_ref();
                    $op!(sink, ix1, ix2, source);
                };
                Ok(())
            }
            fn out(&self) -> LegacyValue {
                self.sink.to_value()
            }
            fn to_string(&self) -> String {
                format!("{:#?}", self)
            }

            fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
                Ok(self.reactive_output_values())
            }
        }
        #[cfg(feature = "semantic-compiler")]
        impl<T, R1, C1, S1, R2, C2, S2, IxVec1, IxVec2> MechFunctionCompiler
            for $struct_name<T, naMatrix<T, R1, C1, S1>, naMatrix<T, R2, C2, S2>, IxVec1, IxVec2>
        where
            T: CompileConst + ConstElem + AsValueKind,
            IxVec1: CompileConst + ConstElem + AsNaKind,
            IxVec2: CompileConst + ConstElem + AsNaKind,
            naMatrix<T, R1, C1, S1>: CompileConst + ConstElem + AsNaKind,
            naMatrix<T, R2, C2, S2>: CompileConst + ConstElem + AsNaKind,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!(
                    "{}<{}{}{}{}{}>",
                    stringify!($struct_name),
                    T::as_value_kind(),
                    naMatrix::<T, R1, C1, S1>::as_na_kind(),
                    naMatrix::<T, R2, C2, S2>::as_na_kind(),
                    IxVec1::as_na_kind(),
                    IxVec2::as_na_kind()
                );
                compile_ternop!(name, self.sink, self.source, self.ixes.0, self.ixes.1, ctx);
            }
        }
    };
}

#[macro_export]
macro_rules! impl_all_fxn_v {
    ($struct_name:ident, $op:ident, $ix:ty $(, $semantic_contract:path)?) => {
        #[derive(Debug)]
        pub struct $struct_name<T, MatA, MatB, IxVec> {
            pub source: Ref<MatB>,
            pub ixes: Ref<IxVec>,
            pub sink: Ref<MatA>,
            pub _marker: PhantomData<T>,
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
            Ref<naMatrix<T, R1, C1, S1>>: ToValue,
            Ref<naMatrix<T, R2, C2, S2>>: ToValue,
            T: Debug
                + Clone
                + Sync
                + Send
                + 'static
                + PartialEq
                + PartialOrd
                + ConstElem
                + AsValueKind,
            #[cfg(feature = "semantic-compiler")]
            T: CompileConst,
            IxVec: ConstElem + AsNaKind + Debug + AsRef<[$ix]> + FunctionRuntimeType,
            #[cfg(feature = "semantic-compiler")]
            IxVec: CompileConst,
            R1: Dim,
            C1: Dim,
            S1: StorageMut<T, R1, C1> + Clone + Debug,
            R2: Dim,
            C2: Dim,
            S2: Storage<T, R2, C2> + Clone + Debug,
            naMatrix<T, R1, C1, S1>: ConstElem + Debug + AsNaKind + FunctionRuntimeType,
            #[cfg(feature = "semantic-compiler")]
            naMatrix<T, R1, C1, S1>: CompileConst,
            naMatrix<T, R2, C2, S2>: ConstElem + Debug + AsNaKind + FunctionRuntimeType,
            #[cfg(feature = "semantic-compiler")]
            naMatrix<T, R2, C2, S2>: CompileConst,
        {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
                <naMatrix<T, R1, C1, S1> as FunctionRuntimeType>::REPRESENTATION,
                <naMatrix<T, R2, C2, S2> as FunctionRuntimeType>::REPRESENTATION,
                <IxVec as FunctionRuntimeType>::REPRESENTATION,
            );

            fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
                match args {
                    FunctionArgs::Binary(out, arg1, arg2) => {
                        let source: Ref<naMatrix<T, R2, C2, S2>> =
                            arg1.try_function_ref(FunctionArgumentRole::Input(0))?;
                        let ixes: Ref<IxVec> =
                            arg2.try_function_ref(FunctionArgumentRole::Input(1))?;
                        let sink: Ref<naMatrix<T, R1, C1, S1>> =
                            out.try_function_ref(FunctionArgumentRole::Output)?;
                        Ok(Box::new(Self {
                            sink,
                            source,
                            ixes,
                            _marker: PhantomData::default(),
                        }))
                    }
                    _ => Err(MechError::new(
                        IncorrectNumberOfArguments {
                            expected: 3,
                            found: args.len(),
                        },
                        None,
                    )
                    .with_compiler_loc()),
                }
            }
        }
        impl<T, R1, C1, S1, R2, C2, S2, IxVec> MechFunctionImpl
            for $struct_name<T, naMatrix<T, R1, C1, S1>, naMatrix<T, R2, C2, S2>, IxVec>
        where
            Ref<naMatrix<T, R1, C1, S1>>: ToValue,
            T: Debug + Clone + Sync + Send + 'static + PartialEq + PartialOrd,
            IxVec: AsRef<[$ix]> + Debug,
            R1: Dim,
            C1: Dim,
            S1: StorageMut<T, R1, C1> + Clone + Debug,
            R2: Dim,
            C2: Dim,
            S2: Storage<T, R2, C2> + Clone + Debug,
        {
            fn solve_result(&self) -> MResult<()> {
                unsafe {
                    let sink_ptr = &mut *self.sink.as_mut_ptr();
                    let source_ptr = &*self.source.as_ptr();
                    let ix_ptr = &(*self.ixes.as_ptr()).as_ref();
                    $op!(source_ptr, ix_ptr, sink_ptr);
                };
                Ok(())
            }
            fn out(&self) -> LegacyValue {
                self.sink.to_value()
            }
            fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
                let contract: Option<&'static OperationContractDeclaration> = None;
                $(let contract = Some(&*$semantic_contract);)?
                contract
            }
            fn to_string(&self) -> String {
                format!("{:#?}", self)
            }

            fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
                Ok(self.reactive_output_values())
            }
        }
        #[cfg(feature = "semantic-compiler")]
        impl<T, R1, C1, S1, R2, C2, S2, IxVec> MechFunctionCompiler
            for $struct_name<T, naMatrix<T, R1, C1, S1>, naMatrix<T, R2, C2, S2>, IxVec>
        where
            T: CompileConst + ConstElem + AsValueKind,
            IxVec: CompileConst + ConstElem + AsNaKind,
            naMatrix<T, R1, C1, S1>: CompileConst + ConstElem + AsNaKind,
            naMatrix<T, R2, C2, S2>: CompileConst + ConstElem + AsNaKind,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!(
                    "{}<{}{}{}{}>",
                    stringify!($struct_name),
                    T::as_value_kind(),
                    naMatrix::<T, R1, C1, S1>::as_na_kind(),
                    naMatrix::<T, R2, C2, S2>::as_na_kind(),
                    IxVec::as_na_kind()
                );
                compile_binop!(name, self.sink, self.source, self.ixes, ctx);
            }
        }
    };
}
