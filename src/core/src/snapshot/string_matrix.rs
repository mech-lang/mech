//! Compact construction of the existing canonical packed String matrix.
//! Payloads are already valid owned UTF-8; callers admit allocation before
//! creating the boxed sequence. Schema, shape and cardinality stay checked.
use super::*;

#[doc(hidden)]
#[derive(Clone, Debug)]
pub struct StringMatrixSnapshotConstructor {
    schema: SchemaId,
    key: SchemaKey,
    shape: ShapeInstance,
    count: usize,
    schemas: Arc<SchemaTable>,
}
impl StringMatrixSnapshotConstructor {
    pub fn bind(
        schema: SchemaId,
        shape: ShapeInstance,
        schemas: Arc<SchemaTable>,
    ) -> Result<Self, SnapshotValueError> {
        let entry = schemas
            .entry(schema)
            .ok_or(SnapshotValueError::UnknownSnapshotSchema { schema })?;
        let mismatch = || SnapshotValueError::SnapshotSchemaDefinitionMismatch { key: entry.key() };
        let checked = entry
            .schema()
            .instantiate_shape(shape.parameter_values().to_vec().into_boxed_slice())
            .map_err(|_| mismatch())?;
        if checked != shape {
            return Err(mismatch());
        }
        let SchemaBody::Matrix {
            element,
            dimensions,
        } = entry.schema().closed_body(&shape).map_err(|_| mismatch())?
        else {
            return Err(mismatch());
        };
        if element.as_ref() != &SchemaBody::String {
            return Err(mismatch());
        }
        let count = dimensions.iter().try_fold(1u64, |count, dimension| {
            count
                .checked_mul(shape.resolve_dimension(dimension)?)
                .ok_or_else(mismatch)
        })?;
        Ok(Self {
            schema,
            key: entry.key(),
            shape,
            count: usize::try_from(count).map_err(|_| mismatch())?,
            schemas,
        })
    }

    /// Consume exactly the admitted owned sequence; no per-cell generic draft,
    /// temporary ValueData array, payload clone or unchecked dynamic cell.
    pub fn construct(self, strings: Box<[Box<str>]>) -> Result<Value, SnapshotValueError> {
        if strings.len() != self.count {
            return Err(SnapshotValueError::AggregateArityMismatchV1 {
                path: SnapshotPath::root(),
                expected: self.count as u64,
                actual: strings.len() as u64,
            });
        }
        Ok(finalized_value(
            self.schema,
            self.key,
            self.shape,
            ValueData::Matrix(MatrixValue {
                elements: SequenceStorage::String(strings),
            }),
            Some(self.schemas),
        ))
    }
}
