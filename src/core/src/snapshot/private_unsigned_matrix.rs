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

/// Owned packed cells and the exact future immutable wrapper. Constructing this
/// owner performs all fallible work. `finish` neither allocates nor admits work.
/// Dropping it before publication releases cells and their original charge.
pub struct PrivateUnsignedMatrix {
    value: Value,
    ownership: SnapshotBudgetOwnership,
    append: Option<PrivateUnsignedAppendPreparation>,
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
        let budget = reservation.budget();
        let (required_bytes, nodes) = value.memory_budget_ownership_witness()?;
        let admitted_bytes =
            value.unshared_memory_budget_admission_for_required(&budget, required_bytes)?;
        if admitted_bytes > reservation.capacity_bytes() {
            return Err(crate::MemoryRuntimeError::BudgetExceeded {
                operation: None,
                requested: admitted_bytes,
                limit: reservation.capacity_bytes(),
            }
            .into());
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
            append: None,
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
            unreachable!("checked packed matrix")
        };
        matrix.elements()
    }

    pub fn elements_mut(&mut self) -> UnsignedMatrixViewMut<'_> {
        let root = Arc::get_mut(&mut self.value.root).expect("private wrapper has no aliases");
        let data = Arc::get_mut(&mut root.data).expect("private data has no aliases or identities");
        let ValueData::Matrix(matrix) = &mut data.data else {
            unreachable!("checked packed matrix")
        };
        match &mut matrix.elements {
            SequenceStorage::U8(words) => UnsignedMatrixViewMut::U8(words),
            SequenceStorage::U16(words) => UnsignedMatrixViewMut::U16(words),
            SequenceStorage::U32(words) => UnsignedMatrixViewMut::U32(words),
            SequenceStorage::U64(words) => UnsignedMatrixViewMut::U64(words),
            _ => unreachable!("checked plain unsigned kind"),
        }
    }

    pub fn memory_budget(&self) -> crate::ManagedMemoryBudget {
        self.ownership.reservation.budget()
    }

    pub fn retained_capacity_bytes(&self) -> u64 {
        self.ownership.reservation.capacity_bytes()
            + self.append.as_ref().map_or(0, |a| a.retained_bytes())
    }

    /// Publish exactly once after all private borrows end. All storage, node
    /// and byte claims already exist. Fresh identities/import links are exposed
    /// only after the initialized cells have become immutable.
    pub fn finish(mut self) -> Value {
        self.finish_append_shape();
        let Self {
            mut value,
            ownership,
            append: _,
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
