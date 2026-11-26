//! A fixed-shape private phase with an already admitted publication owner.
//! No old frozen identity survives the exclusive handoff. No Value, weak data
//! identity, or import registration escapes before the private phase ends.
use super::*;
use crate::snapshot::SequenceView;
#[path = "private_unsigned_append.rs"]
mod appends;
pub use appends::PrivateUnsignedAppendPreparation;

/// Mutable initialized cells; this surface cannot grow, truncate or change kind.
#[derive(Debug)]
pub enum UnsignedMatrixViewMut<'a> {
    U8(&'a mut [u8]),
    U16(&'a mut [u16]),
    U32(&'a mut [u32]),
    U64(&'a mut [u64]),
}

/// Exact initialized scalar cells. This borrow cannot grow or change kind.
#[derive(Debug)]
pub enum NumericMatrixViewMut<'a> {
    Bool(&'a mut [bool]),
    U8(&'a mut [u8]),
    U16(&'a mut [u16]),
    U32(&'a mut [u32]),
    U64(&'a mut [u64]),
    I64(&'a mut [i64]),
    F64(&'a mut [crate::snapshot::F64Bits]),
}

/// One unexposed publication owner shared by all exact plain scalar kinds.
/// All fallible admission completes before mutable initialized cells escape.
/// No immutable Value or frozen identity is exposed until `finish`.
pub struct PrivateNumericMatrix {
    value: Value,
    ownership: SnapshotBudgetOwnership,
}

/// Unsigned growth retains its narrower preparation and initialization API.
/// Its fixed-shape owner shares the scalar publication and account machinery.
pub struct PrivateUnsignedMatrix {
    numeric: PrivateNumericMatrix,
    append: Option<PrivateUnsignedAppendPreparation>,
}

impl PrivateNumericMatrix {
    fn admit(
        value: Value,
        reservation: &mut crate::ManagedMemoryReservation,
    ) -> crate::MemoryRuntimeResult<Self> {
        let schemas = value
            .schemas
            .as_ref()
            .expect("checked private schema")
            .clone();
        let budget = reservation.budget();
        let (required_bytes, nodes) = value.memory_budget_ownership_witness()?;
        let admitted_bytes =
            value.unshared_memory_budget_admission_for_required(&budget, required_bytes)?;
        if admitted_bytes > reservation.capacity_bytes() {
            return Err(crate::MemoryRuntimeError::BudgetExceeded {
                operation: None,
                requested: admitted_bytes,
                limit: reservation.capacity_bytes(),
            });
        }
        let bytes = Value::budget_registration_allocation_bytes();
        budget.check_snapshot_import_allocation(
            bytes,
            core::mem::align_of::<SnapshotBudgetRegistration>() as u32,
        )?;
        let registration = Arc::try_new(SnapshotBudgetRegistration {
            budget: budget.clone(),
            schemas: Arc::downgrade(&schemas),
            shape: value.shape.clone(),
            owner: SharedOwnershipCell::new(),
            links: SnapshotBudgetGate::default(),
        })
        .map_err(|_| crate::MemoryRuntimeError::AllocationFailed {
            object: None,
            requested: bytes,
            alignment: core::mem::align_of::<SnapshotBudgetRegistration>() as u32,
            space: crate::MemorySpace::Host,
        })?;
        let node_charge = budget.retain_snapshot_nodes(nodes)?;
        let reservation = reservation.split_capacity(admitted_bytes)?;
        Ok(Self {
            value,
            ownership: SnapshotBudgetOwnership {
                registration,
                schemas,
                required_bytes,
                reservation,
                node_charge,
            },
        })
    }

    pub fn elements(&self) -> SequenceView<'_> {
        let ValueData::Matrix(matrix) = self.value.data() else {
            unreachable!("checked packed scalar matrix")
        };
        matrix.elements()
    }

    pub fn elements_mut(&mut self) -> NumericMatrixViewMut<'_> {
        let root = Arc::get_mut(&mut self.value.root).expect("private wrapper has no aliases");
        let data = Arc::get_mut(&mut root.data).expect("private data has no aliases or identities");
        let ValueData::Matrix(matrix) = &mut data.data else {
            unreachable!("checked packed scalar matrix")
        };
        match &mut matrix.elements {
            SequenceStorage::Bool(v) => NumericMatrixViewMut::Bool(v),
            SequenceStorage::U8(v) => NumericMatrixViewMut::U8(v),
            SequenceStorage::U16(v) => NumericMatrixViewMut::U16(v),
            SequenceStorage::U32(v) => NumericMatrixViewMut::U32(v),
            SequenceStorage::U64(v) => NumericMatrixViewMut::U64(v),
            SequenceStorage::I64(v) => NumericMatrixViewMut::I64(v),
            SequenceStorage::F64(v) => NumericMatrixViewMut::F64(v),
            _ => unreachable!("checked plain scalar kind"),
        }
    }

    pub fn memory_budget(&self) -> crate::ManagedMemoryBudget {
        self.ownership.reservation.budget()
    }

    pub fn retained_capacity_bytes(&self) -> u64 {
        self.ownership.reservation.capacity_bytes()
    }

    /// Publish exactly once after all private borrows end. All storage, node
    /// and byte claims already exist. Fresh identities/import links are exposed
    /// only after the initialized cells have become immutable.
    pub fn finish(self) -> Value {
        let Self {
            mut value,
            ownership,
        } = self;
        let root = Arc::get_mut(&mut value.root).expect("private wrapper has no aliases");
        let data = Arc::get_mut(&mut root.data).expect("private data has no aliases or identities");
        // No data identity or publication escapes the private phase. Exclusive
        // access initializes the already-admitted registry without allocating
        // a platform lock. Install it before exposing the wrapper's weak owner.
        with_exclusive_budget_gate(&mut data.budget_imports, |imports| {
            debug_assert!(imports.is_none());
            *imports = Some(ownership.registration.clone());
        });
        root.memory_budget = Some(ownership);
        let _ = value
            .root
            .memory_budget
            .as_ref()
            .expect("private publication retains its admission")
            .registration
            .owner
            .set(Arc::downgrade(&value.root));
        value
    }
}

impl PrivateUnsignedMatrix {
    pub(super) fn prepare(
        schema: SchemaId,
        shape: ShapeInstance,
        words: UnsignedMatrixBuffer,
        schemas: Arc<SchemaTable>,
        mut reservation: crate::ManagedMemoryReservation,
    ) -> Result<Self, SnapshotValueError> {
        // This checks exact plain unsigned kind, shape and cardinality. The
        // fresh wrapper/data owners are unregistered and cannot have weak or
        // strong aliases; the Value remains confined inside this private type.
        let value = Value::from_unsigned_matrix_buffer(schema, shape, words, schemas.clone())?;
        Ok(Self {
            numeric: PrivateNumericMatrix::admit(value, &mut reservation)?,
            append: None,
        })
    }

    pub fn elements(&self) -> SequenceView<'_> {
        self.numeric.elements()
    }

    pub fn elements_mut(&mut self) -> UnsignedMatrixViewMut<'_> {
        match self.numeric.elements_mut() {
            NumericMatrixViewMut::U8(v) => UnsignedMatrixViewMut::U8(v),
            NumericMatrixViewMut::U16(v) => UnsignedMatrixViewMut::U16(v),
            NumericMatrixViewMut::U32(v) => UnsignedMatrixViewMut::U32(v),
            NumericMatrixViewMut::U64(v) => UnsignedMatrixViewMut::U64(v),
            _ => unreachable!("checked plain unsigned kind"),
        }
    }

    pub fn memory_budget(&self) -> crate::ManagedMemoryBudget {
        self.numeric.memory_budget()
    }

    pub fn retained_capacity_bytes(&self) -> u64 {
        self.numeric.retained_capacity_bytes()
            + self.append.as_ref().map_or(0, |a| a.retained_bytes())
    }

    pub fn finish(mut self) -> Value {
        self.finish_append_shape();
        self.numeric.finish()
    }
}

impl Value {
    /// Confine a fresh, exclusive unadmitted scalar matrix. Existing immutable
    /// aliases, admitted imports and frozen/weak data identities cannot enter.
    pub fn into_private_numeric_matrix(
        self,
        reservation: &mut crate::ManagedMemoryReservation,
    ) -> crate::MemoryRuntimeResult<PrivateNumericMatrix> {
        self.into_private_matrix_checked(reservation, false)
    }

    /// The maintained unsigned surface still rejects Bool/I64/F64 matrices.
    pub fn into_private_unsigned_matrix(
        self,
        reservation: &mut crate::ManagedMemoryReservation,
    ) -> crate::MemoryRuntimeResult<PrivateUnsignedMatrix> {
        Ok(PrivateUnsignedMatrix {
            numeric: self.into_private_matrix_checked(reservation, true)?,
            append: None,
        })
    }

    fn into_private_matrix_checked(
        mut self,
        reservation: &mut crate::ManagedMemoryReservation,
        unsigned: bool,
    ) -> crate::MemoryRuntimeResult<PrivateNumericMatrix> {
        let plain = self.schemas.as_ref().and_then(|s| s.get(self.schema)).is_some_and(|s| {
            if unsigned {
                matches!(s.body(), SchemaBody::Matrix { element, .. }
                    if matches!(element.as_ref(), SchemaBody::UnsignedInteger(
                        IntegerWidth::W8 | IntegerWidth::W16 | IntegerWidth::W32 | IntegerWidth::W64)))
            } else {
                super::numeric_matrix::plain_numeric_matrix(s.body())
            }
        });
        let invalid = || crate::MemoryRuntimeError::CandidateValidationFailed {
            object: None,
            reason: "private matrix requires an exact fresh exclusive unadmitted scalar owner"
                .into(),
        };
        if !plain || self.root.memory_budget.is_some() {
            return Err(invalid());
        }
        let root = Arc::get_mut(&mut self.root).ok_or_else(invalid)?;
        let data = Arc::get_mut(&mut root.data).ok_or_else(invalid)?;
        let packed = matches!(
            &data.data,
            ValueData::Matrix(MatrixValue {
                elements: SequenceStorage::U8(_)
                    | SequenceStorage::U16(_)
                    | SequenceStorage::U32(_)
                    | SequenceStorage::U64(_)
            })
        ) || (!unsigned
            && matches!(
                &data.data,
                ValueData::Matrix(MatrixValue {
                    elements: SequenceStorage::Bool(_)
                        | SequenceStorage::I64(_)
                        | SequenceStorage::F64(_)
                })
            ));
        if data.ownership.get().is_some() || !packed {
            return Err(invalid());
        }
        data.resident_token.take();
        data.string_ascii_without_crlf.take();
        PrivateNumericMatrix::admit(self, reservation)
    }
}
