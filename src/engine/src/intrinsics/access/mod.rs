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
    CanonicalAccessSelector, canonical_draft, canonical_indices,
};
#[cfg(feature = "semantic-compiler")]
use crate::{
    AccessMode, AliasPolicy, BytecodeCompilerContext, CanonicalFunctionSpecializer,
    ChangeDetectionPolicy, DeliveryMode, DimensionExpr, ExternalInteraction, FunctionInstance,
    FunctionInvocation, FunctionStatePort, FunctionValueRepresentation, GenericError,
    InputPortLayout, InputPortPolicy, MechFunctionCompiler, MechFunctionImpl,
    OperationContractDeclaration, OutputConstruction, OutputPortPolicy, ReactiveNodeKind, Register,
    SchemaBody, ShapeRule, SpecializationContext, SpecializationInput, SpecializationInvocation,
    SpecializedFunction, ValueCell, ValueData, ValueDataDraft, compile_value_cell_register,
    hash_str,
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
fn canonical_access_contract(input_count: usize, shape: ShapeRule) -> OperationContractDeclaration {
    OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(
            vec![
                InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                };
                input_count
            ]
            .into_boxed_slice(),
        ),
        outputs: vec![OutputPortPolicy {
            access: AccessMode::Write,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::FullWrite { shape },
            alias: AliasPolicy::NoAlias,
            change_detection: ChangeDetectionPolicy::KernelReported,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    }
}

#[cfg(feature = "semantic-compiler")]
static PURE_CANONICAL_ACCESS_COPY_CONTRACT: std::sync::LazyLock<OperationContractDeclaration> =
    std::sync::LazyLock::new(|| canonical_access_contract(1, ShapeRule::SameAsInput { input: 0 }));
#[cfg(feature = "semantic-compiler")]
static PURE_CANONICAL_ACCESS_BINARY_CONTRACT: std::sync::LazyLock<OperationContractDeclaration> =
    std::sync::LazyLock::new(|| canonical_access_contract(2, ShapeRule::Declared));
#[cfg(feature = "semantic-compiler")]
static PURE_CANONICAL_ACCESS_TERNARY_CONTRACT: std::sync::LazyLock<OperationContractDeclaration> =
    std::sync::LazyLock::new(|| canonical_access_contract(3, ShapeRule::Declared));
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
            const SIGNATURE: RuntimeFunctionSignature =
                RuntimeFunctionSignature::nullary(FunctionValueRepresentation::AnyValue);

            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                Ok(Box::new(Self {
                    output: invocation.expect_nullary()?.value(),
                }))
            }
        }

        impl MechFunctionImpl for $factory {
            fn solve_result(&self) -> MResult<()> {
                Ok(())
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
    fn solve_result(&self) -> MResult<()> {
        let next = canonical_access_result(&self.source, &self.selectors)?;
        self.output.replace(&next.snapshot()?)
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
impl CanonicalAccess {
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
        context.emit_varop(hash_str(self.semantic_name()), output, arguments);
        Ok(output)
    }
}

#[cfg(feature = "semantic-compiler")]
fn canonical_access_result(
    source: &ValueCell,
    selectors: &[CanonicalAccessSelector],
) -> MResult<ValueCell> {
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
                return ValueCell::dynamic_matrix_from_cells(values.len(), 1, &values);
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
            let _ = element;
            ValueCell::dynamic_matrix_from_cells(
                selected_rows.len(),
                selected_columns.len(),
                &values,
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
    context.certify_instance(
        FunctionInstance::new(
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
    fn solve_result(&self) -> MResult<()> {
        self.output.replace(&self.result()?.snapshot()?)
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
    context.certify_instance(
        FunctionInstance::new(
            Box::new(CanonicalSwizzle {
                output: output.clone(),
                ..implementation
            }),
            FunctionInvocation::variadic(output, inputs),
        ),
        mech_core::RuntimeFunctionId::from_name("CanonicalSwizzle"),
        mech_core::ExecutionTarget::DirectRuntime,
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

        let selected =
            canonical_access_result(&map, &[CanonicalAccessSelector::Cell(selector)]).unwrap();
        assert!(matches!(
            selected.snapshot().unwrap().data(),
            ValueData::String(value) if value.as_ref() == "zero"
        ));
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
        assert!(access.solve_result().is_err());
        assert!(matches!(
            output.snapshot().unwrap().data(),
            ValueData::U64(7)
        ));
    }
}
