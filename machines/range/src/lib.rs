#![cfg_attr(not(test), no_main)]
#![feature(where_clause_attrs)]

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[doc(hidden)]
#[cfg(feature = "native-link")]
pub mod __mech_native {
    pub use crate::catalog::__mech_native::*;
}

#[cfg(feature = "matrix")]
extern crate nalgebra as na;
extern crate paste;

#[cfg(feature = "runtime")]
pub mod catalog;
#[cfg(feature = "runtime")]
pub use self::catalog::*;

#[cfg(feature = "exclusive")]
pub mod exclusive;
#[cfg(feature = "exclusive")]
pub mod exclusive_increment;
#[cfg(feature = "inclusive")]
pub mod inclusive;
#[cfg(feature = "inclusive")]
pub mod inclusive_increment;

#[cfg(feature = "exclusive")]
pub use self::exclusive::*;
#[cfg(feature = "exclusive")]
pub use self::exclusive_increment::*;
#[cfg(feature = "inclusive")]
pub use self::inclusive::*;
#[cfg(feature = "inclusive")]
pub use self::inclusive_increment::*;

#[cfg(test)]
mod port_tests;

use mech_core::{ManagedElement, ManagedValueViewMut, MechErrorKind};

#[cfg(test)]
fn test_managed_factory<F: mech_core::MechFunctionFactory>(
    invocation: mech_core::FunctionInvocation,
    operation: &'static str,
) -> mech_core::SpecializedFunction {
    use mech_core::{
        ExecutionTarget, ResolvedOperationDescriptor, RuntimeFunctionId, SpecializedFunction,
    };

    let implementation = F::new_invocation(invocation.clone()).unwrap();
    let contract = F::declared_operation_contract()
        .or_else(|| implementation.semantic_operation_contract())
        .expect("managed range fixture requires an operation contract");
    SpecializedFunction::syntax_directed(
        (implementation, invocation),
        ResolvedOperationDescriptor::from_name(operation, contract.clone()).unwrap(),
        RuntimeFunctionId::from_name(operation),
        ExecutionTarget::DirectRuntime,
        F::implementation_memory_class(),
    )
    .unwrap()
}

#[cfg(test)]
fn assert_test_value(actual: &mech_core::ValueCell, expected: mech_core::ValueCell) {
    let actual = actual.snapshot().unwrap();
    let expected = expected.snapshot().unwrap();
    let actual_schemas = actual.schemas().unwrap();
    let expected_schemas = expected.schemas().unwrap();
    assert!(
        actual
            .language_eq(&actual_schemas, &expected, &expected_schemas)
            .unwrap()
    );
}

#[cfg(feature = "range")]
fn fill_managed_range<T>(
    from: T,
    step: Option<T>,
    to: T,
    inclusive: bool,
    output: &mut ManagedValueViewMut<'_, T>,
) -> mech_core::MResult<()>
where
    T: ManagedElement + mech_core::CanonicalRangeScalar,
{
    let size = mech_core::canonical_range_size(from, step, to, inclusive).map_err(|error| {
        mech_core::function_shape_contract_violation(
            "range_construction",
            format!("canonical typed range cardinality failed: {error:?}"),
        )
    })?;
    if size != output.len() {
        return Err(mech_core::function_shape_contract_violation(
            "range_construction",
            "resolved output cardinality disagrees with the managed stage",
        ));
    }
    let step = step.unwrap_or_else(T::one);
    let mut current = from;
    output.try_fill_column_major(|index| {
        let value = current;
        if index + 1 < size {
            current = current.checked_step(step).ok_or_else(|| {
                mech_core::function_shape_contract_violation(
                    "range_construction",
                    "canonical typed range evaluation overflowed",
                )
            })?;
        }
        Ok(value)
    })
}

#[cfg(feature = "range")]
fn planned_range_output_shape<T>(
    from: &mech_core::ManagedPort<T>,
    step: Option<&mech_core::ManagedPort<T>>,
    to: &mech_core::ManagedPort<T>,
    output: &mech_core::ManagedPort<T>,
    inclusive: bool,
) -> mech_core::MResult<mech_core::ShapeInstance>
where
    T: ManagedElement + mech_core::CanonicalMatrixElementBacking + mech_core::CanonicalRangeScalar,
{
    let scalar = |port: &mech_core::ManagedPort<T>| -> mech_core::MResult<T> {
        let value = port.cell().snapshot()?;
        T::from_data(value.data()).ok_or_else(|| {
            mech_core::function_shape_contract_violation(
                "range_construction",
                "range scalar snapshot disagrees with its closed element type",
            )
        })
    };
    let from = scalar(from)?;
    let step = step.map(scalar).transpose()?;
    let to = scalar(to)?;
    let size = mech_core::canonical_range_size(from, step, to, inclusive).map_err(|error| {
        mech_core::function_shape_contract_violation(
            "range_construction",
            format!("canonical typed range cardinality failed: {error:?}"),
        )
    })?;
    let value = output.cell().snapshot()?;
    let schemas = value.schemas().ok_or_else(|| {
        mech_core::function_shape_contract_violation(
            "range_construction",
            "range output has no retained schema table",
        )
    })?;
    let schema = schemas.entry(value.schema()).ok_or_else(|| {
        mech_core::function_shape_contract_violation(
            "range_construction",
            "range output schema is missing from its retained table",
        )
    })?;
    mech_core::shape_for_resolved_extents(schema.schema(), &[1, size as u64])
        .map_err(|error| mech_core::MechError::new(error, None).with_compiler_loc())
}

#[cfg(all(feature = "runtime", feature = "range"))]
#[macro_export]
macro_rules! impl_managed_binary_range {
    ($name:ident, $contract:expr, $inclusive:expr) => {
        #[derive(Debug)]
        pub struct $name<T, MatA> {
            from: mech_core::ManagedPort<T>,
            to: mech_core::ManagedPort<T>,
            out: mech_core::ManagedPort<T>,
            marker: core::marker::PhantomData<MatA>,
        }

        impl<T, MatA> mech_core::MechFunctionFactory for $name<T, MatA>
        where
            T: ManagedElement
                + CanonicalMatrixElementBacking
                + FunctionRuntimeType
                + FunctionPortBacking
                + mech_core::CanonicalRangeScalar,
            MatA: FunctionStateBacking,
        {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
                MatA::REPRESENTATION,
                T::REPRESENTATION,
                T::REPRESENTATION,
            );

            fn implementation_memory_class() -> ImplementationMemoryClass {
                ImplementationMemoryClass::NoAdditionalScratch
            }

            fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
                Some($contract)
            }

            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                let (out, from, to) = invocation.expect_binary()?;
                let _ = out.try_managed::<MatA>()?;
                Ok(Box::new(Self {
                    from: from.try_managed_element::<T>()?,
                    to: to.try_managed_element::<T>()?,
                    out: out.try_managed_element::<T>()?,
                    marker: core::marker::PhantomData,
                }))
            }
        }

        impl<T, MatA> MechFunctionImpl for $name<T, MatA>
        where
            T: ManagedElement + CanonicalMatrixElementBacking + mech_core::CanonicalRangeScalar,
            MatA: FunctionStateBacking,
        {
            fn planned_output_shapes(&self) -> MResult<Option<Box<[ShapeInstance]>>> {
                Ok(Some(
                    vec![crate::planned_range_output_shape(
                        &self.from, None, &self.to, &self.out, $inclusive,
                    )?]
                    .into_boxed_slice(),
                ))
            }

            fn solve_managed(
                &self,
                frame: &mut KernelMemoryFrame<'_>,
                _services: &mut dyn MechExecutionServices,
            ) -> MResult<ReactiveSolveStatus> {
                frame.with_binary_port_views(
                    &self.from,
                    &self.to,
                    &self.out,
                    |from, to, out| {
                        if from.len() != 1 || to.len() != 1 {
                            return Err(function_shape_contract_violation(
                                "range_construction",
                                "range endpoints must be scalar",
                            ));
                        }
                        crate::fill_managed_range(
                            from.get_column_major(0).expect("validated range start"),
                            None,
                            to.get_column_major(0).expect("validated range end"),
                            $inclusive,
                            out,
                        )
                    },
                )?;
                Ok(ReactiveSolveStatus::Changed)
            }

            fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
                Some(FunctionStatePort::from_cell(self.out.cell()))
            }

            fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
                Ok(Some(vec![FunctionStatePort::from_cell(self.out.cell())]))
            }

            fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
                Some($contract)
            }

            fn to_string(&self) -> String {
                stringify!($name).into()
            }
        }

        #[cfg(feature = "semantic-compiler")]
        impl<T, MatA> MechFunctionCompiler for $name<T, MatA>
        where
            T: ManagedElement + FunctionRuntimeType,
            MatA: FunctionStateBacking,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!(
                    "{}<{}{}>",
                    stringify!($name),
                    T::REPRESENTATION,
                    function_matrix_storage_name::<MatA>(),
                );
                let output = compile_value_cell_register(self.out.cell(), ctx)?;
                let from = compile_value_cell_register(self.from.cell(), ctx)?;
                let to = compile_value_cell_register(self.to.cell(), ctx)?;
                ctx.emit_binop(hash_str(&name), output, from, to);
                Ok(output)
            }
        }
    };
}

#[cfg(all(feature = "runtime", feature = "range"))]
#[macro_export]
macro_rules! impl_managed_ternary_range {
    ($name:ident, $contract:expr, $inclusive:expr) => {
        #[derive(Debug)]
        pub struct $name<T, MatA> {
            from: mech_core::ManagedPort<T>,
            step: mech_core::ManagedPort<T>,
            to: mech_core::ManagedPort<T>,
            out: mech_core::ManagedPort<T>,
            marker: core::marker::PhantomData<MatA>,
        }

        impl<T, MatA> mech_core::MechFunctionFactory for $name<T, MatA>
        where
            T: ManagedElement
                + CanonicalMatrixElementBacking
                + FunctionRuntimeType
                + FunctionPortBacking
                + mech_core::CanonicalRangeScalar,
            MatA: FunctionStateBacking,
        {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::ternary(
                MatA::REPRESENTATION,
                T::REPRESENTATION,
                T::REPRESENTATION,
                T::REPRESENTATION,
            );

            fn implementation_memory_class() -> ImplementationMemoryClass {
                ImplementationMemoryClass::NoAdditionalScratch
            }

            fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
                Some($contract)
            }

            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                let (out, from, step, to) = invocation.expect_ternary()?;
                let _ = out.try_managed::<MatA>()?;
                Ok(Box::new(Self {
                    from: from.try_managed_element::<T>()?,
                    step: step.try_managed_element::<T>()?,
                    to: to.try_managed_element::<T>()?,
                    out: out.try_managed_element::<T>()?,
                    marker: core::marker::PhantomData,
                }))
            }
        }

        impl<T, MatA> MechFunctionImpl for $name<T, MatA>
        where
            T: ManagedElement + CanonicalMatrixElementBacking + mech_core::CanonicalRangeScalar,
            MatA: FunctionStateBacking,
        {
            fn planned_output_shapes(&self) -> MResult<Option<Box<[ShapeInstance]>>> {
                Ok(Some(
                    vec![crate::planned_range_output_shape(
                        &self.from,
                        Some(&self.step),
                        &self.to,
                        &self.out,
                        $inclusive,
                    )?]
                    .into_boxed_slice(),
                ))
            }

            fn solve_managed(
                &self,
                frame: &mut KernelMemoryFrame<'_>,
                _services: &mut dyn MechExecutionServices,
            ) -> MResult<ReactiveSolveStatus> {
                frame.with_ternary_typed_port_views(
                    &self.from,
                    &self.step,
                    &self.to,
                    &self.out,
                    |from, step, to, out| {
                        if from.len() != 1 || step.len() != 1 || to.len() != 1 {
                            return Err(function_shape_contract_violation(
                                "range_construction",
                                "range endpoints and increment must be scalar",
                            ));
                        }
                        crate::fill_managed_range(
                            from.get_column_major(0).expect("validated range start"),
                            Some(step.get_column_major(0).expect("validated range step")),
                            to.get_column_major(0).expect("validated range end"),
                            $inclusive,
                            out,
                        )
                    },
                )?;
                Ok(ReactiveSolveStatus::Changed)
            }

            fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
                Some(FunctionStatePort::from_cell(self.out.cell()))
            }

            fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
                Ok(Some(vec![FunctionStatePort::from_cell(self.out.cell())]))
            }

            fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
                Some($contract)
            }

            fn to_string(&self) -> String {
                stringify!($name).into()
            }
        }

        #[cfg(feature = "semantic-compiler")]
        impl<T, MatA> MechFunctionCompiler for $name<T, MatA>
        where
            T: ManagedElement + FunctionRuntimeType,
            MatA: FunctionStateBacking,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!(
                    "{}<{}{}>",
                    stringify!($name),
                    T::REPRESENTATION,
                    function_matrix_storage_name::<MatA>(),
                );
                let output = compile_value_cell_register(self.out.cell(), ctx)?;
                let from = compile_value_cell_register(self.from.cell(), ctx)?;
                let step = compile_value_cell_register(self.step.cell(), ctx)?;
                let to = compile_value_cell_register(self.to.cell(), ctx)?;
                ctx.emit_ternop(hash_str(&name), output, from, step, to);
                Ok(output)
            }
        }
    };
}

// ----------------------------------------------------------------------------
// Range Library
// ----------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct EmptyRangeError;
impl MechErrorKind for EmptyRangeError {
    fn name(&self) -> &str {
        "EmptyRange"
    }
    fn message(&self) -> String {
        "Range size must be > 0".to_string()
    }
}

#[derive(Debug, Clone)]
pub struct RangeSizeOverflowError;

impl MechErrorKind for RangeSizeOverflowError {
    fn name(&self) -> &str {
        "RangeSizeOverflow"
    }
    fn message(&self) -> String {
        "Range size overflow".to_string()
    }
}

#[doc(hidden)]
#[macro_export]
#[cfg(feature = "source")]
macro_rules! bind_dynamic_binary_range {
    ($factory:ident, $scalar:ty, $first:expr, $second:expr, $inclusive:expr, $context:expr) => {{
        let inputs = vec![$first.cell()?.clone(), $second.cell()?.clone()].into_boxed_slice();
        let size = $crate::catalog::canonical_range_size(&inputs, $inclusive, false)?;
        let semantic_inputs = [$first, $second];
        return $context.bind_resolved_runtime(
            mech_core::RuntimeBindingSelector::Operation($context.resolved_call()?.operation.id),
            mech_core::ExecutionTarget::DirectRuntime,
            vec![vec![1, size as u64].into_boxed_slice()].into_boxed_slice(),
            &semantic_inputs,
        );
    }};
}

#[doc(hidden)]
#[macro_export]
#[cfg(feature = "source")]
macro_rules! bind_dynamic_ternary_range {
    ($factory:ident, $scalar:ty, $first:expr, $step:expr, $last:expr, $inclusive:expr, $context:expr) => {{
        let inputs = vec![
            $first.cell()?.clone(),
            $step.cell()?.clone(),
            $last.cell()?.clone(),
        ]
        .into_boxed_slice();
        let size = $crate::catalog::canonical_range_size(&inputs, $inclusive, true)?;
        let semantic_inputs = [$first, $step, $last];
        return $context.bind_resolved_runtime(
            mech_core::RuntimeBindingSelector::Operation($context.resolved_call()?.operation.id),
            mech_core::ExecutionTarget::DirectRuntime,
            vec![vec![1, size as u64].into_boxed_slice()].into_boxed_slice(),
            &semantic_inputs,
        );
    }};
}

#[macro_export]
macro_rules! range_size_to_usize {
    // Float f32 branch
    ($diff:expr, f32) => {{
        let v: f32 = $diff;
        if v < 0.0 {
            return Err(MechError::new(RangeSizeOverflowError {}, None).with_compiler_loc());
        }
        v as usize
    }};

    // Float f64 branch
    ($diff:expr, f64) => {{
        let v: f64 = $diff;
        if v < 0.0 {
            return Err(MechError::new(RangeSizeOverflowError {}, None).with_compiler_loc());
        }
        v as usize
    }};

    // Integer branch
    ($diff:expr, $ty:ty) => {{
        $diff
            .try_into()
            .map_err(|_| MechError::new(RangeSizeOverflowError {}, None).with_compiler_loc())?
    }};
}
