//! Exclusive packed matrix handoff, sharing the immutable-root transfer rules.
use super::numeric_matrix::UnsignedMatrixBuffer;
use super::owned_string::OwnedSnapshotParts;
use super::*;

#[derive(Debug)]
pub struct OwnedSnapshotUnsignedMatrix {
    words: UnsignedMatrixBuffer,
    schemas: Arc<SchemaTable>,
    reservation: crate::ManagedMemoryReservation,
    schema: SchemaId,
    key: SchemaKey,
}

#[derive(Debug)]
pub enum UnsignedMatrixOwnershipTransfer {
    Unavailable(Value),
    Owned(OwnedSnapshotUnsignedMatrix),
}

impl OwnedSnapshotUnsignedMatrix {
    /// Prepare one private, fixed-shape update phase and its final frozen owner.
    /// Every fallible allocation/admission finishes before mutable cells escape.
    /// The result is not an immutable Value until it is consumed by `finish`.
    pub fn into_private_updates(
        self,
        shape: ShapeInstance,
    ) -> Result<super::PrivateUnsignedMatrix, SnapshotValueError> {
        let Self {
            words,
            schemas,
            reservation,
            schema,
            ..
        } = self;
        super::PrivateUnsignedMatrix::prepare(schema, shape, words, schemas, reservation)
    }

    pub fn elements(&self) -> super::super::SequenceView<'_> {
        self.words.elements()
    }
    pub fn capacity_bytes(&self) -> usize {
        self.words.capacity_bytes()
    }
    pub const fn schema(&self) -> SchemaId {
        self.schema
    }
    pub const fn schema_key(&self) -> SchemaKey {
        self.key
    }
    pub fn memory_budget(&self) -> crate::ManagedMemoryBudget {
        self.reservation.budget()
    }
    pub fn retained_capacity_bytes(&self) -> u64 {
        self.reservation.capacity_bytes()
    }
    /// Keep the returned charge until the backing retires or is handed to an
    /// admitted owner. Mutating detached words cannot mutate any prior Value.
    pub fn into_parts(self) -> (UnsignedMatrixBuffer, crate::ManagedMemoryReservation) {
        let Self {
            words,
            schemas,
            reservation,
            ..
        } = self;
        drop(schemas);
        (words, reservation)
    }
}

impl Value {
    /// Consume same-account exclusive packed words without copying. Source
    /// liveness is the compiled caller's separate obligation. Clones, imports,
    /// physical payload tickets and interval-constrained kinds stay immutable.
    pub fn try_into_owned_unsigned_matrix(
        self,
        budget: &crate::ManagedMemoryBudget,
    ) -> crate::MemoryRuntimeResult<UnsignedMatrixOwnershipTransfer> {
        let plain = self
            .schemas
            .as_ref()
            .and_then(|table| table.get(self.schema))
            .is_some_and(|schema| {
                matches!(schema.body(), SchemaBody::Matrix { element, .. }
                if matches!(element.as_ref(), SchemaBody::UnsignedInteger(
                    IntegerWidth::W8 | IntegerWidth::W16 | IntegerWidth::W32 | IntegerWidth::W64)))
            });
        if !plain
            || !matches!(
                self.data(),
                ValueData::Matrix(MatrixValue {
                    elements: SequenceStorage::U8(_)
                        | SequenceStorage::U16(_)
                        | SequenceStorage::U32(_)
                        | SequenceStorage::U64(_)
                })
            )
        {
            return Ok(UnsignedMatrixOwnershipTransfer::Unavailable(self));
        }
        match self.try_into_owned_snapshot(budget)? {
            Err(value) => Ok(UnsignedMatrixOwnershipTransfer::Unavailable(value)),
            Ok(OwnedSnapshotParts {
                data,
                schemas,
                reservation,
                schema,
                key,
            }) => {
                let ValueData::Matrix(matrix) = data else {
                    unreachable!("checked matrix kind")
                };
                let words = match matrix.elements {
                    SequenceStorage::U8(words) => UnsignedMatrixBuffer::U8(words),
                    SequenceStorage::U16(words) => UnsignedMatrixBuffer::U16(words),
                    SequenceStorage::U32(words) => UnsignedMatrixBuffer::U32(words),
                    SequenceStorage::U64(words) => UnsignedMatrixBuffer::U64(words),
                    _ => unreachable!("checked packed kind"),
                };
                Ok(UnsignedMatrixOwnershipTransfer::Owned(
                    OwnedSnapshotUnsignedMatrix {
                        words,
                        schemas,
                        reservation,
                        schema,
                        key,
                    },
                ))
            }
        }
    }
}

#[cfg(test)]
#[path = "owned_unsigned_matrix_tests.rs"]
mod tests;
