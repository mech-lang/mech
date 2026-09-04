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

fn assignment_source_out_of_bounds(required: usize, actual: usize) -> MechError {
    function_shape_contract_violation(
        "assign_slice",
        format!(
            "reactive assignment selector requires source offset {required}, but the source has {actual} elements"
        ),
    )
}

fn require_assignment_source_index(actual: usize, index: usize) -> MResult<()> {
    if index >= actual {
        return Err(assignment_source_out_of_bounds(index, actual));
    }
    Ok(())
}

fn require_assignment_selector_len(axis: &str, actual: usize, expected: usize) -> MResult<()> {
    if actual != expected {
        return Err(function_shape_contract_violation(
            "assign_slice",
            format!(
                "reactive assignment {axis} selector has length {actual}, but the sink requires {expected}"
            ),
        ));
    }
    Ok(())
}

fn require_assignment_index(axis: &str, index: usize, extent: usize) -> MResult<()> {
    if index == 0 || index > extent {
        return Err(function_shape_contract_violation(
            "assign_slice",
            format!(
                "reactive assignment {axis} index {index} is outside the one-based sink extent 1..={extent}"
            ),
        ));
    }
    Ok(())
}

fn require_assignment_indices(axis: &str, indices: &[usize], extent: usize) -> MResult<()> {
    for &index in indices {
        require_assignment_index(axis, index, extent)?;
    }
    Ok(())
}

fn require_assignment_source_layout(
    source_rows: usize,
    source_columns: usize,
    required_rows: usize,
    required_columns: usize,
    broadcast_rows: bool,
    broadcast_columns: bool,
) -> MResult<()> {
    let rows_valid = source_rows >= required_rows || (broadcast_rows && source_rows == 1);
    let columns_valid =
        source_columns >= required_columns || (broadcast_columns && source_columns == 1);
    if !rows_valid || !columns_valid {
        return Err(function_shape_contract_violation(
            "assign_slice",
            format!(
                "reactive assignment selector requires source layout {required_rows}x{required_columns}{}{}, but the source is {source_rows}x{source_columns}",
                if broadcast_rows {
                    " or one broadcast row"
                } else {
                    ""
                },
                if broadcast_columns {
                    " or one broadcast column"
                } else {
                    ""
                },
            ),
        ));
    }
    Ok(())
}

fn checked_assignment_source_offset(row: usize, column: usize, rows: usize) -> MResult<usize> {
    column
        .checked_mul(rows)
        .and_then(|offset| offset.checked_add(row))
        .ok_or_else(|| {
            function_shape_contract_violation(
                "assign_slice",
                "reactive assignment source offset overflowed usize",
            )
        })
}

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
            T: Scalar
                + Clone
                + Debug
                + Sync
                + Send
                + 'static
                + ConstElem
                + FunctionRuntimeType
                + FunctionPortBacking,
            #[cfg(feature = "semantic-compiler")]
            T: CompileConst + CanonicalMatrixElementBacking,
            IxVec: ConstElem + Debug + AsRef<[$ix]> + FunctionPortBacking,
            #[cfg(feature = "semantic-compiler")]
            IxVec: CompileConst,
            R1: Dim,
            C1: Dim,
            S1: StorageMut<T, R1, C1> + Clone + Debug,
            naMatrix<T, R1, C1, S1>: ConstElem + Debug + FunctionStateBacking,
            #[cfg(feature = "semantic-compiler")]
            naMatrix<T, R1, C1, S1>: CompileConst,
        {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
                <naMatrix<T, R1, C1, S1> as FunctionRuntimeType>::REPRESENTATION,
                T::REPRESENTATION,
                IxVec::REPRESENTATION,
            );

            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                mech_core::ImplementationMemoryClass::NoAdditionalScratch
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
            T: CompileConst + ConstElem + FunctionRuntimeType + CanonicalMatrixElementBacking,
            IxVec: CompileConst + ConstElem,
            naMatrix<T, R1, C1, S1>: CompileConst + ConstElem,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!(
                    "{}<{}{}{}>",
                    stringify!($struct_name),
                    <T as FunctionRuntimeType>::REPRESENTATION,
                    function_matrix_storage_name::<naMatrix<T, R1, C1, S1>>(),
                    function_matrix_storage_name::<IxVec>()
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
        require_assignment_index("linear", $ix, ($sink).len())?;
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
            if !($sink).is_empty() {
                require_assignment_source_index(($source).len(), ($sink).len() - 1)?;
            }
            for ix in 0..$sink.len() {
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
            T: Scalar
                + Clone
                + Debug
                + Sync
                + Send
                + 'static
                + ConstElem
                + FunctionRuntimeType
                + FunctionPortBacking,
            #[cfg(feature = "semantic-compiler")]
            T: CompileConst + CanonicalMatrixElementBacking,
            R: Dim,
            C: Dim,
            S: StorageMut<T, R, C> + Clone + Debug,
            naMatrix<T, R, C, S>: ConstElem + FunctionStateBacking,
            $ix: FunctionPortBacking,
            #[cfg(feature = "semantic-compiler")]
            naMatrix<T, R, C, S>: CompileConst,
        {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
                <naMatrix<T, R, C, S> as FunctionRuntimeType>::REPRESENTATION,
                T::REPRESENTATION,
                <$ix as FunctionRuntimeType>::REPRESENTATION,
            );

            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                mech_core::ImplementationMemoryClass::NoAdditionalScratch
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
            T: CompileConst + ConstElem + FunctionRuntimeType + CanonicalMatrixElementBacking,
            naMatrix<T, R, C, S>: CompileConst + ConstElem,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!(
                    "{}<{}{}>",
                    stringify!($struct_name),
                    <T as FunctionRuntimeType>::REPRESENTATION,
                    function_matrix_storage_name::<naMatrix<T, R, C, S>>()
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
        require_assignment_indices("linear", ($ix).as_ref(), ($sink).len())?;
        for i in 0..($ix).len() {
            ($sink)[($ix)[i] - 1] = ($source).clone();
        }
    };
}

macro_rules! set_1d_range_b {
    ($source:expr, $ix:expr, $sink:expr) => {
        require_assignment_selector_len("linear", ($ix).len(), ($sink).len())?;
        for i in 0..($ix).len() {
            if $ix[i] == true {
                ($sink)[i] = ($source).clone();
            }
        }
    };
}

macro_rules! set_1d_range_vec {
    ($source:expr, $ix:expr, $sink:expr) => {
        require_assignment_indices("linear", ($ix).as_ref(), ($sink).len())?;
        if !($ix).is_empty() {
            require_assignment_source_index(($source).len(), ($ix).len() - 1)?;
        }
        for i in 0..($ix).len() {
            ($sink)[($ix)[i] - 1] = ($source)[i].clone();
        }
    };
}

macro_rules! set_1d_range_vec_b {
    ($source:expr, $ix:expr, $sink:expr) => {
        require_assignment_selector_len("linear", ($ix).len(), ($sink).len())?;
        for (i, selected) in ($ix).iter().enumerate() {
            if *selected {
                require_assignment_source_index(($source).len(), i)?;
            }
        }
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
    T: Debug
        + Clone
        + Sync
        + Send
        + PartialEq
        + 'static
        + ConstElem
        + FunctionRuntimeType
        + FunctionPortBacking,
    #[cfg(feature = "semantic-compiler")]
    T: CompileConst + CanonicalMatrixElementBacking,
    R: Dim,
    C: Dim,
    S: StorageMut<T, R, C> + Debug + IsContiguous + 'static,
    naMatrix<T, R, C, S>: ConstElem + Debug + FunctionStateBacking,
    #[cfg(feature = "semantic-compiler")]
    naMatrix<T, R, C, S>: CompileConst,
{
    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::unary(
        <naMatrix<T, R, C, S> as FunctionRuntimeType>::REPRESENTATION,
        T::REPRESENTATION,
    );

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
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
    T: CompileConst + ConstElem + FunctionRuntimeType + CanonicalMatrixElementBacking,
    naMatrix<T, R, C, S>: CompileConst + ConstElem,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!(
            "Set1DAS<{}{}>",
            <T as FunctionRuntimeType>::REPRESENTATION,
            function_matrix_storage_name::<naMatrix<T, R, C, S>>()
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
    naMatrix<T, R1, C1, S1>: FunctionStateBacking,
    T: Scalar
        + Clone
        + Debug
        + Sync
        + Send
        + 'static
        + ConstElem
        + FunctionRuntimeType
        + FunctionPortBacking,
    #[cfg(feature = "semantic-compiler")]
    T: CompileConst + CanonicalMatrixElementBacking,
    R1: Dim,
    C1: Dim,
    S1: StorageMut<T, R1, C1> + Clone + Debug,
    naMatrix<T, R1, C1, S1>: ConstElem + FunctionStateBacking,
    #[cfg(feature = "semantic-compiler")]
    naMatrix<T, R1, C1, S1>: CompileConst,
{
    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::ternary(
        <naMatrix<T, R1, C1, S1> as FunctionRuntimeType>::REPRESENTATION,
        T::REPRESENTATION,
        <usize as FunctionRuntimeType>::REPRESENTATION,
        <usize as FunctionRuntimeType>::REPRESENTATION,
    );

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
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

    fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_MATRIX_ELEMENT_ASSIGNMENT_CONTRACT)
    }
}
impl<T, R1, C1, S1> MechFunctionImpl for Assign2DSSS<T, naMatrix<T, R1, C1, S1>>
where
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
            require_assignment_index("row", r, sink_ptr.nrows())?;
            require_assignment_index("column", c, sink_ptr.ncols())?;
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
    T: CompileConst + ConstElem + FunctionRuntimeType + CanonicalMatrixElementBacking,
    naMatrix<T, R1, C1, S1>: CompileConst + ConstElem,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!(
            "Assign2DSSS<{}{}>",
            <T as FunctionRuntimeType>::REPRESENTATION,
            function_matrix_storage_name::<naMatrix<T, R1, C1, S1>>()
        );
        compile_ternop!(name, self.sink, self.source, self.ixes.0, self.ixes.1, ctx);
    }
}

macro_rules! assign_2d_all_scalar {
    ($source:expr, $ix:expr, $sink:expr) => {
        require_assignment_index("column", $ix, ($sink).ncols())?;
        for i in 0..$sink.nrows() {
            ($sink).column_mut($ix - 1)[i] = ($source).clone();
        }
    };
}

macro_rules! assign_2d_all_vector {
    ($source:expr, $ix:expr, $sink:expr) => {
        require_assignment_index("column", *$ix, ($sink).ncols())?;
        if ($sink).nrows() != 0 {
            require_assignment_source_index(($source).len(), ($sink).nrows() - 1)?;
        }
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
            T: Debug
                + Clone
                + Sync
                + Send
                + 'static
                + PartialEq
                + PartialOrd
                + ConstElem
                + FunctionRuntimeType,
            #[cfg(feature = "semantic-compiler")]
            T: CompileConst + CanonicalMatrixElementBacking,
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
            $ix: FunctionPortBacking,
            #[cfg(feature = "semantic-compiler")]
            naMatrix<T, R2, C2, S2>: CompileConst,
        {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
                <naMatrix<T, R1, C1, S1> as FunctionRuntimeType>::REPRESENTATION,
                <naMatrix<T, R2, C2, S2> as FunctionRuntimeType>::REPRESENTATION,
                <$ix as FunctionRuntimeType>::REPRESENTATION,
            );

            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                mech_core::ImplementationMemoryClass::NoAdditionalScratch
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
            T: CompileConst + ConstElem + FunctionRuntimeType + CanonicalMatrixElementBacking,
            naMatrix<T, R1, C1, S1>: CompileConst + ConstElem,
            naMatrix<T, R2, C2, S2>: CompileConst + ConstElem,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!(
                    "{}<{}{}{}>",
                    stringify!($struct_name),
                    <T as FunctionRuntimeType>::REPRESENTATION,
                    function_matrix_storage_name::<naMatrix<T, R1, C1, S1>>(),
                    function_matrix_storage_name::<naMatrix<T, R2, C2, S2>>()
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
        require_assignment_index("row", $ix, ($sink).nrows())?;
        for i in 0..$sink.ncols() {
            ($sink).row_mut($ix - 1)[i] = ($source).clone();
        }
    };
}

macro_rules! assign_2d_scalar_all_vector {
    ($source:expr, $ix:expr, $sink:expr) => {
        require_assignment_index("row", *$ix, ($sink).nrows())?;
        if ($sink).ncols() != 0 {
            require_assignment_source_index(($source).len(), ($sink).ncols() - 1)?;
        }
        for i in 0..$sink.ncols() {
            ($sink).row_mut($ix - 1)[i] = ($source)[i].clone();
        }
    };
}

impl_assign_fxn_s!(Assign2DSAS, assign_2d_scalar_all_scalar, usize);
impl_assign_scalar_fxn_v!(Assign2DSAV, assign_2d_scalar_all_vector, usize);

macro_rules! assign_2d_range_scalar {
    ($sink:expr, $ix1:expr, $ix2:expr, $source:expr) => {
        require_assignment_indices("row", ($ix1).as_ref(), ($sink).nrows())?;
        require_assignment_index("column", $ix2, ($sink).ncols())?;
        let mut col = ($sink).column_mut($ix2 - 1);
        for &rix in ($ix1).iter() {
            col[rix - 1] = ($source).clone();
        }
    };
}

macro_rules! assign_2d_range_scalar_v {
    ($sink:expr, $ix1:expr, $ix2:expr, $source:expr) => {
        require_assignment_indices("row", ($ix1).as_ref(), ($sink).nrows())?;
        require_assignment_index("column", $ix2, ($sink).ncols())?;
        if !($ix1).is_empty() {
            require_assignment_source_index(($source).len(), ($ix1).len() - 1)?;
        }
        let mut col = ($sink).column_mut($ix2 - 1);
        for (i, &rix) in ($ix1).iter().enumerate() {
            col[rix - 1] = ($source)[i].clone();
        }
    };
}

macro_rules! assign_2d_range_scalar_b {
    ($sink:expr, $ix1:expr, $ix2:expr, $source:expr) => {
        require_assignment_selector_len("row", ($ix1).len(), ($sink).nrows())?;
        require_assignment_index("column", $ix2, ($sink).ncols())?;
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
        require_assignment_selector_len("row", ($ix1).len(), ($sink).nrows())?;
        require_assignment_index("column", $ix2, ($sink).ncols())?;
        for (rix, is_selected) in ($ix1).iter().enumerate() {
            if *is_selected {
                require_assignment_source_index(($source).len(), rix)?;
            }
        }
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
            T: Scalar
                + Clone
                + Debug
                + Sync
                + Send
                + 'static
                + ConstElem
                + FunctionRuntimeType
                + FunctionPortBacking,
            #[cfg(feature = "semantic-compiler")]
            T: CompileConst + CanonicalMatrixElementBacking,
            IxVec: ConstElem + Debug + AsRef<[$ix]> + FunctionPortBacking,
            #[cfg(feature = "semantic-compiler")]
            IxVec: CompileConst,
            R: Dim,
            C: Dim,
            S: StorageMut<T, R, C> + Clone + Debug,
            naMatrix<T, R, C, S>: ConstElem + Debug + FunctionStateBacking,
            #[cfg(feature = "semantic-compiler")]
            naMatrix<T, R, C, S>: CompileConst,
        {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::ternary(
                <naMatrix<T, R, C, S> as FunctionRuntimeType>::REPRESENTATION,
                T::REPRESENTATION,
                IxVec::REPRESENTATION,
                <usize as FunctionRuntimeType>::REPRESENTATION,
            );

            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                mech_core::ImplementationMemoryClass::NoAdditionalScratch
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
            T: CompileConst + ConstElem + FunctionRuntimeType + CanonicalMatrixElementBacking,
            IxVec: CompileConst + ConstElem,
            naMatrix<T, R, C, S>: CompileConst + ConstElem,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!(
                    "{}<{}{}{}>",
                    stringify!($struct_name),
                    <T as FunctionRuntimeType>::REPRESENTATION,
                    function_matrix_storage_name::<naMatrix<T, R, C, S>>(),
                    function_matrix_storage_name::<IxVec>()
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
            T: Debug
                + Clone
                + Sync
                + Send
                + 'static
                + PartialEq
                + PartialOrd
                + ConstElem
                + FunctionRuntimeType,
            #[cfg(feature = "semantic-compiler")]
            T: CompileConst + CanonicalMatrixElementBacking,
            IxVec: ConstElem + Debug + AsRef<[$ix]> + FunctionPortBacking,
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
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::ternary(
                <naMatrix<T, R1, C1, S1> as FunctionRuntimeType>::REPRESENTATION,
                <naMatrix<T, R2, C2, S2> as FunctionRuntimeType>::REPRESENTATION,
                IxVec::REPRESENTATION,
                <usize as FunctionRuntimeType>::REPRESENTATION,
            );

            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                mech_core::ImplementationMemoryClass::NoAdditionalScratch
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
        require_assignment_index("row", $ix1, ($sink).nrows())?;
        require_assignment_indices("column", ($ix2).as_ref(), ($sink).ncols())?;
        for i in 0..($ix2).len() {
            let cix = $ix2[i] - 1;
            ($sink).row_mut($ix1 - 1)[cix] = ($source).clone();
        }
    };
}

macro_rules! assign_2d_scalar_range_v {
    ($sink:expr, $ix1:expr, $ix2:expr, $source:expr) => {
        require_assignment_index("row", $ix1, ($sink).nrows())?;
        require_assignment_indices("column", ($ix2).as_ref(), ($sink).ncols())?;
        if !($ix2).is_empty() {
            require_assignment_source_index(($source).len(), ($ix2).len() - 1)?;
        }
        for i in 0..($ix2).len() {
            let cix = $ix2[i] - 1;
            ($sink).row_mut($ix1 - 1)[cix] = ($source)[i].clone();
        }
    };
}

macro_rules! assign_2d_scalar_range_b {
    ($sink:expr, $ix1:expr, $ix2:expr, $source:expr) => {
        require_assignment_selector_len("column", ($ix2).len(), ($sink).ncols())?;
        require_assignment_index("row", $ix1, ($sink).nrows())?;
        for cix in 0..($ix2).len() {
            if $ix2[cix] == true {
                ($sink).row_mut($ix1 - 1)[cix] = ($source).clone();
            }
        }
    };
}

macro_rules! assign_2d_scalar_range_vb {
    ($sink:expr, $ix1:expr, $ix2:expr, $source:expr) => {
        require_assignment_selector_len("column", ($ix2).len(), ($sink).ncols())?;
        require_assignment_index("row", $ix1, ($sink).nrows())?;
        for (cix, selected) in ($ix2).iter().enumerate() {
            if *selected {
                require_assignment_source_index(($source).len(), cix)?;
            }
        }
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
            T: Scalar
                + Clone
                + Debug
                + Sync
                + Send
                + 'static
                + ConstElem
                + FunctionRuntimeType
                + FunctionPortBacking,
            #[cfg(feature = "semantic-compiler")]
            T: CompileConst + CanonicalMatrixElementBacking,
            IxVec: ConstElem + Debug + AsRef<[$ix]> + FunctionPortBacking,
            #[cfg(feature = "semantic-compiler")]
            IxVec: CompileConst,
            R: Dim,
            C: Dim,
            S: StorageMut<T, R, C> + Clone + Debug,
            naMatrix<T, R, C, S>: ConstElem + Debug + FunctionStateBacking,
            #[cfg(feature = "semantic-compiler")]
            naMatrix<T, R, C, S>: CompileConst,
        {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::ternary(
                <naMatrix<T, R, C, S> as FunctionRuntimeType>::REPRESENTATION,
                T::REPRESENTATION,
                <usize as FunctionRuntimeType>::REPRESENTATION,
                IxVec::REPRESENTATION,
            );

            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                mech_core::ImplementationMemoryClass::NoAdditionalScratch
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
            T: CompileConst + ConstElem + FunctionRuntimeType + CanonicalMatrixElementBacking,
            IxVec: CompileConst + ConstElem,
            naMatrix<T, R, C, S>: CompileConst + ConstElem,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!(
                    "{}<{}{}{}>",
                    stringify!($struct_name),
                    <T as FunctionRuntimeType>::REPRESENTATION,
                    function_matrix_storage_name::<naMatrix<T, R, C, S>>(),
                    function_matrix_storage_name::<IxVec>()
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
            T: Debug
                + Clone
                + Sync
                + Send
                + 'static
                + PartialEq
                + PartialOrd
                + ConstElem
                + FunctionRuntimeType,
            #[cfg(feature = "semantic-compiler")]
            T: CompileConst + CanonicalMatrixElementBacking,
            IxVec: ConstElem + Debug + AsRef<[$ix]> + FunctionPortBacking,
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
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::ternary(
                <naMatrix<T, R1, C1, S1> as FunctionRuntimeType>::REPRESENTATION,
                <naMatrix<T, R2, C2, S2> as FunctionRuntimeType>::REPRESENTATION,
                <usize as FunctionRuntimeType>::REPRESENTATION,
                IxVec::REPRESENTATION,
            );

            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                mech_core::ImplementationMemoryClass::NoAdditionalScratch
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
        require_assignment_indices("row", ($ix1).as_ref(), ($sink).nrows())?;
        require_assignment_indices("column", ($ix2).as_ref(), ($sink).ncols())?;
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
        require_assignment_indices("row", ($ix1).as_ref(), ($sink).nrows())?;
        require_assignment_indices("column", ($ix2).as_ref(), ($sink).ncols())?;
        let required_source_len = ($ix1).len().checked_mul(($ix2).len()).ok_or_else(|| {
            function_shape_contract_violation(
                "assign_slice",
                "reactive assignment source length overflowed usize",
            )
        })?;
        if required_source_len != 0 {
            require_assignment_source_index(($source).len(), required_source_len - 1)?;
        }
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
        require_assignment_selector_len("row", ($ix1).len(), ($sink).nrows())?;
        require_assignment_selector_len("column", ($ix2).len(), ($sink).ncols())?;
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
        require_assignment_selector_len("row", ($ix1).len(), ($sink).nrows())?;
        require_assignment_selector_len("column", ($ix2).len(), ($sink).ncols())?;
        for r in 0..($ix1).len() {
            if $ix1[r] {
                for c in 0..($ix2).len() {
                    if $ix2[c] {
                        let offset = r
                            .checked_mul(($ix2).len())
                            .and_then(|offset| offset.checked_add(c))
                            .ok_or_else(|| {
                                function_shape_contract_violation(
                                    "assign_slice",
                                    "reactive assignment source offset overflowed usize",
                                )
                            })?;
                        require_assignment_source_index(($source).len(), offset)?;
                    }
                }
            }
        }
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
        require_assignment_selector_len("row", ($ix1).len(), ($sink).nrows())?;
        require_assignment_indices("column", ($ix2).as_ref(), ($sink).ncols())?;
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
        require_assignment_selector_len("row", ($ix1).len(), ($sink).nrows())?;
        require_assignment_indices("column", ($ix2).as_ref(), ($sink).ncols())?;
        let nrows = $sink.nrows();
        for cix in 0..($ix2).len() {
            let c = $ix2[cix] - 1;
            for r in 0..($ix1).len() {
                if $ix1[r] {
                    let offset = checked_assignment_source_offset(r, c, nrows)?;
                    require_assignment_source_index(($source).len(), offset)?;
                }
            }
        }
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
        require_assignment_selector_len("column", ($ix2).len(), ($sink).ncols())?;
        require_assignment_indices("row", ($ix1).as_ref(), ($sink).nrows())?;
        for &row in ($ix1).iter() {
            let r = row - 1;
            for c in 0..$ix2.len() {
                if $ix2[c] {
                    ($sink)[(r, c)] = $source.clone();
                }
            }
        }
    };
}

macro_rules! assign_2d_range_range_vub {
    ($sink:expr, $ix1:expr, $ix2:expr, $source:expr) => {
        require_assignment_selector_len("column", ($ix2).len(), ($sink).ncols())?;
        require_assignment_indices("row", ($ix1).as_ref(), ($sink).nrows())?;
        let nrows = $sink.nrows();
        for c in 0..$ix2.len() {
            if $ix2[c] {
                for rix in 0..$ix1.len() {
                    let r = $ix1[rix] - 1;
                    let offset = checked_assignment_source_offset(r, c, nrows)?;
                    require_assignment_source_index(($source).len(), offset)?;
                }
            }
        }
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
            T: Scalar
                + Clone
                + Debug
                + Sync
                + Send
                + 'static
                + ConstElem
                + FunctionRuntimeType
                + FunctionPortBacking,
            #[cfg(feature = "semantic-compiler")]
            T: CompileConst + CanonicalMatrixElementBacking,
            IxVec1: ConstElem + Debug + AsRef<[$ix1]> + FunctionPortBacking,
            #[cfg(feature = "semantic-compiler")]
            IxVec1: CompileConst,
            IxVec2: ConstElem + Debug + AsRef<[$ix2]> + FunctionPortBacking,
            #[cfg(feature = "semantic-compiler")]
            IxVec2: CompileConst,
            R: Dim,
            C: Dim,
            S: StorageMut<T, R, C> + Clone + Debug,
            naMatrix<T, R, C, S>: ConstElem + Debug + FunctionStateBacking,
            #[cfg(feature = "semantic-compiler")]
            naMatrix<T, R, C, S>: CompileConst,
        {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::ternary(
                <naMatrix<T, R, C, S> as FunctionRuntimeType>::REPRESENTATION,
                T::REPRESENTATION,
                IxVec1::REPRESENTATION,
                IxVec2::REPRESENTATION,
            );

            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                mech_core::ImplementationMemoryClass::NoAdditionalScratch
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
            T: CompileConst + ConstElem + FunctionRuntimeType + CanonicalMatrixElementBacking,
            IxVec1: CompileConst + ConstElem,
            IxVec2: CompileConst + ConstElem,
            naMatrix<T, R, C, S>: CompileConst + ConstElem,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!(
                    "{}<{}{}{}{}>",
                    stringify!($struct_name),
                    <T as FunctionRuntimeType>::REPRESENTATION,
                    function_matrix_storage_name::<naMatrix<T, R, C, S>>(),
                    function_matrix_storage_name::<IxVec1>(),
                    function_matrix_storage_name::<IxVec2>()
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
        require_assignment_indices("column", ($ix).as_ref(), ($sink).ncols())?;
        for cix in $ix.iter() {
            for rix in 0..($sink).nrows() {
                ($sink).column_mut(cix - 1)[rix] = ($source).clone();
            }
        }
    };
}

macro_rules! assign_2d_all_range_b {
    ($source:expr, $ix:expr, $sink:expr) => {
        require_assignment_selector_len("column", ($ix).len(), ($sink).ncols())?;
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
        require_assignment_indices("column", ($ix).as_ref(), ($sink).ncols())?;
        let nsrc = $source.ncols();
        require_assignment_source_layout(
            ($source).nrows(),
            nsrc,
            ($sink).nrows(),
            ($ix).len(),
            false,
            true,
        )?;
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
        require_assignment_selector_len("column", ($ix).len(), ($sink).ncols())?;
        let nsrc = $source.ncols();
        let selected_columns = ($ix).iter().filter(|selected| **selected).count();
        require_assignment_source_layout(
            ($source).nrows(),
            nsrc,
            ($sink).nrows(),
            selected_columns,
            false,
            true,
        )?;
        let mut src_i = 0;
        for (i, cix) in (&$ix).iter().enumerate() {
            if *cix == true {
                let mut sink_col = ($sink).column_mut(i);
                let src_col = ($source).column(src_i % nsrc);
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
        require_assignment_indices("row", ($ix).as_ref(), ($sink).nrows())?;
        for cix in 0..($sink).ncols() {
            for rix in $ix.iter() {
                ($sink).column_mut(cix)[rix - 1] = ($source).clone();
            }
        }
    };
}

macro_rules! assign_2d_range_all_b {
    ($source:expr, $ix:expr, $sink:expr) => {
        require_assignment_selector_len("row", ($ix).len(), ($sink).nrows())?;
        for cix in 0..($sink).ncols() {
            for rix in 0..$ix.len() {
                if $ix[rix] == true {
                    ($sink).column_mut(cix)[rix] = ($source).clone();
                }
            }
        }
    };
}

macro_rules! assign_2d_range_all_v {
    ($source:expr, $ix:expr, $sink:expr) => {{
        require_assignment_indices("row", ($ix).as_ref(), ($sink).nrows())?;
        let nsrc = $source.nrows();
        require_assignment_source_layout(
            nsrc,
            ($source).ncols(),
            ($ix).len(),
            ($sink).ncols(),
            true,
            false,
        )?;
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
        require_assignment_selector_len("row", ($ix).len(), ($sink).nrows())?;
        for (i, selected) in ($ix).iter().enumerate() {
            if *selected {
                require_assignment_source_layout(
                    ($source).nrows(),
                    ($source).ncols(),
                    i + 1,
                    ($sink).ncols(),
                    false,
                    false,
                )?;
            }
        }
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

#[cfg(all(
    test,
    feature = "matrixd",
    feature = "vectord",
    feature = "logical_indexing",
    feature = "u8"
))]
mod tests {
    use super::*;
    use mech_core::{FunctionInvocation, Ref};
    use nalgebra::{DMatrix, DVector};

    #[test]
    fn column_assignment_routes_each_rectangular_source_column() {
        let source = Ref::new(DMatrix::from_row_slice(2, 3, &[1_u8, 2, 3, 4, 5, 6]));
        let columns = Ref::new(DVector::from_vec(vec![1_usize, 2, 3]));
        let sink = Ref::new(DMatrix::<u8>::zeros(2, 3));
        let function = Set2DARV::<u8, DMatrix<u8>, DMatrix<u8>, DVector<usize>>::new_invocation(
            FunctionInvocation::binary(
                ValueCell::from_exact_matrix_ref(sink.clone(), 2, 3).unwrap(),
                ValueCell::from_exact_matrix_ref(source, 2, 3).unwrap(),
                ValueCell::from_exact_matrix_ref(columns, 3, 1).unwrap(),
            ),
        )
        .unwrap();

        function.solve_result().unwrap();
        assert_eq!(
            *sink.borrow(),
            DMatrix::from_row_slice(2, 3, &[1, 2, 3, 4, 5, 6])
        );
    }

    #[test]
    fn reactive_sparse_mask_is_revalidated_before_any_write() {
        let source = Ref::new(DVector::from_vec(vec![9_u8]));
        let mask = Ref::new(DVector::from_vec(vec![true, false, false]));
        let sink = Ref::new(DVector::<u8>::zeros(3));
        let function = Assign1DRVB::<u8, DVector<u8>, DVector<u8>, DVector<bool>>::new_invocation(
            FunctionInvocation::binary(
                ValueCell::from_exact_matrix_ref(sink.clone(), 3, 1).unwrap(),
                ValueCell::from_exact_matrix_ref(source, 1, 1).unwrap(),
                ValueCell::from_exact_matrix_ref(mask.clone(), 3, 1).unwrap(),
            ),
        )
        .unwrap();

        function.solve_result().unwrap();
        assert_eq!(sink.borrow().as_slice(), &[9, 0, 0]);

        *mask.borrow_mut() = DVector::from_vec(vec![false, true, false]);
        let error = function.solve_result().unwrap_err();
        assert!(error.kind_message().contains("source offset 1"));
        assert_eq!(sink.borrow().as_slice(), &[9, 0, 0]);
    }

    #[test]
    fn reactive_mask_extents_are_revalidated_before_any_matrix_write() {
        let source = Ref::new(DMatrix::from_element(2, 4, 7_u8));
        let columns = Ref::new(DVector::from_vec(vec![true, false, false]));
        let sink = Ref::new(DMatrix::<u8>::zeros(2, 3));
        let function = Set2DARVB::<u8, DMatrix<u8>, DMatrix<u8>, DVector<bool>>::new_invocation(
            FunctionInvocation::binary(
                ValueCell::from_exact_matrix_ref(sink.clone(), 2, 3).unwrap(),
                ValueCell::from_exact_matrix_ref(source, 2, 4).unwrap(),
                ValueCell::from_exact_matrix_ref(columns.clone(), 3, 1).unwrap(),
            ),
        )
        .unwrap();

        function.solve_result().unwrap();
        let before = sink.borrow().clone();
        *columns.borrow_mut() = DVector::from_vec(vec![true, false, false, true]);
        let error = function.solve_result().unwrap_err();
        assert!(
            error
                .kind_message()
                .contains("column selector has length 4")
        );
        assert_eq!(*sink.borrow(), before);

        let source = Ref::new(DMatrix::from_element(3, 3, 8_u8));
        let rows = Ref::new(DVector::from_vec(vec![true, false]));
        let sink = Ref::new(DMatrix::<u8>::zeros(2, 3));
        let function = Set2DRAVB::<u8, DMatrix<u8>, DMatrix<u8>, DVector<bool>>::new_invocation(
            FunctionInvocation::binary(
                ValueCell::from_exact_matrix_ref(sink.clone(), 2, 3).unwrap(),
                ValueCell::from_exact_matrix_ref(source, 3, 3).unwrap(),
                ValueCell::from_exact_matrix_ref(rows.clone(), 2, 1).unwrap(),
            ),
        )
        .unwrap();

        function.solve_result().unwrap();
        let before = sink.borrow().clone();
        *rows.borrow_mut() = DVector::from_vec(vec![true, false, true]);
        let error = function.solve_result().unwrap_err();
        assert!(error.kind_message().contains("row selector has length 3"));
        assert_eq!(*sink.borrow(), before);
    }

    #[test]
    fn reactive_numeric_indices_are_revalidated_before_any_matrix_write() {
        let source_cell = ValueCell::from_exact(9_u8).unwrap();
        let row_cell = ValueCell::from_exact(1_usize).unwrap();
        let column_cell = ValueCell::from_exact(1_usize).unwrap();
        let sink = Ref::new(DMatrix::<u8>::zeros(2, 3));
        let function = Assign2DSSS::<u8, DMatrix<u8>>::new_invocation(FunctionInvocation::ternary(
            ValueCell::from_exact_matrix_ref(sink.clone(), 2, 3).unwrap(),
            source_cell,
            row_cell.clone(),
            column_cell.clone(),
        ))
        .unwrap();

        function.solve_result().unwrap();
        let before = sink.borrow().clone();
        row_cell
            .replace(&ValueCell::from_exact(3_usize).unwrap().snapshot().unwrap())
            .unwrap();
        assert!(function.solve_result().is_err());
        assert_eq!(*sink.borrow(), before);
        row_cell
            .replace(&ValueCell::from_exact(1_usize).unwrap().snapshot().unwrap())
            .unwrap();
        column_cell
            .replace(&ValueCell::from_exact(4_usize).unwrap().snapshot().unwrap())
            .unwrap();
        assert!(function.solve_result().is_err());
        assert_eq!(*sink.borrow(), before);
    }
}
