use mech_core::snapshot::SnapshotValidationContext;
use mech_core::{
    CellSlotId, ConstantId, ConstantStore, DimensionExpr, LegacyValue, SchemaBody, SchemaDraft,
    SchemaId, SchemaTable, SnapshotValueError, Value, ValueDataDraft, ValueDraft,
};

use super::snapshot::data_draft;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExpressionIR {
    Empty,
    Constant(ConstantId),
    Slot(CellSlotId),
    MatrixLiteral(MatrixLiteralIR),
    Selection(SelectionIR),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MatrixLiteralIR {
    pub rows: u64,
    pub columns: u64,
    pub elements: Box<[ExpressionIR]>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SelectionIR {
    All,
    Index(Box<ExpressionIR>),
    Range {
        start: Option<Box<ExpressionIR>>,
        end: Option<Box<ExpressionIR>>,
        step: Option<Box<ExpressionIR>>,
    },
}

#[derive(Debug)]
pub enum CompilerIrError {
    PseudoValueNotCompilerIr,
    UnknownSchema {
        schema: SchemaId,
    },
    UnknownConstant {
        constant: ConstantId,
    },
    MatrixSchemaRequired {
        schema: SchemaId,
    },
    DynamicMatrixSchemaUnsupported {
        schema: SchemaId,
    },
    MatrixLiteralRankMismatch {
        schema: SchemaId,
    },
    MatrixLiteralShapeMismatch {
        expected_rows: u64,
        expected_columns: u64,
        found_rows: u64,
        found_columns: u64,
    },
    MatrixLiteralCardinalityOverflow,
    MatrixLiteralElementCount {
        expected: u64,
        found: usize,
    },
    LegacyMatrixLiteralRank {
        rank: usize,
    },
    MatrixLiteralElementNotConstant {
        index: usize,
    },
    HeterogeneousMatrixLiteral {
        index: usize,
    },
    MatrixLiteralElementUnsupported {
        index: usize,
    },
    Snapshot(SnapshotValueError),
}

impl From<SnapshotValueError> for CompilerIrError {
    fn from(error: SnapshotValueError) -> Self {
        Self::Snapshot(error)
    }
}

/// Moves legacy parser pseudo-values into compiler-only IR.
///
/// Ordinary runtime values must be inserted into `ConstantStore` first and
/// represented by `ExpressionIR::Constant`; they are deliberately rejected
/// here.
pub fn compiler_ir_from_legacy_pseudo_value(
    value: &LegacyValue,
) -> Result<ExpressionIR, CompilerIrError> {
    match value {
        LegacyValue::Empty => Ok(ExpressionIR::Empty),
        LegacyValue::IndexAll => Ok(ExpressionIR::Selection(SelectionIR::All)),
        _ => Err(CompilerIrError::PseudoValueNotCompilerIr),
    }
}

/// Moves the legacy heterogeneous matrix container into compiler-only literal
/// IR. Elements have already been lowered to expressions; only the source
/// matrix's deterministic rank-2 shape is retained.
#[cfg(feature = "matrix")]
pub fn matrix_literal_ir_from_legacy(
    value: &LegacyValue,
    elements: Box<[ExpressionIR]>,
) -> Result<MatrixLiteralIR, CompilerIrError> {
    let LegacyValue::MatrixValue(matrix) = value else {
        return Err(CompilerIrError::PseudoValueNotCompilerIr);
    };
    let shape = matrix.shape();
    let [rows, columns] = shape.as_slice() else {
        return Err(CompilerIrError::LegacyMatrixLiteralRank { rank: shape.len() });
    };
    let rows =
        u64::try_from(*rows).map_err(|_| CompilerIrError::MatrixLiteralCardinalityOverflow)?;
    let columns =
        u64::try_from(*columns).map_err(|_| CompilerIrError::MatrixLiteralCardinalityOverflow)?;
    let expected = rows
        .checked_mul(columns)
        .ok_or(CompilerIrError::MatrixLiteralCardinalityOverflow)?;
    if usize::try_from(expected).ok() != Some(elements.len()) {
        return Err(CompilerIrError::MatrixLiteralElementCount {
            expected,
            found: elements.len(),
        });
    }
    Ok(MatrixLiteralIR {
        rows,
        columns,
        elements,
    })
}

impl MatrixLiteralIR {
    /// Resolves a fully constant, homogeneous matrix literal into one final
    /// immutable matrix snapshot.
    ///
    /// Slot and selection expressions remain compiler IR until graph lowering;
    /// they cannot be smuggled into a final heterogeneous matrix value.
    pub fn resolve_constant(
        &self,
        schema: SchemaId,
        schemas: &SchemaTable,
        constants: &ConstantStore,
    ) -> Result<Value, CompilerIrError> {
        let schema_definition = schemas
            .get(schema)
            .ok_or(CompilerIrError::UnknownSchema { schema })?;
        if !schema_definition.dimension_parameters().is_empty() {
            return Err(CompilerIrError::DynamicMatrixSchemaUnsupported { schema });
        }
        let SchemaBody::Matrix {
            element,
            dimensions,
        } = schema_definition.body()
        else {
            return Err(CompilerIrError::MatrixSchemaRequired { schema });
        };
        let [
            DimensionExpr::Constant(expected_rows),
            DimensionExpr::Constant(expected_columns),
        ] = dimensions.as_ref()
        else {
            return Err(CompilerIrError::MatrixLiteralRankMismatch { schema });
        };
        if (*expected_rows, *expected_columns) != (self.rows, self.columns) {
            return Err(CompilerIrError::MatrixLiteralShapeMismatch {
                expected_rows: *expected_rows,
                expected_columns: *expected_columns,
                found_rows: self.rows,
                found_columns: self.columns,
            });
        }
        let expected = self
            .rows
            .checked_mul(self.columns)
            .ok_or(CompilerIrError::MatrixLiteralCardinalityOverflow)?;
        if usize::try_from(expected).ok() != Some(self.elements.len()) {
            return Err(CompilerIrError::MatrixLiteralElementCount {
                expected,
                found: self.elements.len(),
            });
        }

        let element_schema = SchemaDraft {
            dimension_parameters: Box::new([]),
            body: (**element).clone(),
        }
        .finalize()
        .map_err(|error| CompilerIrError::Snapshot(error.into()))?;
        let expected_key = element_schema.key();
        let mut values = Vec::with_capacity(self.elements.len());
        for (index, expression) in self.elements.iter().enumerate() {
            let ExpressionIR::Constant(constant) = expression else {
                return Err(CompilerIrError::MatrixLiteralElementNotConstant { index });
            };
            let value = constants
                .get(*constant)
                .ok_or(CompilerIrError::UnknownConstant {
                    constant: *constant,
                })?;
            if value.schema_key() != expected_key {
                return Err(CompilerIrError::HeterogeneousMatrixLiteral { index });
            }
            values.push(
                data_draft(value.data(), element_schema.body())
                    .ok_or(CompilerIrError::MatrixLiteralElementUnsupported { index })?,
            );
        }

        Ok(ValueDraft {
            schema,
            shape_values: Box::new([]),
            data: ValueDataDraft::Matrix(values.into_boxed_slice()),
        }
        .finalize(&SnapshotValidationContext::new(schemas))?)
    }
}
