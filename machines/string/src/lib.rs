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
use std::sync::LazyLock;

#[cfg(test)]
#[path = "../tests/support/r6_allocation_probe.rs"]
mod allocation_probe;

#[cfg(test)]
#[global_allocator]
static TEST_ALLOCATOR: allocation_probe::ProbeAllocator = allocation_probe::ProbeAllocator;

static PURE_STRING_BINARY_EXACT_SCALAR: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| string_binary_contract(ChangeDetectionPolicy::ExactScalar));
static PURE_STRING_BINARY_KERNEL_REPORTED: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| string_binary_contract(ChangeDetectionPolicy::KernelReported));

fn string_binary_contract(change_detection: ChangeDetectionPolicy) -> OperationContractDeclaration {
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

fn string_binary_full_write_contract(
    output: FunctionValueRepresentation,
) -> &'static OperationContractDeclaration {
    match output {
        FunctionValueRepresentation::Matrix { .. } => &PURE_STRING_BINARY_KERNEL_REPORTED,
        _ => &PURE_STRING_BINARY_EXACT_SCALAR,
    }
}

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

#[cfg(feature = "runtime")]
pub mod catalog;
#[cfg(feature = "runtime")]
pub use self::catalog::*;

#[cfg(feature = "concat")]
pub mod concat;

#[cfg(all(feature = "concat", feature = "source"))]
pub use self::concat::*;

// ----------------------------------------------------------------------------
// String Library
// ----------------------------------------------------------------------------

pub trait Concat {
    fn concat(&self, rhs: &Self) -> Self;
}

impl Concat for String {
    fn concat(&self, rhs: &Self) -> Self {
        let mut s = self.clone();
        s.push_str(rhs);
        s
    }
}

fn canonical_string_extents(value: &Value) -> MResult<Option<(usize, usize)>> {
    let schemas = value.schemas().ok_or_else(|| {
        function_shape_contract_violation("string/concat", "canonical input has no schema table")
    })?;
    let schema = schemas.entry(value.schema()).ok_or_else(|| {
        function_shape_contract_violation("string/concat", "canonical input schema is missing")
    })?;
    let SchemaBody::Matrix { dimensions, .. } = schema.schema().body() else {
        return Ok(None);
    };
    let [rows, columns] = dimensions.as_ref() else {
        return Err(function_shape_contract_violation(
            "string/concat",
            "string matrix must have rank two",
        ));
    };
    Ok(Some((
        usize::try_from(
            value
                .shape()
                .resolve_dimension(rows)
                .map_err(MechError::from)?,
        )
        .map_err(|_| {
            function_shape_contract_violation("string/concat", "row extent exceeds usize")
        })?,
        usize::try_from(
            value
                .shape()
                .resolve_dimension(columns)
                .map_err(MechError::from)?,
        )
        .map_err(|_| {
            function_shape_contract_violation("string/concat", "column extent exceeds usize")
        })?,
    )))
}

fn canonical_string_at<'a>(
    value: &'a Value,
    extents: Option<(usize, usize)>,
    row: usize,
    column: usize,
) -> MResult<&'a str> {
    match (value.data(), extents) {
        (ValueData::String(value), None) => Ok(value.as_ref()),
        (ValueData::Matrix(matrix), Some((rows, columns))) => {
            let mech_core::snapshot::SequenceView::String(values) = matrix.elements() else {
                return Err(function_shape_contract_violation(
                    "string/concat",
                    "matrix payload is not String-backed",
                ));
            };
            let row = if rows == 1 { 0 } else { row };
            let column = if columns == 1 { 0 } else { column };
            values
                .get(row.saturating_mul(columns).saturating_add(column))
                .map(|value| value.as_ref())
                .ok_or_else(|| {
                    function_shape_contract_violation(
                        "string/concat",
                        "broadcast coordinate is outside the String input",
                    )
                })
        }
        _ => Err(function_shape_contract_violation(
            "string/concat",
            "canonical String input disagrees with its schema",
        )),
    }
}

#[derive(Clone, Copy)]
struct CanonicalConcatGeometry {
    lhs: Option<(usize, usize)>,
    rhs: Option<(usize, usize)>,
    output: Option<(usize, usize)>,
}

fn canonical_concat_geometry(lhs: &Value, rhs: &Value) -> MResult<CanonicalConcatGeometry> {
    let lhs_extents = canonical_string_extents(lhs)?;
    let rhs_extents = canonical_string_extents(rhs)?;
    let output = match (lhs_extents, rhs_extents) {
        (None, None) => None,
        (Some(extents), None) | (None, Some(extents)) => Some(extents),
        (Some((left_rows, left_columns)), Some((right_rows, right_columns))) => {
            let rows = left_rows.max(right_rows);
            let columns = left_columns.max(right_columns);
            if (left_rows != 1 && left_rows != rows)
                || (right_rows != 1 && right_rows != rows)
                || (left_columns != 1 && left_columns != columns)
                || (right_columns != 1 && right_columns != columns)
            {
                return Err(function_shape_contract_violation(
                    "string/concat",
                    "String matrix broadcast dimensions are incompatible",
                ));
            }
            Some((rows, columns))
        }
    };
    Ok(CanonicalConcatGeometry {
        lhs: lhs_extents,
        rhs: rhs_extents,
        output,
    })
}

fn canonical_concat_footprint(
    lhs: &Value,
    rhs: &Value,
    output: &ValueCell,
) -> MResult<CurrentMemoryFootprint> {
    let geometry = canonical_concat_geometry(lhs, rhs)?;
    let (rows, columns, matrix) = match geometry.output {
        Some((rows, columns)) => (rows, columns, true),
        None => (1, 1, false),
    };
    let count = rows.checked_mul(columns).ok_or_else(|| {
        function_shape_contract_violation("string/concat", "output cardinality overflowed usize")
    })?;
    let mut payload_bytes = 0_u64;
    for row in 0..rows {
        for column in 0..columns {
            let left = canonical_string_at(lhs, geometry.lhs, row, column)?;
            let right = canonical_string_at(rhs, geometry.rhs, row, column)?;
            let length = left.len().checked_add(right.len()).ok_or_else(|| {
                function_shape_contract_violation(
                    "string/concat",
                    "output String length overflowed usize",
                )
            })?;
            payload_bytes = payload_bytes
                .checked_add(u64::try_from(length).map_err(|_| {
                    function_shape_contract_violation(
                        "string/concat",
                        "output String length exceeded the portable footprint domain",
                    )
                })?)
                .ok_or_else(|| {
                    function_shape_contract_violation(
                        "string/concat",
                        "output String payload footprint overflowed u64",
                    )
                })?;
        }
    }
    let logical_elements = u64::try_from(count).map_err(|_| {
        function_shape_contract_violation(
            "string/concat",
            "output cardinality exceeded the portable footprint domain",
        )
    })?;
    let shape_parameter_count = output.shape().parameter_values().len();
    let footprint = mech_core::snapshot::prospective_string_value_footprint(
        shape_parameter_count,
        matrix,
        logical_elements,
        payload_bytes,
    )
    .map_err(|_| {
        function_shape_contract_violation(
            "string/concat",
            "output canonical footprint overflowed the admitted domain",
        )
    })?;
    Ok(CurrentMemoryFootprint {
        logical_elements,
        payload_bytes: footprint.retained_bytes,
        encoded_bytes: footprint.encoded_bytes,
        retained_nodes: footprint.node_count,
        shape_parameter_count: shape_parameter_count as u64,
        ..CurrentMemoryFootprint::default()
    })
}

fn canonical_concat_value(
    lhs: &Value,
    rhs: &Value,
    output: &ValueCell,
    construction: &mut mech_core::FrozenSnapshotConstruction,
) -> MResult<Value> {
    let geometry = canonical_concat_geometry(lhs, rhs)?;
    let Some((rows, columns)) = geometry.output else {
        let next = ValueDataDraft::String(
            construction.try_concatenate_string(
                canonical_string_at(lhs, None, 0, 0)?,
                canonical_string_at(rhs, None, 0, 0)?,
            )?,
        );
        return construction.try_rebuild_data_draft(output, next);
    };
    let count = rows.checked_mul(columns).ok_or_else(|| {
        function_shape_contract_violation("string/concat", "output cardinality overflowed usize")
    })?;
    let values = construction.try_boxed_slice_with(count, |construction, index| {
        let row = index / columns;
        let column = index % columns;
        Ok(ValueDataDraft::String(
            construction.try_concatenate_string(
                canonical_string_at(lhs, geometry.lhs, row, column)?,
                canonical_string_at(rhs, geometry.rhs, row, column)?,
            )?,
        ))
    })?;
    let dimensions = construction.try_boxed_slice_with(2, |_construction, index| {
        Ok(if index == 0 {
            rows as u64
        } else {
            columns as u64
        })
    })?;
    construction.try_rebuild_matrix_drafts(output, dimensions, values)
}

#[cfg(test)]
pub(crate) fn test_managed_factory<F: MechFunctionFactory>(
    invocation: FunctionInvocation,
    operation: &'static str,
) -> SpecializedFunction {
    test_managed_factory_for_target::<F>(invocation, operation, ExecutionTarget::DirectRuntime)
}

#[cfg(test)]
pub(crate) fn test_managed_factory_for_target<F: MechFunctionFactory>(
    invocation: FunctionInvocation,
    operation: &'static str,
    target: ExecutionTarget,
) -> SpecializedFunction {
    let implementation = F::new_invocation(invocation.clone()).unwrap();
    let contract = F::declared_operation_contract().unwrap().clone();
    SpecializedFunction::syntax_directed(
        (implementation, invocation),
        ResolvedOperationDescriptor::from_name(operation, contract).unwrap(),
        RuntimeFunctionId::from_name(operation),
        target,
        F::implementation_memory_class(),
    )
    .unwrap()
}

#[cfg(all(test, feature = "source"))]
pub(crate) fn test_source_specialize(
    catalog: &FunctionCatalog,
    name: &str,
    cells: Vec<ValueCell>,
) -> SpecializedFunction {
    let entry = catalog.specializer(OperationId::from_name(name)).unwrap();
    let originals = cells
        .iter()
        .map(ValueCell::resolved_type)
        .collect::<MResult<Vec<_>>>()
        .unwrap();
    let SourceTypeAuthority::Schemes(declaration) = &entry.type_authority else {
        panic!("String source operation must have one semantic scheme authority")
    };
    let instantiated = declaration.template.map(|template| {
        FunctionTypeDeclaration::from_schemes(
            instantiate_source_scheme_template(template, &originals).unwrap(),
        )
    });
    let declaration = instantiated.as_ref().unwrap_or(declaration);
    let candidates = declaration
        .overloads
        .iter()
        .map(|overload| TypeOverloadCandidate {
            id: u64::from(overload.id),
            scheme: &overload.scheme,
        })
        .collect::<Vec<_>>();
    let resolved = resolve_type_overloads(
        TypeConstraintOrigin::new(name, None),
        &candidates,
        &originals,
        None,
    )
    .unwrap();
    let overload_id = u32::try_from(resolved.candidate_ids[0]).unwrap();
    let overload = declaration
        .overloads
        .iter()
        .find(|overload| overload.id == overload_id)
        .unwrap();
    let converted_inputs = resolved
        .conversions
        .iter()
        .map(|plan| plan.target.clone())
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let operation = entry
        .resolved_operation(converted_inputs.len(), &resolved.outputs)
        .unwrap();
    let resolved = ResolvedCall {
        operation,
        overload_id,
        original_inputs: originals.into_boxed_slice(),
        converted_inputs,
        input_conversions: resolved.conversions,
        outputs: resolved.outputs,
        output_schema_rules: overload.output_schema_rules.clone(),
    };
    let invocation = SpecializationInvocation::from_cells(cells.into_boxed_slice());
    let mut context = SpecializationContext::for_resolved_invocation(
        &invocation,
        Some(catalog),
        entry.operation.id,
        name,
        resolved,
    )
    .unwrap();
    entry
        .specializer
        .specialize_invocation(&invocation, &mut context)
        .unwrap()
}

#[macro_export]
macro_rules! impl_string_binop {
    ($struct_name:ident, $arg1_type:ty, $arg2_type:ty, $out_type:ty, $op:ident) => {
        #[derive(Debug)]
        pub(crate) struct $struct_name<T> {
            lhs: ManagedPort<T>,
            rhs: ManagedPort<T>,
            out: ManagedPort<T>,
            marker: core::marker::PhantomData<($arg1_type, $arg2_type, $out_type)>,
        }
        impl<T> MechFunctionFactory for $struct_name<T>
        where
            T: std::fmt::Debug
                + Clone
                + Sync
                + Send
                + 'static
                + FunctionRuntimeType
                + FunctionPortBacking
                + Concat,
            #[cfg(feature = "semantic-compiler")]
            T: CanonicalMatrixElementBacking + ConstElem + CompileConst,
            $arg1_type: FunctionPortBacking,
            $arg2_type: FunctionPortBacking,
            $out_type: FunctionStateBacking,
        {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
                <$out_type as FunctionRuntimeType>::REPRESENTATION,
                <$arg1_type as FunctionRuntimeType>::REPRESENTATION,
                <$arg2_type as FunctionRuntimeType>::REPRESENTATION,
            );

            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                mech_core::ImplementationMemoryClass::CanonicalFinalize
            }

            fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
                Some($crate::string_binary_full_write_contract(
                    <$out_type as FunctionRuntimeType>::REPRESENTATION,
                ))
            }

            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                let (out, lhs, rhs) = invocation.expect_binary()?;
                let _ = lhs.try_managed::<$arg1_type>()?;
                let _ = rhs.try_managed::<$arg2_type>()?;
                let _ = out.try_managed::<$out_type>()?;
                Ok(Box::new(Self {
                    lhs: lhs.try_managed_element::<T>()?,
                    rhs: rhs.try_managed_element::<T>()?,
                    out: out.try_managed_element::<T>()?,
                    marker: core::marker::PhantomData,
                }))
            }
        }
        impl<T> MechFunctionImpl for $struct_name<T>
        where
            T: std::fmt::Debug + Clone + Sync + Send + 'static + Concat,
            #[cfg(feature = "semantic-compiler")]
            T: CanonicalMatrixElementBacking,
            $out_type: FunctionStateBacking,
        {
            fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
                Some(FunctionStatePort::from_cell(self.out.cell()))
            }
            fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
                Ok(Some(vec![FunctionStatePort::from_cell(self.out.cell())]))
            }
            fn planned_output_footprints(
                &self,
            ) -> MResult<Option<Box<[CurrentMemoryFootprint]>>> {
                let lhs = self.lhs.cell().snapshot()?;
                let rhs = self.rhs.cell().snapshot()?;
                Ok(Some(
                    vec![$crate::canonical_concat_footprint(
                        &lhs,
                        &rhs,
                        self.out.cell(),
                    )?]
                    .into_boxed_slice(),
                ))
            }
            fn solve_managed(
                &self,
                frame: &mut mech_core::KernelMemoryFrame<'_>,
                _services: &mut dyn mech_core::MechExecutionServices,
            ) -> MResult<mech_core::ReactiveSolveStatus> {
                $op!();
                frame.with_admitted_canonical_binary_port_values(
                    &self.lhs,
                    &self.rhs,
                    &self.out,
                    |lhs, rhs, output| {
                        $crate::canonical_concat_footprint(lhs, rhs, output)
                    },
                    |lhs, rhs, output, construction| {
                        Ok(((), $crate::canonical_concat_value(
                            lhs,
                            rhs,
                            output,
                            construction,
                        )?))
                    },
                )?;
                Ok(mech_core::ReactiveSolveStatus::Changed)
            }
            fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
                Some($crate::string_binary_full_write_contract(
                    <$out_type as FunctionRuntimeType>::REPRESENTATION,
                ))
            }
            fn to_string(&self) -> String {
                format!("{:#?}", self)
            }
        }
        #[cfg(feature = "semantic-compiler")]
        impl<T> MechFunctionCompiler for $struct_name<T>
        where
            T: CanonicalMatrixElementBacking + ConstElem + CompileConst + FunctionRuntimeType,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!(
                    "{}<{}>",
                    stringify!($struct_name),
                    <T as FunctionRuntimeType>::REPRESENTATION
                );
                let output = compile_value_cell_register(self.out.cell(), ctx)?;
                let lhs = compile_value_cell_register(self.lhs.cell(), ctx)?;
                let rhs = compile_value_cell_register(self.rhs.cell(), ctx)?;
                ctx.emit_binop(hash_str(&name), output, lhs, rhs);
                Ok(output)
            }
        }
    };
}

#[macro_export]
macro_rules! impl_string_fxns {
    ($lib:ident) => {
        impl_fxns!($lib, T, T, impl_string_binop);
    };
}
