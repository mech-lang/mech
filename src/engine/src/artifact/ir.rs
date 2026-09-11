use mech_core::snapshot::{OptionDraft, SnapshotValidationContext};
use mech_core::{
    CellSlotId, ConstantId, ConstantStore, SchemaBody, SchemaDraft, SchemaId, SchemaTable,
    SnapshotValueError, Value, ValueDataDraft, ValueDraft,
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
    UnknownSchema {
        schema: SchemaId,
    },
    UnknownConstant {
        constant: ConstantId,
    },
    MatrixLiteralSchemaNotMatrix,
    MatrixLiteralShapeMismatch {
        expected_rows: u64,
        expected_columns: u64,
        actual_elements: usize,
    },
    MatrixLiteralSchemaShapeMismatch {
        ir_rows: u64,
        ir_columns: u64,
    },
    MatrixLiteralCardinalityOverflow,
    EmptyMatrixLiteralElementRequiresOption {
        index: usize,
    },
    HeterogeneousMatrixLiteral {
        index: usize,
    },
    UnresolvedMatrixLiteralElement {
        index: usize,
    },
    DynamicEmptyMatrixLiteralUnsupported {
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

impl MatrixLiteralIR {
    pub fn contains_slots(&self) -> bool {
        self.elements.iter().any(expression_contains_slot)
    }

    pub fn contains_empty(&self) -> bool {
        self.elements.iter().any(expression_contains_empty)
    }

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
        self.resolve_constant_with_shape(schema, Box::new([]), schemas, constants)
    }

    /// Resolves a constant matrix using the already-certified current shape
    /// for a schema whose permitted axes may remain dynamic.
    pub fn resolve_constant_with_shape(
        &self,
        schema: SchemaId,
        shape_values: Box<[u64]>,
        schemas: &SchemaTable,
        constants: &ConstantStore,
    ) -> Result<Value, CompilerIrError> {
        let schema_definition = schemas
            .get(schema)
            .ok_or(CompilerIrError::UnknownSchema { schema })?;
        let shape = schema_definition
            .instantiate_shape(shape_values.clone())
            .map_err(|error| CompilerIrError::Snapshot(error.into()))?;
        let SchemaBody::Matrix {
            element,
            dimensions,
        } = schema_definition.body()
        else {
            return Err(CompilerIrError::MatrixLiteralSchemaNotMatrix);
        };
        let expected_elements = self
            .rows
            .checked_mul(self.columns)
            .ok_or(CompilerIrError::MatrixLiteralCardinalityOverflow)?;
        if usize::try_from(expected_elements).ok() != Some(self.elements.len()) {
            return Err(CompilerIrError::MatrixLiteralShapeMismatch {
                expected_rows: self.rows,
                expected_columns: self.columns,
                actual_elements: self.elements.len(),
            });
        }
        let [expected_rows, expected_columns] = dimensions.as_ref() else {
            return Err(CompilerIrError::MatrixLiteralSchemaShapeMismatch {
                ir_rows: self.rows,
                ir_columns: self.columns,
            });
        };
        let expected_rows = shape
            .resolve_dimension(expected_rows)
            .map_err(|error| CompilerIrError::Snapshot(error.into()))?;
        let expected_columns = shape
            .resolve_dimension(expected_columns)
            .map_err(|error| CompilerIrError::Snapshot(error.into()))?;
        if (expected_rows, expected_columns) != (self.rows, self.columns) {
            return Err(CompilerIrError::MatrixLiteralSchemaShapeMismatch {
                ir_rows: self.rows,
                ir_columns: self.columns,
            });
        }

        let element_schema = SchemaDraft {
            dimension_parameters: Box::new([]),
            body: (**element).clone(),
        }
        .finalize()
        .map_err(|error| CompilerIrError::Snapshot(error.into()))?;
        let expected_key = element_schema.key();
        let option_element = match element_schema.body() {
            SchemaBody::Option(element) => Some(
                SchemaDraft {
                    dimension_parameters: Box::new([]),
                    body: (**element).clone(),
                }
                .finalize()
                .map_err(|error| CompilerIrError::Snapshot(error.into()))?,
            ),
            _ => None,
        };
        let mut values = Vec::with_capacity(self.elements.len());
        for (index, expression) in self.elements.iter().enumerate() {
            match expression {
                ExpressionIR::Empty => {
                    if option_element.is_none() {
                        return Err(CompilerIrError::EmptyMatrixLiteralElementRequiresOption {
                            index,
                        });
                    }
                    values.push(ValueDataDraft::Option(OptionDraft {
                        present: false,
                        value: None,
                    }));
                }
                ExpressionIR::Constant(constant) => {
                    let value =
                        constants
                            .get(*constant)
                            .ok_or(CompilerIrError::UnknownConstant {
                                constant: *constant,
                            })?;
                    if value.schema_key() == expected_key {
                        values.push(
                            data_draft(value.data(), element_schema.body()).ok_or(
                                CompilerIrError::MatrixLiteralElementUnsupported { index },
                            )?,
                        );
                    } else if let Some(option_element) = &option_element {
                        if value.schema_key() != option_element.key() {
                            return Err(CompilerIrError::HeterogeneousMatrixLiteral { index });
                        }
                        values.push(ValueDataDraft::Option(OptionDraft {
                            present: true,
                            value: Some(Box::new(
                                data_draft(value.data(), option_element.body()).ok_or(
                                    CompilerIrError::MatrixLiteralElementUnsupported { index },
                                )?,
                            )),
                        }));
                    } else {
                        return Err(CompilerIrError::HeterogeneousMatrixLiteral { index });
                    }
                }
                ExpressionIR::Slot(_)
                | ExpressionIR::MatrixLiteral(_)
                | ExpressionIR::Selection(_) => {
                    return Err(CompilerIrError::UnresolvedMatrixLiteralElement { index });
                }
            }
        }

        Ok(ValueDraft {
            schema,
            shape_values,
            data: ValueDataDraft::Matrix(values.into_boxed_slice()),
        }
        .finalize(&SnapshotValidationContext::new(schemas))?)
    }
}

fn expression_contains_slot(expression: &ExpressionIR) -> bool {
    match expression {
        ExpressionIR::Slot(_) => true,
        ExpressionIR::MatrixLiteral(literal) => literal.contains_slots(),
        ExpressionIR::Selection(selection) => {
            selection_contains(selection, expression_contains_slot)
        }
        ExpressionIR::Empty | ExpressionIR::Constant(_) => false,
    }
}

fn expression_contains_empty(expression: &ExpressionIR) -> bool {
    match expression {
        ExpressionIR::Empty => true,
        ExpressionIR::MatrixLiteral(literal) => literal.contains_empty(),
        ExpressionIR::Selection(selection) => {
            selection_contains(selection, expression_contains_empty)
        }
        ExpressionIR::Constant(_) | ExpressionIR::Slot(_) => false,
    }
}

fn selection_contains(selection: &SelectionIR, predicate: fn(&ExpressionIR) -> bool) -> bool {
    match selection {
        SelectionIR::All => false,
        SelectionIR::Index(index) => predicate(index),
        SelectionIR::Range { start, end, step } => [start, end, step]
            .into_iter()
            .flatten()
            .any(|expression| predicate(expression)),
    }
}
