//! Plain scalar matrices use the same exclusive, same-account handoff as the
//! maintained unsigned and String paths. No immutable payload is mutated.
use super::numeric_matrix::{NumericMatrixBuffer, UnsignedMatrixBuffer, plain_numeric_matrix};
use super::owned_string::OwnedSnapshotParts;
use super::*;

#[derive(Debug)]
pub struct OwnedSnapshotNumericMatrix {
    buffer: NumericMatrixBuffer,
    schemas: Arc<SchemaTable>,
    reservation: crate::ManagedMemoryReservation,
}

#[derive(Debug)]
pub enum NumericMatrixOwnershipTransfer {
    Unavailable(Value),
    Owned(OwnedSnapshotNumericMatrix),
}

impl OwnedSnapshotNumericMatrix {
    /// Keep the transferred claim alive until the detached backing retires or
    /// is registered under a fresh canonical owner in this same account.
    pub fn into_parts(self) -> (NumericMatrixBuffer, crate::ManagedMemoryReservation) {
        let Self {
            buffer,
            schemas,
            reservation,
        } = self;
        drop(schemas);
        (buffer, reservation)
    }
}

impl Value {
    /// Consume only an exclusive plain packed matrix in the supplied account.
    /// Ordinary aliases, imports and physical payload tickets remain frozen.
    /// Source last use is still the compiled caller's independent obligation.
    pub fn try_into_owned_numeric_matrix(
        self,
        budget: &crate::ManagedMemoryBudget,
    ) -> crate::MemoryRuntimeResult<NumericMatrixOwnershipTransfer> {
        let plain = self
            .schemas
            .as_ref()
            .and_then(|table| table.get(self.schema))
            .is_some_and(|schema| plain_numeric_matrix(schema.body()));
        if !plain
            || !matches!(
                self.data(),
                ValueData::Matrix(MatrixValue {
                    elements: SequenceStorage::Bool(_)
                        | SequenceStorage::U8(_)
                        | SequenceStorage::U16(_)
                        | SequenceStorage::U32(_)
                        | SequenceStorage::U64(_)
                        | SequenceStorage::I64(_)
                        | SequenceStorage::F64(_)
                })
            )
        {
            return Ok(NumericMatrixOwnershipTransfer::Unavailable(self));
        }
        match self.try_into_owned_snapshot(budget)? {
            Err(value) => Ok(NumericMatrixOwnershipTransfer::Unavailable(value)),
            Ok(OwnedSnapshotParts {
                data,
                schemas,
                reservation,
                ..
            }) => {
                let ValueData::Matrix(matrix) = data else {
                    unreachable!("checked matrix kind")
                };
                let buffer = match matrix.elements {
                    SequenceStorage::U8(v) => {
                        NumericMatrixBuffer::Unsigned(UnsignedMatrixBuffer::U8(v))
                    }
                    SequenceStorage::U16(v) => {
                        NumericMatrixBuffer::Unsigned(UnsignedMatrixBuffer::U16(v))
                    }
                    SequenceStorage::U32(v) => {
                        NumericMatrixBuffer::Unsigned(UnsignedMatrixBuffer::U32(v))
                    }
                    SequenceStorage::U64(v) => {
                        NumericMatrixBuffer::Unsigned(UnsignedMatrixBuffer::U64(v))
                    }
                    SequenceStorage::Bool(v) => NumericMatrixBuffer::Bool(v),
                    SequenceStorage::I64(v) => NumericMatrixBuffer::I64(v),
                    SequenceStorage::F64(v) => NumericMatrixBuffer::F64(v),
                    _ => unreachable!("checked packed scalar kind"),
                };
                Ok(NumericMatrixOwnershipTransfer::Owned(
                    OwnedSnapshotNumericMatrix {
                        buffer,
                        schemas,
                        reservation,
                    },
                ))
            }
        }
    }
}

#[cfg(test)]
#[path = "owned_numeric_matrix_tests.rs"]
mod tests;
