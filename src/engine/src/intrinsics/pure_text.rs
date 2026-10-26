//! Pure numeric text and collection operations shared by ordinary source,
//! managed runtime factories and resident artifact execution.
use crate::*;
use core::fmt::Write;
use mech_core::snapshot::{SequenceView, prospective_string_value_footprint};
use std::sync::LazyLock;

fn invalid(message: &'static str) -> MechError {
    MechError::new(
        GenericError {
            msg: message.to_owned(),
        },
        None,
    )
    .with_compiler_loc()
}

pub(crate) fn parse_u64(text: &str) -> MResult<u64> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(invalid(
            "number/parse-u64 expects nonempty unsigned decimal text",
        ));
    }
    text.bytes().try_fold(0_u64, |value, byte| {
        value
            .checked_mul(10)
            .and_then(|value| value.checked_add(u64::from(byte - b'0')))
            .ok_or_else(|| invalid("number/parse-u64 value exceeds u64::MAX"))
    })
}

pub(crate) fn grapheme_length(text: &str) -> MResult<u64> {
    u64::try_from(super::text_graphemes::count(text))
        .map_err(|_| invalid("sequence/length exceeds u64::MAX"))
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum NumericTextValue {
    Unsigned(u128),
    Signed(i128),
    Float32(f32),
    Float64(f64),
}

pub(crate) fn numeric_at(data: &ValueData, index: usize) -> MResult<NumericTextValue> {
    use NumericTextValue::{Float32, Float64, Signed, Unsigned};
    macro_rules! scalar {
        ($value:expr, $variant:ident, $type:ty) => {
            if index == 0 {
                Ok($variant(*$value as $type))
            } else {
                Err(invalid("numeric format index is out of bounds"))
            }
        };
    }
    macro_rules! element {
        ($values:expr, $variant:ident, $type:ty) => {
            $values
                .get(index)
                .map(|value| $variant(*value as $type))
                .ok_or_else(|| invalid("numeric format index is out of bounds"))
        };
    }
    match data {
        ValueData::U8(value) => scalar!(value, Unsigned, u128),
        ValueData::U16(value) => scalar!(value, Unsigned, u128),
        ValueData::U32(value) => scalar!(value, Unsigned, u128),
        ValueData::U64(value) => scalar!(value, Unsigned, u128),
        ValueData::U128(value) => scalar!(value, Unsigned, u128),
        ValueData::I8(value) => scalar!(value, Signed, i128),
        ValueData::I16(value) => scalar!(value, Signed, i128),
        ValueData::I32(value) => scalar!(value, Signed, i128),
        ValueData::I64(value) => scalar!(value, Signed, i128),
        ValueData::I128(value) => scalar!(value, Signed, i128),
        ValueData::F32(value) if index == 0 => Ok(Float32(value.to_f32())),
        ValueData::F64(value) if index == 0 => Ok(Float64(value.to_f64())),
        ValueData::Matrix(matrix) => match matrix.elements() {
            SequenceView::U8(values) => element!(values, Unsigned, u128),
            SequenceView::U16(values) => element!(values, Unsigned, u128),
            SequenceView::U32(values) => element!(values, Unsigned, u128),
            SequenceView::U64(values) => element!(values, Unsigned, u128),
            SequenceView::U128(values) => element!(values, Unsigned, u128),
            SequenceView::I8(values) => element!(values, Signed, i128),
            SequenceView::I16(values) => element!(values, Signed, i128),
            SequenceView::I32(values) => element!(values, Signed, i128),
            SequenceView::I64(values) => element!(values, Signed, i128),
            SequenceView::I128(values) => element!(values, Signed, i128),
            SequenceView::F32(values) => values
                .get(index)
                .map(|value| Float32(value.to_f32()))
                .ok_or_else(|| invalid("numeric format index is out of bounds")),
            SequenceView::F64(values) => values
                .get(index)
                .map(|value| Float64(value.to_f64()))
                .ok_or_else(|| invalid("numeric format index is out of bounds")),
            SequenceView::Values(values) => values
                .get(index)
                .ok_or_else(|| invalid("numeric format index is out of bounds"))
                .and_then(|value| numeric_at(value, 0)),
            _ => Err(invalid(
                "number/format expects plain integer or floating-point values",
            )),
        },
        _ => Err(invalid(
            "number/format expects plain integer or floating-point values",
        )),
    }
}

pub(crate) fn format_schema_supported(schema: &SchemaBody) -> bool {
    match schema {
        SchemaBody::UnsignedInteger(_)
        | SchemaBody::SignedInteger(_)
        | SchemaBody::FloatingPoint(_) => true,
        SchemaBody::Matrix {
            element,
            dimensions,
        } if dimensions.len() == 2 => {
            matches!(
                element.as_ref(),
                SchemaBody::UnsignedInteger(_)
                    | SchemaBody::SignedInteger(_)
                    | SchemaBody::FloatingPoint(_)
            )
        }
        _ => false,
    }
}

pub(crate) fn validate_format_precision(schema: &SchemaBody, precision: u64) -> MResult<()> {
    if precision > 12 {
        return Err(invalid("decimal precision exceeds 12"));
    }
    let scalar = match schema {
        SchemaBody::Matrix { element, .. } => element.as_ref(),
        other => other,
    };
    if !format_schema_supported(schema) {
        return Err(invalid(
            "number/format expects plain integer or floating-point values",
        ));
    }
    if precision != 0
        && matches!(
            scalar,
            SchemaBody::UnsignedInteger(_) | SchemaBody::SignedInteger(_)
        )
    {
        return Err(invalid("integer formatting requires precision zero"));
    }
    Ok(())
}

pub(crate) fn write_number(
    writer: &mut dyn Write,
    value: NumericTextValue,
    precision: u64,
) -> MResult<()> {
    use NumericTextValue::*;
    if precision > 12 {
        return Err(invalid("decimal precision exceeds 12"));
    }
    let result = match value {
        Unsigned(value) if precision == 0 => write!(writer, "{value}"),
        Signed(value) if precision == 0 => write!(writer, "{value}"),
        Unsigned(_) | Signed(_) => {
            return Err(invalid("integer formatting requires precision zero"));
        }
        Float32(value) => write!(writer, "{:.*}", precision as usize, value),
        Float64(value) => write!(writer, "{:.*}", precision as usize, value),
    };
    result.map_err(|_| invalid("numeric format output exceeds its admitted payload"))
}

#[derive(Default)]
struct ByteCounter(usize);
impl Write for ByteCounter {
    fn write_str(&mut self, text: &str) -> core::fmt::Result {
        self.0 = self.0.checked_add(text.len()).ok_or(core::fmt::Error)?;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
enum Operation {
    Parse,
    Format,
    Length,
}
impl Operation {
    fn name(self) -> &'static str {
        match self {
            Self::Parse => "number/parse-u64",
            Self::Format => "number/format",
            Self::Length => "sequence/length",
        }
    }
    fn arity(self) -> usize {
        if matches!(self, Self::Format) { 2 } else { 1 }
    }
    fn contract(self, matrix: bool) -> &'static OperationContractDeclaration {
        match self {
            Self::Parse => &PARSE_CONTRACT,
            Self::Format if matrix => &FORMAT_MATRIX_CONTRACT,
            Self::Format => &FORMAT_SCALAR_CONTRACT,
            Self::Length => &LENGTH_CONTRACT,
        }
    }
}
static PARSE_CONTRACT: LazyLock<OperationContractDeclaration> = LazyLock::new(|| {
    mech_core::maintained_operation_contract("number/parse-u64", 1, false).unwrap()
});
static FORMAT_SCALAR_CONTRACT: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| mech_core::maintained_operation_contract("number/format", 2, false).unwrap());
static FORMAT_MATRIX_CONTRACT: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| mech_core::maintained_operation_contract("number/format", 2, true).unwrap());
static LENGTH_CONTRACT: LazyLock<OperationContractDeclaration> = LazyLock::new(|| {
    mech_core::maintained_operation_contract("sequence/length", 1, false).unwrap()
});

fn validate(operation: Operation, output: &ValueCell, inputs: &[ValueCell]) -> MResult<()> {
    if inputs.len() != operation.arity() {
        return Err(invalid("pure text operation arity mismatch"));
    }
    let source = inputs[0].closed_schema_body()?;
    let expected = match operation {
        Operation::Parse if source == SchemaBody::String => {
            SchemaBody::UnsignedInteger(IntegerWidth::W64)
        }
        Operation::Length if matches!(source, SchemaBody::String | SchemaBody::Matrix { .. }) => {
            if matches!(&source, SchemaBody::Matrix { dimensions, .. } if dimensions.len() != 2) {
                return Err(invalid("sequence/length expects a two-dimensional matrix"));
            }
            SchemaBody::UnsignedInteger(IntegerWidth::W64)
        }
        Operation::Format if format_schema_supported(&source) => {
            if inputs[1].closed_schema_body()? != SchemaBody::UnsignedInteger(IntegerWidth::W64) {
                return Err(invalid("number/format precision must be a u64 scalar"));
            }
            match source {
                SchemaBody::Matrix { dimensions, .. } => SchemaBody::Matrix {
                    element: Box::new(SchemaBody::String),
                    dimensions,
                },
                _ => SchemaBody::String,
            }
        }
        _ => return Err(invalid("unsupported pure text operation input schema")),
    };
    if output.closed_schema_body()? != expected {
        return Err(invalid("pure text operation output schema mismatch"));
    }
    Ok(())
}
fn validate_parse(output: &ValueCell, inputs: &[ValueCell]) -> MResult<()> {
    validate(Operation::Parse, output, inputs)
}
fn validate_format_scalar(output: &ValueCell, inputs: &[ValueCell]) -> MResult<()> {
    validate(Operation::Format, output, inputs)?;
    if output.closed_schema_body()? != SchemaBody::String {
        return Err(invalid("scalar formatter requires a String output"));
    }
    Ok(())
}
fn validate_format_matrix(output: &ValueCell, inputs: &[ValueCell]) -> MResult<()> {
    validate(Operation::Format, output, inputs)?;
    if !matches!(output.closed_schema_body()?, SchemaBody::Matrix { .. }) {
        return Err(invalid("matrix formatter requires a String matrix output"));
    }
    Ok(())
}
fn validate_length(output: &ValueCell, inputs: &[ValueCell]) -> MResult<()> {
    validate(Operation::Length, output, inputs)
}

#[derive(Debug)]
struct PureTextRuntime {
    operation: Operation,
    inputs: Vec<ValueCell>,
    output: ValueCell,
    matrix: bool,
    #[cfg(feature = "u64")]
    fixed_count_output: Option<ManagedPort<u64>>,
}
impl PureTextRuntime {
    fn new(operation: Operation, invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let inputs = invocation.input_cells().to_vec();
        let output = invocation.output_cell().clone();
        validate(operation, &output, &inputs)?;
        let matrix = matches!(output.closed_schema_body()?, SchemaBody::Matrix { .. });
        #[cfg(feature = "u64")]
        let fixed_count_output = if !matches!(operation, Operation::Format)
            && !output.requires_canonical_output_builder()?
        {
            Some(invocation.output().try_managed::<u64>()?)
        } else {
            None
        };
        Ok(Box::new(Self {
            operation,
            inputs,
            output,
            matrix,
            #[cfg(feature = "u64")]
            fixed_count_output,
        }))
    }
    fn footprint(&self) -> MResult<CurrentMemoryFootprint> {
        if !matches!(self.operation, Operation::Format) {
            return self.output.current_memory_footprint();
        }
        let source = self.inputs[0].closed_schema_body()?;
        let (element, count) = match &source {
            SchemaBody::Matrix {
                element,
                dimensions,
            } => {
                let count = dimensions
                    .iter()
                    .try_fold(1u64, |count, dimension| match dimension {
                        DimensionExpr::Constant(extent) => count.checked_mul(*extent),
                        _ => None,
                    })
                    .ok_or_else(|| invalid("numeric format cardinality exceeds u64"))?;
                (element.as_ref(), count)
            }
            source => (source, 1),
        };
        // Plan from schemas without reconstructing an owned input or evaluating
        // a pure operation before its branch executes. These bounds include the
        // sign, decimal point and the maximum supported twelve fractional digits.
        let bytes_per_element = match element {
            SchemaBody::UnsignedInteger(IntegerWidth::W8) => 3,
            SchemaBody::UnsignedInteger(IntegerWidth::W16) => 5,
            SchemaBody::UnsignedInteger(IntegerWidth::W32) => 10,
            SchemaBody::UnsignedInteger(IntegerWidth::W64) => 20,
            SchemaBody::UnsignedInteger(IntegerWidth::W128) => 39,
            SchemaBody::SignedInteger(IntegerWidth::W8) => 4,
            SchemaBody::SignedInteger(IntegerWidth::W16) => 6,
            SchemaBody::SignedInteger(IntegerWidth::W32) => 11,
            SchemaBody::SignedInteger(IntegerWidth::W64) => 20,
            SchemaBody::SignedInteger(IntegerWidth::W128) => 40,
            SchemaBody::FloatingPoint(FloatWidth::W32) => 53,
            SchemaBody::FloatingPoint(FloatWidth::W64) => 324,
            _ => return Err(invalid("unsupported numeric format input schema")),
        };
        let payload = count
            .checked_mul(bytes_per_element)
            .ok_or_else(|| invalid("numeric format payload exceeds u64"))?;
        let shape_count = self.output.shape().parameter_values().len();
        let footprint =
            prospective_string_value_footprint(shape_count, self.matrix, count, payload)
                .map_err(|_| invalid("numeric format footprint exceeds u64"))?;
        Ok(CurrentMemoryFootprint {
            logical_elements: count,
            payload_bytes: footprint.retained_bytes,
            encoded_bytes: footprint.encoded_bytes,
            retained_nodes: footprint.node_count,
            schema_bytes: self.output.current_memory_footprint()?.schema_bytes,
            shape_parameter_count: shape_count as u64,
            ..CurrentMemoryFootprint::default()
        })
    }
    fn construct(
        &self,
        construction: &mut FrozenSnapshotConstruction,
        source: &Value,
        precision: Option<u64>,
    ) -> MResult<Value> {
        match self.operation {
            Operation::Parse => {
                let ValueData::String(text) = source.data() else {
                    return Err(invalid("number/parse-u64 expects a String"));
                };
                construction
                    .try_rebuild_data_draft(&self.output, ValueDataDraft::U64(parse_u64(text)?))
            }
            Operation::Length => {
                let count = match source.data() {
                    ValueData::String(text) => grapheme_length(text)?,
                    ValueData::Matrix(matrix) => u64::try_from(matrix.elements().len())
                        .map_err(|_| invalid("sequence/length exceeds u64"))?,
                    _ => return Err(invalid("sequence/length expects a String or matrix")),
                };
                construction.try_rebuild_data_draft(&self.output, ValueDataDraft::U64(count))
            }
            Operation::Format => {
                let precision = precision.ok_or_else(|| invalid("missing format precision"))?;
                let schemas = source
                    .schemas()
                    .ok_or_else(|| invalid("numeric formatter lacks captured schemas"))?;
                let source_schema = schemas
                    .get(source.schema())
                    .ok_or_else(|| invalid("numeric formatter lacks its captured schema"))?
                    .body();
                let (dimensions, count) = match (source_schema, source.data()) {
                    (SchemaBody::Matrix { dimensions, .. }, ValueData::Matrix(matrix)) => {
                        let [rows, columns] = dimensions.as_ref() else {
                            return Err(invalid("numeric formatter requires rank two"));
                        };
                        (
                            [
                                source
                                    .shape()
                                    .resolve_dimension(rows)
                                    .map_err(MechError::from)?,
                                source
                                    .shape()
                                    .resolve_dimension(columns)
                                    .map_err(MechError::from)?,
                            ],
                            matrix.elements().len(),
                        )
                    }
                    (_, _) => ([1, 1], 1),
                };
                self.construct_formatted(
                    construction,
                    source_schema,
                    dimensions,
                    count,
                    precision,
                    |index| numeric_at(source.data(), index),
                )
            }
        }
    }

    fn construct_formatted(
        &self,
        construction: &mut FrozenSnapshotConstruction,
        schema: &SchemaBody,
        dimensions: [u64; 2],
        count: usize,
        precision: u64,
        mut number: impl FnMut(usize) -> MResult<NumericTextValue>,
    ) -> MResult<Value> {
        validate_format_precision(schema, precision)?;
        let mut format_one = |construction: &mut FrozenSnapshotConstruction, index| {
            let value = number(index)?;
            let mut counter = ByteCounter::default();
            write_number(&mut counter, value, precision)?;
            struct Bytes {
                data: Vec<u8>,
                capacity: usize,
            }
            impl Write for Bytes {
                fn write_str(&mut self, text: &str) -> core::fmt::Result {
                    if self
                        .data
                        .len()
                        .checked_add(text.len())
                        .ok_or(core::fmt::Error)?
                        > self.capacity
                    {
                        return Err(core::fmt::Error);
                    }
                    self.data.extend_from_slice(text.as_bytes());
                    Ok(())
                }
            }
            let mut bytes = Bytes {
                data: construction.try_vec_with_capacity(counter.0)?,
                capacity: counter.0,
            };
            write_number(&mut bytes, value, precision)?;
            let text = String::from_utf8(bytes.data)
                .map_err(|_| invalid("numeric formatter produced invalid Unicode"))?;
            Ok(ValueDataDraft::String(text))
        };
        if !self.matrix {
            let value = format_one(construction, 0)?;
            return construction.try_rebuild_data_draft(&self.output, value);
        }
        let values = construction.try_boxed_slice_with(count, format_one)?;
        let dimensions = construction.try_boxed_slice_with(2, |_, index| Ok(dimensions[index]))?;
        construction.try_rebuild_matrix_drafts(&self.output, dimensions, values)
    }

    fn construct_dense_format(
        &self,
        frame: &KernelMemoryFrame<'_>,
        construction: &mut FrozenSnapshotConstruction,
        precision: u64,
    ) -> MResult<Value> {
        // A canonical-only profile has no enabled dense numeric branch.
        let _ = (&construction, precision);
        let descriptor = frame.input_cell_descriptor(&self.inputs[0], 0)?;
        let schema = descriptor.schema().body();
        let element = match schema {
            SchemaBody::Matrix { element, .. } => element.as_ref(),
            body => body,
        };
        #[cfg(any(
            feature = "u8",
            feature = "u16",
            feature = "u32",
            feature = "u64",
            feature = "u128",
            feature = "i8",
            feature = "i16",
            feature = "i32",
            feature = "i64",
            feature = "i128",
            feature = "f32",
            feature = "f64"
        ))]
        macro_rules! typed {
            ($type:ty, $convert:expr) => {
                frame.with_input_cell_value_view::<$type, _>(&self.inputs[0], 0, |view| {
                    let dimensions = [
                        u64::try_from(view.rows())
                            .map_err(|_| invalid("numeric format rows exceed u64"))?,
                        u64::try_from(view.columns())
                            .map_err(|_| invalid("numeric format columns exceed u64"))?,
                    ];
                    self.construct_formatted(
                        construction,
                        schema,
                        dimensions,
                        view.len(),
                        precision,
                        |index| {
                            let value = view
                                .get(index / view.columns(), index % view.columns())
                                .ok_or_else(|| {
                                    invalid("numeric formatter input view is out of bounds")
                                })?;
                            Ok(($convert)(value))
                        },
                    )
                })
            };
        }
        match element {
            #[cfg(feature = "u8")]
            SchemaBody::UnsignedInteger(IntegerWidth::W8) => {
                typed!(u8, |v| NumericTextValue::Unsigned(u128::from(v)))
            }
            #[cfg(feature = "u16")]
            SchemaBody::UnsignedInteger(IntegerWidth::W16) => {
                typed!(u16, |v| NumericTextValue::Unsigned(u128::from(v)))
            }
            #[cfg(feature = "u32")]
            SchemaBody::UnsignedInteger(IntegerWidth::W32) => {
                typed!(u32, |v| NumericTextValue::Unsigned(u128::from(v)))
            }
            #[cfg(feature = "u64")]
            SchemaBody::UnsignedInteger(IntegerWidth::W64) => {
                typed!(u64, |v| NumericTextValue::Unsigned(u128::from(v)))
            }
            #[cfg(feature = "u128")]
            SchemaBody::UnsignedInteger(IntegerWidth::W128) => {
                typed!(u128, NumericTextValue::Unsigned)
            }
            #[cfg(feature = "i8")]
            SchemaBody::SignedInteger(IntegerWidth::W8) => {
                typed!(i8, |v| NumericTextValue::Signed(i128::from(v)))
            }
            #[cfg(feature = "i16")]
            SchemaBody::SignedInteger(IntegerWidth::W16) => {
                typed!(i16, |v| NumericTextValue::Signed(i128::from(v)))
            }
            #[cfg(feature = "i32")]
            SchemaBody::SignedInteger(IntegerWidth::W32) => {
                typed!(i32, |v| NumericTextValue::Signed(i128::from(v)))
            }
            #[cfg(feature = "i64")]
            SchemaBody::SignedInteger(IntegerWidth::W64) => {
                typed!(i64, |v| NumericTextValue::Signed(i128::from(v)))
            }
            #[cfg(feature = "i128")]
            SchemaBody::SignedInteger(IntegerWidth::W128) => typed!(i128, NumericTextValue::Signed),
            #[cfg(feature = "f32")]
            SchemaBody::FloatingPoint(FloatWidth::W32) => typed!(f32, NumericTextValue::Float32),
            #[cfg(feature = "f64")]
            SchemaBody::FloatingPoint(FloatWidth::W64) => typed!(f64, NumericTextValue::Float64),
            _ => Err(invalid(
                "numeric formatter has no enabled dense input representation",
            )),
        }
    }
}

impl MechFunctionImpl for PureTextRuntime {
    fn planned_output_footprints(&self) -> MResult<Option<Box<[CurrentMemoryFootprint]>>> {
        Ok(Some(vec![self.footprint()?].into_boxed_slice()))
    }
    fn solve_managed(
        &self,
        frame: &mut KernelMemoryFrame<'_>,
        _: &mut dyn MechExecutionServices,
    ) -> MResult<ReactiveSolveStatus> {
        #[cfg(feature = "u64")]
        if let Some(output) = &self.fixed_count_output {
            let descriptor = frame.input_cell_descriptor(&self.inputs[0], 0)?;
            let count = if let (Operation::Length, SchemaBody::Matrix { dimensions, .. }) =
                (self.operation, descriptor.schema().body())
            {
                dimensions.iter().try_fold(1u64, |count, extent| {
                    let extent = descriptor
                        .shape()
                        .resolve_dimension(extent)
                        .map_err(MechError::from)?;
                    count
                        .checked_mul(extent)
                        .ok_or_else(|| invalid("sequence/length exceeds u64"))
                })?
            } else {
                // String inputs share their immutable canonical root after
                // lease validation; no owned payload is reconstructed.
                let source = frame.snapshot_input_cell(&self.inputs[0], 0)?;
                let ValueData::String(text) = source.data() else {
                    return Err(invalid("pure text count operation expects a String"));
                };
                match self.operation {
                    Operation::Parse => parse_u64(text)?,
                    Operation::Length => grapheme_length(text)?,
                    Operation::Format => unreachable!("formatter has a String output"),
                }
            };
            frame
                .with_output_port_view(output, |view| view.try_fill_column_major(|_| Ok(count)))?;
            return Ok(ReactiveSolveStatus::Changed);
        }
        frame.with_admitted_canonical_output(
            &self.output,
            self.footprint()?,
            |frame, construction| {
                if matches!(self.operation, Operation::Length) {
                    let descriptor = frame.input_cell_descriptor(&self.inputs[0], 0)?;
                    if let SchemaBody::Matrix { dimensions, .. } = descriptor.schema().body() {
                        let count = dimensions.iter().try_fold(1u64, |count, extent| {
                            let extent = descriptor
                                .shape()
                                .resolve_dimension(extent)
                                .map_err(MechError::from)?;
                            count
                                .checked_mul(extent)
                                .ok_or_else(|| invalid("sequence/length exceeds u64"))
                        })?;
                        let next = construction
                            .try_rebuild_data_draft(&self.output, ValueDataDraft::U64(count))?;
                        return Ok(((), next));
                    }
                }
                let precision = if matches!(self.operation, Operation::Format) {
                    #[cfg(feature = "u64")]
                    if !self.inputs[1].requires_canonical_output_builder()? {
                        let precision = frame.with_input_cell_value_view::<u64, _>(
                            &self.inputs[1],
                            1,
                            |view| {
                                view.get(0, 0)
                                    .ok_or_else(|| invalid("format precision view is empty"))
                            },
                        )?;
                        if !self.inputs[0].requires_canonical_output_builder()? {
                            return Ok((
                                (),
                                self.construct_dense_format(frame, construction, precision)?,
                            ));
                        }
                        let source = frame.snapshot_input_cell_with_construction(
                            &self.inputs[0],
                            0,
                            construction,
                        )?;
                        return Ok(((), self.construct(construction, &source, Some(precision))?));
                    }
                    let value = frame.snapshot_input_cell_with_construction(
                        &self.inputs[1],
                        1,
                        construction,
                    )?;
                    let ValueData::U64(precision) = value.data() else {
                        return Err(invalid("number/format precision must be a u64 scalar"));
                    };
                    let precision = *precision;
                    if !self.inputs[0].requires_canonical_output_builder()? {
                        return Ok((
                            (),
                            self.construct_dense_format(frame, construction, precision)?,
                        ));
                    }
                    Some(precision)
                } else {
                    None
                };
                let source = frame.snapshot_input_cell_with_construction(
                    &self.inputs[0],
                    0,
                    construction,
                )?;
                Ok(((), self.construct(construction, &source, precision)?))
            },
        )?;
        Ok(ReactiveSolveStatus::Changed)
    }
    fn semantic_operation_name(&self) -> Option<&str> {
        Some(self.operation.name())
    }
    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(self.operation.contract(self.matrix))
    }
    fn to_string(&self) -> String {
        self.operation.name().to_owned()
    }
}
#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for PureTextRuntime {
    fn compiler_owned_value_cells(&self) -> Vec<ValueCell> {
        self.inputs
            .iter()
            .cloned()
            .chain(std::iter::once(self.output.clone()))
            .collect()
    }
    fn compile(&self, context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let output = compile_value_cell_register(&self.output, context)?;
        let first = compile_value_cell_register(&self.inputs[0], context)?;
        let name = if matches!(self.operation, Operation::Format) {
            if self.matrix {
                "NumberFormatMatrixCanonical"
            } else {
                "NumberFormatScalarCanonical"
            }
        } else {
            self.operation.name()
        };
        let function = context.function_id(name)?;
        if matches!(self.operation, Operation::Format) {
            let second = compile_value_cell_register(&self.inputs[1], context)?;
            context.emit_binop(function, output, first, second);
        } else {
            context.emit_unop(function, output, first);
        }
        Ok(output)
    }
}

macro_rules! factory {
    ($type:ident, $op:ident, $matrix:expr, $signature:expr) => {
        struct $type;
        impl MechFunctionFactory for $type {
            const SIGNATURE: RuntimeFunctionSignature = $signature;
            fn implementation_memory_class() -> ImplementationMemoryClass {
                ImplementationMemoryClass::CanonicalFinalize
            }
            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                PureTextRuntime::new(Operation::$op, invocation)
            }
            fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
                Some(Operation::$op.contract($matrix))
            }
        }
    };
}
factory!(
    ParseRuntime,
    Parse,
    false,
    RuntimeFunctionSignature::unary(
        FunctionValueRepresentation::AnyValue,
        FunctionValueRepresentation::String
    )
);
factory!(
    FormatScalarRuntime,
    Format,
    false,
    RuntimeFunctionSignature::binary(
        FunctionValueRepresentation::String,
        FunctionValueRepresentation::AnyValue,
        FunctionValueRepresentation::AnyValue
    )
);
factory!(
    FormatMatrixRuntime,
    Format,
    true,
    RuntimeFunctionSignature::binary(
        FunctionValueRepresentation::Matrix {
            element: FunctionMatrixElement::String,
            storage: FunctionMatrixStoragePattern::AnyStorage,
        },
        FunctionValueRepresentation::AnyValue,
        FunctionValueRepresentation::AnyValue
    )
);
factory!(
    LengthRuntime,
    Length,
    false,
    RuntimeFunctionSignature::unary(
        FunctionValueRepresentation::AnyValue,
        FunctionValueRepresentation::AnyValue
    )
);

macro_rules! registration {
    ($register:ident, $install:ident, $factory:ty, $factory_name:literal, $name:literal, $validator:ident) => {
        mech_core::declare_native_runtime_factory! {
            cfg: feature = "pure_text",
            registration: $register, installer: $install,
            name: $factory_name, factory_type: $factory,
            contract: RuntimeFunctionContract::canonical_custom($name, RuntimeOutputAliasPolicy::DisallowInputAlias, $validator),
            operations: [OperationId::from_name($name)],
            package: "mech-engine", crate_name: "mech_engine",
            installer_path: concat!("mech_engine::__mech_native::", stringify!($install)),
            extra_cargo_features: ["pure_text"],
        }
    }
}
registration!(
    register_parse_u64,
    install_parse_u64,
    ParseRuntime,
    "number/parse-u64",
    "number/parse-u64",
    validate_parse
);
registration!(
    register_number_format_scalar,
    install_number_format_scalar,
    FormatScalarRuntime,
    "NumberFormatScalarCanonical",
    "number/format",
    validate_format_scalar
);
registration!(
    register_number_format_matrix,
    install_number_format_matrix,
    FormatMatrixRuntime,
    "NumberFormatMatrixCanonical",
    "number/format",
    validate_format_matrix
);
registration!(
    register_sequence_length,
    install_sequence_length,
    LengthRuntime,
    "sequence/length",
    "sequence/length",
    validate_length
);

pub(crate) fn install_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    register_parse_u64(builder)?;
    register_number_format_scalar(builder)?;
    register_number_format_matrix(builder)?;
    register_sequence_length(builder)
}

#[cfg(feature = "semantic-compiler")]
struct SourceOperation(Operation);
#[cfg(feature = "semantic-compiler")]
impl CanonicalFunctionSpecializer for SourceOperation {
    fn specialize_invocation(
        &self,
        invocation: &SpecializationInvocation,
        context: &mut SpecializationContext<'_>,
    ) -> MResult<SpecializedFunction> {
        if invocation.len() != self.0.arity() {
            return Err(invalid("pure text operation arity mismatch"));
        }
        let inputs = invocation.inputs().iter().collect::<Vec<_>>();
        let extents = if matches!(self.0, Operation::Format) {
            inputs[0]
                .cell()?
                .resolved_descriptor()?
                .current_extents()
                .map_err(MechError::from)?
        } else {
            Box::new([])
        };
        context.bind_resolved_runtime(
            RuntimeBindingSelector::Operation(OperationId::from_name(self.0.name())),
            ExecutionTarget::DirectRuntime,
            vec![extents].into_boxed_slice(),
            &inputs,
        )
    }
}
#[cfg(feature = "semantic-compiler")]
pub(crate) fn install_source(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    for (operation, module, item) in [
        (Operation::Parse, "number", "parse-u64"),
        (Operation::Format, "number", "format"),
        (Operation::Length, "sequence", "length"),
    ] {
        let name = operation.name();
        let declaration = mech_core::maintained_source_type_declaration(name)?;
        let contracts = if matches!(operation, Operation::Format) {
            vec![
                operation.contract(false).clone(),
                operation.contract(true).clone(),
            ]
        } else {
            vec![operation.contract(false).clone()]
        };
        let id = builder.insert_canonical_specializer_with_contracts(
            name,
            declaration,
            contracts,
            std::sync::Arc::new(SourceOperation(operation)),
        )?;
        builder.insert_export(FunctionExport {
            operation: id,
            canonical_name: name.to_owned(),
            module: Some(module.to_owned()),
            item: Some(item.to_owned()),
            exposure: FunctionExposure::ModuleOnly,
        })?;
    }
    Ok(())
}
