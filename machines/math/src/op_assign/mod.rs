use crate::*;

paste! {
    const _: Option<&dyn Display> = None;
}

use std::sync::LazyLock;

#[cfg(feature = "matrix")]
use nalgebra::{
    Dim,
    base::{Matrix as naMatrix, Storage, StorageMut},
};

#[cfg(all(feature = "add_assign", any(feature = "matrix", feature = "source")))]
pub mod add_assign;
#[cfg(all(feature = "div_assign", any(feature = "matrix", feature = "source")))]
pub mod div_assign;
#[cfg(all(feature = "mul_assign", any(feature = "matrix", feature = "source")))]
pub mod mul_assign;
#[cfg(all(feature = "sub_assign", any(feature = "matrix", feature = "source")))]
pub mod sub_assign;

#[cfg(feature = "add_assign")]
pub use self::add_assign::*;
#[cfg(feature = "div_assign")]
pub use self::div_assign::*;
#[cfg(feature = "mul_assign")]
pub use self::mul_assign::*;
#[cfg(feature = "sub_assign")]
pub use self::sub_assign::*;

pub trait RuntimeCheckedOpAssign: Copy {
    fn runtime_checked_add(self, rhs: Self) -> Option<Self>;
    fn runtime_checked_sub(self, rhs: Self) -> Option<Self>;
    fn runtime_checked_mul(self, rhs: Self) -> Option<Self>;
    fn runtime_checked_div(self, rhs: Self) -> Option<Self>;
}

macro_rules! impl_checked_integer_op_assign {
    ($($type:ty),+ $(,)?) => {
        $(
            impl RuntimeCheckedOpAssign for $type {
                fn runtime_checked_add(self, rhs: Self) -> Option<Self> { self.checked_add(rhs) }
                fn runtime_checked_sub(self, rhs: Self) -> Option<Self> { self.checked_sub(rhs) }
                fn runtime_checked_mul(self, rhs: Self) -> Option<Self> { self.checked_mul(rhs) }
                fn runtime_checked_div(self, rhs: Self) -> Option<Self> { self.checked_div(rhs) }
            }
        )+
    };
}

impl_checked_integer_op_assign!(i8, i16, i32, i64, i128, u8, u16, u32, u64, u128);

macro_rules! impl_ieee_op_assign {
    ($($type:ty),+ $(,)?) => {
        $(
            impl RuntimeCheckedOpAssign for $type {
                fn runtime_checked_add(self, rhs: Self) -> Option<Self> { Some(self + rhs) }
                fn runtime_checked_sub(self, rhs: Self) -> Option<Self> { Some(self - rhs) }
                fn runtime_checked_mul(self, rhs: Self) -> Option<Self> { Some(self * rhs) }
                fn runtime_checked_div(self, rhs: Self) -> Option<Self> { Some(self / rhs) }
            }
        )+
    };
}

impl_ieee_op_assign!(f32, f64);
#[cfg(feature = "complex")]
impl_ieee_op_assign!(C64);

#[cfg(feature = "rational")]
impl RuntimeCheckedOpAssign for R64 {
    fn runtime_checked_add(self, rhs: Self) -> Option<Self> {
        self.checked_add(rhs)
    }
    fn runtime_checked_sub(self, rhs: Self) -> Option<Self> {
        self.checked_sub(rhs)
    }
    fn runtime_checked_mul(self, rhs: Self) -> Option<Self> {
        self.checked_mul(rhs)
    }
    fn runtime_checked_div(self, rhs: Self) -> Option<Self> {
        self.checked_div(rhs)
    }
}

macro_rules! checked_op_assign {
    ($name:ident, $method:ident, $operation:literal) => {
        fn $name<T: RuntimeCheckedOpAssign>(lhs: T, rhs: T) -> MResult<T> {
            lhs.$method(rhs)
                .ok_or_else(|| arithmetic_overflow::<T>($operation))
        }
    };
}

#[cfg(feature = "add_assign")]
checked_op_assign!(
    checked_add_assign,
    runtime_checked_add,
    "addition assignment"
);
#[cfg(feature = "sub_assign")]
checked_op_assign!(
    checked_sub_assign,
    runtime_checked_sub,
    "subtraction assignment"
);
#[cfg(feature = "mul_assign")]
checked_op_assign!(
    checked_mul_assign,
    runtime_checked_mul,
    "multiplication assignment"
);
#[cfg(feature = "div_assign")]
checked_op_assign!(
    checked_div_assign,
    runtime_checked_div,
    "division assignment"
);

pub(crate) static PURE_WHOLE_VALUE_RMW_CONTRACT: LazyLock<OperationContractDeclaration> =
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

#[cfg(feature = "matrix")]
static PURE_INDEXED_AXIS_ZERO_RMW_CONTRACT: LazyLock<OperationContractDeclaration> =
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

#[cfg(feature = "matrix")]
fn checked_one_based_index(index: usize, len: usize) -> MResult<usize> {
    if index == 0 || index > len {
        return Err(function_shape_contract_violation(
            "op_assign_slice",
            format!("index {index} is outside the valid 1..={len} range"),
        ));
    }
    Ok(index - 1)
}

#[cfg(feature = "matrix")]
fn validate_mask_len(mask_len: usize, sink_len: usize) -> MResult<()> {
    if mask_len > sink_len {
        return Err(function_shape_contract_violation(
            "op_assign_slice",
            format!("boolean index has {mask_len} elements, output has {sink_len}"),
        ));
    }
    Ok(())
}

#[cfg(feature = "matrix")]
fn validate_source_len(source_len: usize, selected_len: usize) -> MResult<()> {
    if source_len < selected_len {
        return Err(function_shape_contract_violation(
            "op_assign_slice",
            format!("source has {source_len} elements, selection requires {selected_len}"),
        ));
    }
    Ok(())
}

#[cfg(feature = "matrix")]
fn apply_index_scalar_positions<T: ManagedElement>(
    source: ManagedValueView<'_, T>,
    indexes: ManagedValueView<'_, usize>,
    sink: &mut ManagedValueViewMut<'_, T>,
    operation: impl Fn(T, T) -> MResult<T>,
) -> MResult<()> {
    let source = source.get_column_major(0).ok_or_else(|| {
        function_shape_contract_violation("op_assign_slice", "scalar source is empty")
    })?;
    for position in 0..indexes.len() {
        let index = indexes
            .get_column_major(position)
            .expect("validated selector lane");
        let destination = checked_one_based_index(index, sink.len())?;
        let current = sink
            .get_column_major(destination)
            .expect("initialized destination lane");
        sink.try_set_column_major(destination, operation(current, source)?)?;
    }
    Ok(())
}

#[cfg(feature = "matrix")]
fn apply_index_scalar_mask<T: ManagedElement>(
    source: ManagedValueView<'_, T>,
    mask: ManagedValueView<'_, bool>,
    sink: &mut ManagedValueViewMut<'_, T>,
    operation: impl Fn(T, T) -> MResult<T>,
) -> MResult<()> {
    validate_mask_len(mask.len(), sink.len())?;
    let source = source.get_column_major(0).ok_or_else(|| {
        function_shape_contract_violation("op_assign_slice", "scalar source is empty")
    })?;
    for position in 0..mask.len() {
        if mask
            .get_column_major(position)
            .expect("validated mask lane")
        {
            let current = sink
                .get_column_major(position)
                .expect("initialized destination lane");
            sink.try_set_column_major(position, operation(current, source)?)?;
        }
    }
    Ok(())
}

#[cfg(feature = "matrix")]
fn apply_index_vector_positions<T: ManagedElement>(
    source: ManagedValueView<'_, T>,
    indexes: ManagedValueView<'_, usize>,
    sink: &mut ManagedValueViewMut<'_, T>,
    operation: impl Fn(T, T) -> MResult<T>,
) -> MResult<()> {
    validate_source_len(source.len(), indexes.len())?;
    for position in 0..indexes.len() {
        let index = indexes
            .get_column_major(position)
            .expect("validated selector lane");
        let destination = checked_one_based_index(index, sink.len())?;
        let current = sink
            .get_column_major(destination)
            .expect("initialized destination lane");
        let source = source
            .get_column_major(position)
            .expect("validated source lane");
        sink.try_set_column_major(destination, operation(current, source)?)?;
    }
    Ok(())
}

#[cfg(feature = "matrix")]
fn apply_index_vector_mask<T: ManagedElement>(
    source: ManagedValueView<'_, T>,
    mask: ManagedValueView<'_, bool>,
    sink: &mut ManagedValueViewMut<'_, T>,
    operation: impl Fn(T, T) -> MResult<T>,
) -> MResult<()> {
    validate_mask_len(mask.len(), sink.len())?;
    validate_source_len(source.len(), mask.len())?;
    for position in 0..mask.len() {
        if mask
            .get_column_major(position)
            .expect("validated mask lane")
        {
            let current = sink
                .get_column_major(position)
                .expect("initialized destination lane");
            let source = source
                .get_column_major(position)
                .expect("validated source lane");
            sink.try_set_column_major(position, operation(current, source)?)?;
        }
    }
    Ok(())
}

#[cfg(feature = "matrix")]
fn apply_rows_scalar_positions<T: ManagedElement>(
    source: ManagedValueView<'_, T>,
    rows: ManagedValueView<'_, usize>,
    sink: &mut ManagedValueViewMut<'_, T>,
    operation: impl Fn(T, T) -> MResult<T>,
) -> MResult<()> {
    let source = source.get_column_major(0).ok_or_else(|| {
        function_shape_contract_violation("op_assign_slice", "scalar source is empty")
    })?;
    for position in 0..rows.len() {
        checked_one_based_index(
            rows.get_column_major(position)
                .expect("validated selector lane"),
            sink.rows(),
        )?;
    }
    for column in 0..sink.columns() {
        for position in 0..rows.len() {
            let row = rows
                .get_column_major(position)
                .expect("validated selector lane")
                - 1;
            let destination = row + column * sink.rows();
            let current = sink
                .get_column_major(destination)
                .expect("initialized destination lane");
            sink.try_set_column_major(destination, operation(current, source)?)?;
        }
    }
    Ok(())
}

#[cfg(feature = "matrix")]
fn apply_rows_scalar_mask<T: ManagedElement>(
    source: ManagedValueView<'_, T>,
    mask: ManagedValueView<'_, bool>,
    sink: &mut ManagedValueViewMut<'_, T>,
    operation: impl Fn(T, T) -> MResult<T>,
) -> MResult<()> {
    validate_mask_len(mask.len(), sink.rows())?;
    let source = source.get_column_major(0).ok_or_else(|| {
        function_shape_contract_violation("op_assign_slice", "scalar source is empty")
    })?;
    for column in 0..sink.columns() {
        for row in 0..mask.len() {
            if mask.get_column_major(row).expect("validated mask lane") {
                let destination = row + column * sink.rows();
                let current = sink
                    .get_column_major(destination)
                    .expect("initialized destination lane");
                sink.try_set_column_major(destination, operation(current, source)?)?;
            }
        }
    }
    Ok(())
}

#[cfg(feature = "matrix")]
fn apply_rows_matrix_positions<T: ManagedElement>(
    source: ManagedValueView<'_, T>,
    rows: ManagedValueView<'_, usize>,
    sink: &mut ManagedValueViewMut<'_, T>,
    operation: impl Fn(T, T) -> MResult<T>,
) -> MResult<()> {
    validate_source_len(source.rows(), usize::from(!rows.is_empty()))?;
    if source.columns() != sink.columns() {
        return Err(function_shape_contract_violation(
            "op_assign_slice",
            "source and output column counts disagree",
        ));
    }
    for selected in 0..rows.len() {
        let destination_row = checked_one_based_index(
            rows.get_column_major(selected)
                .expect("validated selector lane"),
            sink.rows(),
        )?;
        let source_row = selected % source.rows();
        for column in 0..sink.columns() {
            let destination = destination_row + column * sink.rows();
            let source_index = source_row + column * source.rows();
            let current = sink
                .get_column_major(destination)
                .expect("initialized destination lane");
            let source = source
                .get_column_major(source_index)
                .expect("validated source lane");
            sink.try_set_column_major(destination, operation(current, source)?)?;
        }
    }
    Ok(())
}

#[cfg(feature = "matrix")]
fn apply_rows_matrix_mask<T: ManagedElement>(
    source: ManagedValueView<'_, T>,
    mask: ManagedValueView<'_, bool>,
    sink: &mut ManagedValueViewMut<'_, T>,
    operation: impl Fn(T, T) -> MResult<T>,
) -> MResult<()> {
    validate_mask_len(mask.len(), sink.rows())?;
    let selected = (0..mask.len())
        .filter(|index| mask.get_column_major(*index).unwrap_or(false))
        .count();
    validate_source_len(source.rows(), selected)?;
    if source.columns() != sink.columns() {
        return Err(function_shape_contract_violation(
            "op_assign_slice",
            "source and output column counts disagree",
        ));
    }
    let mut source_row = 0;
    for row in 0..mask.len() {
        if mask.get_column_major(row).expect("validated mask lane") {
            for column in 0..sink.columns() {
                let destination = row + column * sink.rows();
                let source_index = source_row + column * source.rows();
                let current = sink
                    .get_column_major(destination)
                    .expect("initialized destination lane");
                let source = source
                    .get_column_major(source_index)
                    .expect("validated source lane");
                sink.try_set_column_major(destination, operation(current, source)?)?;
            }
            source_row += 1;
        }
    }
    Ok(())
}

#[macro_export]
macro_rules! impl_op_assign_range_fxn_s {
    ($struct_name:ident, $op:ident, $ix:ty) => {
        #[derive(Debug)]
        pub struct $struct_name<T, MatA, IxVec> {
            pub base: ManagedPort<T>,
            pub source: ManagedPort<T>,
            pub ixes: ManagedPort<$ix>,
            pub sink: ManagedPort<T>,
            pub _marker: PhantomData<(MatA, IxVec)>,
        }
        impl<T, R1: 'static, C1: 'static, S1: 'static, IxVec: 'static> MechFunctionFactory
            for $struct_name<T, naMatrix<T, R1, C1, S1>, IxVec>
        where
            T: Copy
                + Debug
                + Clone
                + Sync
                + Send
                + 'static
                + Div<Output = T>
                + DivAssign
                + Add<Output = T>
                + AddAssign
                + Sub<Output = T>
                + SubAssign
                + Mul<Output = T>
                + MulAssign
                + Zero
                + One
                + PartialEq
                + PartialOrd
                + FunctionRuntimeType,
            T: RuntimeCheckedOpAssign,
            T: ManagedElement + FunctionPortBacking,
            #[cfg(feature = "semantic-compiler")]
            T: CompileConst + ConstElem,
            IxVec: Debug + AsRef<[$ix]>,
            #[cfg(feature = "semantic-compiler")]
            IxVec: CompileConst + ConstElem,
            R1: Dim,
            C1: Dim,
            S1: StorageMut<T, R1, C1> + Clone + Debug,
            naMatrix<T, R1, C1, S1>: Debug,
            naMatrix<T, R1, C1, S1>: FunctionStateBacking,
            IxVec: FunctionPortBacking,
            #[cfg(feature = "semantic-compiler")]
            naMatrix<T, R1, C1, S1>: CompileConst + ConstElem,
        {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
                <naMatrix<T, R1, C1, S1> as FunctionRuntimeType>::REPRESENTATION,
                T::REPRESENTATION,
                IxVec::REPRESENTATION,
            );

            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                mech_core::ImplementationMemoryClass::NoAdditionalScratch
            }

            fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
                Some(&PURE_INDEXED_AXIS_ZERO_RMW_CONTRACT)
            }

            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                let (sink, source, ixes) = invocation.expect_binary()?;
                let _ = sink.try_managed::<naMatrix<T, R1, C1, S1>>()?;
                let _ = ixes.try_managed::<IxVec>()?;
                let base = sink.try_managed_element_base_input::<T>(0)?;
                let sink = sink.try_managed_element::<T>()?;
                let source = source.try_managed_at::<T>(1)?;
                let ixes = ixes.try_managed_element_at::<$ix>(2)?;
                Ok(Box::new(Self {
                    base,
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
            T: Copy
                + Debug
                + Clone
                + Sync
                + Send
                + 'static
                + Div<Output = T>
                + DivAssign
                + Add<Output = T>
                + AddAssign
                + Sub<Output = T>
                + SubAssign
                + Mul<Output = T>
                + MulAssign
                + Zero
                + One
                + PartialEq
                + PartialOrd,
            T: RuntimeCheckedOpAssign,
            T: ManagedElement,
            IxVec: AsRef<[$ix]> + Debug,
            R1: Dim,
            C1: Dim,
            S1: StorageMut<T, R1, C1> + Clone + Debug,
            naMatrix<T, R1, C1, S1>: FunctionStateBacking,
            $ix: ManagedElement,
        {
            fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
                Some(FunctionStatePort::from_cell(self.sink.cell()))
            }
            fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
                Ok(Some(vec![FunctionStatePort::from_cell(self.sink.cell())]))
            }
            fn solve_managed(
                &self,
                frame: &mut mech_core::KernelMemoryFrame<'_>,
                _services: &mut dyn mech_core::MechExecutionServices,
            ) -> MResult<mech_core::ReactiveSolveStatus> {
                frame.with_ternary_typed_port_views(
                    &self.base,
                    &self.source,
                    &self.ixes,
                    &self.sink,
                    |base, source, ixes, sink| {
                        if base.len() != sink.len() || source.len() != 1 {
                            return Err(function_shape_contract_violation(
                                "op_assign_slice",
                                "managed indexed scalar assignment geometry disagrees",
                            ));
                        }
                        sink.try_fill_column_major(|index| {
                            Ok(base.get_column_major(index).expect("validated base lane"))
                        })?;
                        $op!(source, ixes, sink)
                    },
                )?;
                Ok(mech_core::ReactiveSolveStatus::Changed)
            }
            fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
                Some(&PURE_INDEXED_AXIS_ZERO_RMW_CONTRACT)
            }
            fn to_string(&self) -> String {
                format!("{:#?}", self)
            }
        }
        #[cfg(feature = "semantic-compiler")]
        impl<T, R1, C1, S1, IxVec> MechFunctionCompiler
            for $struct_name<T, naMatrix<T, R1, C1, S1>, IxVec>
        where
            T: CompileConst + ConstElem + FunctionRuntimeType,
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
                let out = compile_value_cell_register(self.sink.cell(), ctx)?;
                let source = compile_value_cell_register(self.source.cell(), ctx)?;
                let ixes = compile_value_cell_register(self.ixes.cell(), ctx)?;
                let function = ctx.function_id(&name)?;
                ctx.emit_binop(function, out, source, ixes);
                Ok(out)
            }
        }
    };
}

#[macro_export]
macro_rules! impl_op_assign_range_fxn_v {
    ($struct_name:ident, $op:ident, $ix:ty) => {
        #[cfg(feature = "matrix")]
        #[derive(Debug)]
        pub struct $struct_name<T, MatA, MatB, IxVec> {
            pub base: ManagedPort<T>,
            pub source: ManagedPort<T>,
            pub ixes: ManagedPort<$ix>,
            pub sink: ManagedPort<T>,
            pub _marker: PhantomData<(MatA, MatB, IxVec)>,
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
            T: Copy
                + Debug
                + Clone
                + Sync
                + Send
                + 'static
                + Div<Output = T>
                + DivAssign
                + Add<Output = T>
                + AddAssign
                + Sub<Output = T>
                + SubAssign
                + Mul<Output = T>
                + MulAssign
                + Zero
                + One
                + PartialEq
                + PartialOrd
                + FunctionRuntimeType,
            T: RuntimeCheckedOpAssign,
            T: ManagedElement + FunctionPortBacking,
            #[cfg(feature = "semantic-compiler")]
            T: CompileConst + ConstElem,
            IxVec: Debug + AsRef<[$ix]>,
            #[cfg(feature = "semantic-compiler")]
            IxVec: CompileConst + ConstElem,
            R1: Dim,
            C1: Dim,
            S1: StorageMut<T, R1, C1> + Clone + Debug,
            R2: Dim,
            C2: Dim,
            S2: Storage<T, R2, C2> + Clone + Debug,
            naMatrix<T, R1, C1, S1>: Debug,
            #[cfg(feature = "semantic-compiler")]
            naMatrix<T, R1, C1, S1>: CompileConst + ConstElem,
            naMatrix<T, R2, C2, S2>: Debug,
            naMatrix<T, R1, C1, S1>: FunctionStateBacking,
            naMatrix<T, R2, C2, S2>: FunctionPortBacking,
            IxVec: FunctionPortBacking,
            #[cfg(feature = "semantic-compiler")]
            naMatrix<T, R2, C2, S2>: CompileConst + ConstElem,
        {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
                <naMatrix<T, R1, C1, S1> as FunctionRuntimeType>::REPRESENTATION,
                <naMatrix<T, R2, C2, S2> as FunctionRuntimeType>::REPRESENTATION,
                IxVec::REPRESENTATION,
            );

            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                mech_core::ImplementationMemoryClass::NoAdditionalScratch
            }

            fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
                Some(&PURE_INDEXED_AXIS_ZERO_RMW_CONTRACT)
            }

            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                let (sink, source, ixes) = invocation.expect_binary()?;
                let _ = sink.try_managed::<naMatrix<T, R1, C1, S1>>()?;
                let _ = source.try_managed::<naMatrix<T, R2, C2, S2>>()?;
                let _ = ixes.try_managed::<IxVec>()?;
                let base = sink.try_managed_element_base_input::<T>(0)?;
                let sink = sink.try_managed_element::<T>()?;
                let source = source.try_managed_element_at::<T>(1)?;
                let ixes = ixes.try_managed_element_at::<$ix>(2)?;
                Ok(Box::new(Self {
                    base,
                    sink,
                    source,
                    ixes,
                    _marker: PhantomData::default(),
                }))
            }
        }
        impl<T, R1, C1, S1, R2, C2, S2, IxVec> MechFunctionImpl
            for $struct_name<T, naMatrix<T, R1, C1, S1>, naMatrix<T, R2, C2, S2>, IxVec>
        where
            T: Copy
                + Debug
                + Clone
                + Sync
                + Send
                + 'static
                + Div<Output = T>
                + DivAssign
                + Add<Output = T>
                + AddAssign
                + Sub<Output = T>
                + SubAssign
                + Mul<Output = T>
                + MulAssign
                + Zero
                + One
                + PartialEq
                + PartialOrd,
            T: RuntimeCheckedOpAssign,
            T: ManagedElement,
            IxVec: AsRef<[$ix]> + Debug,
            R1: Dim,
            C1: Dim,
            S1: StorageMut<T, R1, C1> + Clone + Debug,
            R2: Dim,
            C2: Dim,
            S2: Storage<T, R2, C2> + Clone + Debug,
            naMatrix<T, R1, C1, S1>: FunctionStateBacking,
            T: ManagedElement,
            $ix: ManagedElement,
        {
            fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
                Some(FunctionStatePort::from_cell(self.sink.cell()))
            }
            fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
                Ok(Some(vec![FunctionStatePort::from_cell(self.sink.cell())]))
            }
            fn solve_managed(
                &self,
                frame: &mut mech_core::KernelMemoryFrame<'_>,
                _services: &mut dyn mech_core::MechExecutionServices,
            ) -> MResult<mech_core::ReactiveSolveStatus> {
                frame.with_ternary_typed_port_views(
                    &self.base,
                    &self.source,
                    &self.ixes,
                    &self.sink,
                    |base, source, ixes, sink| {
                        if base.len() != sink.len() {
                            return Err(function_shape_contract_violation(
                                "op_assign_slice",
                                "managed indexed matrix assignment geometry disagrees",
                            ));
                        }
                        sink.try_fill_column_major(|index| {
                            Ok(base.get_column_major(index).expect("validated base lane"))
                        })?;
                        $op!(source, ixes, sink)
                    },
                )?;
                Ok(mech_core::ReactiveSolveStatus::Changed)
            }
            fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
                Some(&PURE_INDEXED_AXIS_ZERO_RMW_CONTRACT)
            }
            fn to_string(&self) -> String {
                format!("{:#?}", self)
            }
        }
        #[cfg(feature = "semantic-compiler")]
        impl<T, R1, C1, S1, R2, C2, S2, IxVec> MechFunctionCompiler
            for $struct_name<T, naMatrix<T, R1, C1, S1>, naMatrix<T, R2, C2, S2>, IxVec>
        where
            T: CompileConst + ConstElem + FunctionRuntimeType,
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
                let out = compile_value_cell_register(self.sink.cell(), ctx)?;
                let source = compile_value_cell_register(self.source.cell(), ctx)?;
                let ixes = compile_value_cell_register(self.ixes.cell(), ctx)?;
                let function = ctx.function_id(&name)?;
                ctx.emit_binop(function, out, source, ixes);
                Ok(out)
            }
        }
    };
}

#[cfg(feature = "source")]
#[macro_export]
macro_rules! try_canonical_op_assign_vs_shape {
    (($operation:ident, $sink:ident, $source:ident); $scalar:ty; $matrix:ty) => {
        paste::paste! {
            if $sink.representation() == <$matrix as FunctionRuntimeType>::REPRESENTATION
                && $source.representation() == <$scalar as FunctionRuntimeType>::REPRESENTATION
            {
                return SpecializedFunction::bind_factory::<[<$operation AssignVS>]<$scalar, $matrix>>(
                    $sink.cell()?.clone(),
                    vec![$sink.cell()?.clone(), $source.cell()?.clone()].into_boxed_slice(),
                );
            }
        }
    };
}

#[cfg(feature = "source")]
#[macro_export]
macro_rules! try_canonical_op_assign_vs_scalar {
    (($operation:ident, $sink:ident, $source:ident); $scalar:ty) => {
        #[cfg(feature = "matrix1")]
        $crate::try_canonical_op_assign_vs_shape!(($operation, $sink, $source); $scalar; Matrix1<$scalar>);
        #[cfg(feature = "matrix2")]
        $crate::try_canonical_op_assign_vs_shape!(($operation, $sink, $source); $scalar; Matrix2<$scalar>);
        #[cfg(feature = "matrix2x3")]
        $crate::try_canonical_op_assign_vs_shape!(($operation, $sink, $source); $scalar; Matrix2x3<$scalar>);
        #[cfg(feature = "matrix3x2")]
        $crate::try_canonical_op_assign_vs_shape!(($operation, $sink, $source); $scalar; Matrix3x2<$scalar>);
        #[cfg(feature = "matrix3")]
        $crate::try_canonical_op_assign_vs_shape!(($operation, $sink, $source); $scalar; Matrix3<$scalar>);
        #[cfg(feature = "matrix4")]
        $crate::try_canonical_op_assign_vs_shape!(($operation, $sink, $source); $scalar; Matrix4<$scalar>);
        #[cfg(feature = "matrixd")]
        $crate::try_canonical_op_assign_vs_shape!(($operation, $sink, $source); $scalar; DMatrix<$scalar>);
        #[cfg(feature = "vector2")]
        $crate::try_canonical_op_assign_vs_shape!(($operation, $sink, $source); $scalar; Vector2<$scalar>);
        #[cfg(feature = "vector3")]
        $crate::try_canonical_op_assign_vs_shape!(($operation, $sink, $source); $scalar; Vector3<$scalar>);
        #[cfg(feature = "vector4")]
        $crate::try_canonical_op_assign_vs_shape!(($operation, $sink, $source); $scalar; Vector4<$scalar>);
        #[cfg(feature = "vectord")]
        $crate::try_canonical_op_assign_vs_shape!(($operation, $sink, $source); $scalar; DVector<$scalar>);
        #[cfg(feature = "row_vector2")]
        $crate::try_canonical_op_assign_vs_shape!(($operation, $sink, $source); $scalar; RowVector2<$scalar>);
        #[cfg(feature = "row_vector3")]
        $crate::try_canonical_op_assign_vs_shape!(($operation, $sink, $source); $scalar; RowVector3<$scalar>);
        #[cfg(feature = "row_vector4")]
        $crate::try_canonical_op_assign_vs_shape!(($operation, $sink, $source); $scalar; RowVector4<$scalar>);
        #[cfg(feature = "row_vectord")]
        $crate::try_canonical_op_assign_vs_shape!(($operation, $sink, $source); $scalar; RowDVector<$scalar>);
    };
}

#[cfg(feature = "source")]
#[macro_export]
macro_rules! try_canonical_op_assign_vs {
    (($operation:ident, $sink:ident, $source:ident)) => {
        #[cfg(feature = "u8")]
        $crate::try_canonical_op_assign_vs_scalar!(($operation, $sink, $source); u8);
        #[cfg(feature = "u16")]
        $crate::try_canonical_op_assign_vs_scalar!(($operation, $sink, $source); u16);
        #[cfg(feature = "u32")]
        $crate::try_canonical_op_assign_vs_scalar!(($operation, $sink, $source); u32);
        #[cfg(feature = "u64")]
        $crate::try_canonical_op_assign_vs_scalar!(($operation, $sink, $source); u64);
        #[cfg(feature = "u128")]
        $crate::try_canonical_op_assign_vs_scalar!(($operation, $sink, $source); u128);
        #[cfg(feature = "i8")]
        $crate::try_canonical_op_assign_vs_scalar!(($operation, $sink, $source); i8);
        #[cfg(feature = "i16")]
        $crate::try_canonical_op_assign_vs_scalar!(($operation, $sink, $source); i16);
        #[cfg(feature = "i32")]
        $crate::try_canonical_op_assign_vs_scalar!(($operation, $sink, $source); i32);
        #[cfg(feature = "i64")]
        $crate::try_canonical_op_assign_vs_scalar!(($operation, $sink, $source); i64);
        #[cfg(feature = "i128")]
        $crate::try_canonical_op_assign_vs_scalar!(($operation, $sink, $source); i128);
        #[cfg(feature = "f32")]
        $crate::try_canonical_op_assign_vs_scalar!(($operation, $sink, $source); f32);
        #[cfg(feature = "f64")]
        $crate::try_canonical_op_assign_vs_scalar!(($operation, $sink, $source); f64);
        #[cfg(feature = "r64")]
        $crate::try_canonical_op_assign_vs_scalar!(($operation, $sink, $source); R64);
        #[cfg(feature = "c64")]
        $crate::try_canonical_op_assign_vs_scalar!(($operation, $sink, $source); C64);
    };
}

#[cfg(feature = "source")]
#[macro_export]
macro_rules! impl_canonical_op_assign_specializers {
    (
        $value_specializer:ident,
        $range_specializer:ident,
        $range_all_specializer:ident,
        $operation:ident,
        $value_prefix:literal,
        $range_prefix:literal,
        $range_all_prefix:literal
    ) => {
        pub struct $value_specializer;

        impl CanonicalFunctionSpecializer for $value_specializer {
            fn specialize_invocation(
                &self,
                invocation: &SpecializationInvocation,
                context: &mut SpecializationContext<'_>,
            ) -> MResult<SpecializedFunction> {
                if invocation.len() != 2 {
                    return Err(MechError::new(
                        IncorrectNumberOfArguments {
                            expected: 2,
                            found: invocation.len(),
                        },
                        None,
                    )
                    .with_compiler_loc());
                }
                let sink = invocation.input(0).expect("validated assignment sink");
                let source = invocation.input(1).expect("validated assignment source");
                context.bind_resolved_runtime_existing_output(
                    RuntimeBindingSelector::Operation(context.resolved_call()?.operation.id),
                    ExecutionTarget::DirectRuntime,
                    sink,
                    &[source],
                )
            }
        }

        pub struct $range_specializer;

        impl CanonicalFunctionSpecializer for $range_specializer {
            fn specialize_invocation(
                &self,
                invocation: &SpecializationInvocation,
                context: &mut SpecializationContext<'_>,
            ) -> MResult<SpecializedFunction> {
                if invocation.len() != 3 {
                    return Err(MechError::new(
                        IncorrectNumberOfArguments {
                            expected: 3,
                            found: invocation.len(),
                        },
                        None,
                    )
                    .with_compiler_loc());
                }
                let sink = invocation
                    .input(0)
                    .expect("validated indexed assignment sink");
                let source = invocation
                    .input(1)
                    .expect("validated indexed assignment source");
                let index = invocation.input(2).expect("validated assignment index");
                context.bind_resolved_runtime_existing_output(
                    RuntimeBindingSelector::Operation(context.resolved_call()?.operation.id),
                    ExecutionTarget::DirectRuntime,
                    sink,
                    &[source, index],
                )
            }
        }

        pub struct $range_all_specializer;

        impl CanonicalFunctionSpecializer for $range_all_specializer {
            fn specialize_invocation(
                &self,
                invocation: &SpecializationInvocation,
                context: &mut SpecializationContext<'_>,
            ) -> MResult<SpecializedFunction> {
                if invocation.len() != 4 {
                    return Err(MechError::new(
                        IncorrectNumberOfArguments {
                            expected: 4,
                            found: invocation.len(),
                        },
                        None,
                    )
                    .with_compiler_loc());
                }
                let sink = invocation
                    .input(0)
                    .expect("validated indexed assignment sink");
                let source = invocation
                    .input(1)
                    .expect("validated indexed assignment source");
                let row_index = invocation.input(2).expect("validated row assignment index");
                invocation
                    .input(3)
                    .expect("validated all-selection input")
                    .require_matrix_all_selection()?;
                context.bind_resolved_runtime_existing_output(
                    RuntimeBindingSelector::Operation(context.resolved_call()?.operation.id),
                    ExecutionTarget::DirectRuntime,
                    sink,
                    &[source, row_index],
                )
            }
        }
    };
}

#[macro_export]
macro_rules! impl_assign_scalar_scalar {
  ($op_name:tt, $checked_op:ident) => {
    paste::paste! {
      #[derive(Debug)]
      pub(crate) struct [<$op_name AssignSS>]<T> {
        base: ManagedPort<T>,
        sink: ManagedPort<T>,
        source: ManagedPort<T>,
      }
      impl<T> MechFunctionFactory for [<$op_name AssignSS>]<T>
      where
        T: Debug + Clone + Sync + Send + 'static +
           $op_name<Output = T> + [<$op_name Assign>] +
           PartialEq + PartialOrd + FunctionRuntimeType,
        T: RuntimeCheckedOpAssign,
        T: ManagedElement + FunctionPortBacking,
        #[cfg(feature = "semantic-compiler")]
        T: CompileConst + ConstElem,
        T: FunctionStateBacking,
      {
        const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::unary(
          T::REPRESENTATION,
          T::REPRESENTATION,
        );

            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                mech_core::ImplementationMemoryClass::NoAdditionalScratch
            }

        fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
          Some(&PURE_WHOLE_VALUE_RMW_CONTRACT)
        }

        fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
          let (sink, source) = invocation.expect_unary()?;
          let base = sink.try_managed_base_input::<T>(0)?;
          let output = sink.try_managed::<T>()?;
          let source = source.try_managed_at::<T>(1)?;
          Ok(Box::new(Self { base, sink: output, source }))
        }

      }
      impl<T> MechFunctionImpl for [<$op_name AssignSS>]<T>
      where
        T: Debug + Clone + Sync + Send + 'static +
           $op_name<Output = T> + [<$op_name Assign>] +
           PartialEq + PartialOrd,
        T: RuntimeCheckedOpAssign,
        T: ManagedElement,
        T: FunctionStateBacking,
      {
        fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
          Some(FunctionStatePort::from_cell(self.sink.cell()))
        }
        fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
          Ok(Some(vec![FunctionStatePort::from_cell(self.sink.cell())]))
        }
        fn solve_managed(
                &self,
                frame: &mut mech_core::KernelMemoryFrame<'_>,
                _services: &mut dyn mech_core::MechExecutionServices,
            ) -> MResult<mech_core::ReactiveSolveStatus> {
          frame.with_binary_port_views(&self.base, &self.source, &self.sink, |base, source, sink| {
            if base.len() != 1 || source.len() != 1 || sink.len() != 1 {
              return Err(MechError::new(GenericError { msg: "scalar assignment requires scalar managed ports".into() }, None));
            }
            let next = $checked_op(base.get_column_major(0).unwrap(), source.get_column_major(0).unwrap())?;
            sink.try_fill_column_major(|_| Ok(next))
          })?;
            Ok(mech_core::ReactiveSolveStatus::Changed)
        }
        fn reactive_node_kind(&self) -> ReactiveNodeKind { ReactiveNodeKind::Register }
        fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
          Some(&PURE_WHOLE_VALUE_RMW_CONTRACT)
        }
        fn to_string(&self) -> String { format!("{:#?}", self) }
      }
      #[cfg(feature = "semantic-compiler")]
      impl<T> MechFunctionCompiler for [<$op_name AssignSS>]<T>
      where
        T: CompileConst + ConstElem + FunctionRuntimeType,
      {
        fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
          let name = format!("{}AssignSS<{}>", stringify!($op_name), <T as FunctionRuntimeType>::REPRESENTATION);
          let out = compile_value_cell_register(self.sink.cell(), ctx)?;
          let source = compile_value_cell_register(self.source.cell(), ctx)?;
          let function = ctx.function_id(&name)?;
          ctx.emit_unop(function, out, source);
          Ok(out)
        }
      }
    }
  };
}

#[macro_export]
macro_rules! impl_assign_vector_vector {
  ($op_name:tt, $checked_op:ident) => {
    paste::paste! {
      #[derive(Debug)]
      pub struct [<$op_name AssignVV>]<T, MatA, MatB> {
        pub base: ManagedPort<T>,
        pub sink: ManagedPort<T>,
        pub source: ManagedPort<T>,
        _marker: PhantomData<(MatA, MatB)>,
      }
      impl<T, MatA, MatB> MechFunctionFactory for [<$op_name AssignVV>]<T, MatA, MatB>
      where
        T: Debug + Clone + Sync + Send + 'static + [<$op_name Assign>] +
        FunctionRuntimeType,
        T: RuntimeCheckedOpAssign,
        T: ManagedElement + FunctionPortBacking,
        #[cfg(feature = "semantic-compiler")]
        T: CompileConst + ConstElem,
        for<'a> &'a MatA: IntoIterator<Item = &'a T>,
        for<'a> &'a mut MatA: IntoIterator<Item = &'a mut T>,
        for<'a> &'a MatB: IntoIterator<Item = &'a T>,
        MatA: Debug + Clone + FunctionRuntimeType + 'static,
        #[cfg(feature = "semantic-compiler")]
        MatA: CompileConst + ConstElem,
        MatB: Debug + FunctionRuntimeType + 'static,
        MatA: FunctionStateBacking,
        MatB: FunctionPortBacking,
        #[cfg(feature = "semantic-compiler")]
        MatB: CompileConst + ConstElem,
      {
        const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::unary(
          MatA::REPRESENTATION,
          MatB::REPRESENTATION,
        );

            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                mech_core::ImplementationMemoryClass::NoAdditionalScratch
            }

        fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
          Some(&PURE_WHOLE_VALUE_RMW_CONTRACT)
        }

        fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
          let (sink, source) = invocation.expect_unary()?;
          let _ = sink.try_managed::<MatA>()?;
          let _ = source.try_managed::<MatB>()?;
          let base = sink.try_managed_element_base_input::<T>(0)?;
          let output = sink.try_managed_element::<T>()?;
          let source = source.try_managed_element_at::<T>(1)?;
          Ok(Box::new(Self { base, sink: output, source, _marker: PhantomData::default() }))
        }

      }
      impl<T, MatA, MatB> MechFunctionImpl for [<$op_name AssignVV>]<T, MatA, MatB>
      where
        T: Debug + Clone + Sync + Send + 'static + [<$op_name Assign>],
        T: RuntimeCheckedOpAssign,
        T: ManagedElement,
        for<'a> &'a MatA: IntoIterator<Item = &'a T>,
        for<'a> &'a mut MatA: IntoIterator<Item = &'a mut T>,
        for<'a> &'a MatB: IntoIterator<Item = &'a T>,
        MatA: Debug + Clone + FunctionStateBacking + 'static,
        MatB: Debug,
      {
        fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
          Some(FunctionStatePort::from_cell(self.sink.cell()))
        }
        fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
          Ok(Some(vec![FunctionStatePort::from_cell(self.sink.cell())]))
        }
        fn solve_managed(
                &self,
                frame: &mut mech_core::KernelMemoryFrame<'_>,
                _services: &mut dyn mech_core::MechExecutionServices,
            ) -> MResult<mech_core::ReactiveSolveStatus> {
          frame.with_binary_port_views(&self.base, &self.source, &self.sink, |base, source, sink| {
            if base.len() != source.len() || base.len() != sink.len() {
              return Err(MechError::new(GenericError { msg: "matrix assignment managed-port lengths disagree".into() }, None));
            }
            sink.try_fill_column_major(|index| {
              $checked_op(base.get_column_major(index).unwrap(), source.get_column_major(index).unwrap())
            })
          })?;
            Ok(mech_core::ReactiveSolveStatus::Changed)
        }
        fn reactive_node_kind(&self) -> ReactiveNodeKind { ReactiveNodeKind::Register }
        fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
          Some(&PURE_WHOLE_VALUE_RMW_CONTRACT)
        }
        fn to_string(&self) -> String {format!("{:#?}", self)}
      }
      #[cfg(feature = "semantic-compiler")]
      impl<T, MatA, MatB> MechFunctionCompiler for [<$op_name AssignVV>]<T, MatA, MatB>
      where
        T: CompileConst + ConstElem + FunctionRuntimeType,
        MatA: CompileConst + ConstElem + FunctionRuntimeType,
        MatB: CompileConst + ConstElem + FunctionRuntimeType,
      {
        fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
          let name = format!("{}AssignVV<{}>", stringify!($op_name), <MatA as FunctionRuntimeType>::REPRESENTATION);
          let out = compile_value_cell_register(self.sink.cell(), ctx)?;
          let source = compile_value_cell_register(self.source.cell(), ctx)?;
          let function = ctx.function_id(&name)?;
          ctx.emit_unop(function, out, source);
          Ok(out)
        }
      }
    }
  };
}

#[macro_export]
macro_rules! impl_assign_vector_scalar {
  ($op_name:tt, $checked_op:ident) => {
    paste::paste! {
      #[derive(Debug)]
      pub struct [<$op_name AssignVS>]<T, MatA> {
        pub base: ManagedPort<T>,
        pub sink: ManagedPort<T>,
        pub source: ManagedPort<T>,
        _marker: PhantomData<MatA>,
      }
      impl<T, MatA> MechFunctionFactory for [<$op_name AssignVS>]<T, MatA>
      where
        T: Debug + Clone + Sync + Send + 'static + [<$op_name Assign>] +
        FunctionRuntimeType,
        T: RuntimeCheckedOpAssign,
        T: ManagedElement,
        #[cfg(feature = "semantic-compiler")]
        T: CompileConst + ConstElem,
        for<'a> &'a MatA: IntoIterator<Item = &'a T>,
        for<'a> &'a mut MatA: IntoIterator<Item = &'a mut T>,
        MatA: Debug + Clone + FunctionRuntimeType + 'static,
        MatA: FunctionStateBacking,
        T: FunctionPortBacking,
        #[cfg(feature = "semantic-compiler")]
        MatA: CompileConst + ConstElem,
      {
        const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
          MatA::REPRESENTATION,
          FunctionValueRepresentation::AnyValue,
          T::REPRESENTATION,
        );

            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                mech_core::ImplementationMemoryClass::NoAdditionalScratch
            }

        fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
          Some(&PURE_WHOLE_VALUE_RMW_CONTRACT)
        }

        fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
          let (sink, _base, source) = invocation.expect_binary()?;
          let _ = sink.try_managed::<MatA>()?;
          let base = sink.try_managed_element_base_input::<T>(0)?;
          let output = sink.try_managed_element::<T>()?;
          let source = source.try_managed_at::<T>(1)?;
          Ok(Box::new(Self { base, sink: output, source, _marker: PhantomData::default() }))
        }

      }
      impl<T, MatA> MechFunctionImpl for [<$op_name AssignVS>]<T, MatA>
      where
        T: Debug + Clone + Sync + Send + 'static + [<$op_name Assign>],
        T: RuntimeCheckedOpAssign,
        T: ManagedElement,
        for<'a> &'a MatA: IntoIterator<Item = &'a T>,
        for<'a> &'a mut MatA: IntoIterator<Item = &'a mut T>,
        MatA: Debug + Clone + FunctionStateBacking + 'static,
      {
        fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
          Some(FunctionStatePort::from_cell(self.sink.cell()))
        }
        fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
          Ok(Some(vec![FunctionStatePort::from_cell(self.sink.cell())]))
        }
        fn solve_managed(
                &self,
                frame: &mut mech_core::KernelMemoryFrame<'_>,
                _services: &mut dyn mech_core::MechExecutionServices,
            ) -> MResult<mech_core::ReactiveSolveStatus> {
          let source = frame.with_port_slice(&self.source, |source| source[0])?;
          frame.with_unary_port_views(&self.base, &self.sink, |base, sink| {
            if base.len() != sink.len() {
              return Err(MechError::new(GenericError { msg: "matrix scalar-assignment managed-port lengths disagree".into() }, None));
            }
            sink.try_fill_column_major(|index| {
              $checked_op(base.get_column_major(index).unwrap(), source)
            })
          })?;
            Ok(mech_core::ReactiveSolveStatus::Changed)
        }
        fn reactive_node_kind(&self) -> ReactiveNodeKind { ReactiveNodeKind::Register }
        fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
          Some(&PURE_WHOLE_VALUE_RMW_CONTRACT)
        }
        fn to_string(&self) -> String {format!("{:#?}", self)}
      }
      #[cfg(feature = "semantic-compiler")]
      impl<T, MatA> MechFunctionCompiler for [<$op_name AssignVS>]<T, MatA>
      where
        T: CompileConst + ConstElem + FunctionRuntimeType,
        MatA: CompileConst + ConstElem + FunctionRuntimeType,
      {
        fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
          let name = format!("{}AssignVS<{}>", stringify!($op_name), <MatA as FunctionRuntimeType>::REPRESENTATION);
          let out = compile_value_cell_register(self.sink.cell(), ctx)?;
          let source = compile_value_cell_register(self.source.cell(), ctx)?;
          let function = ctx.function_id(&name)?;
          ctx.emit_unop(function, out, source);
          Ok(out)
        }
      }
    }
  }
}

#[cfg(not(any(feature = "matrix", feature = "source")))]
macro_rules! impl_scalar_op_assign_module {
    ($module:ident, $op_name:tt, $checked_op:ident) => {
        pub mod $module {
            use super::*;

            impl_assign_scalar_scalar!($op_name, $checked_op);
            impl_assign_vector_vector!($op_name, $checked_op);
            impl_assign_vector_scalar!($op_name, $checked_op);
        }
    };
}

#[cfg(all(
    feature = "add_assign",
    not(any(feature = "matrix", feature = "source"))
))]
impl_scalar_op_assign_module!(add_assign, Add, checked_add_assign);
#[cfg(all(
    feature = "div_assign",
    not(any(feature = "matrix", feature = "source"))
))]
impl_scalar_op_assign_module!(div_assign, Div, checked_div_assign);
#[cfg(all(
    feature = "mul_assign",
    not(any(feature = "matrix", feature = "source"))
))]
impl_scalar_op_assign_module!(mul_assign, Mul, checked_mul_assign);
#[cfg(all(
    feature = "sub_assign",
    not(any(feature = "matrix", feature = "source"))
))]
impl_scalar_op_assign_module!(sub_assign, Sub, checked_sub_assign);

#[cfg(test)]
mod port_tests;
