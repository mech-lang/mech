use crate::intrinsics::*;
use nalgebra::{
    Dim, IsContiguous, Scalar,
    base::{Matrix as naMatrix, Storage, StorageMut},
};
use std::fmt::Debug;
use std::marker::PhantomData;

macro_rules! optional_operation_contract {
    () => {
        None
    };
    ($contract:path) => {
        Some(&*$contract)
    };
}
use std::sync::LazyLock;

static PURE_MATRIX_ELEMENT_ASSIGNMENT_CONTRACT: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| OperationContractDeclaration {
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
            access: AccessMode::ReadWrite,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::ReadModifyWrite {
                base_input: 0,
                regions: RegionPolicy::SingleElement,
            },
            alias: AliasPolicy::MayAlias { input: 0 },
            change_detection: ChangeDetectionPolicy::KernelReported,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    });

// Assign -----------------------------------------------------------------

#[macro_export]
macro_rules! impl_set_all_fxn_s {
    ($struct_name:ident, $op:ident, $ix:ty $(, $semantic_contract:path)?) => {
        #[derive(Debug)]
        pub struct $struct_name<T, MatA, IxVec> {
            pub source: Ref<T>,
            pub ixes: Ref<IxVec>,
            pub sink: Ref<MatA>,
            pub _marker: PhantomData<T>,
        }
        impl<T, R1, C1, S1: 'static, IxVec: 'static> MechFunctionFactory
            for $struct_name<T, naMatrix<T, R1, C1, S1>, IxVec>
        where
            Ref<naMatrix<T, R1, C1, S1>>: ToValue,
            T: Scalar
                + Clone
                + Debug
                + Sync
                + Send
                + 'static
                + ConstElem
                + AsValueKind
                + FunctionPortBacking,
            #[cfg(feature = "semantic-compiler")]
            T: CompileConst,
            IxVec: ConstElem + Debug + AsRef<[$ix]> + AsNaKind + FunctionPortBacking,
            #[cfg(feature = "semantic-compiler")]
            IxVec: CompileConst,
            R1: Dim,
            C1: Dim,
            S1: StorageMut<T, R1, C1> + Clone + Debug,
            naMatrix<T, R1, C1, S1>: ConstElem + Debug + AsNaKind + FunctionStateBacking,
            #[cfg(feature = "semantic-compiler")]
            naMatrix<T, R1, C1, S1>: CompileConst,
        {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
                <naMatrix<T, R1, C1, S1> as FunctionRuntimeType>::REPRESENTATION,
                T::REPRESENTATION,
                IxVec::REPRESENTATION,
            );

            fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
                Self::new_invocation(args.into())
            }
            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                let (out, arg1, arg2) = invocation.expect_binary()?;
                        let source: Ref<T> =
                            arg1.try_ref()?;
                        let ixes: Ref<IxVec> =
                            arg2.try_ref()?;
                        let sink: Ref<naMatrix<T, R1, C1, S1>> =
                            out.try_ref()?;
                        Ok(Box::new(Self {
                            sink,
                            source,
                            ixes,
                            _marker: PhantomData::default(),
                        }))
            }
        }
        impl<T, R1, C1, S1, IxVec> MechFunctionImpl
            for $struct_name<T, naMatrix<T, R1, C1, S1>, IxVec>
        where
            Ref<naMatrix<T, R1, C1, S1>>: ToValue,
            naMatrix<T, R1, C1, S1>: FunctionStateBacking,
            T: Scalar + Clone + Debug + Sync + Send + 'static,
            IxVec: AsRef<[$ix]> + Debug,
            R1: Dim,
            C1: Dim,
            S1: StorageMut<T, R1, C1> + Clone + Debug,
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
            fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
                Some(FunctionStatePort::from_ref(&self.sink))
            }
            fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
                Ok(Some(vec![FunctionStatePort::from_ref(&self.sink)]))
            }
            fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
                optional_operation_contract!($($semantic_contract)?)
            }
            fn to_string(&self) -> String {
                format!("{:#?}", self)
            }

        }
        #[cfg(feature = "semantic-compiler")]
        impl<T, R1, C1, S1, IxVec> MechFunctionCompiler
            for $struct_name<T, naMatrix<T, R1, C1, S1>, IxVec>
        where
            T: CompileConst + ConstElem + AsValueKind,
            IxVec: CompileConst + ConstElem + AsNaKind,
            naMatrix<T, R1, C1, S1>: CompileConst + ConstElem + AsNaKind,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!(
                    "{}<{}{}{}>",
                    stringify!($struct_name),
                    T::as_value_kind(),
                    naMatrix::<T, R1, C1, S1>::as_na_kind(),
                    IxVec::as_na_kind()
                );
                compile_binop!(name, self.sink, self.source, self.ixes, ctx);
            }
        }
    };
}

// x[1] = 1 ------------------------------------------------------------------

#[macro_export]
macro_rules! assign_1d_scalar {
    ($source:expr, $ix:expr, $sink:expr) => {
        ($sink)[$ix - 1] = ($source).clone();
    };
}

#[macro_export]
macro_rules! assign_1d_scalar_b {
    ($source:expr, $ix:expr, $sink:expr) => {
        if $ix {
            for ix in 0..$sink.len() {
                $sink[ix] = $source.clone();
            }
        }
    };
}

#[macro_export]
macro_rules! assign_1d_scalar_vb {
    ($source:expr, $ix:expr, $sink:expr) => {
        if *$ix {
            let len = $sink.len().min($source.len());
            for ix in 0..len {
                $sink[ix] = $source[ix].clone();
            }
        }
    };
}

#[macro_export]
macro_rules! impl_assign_fxn_s {
    ($struct_name:ident, $op:ident, $ix:ty $(, $semantic_contract:ident)?) => {
        #[derive(Debug)]
        pub struct $struct_name<T, MatA> {
            pub source: Ref<T>,
            pub ixes: Ref<$ix>,
            pub sink: Ref<MatA>,
            pub _marker: PhantomData<T>,
        }
        impl<T, R, C, S: 'static> MechFunctionFactory for $struct_name<T, naMatrix<T, R, C, S>>
        where
            Ref<naMatrix<T, R, C, S>>: ToValue,
            T: Scalar
                + Clone
                + Debug
                + Sync
                + Send
                + 'static
                + ConstElem
                + AsValueKind
                + FunctionPortBacking,
            #[cfg(feature = "semantic-compiler")]
            T: CompileConst,
            R: Dim,
            C: Dim,
            S: StorageMut<T, R, C> + Clone + Debug,
            naMatrix<T, R, C, S>: ConstElem + AsNaKind + FunctionStateBacking,
            $ix: FunctionPortBacking,
            #[cfg(feature = "semantic-compiler")]
            naMatrix<T, R, C, S>: CompileConst,
        {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
                <naMatrix<T, R, C, S> as FunctionRuntimeType>::REPRESENTATION,
                T::REPRESENTATION,
                <$ix as FunctionRuntimeType>::REPRESENTATION,
            );

            fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
                Self::new_invocation(args.into())
            }
            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                let (out, arg1, arg2) = invocation.expect_binary()?;
                        let source: Ref<T> =
                            arg1.try_ref()?;
                        let ixes: Ref<$ix> =
                            arg2.try_ref()?;
                        let sink: Ref<naMatrix<T, R, C, S>> =
                            out.try_ref()?;
                        Ok(Box::new(Self {
                            sink,
                            source,
                            ixes,
                            _marker: PhantomData::default(),
                        }))
            }
        }
        impl<T, R, C, S> MechFunctionImpl for $struct_name<T, naMatrix<T, R, C, S>>
        where
            Ref<naMatrix<T, R, C, S>>: ToValue,
            naMatrix<T, R, C, S>: FunctionStateBacking,
            T: Scalar + Clone + Debug + Sync + Send + 'static,
            R: Dim,
            C: Dim,
            S: StorageMut<T, R, C> + Clone + Debug,
        {
            fn solve_result(&self) -> MResult<()> {
                unsafe {
                    let sink_ptr = &mut *self.sink.as_mut_ptr();
                    let ix_val = (*self.ixes.as_ptr()).clone();
                    let source_val = (*self.source.as_ptr()).clone();
                    $op!(source_val, ix_val, sink_ptr);
                };
                Ok(())
            }
            fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
                Some(FunctionStatePort::from_ref(&self.sink))
            }
            fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
                Ok(Some(vec![FunctionStatePort::from_ref(&self.sink)]))
            }
            fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
                optional_operation_contract!($($semantic_contract)?)
            }
            fn to_string(&self) -> String {
                format!("{:#?}", self)
            }

        }
        #[cfg(feature = "semantic-compiler")]
        impl<T, R, C, S> MechFunctionCompiler for $struct_name<T, naMatrix<T, R, C, S>>
        where
            T: CompileConst + ConstElem + AsValueKind,
            naMatrix<T, R, C, S>: CompileConst + ConstElem + AsNaKind,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!(
                    "{}<{}{}>",
                    stringify!($struct_name),
                    T::as_value_kind(),
                    naMatrix::<T, R, C, S>::as_na_kind()
                );
                compile_binop!(name, self.sink, self.source, self.ixes, ctx);
            }
        }
    };
}

impl_assign_fxn_s!(
    Assign1DS,
    assign_1d_scalar,
    usize,
    PURE_MATRIX_AXIS_ZERO_ASSIGNMENT_CONTRACT
);
impl_assign_fxn_s!(
    Assign1DB,
    assign_1d_scalar_b,
    bool,
    PURE_MATRIX_WHOLE_ASSIGNMENT_CONTRACT
);
impl_assign_scalar_fxn_v!(Assign1DVB, assign_1d_scalar_vb, bool);

// x[1..3] = 1 ----------------------------------------------------------------

macro_rules! set_1d_range {
    ($source:expr, $ix:expr, $sink:expr) => {
        for i in 0..($ix).len() {
            ($sink)[($ix)[i] - 1] = ($source).clone();
        }
    };
}

macro_rules! set_1d_range_b {
    ($source:expr, $ix:expr, $sink:expr) => {
        for i in 0..($ix).len() {
            if $ix[i] == true {
                ($sink)[i] = ($source).clone();
            }
        }
    };
}

macro_rules! set_1d_range_vec {
    ($source:expr, $ix:expr, $sink:expr) => {
        for i in 0..($ix).len() {
            ($sink)[($ix)[i] - 1] = ($source)[i].clone();
        }
    };
}

macro_rules! set_1d_range_vec_b {
    ($source:expr, $ix:expr, $sink:expr) => {
        for i in 0..($ix).len() {
            if $ix[i] == true {
                ($sink)[i] = ($source)[i].clone();
            }
        }
    };
}

impl_set_all_fxn_s!(
    Assign1DRS,
    set_1d_range,
    usize,
    PURE_MATRIX_AXIS_ZERO_ASSIGNMENT_CONTRACT
);
impl_set_all_fxn_s!(
    Assign1DRB,
    set_1d_range_b,
    bool,
    PURE_MATRIX_AXIS_ZERO_ASSIGNMENT_CONTRACT
);
impl_all_fxn_v!(
    Assign1DRV,
    set_1d_range_vec,
    usize,
    PURE_MATRIX_AXIS_ZERO_ASSIGNMENT_CONTRACT
);
impl_all_fxn_v!(
    Assign1DRVB,
    set_1d_range_vec_b,
    bool,
    PURE_MATRIX_AXIS_ZERO_ASSIGNMENT_CONTRACT
);

// x[:] = 1 ------------------------------------------------------------------

#[derive(Debug)]
pub struct Set1DAS<T, Sink> {
    pub source: Ref<T>,
    pub sink: Ref<Sink>,
    pub _marker: PhantomData<T>,
}
impl<T, R, C, S> MechFunctionFactory for Set1DAS<T, naMatrix<T, R, C, S>>
where
    Ref<naMatrix<T, R, C, S>>: ToValue,
    T: Debug
        + Clone
        + Sync
        + Send
        + PartialEq
        + 'static
        + ConstElem
        + AsValueKind
        + FunctionPortBacking,
    #[cfg(feature = "semantic-compiler")]
    T: CompileConst,
    R: Dim,
    C: Dim,
    S: StorageMut<T, R, C> + Debug + IsContiguous + 'static,
    naMatrix<T, R, C, S>: ConstElem + Debug + AsNaKind + FunctionStateBacking,
    #[cfg(feature = "semantic-compiler")]
    naMatrix<T, R, C, S>: CompileConst,
{
    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::unary(
        <naMatrix<T, R, C, S> as FunctionRuntimeType>::REPRESENTATION,
        T::REPRESENTATION,
    );

    fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        Self::new_invocation(args.into())
    }
    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, arg1) = invocation.expect_unary()?;
        let source: Ref<T> = arg1.try_ref()?;
        let sink: Ref<naMatrix<T, R, C, S>> = out.try_ref()?;
        Ok(Box::new(Self {
            sink,
            source,
            _marker: PhantomData::default(),
        }))
    }
}
impl<T, R, C, S> MechFunctionImpl for Set1DAS<T, naMatrix<T, R, C, S>>
where
    T: Debug + Clone + Sync + Send + PartialEq + 'static,
    naMatrix<T, R, C, S>: FunctionStateBacking,
    R: Dim,
    C: Dim,
    S: StorageMut<T, R, C> + Debug + IsContiguous,
    Ref<naMatrix<T, R, C, S>>: ToValue,
{
    fn solve_result(&self) -> MResult<()> {
        unsafe {
            let sink = &mut *self.sink.as_mut_ptr();
            let source_val = (*self.source.as_ptr()).clone();
            let slice = sink.as_mut_slice();
            for elem in slice.iter_mut() {
                *elem = source_val.clone();
            }
        };
        Ok(())
    }
    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        Some(FunctionStatePort::from_ref(&self.sink))
    }
    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        Ok(Some(vec![FunctionStatePort::from_ref(&self.sink)]))
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}
#[cfg(feature = "semantic-compiler")]
impl<T, R, C, S> MechFunctionCompiler for Set1DAS<T, naMatrix<T, R, C, S>>
where
    T: CompileConst + ConstElem + AsValueKind,
    naMatrix<T, R, C, S>: CompileConst + ConstElem + AsNaKind,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!(
            "Set1DAS<{}{}>",
            T::as_value_kind(),
            naMatrix::<T, R, C, S>::as_na_kind()
        );
        compile_unop!(name, self.sink, self.source, ctx);
    }
}

#[derive(Debug)]
pub struct Assign2DSSS<T, MatA> {
    pub source: Ref<T>,
    pub ixes: (Ref<usize>, Ref<usize>),
    pub sink: Ref<MatA>,
    pub _marker: PhantomData<T>,
}
impl<T, R1, C1, S1: 'static> MechFunctionFactory for Assign2DSSS<T, naMatrix<T, R1, C1, S1>>
where
    Ref<naMatrix<T, R1, C1, S1>>: ToValue,
    naMatrix<T, R1, C1, S1>: FunctionStateBacking,
    T: Scalar
        + Clone
        + Debug
        + Sync
        + Send
        + 'static
        + ConstElem
        + AsValueKind
        + FunctionPortBacking,
    #[cfg(feature = "semantic-compiler")]
    T: CompileConst,
    R1: Dim,
    C1: Dim,
    S1: StorageMut<T, R1, C1> + Clone + Debug,
    naMatrix<T, R1, C1, S1>: ConstElem + AsNaKind + FunctionStateBacking,
    #[cfg(feature = "semantic-compiler")]
    naMatrix<T, R1, C1, S1>: CompileConst,
{
    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::ternary(
        <naMatrix<T, R1, C1, S1> as FunctionRuntimeType>::REPRESENTATION,
        T::REPRESENTATION,
        <usize as FunctionRuntimeType>::REPRESENTATION,
        <usize as FunctionRuntimeType>::REPRESENTATION,
    );

    fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        Self::new_invocation(args.into())
    }
    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, arg1, arg2, arg3) = invocation.expect_ternary()?;
        let source: Ref<T> = arg1.try_ref()?;
        let ix1: Ref<usize> = arg2.try_ref()?;
        let ix2: Ref<usize> = arg3.try_ref()?;
        let sink: Ref<naMatrix<T, R1, C1, S1>> = out.try_ref()?;
        Ok(Box::new(Self {
            sink,
            source,
            ixes: (ix1, ix2),
            _marker: PhantomData,
        }))
    }
}
impl<T, R1, C1, S1> MechFunctionImpl for Assign2DSSS<T, naMatrix<T, R1, C1, S1>>
where
    Ref<naMatrix<T, R1, C1, S1>>: ToValue,
    naMatrix<T, R1, C1, S1>: FunctionStateBacking,
    T: Scalar + Clone + Debug + Sync + Send + 'static,
    R1: Dim,
    C1: Dim,
    S1: StorageMut<T, R1, C1> + Clone + Debug,
{
    fn solve_result(&self) -> MResult<()> {
        unsafe {
            let sink_ptr = &mut *self.sink.as_mut_ptr();
            let source_val = (*self.source.as_ptr()).clone();
            let r = (*self.ixes.0.as_ptr()).clone();
            let c = (*self.ixes.1.as_ptr()).clone();
            sink_ptr[(r - 1, c - 1)] = source_val;
        };
        Ok(())
    }
    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        Some(FunctionStatePort::from_ref(&self.sink))
    }
    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        Ok(Some(vec![FunctionStatePort::from_ref(&self.sink)]))
    }
    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_MATRIX_ELEMENT_ASSIGNMENT_CONTRACT)
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}
#[cfg(feature = "semantic-compiler")]
impl<T, R1, C1, S1> MechFunctionCompiler for Assign2DSSS<T, naMatrix<T, R1, C1, S1>>
where
    T: CompileConst + ConstElem + AsValueKind,
    naMatrix<T, R1, C1, S1>: CompileConst + ConstElem + AsNaKind,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!(
            "Assign2DSSS<{}{}>",
            T::as_value_kind(),
            naMatrix::<T, R1, C1, S1>::as_na_kind()
        );
        compile_ternop!(name, self.sink, self.source, self.ixes.0, self.ixes.1, ctx);
    }
}

macro_rules! assign_2d_all_scalar {
    ($source:expr, $ix:expr, $sink:expr) => {
        for i in 0..$sink.nrows() {
            ($sink).column_mut($ix - 1)[i] = ($source).clone();
        }
    };
}

macro_rules! assign_2d_all_vector {
    ($source:expr, $ix:expr, $sink:expr) => {
        for i in 0..$sink.nrows() {
            ($sink).column_mut($ix - 1)[i] = ($source)[i].clone();
        }
    };
}

#[macro_export]
macro_rules! impl_assign_scalar_fxn_v {
    ($struct_name:ident, $op:ident, $ix:ty) => {
        #[derive(Debug)]
        pub struct $struct_name<T, MatA, MatB> {
            pub source: Ref<MatB>,
            pub ixes: Ref<$ix>,
            pub sink: Ref<MatA>,
            pub _marker: PhantomData<T>,
        }
        impl<T, R1: 'static, C1: 'static, S1: 'static, R2: 'static, C2: 'static, S2: 'static>
            MechFunctionFactory
            for $struct_name<T, naMatrix<T, R1, C1, S1>, naMatrix<T, R2, C2, S2>>
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
            R1: Dim,
            C1: Dim,
            S1: StorageMut<T, R1, C1> + Clone + Debug,
            R2: Dim,
            C2: Dim,
            S2: Storage<T, R2, C2> + Clone + Debug,
            naMatrix<T, R1, C1, S1>: ConstElem + Debug + AsNaKind + FunctionStateBacking,
            #[cfg(feature = "semantic-compiler")]
            naMatrix<T, R1, C1, S1>: CompileConst,
            naMatrix<T, R2, C2, S2>: ConstElem + Debug + AsNaKind + FunctionPortBacking,
            $ix: FunctionPortBacking,
            #[cfg(feature = "semantic-compiler")]
            naMatrix<T, R2, C2, S2>: CompileConst,
        {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
                <naMatrix<T, R1, C1, S1> as FunctionRuntimeType>::REPRESENTATION,
                <naMatrix<T, R2, C2, S2> as FunctionRuntimeType>::REPRESENTATION,
                <$ix as FunctionRuntimeType>::REPRESENTATION,
            );

            fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
                Self::new_invocation(args.into())
            }
            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                let (out, arg1, arg2) = invocation.expect_binary()?;
                let source: Ref<naMatrix<T, R2, C2, S2>> = arg1.try_ref()?;
                let ixes: Ref<$ix> = arg2.try_ref()?;
                let sink: Ref<naMatrix<T, R1, C1, S1>> = out.try_ref()?;
                Ok(Box::new(Self {
                    sink,
                    source,
                    ixes,
                    _marker: PhantomData::default(),
                }))
            }
        }
        impl<T, R1, C1, S1, R2, C2, S2> MechFunctionImpl
            for $struct_name<T, naMatrix<T, R1, C1, S1>, naMatrix<T, R2, C2, S2>>
        where
            Ref<naMatrix<T, R1, C1, S1>>: ToValue,
            naMatrix<T, R1, C1, S1>: FunctionStateBacking,
            T: Debug + Clone + Sync + Send + 'static + PartialEq + PartialOrd,
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
                    let ix_ptr = &(*self.ixes.as_ptr());
                    $op!(source_ptr, ix_ptr, sink_ptr);
                };
                Ok(())
            }
            fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
                Some(FunctionStatePort::from_ref(&self.sink))
            }
            fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
                Ok(Some(vec![FunctionStatePort::from_ref(&self.sink)]))
            }
            fn to_string(&self) -> String {
                format!("{:#?}", self)
            }
        }
        #[cfg(feature = "semantic-compiler")]
        impl<T, R1, C1, S1, R2, C2, S2> MechFunctionCompiler
            for $struct_name<T, naMatrix<T, R1, C1, S1>, naMatrix<T, R2, C2, S2>>
        where
            T: CompileConst + ConstElem + AsValueKind,
            naMatrix<T, R1, C1, S1>: CompileConst + ConstElem + AsNaKind,
            naMatrix<T, R2, C2, S2>: CompileConst + ConstElem + AsNaKind,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!(
                    "{}<{}{}{}>",
                    stringify!($struct_name),
                    T::as_value_kind(),
                    naMatrix::<T, R1, C1, S1>::as_na_kind(),
                    naMatrix::<T, R2, C2, S2>::as_na_kind()
                );
                compile_binop!(name, self.sink, self.source, self.ixes, ctx);
            }
        }
    };
}

impl_assign_fxn_s!(Assign2DASS, assign_2d_all_scalar, usize);
impl_assign_scalar_fxn_v!(Assign2DASV, assign_2d_all_vector, usize);

macro_rules! assign_2d_scalar_all_scalar {
    ($source:expr, $ix:expr, $sink:expr) => {
        for i in 0..$sink.ncols() {
            ($sink).row_mut($ix - 1)[i] = ($source).clone();
        }
    };
}

macro_rules! assign_2d_scalar_all_vector {
    ($source:expr, $ix:expr, $sink:expr) => {
        for i in 0..$sink.ncols() {
            ($sink).row_mut($ix - 1)[i] = ($source)[i].clone();
        }
    };
}

impl_assign_fxn_s!(Assign2DSAS, assign_2d_scalar_all_scalar, usize);
impl_assign_scalar_fxn_v!(Assign2DSAV, assign_2d_scalar_all_vector, usize);

macro_rules! assign_2d_range_scalar {
    ($sink:expr, $ix1:expr, $ix2:expr, $source:expr) => {
        let mut col = ($sink).column_mut($ix2 - 1);
        for &rix in ($ix1).iter() {
            col[rix - 1] = ($source).clone();
        }
    };
}

macro_rules! assign_2d_range_scalar_v {
    ($sink:expr, $ix1:expr, $ix2:expr, $source:expr) => {
        let mut col = ($sink).column_mut($ix2 - 1);
        for (i, &rix) in ($ix1).iter().enumerate() {
            col[rix - 1] = ($source)[i].clone();
        }
    };
}

macro_rules! assign_2d_range_scalar_b {
    ($sink:expr, $ix1:expr, $ix2:expr, $source:expr) => {
        let mut col = ($sink).column_mut($ix2 - 1);
        for (rix, &is_selected) in ($ix1).iter().enumerate() {
            if is_selected {
                col[rix] = ($source).clone();
            }
        }
    };
}

macro_rules! assign_2d_range_scalar_vb {
    ($sink:expr, $ix1:expr, $ix2:expr, $source:expr) => {
        let mut col = ($sink).column_mut($ix2 - 1);
        for (rix, &is_selected) in ($ix1).iter().enumerate() {
            if is_selected {
                col[rix] = ($source)[rix].clone();
            }
        }
    };
}

#[macro_export]
macro_rules! impl_assign_range_scalar_fxn_s {
    ($struct_name:ident, $op:tt, $ix:ty) => {
        #[derive(Debug)]
        pub struct $struct_name<T, MatA, IxVec> {
            pub source: Ref<T>,
            pub ixes: (Ref<IxVec>, Ref<usize>),
            pub sink: Ref<MatA>,
            pub _marker: PhantomData<T>,
        }
        impl<T, R, C, S: 'static, IxVec: 'static> MechFunctionFactory
            for $struct_name<T, na::Matrix<T, R, C, S>, IxVec>
        where
            Ref<naMatrix<T, R, C, S>>: ToValue,
            T: Scalar
                + Clone
                + Debug
                + Sync
                + Send
                + 'static
                + ConstElem
                + AsValueKind
                + FunctionPortBacking,
            #[cfg(feature = "semantic-compiler")]
            T: CompileConst,
            IxVec: ConstElem + Debug + AsRef<[$ix]> + AsNaKind + FunctionPortBacking,
            #[cfg(feature = "semantic-compiler")]
            IxVec: CompileConst,
            R: Dim,
            C: Dim,
            S: StorageMut<T, R, C> + Clone + Debug,
            naMatrix<T, R, C, S>: ConstElem + Debug + AsNaKind + FunctionStateBacking,
            #[cfg(feature = "semantic-compiler")]
            naMatrix<T, R, C, S>: CompileConst,
        {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::ternary(
                <naMatrix<T, R, C, S> as FunctionRuntimeType>::REPRESENTATION,
                T::REPRESENTATION,
                IxVec::REPRESENTATION,
                <usize as FunctionRuntimeType>::REPRESENTATION,
            );

            fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
                Self::new_invocation(args.into())
            }
            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                let (out, arg1, arg2, arg3) = invocation.expect_ternary()?;
                let source: Ref<T> = arg1.try_ref()?;
                let ix1: Ref<IxVec> = arg2.try_ref()?;
                let ix2: Ref<usize> = arg3.try_ref()?;
                let sink: Ref<na::Matrix<T, R, C, S>> = out.try_ref()?;
                Ok(Box::new(Self {
                    sink,
                    source,
                    ixes: (ix1, ix2),
                    _marker: PhantomData,
                }))
            }
        }
        impl<T, R, C, S, IxVec> MechFunctionImpl for $struct_name<T, na::Matrix<T, R, C, S>, IxVec>
        where
            Ref<naMatrix<T, R, C, S>>: ToValue,
            naMatrix<T, R, C, S>: FunctionStateBacking,
            T: Scalar + Clone + Debug + Sync + Send + 'static,
            IxVec: AsRef<[$ix]> + Debug,
            R: Dim,
            C: Dim,
            S: StorageMut<T, R, C> + Clone + Debug,
        {
            fn solve_result(&self) -> MResult<()> {
                unsafe {
                    let sink = &mut *self.sink.as_mut_ptr();
                    let source = &*self.source.as_ptr();
                    let ix1 = (*self.ixes.0.as_ptr()).as_ref();
                    let ix2 = (*self.ixes.1.as_ptr());
                    $op!(sink, ix1, ix2, source);
                };
                Ok(())
            }
            fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
                Some(FunctionStatePort::from_ref(&self.sink))
            }
            fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
                Ok(Some(vec![FunctionStatePort::from_ref(&self.sink)]))
            }
            fn to_string(&self) -> String {
                format!("{:#?}", self)
            }
        }
        #[cfg(feature = "semantic-compiler")]
        impl<T, R, C, S, IxVec> MechFunctionCompiler
            for $struct_name<T, na::Matrix<T, R, C, S>, IxVec>
        where
            T: CompileConst + ConstElem + AsValueKind,
            IxVec: CompileConst + ConstElem + AsNaKind,
            naMatrix<T, R, C, S>: CompileConst + ConstElem + AsNaKind,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!(
                    "{}<{}{}{}>",
                    stringify!($struct_name),
                    T::as_value_kind(),
                    naMatrix::<T, R, C, S>::as_na_kind(),
                    IxVec::as_na_kind()
                );
                compile_ternop!(name, self.sink, self.source, self.ixes.0, self.ixes.1, ctx);
            }
        }
    };
}

#[macro_export]
macro_rules! impl_assign_range_scalar_fxn_v {
    ($struct_name:ident, $op:ident, $ix:ty) => {
        #[derive(Debug)]
        pub struct $struct_name<T, MatA, MatB, IxVec> {
            pub source: Ref<MatB>,
            pub ixes: (Ref<IxVec>, Ref<usize>),
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
            IxVec: ConstElem + AsNaKind + Debug + AsRef<[$ix]> + FunctionPortBacking,
            #[cfg(feature = "semantic-compiler")]
            IxVec: CompileConst,
            R1: Dim,
            C1: Dim,
            S1: StorageMut<T, R1, C1> + Clone + Debug,
            R2: Dim,
            C2: Dim,
            S2: Storage<T, R2, C2> + Clone + Debug,
            naMatrix<T, R1, C1, S1>: ConstElem + Debug + AsNaKind + FunctionStateBacking,
            #[cfg(feature = "semantic-compiler")]
            naMatrix<T, R1, C1, S1>: CompileConst,
            naMatrix<T, R2, C2, S2>: ConstElem + Debug + AsNaKind + FunctionPortBacking,
            #[cfg(feature = "semantic-compiler")]
            naMatrix<T, R2, C2, S2>: CompileConst,
        {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::ternary(
                <naMatrix<T, R1, C1, S1> as FunctionRuntimeType>::REPRESENTATION,
                <naMatrix<T, R2, C2, S2> as FunctionRuntimeType>::REPRESENTATION,
                IxVec::REPRESENTATION,
                <usize as FunctionRuntimeType>::REPRESENTATION,
            );

            fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
                Self::new_invocation(args.into())
            }
            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                let (out, arg1, arg2, arg3) = invocation.expect_ternary()?;
                let source: Ref<naMatrix<T, R2, C2, S2>> = arg1.try_ref()?;
                let ix1: Ref<IxVec> = arg2.try_ref()?;
                let ix2: Ref<usize> = arg3.try_ref()?;
                let sink: Ref<naMatrix<T, R1, C1, S1>> = out.try_ref()?;
                Ok(Box::new(Self {
                    sink,
                    source,
                    ixes: (ix1, ix2),
                    _marker: PhantomData::default(),
                }))
            }
        }
        impl<T, R1, C1, S1, R2, C2, S2, IxVec> MechFunctionImpl
            for $struct_name<T, naMatrix<T, R1, C1, S1>, naMatrix<T, R2, C2, S2>, IxVec>
        where
            Ref<naMatrix<T, R1, C1, S1>>: ToValue,
            naMatrix<T, R1, C1, S1>: FunctionStateBacking,
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
                    let sink = &mut *self.sink.as_mut_ptr();
                    let source = &*self.source.as_ptr();
                    let ix1 = (*self.ixes.0.as_ptr()).as_ref();
                    let ix2 = (*self.ixes.1.as_ptr());
                    $op!(sink, ix1, ix2, source);
                };
                Ok(())
            }
            fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
                Some(FunctionStatePort::from_ref(&self.sink))
            }
            fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
                Ok(Some(vec![FunctionStatePort::from_ref(&self.sink)]))
            }
            fn to_string(&self) -> String {
                format!("{:#?}", self)
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
                compile_ternop!(name, self.sink, self.source, self.ixes.0, self.ixes.1, ctx);
            }
        }
    };
}

impl_assign_range_scalar_fxn_s!(Assign2DSSMD, assign_2d_range_scalar, usize);

impl_assign_range_scalar_fxn_s!(Assign2DRSS, assign_2d_range_scalar, usize);
impl_assign_range_scalar_fxn_s!(Assign2DRSB, assign_2d_range_scalar_b, bool);
impl_assign_range_scalar_fxn_v!(Assign2DRSV, assign_2d_range_scalar_v, usize);
impl_assign_range_scalar_fxn_v!(Assign2DRSVB, assign_2d_range_scalar_vb, bool);

macro_rules! assign_2d_scalar_range {
    ($sink:expr, $ix1:expr, $ix2:expr, $source:expr) => {
        for i in 0..($ix2).len() {
            let cix = $ix2[i] - 1;
            ($sink).row_mut($ix1 - 1)[cix] = ($source).clone();
        }
    };
}

macro_rules! assign_2d_scalar_range_v {
    ($sink:expr, $ix1:expr, $ix2:expr, $source:expr) => {
        for i in 0..($ix2).len() {
            let cix = $ix2[i] - 1;
            ($sink).row_mut($ix1 - 1)[cix] = ($source)[i].clone();
        }
    };
}

macro_rules! assign_2d_scalar_range_b {
    ($sink:expr, $ix1:expr, $ix2:expr, $source:expr) => {
        for cix in 0..($ix2).len() {
            if $ix2[cix] == true {
                ($sink).row_mut($ix1 - 1)[cix] = ($source).clone();
            }
        }
    };
}

macro_rules! assign_2d_scalar_range_vb {
    ($sink:expr, $ix1:expr, $ix2:expr, $source:expr) => {
        for cix in 0..($ix2).len() {
            if $ix2[cix] == true {
                ($sink).row_mut($ix1 - 1)[cix] = ($source)[cix].clone();
            }
        }
    };
}

#[macro_export]
macro_rules! impl_assign_scalar_range_fxn_s {
    ($struct_name:ident, $op:tt, $ix:ty) => {
        #[derive(Debug)]
        pub struct $struct_name<T, MatA, IxVec> {
            pub source: Ref<T>,
            pub ixes: (Ref<usize>, Ref<IxVec>),
            pub sink: Ref<MatA>,
            pub _marker: PhantomData<T>,
        }
        impl<T, R, C, S: 'static, IxVec: 'static> MechFunctionFactory
            for $struct_name<T, na::Matrix<T, R, C, S>, IxVec>
        where
            Ref<naMatrix<T, R, C, S>>: ToValue,
            T: Scalar
                + Clone
                + Debug
                + Sync
                + Send
                + 'static
                + ConstElem
                + AsValueKind
                + FunctionPortBacking,
            #[cfg(feature = "semantic-compiler")]
            T: CompileConst,
            IxVec: ConstElem + Debug + AsRef<[$ix]> + AsNaKind + FunctionPortBacking,
            #[cfg(feature = "semantic-compiler")]
            IxVec: CompileConst,
            R: Dim,
            C: Dim,
            S: StorageMut<T, R, C> + Clone + Debug,
            naMatrix<T, R, C, S>: ConstElem + Debug + AsNaKind + FunctionStateBacking,
            #[cfg(feature = "semantic-compiler")]
            naMatrix<T, R, C, S>: CompileConst,
        {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::ternary(
                <naMatrix<T, R, C, S> as FunctionRuntimeType>::REPRESENTATION,
                T::REPRESENTATION,
                <usize as FunctionRuntimeType>::REPRESENTATION,
                IxVec::REPRESENTATION,
            );

            fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
                Self::new_invocation(args.into())
            }
            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                let (out, arg1, arg2, arg3) = invocation.expect_ternary()?;
                let source: Ref<T> = arg1.try_ref()?;
                let ix1: Ref<usize> = arg2.try_ref()?;
                let ix2: Ref<IxVec> = arg3.try_ref()?;
                let sink: Ref<na::Matrix<T, R, C, S>> = out.try_ref()?;
                Ok(Box::new(Self {
                    sink,
                    source,
                    ixes: (ix1, ix2),
                    _marker: PhantomData,
                }))
            }
        }
        impl<T, R, C, S, IxVec> MechFunctionImpl for $struct_name<T, na::Matrix<T, R, C, S>, IxVec>
        where
            Ref<naMatrix<T, R, C, S>>: ToValue,
            naMatrix<T, R, C, S>: FunctionStateBacking,
            T: Scalar + Clone + Debug + Sync + Send + 'static,
            IxVec: AsRef<[$ix]> + Debug,
            R: Dim,
            C: Dim,
            S: StorageMut<T, R, C> + Clone + Debug,
        {
            fn solve_result(&self) -> MResult<()> {
                unsafe {
                    let sink = &mut *self.sink.as_mut_ptr();
                    let source = &*self.source.as_ptr();
                    let ix1 = (*self.ixes.0.as_ptr());
                    let ix2 = (*self.ixes.1.as_ptr()).as_ref();
                    $op!(sink, ix1, ix2, source);
                };
                Ok(())
            }
            fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
                Some(FunctionStatePort::from_ref(&self.sink))
            }
            fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
                Ok(Some(vec![FunctionStatePort::from_ref(&self.sink)]))
            }
            fn to_string(&self) -> String {
                format!("{:#?}", self)
            }
        }
        #[cfg(feature = "semantic-compiler")]
        impl<T, R, C, S, IxVec> MechFunctionCompiler
            for $struct_name<T, na::Matrix<T, R, C, S>, IxVec>
        where
            T: CompileConst + ConstElem + AsValueKind,
            IxVec: CompileConst + ConstElem + AsNaKind,
            naMatrix<T, R, C, S>: CompileConst + ConstElem + AsNaKind,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!(
                    "{}<{}{}{}>",
                    stringify!($struct_name),
                    T::as_value_kind(),
                    naMatrix::<T, R, C, S>::as_na_kind(),
                    IxVec::as_na_kind()
                );
                compile_ternop!(name, self.sink, self.source, self.ixes.0, self.ixes.1, ctx);
            }
        }
    };
}

#[macro_export]
macro_rules! impl_assign_scalar_range_fxn_v {
    ($struct_name:ident, $op:ident, $ix:ty) => {
        #[derive(Debug)]
        pub struct $struct_name<T, MatA, MatB, IxVec> {
            pub source: Ref<MatB>,
            pub ixes: (Ref<usize>, Ref<IxVec>),
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
            IxVec: ConstElem + AsNaKind + Debug + AsRef<[$ix]> + FunctionPortBacking,
            #[cfg(feature = "semantic-compiler")]
            IxVec: CompileConst,
            R1: Dim,
            C1: Dim,
            S1: StorageMut<T, R1, C1> + Clone + Debug,
            R2: Dim,
            C2: Dim,
            S2: Storage<T, R2, C2> + Clone + Debug,
            naMatrix<T, R1, C1, S1>: ConstElem + Debug + AsNaKind + FunctionStateBacking,
            #[cfg(feature = "semantic-compiler")]
            naMatrix<T, R1, C1, S1>: CompileConst,
            naMatrix<T, R2, C2, S2>: ConstElem + Debug + AsNaKind + FunctionPortBacking,
            #[cfg(feature = "semantic-compiler")]
            naMatrix<T, R2, C2, S2>: CompileConst,
        {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::ternary(
                <naMatrix<T, R1, C1, S1> as FunctionRuntimeType>::REPRESENTATION,
                <naMatrix<T, R2, C2, S2> as FunctionRuntimeType>::REPRESENTATION,
                <usize as FunctionRuntimeType>::REPRESENTATION,
                IxVec::REPRESENTATION,
            );

            fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
                Self::new_invocation(args.into())
            }
            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                let (out, arg1, arg2, arg3) = invocation.expect_ternary()?;
                let source: Ref<naMatrix<T, R2, C2, S2>> = arg1.try_ref()?;
                let ix1: Ref<usize> = arg2.try_ref()?;
                let ix2: Ref<IxVec> = arg3.try_ref()?;
                let sink: Ref<naMatrix<T, R1, C1, S1>> = out.try_ref()?;
                Ok(Box::new(Self {
                    sink,
                    source,
                    ixes: (ix1, ix2),
                    _marker: PhantomData::default(),
                }))
            }
        }
        impl<T, R1, C1, S1, R2, C2, S2, IxVec> MechFunctionImpl
            for $struct_name<T, naMatrix<T, R1, C1, S1>, naMatrix<T, R2, C2, S2>, IxVec>
        where
            Ref<naMatrix<T, R1, C1, S1>>: ToValue,
            naMatrix<T, R1, C1, S1>: FunctionStateBacking,
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
                    let sink = &mut *self.sink.as_mut_ptr();
                    let source = &*self.source.as_ptr();
                    let ix1 = (*self.ixes.0.as_ptr());
                    let ix2 = (*self.ixes.1.as_ptr()).as_ref();
                    $op!(sink, ix1, ix2, source);
                };
                Ok(())
            }
            fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
                Some(FunctionStatePort::from_ref(&self.sink))
            }
            fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
                Ok(Some(vec![FunctionStatePort::from_ref(&self.sink)]))
            }
            fn to_string(&self) -> String {
                format!("{:#?}", self)
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
                compile_ternop!(name, self.sink, self.source, self.ixes.0, self.ixes.1, ctx);
            }
        }
    };
}

impl_assign_scalar_range_fxn_s!(Assign2DSRS, assign_2d_scalar_range, usize);
impl_assign_scalar_range_fxn_s!(Assign2DSRB, assign_2d_scalar_range_b, bool);
impl_assign_scalar_range_fxn_v!(Assign2DSRV, assign_2d_scalar_range_v, usize);
impl_assign_scalar_range_fxn_v!(Assign2DSRVB, assign_2d_scalar_range_vb, bool);

macro_rules! assign_2d_range_range {
    ($sink:expr, $ix1:expr, $ix2:expr, $source:expr) => {
        for rix in 0..($ix1).len() {
            let r = $ix1[rix] - 1;
            for cix in 0..($ix2).len() {
                let c = $ix2[cix] - 1;
                ($sink)[(r, c)] = ($source).clone();
            }
        }
    };
}

macro_rules! assign_2d_range_range_v {
    ($sink:expr, $ix1:expr, $ix2:expr, $source:expr) => {
        for rix in 0..($ix1).len() {
            let r = $ix1[rix] - 1;
            for cix in 0..($ix2).len() {
                let c = $ix2[cix] - 1;
                ($sink)[(r, c)] = ($source)[rix * ($ix2).len() + cix].clone();
            }
        }
    };
}

macro_rules! assign_2d_range_range_b {
    ($sink:expr, $ix1:expr, $ix2:expr, $source:expr) => {
        for r in 0..($ix1).len() {
            if $ix1[r] == true {
                for c in 0..($ix2).len() {
                    if $ix2[c] == true {
                        ($sink)[(r, c)] = ($source).clone();
                    }
                }
            }
        }
    };
}

macro_rules! assign_2d_range_range_vb {
    ($sink:expr, $ix1:expr, $ix2:expr, $source:expr) => {
        for r in 0..($ix1).len() {
            if $ix1[r] == true {
                for c in 0..($ix2).len() {
                    if $ix2[c] == true {
                        ($sink)[(r, c)] = ($source)[r * ($ix2).len() + c].clone();
                    }
                }
            }
        }
    };
}

macro_rules! assign_2d_range_range_bu {
    ($sink:expr, $ix1:expr, $ix2:expr, $source:expr) => {
        for r in 0..($ix1).len() {
            if $ix1[r] == true {
                for cix in 0..($ix2).len() {
                    let c = $ix2[cix] - 1;
                    ($sink)[(r, c)] = ($source).clone();
                }
            }
        }
    };
}

macro_rules! assign_2d_range_range_vbu {
    ($sink:expr, $ix1:expr, $ix2:expr, $source:expr) => {
        let nrows = $sink.nrows();
        for cix in 0..($ix2).len() {
            let c = $ix2[cix] - 1;
            for r in 0..($ix1).len() {
                if $ix1[r] {
                    let offset = r + c * nrows;
                    ($sink)[(r, c)] = ($source)[offset].clone();
                }
            }
        }
    };
}

macro_rules! assign_2d_range_range_ub {
    ($sink:expr, $ix1:expr, $ix2:expr, $source:expr) => {
        for r in 0..$ix1.len() {
            if $ix1[r] != 0 {
                for c in 0..$ix2.len() {
                    if $ix2[c] {
                        ($sink)[(r, c)] = $source.clone();
                    }
                }
            }
        }
    };
}

macro_rules! assign_2d_range_range_vub {
    ($sink:expr, $ix1:expr, $ix2:expr, $source:expr) => {
        let nrows = $sink.nrows();
        for c in 0..$ix2.len() {
            if $ix2[c] {
                for rix in 0..$ix1.len() {
                    let r = $ix1[rix] - 1;
                    let offset = r + c * nrows;
                    ($sink)[(r, c)] = ($source)[offset].clone();
                }
            }
        }
    };
}

#[macro_export]
macro_rules! impl_assign_range_range_fxn_s {
    ($struct_name:ident, $op:tt, $ix1:ty, $ix2:ty) => {
        #[derive(Debug)]
        pub struct $struct_name<T, MatA, IxVec1, IxVec2> {
            pub source: Ref<T>,
            pub ixes: (Ref<IxVec1>, Ref<IxVec2>),
            pub sink: Ref<MatA>,
            pub _marker: PhantomData<T>,
        }
        impl<T, R, C, S: 'static, IxVec1: 'static, IxVec2: 'static> MechFunctionFactory
            for $struct_name<T, na::Matrix<T, R, C, S>, IxVec1, IxVec2>
        where
            Ref<naMatrix<T, R, C, S>>: ToValue,
            T: Scalar
                + Clone
                + Debug
                + Sync
                + Send
                + 'static
                + ConstElem
                + AsValueKind
                + FunctionPortBacking,
            #[cfg(feature = "semantic-compiler")]
            T: CompileConst,
            IxVec1: ConstElem + Debug + AsRef<[$ix1]> + AsNaKind + FunctionPortBacking,
            #[cfg(feature = "semantic-compiler")]
            IxVec1: CompileConst,
            IxVec2: ConstElem + Debug + AsRef<[$ix2]> + AsNaKind + FunctionPortBacking,
            #[cfg(feature = "semantic-compiler")]
            IxVec2: CompileConst,
            R: Dim,
            C: Dim,
            S: StorageMut<T, R, C> + Clone + Debug,
            naMatrix<T, R, C, S>: ConstElem + Debug + AsNaKind + FunctionStateBacking,
            #[cfg(feature = "semantic-compiler")]
            naMatrix<T, R, C, S>: CompileConst,
        {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::ternary(
                <naMatrix<T, R, C, S> as FunctionRuntimeType>::REPRESENTATION,
                T::REPRESENTATION,
                IxVec1::REPRESENTATION,
                IxVec2::REPRESENTATION,
            );

            fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
                Self::new_invocation(args.into())
            }
            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                let (out, arg1, arg2, arg3) = invocation.expect_ternary()?;
                let source: Ref<T> = arg1.try_ref()?;
                let ix1: Ref<IxVec1> = arg2.try_ref()?;
                let ix2: Ref<IxVec2> = arg3.try_ref()?;
                let sink: Ref<na::Matrix<T, R, C, S>> = out.try_ref()?;
                Ok(Box::new(Self {
                    sink,
                    source,
                    ixes: (ix1, ix2),
                    _marker: PhantomData,
                }))
            }
        }
        impl<T, R, C, S, IxVec1, IxVec2> MechFunctionImpl
            for $struct_name<T, na::Matrix<T, R, C, S>, IxVec1, IxVec2>
        where
            Ref<naMatrix<T, R, C, S>>: ToValue,
            naMatrix<T, R, C, S>: FunctionStateBacking,
            T: Scalar + Clone + Debug + Sync + Send + 'static,
            IxVec1: AsRef<[$ix1]> + Debug,
            IxVec2: AsRef<[$ix2]> + Debug,
            R: Dim,
            C: Dim,
            S: StorageMut<T, R, C> + Clone + Debug,
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
            fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
                Some(FunctionStatePort::from_ref(&self.sink))
            }
            fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
                Ok(Some(vec![FunctionStatePort::from_ref(&self.sink)]))
            }
            fn to_string(&self) -> String {
                format!("{:#?}", self)
            }
        }
        #[cfg(feature = "semantic-compiler")]
        impl<T, R, C, S, IxVec1, IxVec2> MechFunctionCompiler
            for $struct_name<T, na::Matrix<T, R, C, S>, IxVec1, IxVec2>
        where
            T: CompileConst + ConstElem + AsValueKind,
            IxVec1: CompileConst + ConstElem + AsNaKind,
            IxVec2: CompileConst + ConstElem + AsNaKind,
            naMatrix<T, R, C, S>: CompileConst + ConstElem + AsNaKind,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!(
                    "{}<{}{}{}{}>",
                    stringify!($struct_name),
                    T::as_value_kind(),
                    naMatrix::<T, R, C, S>::as_na_kind(),
                    IxVec1::as_na_kind(),
                    IxVec2::as_na_kind()
                );
                compile_ternop!(name, self.sink, self.source, self.ixes.0, self.ixes.1, ctx);
            }
        }
    };
}

impl_assign_range_range_fxn_s!(Assign2DRRS, assign_2d_range_range, usize, usize);
impl_range_range_fxn_v!(Assign2DRRV, assign_2d_range_range_v, usize, usize);

impl_assign_range_range_fxn_s!(Assign2DRRBB, assign_2d_range_range_b, bool, bool);
impl_range_range_fxn_v!(Assign2DRRVBB, assign_2d_range_range_vb, bool, bool);

impl_assign_range_range_fxn_s!(Assign2DRRBU, assign_2d_range_range_bu, bool, usize);
impl_range_range_fxn_v!(Assign2DRRVBU, assign_2d_range_range_vbu, bool, usize);

impl_assign_range_range_fxn_s!(Assign2DRRUB, assign_2d_range_range_ub, usize, bool);
impl_range_range_fxn_v!(Assign2DRRVUB, assign_2d_range_range_vub, usize, bool);

// x[:,1..3] = 1 ------------------------------------------------------------------

macro_rules! assign_2d_all_range {
    ($source:expr, $ix:expr, $sink:expr) => {
        for cix in $ix.iter() {
            for rix in 0..($sink).nrows() {
                ($sink).column_mut(cix - 1)[rix] = ($source).clone();
            }
        }
    };
}

macro_rules! assign_2d_all_range_b {
    ($source:expr, $ix:expr, $sink:expr) => {
        for cix in 0..$ix.len() {
            for rix in 0..($sink).nrows() {
                if $ix[cix] == true {
                    ($sink).column_mut(cix)[rix] = ($source).clone();
                }
            }
        }
    };
}

macro_rules! assign_2d_all_range_v {
    ($source:expr, $ix:expr, $sink:expr) => {{
        let nsrc = $source.nrows();
        for (i, &cix) in $ix.iter().enumerate() {
            let col_index = cix - 1;
            let mut sink_col = $sink.column_mut(col_index);
            let src_col = $source.column(i % nsrc); // wrap around!
            for (dst, src) in sink_col.iter_mut().zip(src_col.iter()) {
                *dst = src.clone();
            }
        }
    }};
}

macro_rules! assign_2d_all_range_vb {
    ($source:expr, $ix:expr, $sink:expr) => {{
        let mut src_i = 0;
        for (i, cix) in (&$ix).iter().enumerate() {
            if *cix == true {
                let mut sink_col = ($sink).column_mut(i);
                let src_col = ($source).column(src_i);
                for (dst, src) in sink_col.iter_mut().zip(src_col.iter()) {
                    *dst = src.clone();
                }
                src_i += 1;
            }
        }
    }};
}

impl_all_fxn_v!(Set2DARV, assign_2d_all_range_v, usize);
impl_set_all_fxn_s!(Set2DARS, assign_2d_all_range, usize);
impl_set_all_fxn_s!(Set2DARB, assign_2d_all_range_b, bool);
impl_all_fxn_v!(Set2DARVB, assign_2d_all_range_vb, bool);

// x[1..3,:] = 1 ------------------------------------------------------------------

macro_rules! assign_2d_range_all {
    ($source:expr, $ix:expr, $sink:expr) => {
        for cix in 0..($sink).ncols() {
            for rix in $ix.iter() {
                ($sink).column_mut(cix)[rix - 1] = ($source).clone();
            }
        }
    };
}

macro_rules! assign_2d_range_all_b {
    ($source:expr, $ix:expr, $sink:expr) => {
        for cix in 0..($sink).ncols() {
            for rix in 0..$ix.len() {
                if $ix[rix] == true {
                    ($sink).column_mut(cix)[rix - 1] = ($source).clone();
                }
            }
        }
    };
}

macro_rules! assign_2d_range_all_v {
    ($source:expr, $ix:expr, $sink:expr) => {{
        let nsrc = $source.nrows();
        for (i, &rix) in $ix.iter().enumerate() {
            let row_index = rix - 1;
            let mut sink_row = $sink.row_mut(row_index);
            let src_row = $source.row(i % nsrc); // wrap around!
            for (dst, src) in sink_row.iter_mut().zip(src_row.iter()) {
                *dst = src.clone();
            }
        }
    }};
}

macro_rules! assign_2d_range_all_vb {
    ($source:expr, $ix:expr, $sink:expr) => {{
        for (i, rix) in ($ix).iter().enumerate() {
            if *rix {
                let mut sink_row = ($sink).row_mut(i);
                let src_row = ($source).row(i);
                for (dst, src) in sink_row.iter_mut().zip(src_row.iter()) {
                    *dst = src.clone();
                }
            }
        }
    }};
}

impl_all_fxn_v!(Set2DRAV, assign_2d_range_all_v, usize);
impl_set_all_fxn_s!(Set2DRAS, assign_2d_range_all, usize);
impl_set_all_fxn_s!(Set2DRAB, assign_2d_range_all_b, bool);
impl_all_fxn_v!(Set2DRAVB, assign_2d_range_all_vb, bool);

static PURE_MATRIX_AXIS_ZERO_ASSIGNMENT_CONTRACT: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| OperationContractDeclaration {
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
                InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                },
            ]
            .into_boxed_slice(),
        ),
        outputs: vec![OutputPortPolicy {
            access: AccessMode::ReadWrite,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::ReadModifyWrite {
                base_input: 0,
                regions: RegionPolicy::IndexedAxis { axis: 0 },
            },
            alias: AliasPolicy::MayAlias { input: 0 },
            change_detection: ChangeDetectionPolicy::KernelReported,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    });

static PURE_MATRIX_WHOLE_ASSIGNMENT_CONTRACT: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| OperationContractDeclaration {
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
                InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                },
            ]
            .into_boxed_slice(),
        ),
        outputs: vec![OutputPortPolicy {
            access: AccessMode::ReadWrite,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::ReadModifyWrite {
                base_input: 0,
                regions: RegionPolicy::WholeValue,
            },
            alias: AliasPolicy::MayAlias { input: 0 },
            change_detection: ChangeDetectionPolicy::KernelReported,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    });
