#[cfg(feature = "semantic-compiler")]
use crate::intrinsics::canonical_access::{
    CanonicalAccessSelector, canonical_draft, canonical_indices,
};
use crate::intrinsics::*;

pub mod catalog;
pub use self::catalog::install_runtime;

#[cfg(feature = "map")]
pub mod map;
#[cfg(feature = "matrix")]
pub mod matrix;
#[cfg(feature = "record")]
pub mod record;
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
#[cfg(feature = "table")]
pub use self::table::*;
#[cfg(feature = "tuple")]
pub use self::tuple::*;

// ----------------------------------------------------------------------------
// Assign
// ----------------------------------------------------------------------------

// x = 1 ----------------------------------------------------------------------

static PURE_STATE_REGISTER_CONTRACT: std::sync::LazyLock<OperationContractDeclaration> =
    std::sync::LazyLock::new(|| OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(
            vec![InputPortPolicy {
                access: AccessMode::Read,
                delivery: DeliveryMode::Signal,
            }]
            .into_boxed_slice(),
        ),
        outputs: vec![OutputPortPolicy {
            access: AccessMode::Write,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::FullWrite {
                shape: ShapeRule::SameAsInput { input: 0 },
            },
            alias: AliasPolicy::NoAlias,
            change_detection: ChangeDetectionPolicy::KernelReported,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    });

#[cfg(feature = "semantic-compiler")]
fn indexed_state_register_contract(
    input_count: usize,
    regions: RegionPolicy,
) -> OperationContractDeclaration {
    OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(
            (0..input_count)
                .map(|_| InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        ),
        outputs: vec![OutputPortPolicy {
            access: AccessMode::ReadWrite,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::ReadModifyWrite {
                base_input: 0,
                regions,
            },
            alias: AliasPolicy::MayAlias { input: 0 },
            change_detection: ChangeDetectionPolicy::KernelReported,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    }
}

#[cfg(feature = "semantic-compiler")]
static PURE_INDEXED_STATE_REGISTER_CONTRACT: std::sync::LazyLock<OperationContractDeclaration> =
    std::sync::LazyLock::new(|| {
        indexed_state_register_contract(3, RegionPolicy::IndexedAxis { axis: 0 })
    });

#[cfg(feature = "semantic-compiler")]
static PURE_ROW_INDEXED_STATE_REGISTER_CONTRACT: std::sync::LazyLock<OperationContractDeclaration> =
    std::sync::LazyLock::new(|| {
        indexed_state_register_contract(3, RegionPolicy::IndexedAxis { axis: 0 })
    });

#[cfg(feature = "semantic-compiler")]
static PURE_COLUMN_INDEXED_STATE_REGISTER_CONTRACT: std::sync::LazyLock<
    OperationContractDeclaration,
> = std::sync::LazyLock::new(|| {
    indexed_state_register_contract(3, RegionPolicy::IndexedAxis { axis: 1 })
});

#[cfg(feature = "semantic-compiler")]
static PURE_RECTANGULAR_STATE_REGISTER_CONTRACT: std::sync::LazyLock<OperationContractDeclaration> =
    std::sync::LazyLock::new(|| indexed_state_register_contract(4, RegionPolicy::RectangularRegion));

#[cfg(feature = "semantic-compiler")]
static PURE_WHOLE_VALUE_STATE_REGISTER_CONTRACT: std::sync::LazyLock<OperationContractDeclaration> =
    std::sync::LazyLock::new(|| indexed_state_register_contract(2, RegionPolicy::WholeValue));

#[cfg(feature = "semantic-compiler")]
static PURE_COLLECTION_ENTRY_STATE_REGISTER_CONTRACT: std::sync::LazyLock<
    OperationContractDeclaration,
> = std::sync::LazyLock::new(|| indexed_state_register_contract(3, RegionPolicy::CollectionEntry));

#[cfg(feature = "semantic-compiler")]
static PURE_SINGLE_ELEMENT_STATE_REGISTER_CONTRACT: std::sync::LazyLock<
    OperationContractDeclaration,
> = std::sync::LazyLock::new(|| indexed_state_register_contract(3, RegionPolicy::SingleElement));

#[cfg(all(feature = "resident-artifact", feature = "semantic-compiler"))]
pub(crate) fn install_frozen_ekf_state_runtime(
    builder: &mut FunctionCatalogBuilder,
) -> MResult<()> {
    builder.insert_runtime_factory_with_semantic_contract::<Assign<DVector<f64>>>(
        "Assign<f64DVector>",
        RuntimeFunctionContract::same_shape(RuntimeOutputAliasPolicy::AllowInputAlias),
        mech_core::OperationId::from_name("core/assign"),
        &PURE_STATE_REGISTER_CONTRACT,
    )?;
    builder.insert_runtime_factory_with_semantic_contract::<Assign<DMatrix<f64>>>(
        "Assign<f64DMatrix>",
        RuntimeFunctionContract::same_shape(RuntimeOutputAliasPolicy::AllowInputAlias),
        mech_core::OperationId::from_name("core/assign"),
        &PURE_STATE_REGISTER_CONTRACT,
    )?;
    #[cfg(feature = "vector3")]
    builder.insert_runtime_factory_with_semantic_contract::<Assign<Vector3<f64>>>(
        "Assign<f64Vector3>",
        RuntimeFunctionContract::same_shape(RuntimeOutputAliasPolicy::AllowInputAlias),
        mech_core::OperationId::from_name("core/assign"),
        &PURE_STATE_REGISTER_CONTRACT,
    )?;
    #[cfg(feature = "matrix3")]
    builder.insert_runtime_factory_with_semantic_contract::<Assign<Matrix3<f64>>>(
        "Assign<f64Matrix3>",
        RuntimeFunctionContract::same_shape(RuntimeOutputAliasPolicy::AllowInputAlias),
        mech_core::OperationId::from_name("core/assign"),
        &PURE_STATE_REGISTER_CONTRACT,
    )?;
    Ok(())
}

trait AssignRuntimeName {
    #[cfg(feature = "semantic-compiler")]
    fn assign_runtime_name() -> String;
}

macro_rules! impl_scalar_assign_runtime_name {
    ($type:ty, $name:literal, $feature:literal) => {
        #[cfg(feature = $feature)]
        impl AssignRuntimeName for $type {
            #[cfg(feature = "semantic-compiler")]
            fn assign_runtime_name() -> String {
                concat!("Assign<", $name, ">").to_string()
            }
        }
    };
}

impl_scalar_assign_runtime_name!(u8, "u8", "u8");
impl_scalar_assign_runtime_name!(u16, "u16", "u16");
impl_scalar_assign_runtime_name!(u32, "u32", "u32");
impl_scalar_assign_runtime_name!(u64, "u64", "u64");
impl_scalar_assign_runtime_name!(u128, "u128", "u128");
impl_scalar_assign_runtime_name!(i8, "i8", "i8");
impl_scalar_assign_runtime_name!(i16, "i16", "i16");
impl_scalar_assign_runtime_name!(i32, "i32", "i32");
impl_scalar_assign_runtime_name!(i64, "i64", "i64");
impl_scalar_assign_runtime_name!(i128, "i128", "i128");
impl_scalar_assign_runtime_name!(f32, "f32", "f32");
impl_scalar_assign_runtime_name!(f64, "f64", "f64");
impl_scalar_assign_runtime_name!(bool, "bool", "bool");
impl_scalar_assign_runtime_name!(String, "string", "string");
impl_scalar_assign_runtime_name!(R64, "r64", "r64");
impl_scalar_assign_runtime_name!(C64, "c64", "c64");

impl AssignRuntimeName for usize {
    #[cfg(feature = "semantic-compiler")]
    fn assign_runtime_name() -> String {
        "Assign<index>".to_string()
    }
}

macro_rules! impl_matrix_assign_runtime_name {
    ($shape:ident, $feature:literal) => {
        #[cfg(feature = $feature)]
        impl<T> AssignRuntimeName for $shape<T>
        where
            T: FunctionRuntimeType,
        {
            #[cfg(feature = "semantic-compiler")]
            fn assign_runtime_name() -> String {
                format!(
                    "Assign<{}{}>",
                    <T as FunctionRuntimeType>::REPRESENTATION,
                    stringify!($shape)
                )
            }
        }
    };
}

impl_matrix_assign_runtime_name!(Matrix1, "matrix1");
impl_matrix_assign_runtime_name!(Matrix2, "matrix2");
impl_matrix_assign_runtime_name!(Matrix2x3, "matrix2x3");
impl_matrix_assign_runtime_name!(Matrix3x2, "matrix3x2");
impl_matrix_assign_runtime_name!(Matrix3, "matrix3");
impl_matrix_assign_runtime_name!(Matrix4, "matrix4");
impl_matrix_assign_runtime_name!(DMatrix, "matrixd");
impl_matrix_assign_runtime_name!(Vector2, "vector2");
impl_matrix_assign_runtime_name!(Vector3, "vector3");
impl_matrix_assign_runtime_name!(Vector4, "vector4");
impl_matrix_assign_runtime_name!(DVector, "vectord");
impl_matrix_assign_runtime_name!(RowVector2, "row_vector2");
impl_matrix_assign_runtime_name!(RowVector3, "row_vector3");
impl_matrix_assign_runtime_name!(RowVector4, "row_vector4");
impl_matrix_assign_runtime_name!(RowDVector, "row_vectord");

#[derive(Debug)]
struct Assign<T> {
    sink: Ref<T>,
    source: Ref<T>,
}

impl<T> MechFunctionFactory for Assign<T>
where
    T: Clone + Debug + Sync + Send + 'static,
    #[cfg(feature = "semantic-compiler")]
    T: ConstElem + FunctionRuntimeType,
    #[cfg(feature = "semantic-compiler")]
    T: CompileConst,
    T: FunctionStateBacking,
    T: AssignRuntimeName,
{
    const SIGNATURE: RuntimeFunctionSignature =
        RuntimeFunctionSignature::unary(T::REPRESENTATION, T::REPRESENTATION);

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (sink, source) = invocation.expect_unary()?;
        Ok(Box::new(Self {
            sink: sink.try_ref()?,
            source: source.try_ref()?,
        }))
    }
}
impl<T> MechFunctionImpl for Assign<T>
where
    T: Clone + Debug + FunctionStateBacking + 'static,
{
    fn solve_result(&self) -> MResult<()> {
        let source_ptr = self.source.as_ptr();
        let sink_ptr = self.sink.as_mut_ptr();
        unsafe {
            *sink_ptr = (*source_ptr).clone();
        };
        Ok(())
    }
    fn stage_register(&self) -> MResult<Box<dyn ReactiveRegisterCommit>> {
        let next = self.source.borrow().clone();
        let output_cells = self.reactive_output_cell_ids();
        Ok(Box::new(ReactiveRegisterWrite::new(
            self.sink.clone(),
            next,
            output_cells,
        )))
    }
    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        Some(FunctionStatePort::from_ref(&self.sink))
    }
    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        Ok(Some(vec![FunctionStatePort::from_ref(&self.sink)]))
    }
    fn reactive_node_kind(&self) -> ReactiveNodeKind {
        ReactiveNodeKind::Register
    }
    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_STATE_REGISTER_CONTRACT)
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}
#[cfg(feature = "semantic-compiler")]
impl<T> MechFunctionCompiler for Assign<T>
where
    T: CompileConst + ConstElem + FunctionRuntimeType + AssignRuntimeName,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = T::assign_runtime_name();
        compile_unop!(name, self.sink, self.source, ctx);
    }
}
#[derive(Debug, Clone)]
pub struct EmptyAssignmentNotBytecodeCompilable;
impl MechErrorKind for EmptyAssignmentNotBytecodeCompilable {
    fn name(&self) -> &str {
        "EmptyAssignmentNotBytecodeCompilable"
    }

    fn message(&self) -> String {
        "empty stable assignment is not currently bytecode-compilable".to_string()
    }
}

#[cfg(feature = "semantic-compiler")]
pub struct AssignValue {}

#[cfg(feature = "semantic-compiler")]
#[derive(Debug)]
struct AssignCanonicalCell {
    sink: ValueCell,
    source: ValueCell,
}

#[cfg(feature = "semantic-compiler")]
impl MechFunctionImpl for AssignCanonicalCell {
    fn solve_result(&self) -> MResult<()> {
        let source = canonical_assignment_value(&self.sink, &self.source)?;
        self.sink.replace(&source)
    }

    fn stage_register(&self) -> MResult<Box<dyn ReactiveRegisterCommit>> {
        Ok(Box::new(ReactiveValueCellWrite::new(
            self.sink.clone(),
            canonical_assignment_value(&self.sink, &self.source)?,
        )?))
    }

    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        Some(FunctionStatePort::from_cell(&self.sink))
    }

    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        Ok(Some(vec![FunctionStatePort::from_cell(&self.sink)]))
    }

    fn reactive_node_kind(&self) -> ReactiveNodeKind {
        ReactiveNodeKind::Register
    }

    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_STATE_REGISTER_CONTRACT)
    }

    fn to_string(&self) -> String {
        "AssignCanonicalCell".to_owned()
    }
}

#[cfg(feature = "semantic-compiler")]
fn canonical_assignment_value(sink: &ValueCell, source: &ValueCell) -> MResult<Value> {
    let value = source.snapshot()?;
    if value.schema_key() == sink.schema_key() {
        return Ok(value);
    }
    if source.closed_schema_body()? != sink.closed_schema_body()? {
        return Ok(value);
    }
    let draft = value.canonical_data_draft().map_err(|error| {
        MechError::new(ValueCellSnapshotFailure { error }, None).with_compiler_loc()
    })?;
    sink.rebuild_data_draft(draft)
}

#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for AssignCanonicalCell {
    fn compiler_owned_value_cells(&self) -> Vec<ValueCell> {
        vec![self.sink.clone(), self.source.clone()]
    }

    fn compile(&self, context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let output = compile_value_cell_register(&self.sink, context)?;
        let source = compile_value_cell_register(&self.source, context)?;
        let operation = canonical_assignment_runtime_name(self.sink.representation())?;
        context.emit_unop(hash_str(&operation), output, source);
        Ok(output)
    }
}

#[cfg(feature = "semantic-compiler")]
fn canonical_assignment_runtime_name(
    representation: FunctionValueRepresentation,
) -> MResult<String> {
    let scalar = match representation {
        FunctionValueRepresentation::U8 => Some("u8"),
        FunctionValueRepresentation::U16 => Some("u16"),
        FunctionValueRepresentation::U32 => Some("u32"),
        FunctionValueRepresentation::U64 => Some("u64"),
        FunctionValueRepresentation::U128 => Some("u128"),
        FunctionValueRepresentation::I8 => Some("i8"),
        FunctionValueRepresentation::I16 => Some("i16"),
        FunctionValueRepresentation::I32 => Some("i32"),
        FunctionValueRepresentation::I64 => Some("i64"),
        FunctionValueRepresentation::I128 => Some("i128"),
        FunctionValueRepresentation::F32 => Some("f32"),
        FunctionValueRepresentation::F64 => Some("f64"),
        FunctionValueRepresentation::Bool => Some("bool"),
        FunctionValueRepresentation::String => Some("string"),
        FunctionValueRepresentation::R64 => Some("r64"),
        FunctionValueRepresentation::C64 => Some("c64"),
        FunctionValueRepresentation::Index => return Ok("Assign<index>".to_owned()),
        FunctionValueRepresentation::Matrix {
            element,
            storage: FunctionMatrixStoragePattern::Exact(storage),
        } => {
            let element = match element {
                FunctionMatrixElement::Index => "index",
                FunctionMatrixElement::Bool => "bool",
                FunctionMatrixElement::String => "string",
                FunctionMatrixElement::U8 => "u8",
                FunctionMatrixElement::U16 => "u16",
                FunctionMatrixElement::U32 => "u32",
                FunctionMatrixElement::U64 => "u64",
                FunctionMatrixElement::U128 => "u128",
                FunctionMatrixElement::I8 => "i8",
                FunctionMatrixElement::I16 => "i16",
                FunctionMatrixElement::I32 => "i32",
                FunctionMatrixElement::I64 => "i64",
                FunctionMatrixElement::I128 => "i128",
                FunctionMatrixElement::F32 => "f32",
                FunctionMatrixElement::F64 => "f64",
                FunctionMatrixElement::C64 => "complex",
                FunctionMatrixElement::R64 => "rational",
                FunctionMatrixElement::Value => "value",
            };
            let storage = match storage {
                FunctionMatrixRepresentation::Matrix1 => "Matrix1",
                FunctionMatrixRepresentation::Matrix2 => "Matrix2",
                FunctionMatrixRepresentation::Matrix3 => "Matrix3",
                FunctionMatrixRepresentation::Matrix4 => "Matrix4",
                FunctionMatrixRepresentation::Matrix2x3 => "Matrix2x3",
                FunctionMatrixRepresentation::Matrix3x2 => "Matrix3x2",
                FunctionMatrixRepresentation::RowVector2 => "RowVector2",
                FunctionMatrixRepresentation::RowVector3 => "RowVector3",
                FunctionMatrixRepresentation::RowVector4 => "RowVector4",
                FunctionMatrixRepresentation::Vector2 => "Vector2",
                FunctionMatrixRepresentation::Vector3 => "Vector3",
                FunctionMatrixRepresentation::Vector4 => "Vector4",
                FunctionMatrixRepresentation::RowVectorD => "RowDVector",
                FunctionMatrixRepresentation::VectorD => "DVector",
                FunctionMatrixRepresentation::MatrixD => "DMatrix",
            };
            return Ok(format!("Assign<{element}{storage}>"));
        }
        _ => None,
    };
    scalar.map(|name| format!("Assign<{name}>")).ok_or_else(|| {
        MechError::new(
            GenericError {
                msg: format!(
                    "canonical assignment representation {representation:?} has no bytecode runtime"
                ),
            },
            None,
        )
        .with_compiler_loc()
    })
}

#[cfg(feature = "semantic-compiler")]
#[derive(Debug)]
struct AssignCanonicalSelection {
    sink: ValueCell,
    source: ValueCell,
    selectors: Vec<CanonicalAccessSelector>,
    selection_kind: CanonicalAssignmentSelectionKind,
}

#[cfg(feature = "semantic-compiler")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CanonicalAssignmentSelectionKind {
    WholeValue,
    Linear,
    Rows,
    Columns,
    Rectangular,
    CollectionEntry,
    SingleElement,
}

#[cfg(feature = "semantic-compiler")]
impl CanonicalAssignmentSelectionKind {
    fn contract(self) -> &'static OperationContractDeclaration {
        match self {
            Self::WholeValue => &PURE_WHOLE_VALUE_STATE_REGISTER_CONTRACT,
            Self::Linear => &PURE_INDEXED_STATE_REGISTER_CONTRACT,
            Self::Rows => &PURE_ROW_INDEXED_STATE_REGISTER_CONTRACT,
            Self::Columns => &PURE_COLUMN_INDEXED_STATE_REGISTER_CONTRACT,
            Self::Rectangular => &PURE_RECTANGULAR_STATE_REGISTER_CONTRACT,
            Self::CollectionEntry => &PURE_COLLECTION_ENTRY_STATE_REGISTER_CONTRACT,
            Self::SingleElement => &PURE_SINGLE_ELEMENT_STATE_REGISTER_CONTRACT,
        }
    }

    const fn operation(self) -> &'static str {
        match self {
            Self::WholeValue => "core/assign/whole-value",
            Self::Linear => "core/assign/indexed-axis",
            Self::Rows => "core/assign/indexed-rows",
            Self::Columns => "core/assign/indexed-columns",
            Self::Rectangular => "core/assign/indexed-rectangle",
            Self::CollectionEntry => "core/assign/collection-entry",
            Self::SingleElement => "core/assign/single-element",
        }
    }
}

#[cfg(feature = "semantic-compiler")]
impl AssignCanonicalSelection {
    fn next_value(&self) -> MResult<Value> {
        let sink_schema = self.sink.closed_schema_body()?;
        match &sink_schema {
            SchemaBody::Tuple(elements) => {
                let [selector] = self.selectors.as_slice() else {
                    return Err(MechError::new(
                        IncorrectNumberOfArguments {
                            expected: 3,
                            found: self.selectors.len() + 2,
                        },
                        None,
                    )
                    .with_compiler_loc());
                };
                let index = canonical_indices(selector, elements.len())?[0];
                if self.source.closed_schema_body()? != elements[index] {
                    return Err(MechError::new(
                        GenericError {
                            msg:
                                "tuple assignment source schema does not match the selected element"
                                    .to_owned(),
                        },
                        None,
                    )
                    .with_compiler_loc());
                }
                let ValueDataDraft::Tuple(mut values) = canonical_draft(&self.sink)? else {
                    unreachable!()
                };
                values[index] = canonical_draft(&self.source)?;
                return ValueCell::from_schema_data(sink_schema, ValueDataDraft::Tuple(values))?
                    .snapshot();
            }
            SchemaBody::Record(fields) => {
                let [CanonicalAccessSelector::Cell(selector)] = self.selectors.as_slice() else {
                    return Err(MechError::new(
                        GenericError {
                            msg: "record assignment requires one id selector".to_owned(),
                        },
                        None,
                    )
                    .with_compiler_loc());
                };
                let ValueData::Id(field_id) = selector.snapshot()?.data().clone() else {
                    return Err(MechError::new(
                        GenericError {
                            msg: "record assignment requires one id selector".to_owned(),
                        },
                        None,
                    )
                    .with_compiler_loc());
                };
                let index = fields
                    .iter()
                    .position(|field| hash_str(&field.name) == field_id)
                    .ok_or_else(|| {
                        #[cfg(feature = "record")]
                        {
                            MechError::new(UndefinedRecordFieldError { id: field_id }, None)
                                .with_compiler_loc()
                        }
                        #[cfg(not(feature = "record"))]
                        {
                            MechError::new(
                                GenericError {
                                    msg: format!("record field {field_id:?} is not defined"),
                                },
                                None,
                            )
                            .with_compiler_loc()
                        }
                    })?;
                if self.source.closed_schema_body()? != fields[index].schema {
                    return Err(MechError::new(
                        GenericError {
                            msg:
                                "record assignment source schema does not match the selected field"
                                    .to_owned(),
                        },
                        None,
                    )
                    .with_compiler_loc());
                }
                let ValueDataDraft::Record(mut values) = canonical_draft(&self.sink)? else {
                    unreachable!()
                };
                values[index].value = canonical_draft(&self.source)?;
                return ValueCell::from_schema_data(sink_schema, ValueDataDraft::Record(values))?
                    .snapshot();
            }
            SchemaBody::Table { columns, .. } => {
                let [CanonicalAccessSelector::Cell(selector)] = self.selectors.as_slice() else {
                    return Err(MechError::new(
                        GenericError {
                            msg: "table column assignment requires one id selector".to_owned(),
                        },
                        None,
                    )
                    .with_compiler_loc());
                };
                let ValueData::Id(column_id) = selector.snapshot()?.data().clone() else {
                    return Err(MechError::new(
                        GenericError {
                            msg: "table column assignment requires one id selector".to_owned(),
                        },
                        None,
                    )
                    .with_compiler_loc());
                };
                let index = columns
                    .iter()
                    .position(|column| hash_str(&column.name) == column_id)
                    .ok_or_else(|| {
                        #[cfg(feature = "table")]
                        {
                            MechError::new(UndefinedTableColumnError { id: column_id }, None)
                                .with_compiler_loc()
                        }
                        #[cfg(not(feature = "table"))]
                        {
                            MechError::new(
                                GenericError {
                                    msg: format!("table column {column_id:?} is not defined"),
                                },
                                None,
                            )
                            .with_compiler_loc()
                        }
                    })?;
                let SchemaBody::Matrix {
                    element: source_element,
                    ..
                } = self.source.closed_schema_body()?
                else {
                    return Err(MechError::new(
                        GenericError {
                            msg: "table column assignment requires a matrix source".to_owned(),
                        },
                        None,
                    )
                    .with_compiler_loc());
                };
                if *source_element != columns[index].schema {
                    return Err(MechError::new(
                        GenericError {
                            msg: "table column assignment source schema does not match the column"
                                .to_owned(),
                        },
                        None,
                    )
                    .with_compiler_loc());
                }
                let ValueDataDraft::Matrix(source_values) = canonical_draft(&self.source)? else {
                    unreachable!()
                };
                let ValueDataDraft::Table(mut values) = canonical_draft(&self.sink)? else {
                    unreachable!()
                };
                if source_values.len() != values[index].values.len() {
                    return Err(MechError::new(
                        GenericError {
                            msg: "table column assignment cannot change the table row count"
                                .to_owned(),
                        },
                        None,
                    )
                    .with_compiler_loc());
                }
                values[index].values = source_values;
                return ValueCell::from_schema_data(sink_schema, ValueDataDraft::Table(values))?
                    .snapshot();
            }
            SchemaBody::Map { key, value, .. } => {
                let [CanonicalAccessSelector::Cell(selector)] = self.selectors.as_slice() else {
                    return Err(MechError::new(
                        GenericError {
                            msg: "map assignment requires one key selector".to_owned(),
                        },
                        None,
                    )
                    .with_compiler_loc());
                };
                if self.source.closed_schema_body()? != **value {
                    return Err(MechError::new(
                        GenericError {
                            msg: "map assignment source schema does not match the map value"
                                .to_owned(),
                        },
                        None,
                    )
                    .with_compiler_loc());
                }
                let replacement = canonical_draft(&self.source)?;
                let ValueDataDraft::Map(mut entries) = canonical_draft(&self.sink)? else {
                    unreachable!()
                };
                let mut found = false;
                for entry in &mut entries {
                    let candidate =
                        ValueCell::from_schema_data((**key).clone(), entry.items[0].clone())?;
                    if candidate.key_eq(selector)? {
                        entry.items[1] = replacement.clone();
                        found = true;
                        break;
                    }
                }
                if !found {
                    return Err(MechError::new(
                        GenericError {
                            msg: "canonical map key is not present".to_owned(),
                        },
                        None,
                    )
                    .with_compiler_loc());
                }
                return ValueCell::from_schema_data(sink_schema, ValueDataDraft::Map(entries))?
                    .snapshot();
            }
            _ => {}
        }
        let SchemaBody::Matrix {
            element,
            dimensions,
        } = sink_schema
        else {
            return Err(MechError::new(
                GenericError {
                    msg: "indexed assignment requires a matrix sink".to_owned(),
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
        let dimension_error = |axis: &str| {
            MechError::new(
                GenericError {
                    msg: format!("matrix {axis} extent exceeds the target index width"),
                },
                None,
            )
            .with_compiler_loc()
        };
        let rows = usize::try_from(*rows).map_err(|_| dimension_error("row"))?;
        let columns = usize::try_from(*columns).map_err(|_| dimension_error("column"))?;
        let element_count = rows.checked_mul(columns).ok_or_else(|| {
            MechError::new(
                GenericError {
                    msg: "matrix element count exceeds the target index width".to_owned(),
                },
                None,
            )
            .with_compiler_loc()
        })?;
        let positions = match self.selectors.as_slice() {
            [CanonicalAccessSelector::All]
            | [CanonicalAccessSelector::All, CanonicalAccessSelector::All] => {
                (0..element_count).collect::<Vec<_>>()
            }
            [selector] => canonical_indices(selector, element_count)?
                .into_iter()
                .map(|linear| {
                    let row = linear % rows;
                    let column = linear / rows;
                    row * columns + column
                })
                .collect::<Vec<_>>(),
            [row, column] => {
                let selected_rows = canonical_indices(row, rows)?;
                let selected_columns = canonical_indices(column, columns)?;
                let selection_count = selected_rows
                    .len()
                    .checked_mul(selected_columns.len())
                    .ok_or_else(|| {
                        MechError::new(
                            GenericError {
                                msg: "matrix assignment selection is too large".to_owned(),
                            },
                            None,
                        )
                        .with_compiler_loc()
                    })?;
                let mut positions = Vec::with_capacity(selection_count);
                for row in selected_rows {
                    for column in &selected_columns {
                        positions.push(row * columns + *column);
                    }
                }
                positions
            }
            _ => {
                return Err(MechError::new(
                    IncorrectNumberOfArguments {
                        expected: 3,
                        found: self.selectors.len() + 2,
                    },
                    None,
                )
                .with_compiler_loc());
            }
        };

        let mut sink_values = match canonical_draft(&self.sink)? {
            ValueDataDraft::Matrix(values) => values.into_vec(),
            _ => unreachable!("validated matrix sink retains matrix data"),
        };
        let source_schema = self.source.closed_schema_body()?;
        let source_values = match source_schema {
            SchemaBody::Matrix {
                element: source_element,
                ..
            } => {
                if source_element.as_ref() != element.as_ref() {
                    return Err(MechError::new(
                        GenericError {
                            msg: "indexed assignment source element schema does not match the sink"
                                .to_owned(),
                        },
                        None,
                    )
                    .with_compiler_loc());
                }
                let values = match canonical_draft(&self.source)? {
                    ValueDataDraft::Matrix(values) => values.into_vec(),
                    _ => unreachable!(),
                };
                if values.len() == sink_values.len() && values.len() != positions.len() {
                    positions
                        .iter()
                        .map(|position| values[*position].clone())
                        .collect::<Vec<_>>()
                } else if values.len() == positions.len() {
                    values
                } else {
                    return Err(MechError::new(
                        GenericError {
                            msg: format!(
                                "indexed assignment selected {} cells but the source has {}",
                                positions.len(),
                                values.len()
                            ),
                        },
                        None,
                    )
                    .with_compiler_loc());
                }
            }
            source_schema if source_schema == *element => {
                vec![canonical_draft(&self.source)?; positions.len()]
            }
            _ => {
                return Err(MechError::new(
                    GenericError {
                        msg: "indexed assignment source schema does not match the sink element"
                            .to_owned(),
                    },
                    None,
                )
                .with_compiler_loc());
            }
        };
        for (position, value) in positions.into_iter().zip(source_values) {
            sink_values[position] = value;
        }
        self.sink.rebuild_matrix_drafts(
            vec![rows as u64, columns as u64].into_boxed_slice(),
            sink_values.into_boxed_slice(),
        )
    }
}

#[cfg(feature = "semantic-compiler")]
impl MechFunctionImpl for AssignCanonicalSelection {
    fn solve_result(&self) -> MResult<()> {
        self.sink.replace(&self.next_value()?)
    }

    fn stage_register(&self) -> MResult<Box<dyn ReactiveRegisterCommit>> {
        Ok(Box::new(ReactiveValueCellWrite::new(
            self.sink.clone(),
            self.next_value()?,
        )?))
    }

    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        Some(FunctionStatePort::from_cell(&self.sink))
    }

    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        Ok(Some(vec![FunctionStatePort::from_cell(&self.sink)]))
    }

    fn reactive_node_kind(&self) -> ReactiveNodeKind {
        ReactiveNodeKind::Register
    }

    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(self.selection_kind.contract())
    }

    fn semantic_operation_name(&self) -> Option<&str> {
        Some(self.selection_kind.operation())
    }

    fn to_string(&self) -> String {
        "AssignCanonicalSelection".to_owned()
    }
}

#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for AssignCanonicalSelection {
    fn compiler_owned_value_cells(&self) -> Vec<ValueCell> {
        let mut cells = vec![self.sink.clone(), self.source.clone()];
        cells.extend(self.selectors.iter().filter_map(|selector| match selector {
            CanonicalAccessSelector::Cell(cell) => Some(cell.clone()),
            CanonicalAccessSelector::All => None,
        }));
        cells
    }

    fn compile(&self, context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let output = compile_value_cell_register(&self.sink, context)?;
        let source = compile_value_cell_register(&self.source, context)?;
        let selectors = self
            .selectors
            .iter()
            .filter_map(|selector| match selector {
                CanonicalAccessSelector::Cell(selector) => Some(selector),
                CanonicalAccessSelector::All => None,
            })
            .map(|selector| compile_value_cell_register(selector, context))
            .collect::<MResult<Vec<_>>>()?;
        if selectors.len()
            != match self.selection_kind {
                CanonicalAssignmentSelectionKind::WholeValue => 0,
                CanonicalAssignmentSelectionKind::Rectangular => 2,
                _ => 1,
            }
        {
            return Err(MechError::new(
                GenericError {
                    msg: "canonical bytecode assignment selector metadata is inconsistent"
                        .to_owned(),
                },
                None,
            )
            .with_compiler_loc());
        }
        let mut arguments = Vec::with_capacity(2 + selectors.len());
        arguments.push(output);
        arguments.push(source);
        arguments.extend(selectors);
        context.emit_varop(hash_str(self.selection_kind.operation()), output, arguments);
        Ok(output)
    }
}

#[cfg(feature = "semantic-compiler")]
fn canonical_indexed_assignment(
    invocation: &SpecializationInvocation,
) -> MResult<SpecializedFunction> {
    let sink = invocation
        .input(0)
        .expect("validated assignment sink")
        .cell()?
        .clone();
    let source = invocation
        .input(1)
        .expect("validated assignment source")
        .cell()?
        .clone();
    let selectors = invocation.inputs()[2..]
        .iter()
        .map(CanonicalAccessSelector::from_input)
        .collect::<MResult<Vec<_>>>()?;
    let selection_kind = match sink.closed_schema_body()? {
        SchemaBody::Map { .. } | SchemaBody::Table { .. } => {
            CanonicalAssignmentSelectionKind::CollectionEntry
        }
        SchemaBody::Tuple(_) | SchemaBody::Record(_) => {
            CanonicalAssignmentSelectionKind::SingleElement
        }
        SchemaBody::Matrix { .. } => match selectors.as_slice() {
            [CanonicalAccessSelector::All]
            | [CanonicalAccessSelector::All, CanonicalAccessSelector::All] => {
                CanonicalAssignmentSelectionKind::WholeValue
            }
            [CanonicalAccessSelector::Cell(_)] => CanonicalAssignmentSelectionKind::Linear,
            [
                CanonicalAccessSelector::Cell(_),
                CanonicalAccessSelector::All,
            ] => CanonicalAssignmentSelectionKind::Rows,
            [
                CanonicalAccessSelector::All,
                CanonicalAccessSelector::Cell(_),
            ] => CanonicalAssignmentSelectionKind::Columns,
            [
                CanonicalAccessSelector::Cell(_),
                CanonicalAccessSelector::Cell(_),
            ] => CanonicalAssignmentSelectionKind::Rectangular,
            _ => {
                return Err(MechError::new(
                    GenericError {
                        msg: "canonical matrix assignment requires one or two selectors".to_owned(),
                    },
                    None,
                )
                .with_compiler_loc());
            }
        },
        _ => CanonicalAssignmentSelectionKind::Linear,
    };
    let inputs = std::iter::once(sink.clone())
        .chain(std::iter::once(source.clone()))
        .chain(selectors.iter().filter_map(|selector| match selector {
            CanonicalAccessSelector::Cell(cell) => Some(cell.clone()),
            CanonicalAccessSelector::All => None,
        }))
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let implementation = AssignCanonicalSelection {
        sink: sink.clone(),
        source,
        selectors,
        selection_kind,
    };
    implementation.next_value()?;
    Ok(SpecializedFunction::new(FunctionInstance::new(
        Box::new(implementation),
        FunctionInvocation::variadic(sink, inputs),
    )))
}

#[cfg(feature = "semantic-compiler")]
impl CanonicalFunctionSpecializer for AssignValue {
    fn specialize_invocation(
        &self,
        invocation: &SpecializationInvocation,
        context: &mut SpecializationContext<'_>,
    ) -> MResult<SpecializedFunction> {
        if !(2..=4).contains(&invocation.len()) {
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
        if invocation.len() > 2 {
            return canonical_indexed_assignment(invocation);
        }
        let _ = context;
        let sink = sink.cell()?.clone();
        let source = source.cell()?.clone();
        sink.preflight_replace()?;
        Ok(SpecializedFunction::new(FunctionInstance::new(
            Box::new(AssignCanonicalCell {
                sink: sink.clone(),
                source: source.clone(),
            }),
            FunctionInvocation::unary(sink, source),
        )))
    }
}

#[cfg(feature = "semantic-compiler")]
pub struct AssignColumn {}
#[cfg(feature = "semantic-compiler")]
impl CanonicalFunctionSpecializer for AssignColumn {
    fn specialize_invocation(
        &self,
        invocation: &SpecializationInvocation,
        _: &mut SpecializationContext<'_>,
    ) -> MResult<SpecializedFunction> {
        canonical_indexed_assignment(invocation)
    }
}

// x += y ----------------------------------------------------------------------

#[cfg(feature = "semantic-compiler")]
#[derive(Debug)]
struct AddAssignCanonicalTable {
    sink: ValueCell,
    source: ValueCell,
}

#[cfg(feature = "semantic-compiler")]
impl AddAssignCanonicalTable {
    fn next_value(&self) -> MResult<Value> {
        let sink_schema = self.sink.closed_schema_body()?;
        let SchemaBody::Table { columns, .. } = &sink_schema else {
            return Err(MechError::new(
                GenericError {
                    msg: "assign/add requires a table sink".to_owned(),
                },
                None,
            )
            .with_compiler_loc());
        };
        let ValueDataDraft::Table(mut sink_columns) = canonical_draft(&self.sink)? else {
            unreachable!()
        };
        match self.source.closed_schema_body()? {
            SchemaBody::Record(fields) => {
                let ValueDataDraft::Record(values) = canonical_draft(&self.source)? else {
                    unreachable!()
                };
                for (index, column) in columns.iter().enumerate() {
                    let source_index = fields
                        .iter()
                        .position(|field| field.name == column.name)
                        .ok_or_else(|| {
                            MechError::new(
                                GenericError {
                                    msg: format!(
                                        "appended record is missing table column `{}`",
                                        column.name
                                    ),
                                },
                                None,
                            )
                            .with_compiler_loc()
                        })?;
                    if fields[source_index].schema != column.schema {
                        return Err(MechError::new(
                            GenericError {
                                msg: format!(
                                    "appended record field `{}` has the wrong schema",
                                    column.name
                                ),
                            },
                            None,
                        )
                        .with_compiler_loc());
                    }
                    let mut column_values = sink_columns[index].values.clone().into_vec();
                    column_values.push(values[source_index].value.clone());
                    sink_columns[index].values = column_values.into_boxed_slice();
                }
            }
            SchemaBody::Table {
                columns: source_schema,
                ..
            } => {
                if source_schema.as_ref() != columns.as_ref() {
                    return Err(MechError::new(
                        GenericError {
                            msg: "appended table schema does not match the sink".to_owned(),
                        },
                        None,
                    )
                    .with_compiler_loc());
                }
                let ValueDataDraft::Table(source_columns) = canonical_draft(&self.source)? else {
                    unreachable!()
                };
                for (sink, source) in sink_columns.iter_mut().zip(source_columns) {
                    let mut values = sink.values.clone().into_vec();
                    values.extend(source.values.into_vec());
                    sink.values = values.into_boxed_slice();
                }
            }
            _ => {
                return Err(MechError::new(
                    GenericError {
                        msg: "assign/add accepts a record or table source".to_owned(),
                    },
                    None,
                )
                .with_compiler_loc());
            }
        }
        ValueCell::from_schema_data(sink_schema, ValueDataDraft::Table(sink_columns))?.snapshot()
    }
}

#[cfg(feature = "semantic-compiler")]
impl MechFunctionImpl for AddAssignCanonicalTable {
    fn solve_result(&self) -> MResult<()> {
        self.sink.replace(&self.next_value()?)
    }

    fn stage_register(&self) -> MResult<Box<dyn ReactiveRegisterCommit>> {
        Ok(Box::new(ReactiveValueCellWrite::new(
            self.sink.clone(),
            self.next_value()?,
        )?))
    }

    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        Some(FunctionStatePort::from_cell(&self.sink))
    }

    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        Ok(Some(vec![FunctionStatePort::from_cell(&self.sink)]))
    }

    fn reactive_node_kind(&self) -> ReactiveNodeKind {
        ReactiveNodeKind::Register
    }

    fn to_string(&self) -> String {
        "AddAssignCanonicalTable".to_owned()
    }
}

#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for AddAssignCanonicalTable {
    fn compiler_owned_value_cells(&self) -> Vec<ValueCell> {
        vec![self.sink.clone(), self.source.clone()]
    }

    fn compile(&self, _: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        Err(MechError::new(
            GenericError {
                msg: "canonical table append is not bytecode-compilable yet".to_owned(),
            },
            None,
        )
        .with_compiler_loc())
    }
}

#[cfg(feature = "semantic-compiler")]
pub struct AddAssignValue {}
#[cfg(feature = "semantic-compiler")]
impl CanonicalFunctionSpecializer for AddAssignValue {
    fn specialize_invocation(
        &self,
        invocation: &SpecializationInvocation,
        _: &mut SpecializationContext<'_>,
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
        let sink = invocation
            .input(0)
            .expect("validated append sink")
            .cell()?
            .clone();
        let source = invocation
            .input(1)
            .expect("validated append source")
            .cell()?
            .clone();
        let implementation = AddAssignCanonicalTable {
            sink: sink.clone(),
            source: source.clone(),
        };
        implementation.next_value()?;
        Ok(SpecializedFunction::new(FunctionInstance::new(
            Box::new(implementation),
            FunctionInvocation::unary(sink, source),
        )))
    }
}

#[cfg(all(test, feature = "semantic-compiler"))]
mod canonical_aggregate_assignment_tests {
    use super::*;

    #[test]
    fn map_assignment_uses_canonical_key_equality() {
        let sink = ValueCell::from_schema_data(
            SchemaBody::Map {
                key: Box::new(SchemaBody::FloatingPoint(mech_core::FloatWidth::W64)),
                value: Box::new(SchemaBody::String),
                cardinality: mech_core::CardinalitySpec::Dynamic { upper_bound: None },
            },
            ValueDataDraft::Map(
                vec![mech_core::snapshot::MapEntryDraft {
                    items: vec![
                        ValueDataDraft::F64(mech_core::snapshot::F64Bits::from_f64(-0.0)),
                        ValueDataDraft::String("old".to_owned()),
                    ]
                    .into_boxed_slice(),
                }]
                .into_boxed_slice(),
            ),
        )
        .unwrap();
        let assignment = AssignCanonicalSelection {
            sink: sink.clone(),
            source: ValueCell::from_exact("new".to_owned()).unwrap(),
            selectors: vec![CanonicalAccessSelector::Cell(
                ValueCell::from_schema_data(
                    SchemaBody::FloatingPoint(mech_core::FloatWidth::W64),
                    ValueDataDraft::F64(mech_core::snapshot::F64Bits::from_f64(0.0)),
                )
                .unwrap(),
            )],
            selection_kind: CanonicalAssignmentSelectionKind::CollectionEntry,
        };

        assignment.solve_result().unwrap();
        let ValueDataDraft::Map(entries) = sink.snapshot().unwrap().canonical_data_draft().unwrap()
        else {
            panic!("map assignment must preserve the map schema");
        };
        assert!(matches!(
            entries[0].items[1],
            ValueDataDraft::String(ref value) if value == "new"
        ));
    }
}
