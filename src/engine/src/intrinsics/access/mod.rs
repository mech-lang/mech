// ----------------------------------------------------------------------------
// Access
// ----------------------------------------------------------------------------

#[cfg(feature = "map")]
pub mod map;
#[cfg(feature = "matrix")]
pub mod matrix;
#[cfg(feature = "record")]
pub mod record;
#[cfg(all(feature = "string", feature = "semantic-compiler"))]
pub mod string;
#[cfg(feature = "table")]
pub mod table;
#[cfg(feature = "tuple")]
pub mod tuple;

#[cfg(feature = "map")]
pub use self::map::*;
#[cfg(feature = "matrix")]
pub use self::matrix::*;
#[cfg(feature = "record")]
pub use self::record::*;
#[cfg(all(feature = "string", feature = "semantic-compiler"))]
pub use self::string::*;
#[cfg(feature = "table")]
pub use self::table::*;
#[cfg(feature = "tuple")]
pub use self::tuple::*;

#[cfg(all(
    feature = "semantic-compiler",
    any(feature = "record", feature = "table")
))]
use crate::UndefinedRecordFieldError;
#[cfg(all(feature = "semantic-compiler", feature = "table"))]
use crate::UndefinedTableColumnError;
#[cfg(feature = "semantic-compiler")]
use crate::intrinsics::canonical_access::{
    CanonicalAccessSelector, canonical_draft, canonical_fixed_matrix_axes, canonical_indices,
    canonical_matrix_result_with_fixed_axes,
};
#[cfg(feature = "semantic-compiler")]
use crate::{
    BytecodeCompilerContext, CanonicalFunctionSpecializer, CurrentMemoryFootprint, DimensionExpr,
    FunctionInvocation, FunctionMatrixElement, FunctionStatePort, FunctionValueRepresentation,
    GenericError, MechFunctionCompiler, MechFunctionImpl, OperationContractDeclaration,
    ReactiveNodeKind, Register, SchemaBody, SpecializationContext, SpecializationInput,
    SpecializationInvocation, SpecializedFunction, ValueCell, ValueData, ValueDataDraft,
    compile_value_cell_register, hash_str,
};
use crate::{FunctionCatalogBuilder, MResult};
#[cfg(all(feature = "native-plan", not(feature = "semantic-compiler")))]
use crate::{
    FunctionInvocation, FunctionValueOutput, FunctionValueRepresentation, MechFunction,
    MechFunctionImpl, ValueCell,
};
#[cfg(all(feature = "native-plan", feature = "semantic-compiler"))]
use crate::{FunctionValueOutput, MechFunction};
#[cfg(feature = "semantic-compiler")]
use crate::{IncorrectNumberOfArguments, MechError};

#[cfg(feature = "semantic-compiler")]
fn canonical_access_contract(input_count: usize) -> OperationContractDeclaration {
    mech_core::maintained_operation_contract("access/scalar", input_count, false)
        .expect("maintained selection contract")
}

#[cfg(feature = "semantic-compiler")]
static PURE_CANONICAL_ACCESS_COPY_CONTRACT: std::sync::LazyLock<OperationContractDeclaration> =
    std::sync::LazyLock::new(|| canonical_access_contract(1));
#[cfg(feature = "semantic-compiler")]
pub(crate) static PURE_CANONICAL_ACCESS_BINARY_CONTRACT: std::sync::LazyLock<
    OperationContractDeclaration,
> = std::sync::LazyLock::new(|| canonical_access_contract(2));
#[cfg(feature = "semantic-compiler")]
pub(crate) static PURE_CANONICAL_ACCESS_TERNARY_CONTRACT: std::sync::LazyLock<
    OperationContractDeclaration,
> = std::sync::LazyLock::new(|| canonical_access_contract(3));
#[cfg(feature = "native-plan")]
use crate::{
    MechFunctionFactory, RuntimeFunctionContract, RuntimeFunctionSignature,
    RuntimeOutputAliasPolicy,
};

#[cfg(feature = "native-plan")]
macro_rules! declare_structural_access_alias {
    (
        $factory:ident,
        $registration:ident,
        $installer:ident,
        $name:literal,
        $path:literal
    ) => {
        #[derive(Debug)]
        struct $factory {
            output: FunctionValueOutput,
        }

        impl MechFunctionFactory for $factory {
            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                mech_core::ImplementationMemoryClass::CanonicalFinalize
            }

            const SIGNATURE: RuntimeFunctionSignature =
                RuntimeFunctionSignature::nullary(FunctionValueRepresentation::AnyValue);

            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                Ok(Box::new(Self {
                    output: invocation.expect_nullary()?.value(),
                }))
            }
        }

        impl MechFunctionImpl for $factory {
            fn solve_managed(
                &self,
                _frame: &mut mech_core::KernelMemoryFrame<'_>,
                _services: &mut dyn mech_core::MechExecutionServices,
            ) -> MResult<mech_core::ReactiveSolveStatus> {
                (|| -> MResult<()> { Ok(()) })()?;
                Ok(mech_core::ReactiveSolveStatus::Changed)
            }

            fn reactive_output_value_cells(&self) -> Vec<ValueCell> {
                vec![self.output.cell().clone()]
            }

            fn to_string(&self) -> String {
                format!("{self:#?}")
            }
        }

        #[cfg(feature = "semantic-compiler")]
        impl MechFunctionCompiler for $factory {
            fn compiler_owned_value_cells(&self) -> Vec<ValueCell> {
                vec![self.output.cell().clone()]
            }

            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let register = self.output.compile_register(ctx)?;
                ctx.emit_nullop(hash_str($name), register);
                Ok(register)
            }
        }

        mech_core::declare_native_runtime_factory! {
            cfg: feature = "access",
            registration: $registration,
            installer: $installer,
            name: $name,
            factory_type: $factory,
            contract: RuntimeFunctionContract::same_shape(
                RuntimeOutputAliasPolicy::DisallowInputAlias,
            ),
            compiler_family: mech_core::RuntimeFamilyId::from_name($name),
            package: "mech-engine", crate_name: "mech_engine",
            installer_path: $path,
            extra_cargo_features: ["access"],
        }
    };
}

#[cfg(feature = "native-plan")]
declare_structural_access_alias!(
    RecordAccessFieldAliasFactory,
    register_record_access_field,
    install_record_access_field,
    "RecordAccessField",
    "mech_engine::__mech_native::install_record_access_field"
);
#[cfg(feature = "native-plan")]
declare_structural_access_alias!(
    RecordAccessSwizzleAliasFactory,
    register_record_access_swizzle,
    install_record_access_swizzle,
    "RecordAccessSwizzle",
    "mech_engine::__mech_native::install_record_access_swizzle"
);
#[cfg(feature = "native-plan")]
declare_structural_access_alias!(
    TableAccessSwizzleAliasFactory,
    register_table_access_swizzle,
    install_table_access_swizzle,
    "TableAccessSwizzle",
    "mech_engine::__mech_native::install_table_access_swizzle"
);

/// Installs every enabled concrete access factory into the supplied catalog.
pub(crate) fn install_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    #[cfg(feature = "matrix")]
    matrix::install_runtime(builder)?;
    #[cfg(feature = "tuple")]
    tuple::install_runtime(builder)?;
    Ok(())
}

/// Installs structural access aliases emitted by the source compiler without
/// adding them to the frozen standard runtime surface.
#[cfg(feature = "native-plan")]
pub(crate) fn install_native_plan(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    register_record_access_field(builder)?;
    register_record_access_swizzle(builder)?;
    register_table_access_swizzle(builder)?;
    Ok(())
}

pub struct AccessScalar {}

#[cfg(all(feature = "matrix", feature = "semantic-compiler"))]
fn canonical_matrix_dimensions(value: &ValueCell) -> MResult<(usize, usize)> {
    let crate::SchemaBody::Matrix { dimensions, .. } = value.closed_schema_body()? else {
        return Err(MechError::new(
            crate::GenericError {
                msg: "matrix access source does not have a matrix schema".to_owned(),
            },
            None,
        )
        .with_compiler_loc());
    };
    let [
        DimensionExpr::Constant(rows),
        DimensionExpr::Constant(columns),
    ] = dimensions.as_ref()
    else {
        unreachable!("closed matrix schemas have constant dimensions")
    };
    let rows = usize::try_from(*rows).map_err(|_| {
        MechError::new(
            crate::GenericError {
                msg: "matrix row extent exceeds the target index width".to_owned(),
            },
            None,
        )
        .with_compiler_loc()
    })?;
    let columns = usize::try_from(*columns).map_err(|_| {
        MechError::new(
            crate::GenericError {
                msg: "matrix column extent exceeds the target index width".to_owned(),
            },
            None,
        )
        .with_compiler_loc()
    })?;
    rows.checked_mul(columns).ok_or_else(|| {
        MechError::new(
            crate::GenericError {
                msg: "matrix element count exceeds the target index width".to_owned(),
            },
            None,
        )
        .with_compiler_loc()
    })?;
    Ok((rows, columns))
}

#[cfg(feature = "semantic-compiler")]
#[derive(Debug)]
struct CanonicalAccess {
    source: ValueCell,
    selectors: Vec<CanonicalAccessSelector>,
    output: ValueCell,
    name: &'static str,
}

#[cfg(feature = "semantic-compiler")]
impl MechFunctionImpl for CanonicalAccess {
    fn planned_output_footprints(&self) -> MResult<Option<Box<[CurrentMemoryFootprint]>>> {
        if !self.output_requires_canonical_builder() {
            return Ok(None);
        }
        let SchemaBody::Matrix { element, .. } = self.source.closed_schema_body()? else {
            // Selecting one member, column, or grapheme cannot retain more
            // canonical data than the already-borrowed aggregate/String that
            // contains it. This complete-source upper bound avoids building a
            // candidate or selector result before admission.
            let mut footprint = self.source.current_memory_footprint()?;
            let output_shape_parameters = self.output.shape().parameter_values().len() as u64;
            if output_shape_parameters > footprint.shape_parameter_count {
                let additional_shape_bytes = output_shape_parameters
                    .checked_sub(footprint.shape_parameter_count)
                    .and_then(|count| count.checked_mul(core::mem::size_of::<u64>() as u64))
                    .ok_or_else(|| {
                        MechError::new(
                            GenericError {
                                msg: "canonical access shape footprint exceeds u64".to_owned(),
                            },
                            None,
                        )
                        .with_compiler_loc()
                    })?;
                footprint.payload_bytes = footprint
                    .payload_bytes
                    .checked_add(additional_shape_bytes)
                    .ok_or_else(|| {
                        MechError::new(
                            GenericError {
                                msg: "canonical access retained footprint exceeds u64".to_owned(),
                            },
                            None,
                        )
                        .with_compiler_loc()
                    })?;
            }
            footprint.shape_parameter_count = output_shape_parameters;
            footprint.logical_elements = self
                .output
                .resolved_descriptor()?
                .current_extents()
                .map_err(MechError::from)?
                .iter()
                .try_fold(1_u64, |product, extent| product.checked_mul(*extent))
                .ok_or_else(|| {
                    MechError::new(
                        GenericError {
                            msg: "canonical access output cardinality exceeds u64".to_owned(),
                        },
                        None,
                    )
                    .with_compiler_loc()
                })?;
            return Ok(Some(vec![footprint].into_boxed_slice()));
        };
        let (rows, columns) = canonical_matrix_dimensions(&self.source)?;
        let selected_count = if self.selectors.len() == 1 {
            canonical_selector_cardinality_bound(
                &self.selectors[0],
                rows.checked_mul(columns).ok_or_else(|| {
                    MechError::new(
                        GenericError {
                            msg: "matrix element count exceeds the target index width".to_owned(),
                        },
                        None,
                    )
                    .with_compiler_loc()
                })?,
            )?
        } else if self.selectors.len() == 2 {
            canonical_selector_cardinality_bound(&self.selectors[0], rows)?
                .checked_mul(canonical_selector_cardinality_bound(
                    &self.selectors[1],
                    columns,
                )?)
                .ok_or_else(|| {
                    MechError::new(
                        GenericError {
                            msg: "matrix access result cardinality exceeds u64".to_owned(),
                        },
                        None,
                    )
                    .with_compiler_loc()
                })?
        } else {
            return Ok(None);
        };
        let source = self.source.snapshot()?;
        let values = source
            .matrix_view()
            .ok_or_else(|| {
                MechError::new(
                    GenericError {
                        msg: "matrix access source has no canonical matrix data".to_owned(),
                    },
                    None,
                )
                .with_compiler_loc()
            })?
            .elements();
        let footprint = self.output.prospective_repeated_sequence_memory_footprint(
            element.as_ref(),
            values,
            selected_count,
        )?;
        Ok(Some(vec![footprint].into_boxed_slice()))
    }

    fn solve_managed(
        &self,
        frame: &mut mech_core::KernelMemoryFrame<'_>,
        _services: &mut dyn mech_core::MechExecutionServices,
    ) -> MResult<mech_core::ReactiveSolveStatus> {
        if self.output_requires_canonical_builder() {
            self.stage_managed(frame)?;
        } else {
            let next = self.next_value()?;
            frame.stage_output_value(&self.output, next.snapshot()?)?;
        }
        Ok(mech_core::ReactiveSolveStatus::Changed)
    }

    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        Some(FunctionStatePort::from_cell(&self.output))
    }

    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        Ok(Some(vec![FunctionStatePort::from_cell(&self.output)]))
    }

    fn reactive_node_kind(&self) -> ReactiveNodeKind {
        ReactiveNodeKind::Combinational
    }

    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(match self.selector_cells().len() {
            0 => &PURE_CANONICAL_ACCESS_COPY_CONTRACT,
            1 => &PURE_CANONICAL_ACCESS_BINARY_CONTRACT,
            2 => &PURE_CANONICAL_ACCESS_TERNARY_CONTRACT,
            _ => unreachable!("canonical access supports at most two concrete selectors"),
        })
    }

    fn semantic_operation_name(&self) -> Option<&str> {
        Some(self.semantic_name())
    }

    fn to_string(&self) -> String {
        self.name.to_owned()
    }
}

#[cfg(feature = "semantic-compiler")]
fn canonical_selector_cardinality_bound(
    selector: &CanonicalAccessSelector,
    upper: usize,
) -> MResult<u64> {
    let overflow = || {
        MechError::new(
            GenericError {
                msg: "matrix selector cardinality exceeds u64".to_owned(),
            },
            None,
        )
        .with_compiler_loc()
    };
    match selector {
        CanonicalAccessSelector::All => u64::try_from(upper).map_err(|_| overflow()),
        CanonicalAccessSelector::Cell(cell)
            if matches!(
                cell.representation(),
                FunctionValueRepresentation::Matrix { .. }
            ) =>
        {
            cell.resolved_descriptor()?
                .current_extents()
                .map_err(MechError::from)?
                .iter()
                .try_fold(1_u64, |product, extent| product.checked_mul(*extent))
                .ok_or_else(overflow)
        }
        CanonicalAccessSelector::Cell(_) => Ok(1),
    }
}

#[cfg(feature = "semantic-compiler")]
impl CanonicalAccess {
    fn typed_matrix(
        source: ValueCell,
        selectors: Vec<CanonicalAccessSelector>,
        output: ValueCell,
    ) -> Self {
        Self {
            source,
            selectors,
            output,
            name: "matrix/access",
        }
    }

    fn output_requires_canonical_builder(&self) -> bool {
        matches!(
            self.output.representation(),
            FunctionValueRepresentation::String
                | FunctionValueRepresentation::Atom
                | FunctionValueRepresentation::Enum
                | FunctionValueRepresentation::Record
                | FunctionValueRepresentation::Map
                | FunctionValueRepresentation::Set
                | FunctionValueRepresentation::Table
                | FunctionValueRepresentation::Tuple
                | FunctionValueRepresentation::Kind
                | FunctionValueRepresentation::AnyValue
                | FunctionValueRepresentation::Matrix {
                    element: FunctionMatrixElement::String | FunctionMatrixElement::Value,
                    ..
                }
        )
    }

    fn prospective_output_footprint(&self) -> MResult<CurrentMemoryFootprint> {
        let footprints = self.planned_output_footprints()?.ok_or_else(|| {
            MechError::new(
                GenericError {
                    msg: format!(
                        "{} has no prospective canonical output footprint",
                        self.semantic_name()
                    ),
                },
                None,
            )
            .with_compiler_loc()
        })?;
        footprints.first().copied().ok_or_else(|| {
            MechError::new(
                GenericError {
                    msg: format!("{} has no output footprint", self.semantic_name()),
                },
                None,
            )
            .with_compiler_loc()
        })
    }

    fn stage_managed(&self, frame: &mut mech_core::KernelMemoryFrame<'_>) -> MResult<()> {
        let footprint = self.prospective_output_footprint()?;
        frame.with_admitted_canonical_output(&self.output, footprint, |_, construction| {
            let next = construction.try_rebind_snapshot_candidate_with(&self.output, || {
                self.next_value()?.snapshot()
            })?;
            Ok(((), next))
        })?;
        Ok(())
    }

    fn next_value(&self) -> MResult<ValueCell> {
        let next = canonical_access_result(&self.source, &self.selectors)?;
        let expected = self.output.closed_schema_body()?;
        let found = next.closed_schema_body()?;
        if found != expected {
            return Err(MechError::new(
                GenericError {
                    msg: format!(
                        "reactive aggregate selector resolved output schema {found:?}, expected {expected:?}"
                    ),
                },
                None,
            )
            .with_compiler_loc());
        }
        Ok(next)
    }

    fn selector_cells(&self) -> Vec<ValueCell> {
        self.selectors
            .iter()
            .filter_map(|selector| match selector {
                CanonicalAccessSelector::Cell(cell) => Some(cell.clone()),
                CanonicalAccessSelector::All => None,
            })
            .collect()
    }

    fn semantic_name(&self) -> &'static str {
        match self.selectors.as_slice() {
            [
                CanonicalAccessSelector::Cell(_),
                CanonicalAccessSelector::All,
            ] => "access/rows",
            [
                CanonicalAccessSelector::All,
                CanonicalAccessSelector::Cell(_),
            ] => "access/columns",
            _ if self.selector_cells().is_empty() => "core/assign",
            _ if matches!(
                self.output.representation(),
                FunctionValueRepresentation::Matrix { .. }
            ) =>
            {
                "access/range"
            }
            _ => "access/scalar",
        }
    }
}

#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for CanonicalAccess {
    fn compiler_owned_value_cells(&self) -> Vec<ValueCell> {
        let mut cells = vec![self.output.clone(), self.source.clone()];
        cells.extend(self.selector_cells());
        cells
    }

    fn compile(&self, context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let output = compile_value_cell_register(&self.output, context)?;
        let source = compile_value_cell_register(&self.source, context)?;
        let mut arguments = vec![source];
        arguments.extend(
            self.selector_cells()
                .iter()
                .map(|selector| compile_value_cell_register(selector, context))
                .collect::<MResult<Vec<_>>>()?,
        );
        context.emit_varop(hash_str(self.name), output, arguments);
        Ok(output)
    }
}

#[cfg(feature = "semantic-compiler")]
fn canonical_access_result(
    source: &ValueCell,
    selectors: &[CanonicalAccessSelector],
) -> MResult<ValueCell> {
    if selectors.len() == 2
        && selectors
            .iter()
            .all(|selector| matches!(selector, CanonicalAccessSelector::All))
    {
        // Whole-value assignment preserves the source's canonical semantic
        // schema and current shape. Reconstructing it through a DMatrix would
        // replace fixed source dimensions with turn-lifetime physical ones.
        return source.detached_clone();
    }
    match source.closed_schema_body()? {
        SchemaBody::Tuple(_) if selectors.len() == 1 => {
            let values = source
                .tuple_elements()?
                .expect("tuple schema retains tuple values");
            let index = canonical_indices(&selectors[0], values.len())?[0];
            values[index].detached_clone()
        }
        SchemaBody::Record(fields) if selectors.len() == 1 => {
            let CanonicalAccessSelector::Cell(selector) = &selectors[0] else {
                return Err(MechError::new(
                    GenericError {
                        msg: "record fields require an id selector".to_owned(),
                    },
                    None,
                )
                .with_compiler_loc());
            };
            let ValueData::Id(field_id) = selector.snapshot()?.data().clone() else {
                return Err(MechError::new(
                    GenericError {
                        msg: "record fields require an id selector".to_owned(),
                    },
                    None,
                )
                .with_compiler_loc());
            };
            let index = fields
                .iter()
                .position(|field| hash_str(&field.name) == field_id)
                .ok_or_else(|| {
                    MechError::new(UndefinedRecordFieldError { id: field_id }, None)
                        .with_compiler_loc()
                })?;
            let ValueDataDraft::Record(values) = canonical_draft(source)? else {
                unreachable!()
            };
            ValueCell::from_schema_data(fields[index].schema.clone(), values[index].value.clone())
        }
        SchemaBody::Map { key, value, .. } if selectors.len() == 1 => {
            let CanonicalAccessSelector::Cell(selector) = &selectors[0] else {
                return Err(MechError::new(
                    GenericError {
                        msg: "map access requires a canonical key".to_owned(),
                    },
                    None,
                )
                .with_compiler_loc());
            };
            let ValueDataDraft::Map(entries) = canonical_draft(source)? else {
                unreachable!()
            };
            for entry in entries {
                let [key_draft, value_draft] = entry.items.into_vec().try_into().map_err(|_| {
                    MechError::new(
                        GenericError {
                            msg: "canonical map entry does not contain a key and value".to_owned(),
                        },
                        None,
                    )
                    .with_compiler_loc()
                })?;
                let candidate = ValueCell::from_schema_data((*key).clone(), key_draft)?;
                if candidate.key_eq(selector)? {
                    return ValueCell::from_schema_data((*value).clone(), value_draft);
                }
            }
            Err(MechError::new(
                GenericError {
                    msg: "canonical map key is not present".to_owned(),
                },
                None,
            )
            .with_compiler_loc())
        }
        SchemaBody::Table { columns, .. } if selectors.len() == 1 => {
            let CanonicalAccessSelector::Cell(selector) = &selectors[0] else {
                return Err(MechError::new(
                    GenericError {
                        msg: "table columns require an id selector".to_owned(),
                    },
                    None,
                )
                .with_compiler_loc());
            };
            let ValueData::Id(column_id) = selector.snapshot()?.data().clone() else {
                return Err(MechError::new(
                    GenericError {
                        msg: "table columns require an id selector".to_owned(),
                    },
                    None,
                )
                .with_compiler_loc());
            };
            let index = columns
                .iter()
                .position(|column| hash_str(&column.name) == column_id)
                .ok_or_else(|| {
                    MechError::new(UndefinedTableColumnError { id: column_id }, None)
                        .with_compiler_loc()
                })?;
            let ValueDataDraft::Table(values) = canonical_draft(source)? else {
                unreachable!()
            };
            let values = values[index].values.clone();
            ValueCell::dynamic_matrix(
                columns[index].schema.clone(),
                vec![values.len() as u64, 1].into_boxed_slice(),
                values,
            )
        }
        SchemaBody::String if selectors.len() == 1 => {
            let index = canonical_indices(&selectors[0], usize::MAX)?[0];
            let ValueData::String(value) = source.snapshot()?.data().clone() else {
                unreachable!()
            };
            let grapheme = grapheme::Graphemes::from_usvs(&value)
                .iter()
                .nth(index)
                .map(|value| value.as_str().to_owned())
                .ok_or_else(|| {
                    MechError::new(crate::intrinsics::IndexOutOfBoundsError, None)
                        .with_compiler_loc()
                })?;
            ValueCell::from_exact(grapheme)
        }
        SchemaBody::Matrix { element, .. } if (1..=2).contains(&selectors.len()) => {
            let (rows, columns) = canonical_matrix_dimensions(source)?;
            let elements = source
                .matrix_elements()?
                .expect("matrix schema retains matrix values");
            if selectors.len() == 1 {
                let element_count = rows.checked_mul(columns).ok_or_else(|| {
                    MechError::new(
                        crate::GenericError {
                            msg: "matrix element count exceeds the target index width".to_owned(),
                        },
                        None,
                    )
                    .with_compiler_loc()
                })?;
                let selected = canonical_indices(&selectors[0], element_count)?;
                let values = selected
                    .iter()
                    .map(|linear| {
                        let row = linear % rows;
                        let column = linear / rows;
                        elements[row * columns + column].clone()
                    })
                    .collect::<Vec<_>>();
                if selectors[0].is_scalar() {
                    return values[0].detached_clone();
                }
                return canonical_matrix_access_result(
                    source,
                    element.as_ref().clone(),
                    values.len(),
                    1,
                    &values,
                    selectors,
                );
            }
            let selected_rows = canonical_indices(&selectors[0], rows)?;
            let selected_columns = canonical_indices(&selectors[1], columns)?;
            if selectors[0].is_scalar() && selectors[1].is_scalar() {
                return elements[selected_rows[0] * columns + selected_columns[0]].detached_clone();
            }
            let values = selected_rows
                .iter()
                .flat_map(|row| {
                    selected_columns
                        .iter()
                        .map(|column| elements[*row * columns + *column].clone())
                })
                .collect::<Vec<_>>();
            canonical_matrix_access_result(
                source,
                element.as_ref().clone(),
                selected_rows.len(),
                selected_columns.len(),
                &values,
                selectors,
            )
        }
        schema => Err(MechError::new(
            GenericError {
                msg: format!("canonical access is not implemented for schema {schema:?}"),
            },
            None,
        )
        .with_compiler_loc()),
    }
}

#[cfg(feature = "semantic-compiler")]
fn canonical_matrix_access_result(
    source: &ValueCell,
    element: SchemaBody,
    rows: usize,
    columns: usize,
    values: &[ValueCell],
    selectors: &[CanonicalAccessSelector],
) -> MResult<ValueCell> {
    let source_axes = canonical_fixed_matrix_axes(source)?;
    let fixed_selection = |selector: &CanonicalAccessSelector, all_fixed: bool| -> MResult<bool> {
        match selector {
            CanonicalAccessSelector::All => Ok(all_fixed),
            CanonicalAccessSelector::Cell(cell) => {
                if matches!(cell.closed_schema_body()?, SchemaBody::Matrix { element, .. }
                    if element.as_ref() == &SchemaBody::Bool)
                {
                    // Even a fixed-size mask can select a different count on
                    // its next value update, including an empty selection.
                    return Ok(false);
                }
                Ok(canonical_fixed_matrix_axes(cell)?
                    .into_iter()
                    .all(|fixed| fixed))
            }
        }
    };
    let fixed_axes = if selectors.len() == 1 {
        [
            fixed_selection(&selectors[0], source_axes.iter().all(|fixed| *fixed))?,
            true,
        ]
    } else {
        [
            fixed_selection(&selectors[0], source_axes[0])?,
            fixed_selection(&selectors[1], source_axes[1])?,
        ]
    };
    canonical_matrix_result_with_fixed_axes(
        source,
        element,
        rows,
        columns,
        fixed_axes,
        values
            .iter()
            .map(canonical_draft)
            .collect::<MResult<Vec<_>>>()?
            .into_boxed_slice(),
    )
}

#[cfg(feature = "semantic-compiler")]
fn canonical_access(
    invocation: &SpecializationInvocation,
    context: &SpecializationContext<'_>,
    fallback_name: &'static str,
) -> MResult<SpecializedFunction> {
    if !(2..=3).contains(&invocation.len()) {
        return Err(MechError::new(
            IncorrectNumberOfArguments {
                expected: 2,
                found: invocation.len(),
            },
            None,
        )
        .with_compiler_loc());
    }
    let source = invocation
        .input(0)
        .expect("validated access source")
        .cell()?
        .clone();
    let selectors = invocation.inputs()[1..]
        .iter()
        .map(CanonicalAccessSelector::from_input)
        .collect::<MResult<Vec<_>>>()?;
    let name = match source.closed_schema_body()? {
        SchemaBody::Tuple(_) => "TupleAccessElement",
        SchemaBody::Record(_) => "RecordAccessField",
        SchemaBody::Map { .. } => "MapAccessField",
        SchemaBody::Table { .. } => "TableAccessColumn",
        SchemaBody::String => "StringAccessScalar",
        SchemaBody::Matrix { .. } => "MatrixAccessCanonical",
        _ => fallback_name,
    };
    let output = canonical_access_result(&source, &selectors)?;
    let inputs = std::iter::once(source.clone())
        .chain(selectors.iter().filter_map(|selector| match selector {
            CanonicalAccessSelector::Cell(cell) => Some(cell.clone()),
            CanonicalAccessSelector::All => None,
        }))
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let contract = match inputs.len() {
        1 => &*PURE_CANONICAL_ACCESS_COPY_CONTRACT,
        2 => &*PURE_CANONICAL_ACCESS_BINARY_CONTRACT,
        3 => &*PURE_CANONICAL_ACCESS_TERNARY_CONTRACT,
        _ => unreachable!("canonical access supports at most two concrete selectors"),
    };
    context.resolve_syntax_operation_contract(contract)?;
    context.certify_instance(
        (
            Box::new(CanonicalAccess {
                source,
                selectors,
                output: output.clone(),
                name,
            }),
            FunctionInvocation::variadic(output, inputs),
        ),
        mech_core::RuntimeFunctionId::from_name(name),
        mech_core::ExecutionTarget::DirectRuntime,
        mech_core::ImplementationMemoryClass::CanonicalFinalize,
    )
}

#[cfg(feature = "semantic-compiler")]
#[derive(Debug)]
struct CanonicalSwizzle {
    source: ValueCell,
    selectors: Vec<CanonicalAccessSelector>,
    output: ValueCell,
}

#[cfg(feature = "semantic-compiler")]
impl CanonicalSwizzle {
    fn result(&self) -> MResult<ValueCell> {
        let values = self
            .selectors
            .iter()
            .map(|selector| canonical_access_result(&self.source, std::slice::from_ref(selector)))
            .collect::<MResult<Vec<_>>>()?;
        ValueCell::tuple_from_cells(&values)
    }
}

#[cfg(feature = "semantic-compiler")]
impl MechFunctionImpl for CanonicalSwizzle {
    fn solve_managed(
        &self,
        frame: &mut mech_core::KernelMemoryFrame<'_>,
        _services: &mut dyn mech_core::MechExecutionServices,
    ) -> MResult<mech_core::ReactiveSolveStatus> {
        let result = self.result()?;
        frame.stage_output_value(&self.output, result.snapshot()?)?;
        Ok(mech_core::ReactiveSolveStatus::Changed)
    }

    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        Some(FunctionStatePort::from_cell(&self.output))
    }

    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        Ok(Some(vec![FunctionStatePort::from_cell(&self.output)]))
    }

    fn reactive_node_kind(&self) -> ReactiveNodeKind {
        ReactiveNodeKind::Combinational
    }

    fn to_string(&self) -> String {
        "CanonicalSwizzle".to_owned()
    }
}

#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for CanonicalSwizzle {
    fn compiler_owned_value_cells(&self) -> Vec<ValueCell> {
        vec![self.output.clone(), self.source.clone()]
    }

    fn compile(&self, _: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        Err(MechError::new(
            GenericError {
                msg: "canonical swizzle is not bytecode-compilable yet".to_owned(),
            },
            None,
        )
        .with_compiler_loc())
    }
}

#[cfg(feature = "semantic-compiler")]
fn canonical_swizzle(
    invocation: &SpecializationInvocation,
    context: &SpecializationContext<'_>,
) -> MResult<SpecializedFunction> {
    if invocation.len() < 2 {
        return Err(MechError::new(
            IncorrectNumberOfArguments {
                expected: 2,
                found: invocation.len(),
            },
            None,
        )
        .with_compiler_loc());
    }
    let source = invocation
        .input(0)
        .expect("validated swizzle source")
        .cell()?
        .clone();
    let selectors = invocation.inputs()[1..]
        .iter()
        .map(CanonicalAccessSelector::from_input)
        .collect::<MResult<Vec<_>>>()?;
    let implementation = CanonicalSwizzle {
        source: source.clone(),
        selectors,
        output: ValueCell::unit(),
    };
    let output = implementation.result()?;
    let inputs = invocation
        .inputs()
        .iter()
        .map(SpecializationInput::cell)
        .collect::<MResult<Vec<_>>>()?
        .into_iter()
        .cloned()
        .collect::<Vec<_>>()
        .into_boxed_slice();
    context.resolve_syntax_operation_contract(&PURE_CANONICAL_ACCESS_BINARY_CONTRACT)?;
    context.certify_instance(
        (
            Box::new(CanonicalSwizzle {
                output: output.clone(),
                ..implementation
            }),
            FunctionInvocation::variadic(output, inputs),
        ),
        mech_core::RuntimeFunctionId::from_name("CanonicalSwizzle"),
        mech_core::ExecutionTarget::DirectRuntime,
        mech_core::ImplementationMemoryClass::CanonicalFinalize,
    )
}

#[cfg(all(feature = "matrix", feature = "semantic-compiler"))]
fn canonical_matrix_access(
    invocation: &SpecializationInvocation,
    _context: &mut SpecializationContext<'_>,
) -> MResult<SpecializedFunction> {
    canonical_access(invocation, _context, "MatrixAccessCanonical")
}

#[cfg(feature = "semantic-compiler")]
impl CanonicalFunctionSpecializer for AccessScalar {
    fn specialize_invocation(
        &self,
        invocation: &SpecializationInvocation,
        context: &mut SpecializationContext<'_>,
    ) -> MResult<SpecializedFunction> {
        #[cfg(feature = "matrix")]
        if invocation.input(0).is_some_and(|input| {
            matches!(
                input.representation(),
                Some(FunctionValueRepresentation::Matrix { .. })
            )
        }) {
            return canonical_matrix_access(invocation, context);
        }
        canonical_access(invocation, context, "CanonicalScalarAccess")
    }
}
pub struct AccessRange {}
#[cfg(feature = "semantic-compiler")]
impl CanonicalFunctionSpecializer for AccessRange {
    fn specialize_invocation(
        &self,
        invocation: &SpecializationInvocation,
        context: &mut SpecializationContext<'_>,
    ) -> MResult<SpecializedFunction> {
        #[cfg(feature = "matrix")]
        if invocation.input(0).is_some_and(|input| {
            matches!(
                input.representation(),
                Some(FunctionValueRepresentation::Matrix { .. })
            )
        }) {
            return canonical_matrix_access(invocation, context);
        }
        canonical_access(invocation, context, "CanonicalRangeAccess")
    }
}
pub struct AccessSwizzle {}
#[cfg(feature = "semantic-compiler")]
impl CanonicalFunctionSpecializer for AccessSwizzle {
    fn specialize_invocation(
        &self,
        invocation: &SpecializationInvocation,
        context: &mut SpecializationContext<'_>,
    ) -> MResult<SpecializedFunction> {
        canonical_swizzle(invocation, context)
    }
}

// ----------------------------------------------------------------------------

// Access Column

pub struct AccessColumn {}
#[cfg(feature = "semantic-compiler")]
impl CanonicalFunctionSpecializer for AccessColumn {
    fn specialize_invocation(
        &self,
        invocation: &SpecializationInvocation,
        context: &mut SpecializationContext<'_>,
    ) -> MResult<SpecializedFunction> {
        canonical_access(invocation, context, "CanonicalColumnAccess")
    }
}

#[cfg(all(test, feature = "semantic-compiler"))]
mod canonical_aggregate_access_tests {
    use super::*;

    fn managed_scalar_access(source: ValueCell, selector: ValueCell) -> SpecializedFunction {
        let output =
            canonical_access_result(&source, &[CanonicalAccessSelector::Cell(selector.clone())])
                .unwrap();
        let invocation =
            FunctionInvocation::binary(output.clone(), source.clone(), selector.clone());
        crate::test_support::managed_implementation_instance(
            Box::new(CanonicalAccess {
                source,
                selectors: vec![CanonicalAccessSelector::Cell(selector)],
                output,
                name: "CanonicalScalarAccess",
            }),
            invocation,
            "test/canonical-scalar-access",
            PURE_CANONICAL_ACCESS_BINARY_CONTRACT.clone(),
            mech_core::ImplementationMemoryClass::CanonicalFinalize,
        )
        .unwrap()
    }

    fn assert_string(value: &ValueCell, expected: &str) {
        assert!(matches!(
            value.snapshot().unwrap().data(),
            ValueData::String(value) if value.as_ref() == expected
        ));
    }

    #[test]
    fn map_access_uses_canonical_key_equality() {
        let map = ValueCell::from_schema_data(
            SchemaBody::Map {
                key: Box::new(SchemaBody::FloatingPoint(mech_core::FloatWidth::W64)),
                value: Box::new(SchemaBody::String),
                cardinality: mech_core::CardinalitySpec::Dynamic { upper_bound: None },
            },
            ValueDataDraft::Map(
                vec![mech_core::snapshot::MapEntryDraft {
                    items: vec![
                        ValueDataDraft::F64(mech_core::snapshot::F64Bits::from_f64(-0.0)),
                        ValueDataDraft::String("zero".to_owned()),
                    ]
                    .into_boxed_slice(),
                }]
                .into_boxed_slice(),
            ),
        )
        .unwrap();
        let selector = ValueCell::from_schema_data(
            SchemaBody::FloatingPoint(mech_core::FloatWidth::W64),
            ValueDataDraft::F64(mech_core::snapshot::F64Bits::from_f64(0.0)),
        )
        .unwrap();

        let function = managed_scalar_access(map, selector);
        function.instance().solve_result().unwrap();
        assert_string(function.instance().output(), "zero");
    }

    #[test]
    fn tuple_and_record_payload_access_execute_through_managed_admission() {
        let tuple = ValueCell::tuple_from_cells(&[
            ValueCell::from_exact(7_u64).unwrap(),
            ValueCell::from_exact("tuple payload".to_owned()).unwrap(),
        ])
        .unwrap();
        let tuple_access = managed_scalar_access(tuple, ValueCell::from_exact(2_usize).unwrap());
        tuple_access.instance().solve_result().unwrap();
        assert_string(tuple_access.instance().output(), "tuple payload");

        let record = ValueCell::record_from_cells(&[
            ("number".to_owned(), ValueCell::from_exact(7_u64).unwrap()),
            (
                "text".to_owned(),
                ValueCell::from_exact("record payload".to_owned()).unwrap(),
            ),
        ])
        .unwrap();
        let selector =
            ValueCell::from_schema_data(SchemaBody::Id, ValueDataDraft::Id(hash_str("text")))
                .unwrap();
        let record_access = managed_scalar_access(record, selector);
        record_access.instance().solve_result().unwrap();
        assert_string(record_access.instance().output(), "record payload");

        let table = ValueCell::table_from_cell_columns(
            vec![(
                mech_core::SchemaField {
                    name: "text".to_owned(),
                    schema: SchemaBody::String,
                },
                vec![
                    ValueCell::from_exact("first row".to_owned()).unwrap(),
                    ValueCell::from_exact("second row".to_owned()).unwrap(),
                ]
                .into_boxed_slice(),
            )]
            .into_boxed_slice(),
            mech_core::CardinalitySpec::Exact(DimensionExpr::Constant(2)),
        )
        .unwrap();
        let selector =
            ValueCell::from_schema_data(SchemaBody::Id, ValueDataDraft::Id(hash_str("text")))
                .unwrap();
        let table_access = managed_scalar_access(table, selector);
        table_access.instance().solve_result().unwrap();
        let value = table_access.instance().output().snapshot().unwrap();
        let mech_core::snapshot::SequenceView::String(values) =
            value.matrix_view().unwrap().elements()
        else {
            panic!("expected String table column")
        };
        assert_eq!(
            values
                .iter()
                .map(|value| value.as_ref())
                .collect::<Vec<_>>(),
            ["first row", "second row"]
        );
    }

    #[test]
    fn reactive_record_selector_schema_change_rejects_without_output_mutation() {
        let record = ValueCell::from_schema_data(
            SchemaBody::Record(
                vec![
                    mech_core::SchemaField {
                        name: "number".to_owned(),
                        schema: SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64),
                    },
                    mech_core::SchemaField {
                        name: "text".to_owned(),
                        schema: SchemaBody::String,
                    },
                ]
                .into_boxed_slice(),
            ),
            ValueDataDraft::Record(
                vec![
                    mech_core::snapshot::NamedValueDraft {
                        name: "number".to_owned(),
                        value: ValueDataDraft::U64(7),
                    },
                    mech_core::snapshot::NamedValueDraft {
                        name: "text".to_owned(),
                        value: ValueDataDraft::String("seven".to_owned()),
                    },
                ]
                .into_boxed_slice(),
            ),
        )
        .unwrap();
        let selector =
            ValueCell::from_schema_data(SchemaBody::Id, ValueDataDraft::Id(hash_str("number")))
                .unwrap();
        let output =
            canonical_access_result(&record, &[CanonicalAccessSelector::Cell(selector.clone())])
                .unwrap();
        let access = CanonicalAccess {
            source: record,
            selectors: vec![CanonicalAccessSelector::Cell(selector.clone())],
            output: output.clone(),
            name: "RecordAccessField",
        };

        selector
            .replace(
                &ValueCell::from_schema_data(SchemaBody::Id, ValueDataDraft::Id(hash_str("text")))
                    .unwrap()
                    .snapshot()
                    .unwrap(),
            )
            .unwrap();
        assert!(access.next_value().is_err());
        assert!(matches!(
            output.snapshot().unwrap().data(),
            ValueData::U64(7)
        ));
    }
}
