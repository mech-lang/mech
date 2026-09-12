use super::sequence::SequenceStorage;
use super::{
    CanonicalKeyValue, DynamicValue, EnumDraft, EnumValue, MapEntryDraft, MapValue, MatrixValue,
    NamedValueDraft, OptionDraft, RecordValue, ReifiedKind, ReifiedType, ReifiedTypeDraft,
    SchemaDataKind, SetValue, SnapshotPath, SnapshotPathSegment, SnapshotValueError,
    TableColumnDraft, TableValue, ValueData, ValueDataDraft, ValueDraft,
};
use crate::{
    FloatWidth, IntegerWidth, NamedKindPathResolver, Schema, SchemaBody, SchemaId, SchemaKey,
    SchemaTable, ShapeInstance,
};
use core::cell::{Cell, OnceCell};

#[cfg(feature = "no_std")]
use alloc::{
    boxed::Box,
    string::String,
    sync::{Arc, Weak},
    vec::Vec,
};
#[cfg(not(feature = "no_std"))]
use std::{
    boxed::Box,
    string::String,
    sync::{Arc, Weak},
    vec::Vec,
};

#[cfg(all(feature = "no_std", not(feature = "std")))]
type SharedOwnershipCell<T> = core::cell::OnceCell<T>;
#[cfg(any(not(feature = "no_std"), feature = "std"))]
type SharedOwnershipCell<T> = std::sync::OnceLock<T>;

#[cfg(all(feature = "no_std", not(feature = "std")))]
type SnapshotBudgetRegistry = core::cell::RefCell<Option<Box<SnapshotBudgetRegistration>>>;
#[cfg(any(not(feature = "no_std"), feature = "std"))]
type SnapshotBudgetRegistry = std::sync::Mutex<Option<Box<SnapshotBudgetRegistration>>>;

pub struct SnapshotValidationContext<'a> {
    schemas: &'a SchemaTable,
    named_kinds: Option<&'a dyn NamedKindPathResolver>,
    canonicalization_budget: Option<&'a SnapshotCanonicalizationBudget>,
    construction_authority: Option<&'a dyn SnapshotConstructionAuthority>,
    shared_schemas: OnceCell<Arc<SchemaTable>>,
}

/// Sealed allocation authority used by managed canonical construction. The
/// ordinary detached-value API has no authority and retains its historical
/// behavior; R6 builders install this capability so every allocation made by
/// the common finalizer is checked before it is attempted.
pub(crate) trait SnapshotConstructionAuthority {
    fn admit_snapshot_allocation(
        &self,
        bytes: u64,
        alignment: u32,
    ) -> Result<(), crate::MemoryRuntimeError>;

    fn allocation_object(&self) -> Option<crate::MemoryObjectId>;
}

/// An incremental fail-closed limit for ordered set/map normalization during
/// snapshot finalization. The budget is shared by recursive finalization, so
/// nested collections cannot each restart the same allowance.
#[derive(Debug)]
pub struct SnapshotCanonicalizationBudget {
    limit: u64,
    consumed: Cell<u64>,
}

impl SnapshotCanonicalizationBudget {
    pub const fn new(limit: u64) -> Self {
        Self {
            limit,
            consumed: Cell::new(0),
        }
    }

    pub fn consumed(&self) -> u64 {
        self.consumed.get()
    }

    pub(crate) fn charge(&self, amount: u64) -> Result<(), SnapshotValueError> {
        let consumed =
            self.consumed.get().checked_add(amount).ok_or(
                SnapshotValueError::CanonicalizationWorkLimitExceededV1 { limit: self.limit },
            )?;
        if consumed > self.limit {
            return Err(SnapshotValueError::CanonicalizationWorkLimitExceededV1 {
                limit: self.limit,
            });
        }
        self.consumed.set(consumed);
        Ok(())
    }
}

impl<'a> SnapshotValidationContext<'a> {
    pub const fn new(schemas: &'a SchemaTable) -> Self {
        Self {
            schemas,
            named_kinds: None,
            canonicalization_budget: None,
            construction_authority: None,
            shared_schemas: OnceCell::new(),
        }
    }

    /// Reuses an already retained immutable schema arena during construction.
    /// Resident binders establish this owner before turns; finalization does
    /// not need to allocate another copy of the schema table.
    pub fn with_shared_schemas(schemas: &'a Arc<SchemaTable>) -> Self {
        let context = Self::new(schemas);
        let _ = context.shared_schemas.set(Arc::clone(schemas));
        context
    }

    pub const fn with_named_kinds(
        schemas: &'a SchemaTable,
        named_kinds: &'a dyn NamedKindPathResolver,
    ) -> Self {
        Self {
            schemas,
            named_kinds: Some(named_kinds),
            canonicalization_budget: None,
            construction_authority: None,
            shared_schemas: OnceCell::new(),
        }
    }

    pub const fn with_canonicalization_budget(
        mut self,
        budget: &'a SnapshotCanonicalizationBudget,
    ) -> Self {
        self.canonicalization_budget = Some(budget);
        self
    }

    pub(crate) const fn with_construction_authority(
        mut self,
        authority: &'a dyn SnapshotConstructionAuthority,
    ) -> Self {
        self.construction_authority = Some(authority);
        self
    }

    pub const fn schemas(&self) -> &'a SchemaTable {
        self.schemas
    }

    pub const fn named_kinds(&self) -> Option<&'a dyn NamedKindPathResolver> {
        self.named_kinds
    }

    fn try_vec_with_capacity<T>(&self, count: usize) -> Result<Vec<T>, SnapshotValueError> {
        let bytes = core::mem::size_of::<T>()
            .checked_mul(count)
            .and_then(|bytes| u64::try_from(bytes).ok())
            .ok_or(crate::MemoryRuntimeError::InvalidLayout {
                object: self
                    .construction_authority
                    .and_then(SnapshotConstructionAuthority::allocation_object),
                size: u64::MAX,
                alignment: u32::try_from(core::mem::align_of::<T>()).unwrap_or(u32::MAX),
                reason: "snapshot finalization vector layout overflows",
            })?;
        let alignment = u32::try_from(core::mem::align_of::<T>()).unwrap_or(u32::MAX);
        if let Some(authority) = self.construction_authority {
            authority.admit_snapshot_allocation(bytes, alignment)?;
        }
        let mut values = Vec::new();
        values.try_reserve_exact(count).map_err(|_| {
            crate::MemoryRuntimeError::AllocationFailed {
                object: self
                    .construction_authority
                    .and_then(SnapshotConstructionAuthority::allocation_object),
                requested: bytes,
                alignment,
                space: crate::MemorySpace::Host,
            }
        })?;
        Ok(values)
    }

    fn try_box<T>(&self, value: T) -> Result<Box<T>, SnapshotValueError> {
        let bytes = core::mem::size_of::<T>() as u64;
        let alignment = u32::try_from(core::mem::align_of::<T>()).unwrap_or(u32::MAX);
        if let Some(authority) = self.construction_authority {
            authority.admit_snapshot_allocation(bytes, alignment)?;
        }
        Box::try_new(value).map_err(|_| {
            crate::MemoryRuntimeError::AllocationFailed {
                object: self
                    .construction_authority
                    .and_then(SnapshotConstructionAuthority::allocation_object),
                requested: bytes,
                alignment,
                space: crate::MemorySpace::Host,
            }
            .into()
        })
    }

    fn try_arc<T>(&self, value: T) -> Result<Arc<T>, SnapshotValueError> {
        let bytes = core::mem::size_of::<T>()
            .checked_add(core::mem::size_of::<usize>().saturating_mul(2))
            .and_then(|bytes| u64::try_from(bytes).ok())
            .unwrap_or(u64::MAX);
        let alignment = u32::try_from(core::mem::align_of::<T>()).unwrap_or(u32::MAX);
        if let Some(authority) = self.construction_authority {
            authority.admit_snapshot_allocation(bytes, alignment)?;
        }
        Arc::try_new(value).map_err(|_| {
            crate::MemoryRuntimeError::AllocationFailed {
                object: self
                    .construction_authority
                    .and_then(SnapshotConstructionAuthority::allocation_object),
                requested: bytes,
                alignment,
                space: crate::MemorySpace::Host,
            }
            .into()
        })
    }

    fn try_boxed_str(&self, value: String) -> Result<Box<str>, SnapshotValueError> {
        let bytes = u64::try_from(value.len()).unwrap_or(u64::MAX);
        if let Some(authority) = self.construction_authority {
            authority.admit_snapshot_allocation(bytes, 1)?;
        }
        Ok(value.into_boxed_str())
    }

    fn try_clone_schemas(&self) -> Result<Arc<SchemaTable>, SnapshotValueError> {
        if let Some(schemas) = self.shared_schemas.get() {
            return Ok(schemas.clone());
        }
        let bytes = self.schemas.clone_allocation_bound_bytes().ok_or(
            crate::MemoryRuntimeError::InvalidLayout {
                object: self
                    .construction_authority
                    .and_then(SnapshotConstructionAuthority::allocation_object),
                size: u64::MAX,
                alignment: u32::try_from(core::mem::align_of::<SchemaTable>()).unwrap_or(u32::MAX),
                reason: "snapshot schema context clone layout overflows",
            },
        )?;
        let alignment = u32::try_from(core::mem::align_of::<SchemaTable>()).unwrap_or(u32::MAX);
        if let Some(authority) = self.construction_authority {
            authority.admit_snapshot_allocation(bytes, alignment)?;
        }
        let schemas = Arc::try_new(self.schemas.clone()).map_err(|_| {
            SnapshotValueError::from(crate::MemoryRuntimeError::AllocationFailed {
                object: self
                    .construction_authority
                    .and_then(SnapshotConstructionAuthority::allocation_object),
                requested: bytes,
                alignment,
                space: crate::MemorySpace::Host,
            })
        })?;
        // Recursive Dynamic values use the same immutable schema owner. The
        // complete schema-table allocation is therefore admitted exactly once
        // per finalization tree rather than once per nested Value node.
        let _ = self.shared_schemas.set(schemas.clone());
        Ok(schemas)
    }
}

#[derive(Clone)]
pub struct Value {
    schema: SchemaId,
    schema_key: SchemaKey,
    shape: Arc<ShapeInstance>,
    root: Arc<FrozenSnapshotStorage>,
    resident_token: u64,
    schemas: Option<Arc<SchemaTable>>,
}

/// Detached immutable canonical ownership. Cloning a [`Value`] retains this
/// root instead of recursively cloning its payload tree. The root contains no
/// mutable memory domain, cell, or executor authority.
#[derive(Debug)]
pub struct FrozenSnapshotStorage {
    data: Arc<FrozenSnapshotData>,
    memory_budget: Option<SnapshotBudgetOwnership>,
}

#[derive(Debug)]
struct SnapshotBudgetOwnership {
    schemas: Arc<SchemaTable>,
    required_bytes: u64,
    reservation: crate::ManagedMemoryReservation,
    identity: SharedOwnershipCell<Weak<FrozenSnapshotStorage>>,
}

#[derive(Debug)]
struct SnapshotBudgetRegistration {
    budget: crate::ManagedMemoryBudget,
    schemas: Weak<SchemaTable>,
    owner: Weak<FrozenSnapshotStorage>,
    next: Option<Box<SnapshotBudgetRegistration>>,
}

/// The immutable canonical tree and its one physical accounting ticket share
/// the same lifetime. Outer [`FrozenSnapshotStorage`] wrappers may be replaced
/// or imported across domains without duplicating that physical charge.
#[derive(Debug)]
struct FrozenSnapshotData {
    data: ValueData,
    // The same immutable shape owner is shared by every ordinary Value clone.
    // Physical and imported-root tickets retain its real allocation as part
    // of this data owner, rather than charging a detached metadata copy.
    shape: Arc<ShapeInstance>,
    // Physical accounting follows the one shared immutable data lifetime,
    // not each cell/import wrapper. Standard builds use a thread-safe once
    // cell so detached Values retain their Send/Sync behavior. no_std has no
    // cross-thread execution surface and uses the core equivalent.
    ownership: SharedOwnershipCell<crate::RetainedPayloadTicket>,
    budget_imports: SnapshotBudgetRegistry,
}

impl FrozenSnapshotData {
    fn with_budget_imports<T>(
        &self,
        apply: impl FnOnce(&mut Option<Box<SnapshotBudgetRegistration>>) -> T,
    ) -> T {
        #[cfg(all(feature = "no_std", not(feature = "std")))]
        let mut imports = self.budget_imports.borrow_mut();
        #[cfg(any(not(feature = "no_std"), feature = "std"))]
        let mut imports = self
            .budget_imports
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        apply(&mut imports)
    }

    fn budget_import(
        &self,
        budget: &crate::ManagedMemoryBudget,
        schemas: &Arc<SchemaTable>,
    ) -> Option<Arc<FrozenSnapshotStorage>> {
        self.with_budget_imports(|imports| Self::find_budget_import(imports, budget, schemas))
    }

    fn find_budget_import(
        imports: &Option<Box<SnapshotBudgetRegistration>>,
        budget: &crate::ManagedMemoryBudget,
        schemas: &Arc<SchemaTable>,
    ) -> Option<Arc<FrozenSnapshotStorage>> {
        let mut current = imports.as_deref();
        while let Some(import) = current {
            if import.budget == *budget && Weak::ptr_eq(&import.schemas, &Arc::downgrade(schemas)) {
                if let Some(owner) = import.owner.upgrade() {
                    return Some(owner);
                }
            }
            current = import.next.as_deref();
        }
        None
    }
}

impl Drop for FrozenSnapshotStorage {
    fn drop(&mut self) {
        let Some(owner) = self
            .memory_budget
            .as_ref()
            .and_then(|ownership| ownership.identity.get())
        else {
            return;
        };
        let removed = self.data.with_budget_imports(|imports| {
            let mut position = imports;
            loop {
                match position {
                    Some(entry) if Weak::ptr_eq(&entry.owner, owner) => {
                        let mut removed = position.take().expect("located immutable import");
                        *position = removed.next.take();
                        break Some(removed);
                    }
                    Some(entry) => position = &mut entry.next,
                    None => break None,
                }
            }
        });
        // Free the exact admitted registration outside its registry lock,
        // before field destruction releases this wrapper's reservation.
        drop(removed);
    }
}

impl core::fmt::Debug for Value {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("Value")
            .field("schema", &self.schema)
            .field("schema_key", &self.schema_key)
            .field("shape", &self.shape)
            .field("data", &self.root.data.data)
            .finish()
    }
}

impl Value {
    #[doc(hidden)]
    pub fn shares_frozen_storage(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.root.data, &other.root.data)
    }

    #[doc(hidden)]
    pub fn shares_retained_payload_ticket(&self, other: &Self) -> bool {
        match (
            self.root.data.ownership.get(),
            other.root.data.ownership.get(),
        ) {
            (Some(left), Some(right)) => left.shares_charge_with(right),
            _ => false,
        }
    }

    /// Shares immutable data, shape, and schema owners without allocating.
    #[cfg(feature = "functions")]
    pub(crate) fn try_clone_for_external_marshalling(
        &self,
        _construction: &dyn SnapshotConstructionAuthority,
    ) -> Result<Self, SnapshotValueError> {
        Ok(self.clone())
    }

    /// Sealed handoff from an admitted mutable payload envelope into a
    /// detached immutable root. The root owns the concrete canonical tree;
    /// the pointer-free ticket owns its retained allocation charge.
    pub(crate) fn into_retained_payload_ticket(
        self,
        ownership: crate::RetainedPayloadTicket,
    ) -> Self {
        // Losing this race only drops the redundant newly admitted ticket;
        // the winning ticket remains inseparable from the shared data Arc.
        let _ = self.root.data.ownership.set(ownership);
        self
    }

    pub(crate) fn has_retained_payload_ticket(&self) -> bool {
        self.root.data.ownership.get().is_some()
    }

    /// Imports this value into an admitted, independently owned budget
    /// wrapper. The caller's other clones remain unchanged. Failed candidates
    /// release their own import claim even when the original data stays alive.
    pub fn into_memory_budget(
        mut self,
        reservation: &mut crate::ManagedMemoryReservation,
    ) -> crate::MemoryRuntimeResult<Self> {
        let budget = reservation.budget();
        let schemas = self.schemas.clone().ok_or_else(|| {
            crate::MemoryRuntimeError::CandidateValidationFailed {
                object: None,
                reason: "immutable snapshot has no retained schema context".into(),
            }
        })?;
        if let Some(owner) = self.root.data.budget_import(&budget, &schemas) {
            self.root = owner;
            return Ok(self);
        }
        let retained_bytes = self.memory_budget_required_bytes()?;
        let admitted_bytes = self.unshared_memory_budget_admission_bytes(&budget)?;
        if admitted_bytes > reservation.capacity_bytes() {
            return Err(crate::MemoryRuntimeError::BudgetExceeded {
                operation: None,
                requested: admitted_bytes,
                limit: reservation.capacity_bytes(),
            });
        }
        budget.check_snapshot_import_allocation(
            core::mem::size_of::<SnapshotBudgetRegistration>() as u64,
            core::mem::align_of::<SnapshotBudgetRegistration>() as u32,
        )?;
        let mut registration = Box::try_new(SnapshotBudgetRegistration {
            budget: budget.clone(),
            schemas: Arc::downgrade(&schemas),
            owner: Weak::new(),
            next: None,
        })
        .map_err(|_| crate::MemoryRuntimeError::AllocationFailed {
            object: None,
            requested: core::mem::size_of::<SnapshotBudgetRegistration>() as u64,
            alignment: core::mem::align_of::<SnapshotBudgetRegistration>() as u32,
            space: crate::MemorySpace::Host,
        })?;
        let data = self.root.data.clone();
        budget.check_snapshot_import_allocation(
            core::mem::size_of::<FrozenSnapshotStorage>() as u64,
            core::mem::align_of::<FrozenSnapshotStorage>() as u32,
        )?;
        let mut prepared = Arc::try_new(FrozenSnapshotStorage {
            data: data.clone(),
            memory_budget: None,
        })
        .map_err(|_| crate::MemoryRuntimeError::AllocationFailed {
            object: None,
            requested: core::mem::size_of::<FrozenSnapshotStorage>() as u64,
            alignment: core::mem::align_of::<FrozenSnapshotStorage>() as u32,
            space: crate::MemorySpace::Host,
        })?;
        let owner = data.with_budget_imports(|imports| {
            if let Some(existing) =
                FrozenSnapshotData::find_budget_import(imports, &budget, &schemas)
            {
                return Ok(existing);
            }
            // All physical construction has succeeded under the reservation.
            // No user code runs under this short metadata gate.
            let charge = reservation.split_capacity(admitted_bytes)?;
            Arc::get_mut(&mut prepared)
                .expect("unpublished import has no shared or weak wrapper owner")
                .memory_budget = Some(SnapshotBudgetOwnership {
                schemas,
                required_bytes: retained_bytes,
                reservation: charge,
                identity: SharedOwnershipCell::new(),
            });
            registration.owner = Arc::downgrade(&prepared);
            let _ = prepared
                .memory_budget
                .as_ref()
                .expect("prepared immutable ownership")
                .identity
                .set(registration.owner.clone());
            registration.next = imports.take();
            *imports = Some(registration);
            Ok::<_, crate::MemoryRuntimeError>(prepared.clone())
        })?;
        self.root = owner;
        Ok(self)
    }

    /// Exact extra wrapper/registration metadata for an independently owned
    /// budget import. Candidate planners include this before materialization.
    pub const fn memory_budget_claim_metadata_bytes() -> u64 {
        Self::shared_owner_allocation_bytes(
            core::mem::size_of::<FrozenSnapshotStorage>(),
            core::mem::align_of::<FrozenSnapshotStorage>(),
        ) + core::mem::size_of::<SnapshotBudgetRegistration>() as u64
    }

    /// Concrete shared-owner allocations retained by one finalized root.
    /// Shape elements and cloned schema contents are supplied separately by
    /// the checked footprint; only their owner headers appear here.
    pub(crate) const fn canonical_owner_allocation_bytes() -> u64 {
        Self::shared_owner_allocation_bytes(
            core::mem::size_of::<FrozenSnapshotStorage>(),
            core::mem::align_of::<FrozenSnapshotStorage>(),
        ) + Self::shared_owner_allocation_bytes(
            core::mem::size_of::<FrozenSnapshotData>(),
            core::mem::align_of::<FrozenSnapshotData>(),
        ) + Self::shared_owner_allocation_bytes(
            core::mem::size_of::<ShapeInstance>(),
            core::mem::align_of::<ShapeInstance>(),
        ) + Self::shared_owner_allocation_bytes(
            crate::RetainedPayloadTicket::ownership_header_bytes() as usize,
            crate::RetainedPayloadTicket::ownership_header_alignment(),
        ) + Self::shared_owner_allocation_bytes(
            core::mem::size_of::<SchemaTable>(),
            core::mem::align_of::<SchemaTable>(),
        ) - core::mem::size_of::<SchemaTable>() as u64
    }

    const fn shared_owner_allocation_bytes(size: usize, alignment: usize) -> u64 {
        // Pinned Arc layout: two pointer-width counters followed by its
        // aligned concrete value, rounded to the complete allocation's
        // alignment. This includes target-specific prefix/tail padding.
        let counters = 2 * core::mem::size_of::<usize>();
        let prefix = counters.div_ceil(alignment) * alignment;
        let allocation_alignment = if alignment > core::mem::align_of::<usize>() {
            alignment
        } else {
            core::mem::align_of::<usize>()
        };
        (prefix + size).div_ceil(allocation_alignment) as u64 * allocation_alignment as u64
    }

    /// Additional capacity needed to retain this actual data and schema owner
    /// in `budget`. Existing same-account import wrappers share ownership;
    /// existing physical accounting stays with its original ticket.
    pub fn memory_budget_admission_bytes(
        &self,
        budget: &crate::ManagedMemoryBudget,
    ) -> crate::MemoryRuntimeResult<u64> {
        if let Some(schemas) = &self.schemas {
            if self.root.data.budget_import(budget, schemas).is_some() {
                return Ok(0);
            }
        }
        self.unshared_memory_budget_admission_bytes(budget)
    }

    fn unshared_memory_budget_admission_bytes(
        &self,
        budget: &crate::ManagedMemoryBudget,
    ) -> crate::MemoryRuntimeResult<u64> {
        let required = self.memory_budget_required_bytes()?;
        let physical = self
            .root
            .data
            .ownership
            .get()
            .and_then(|owner| owner.memory_budget_retained_bytes(budget))
            .unwrap_or(0);
        (required - required.min(physical))
            .checked_add(core::mem::size_of::<SnapshotBudgetRegistration>() as u64)
            .ok_or_else(|| crate::MemoryRuntimeError::CandidateValidationFailed {
                object: None,
                reason: "immutable snapshot import witness overflows".into(),
            })
    }

    /// Checked retained-allocation witness used by `into_memory_budget`.
    /// It covers canonical payload, shared-owner headers and the retained
    /// schema context. Only actual nested dynamic roots add shared-owner
    /// headers; primitive sequence elements do not each count as a root.
    /// This is admitted capacity, not a report of observed physical bytes.
    pub fn memory_budget_required_bytes(&self) -> crate::MemoryRuntimeResult<u64> {
        let schemas = self.schemas.as_deref().ok_or_else(|| {
            crate::MemoryRuntimeError::CandidateValidationFailed {
                object: None,
                reason: "immutable snapshot has no retained schema context".into(),
            }
        })?;
        let footprint = self.retained_footprint(schemas).map_err(|_| {
            crate::MemoryRuntimeError::CandidateValidationFailed {
                object: None,
                reason: "immutable snapshot retained footprint is invalid or overflows".into(),
            }
        })?;
        let arithmetic_error = || crate::MemoryRuntimeError::CandidateValidationFailed {
            object: None,
            reason: "immutable snapshot ownership witness overflows".into(),
        };
        let owner_bytes = self
            .memory_budget_owner_bytes(None)
            .ok_or_else(arithmetic_error)?;
        footprint
            .retained_bytes
            .checked_add(owner_bytes)
            .ok_or_else(arithmetic_error)
    }

    fn memory_budget_owner_bytes(&self, shared_schemas: Option<&Arc<SchemaTable>>) -> Option<u64> {
        let schemas = self.schemas.as_ref()?;
        let schema_bytes = if shared_schemas.is_some_and(|shared| Arc::ptr_eq(shared, schemas)) {
            0
        } else {
            schemas.clone_allocation_bound_bytes()?.checked_add(
                Self::shared_owner_allocation_bytes(
                    core::mem::size_of::<SchemaTable>(),
                    core::mem::align_of::<SchemaTable>(),
                ) - core::mem::size_of::<SchemaTable>() as u64,
            )?
        };
        let owner_bytes = Self::shared_owner_allocation_bytes(
            core::mem::size_of::<FrozenSnapshotStorage>(),
            core::mem::align_of::<FrozenSnapshotStorage>(),
        )
        .checked_add(Self::shared_owner_allocation_bytes(
            core::mem::size_of::<FrozenSnapshotData>(),
            core::mem::align_of::<FrozenSnapshotData>(),
        ))?
        .checked_add(Self::shared_owner_allocation_bytes(
            crate::RetainedPayloadTicket::ownership_header_bytes() as usize,
            crate::RetainedPayloadTicket::ownership_header_alignment(),
        ))?
        .checked_add(Self::shared_owner_allocation_bytes(
            core::mem::size_of::<ShapeInstance>(),
            core::mem::align_of::<ShapeInstance>(),
        ))?;
        owner_bytes
            .checked_add(schema_bytes)?
            .checked_add(Self::nested_budget_owner_bytes(self.data(), schemas)?)
    }

    fn nested_budget_owner_bytes(data: &ValueData, schemas: &Arc<SchemaTable>) -> Option<u64> {
        fn children<'a>(
            mut values: impl Iterator<Item = &'a ValueData>,
            schemas: &Arc<SchemaTable>,
        ) -> Option<u64> {
            values.try_fold(0_u64, |total, value| {
                total.checked_add(Value::nested_budget_owner_bytes(value, schemas)?)
            })
        }
        fn sequence(values: &SequenceStorage, schemas: &Arc<SchemaTable>) -> Option<u64> {
            match values {
                SequenceStorage::Values(values) => children(values.iter(), schemas),
                _ => Some(0),
            }
        }
        match data {
            ValueData::Dynamic(value) => match value.value.as_deref() {
                Some(value) => value.memory_budget_owner_bytes(Some(schemas)),
                None => Some(0),
            },
            ValueData::Enum(value) => children(value.payload().into_iter(), schemas),
            ValueData::Option(value) => children(value.as_deref().into_iter(), schemas),
            ValueData::Tuple(values) => children(values.iter(), schemas),
            ValueData::Record(value) => children(value.fields().iter(), schemas),
            ValueData::Matrix(value) => sequence(&value.elements, schemas),
            ValueData::Table(value) => value.columns.iter().try_fold(0_u64, |total, values| {
                total.checked_add(sequence(values, schemas)?)
            }),
            ValueData::Set(value) => children(
                value.elements().iter().map(CanonicalKeyValue::data),
                schemas,
            ),
            ValueData::Map(value) => children(
                value
                    .entries()
                    .iter()
                    .flat_map(|entry| [entry.key().data(), entry.value()]),
                schemas,
            ),
            _ => Some(0),
        }
    }

    /// Reports this shared root's already transferred capacity in `budget`.
    /// An uncharged or only partially covered root returns None. In particular,
    /// an older payload-only physical ticket is not complete budget clearance
    /// for its retained headers and schema; handoff supplements it first.
    pub fn memory_budget_retained_bytes(&self, budget: &crate::ManagedMemoryBudget) -> Option<u64> {
        let ownership = self.root.memory_budget.as_ref()?;
        let schemas = self.schemas.as_ref()?;
        (ownership.reservation.budget() == *budget && Arc::ptr_eq(schemas, &ownership.schemas))
            .then_some(ownership.required_bytes)
    }

    pub const fn schema(&self) -> SchemaId {
        self.schema
    }

    pub const fn schema_key(&self) -> SchemaKey {
        self.schema_key
    }

    pub fn shape(&self) -> &ShapeInstance {
        debug_assert!(Arc::ptr_eq(&self.shape, &self.root.data.shape));
        &self.shape
    }

    pub fn data(&self) -> &ValueData {
        &self.root.data.data
    }

    /// Returns the immutable schema table that validates this detached value.
    pub fn schemas(&self) -> Option<Arc<SchemaTable>> {
        self.schemas.clone()
    }

    /// Revalidates this immutable payload against an equivalent schema in a
    /// different schema table and returns a value bound to that table.
    pub fn rebind(
        &self,
        schema: SchemaId,
        shape: &ShapeInstance,
        schemas: &SchemaTable,
    ) -> Result<Self, SnapshotValueError> {
        let source_schemas =
            self.schemas
                .as_deref()
                .ok_or(SnapshotValueError::UnknownSnapshotSchema {
                    schema: self.schema,
                })?;
        let source_schema = self.validate_against(source_schemas)?;
        let target_entry = schemas
            .entry(schema)
            .ok_or(SnapshotValueError::UnknownSnapshotSchema { schema })?;
        let target_schema = target_entry.schema();
        let exact_definition = self.schema_key == target_entry.key()
            && source_schema.canonical_bytes() == target_schema.canonical_bytes();
        let equivalent_at_shape =
            crate::cell_binding::close_schema_body(source_schema.body(), &self.shape)
                .and_then(|source| {
                    crate::cell_binding::close_schema_body(target_schema.body(), shape)
                        .map(|target| source == target)
                })
                .unwrap_or(false);
        let target_accepts_source_extent = dynamic_extent_rebind_compatible(
            source_schema.body(),
            &self.shape,
            target_schema.body(),
            shape,
        );
        if !exact_definition && !equivalent_at_shape && !target_accepts_source_extent {
            return Err(SnapshotValueError::SnapshotSchemaDefinitionMismatch {
                key: self.schema_key,
            });
        }
        // Exact metadata in the same table is a true clone. Equivalent closed
        // metadata in another table may share the immutable tree, but the
        // returned value must retain that target table. Dynamic children carry
        // nested schema identities, so they must take the rebuilding path.
        if target_entry.key() == self.schema_key && shape == self.shape.as_ref() && exact_definition
        {
            if core::ptr::eq(source_schemas, schemas) && schema == self.schema {
                return Ok(self.clone());
            }
            if !schema_body_contains_dynamic(target_schema.body()) {
                return Ok(Self {
                    schema,
                    schema_key: target_entry.key(),
                    shape: self.shape.clone(),
                    root: self.root.clone(),
                    resident_token: self.resident_token,
                    schemas: Some(Arc::new(schemas.clone())),
                });
            }
        }
        let data = canonical_data_to_rebound_draft(
            source_schema.body(),
            &self.root.data.data,
            &SnapshotPath::root(),
            schemas,
        )?;
        let data = adapt_dynamic_bytecode_placeholders(
            source_schema.body(),
            target_schema.body(),
            data,
            &SnapshotPath::root(),
        )?;
        ValueDraft {
            schema,
            shape_values: shape.parameter_values().to_vec().into_boxed_slice(),
            data,
        }
        .finalize(&SnapshotValidationContext::new(schemas))
    }

    /// Returns schema-directed draft data suitable for embedding this value in
    /// a newly derived aggregate schema. Nominal identity and option/enum
    /// structure remain governed by the value's originating schema.
    pub fn canonical_data_draft(&self) -> Result<ValueDataDraft, SnapshotValueError> {
        let schemas = self
            .schemas
            .as_deref()
            .ok_or(SnapshotValueError::UnknownSnapshotSchema {
                schema: self.schema,
            })?;
        let schema = schemas
            .get(self.schema)
            .ok_or(SnapshotValueError::UnknownSnapshotSchema {
                schema: self.schema,
            })?;
        canonical_data_to_draft(schema.body(), &self.root.data.data, &SnapshotPath::root())
    }

    /// Compact deterministic token computed when the finalized value is
    /// constructed. Resident receipts use it without consulting schemas or
    /// re-encoding immutable payloads during a turn.
    #[doc(hidden)]
    pub const fn resident_token(&self) -> u64 {
        self.resident_token
    }

    pub fn validate_against<'a>(
        &self,
        schemas: &'a SchemaTable,
    ) -> Result<&'a Schema, SnapshotValueError> {
        let entry = schemas.entry(self.schema);
        if entry.map(|entry| entry.key()) != Some(self.schema_key) {
            return Err(SnapshotValueError::SnapshotSchemaTableMismatch {
                schema: self.schema,
                expected: self.schema_key,
                actual: entry.map(|entry| entry.key()),
            });
        }
        Ok(entry.expect("matching entry exists").schema())
    }

    fn rebuild(
        &self,
        data: ValueDataDraft,
        context: &SnapshotValidationContext<'_>,
    ) -> Result<Self, SnapshotValueError> {
        self.validate_against(context.schemas())?;
        ValueDraft {
            schema: self.schema,
            shape_values: self.shape.parameter_values().to_vec().into_boxed_slice(),
            data,
        }
        .finalize(context)
    }

    pub fn rebuild_enum(
        &self,
        ordinal: u32,
        payload: Option<ValueData>,
        context: &SnapshotValidationContext<'_>,
    ) -> Result<Self, SnapshotValueError> {
        let schema = self.validate_against(context.schemas())?;
        let SchemaBody::Enum { variants, .. } = schema.body() else {
            return Err(rebuild_kind_mismatch(
                schema.body(),
                super::ValueDataKind::Enum,
            ));
        };
        let variant = variants.get(ordinal as usize).ok_or_else(|| {
            SnapshotValueError::EnumOrdinalOutOfRangeV1 {
                path: SnapshotPath::root(),
                ordinal,
                variants: variants.len() as u32,
            }
        })?;
        let payload = match (variant.payload.as_ref(), payload) {
            (Some(schema), Some(payload)) => Some(Box::new(canonical_data_to_draft(
                schema,
                &payload,
                &SnapshotPath::root().child(SnapshotPathSegment::EnumPayload(ordinal)),
            )?)),
            (None, None) => None,
            _ => {
                return Err(SnapshotValueError::EnumPayloadMismatchV1 {
                    path: SnapshotPath::root(),
                });
            }
        };
        self.rebuild(
            ValueDataDraft::Enum(EnumDraft { ordinal, payload }),
            context,
        )
    }

    pub fn set_element_drafts(
        &self,
        schemas: &SchemaTable,
    ) -> Result<Box<[ValueDataDraft]>, SnapshotValueError> {
        let schema = self.validate_against(schemas)?;
        let SchemaBody::Set { element, .. } = schema.body() else {
            return Err(rebuild_kind_mismatch(
                schema.body(),
                super::ValueDataKind::Set,
            ));
        };
        let ValueData::Set(set) = self.data() else {
            unreachable!("validated set schema has set data")
        };
        set.elements()
            .iter()
            .enumerate()
            .map(|(index, value)| {
                canonical_data_to_draft(
                    element,
                    value.data(),
                    &SnapshotPath::root().child(SnapshotPathSegment::SetElement(index as u64)),
                )
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Vec::into_boxed_slice)
    }

    /// Converts canonical set element data back into drafts using this set's
    /// declared element schema without imposing this set's container
    /// cardinality on a derived result.
    pub fn set_element_data_drafts(
        &self,
        schemas: &SchemaTable,
        elements: &[ValueData],
    ) -> Result<Box<[ValueDataDraft]>, SnapshotValueError> {
        let schema = self.validate_against(schemas)?;
        let SchemaBody::Set { element, .. } = schema.body() else {
            return Err(rebuild_kind_mismatch(
                schema.body(),
                super::ValueDataKind::Set,
            ));
        };
        elements
            .iter()
            .enumerate()
            .map(|(index, value)| {
                canonical_data_to_draft(
                    element,
                    value,
                    &SnapshotPath::root().child(SnapshotPathSegment::SetElement(index as u64)),
                )
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Vec::into_boxed_slice)
    }

    pub fn rebuild_option(
        &self,
        payload: Option<ValueData>,
        context: &SnapshotValidationContext<'_>,
    ) -> Result<Self, SnapshotValueError> {
        let schema = self.validate_against(context.schemas())?;
        let SchemaBody::Option(element) = schema.body() else {
            return Err(rebuild_kind_mismatch(
                schema.body(),
                super::ValueDataKind::Option,
            ));
        };
        let payload = payload
            .as_ref()
            .map(|payload| {
                canonical_data_to_draft(
                    element,
                    payload,
                    &SnapshotPath::root().child(SnapshotPathSegment::OptionValue),
                )
                .map(Box::new)
            })
            .transpose()?;
        self.rebuild(
            ValueDataDraft::Option(OptionDraft {
                present: payload.is_some(),
                value: payload,
            }),
            context,
        )
    }

    pub fn rebuild_tuple(
        &self,
        children: Box<[ValueData]>,
        context: &SnapshotValidationContext<'_>,
    ) -> Result<Self, SnapshotValueError> {
        let schema = self.validate_against(context.schemas())?;
        let SchemaBody::Tuple(elements) = schema.body() else {
            return Err(rebuild_kind_mismatch(
                schema.body(),
                super::ValueDataKind::Tuple,
            ));
        };
        ensure_arity(&SnapshotPath::root(), elements.len(), children.len())?;
        let children = elements
            .iter()
            .zip(children.iter())
            .enumerate()
            .map(|(index, (schema, child))| {
                canonical_data_to_draft(
                    schema,
                    child,
                    &SnapshotPath::root().child(SnapshotPathSegment::TupleElement(index as u32)),
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        self.rebuild(ValueDataDraft::Tuple(children.into_boxed_slice()), context)
    }

    pub fn rebuild_record(
        &self,
        children: Box<[ValueData]>,
        context: &SnapshotValidationContext<'_>,
    ) -> Result<Self, SnapshotValueError> {
        let schema = self.validate_against(context.schemas())?;
        let SchemaBody::Record(fields) = schema.body() else {
            return Err(rebuild_kind_mismatch(
                schema.body(),
                super::ValueDataKind::Record,
            ));
        };
        ensure_arity(&SnapshotPath::root(), fields.len(), children.len())?;
        let children = fields
            .iter()
            .zip(children.iter())
            .enumerate()
            .map(|(index, (field, child))| {
                Ok(NamedValueDraft {
                    name: field.name.clone(),
                    value: canonical_data_to_draft(
                        &field.schema,
                        child,
                        &SnapshotPath::root().child(SnapshotPathSegment::RecordField(index as u32)),
                    )?,
                })
            })
            .collect::<Result<Vec<_>, SnapshotValueError>>()?;
        self.rebuild(ValueDataDraft::Record(children.into_boxed_slice()), context)
    }

    pub fn rebuild_matrix(
        &self,
        elements: Box<[ValueData]>,
        context: &SnapshotValidationContext<'_>,
    ) -> Result<Self, SnapshotValueError> {
        let schema = self.validate_against(context.schemas())?;
        let SchemaBody::Matrix { element, .. } = schema.body() else {
            return Err(rebuild_kind_mismatch(
                schema.body(),
                super::ValueDataKind::Matrix,
            ));
        };
        let elements = elements
            .iter()
            .enumerate()
            .map(|(index, value)| {
                canonical_data_to_draft(
                    element,
                    value,
                    &SnapshotPath::root().child(SnapshotPathSegment::MatrixElement(index as u64)),
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        self.rebuild(ValueDataDraft::Matrix(elements.into_boxed_slice()), context)
    }

    pub fn rebuild_table(
        &self,
        columns: Box<[Box<[ValueData]>]>,
        context: &SnapshotValidationContext<'_>,
    ) -> Result<Self, SnapshotValueError> {
        let schema = self.validate_against(context.schemas())?;
        let SchemaBody::Table {
            columns: expected, ..
        } = schema.body()
        else {
            return Err(rebuild_kind_mismatch(
                schema.body(),
                super::ValueDataKind::Table,
            ));
        };
        ensure_arity(&SnapshotPath::root(), expected.len(), columns.len())?;
        let columns = expected
            .iter()
            .zip(columns.iter())
            .enumerate()
            .map(|(column_index, (field, values))| {
                let values = values
                    .iter()
                    .enumerate()
                    .map(|(row_index, value)| {
                        canonical_data_to_draft(
                            &field.schema,
                            value,
                            &SnapshotPath::root()
                                .child(SnapshotPathSegment::TableColumn(column_index as u32))
                                .child(SnapshotPathSegment::TableRow(row_index as u64)),
                        )
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(TableColumnDraft {
                    name: field.name.clone(),
                    values: values.into_boxed_slice(),
                })
            })
            .collect::<Result<Vec<_>, SnapshotValueError>>()?;
        self.rebuild(ValueDataDraft::Table(columns.into_boxed_slice()), context)
    }

    pub fn rebuild_set(
        &self,
        elements: Box<[ValueData]>,
        context: &SnapshotValidationContext<'_>,
    ) -> Result<Self, SnapshotValueError> {
        let schema = self.validate_against(context.schemas())?;
        let SchemaBody::Set { element, .. } = schema.body() else {
            return Err(rebuild_kind_mismatch(
                schema.body(),
                super::ValueDataKind::Set,
            ));
        };
        let elements = elements
            .iter()
            .enumerate()
            .map(|(index, value)| {
                canonical_data_to_draft(
                    element,
                    value,
                    &SnapshotPath::root().child(SnapshotPathSegment::SetElement(index as u64)),
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        self.rebuild(ValueDataDraft::Set(elements.into_boxed_slice()), context)
    }

    pub fn rebuild_set_drafts(
        &self,
        elements: Box<[ValueDataDraft]>,
        context: &SnapshotValidationContext<'_>,
    ) -> Result<Self, SnapshotValueError> {
        let schema = self.validate_against(context.schemas())?;
        if !matches!(schema.body(), SchemaBody::Set { .. }) {
            return Err(rebuild_kind_mismatch(
                schema.body(),
                super::ValueDataKind::Set,
            ));
        }
        self.rebuild(ValueDataDraft::Set(elements), context)
    }

    pub fn rebuild_map(
        &self,
        entries: Box<[(ValueData, ValueData)]>,
        context: &SnapshotValidationContext<'_>,
    ) -> Result<Self, SnapshotValueError> {
        let schema = self.validate_against(context.schemas())?;
        let SchemaBody::Map { key, value, .. } = schema.body() else {
            return Err(rebuild_kind_mismatch(
                schema.body(),
                super::ValueDataKind::Map,
            ));
        };
        let entries = entries
            .iter()
            .enumerate()
            .map(|(index, (entry_key, entry_value))| {
                Ok(MapEntryDraft {
                    items: vec![
                        canonical_data_to_draft(
                            key,
                            entry_key,
                            &SnapshotPath::root().child(SnapshotPathSegment::MapKey(index as u64)),
                        )?,
                        canonical_data_to_draft(
                            value,
                            entry_value,
                            &SnapshotPath::root()
                                .child(SnapshotPathSegment::MapValue(index as u64)),
                        )?,
                    ]
                    .into_boxed_slice(),
                })
            })
            .collect::<Result<Vec<_>, SnapshotValueError>>()?;
        self.rebuild(ValueDataDraft::Map(entries.into_boxed_slice()), context)
    }
}

fn schema_body_contains_dynamic(schema: &SchemaBody) -> bool {
    match schema {
        SchemaBody::Dynamic => true,
        SchemaBody::Enum { variants, .. } => variants.iter().any(|variant| {
            variant
                .payload
                .as_ref()
                .is_some_and(schema_body_contains_dynamic)
        }),
        SchemaBody::Option(value)
        | SchemaBody::Matrix { element: value, .. }
        | SchemaBody::Set { element: value, .. } => schema_body_contains_dynamic(value),
        SchemaBody::Tuple(values) => values.iter().any(schema_body_contains_dynamic),
        SchemaBody::Record(fields)
        | SchemaBody::Table {
            columns: fields, ..
        } => fields
            .iter()
            .any(|field| schema_body_contains_dynamic(&field.schema)),
        SchemaBody::Map { key, value, .. } => {
            schema_body_contains_dynamic(key) || schema_body_contains_dynamic(value)
        }
        SchemaBody::Bool
        | SchemaBody::UnsignedInteger(_)
        | SchemaBody::SignedInteger(_)
        | SchemaBody::FloatingPoint(_)
        | SchemaBody::Complex(_)
        | SchemaBody::Rational64
        | SchemaBody::String
        | SchemaBody::Id
        | SchemaBody::Index
        | SchemaBody::Atom(_)
        | SchemaBody::ReifiedType => false,
    }
}

fn dynamic_extent_rebind_compatible(
    source: &SchemaBody,
    source_shape: &ShapeInstance,
    target: &SchemaBody,
    target_shape: &ShapeInstance,
) -> bool {
    let Ok(source) = crate::cell_binding::close_schema_body(source, source_shape) else {
        return false;
    };
    let Ok(target) = crate::cell_binding::close_schema_body(target, target_shape) else {
        return false;
    };
    closed_schema_rebind_compatible(&source, &target)
}

fn closed_schema_rebind_compatible(source: &SchemaBody, target: &SchemaBody) -> bool {
    if source == target {
        return true;
    }
    let dynamic_target =
        |target: &crate::CardinalitySpec| matches!(target, crate::CardinalitySpec::Dynamic { .. });
    match (source, target) {
        (SchemaBody::ReifiedType, SchemaBody::Dynamic) => true,
        (SchemaBody::Option(source), SchemaBody::Option(target)) => {
            closed_schema_rebind_compatible(source, target)
        }
        (SchemaBody::Tuple(source), SchemaBody::Tuple(target)) => {
            source.len() == target.len()
                && source
                    .iter()
                    .zip(target)
                    .all(|(source, target)| closed_schema_rebind_compatible(source, target))
        }
        (SchemaBody::Record(source), SchemaBody::Record(target)) => {
            fields_rebind_compatible(source, target)
        }
        (
            SchemaBody::Matrix {
                element: source_element,
                dimensions: source_dimensions,
            },
            SchemaBody::Matrix {
                element: target_element,
                dimensions: target_dimensions,
            },
        ) => {
            source_dimensions == target_dimensions
                && closed_schema_rebind_compatible(source_element, target_element)
        }
        (
            SchemaBody::Table {
                columns: source_columns,
                rows: source_rows,
            },
            SchemaBody::Table {
                columns: target_columns,
                rows: target_rows,
            },
        ) => {
            (source_rows == target_rows || dynamic_target(target_rows))
                && fields_rebind_compatible(source_columns, target_columns)
        }
        (
            SchemaBody::Set {
                element: source_element,
                cardinality: source_cardinality,
            },
            SchemaBody::Set {
                element: target_element,
                cardinality: target_cardinality,
            },
        ) => {
            // Canonical bytecode preserves a set RuntimeType's capacity as a
            // dynamic bound. Source typing may still assign the particular
            // literal an exact cardinality. Rebinding is value-level, and the
            // target schema is validated after this compatibility check, so a
            // dynamic source may safely close to an exact target only when
            // the payload actually has that cardinality.
            (source_cardinality == target_cardinality
                || matches!(source_cardinality, crate::CardinalitySpec::Dynamic { .. })
                || dynamic_target(target_cardinality))
                && closed_schema_rebind_compatible(source_element, target_element)
        }
        (
            SchemaBody::Map {
                key: source_key,
                value: source_value,
                cardinality: source_cardinality,
            },
            SchemaBody::Map {
                key: target_key,
                value: target_value,
                cardinality: target_cardinality,
            },
        ) => {
            (source_cardinality == target_cardinality || dynamic_target(target_cardinality))
                && closed_schema_rebind_compatible(source_key, target_key)
                && closed_schema_rebind_compatible(source_value, target_value)
        }
        _ => false,
    }
}

fn adapt_dynamic_bytecode_placeholders(
    source: &SchemaBody,
    target: &SchemaBody,
    draft: ValueDataDraft,
    path: &SnapshotPath,
) -> Result<ValueDataDraft, SnapshotValueError> {
    if source == target {
        return Ok(draft);
    }
    let actual = draft.kind();
    match (source, target, draft) {
        (SchemaBody::ReifiedType, SchemaBody::Dynamic, ValueDataDraft::Type(_)) => {
            Ok(ValueDataDraft::Dynamic(None))
        }
        (SchemaBody::Option(source), SchemaBody::Option(target), ValueDataDraft::Option(draft)) => {
            Ok(ValueDataDraft::Option(OptionDraft {
                present: draft.present,
                value: draft
                    .value
                    .map(|value| {
                        adapt_dynamic_bytecode_placeholders(
                            source,
                            target,
                            *value,
                            &path.child(SnapshotPathSegment::OptionValue),
                        )
                        .map(Box::new)
                    })
                    .transpose()?,
            }))
        }
        (SchemaBody::Tuple(source), SchemaBody::Tuple(target), ValueDataDraft::Tuple(values))
            if source.len() == target.len() && source.len() == values.len() =>
        {
            Ok(ValueDataDraft::Tuple(
                source
                    .iter()
                    .zip(target)
                    .zip(values.into_vec())
                    .enumerate()
                    .map(|(index, ((source, target), value))| {
                        adapt_dynamic_bytecode_placeholders(
                            source,
                            target,
                            value,
                            &path.child(SnapshotPathSegment::TupleElement(index as u32)),
                        )
                    })
                    .collect::<Result<Vec<_>, _>>()?
                    .into_boxed_slice(),
            ))
        }
        (
            SchemaBody::Record(source),
            SchemaBody::Record(target),
            ValueDataDraft::Record(values),
        ) if source.len() == target.len() && source.len() == values.len() => {
            Ok(ValueDataDraft::Record(
                source
                    .iter()
                    .zip(target)
                    .zip(values.into_vec())
                    .enumerate()
                    .map(|(index, ((source, target), value))| {
                        Ok(NamedValueDraft {
                            name: value.name,
                            value: adapt_dynamic_bytecode_placeholders(
                                &source.schema,
                                &target.schema,
                                value.value,
                                &path.child(SnapshotPathSegment::RecordField(index as u32)),
                            )?,
                        })
                    })
                    .collect::<Result<Vec<_>, SnapshotValueError>>()?
                    .into_boxed_slice(),
            ))
        }
        (
            SchemaBody::Table {
                columns: source, ..
            },
            SchemaBody::Table {
                columns: target, ..
            },
            ValueDataDraft::Table(columns),
        ) if source.len() == target.len() && source.len() == columns.len() => {
            Ok(ValueDataDraft::Table(
                source
                    .iter()
                    .zip(target)
                    .zip(columns.into_vec())
                    .enumerate()
                    .map(|(column_index, ((source, target), column))| {
                        Ok(TableColumnDraft {
                            name: column.name,
                            values: column
                                .values
                                .into_vec()
                                .into_iter()
                                .enumerate()
                                .map(|(row_index, value)| {
                                    adapt_dynamic_bytecode_placeholders(
                                        &source.schema,
                                        &target.schema,
                                        value,
                                        &path
                                            .child(SnapshotPathSegment::TableColumn(
                                                column_index as u32,
                                            ))
                                            .child(SnapshotPathSegment::TableRow(row_index as u64)),
                                    )
                                })
                                .collect::<Result<Vec<_>, _>>()?
                                .into_boxed_slice(),
                        })
                    })
                    .collect::<Result<Vec<_>, SnapshotValueError>>()?
                    .into_boxed_slice(),
            ))
        }
        (
            SchemaBody::Matrix {
                element: source, ..
            },
            SchemaBody::Matrix {
                element: target, ..
            },
            ValueDataDraft::Matrix(values),
        ) => Ok(ValueDataDraft::Matrix(
            values
                .into_vec()
                .into_iter()
                .enumerate()
                .map(|(index, value)| {
                    adapt_dynamic_bytecode_placeholders(
                        source,
                        target,
                        value,
                        &path.child(SnapshotPathSegment::MatrixElement(index as u64)),
                    )
                })
                .collect::<Result<Vec<_>, _>>()?
                .into_boxed_slice(),
        )),
        (
            SchemaBody::Set {
                element: source, ..
            },
            SchemaBody::Set {
                element: target, ..
            },
            ValueDataDraft::Set(values),
        ) => Ok(ValueDataDraft::Set(
            values
                .into_vec()
                .into_iter()
                .enumerate()
                .map(|(index, value)| {
                    adapt_dynamic_bytecode_placeholders(
                        source,
                        target,
                        value,
                        &path.child(SnapshotPathSegment::SetElement(index as u64)),
                    )
                })
                .collect::<Result<Vec<_>, _>>()?
                .into_boxed_slice(),
        )),
        (
            SchemaBody::Map {
                key: source_key,
                value: source_value,
                ..
            },
            SchemaBody::Map {
                key: target_key,
                value: target_value,
                ..
            },
            ValueDataDraft::Map(entries),
        ) => Ok(ValueDataDraft::Map(
            entries
                .into_vec()
                .into_iter()
                .enumerate()
                .map(|(index, entry)| {
                    ensure_arity(path, 2, entry.items.len())?;
                    let mut items = entry.items.into_vec().into_iter();
                    let key = items.next().expect("validated map entry key exists");
                    let value = items.next().expect("validated map entry value exists");
                    Ok(MapEntryDraft {
                        items: vec![
                            adapt_dynamic_bytecode_placeholders(
                                source_key,
                                target_key,
                                key,
                                &path.child(SnapshotPathSegment::MapKey(index as u64)),
                            )?,
                            adapt_dynamic_bytecode_placeholders(
                                source_value,
                                target_value,
                                value,
                                &path.child(SnapshotPathSegment::MapValue(index as u64)),
                            )?,
                        ]
                        .into_boxed_slice(),
                    })
                })
                .collect::<Result<Vec<_>, SnapshotValueError>>()?
                .into_boxed_slice(),
        )),
        _ => Err(data_mismatch_kind(target, actual, path)),
    }
}

fn fields_rebind_compatible(source: &[crate::SchemaField], target: &[crate::SchemaField]) -> bool {
    source.len() == target.len()
        && source.iter().zip(target).all(|(source, target)| {
            source.name == target.name
                && closed_schema_rebind_compatible(&source.schema, &target.schema)
        })
}

fn rebuild_kind_mismatch(schema: &SchemaBody, actual: super::ValueDataKind) -> SnapshotValueError {
    SnapshotValueError::SnapshotDataSchemaMismatch {
        path: SnapshotPath::root(),
        expected: schema_kind(schema),
        actual,
    }
}

fn canonical_data_to_draft(
    schema: &SchemaBody,
    data: &ValueData,
    path: &SnapshotPath,
) -> Result<ValueDataDraft, SnapshotValueError> {
    canonical_data_to_draft_with_target(schema, data, path, None)
}

/// Materializes a canonical draft for schema-directed data that has already
/// been validated as part of a finalized snapshot value.
pub fn canonical_snapshot_data_draft(
    schema: &SchemaBody,
    data: &ValueData,
) -> Result<ValueDataDraft, SnapshotValueError> {
    canonical_data_to_draft(schema, data, &SnapshotPath::root())
}

fn canonical_data_to_rebound_draft(
    schema: &SchemaBody,
    data: &ValueData,
    path: &SnapshotPath,
    target_schemas: &SchemaTable,
) -> Result<ValueDataDraft, SnapshotValueError> {
    canonical_data_to_draft_with_target(schema, data, path, Some(target_schemas))
}

fn canonical_data_to_draft_with_target(
    schema: &SchemaBody,
    data: &ValueData,
    path: &SnapshotPath,
    target_schemas: Option<&SchemaTable>,
) -> Result<ValueDataDraft, SnapshotValueError> {
    let draft = match (schema, data) {
        (SchemaBody::Dynamic, ValueData::Dynamic(value)) => {
            let value = value
                .value()
                .map(|value| -> Result<Box<ValueDraft>, SnapshotValueError> {
                    let rebound;
                    let value = if let Some(target_schemas) = target_schemas {
                        let schema = target_schemas.find_by_key(value.schema_key()).ok_or(
                            SnapshotValueError::SnapshotSchemaTableMismatch {
                                schema: value.schema(),
                                expected: value.schema_key(),
                                actual: target_schemas
                                    .entry(value.schema())
                                    .map(|entry| entry.key()),
                            },
                        )?;
                        rebound = value.rebind(schema, value.shape(), target_schemas)?;
                        &rebound
                    } else {
                        value
                    };
                    Ok(Box::new(ValueDraft {
                        schema: value.schema(),
                        shape_values: value.shape().parameter_values().to_vec().into_boxed_slice(),
                        data: value.canonical_data_draft()?,
                    }))
                })
                .transpose()?;
            ValueDataDraft::Dynamic(value)
        }
        (SchemaBody::UnsignedInteger(IntegerWidth::W8), ValueData::U8(value)) => {
            ValueDataDraft::U8(*value)
        }
        (SchemaBody::UnsignedInteger(IntegerWidth::W16), ValueData::U16(value)) => {
            ValueDataDraft::U16(*value)
        }
        (SchemaBody::UnsignedInteger(IntegerWidth::W32), ValueData::U32(value)) => {
            ValueDataDraft::U32(*value)
        }
        (SchemaBody::UnsignedInteger(IntegerWidth::W64), ValueData::U64(value)) => {
            ValueDataDraft::U64(*value)
        }
        (SchemaBody::UnsignedInteger(IntegerWidth::W128), ValueData::U128(value)) => {
            ValueDataDraft::U128(*value)
        }
        (SchemaBody::SignedInteger(IntegerWidth::W8), ValueData::I8(value)) => {
            ValueDataDraft::I8(*value)
        }
        (SchemaBody::SignedInteger(IntegerWidth::W16), ValueData::I16(value)) => {
            ValueDataDraft::I16(*value)
        }
        (SchemaBody::SignedInteger(IntegerWidth::W32), ValueData::I32(value)) => {
            ValueDataDraft::I32(*value)
        }
        (SchemaBody::SignedInteger(IntegerWidth::W64), ValueData::I64(value)) => {
            ValueDataDraft::I64(*value)
        }
        (SchemaBody::SignedInteger(IntegerWidth::W128), ValueData::I128(value)) => {
            ValueDataDraft::I128(*value)
        }
        (SchemaBody::FloatingPoint(FloatWidth::W32), ValueData::F32(value)) => {
            ValueDataDraft::F32(*value)
        }
        (SchemaBody::FloatingPoint(FloatWidth::W64), ValueData::F64(value)) => {
            ValueDataDraft::F64(*value)
        }
        (SchemaBody::Complex(FloatWidth::W32), ValueData::Complex32(value)) => {
            ValueDataDraft::Complex32(*value)
        }
        (SchemaBody::Complex(FloatWidth::W64), ValueData::Complex64(value)) => {
            ValueDataDraft::Complex64(*value)
        }
        (SchemaBody::Rational64, ValueData::Rational64(value)) => ValueDataDraft::Rational64 {
            numerator: value.numerator(),
            denominator: value.denominator(),
        },
        (SchemaBody::Bool, ValueData::Bool(value)) => ValueDataDraft::Bool(*value),
        (SchemaBody::String, ValueData::String(value)) => {
            ValueDataDraft::String(String::from(value.as_ref()))
        }
        (SchemaBody::Id, ValueData::Id(value)) => ValueDataDraft::Id(*value),
        (SchemaBody::Index, ValueData::Index(value)) => ValueDataDraft::Index(*value),
        (SchemaBody::Atom(_), ValueData::Atom) => ValueDataDraft::Atom,
        (SchemaBody::Enum { variants, .. }, ValueData::Enum(value)) => {
            let variant = variants.get(value.ordinal() as usize).ok_or_else(|| {
                SnapshotValueError::EnumOrdinalOutOfRangeV1 {
                    path: path.clone(),
                    ordinal: value.ordinal(),
                    variants: variants.len() as u32,
                }
            })?;
            let payload = match (variant.payload.as_ref(), value.payload()) {
                (Some(schema), Some(payload)) => {
                    Some(Box::new(canonical_data_to_draft_with_target(
                        schema,
                        payload,
                        &path.child(SnapshotPathSegment::EnumPayload(value.ordinal())),
                        target_schemas,
                    )?))
                }
                (None, None) => None,
                _ => {
                    return Err(SnapshotValueError::EnumPayloadMismatchV1 { path: path.clone() });
                }
            };
            ValueDataDraft::Enum(EnumDraft {
                ordinal: value.ordinal(),
                payload,
            })
        }
        (SchemaBody::Option(element), ValueData::Option(value)) => {
            let value = value
                .as_deref()
                .map(|value| {
                    canonical_data_to_draft_with_target(
                        element,
                        value,
                        &path.child(SnapshotPathSegment::OptionValue),
                        target_schemas,
                    )
                    .map(Box::new)
                })
                .transpose()?;
            ValueDataDraft::Option(OptionDraft {
                present: value.is_some(),
                value,
            })
        }
        (SchemaBody::Tuple(elements), ValueData::Tuple(values)) => {
            ensure_arity(path, elements.len(), values.len())?;
            let values = elements
                .iter()
                .zip(values.iter())
                .enumerate()
                .map(|(index, (schema, value))| {
                    canonical_data_to_draft_with_target(
                        schema,
                        value,
                        &path.child(SnapshotPathSegment::TupleElement(index as u32)),
                        target_schemas,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?;
            ValueDataDraft::Tuple(values.into_boxed_slice())
        }
        (SchemaBody::Record(fields), ValueData::Record(value)) => {
            ensure_arity(path, fields.len(), value.fields().len())?;
            let values = fields
                .iter()
                .zip(value.fields().iter())
                .enumerate()
                .map(|(index, (field, value))| {
                    Ok(NamedValueDraft {
                        name: field.name.clone(),
                        value: canonical_data_to_draft_with_target(
                            &field.schema,
                            value,
                            &path.child(SnapshotPathSegment::RecordField(index as u32)),
                            target_schemas,
                        )?,
                    })
                })
                .collect::<Result<Vec<_>, SnapshotValueError>>()?;
            ValueDataDraft::Record(values.into_boxed_slice())
        }
        (SchemaBody::Matrix { element, .. }, ValueData::Matrix(value)) => {
            if let Some(values) = value.elements.scalar_drafts(element) {
                return Ok(ValueDataDraft::Matrix(values));
            }
            let values = value
                .elements
                .to_values()
                .iter()
                .enumerate()
                .map(|(index, value)| {
                    canonical_data_to_draft_with_target(
                        element,
                        value,
                        &path.child(SnapshotPathSegment::MatrixElement(index as u64)),
                        target_schemas,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?;
            ValueDataDraft::Matrix(values.into_boxed_slice())
        }
        (SchemaBody::Table { columns, .. }, ValueData::Table(value)) => {
            ensure_arity(path, columns.len(), value.columns.len())?;
            let columns = columns
                .iter()
                .zip(value.columns.iter())
                .enumerate()
                .map(|(column_index, (column, values))| {
                    let values = values
                        .to_values()
                        .iter()
                        .enumerate()
                        .map(|(row_index, value)| {
                            canonical_data_to_draft_with_target(
                                &column.schema,
                                value,
                                &path
                                    .child(SnapshotPathSegment::TableColumn(column_index as u32))
                                    .child(SnapshotPathSegment::TableRow(row_index as u64)),
                                target_schemas,
                            )
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    Ok(TableColumnDraft {
                        name: column.name.clone(),
                        values: values.into_boxed_slice(),
                    })
                })
                .collect::<Result<Vec<_>, SnapshotValueError>>()?;
            ValueDataDraft::Table(columns.into_boxed_slice())
        }
        (SchemaBody::Set { element, .. }, ValueData::Set(value)) => {
            let values = value
                .elements()
                .iter()
                .enumerate()
                .map(|(index, value)| {
                    canonical_data_to_draft_with_target(
                        element,
                        value.data(),
                        &path.child(SnapshotPathSegment::SetElement(index as u64)),
                        target_schemas,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?;
            ValueDataDraft::Set(values.into_boxed_slice())
        }
        (SchemaBody::Map { key, value, .. }, ValueData::Map(map)) => {
            let entries = map
                .entries()
                .iter()
                .enumerate()
                .map(|(index, entry)| {
                    Ok(MapEntryDraft {
                        items: vec![
                            canonical_data_to_draft_with_target(
                                key,
                                entry.key().data(),
                                &path.child(SnapshotPathSegment::MapKey(index as u64)),
                                target_schemas,
                            )?,
                            canonical_data_to_draft_with_target(
                                value,
                                entry.value(),
                                &path.child(SnapshotPathSegment::MapValue(index as u64)),
                                target_schemas,
                            )?,
                        ]
                        .into_boxed_slice(),
                    })
                })
                .collect::<Result<Vec<_>, SnapshotValueError>>()?;
            ValueDataDraft::Map(entries.into_boxed_slice())
        }
        (SchemaBody::ReifiedType, ValueData::Type(value)) => ValueDataDraft::Type(match value {
            ReifiedType::Kind(value) => {
                ReifiedTypeDraft::CanonicalKind(value.canonical_bytes().to_vec().into_boxed_slice())
            }
            ReifiedType::Schema(value) => ReifiedTypeDraft::Schema(*value),
        }),
        _ => return Err(data_mismatch_kind(schema, data.kind(), path)),
    };
    Ok(draft)
}

const RESIDENT_TOKEN_SEED: u64 = 0x6d65_6368_2d76_616c;

#[inline(always)]
fn token_word(hash: u64, word: u64) -> u64 {
    (hash.rotate_left(17) ^ word).wrapping_mul(0xd6e8_feb8_6659_fd93)
}

fn token_bytes(mut hash: u64, bytes: &[u8]) -> u64 {
    hash = token_word(hash, bytes.len() as u64);
    for byte in bytes {
        hash = token_word(hash, u64::from(*byte));
    }
    hash
}

fn token_sequence(mut hash: u64, sequence: &SequenceStorage) -> u64 {
    macro_rules! words {
        ($tag:literal, $values:expr, $convert:expr) => {{
            let values = $values;
            hash = token_word(hash, $tag);
            hash = token_word(hash, values.len() as u64);
            for value in values.iter().copied() {
                hash = token_word(hash, $convert(value));
            }
        }};
    }
    match sequence {
        SequenceStorage::U8(values) => words!(1, values, u64::from),
        SequenceStorage::U16(values) => words!(2, values, u64::from),
        SequenceStorage::U32(values) => words!(3, values, u64::from),
        SequenceStorage::U64(values) => words!(4, values, core::convert::identity),
        SequenceStorage::U128(values) => {
            hash = token_word(hash, 5);
            hash = token_word(hash, values.len() as u64);
            for value in values.iter().copied() {
                hash = token_word(hash, value as u64);
                hash = token_word(hash, (value >> 64) as u64);
            }
        }
        SequenceStorage::I8(values) => words!(6, values, |value: i8| value as u8 as u64),
        SequenceStorage::I16(values) => words!(7, values, |value: i16| value as u16 as u64),
        SequenceStorage::I32(values) => words!(8, values, |value: i32| value as u32 as u64),
        SequenceStorage::I64(values) => words!(9, values, |value: i64| value as u64),
        SequenceStorage::I128(values) => {
            hash = token_word(hash, 10);
            hash = token_word(hash, values.len() as u64);
            for value in values.iter().copied() {
                let value = value as u128;
                hash = token_word(hash, value as u64);
                hash = token_word(hash, (value >> 64) as u64);
            }
        }
        SequenceStorage::F32(values) => words!(11, values, |value: super::F32Bits| {
            u64::from(value.bits())
        }),
        SequenceStorage::F64(values) => words!(12, values, |value: super::F64Bits| value.bits()),
        SequenceStorage::Complex32(values) => {
            hash = token_word(hash, 13);
            hash = token_word(hash, values.len() as u64);
            for value in values.iter().copied() {
                hash = token_word(hash, u64::from(value.real().bits()));
                hash = token_word(hash, u64::from(value.imaginary().bits()));
            }
        }
        SequenceStorage::Complex64(values) => {
            hash = token_word(hash, 14);
            hash = token_word(hash, values.len() as u64);
            for value in values.iter().copied() {
                hash = token_word(hash, value.real().bits());
                hash = token_word(hash, value.imaginary().bits());
            }
        }
        SequenceStorage::Rational64(values) => {
            hash = token_word(hash, 15);
            hash = token_word(hash, values.len() as u64);
            for value in values.iter() {
                hash = token_word(hash, value.numerator() as u64);
                hash = token_word(hash, value.denominator());
            }
        }
        SequenceStorage::Bool(values) => words!(16, values, u64::from),
        SequenceStorage::String(values) => {
            hash = token_word(hash, 17);
            hash = token_word(hash, values.len() as u64);
            for value in values.iter() {
                hash = token_bytes(hash, value.as_bytes());
            }
        }
        SequenceStorage::Id(values) => words!(18, values, core::convert::identity),
        SequenceStorage::Index(values) => words!(19, values, core::convert::identity),
        SequenceStorage::Unit(count) => {
            hash = token_word(hash, 20);
            hash = token_word(hash, *count);
        }
        SequenceStorage::Values(values) => {
            hash = token_word(hash, 21);
            hash = token_word(hash, values.len() as u64);
            for value in values.iter() {
                hash = token_data(hash, value);
            }
        }
    }
    hash
}

fn token_data(mut hash: u64, data: &ValueData) -> u64 {
    macro_rules! scalar {
        ($tag:literal, $word:expr) => {{
            hash = token_word(hash, $tag);
            hash = token_word(hash, $word);
        }};
    }
    match data {
        ValueData::Dynamic(value) => {
            hash = token_word(hash, 31);
            hash = token_bytes(hash, &value.canonical);
        }
        ValueData::U8(value) => scalar!(1, u64::from(*value)),
        ValueData::U16(value) => scalar!(2, u64::from(*value)),
        ValueData::U32(value) => scalar!(3, u64::from(*value)),
        ValueData::U64(value) => scalar!(4, *value),
        ValueData::U128(value) => {
            scalar!(5, *value as u64);
            hash = token_word(hash, (*value >> 64) as u64);
        }
        ValueData::I8(value) => scalar!(6, *value as u8 as u64),
        ValueData::I16(value) => scalar!(7, *value as u16 as u64),
        ValueData::I32(value) => scalar!(8, *value as u32 as u64),
        ValueData::I64(value) => scalar!(9, *value as u64),
        ValueData::I128(value) => {
            let value = *value as u128;
            scalar!(10, value as u64);
            hash = token_word(hash, (value >> 64) as u64);
        }
        ValueData::F32(value) => scalar!(11, u64::from(value.bits())),
        ValueData::F64(value) => scalar!(12, value.bits()),
        ValueData::Complex32(value) => {
            scalar!(13, u64::from(value.real().bits()));
            hash = token_word(hash, u64::from(value.imaginary().bits()));
        }
        ValueData::Complex64(value) => {
            scalar!(14, value.real().bits());
            hash = token_word(hash, value.imaginary().bits());
        }
        ValueData::Rational64(value) => {
            scalar!(15, value.numerator() as u64);
            hash = token_word(hash, value.denominator());
        }
        ValueData::Bool(value) => scalar!(16, u64::from(*value)),
        ValueData::String(value) => {
            hash = token_word(hash, 17);
            hash = token_bytes(hash, value.as_bytes());
        }
        ValueData::Id(value) => scalar!(18, *value),
        ValueData::Index(value) => scalar!(19, *value),
        ValueData::Atom => hash = token_word(hash, 20),
        ValueData::Enum(value) => {
            scalar!(21, u64::from(value.ordinal()));
            match value.payload() {
                Some(payload) => {
                    hash = token_word(hash, 1);
                    hash = token_data(hash, payload);
                }
                None => hash = token_word(hash, 0),
            }
        }
        ValueData::Option(value) => {
            hash = token_word(hash, 22);
            match value.as_deref() {
                Some(payload) => {
                    hash = token_word(hash, 1);
                    hash = token_data(hash, payload);
                }
                None => hash = token_word(hash, 0),
            }
        }
        ValueData::Tuple(values) => {
            scalar!(23, values.len() as u64);
            for value in values.iter() {
                hash = token_data(hash, value);
            }
        }
        ValueData::Record(value) => {
            scalar!(24, value.fields().len() as u64);
            for field in value.fields() {
                hash = token_data(hash, field);
            }
        }
        ValueData::Matrix(value) => {
            hash = token_word(hash, 25);
            hash = token_sequence(hash, &value.elements);
        }
        ValueData::Table(value) => {
            scalar!(26, value.columns.len() as u64);
            for column in value.columns.iter() {
                hash = token_sequence(hash, column);
            }
        }
        ValueData::Set(value) => {
            scalar!(27, value.elements().len() as u64);
            for element in value.elements() {
                hash = token_data(hash, element.data());
            }
        }
        ValueData::Map(value) => {
            scalar!(28, value.entries().len() as u64);
            for entry in value.entries() {
                hash = token_data(hash, entry.key().data());
                hash = token_data(hash, entry.value());
            }
        }
        ValueData::Type(ReifiedType::Kind(value)) => {
            hash = token_word(hash, 29);
            hash = token_bytes(hash, value.canonical_bytes());
        }
        ValueData::Type(ReifiedType::Schema(value)) => {
            hash = token_word(hash, 30);
            hash = token_bytes(hash, value.as_bytes());
        }
    }
    hash
}

fn finalized_value(
    schema: SchemaId,
    schema_key: SchemaKey,
    shape: ShapeInstance,
    data: ValueData,
    schemas: Option<Arc<SchemaTable>>,
) -> Value {
    let mut resident_token = token_bytes(RESIDENT_TOKEN_SEED, schema_key.as_bytes());
    resident_token = token_word(resident_token, shape.parameter_values().len() as u64);
    for value in shape.parameter_values() {
        resident_token = token_word(resident_token, *value);
    }
    resident_token = token_data(resident_token, &data);
    let shape = Arc::new(shape);
    Value {
        schema,
        schema_key,
        shape: shape.clone(),
        root: Arc::new(FrozenSnapshotStorage {
            data: Arc::new(FrozenSnapshotData {
                data,
                shape,
                ownership: SharedOwnershipCell::new(),
                budget_imports: SnapshotBudgetRegistry::default(),
            }),
            memory_budget: None,
        }),
        resident_token,
        schemas,
    }
}

fn finalized_value_with_construction(
    schema: SchemaId,
    schema_key: SchemaKey,
    shape: ShapeInstance,
    data: ValueData,
    schemas: Option<Arc<SchemaTable>>,
    context: &SnapshotValidationContext<'_>,
) -> Result<Value, SnapshotValueError> {
    let mut resident_token = token_bytes(RESIDENT_TOKEN_SEED, schema_key.as_bytes());
    resident_token = token_word(resident_token, shape.parameter_values().len() as u64);
    for value in shape.parameter_values() {
        resident_token = token_word(resident_token, *value);
    }
    resident_token = token_data(resident_token, &data);
    // Final Arc allocations are directly owned by the immutable tree. They
    // consume the admitted finalization byte allowance, not mutable-envelope
    // block registrations, and cannot grow that registry during construction.
    let shape = context.try_arc(shape)?;
    let data = context.try_arc(FrozenSnapshotData {
        data,
        shape: shape.clone(),
        ownership: SharedOwnershipCell::new(),
        budget_imports: SnapshotBudgetRegistry::default(),
    })?;
    let root = context.try_arc(FrozenSnapshotStorage {
        data,
        memory_budget: None,
    })?;
    Ok(Value {
        schema,
        schema_key,
        shape,
        root,
        resident_token,
        schemas,
    })
}

fn dynamic_canonical(value: Option<&Value>, schema: Option<&SchemaBody>) -> Box<[u8]> {
    let Some(value) = value else {
        return Vec::from([0]).into_boxed_slice();
    };
    let schema = schema.expect("materialized dynamic values carry their concrete schema");
    let shape = value.shape().canonical_bytes();
    let payload = super::encoding::canonical_material(schema, value.data());
    let mut bytes = Vec::with_capacity(
        1 + value.schema_key().as_bytes().len() + 8 + shape.len() + 8 + payload.len(),
    );
    bytes.push(1);
    bytes.extend_from_slice(value.schema_key().as_bytes());
    bytes.extend_from_slice(&(shape.len() as u64).to_le_bytes());
    bytes.extend_from_slice(&shape);
    bytes.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    bytes.extend_from_slice(&payload);
    bytes.into_boxed_slice()
}

fn dynamic_canonical_with_construction(
    value: Option<&Value>,
    schema: Option<&SchemaBody>,
    context: &SnapshotValidationContext<'_>,
) -> Result<Box<[u8]>, SnapshotValueError> {
    let bytes = match value {
        Some(value) => {
            let footprint = value.retained_footprint(context.schemas()).map_err(|_| {
                crate::MemoryRuntimeError::InvalidLayout {
                    object: context
                        .construction_authority
                        .and_then(SnapshotConstructionAuthority::allocation_object),
                    size: u64::MAX,
                    alignment: 1,
                    reason: "dynamic canonical material footprint is invalid",
                }
            })?;
            footprint
                .encoded_bytes
                .checked_mul(2)
                .and_then(|bytes| {
                    bytes.checked_add(5_u64.checked_add(
                        (value.shape().parameter_values().len() as u64).checked_mul(8)?,
                    )?)
                })
                .ok_or(crate::MemoryRuntimeError::InvalidLayout {
                    object: context
                        .construction_authority
                        .and_then(SnapshotConstructionAuthority::allocation_object),
                    size: u64::MAX,
                    alignment: 1,
                    reason: "dynamic canonical material bound overflows",
                })?
        }
        None => 1,
    };
    if let Some(authority) = context.construction_authority {
        authority.admit_snapshot_allocation(bytes, 1)?;
    }
    Ok(dynamic_canonical(value, schema))
}

/// Wraps canonical resident data in a self-describing dynamic snapshot cell.
/// The binder has already validated the schema identity, shape, and physical
/// representation before this constructor is used.
#[doc(hidden)]
pub fn wrap_resident_dynamic_data(
    schema: SchemaId,
    schema_key: SchemaKey,
    shape: ShapeInstance,
    schemas: Arc<SchemaTable>,
    body: &SchemaBody,
    data: ValueData,
) -> ValueData {
    debug_assert_eq!(
        schemas.entry(schema).map(|entry| entry.key()),
        Some(schema_key),
        "resident dynamic values retain their authoritative schema arena"
    );
    let value = finalized_value(schema, schema_key, shape, data, Some(schemas));
    let canonical = dynamic_canonical(Some(&value), Some(body));
    ValueData::Dynamic(DynamicValue {
        value: Some(Box::new(value)),
        canonical,
    })
}

/// A canonical aggregate constructor bound to one schema arena and exact
/// child identities. Binding validates nominal kinds and resolved dimensions;
/// construction rechecks those identities before cloning any child payload.
#[derive(Clone, Debug)]
pub struct CompositeSnapshotConstructor {
    schema: SchemaId,
    shape: ShapeInstance,
    children: Box<[(SchemaKey, ShapeInstance, bool)]>,
    schemas: Arc<SchemaTable>,
}

struct CompositeSchemaComponents<'a> {
    children: Vec<&'a SchemaBody>,
    cardinality: Option<(&'a crate::CardinalitySpec, usize)>,
}

fn composite_schema_components(
    body: &SchemaBody,
    child_count: usize,
) -> Option<CompositeSchemaComponents<'_>> {
    let (children, cardinality) = match body {
        SchemaBody::Tuple(items) if items.len() == child_count => (items.iter().collect(), None),
        SchemaBody::Record(fields) if fields.len() == child_count => {
            (fields.iter().map(|field| &field.schema).collect(), None)
        }
        SchemaBody::Map {
            key,
            value,
            cardinality,
        } if child_count % 2 == 0 => (
            (0..child_count / 2)
                .flat_map(|_| [key.as_ref(), value.as_ref()])
                .collect(),
            Some((cardinality, child_count / 2)),
        ),
        SchemaBody::Table { columns, .. } if columns.is_empty() && child_count == 0 => {
            (Vec::new(), None)
        }
        SchemaBody::Table { columns, rows }
            if !columns.is_empty() && child_count % columns.len() == 0 =>
        {
            let rows_count = child_count / columns.len();
            (
                columns
                    .iter()
                    .flat_map(|column| core::iter::repeat_n(&column.schema, rows_count))
                    .collect(),
                Some((rows, rows_count)),
            )
        }
        _ => return None,
    };
    Some(CompositeSchemaComponents {
        children,
        cardinality,
    })
}

impl CompositeSnapshotConstructor {
    /// Derives the aggregate's shape from the canonical child schemas in
    /// constructor order. Each child shape belongs to its own parameter arena.
    pub fn shape_for_children(
        schema: SchemaId,
        children: &[(SchemaId, ShapeInstance)],
        schemas: &SchemaTable,
    ) -> Result<ShapeInstance, SnapshotValueError> {
        let entry = schemas
            .entry(schema)
            .ok_or(SnapshotValueError::UnknownSnapshotSchema { schema })?;
        let mismatch = || SnapshotValueError::SnapshotSchemaDefinitionMismatch { key: entry.key() };
        // An empty column list gives no row-count witness. Preserve binding
        // support for explicitly shaped tables, but do not infer a Turn fact
        // from an absent child or a lower-bound placeholder.
        if matches!(entry.schema().body(), SchemaBody::Table { columns, .. } if columns.is_empty())
            && !entry.schema().dimension_parameters().is_empty()
        {
            return Err(mismatch());
        }
        let layout = composite_schema_components(entry.schema().body(), children.len())
            .ok_or_else(mismatch)?;
        let components = layout
            .children
            .into_iter()
            .zip(children)
            .map(|(expected, (schema, shape))| {
                let actual = schemas
                    .get(*schema)
                    .ok_or(SnapshotValueError::UnknownSnapshotSchema { schema: *schema })?
                    .closed_body(shape)
                    .map_err(|_| mismatch())?;
                Ok((expected, actual))
            })
            .collect::<Result<Vec<_>, SnapshotValueError>>()?;
        let shape = crate::type_system::resolved_value::shape_for_schema_components(
            entry.schema(),
            &components,
            layout.cardinality,
        )
        .map_err(|_| mismatch())?;
        if let Some((cardinality, count)) = layout.cardinality {
            ensure_collection_cardinality(&SnapshotPath::root(), cardinality, &shape, count)?;
        }
        Ok(shape)
    }
    pub fn bind(
        schema: SchemaId,
        shape: ShapeInstance,
        children: &[(SchemaId, ShapeInstance)],
        schemas: Arc<SchemaTable>,
    ) -> Result<Self, SnapshotValueError> {
        let entry = schemas
            .entry(schema)
            .ok_or(SnapshotValueError::UnknownSnapshotSchema { schema })?;
        let mismatch = || SnapshotValueError::SnapshotSchemaDefinitionMismatch { key: entry.key() };
        let body = entry.schema().closed_body(&shape).map_err(|_| mismatch())?;
        let expected = if let SchemaBody::Matrix {
            element,
            dimensions,
        } = &body
        {
            let expected = dimensions.iter().try_fold(1u64, |size, dimension| {
                size.checked_mul(shape.resolve_dimension(dimension).map_err(|_| mismatch())?)
                    .ok_or_else(mismatch)
            })?;
            ensure_cardinality(&SnapshotPath::root(), expected, children.len())?;
            core::iter::repeat_n(element.as_ref(), children.len()).collect::<Vec<_>>()
        } else {
            let layout = composite_schema_components(&body, children.len()).ok_or_else(mismatch)?;
            if let Some((cardinality, count)) = layout.cardinality {
                ensure_collection_cardinality(&SnapshotPath::root(), cardinality, &shape, count)?;
            }
            layout.children
        };
        if expected.len() != children.len() {
            return Err(mismatch());
        }
        let children = children
            .iter()
            .zip(expected)
            .map(|((schema, shape), expected)| {
                let entry = schemas
                    .entry(*schema)
                    .ok_or(SnapshotValueError::UnknownSnapshotSchema { schema: *schema })?;
                let dynamic = matches!(expected, SchemaBody::Dynamic);
                let actual = entry.schema().closed_body(shape).map_err(|_| mismatch())?;
                if !dynamic && &actual != expected {
                    return Err(mismatch());
                }
                Ok((entry.key(), shape.clone(), dynamic))
            })
            .collect::<Result<Vec<_>, SnapshotValueError>>()?
            .into_boxed_slice();
        Ok(Self {
            schema,
            shape,
            children,
            schemas,
        })
    }

    /// Owned canonical value and shape containers, excluding immutable schema
    /// storage and payloads. Native resident children use the same bound.
    pub fn value_container_bytes(shape_parameters: usize) -> Option<usize> {
        core::mem::size_of::<Value>()
            .checked_add(core::mem::size_of::<FrozenSnapshotStorage>())?
            .checked_add(core::mem::size_of::<FrozenSnapshotData>())?
            .checked_add(core::mem::size_of::<ShapeInstance>())?
            .checked_add(shape_parameters.checked_mul(core::mem::size_of::<u64>())?)
    }

    /// Constructor-owned containers, excluding child payloads (supplied by
    /// their canonical footprints). Table packing temporarily stages one
    /// column of ValueData before producing its packed sequence.
    pub fn allocation_containers(&self) -> Option<(usize, usize)> {
        let body = self.schemas.get(self.schema)?.body();
        let root = Self::value_container_bytes(self.shape.parameter_values().len())?;
        let (aggregate, scratch) = match body {
            SchemaBody::Map { .. } => (
                (self.children.len() / 2)
                    .checked_mul(core::mem::size_of::<super::MapEntryValue>())?,
                0,
            ),
            SchemaBody::Table { columns, .. } => (
                columns
                    .len()
                    .checked_mul(core::mem::size_of::<SequenceStorage>())?,
                if columns.is_empty() {
                    0
                } else {
                    (self.children.len() / columns.len())
                        .checked_mul(core::mem::size_of::<ValueData>())?
                },
            ),
            _ => (0, 0),
        };
        Some((root.checked_add(aggregate)?, scratch))
    }

    pub fn construct(
        &self,
        children: Box<[Value]>,
        budget: Option<&SnapshotCanonicalizationBudget>,
    ) -> Result<Value, SnapshotValueError> {
        let schema = self.schema;
        let shape = &self.shape;
        let entry = self
            .schemas
            .entry(schema)
            .expect("bound output schema remains present");
        let body = entry.schema().body();
        let path = SnapshotPath::root();
        let actual = children.len();
        let mismatch = |expected: usize| SnapshotValueError::AggregateArityMismatchV1 {
            path: path.clone(),
            expected: expected as u64,
            actual: actual as u64,
        };
        if actual != self.children.len() {
            return Err(mismatch(self.children.len()));
        }
        for (child, (key, shape, _)) in children.iter().zip(self.children.iter()) {
            if child.schema_key() != *key || child.shape() != shape {
                return Err(SnapshotValueError::SnapshotSchemaDefinitionMismatch { key: *key });
            }
        }
        let children = children
            .into_vec()
            .into_iter()
            .zip(self.children.iter())
            .map(|(child, (_, _, dynamic))| {
                if *dynamic && !matches!(child.data(), ValueData::Dynamic(_)) {
                    let schemas = child
                        .schemas()
                        .expect("canonical input retains its schema context");
                    let body = schemas
                        .get(child.schema())
                        .expect("validated input schema remains present")
                        .body();
                    wrap_resident_dynamic_data(
                        child.schema(),
                        child.schema_key(),
                        child.shape().clone(),
                        Arc::clone(&schemas),
                        body,
                        child.data().clone(),
                    )
                } else {
                    child.data().clone()
                }
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();
        let data = match body {
            SchemaBody::Tuple(items) => {
                if items.len() != actual {
                    return Err(mismatch(items.len()));
                }
                ValueData::Tuple(children)
            }
            SchemaBody::Record(fields) => {
                if fields.len() != actual {
                    return Err(mismatch(fields.len()));
                }
                ValueData::Record(RecordValue { fields: children })
            }
            SchemaBody::Map {
                key, cardinality, ..
            } => {
                if actual % 2 != 0 {
                    return Err(mismatch(actual + 1));
                }
                ensure_collection_cardinality(&path, cardinality, shape, actual / 2)?;
                let mut entries = Vec::with_capacity(actual / 2);
                let mut children = children.into_vec().into_iter();
                while let Some(key_data) = children.next() {
                    let value = children.next().expect("checked map pair arity");
                    super::relations::insert_map_entry(
                        key,
                        &mut entries,
                        key_data,
                        value,
                        &path,
                        budget,
                    )?;
                }
                ValueData::Map(MapValue {
                    entries: entries.into_boxed_slice(),
                })
            }
            SchemaBody::Table { columns, rows } => {
                let row_count = if columns.is_empty() {
                    if actual != 0 {
                        return Err(mismatch(0));
                    }
                    match rows {
                        crate::CardinalitySpec::Exact(n) => shape.resolve_dimension(n)?,
                        _ => 0,
                    }
                } else {
                    if actual % columns.len() != 0 {
                        return Err(mismatch(columns.len()));
                    }
                    (actual / columns.len()) as u64
                };
                ensure_collection_cardinality(&path, rows, shape, row_count as usize)?;
                let mut children = children.into_vec().into_iter();
                let columns = columns
                    .iter()
                    .map(|column| {
                        SequenceStorage::from_values(
                            &column.schema,
                            children.by_ref().take(row_count as usize).collect(),
                        )
                    })
                    .collect::<Vec<_>>()
                    .into_boxed_slice();
                ValueData::Table(TableValue { columns })
            }
            SchemaBody::Matrix {
                element,
                dimensions,
            } => {
                let expected = dimensions.iter().try_fold(1u64, |size, dimension| {
                    size.checked_mul(shape.resolve_dimension(dimension)?)
                        .ok_or_else(|| mismatch(usize::MAX))
                })?;
                if expected != actual as u64 {
                    return Err(mismatch(expected as usize));
                }
                ValueData::Matrix(MatrixValue {
                    elements: SequenceStorage::from_values(element, children.into_vec()),
                })
            }
            _ => return Err(mismatch(0)),
        };
        Ok(finalized_value(
            schema,
            entry.key(),
            shape.clone(),
            data,
            Some(Arc::clone(&self.schemas)),
        ))
    }
}

/// Rebuilds a canonical `set<f64>` snapshot from candidate values while
/// preserving the output template's authoritative schema and shape.
/// Duplicate candidates use the same normalized key equality as ordinary
/// snapshot finalization.
pub fn rebuild_f64_set_snapshot(template: &Value, candidates: &[f64]) -> Option<Value> {
    let ValueData::Set(expected) = template.data() else {
        return None;
    };
    if expected
        .elements()
        .iter()
        .any(|element| !matches!(element.data(), ValueData::F64(_)))
    {
        return None;
    }

    build_f64_set_snapshot(
        template.schema,
        template.schema_key,
        template.shape.as_ref().clone(),
        template.schemas.as_deref()?,
        Some(expected.elements().len()),
        Some(expected.elements().len()),
        candidates,
    )
}

/// Constructs a canonical `set<f64>` snapshot for an exact or dynamic
/// cardinality contract from metadata already validated by resident
/// activation.
pub fn build_f64_set_snapshot(
    schema: SchemaId,
    schema_key: SchemaKey,
    shape: ShapeInstance,
    schemas: &SchemaTable,
    exact_cardinality: Option<usize>,
    maximum_cardinality: Option<usize>,
    candidates: &[f64],
) -> Option<Value> {
    let element_schema = SchemaBody::FloatingPoint(FloatWidth::W64);
    let mut elements = Vec::with_capacity(candidates.len());
    for (index, candidate) in candidates.iter().copied().enumerate() {
        let data = super::relations::normalized_key_data(
            &element_schema,
            ValueData::F64(super::F64Bits::from_f64(candidate)),
        )
        .ok()?;
        let duplicate = elements.iter().any(|existing: &CanonicalKeyValue| {
            super::relations::compare_key_data(&element_schema, existing.data(), &data)
                .is_ok_and(|order| order == core::cmp::Ordering::Equal)
        });
        if !duplicate {
            super::relations::insert_set_key(
                &element_schema,
                &mut elements,
                data,
                &SnapshotPath::root().child(SnapshotPathSegment::SetElement(index as u64)),
                None,
            )
            .ok()?;
        }
    }
    if exact_cardinality.is_some_and(|expected| elements.len() != expected)
        || maximum_cardinality.is_some_and(|maximum| elements.len() > maximum)
    {
        return None;
    }
    Some(finalized_value(
        schema,
        schema_key,
        shape,
        ValueData::Set(SetValue {
            elements: elements.into_boxed_slice(),
        }),
        Some(Arc::new(schemas.clone())),
    ))
}

/// Tests membership in a canonical `set<f64>` snapshot with set-key float
/// normalization (`-0.0` and NaN payloads included).
pub fn f64_set_snapshot_contains(value: &Value, candidate: f64) -> Option<bool> {
    let ValueData::Set(set) = value.data() else {
        return None;
    };
    let element_schema = SchemaBody::FloatingPoint(FloatWidth::W64);
    let candidate = super::relations::normalized_key_data(
        &element_schema,
        ValueData::F64(super::F64Bits::from_f64(candidate)),
    )
    .ok()?;
    set.elements()
        .iter()
        .map(|element| {
            super::relations::compare_key_data(&element_schema, element.data(), &candidate)
        })
        .collect::<Result<Vec<_>, _>>()
        .ok()
        .map(|orders| {
            orders
                .into_iter()
                .any(|order| order == core::cmp::Ordering::Equal)
        })
}

/// Constructs the canonical output of removing one `f64` element from a
/// resident set snapshot. Key comparison uses the same normalization as set
/// construction, including signed zero and NaN payloads.
pub fn build_f64_set_snapshot_after_remove(
    schema: SchemaId,
    schema_key: SchemaKey,
    shape: ShapeInstance,
    schemas: &SchemaTable,
    exact_cardinality: Option<usize>,
    maximum_cardinality: Option<usize>,
    source: &Value,
    candidate: f64,
) -> Option<Value> {
    let ValueData::Set(set) = source.data() else {
        return None;
    };
    let element_schema = SchemaBody::FloatingPoint(FloatWidth::W64);
    let candidate = super::relations::normalized_key_data(
        &element_schema,
        ValueData::F64(super::F64Bits::from_f64(candidate)),
    )
    .ok()?;
    let values = set
        .elements()
        .iter()
        .filter_map(|element| {
            let equal =
                super::relations::compare_key_data(&element_schema, element.data(), &candidate)
                    .ok()
                    == Some(core::cmp::Ordering::Equal);
            if equal {
                None
            } else {
                match element.data() {
                    ValueData::F64(value) => Some(value.to_f64()),
                    _ => None,
                }
            }
        })
        .collect::<Vec<_>>();
    if values.len()
        + usize::from(f64_set_snapshot_contains(
            source,
            candidate_f64(&candidate),
        )?)
        != set.elements().len()
    {
        return None;
    }
    build_f64_set_snapshot(
        schema,
        schema_key,
        shape,
        schemas,
        exact_cardinality,
        maximum_cardinality,
        &values,
    )
}

fn candidate_f64(candidate: &ValueData) -> f64 {
    match candidate {
        ValueData::F64(value) => value.to_f64(),
        _ => unreachable!("f64 candidate normalization preserves its data kind"),
    }
}

pub(super) fn finalize_value(
    draft: ValueDraft,
    context: &SnapshotValidationContext<'_>,
) -> Result<Value, SnapshotValueError> {
    let entry =
        context
            .schemas
            .entry(draft.schema)
            .ok_or(SnapshotValueError::UnknownSnapshotSchema {
                schema: draft.schema,
            })?;
    let shape = entry.schema().instantiate_shape(draft.shape_values)?;
    let path = SnapshotPath::root();
    let data = finalize_data(entry.schema().body(), draft.data, &shape, context, &path)?;
    let schemas = context.try_clone_schemas()?;
    finalized_value_with_construction(
        draft.schema,
        entry.key(),
        shape,
        data,
        Some(schemas),
        context,
    )
}

pub(super) fn finalize_data(
    schema: &SchemaBody,
    draft: ValueDataDraft,
    shape: &ShapeInstance,
    context: &SnapshotValidationContext<'_>,
    path: &SnapshotPath,
) -> Result<ValueData, SnapshotValueError> {
    let actual_kind = draft.kind();
    macro_rules! exact {
        ($schema:pat, $draft:pat => $value:expr) => {
            if matches!(schema, $schema) {
                if let $draft = draft {
                    return Ok($value);
                }
                return Err(data_mismatch_kind(schema, actual_kind, path));
            }
        };
    }

    exact!(SchemaBody::Bool, ValueDataDraft::Bool(value) => ValueData::Bool(value));
    exact!(SchemaBody::UnsignedInteger(IntegerWidth::W8), ValueDataDraft::U8(value) => ValueData::U8(value));
    exact!(SchemaBody::UnsignedInteger(IntegerWidth::W16), ValueDataDraft::U16(value) => ValueData::U16(value));
    exact!(SchemaBody::UnsignedInteger(IntegerWidth::W32), ValueDataDraft::U32(value) => ValueData::U32(value));
    exact!(SchemaBody::UnsignedInteger(IntegerWidth::W64), ValueDataDraft::U64(value) => ValueData::U64(value));
    exact!(SchemaBody::UnsignedInteger(IntegerWidth::W128), ValueDataDraft::U128(value) => ValueData::U128(value));
    exact!(SchemaBody::SignedInteger(IntegerWidth::W8), ValueDataDraft::I8(value) => ValueData::I8(value));
    exact!(SchemaBody::SignedInteger(IntegerWidth::W16), ValueDataDraft::I16(value) => ValueData::I16(value));
    exact!(SchemaBody::SignedInteger(IntegerWidth::W32), ValueDataDraft::I32(value) => ValueData::I32(value));
    exact!(SchemaBody::SignedInteger(IntegerWidth::W64), ValueDataDraft::I64(value) => ValueData::I64(value));
    exact!(SchemaBody::SignedInteger(IntegerWidth::W128), ValueDataDraft::I128(value) => ValueData::I128(value));
    exact!(SchemaBody::FloatingPoint(FloatWidth::W32), ValueDataDraft::F32(value) => ValueData::F32(value));
    exact!(SchemaBody::FloatingPoint(FloatWidth::W64), ValueDataDraft::F64(value) => ValueData::F64(value));
    exact!(SchemaBody::Complex(FloatWidth::W32), ValueDataDraft::Complex32(value) => ValueData::Complex32(value));
    exact!(SchemaBody::Complex(FloatWidth::W64), ValueDataDraft::Complex64(value) => ValueData::Complex64(value));
    if matches!(schema, SchemaBody::String) {
        if let ValueDataDraft::String(value) = draft {
            return Ok(ValueData::String(context.try_boxed_str(value)?));
        }
        return Err(data_mismatch_kind(schema, actual_kind, path));
    }
    exact!(SchemaBody::Id, ValueDataDraft::Id(value) => ValueData::Id(value));
    if matches!(schema, SchemaBody::Index) {
        if let ValueDataDraft::Index(value) = draft {
            if value == 0 {
                return Err(SnapshotValueError::InvalidIndexV1 {
                    path: path.clone(),
                    value,
                });
            }
            return Ok(ValueData::Index(value));
        }
        return Err(data_mismatch_kind(schema, actual_kind, path));
    }
    exact!(SchemaBody::Atom(_), ValueDataDraft::Atom => ValueData::Atom);

    match (schema, draft) {
        (SchemaBody::Dynamic, ValueDataDraft::Dynamic(draft)) => {
            let value = match draft {
                Some(draft) => Some(context.try_box(finalize_value(*draft, context)?)?),
                None => None,
            };
            let concrete = value
                .as_deref()
                .map(|value| value.validate_against(context.schemas))
                .transpose()?;
            let canonical = dynamic_canonical_with_construction(
                value.as_deref(),
                concrete.map(Schema::body),
                context,
            )?;
            Ok(ValueData::Dynamic(DynamicValue { value, canonical }))
        }
        (
            SchemaBody::Rational64,
            ValueDataDraft::Rational64 {
                numerator,
                denominator,
            },
        ) => Ok(ValueData::Rational64(super::Rational64Value::new(
            numerator,
            denominator,
        )?)),
        (SchemaBody::Enum { variants, .. }, ValueDataDraft::Enum(draft)) => {
            let variant = variants.get(draft.ordinal as usize).ok_or(
                SnapshotValueError::EnumOrdinalOutOfRangeV1 {
                    path: path.clone(),
                    ordinal: draft.ordinal,
                    variants: variants.len() as u32,
                },
            )?;
            let payload_path = path.child(SnapshotPathSegment::EnumPayload(draft.ordinal));
            let payload = match (&variant.payload, draft.payload) {
                (None, None) => None,
                (Some(schema), Some(payload)) => Some(context.try_box(finalize_data(
                    schema,
                    *payload,
                    shape,
                    context,
                    &payload_path,
                )?)?),
                _ => {
                    return Err(SnapshotValueError::EnumPayloadMismatchV1 { path: path.clone() });
                }
            };
            Ok(ValueData::Enum(EnumValue {
                ordinal: draft.ordinal,
                payload,
            }))
        }
        (SchemaBody::Option(element), ValueDataDraft::Option(draft)) => {
            let value = match (draft.present, draft.value) {
                (false, None) => None,
                (true, Some(value)) => Some(context.try_box(finalize_data(
                    element,
                    *value,
                    shape,
                    context,
                    &path.child(SnapshotPathSegment::OptionValue),
                )?)?),
                (present, value) => {
                    return Err(SnapshotValueError::PayloadCardinalityMismatchV1 {
                        path: path.clone(),
                        expected: u64::from(present),
                        actual: u64::from(value.is_some()),
                    });
                }
            };
            Ok(ValueData::Option(value))
        }
        (SchemaBody::Tuple(elements), ValueDataDraft::Tuple(values)) => {
            ensure_arity(path, elements.len(), values.len())?;
            let mut finalized = context.try_vec_with_capacity(values.len())?;
            for (index, (schema, draft)) in elements.iter().zip(values.into_vec()).enumerate() {
                finalized.push(finalize_data(
                    schema,
                    draft,
                    shape,
                    context,
                    &path.child(SnapshotPathSegment::TupleElement(index as u32)),
                )?);
            }
            Ok(ValueData::Tuple(finalized.into_boxed_slice()))
        }
        (SchemaBody::Record(fields), ValueDataDraft::Record(values)) => {
            let values = order_named_values(fields, values, path, context)?;
            let mut finalized = context.try_vec_with_capacity(values.len())?;
            for (index, (field, draft)) in fields.iter().zip(values).enumerate() {
                finalized.push(finalize_data(
                    &field.schema,
                    draft,
                    shape,
                    context,
                    &path.child(SnapshotPathSegment::RecordField(index as u32)),
                )?);
            }
            Ok(ValueData::Record(RecordValue {
                fields: finalized.into_boxed_slice(),
            }))
        }
        (
            SchemaBody::Matrix {
                element,
                dimensions,
            },
            ValueDataDraft::Matrix(values),
        ) => {
            let expected = resolved_product(dimensions, shape)?;
            ensure_cardinality(path, expected, values.len())?;
            if scalar_sequence_schema(element) {
                return Ok(ValueData::Matrix(MatrixValue {
                    elements: finalize_scalar_sequence(
                        element,
                        values,
                        path,
                        ScalarSequenceElement::Matrix,
                        context,
                    )?,
                }));
            }
            let mut finalized = context.try_vec_with_capacity(values.len())?;
            for (index, draft) in values.into_vec().into_iter().enumerate() {
                finalized.push(finalize_data(
                    element,
                    draft,
                    shape,
                    context,
                    &path.child(SnapshotPathSegment::MatrixElement(index as u64)),
                )?);
            }
            Ok(ValueData::Matrix(MatrixValue {
                elements: SequenceStorage::from_values(element, finalized),
            }))
        }
        (SchemaBody::Table { columns, rows }, ValueDataDraft::Table(values)) => {
            let values = order_table_columns(columns, values, path, context)?;
            let actual_rows = values.first().map_or(0, |values| values.len());
            ensure_collection_cardinality(path, rows, shape, actual_rows)?;
            let mut finalized_columns = context.try_vec_with_capacity(values.len())?;
            for (column_index, (column, drafts)) in columns.iter().zip(values).enumerate() {
                if drafts.len() != actual_rows {
                    return Err(SnapshotValueError::PayloadCardinalityMismatchV1 {
                        path: path.clone(),
                        expected: actual_rows as u64,
                        actual: drafts.len() as u64,
                    });
                }
                if scalar_sequence_schema(&column.schema) {
                    finalized_columns.push(finalize_scalar_sequence(
                        &column.schema,
                        drafts,
                        path,
                        ScalarSequenceElement::TableColumn(column_index as u32),
                        context,
                    )?);
                    continue;
                }
                let mut finalized = context.try_vec_with_capacity(drafts.len())?;
                for (row, draft) in drafts.into_vec().into_iter().enumerate() {
                    let column_path = path
                        .child(SnapshotPathSegment::TableColumn(column_index as u32))
                        .child(SnapshotPathSegment::TableRow(row as u64));
                    finalized.push(finalize_data(
                        &column.schema,
                        draft,
                        shape,
                        context,
                        &column_path,
                    )?);
                }
                finalized_columns.push(SequenceStorage::from_values(&column.schema, finalized));
            }
            Ok(ValueData::Table(TableValue {
                columns: finalized_columns.into_boxed_slice(),
            }))
        }
        (
            SchemaBody::Set {
                element,
                cardinality,
            },
            ValueDataDraft::Set(values),
        ) => {
            ensure_collection_cardinality(path, cardinality, shape, values.len())?;
            let mut finalized = context.try_vec_with_capacity(values.len())?;
            for (index, draft) in values.into_vec().into_iter().enumerate() {
                let element_path = path.child(SnapshotPathSegment::SetElement(index as u64));
                let data = finalize_data(element, draft, shape, context, &element_path)?;
                super::relations::insert_set_key(
                    element,
                    &mut finalized,
                    data,
                    &element_path,
                    context.canonicalization_budget,
                )?;
            }
            Ok(ValueData::Set(SetValue {
                elements: finalized.into_boxed_slice(),
            }))
        }
        (
            SchemaBody::Map {
                key,
                value,
                cardinality,
            },
            ValueDataDraft::Map(entries),
        ) => {
            ensure_collection_cardinality(path, cardinality, shape, entries.len())?;
            let mut finalized = context.try_vec_with_capacity(entries.len())?;
            for (index, entry) in entries.into_vec().into_iter().enumerate() {
                if entry.items.len() != 2 {
                    return Err(SnapshotValueError::MapEntryArityMismatchV1 {
                        path: path.clone(),
                        actual: entry.items.len() as u64,
                    });
                }
                let mut items = entry.items.into_vec().into_iter();
                let key_data = finalize_data(
                    key,
                    items.next().expect("validated map key exists"),
                    shape,
                    context,
                    &path.child(SnapshotPathSegment::MapKey(index as u64)),
                )?;
                let value_data = finalize_data(
                    value,
                    items.next().expect("validated map value exists"),
                    shape,
                    context,
                    &path.child(SnapshotPathSegment::MapValue(index as u64)),
                )?;
                super::relations::insert_map_entry(
                    key,
                    &mut finalized,
                    key_data,
                    value_data,
                    &path.child(SnapshotPathSegment::MapKey(index as u64)),
                    context.canonicalization_budget,
                )?;
            }
            Ok(ValueData::Map(MapValue {
                entries: finalized.into_boxed_slice(),
            }))
        }
        (SchemaBody::ReifiedType, ValueDataDraft::Type(draft)) => {
            let reified = match draft {
                ReifiedTypeDraft::Schema(key) => ReifiedType::Schema(key),
                ReifiedTypeDraft::CanonicalKind(bytes) => {
                    ReifiedType::Kind(ReifiedKind::from_canonical_bytes(bytes)?)
                }
                ReifiedTypeDraft::Kind {
                    kind,
                    dimension_parameters,
                } => ReifiedType::Kind(ReifiedKind::from_closed_kind_with_optional_resolver(
                    &kind,
                    &dimension_parameters,
                    context.named_kinds,
                )?),
            };
            Ok(ValueData::Type(reified))
        }
        (_, draft) => Err(data_mismatch(schema, &draft, path)),
    }
}

fn scalar_sequence_schema(schema: &SchemaBody) -> bool {
    matches!(
        schema,
        SchemaBody::UnsignedInteger(_)
            | SchemaBody::SignedInteger(_)
            | SchemaBody::FloatingPoint(_)
            | SchemaBody::Complex(_)
            | SchemaBody::Rational64
            | SchemaBody::Bool
            | SchemaBody::String
            | SchemaBody::Id
            | SchemaBody::Index
            | SchemaBody::Atom(_)
    )
}

/// Finalizes a homogeneous scalar sequence directly into its schema-directed
/// packed storage. The generic recursive path first expands every scalar into
/// `ValueData`, allocates one diagnostic path per element, and then allocates
/// a second packed vector. Besides being unnecessary, that makes ordinary
/// large numeric ingress prohibitively expensive in WASM. A path is now
/// materialized only for the element that actually fails validation.
fn finalize_scalar_sequence(
    schema: &SchemaBody,
    values: Box<[ValueDataDraft]>,
    path: &SnapshotPath,
    element: ScalarSequenceElement,
    context: &SnapshotValidationContext<'_>,
) -> Result<SequenceStorage, SnapshotValueError> {
    macro_rules! pack {
        ($draft:ident, $storage:ident) => {{
            let mut packed = context.try_vec_with_capacity(values.len())?;
            for (index, draft) in values.into_vec().into_iter().enumerate() {
                let actual = draft.kind();
                let ValueDataDraft::$draft(value) = draft else {
                    return Err(data_mismatch_kind(
                        schema,
                        actual,
                        &element.path(path, index),
                    ));
                };
                packed.push(value);
            }
            Ok(SequenceStorage::$storage(packed.into_boxed_slice()))
        }};
    }

    match schema {
        SchemaBody::UnsignedInteger(IntegerWidth::W8) => pack!(U8, U8),
        SchemaBody::UnsignedInteger(IntegerWidth::W16) => pack!(U16, U16),
        SchemaBody::UnsignedInteger(IntegerWidth::W32) => pack!(U32, U32),
        SchemaBody::UnsignedInteger(IntegerWidth::W64) => pack!(U64, U64),
        SchemaBody::UnsignedInteger(IntegerWidth::W128) => pack!(U128, U128),
        SchemaBody::SignedInteger(IntegerWidth::W8) => pack!(I8, I8),
        SchemaBody::SignedInteger(IntegerWidth::W16) => pack!(I16, I16),
        SchemaBody::SignedInteger(IntegerWidth::W32) => pack!(I32, I32),
        SchemaBody::SignedInteger(IntegerWidth::W64) => pack!(I64, I64),
        SchemaBody::SignedInteger(IntegerWidth::W128) => pack!(I128, I128),
        SchemaBody::FloatingPoint(FloatWidth::W32) => pack!(F32, F32),
        SchemaBody::FloatingPoint(FloatWidth::W64) => pack!(F64, F64),
        SchemaBody::Complex(FloatWidth::W32) => pack!(Complex32, Complex32),
        SchemaBody::Complex(FloatWidth::W64) => pack!(Complex64, Complex64),
        SchemaBody::Bool => pack!(Bool, Bool),
        SchemaBody::Id => pack!(Id, Id),
        SchemaBody::String => {
            let mut packed = context.try_vec_with_capacity(values.len())?;
            for (index, draft) in values.into_vec().into_iter().enumerate() {
                let actual = draft.kind();
                let ValueDataDraft::String(value) = draft else {
                    return Err(data_mismatch_kind(
                        schema,
                        actual,
                        &element.path(path, index),
                    ));
                };
                packed.push(context.try_boxed_str(value)?);
            }
            Ok(SequenceStorage::String(packed.into_boxed_slice()))
        }
        SchemaBody::Rational64 => {
            let mut packed = context.try_vec_with_capacity(values.len())?;
            for (index, draft) in values.into_vec().into_iter().enumerate() {
                let actual = draft.kind();
                let ValueDataDraft::Rational64 {
                    numerator,
                    denominator,
                } = draft
                else {
                    return Err(data_mismatch_kind(
                        schema,
                        actual,
                        &element.path(path, index),
                    ));
                };
                packed.push(super::Rational64Value::new(numerator, denominator)?);
            }
            Ok(SequenceStorage::Rational64(packed.into_boxed_slice()))
        }
        SchemaBody::Index => {
            let mut packed = context.try_vec_with_capacity(values.len())?;
            for (index, draft) in values.into_vec().into_iter().enumerate() {
                let actual = draft.kind();
                let ValueDataDraft::Index(value) = draft else {
                    return Err(data_mismatch_kind(
                        schema,
                        actual,
                        &element.path(path, index),
                    ));
                };
                if value == 0 {
                    return Err(SnapshotValueError::InvalidIndexV1 {
                        path: element.path(path, index),
                        value,
                    });
                }
                packed.push(value);
            }
            Ok(SequenceStorage::Index(packed.into_boxed_slice()))
        }
        SchemaBody::Atom(_) => {
            for (index, draft) in values.iter().enumerate() {
                if !matches!(draft, ValueDataDraft::Atom) {
                    return Err(data_mismatch_kind(
                        schema,
                        draft.kind(),
                        &element.path(path, index),
                    ));
                }
            }
            Ok(SequenceStorage::Unit(values.len() as u64))
        }
        _ => unreachable!("scalar sequence fast path is selected by its closed schema"),
    }
}

#[derive(Clone, Copy)]
enum ScalarSequenceElement {
    Matrix,
    TableColumn(u32),
}

impl ScalarSequenceElement {
    fn path(self, root: &SnapshotPath, index: usize) -> SnapshotPath {
        match self {
            Self::Matrix => root.child(SnapshotPathSegment::MatrixElement(index as u64)),
            Self::TableColumn(column) => root
                .child(SnapshotPathSegment::TableColumn(column))
                .child(SnapshotPathSegment::TableRow(index as u64)),
        }
    }
}

fn ensure_arity(
    path: &SnapshotPath,
    expected: usize,
    actual: usize,
) -> Result<(), SnapshotValueError> {
    if expected == actual {
        return Ok(());
    }
    Err(SnapshotValueError::AggregateArityMismatchV1 {
        path: path.clone(),
        expected: expected as u64,
        actual: actual as u64,
    })
}

fn ensure_cardinality(
    path: &SnapshotPath,
    expected: u64,
    actual: usize,
) -> Result<(), SnapshotValueError> {
    let actual = actual as u64;
    if expected == actual {
        return Ok(());
    }
    Err(SnapshotValueError::PayloadCardinalityMismatchV1 {
        path: path.clone(),
        expected,
        actual,
    })
}

fn ensure_collection_cardinality(
    path: &SnapshotPath,
    cardinality: &crate::CardinalitySpec,
    shape: &ShapeInstance,
    actual: usize,
) -> Result<(), SnapshotValueError> {
    match cardinality {
        crate::CardinalitySpec::Exact(value) => ensure_cardinality(
            path,
            crate::schema::evaluate_dimension(value, shape.parameter_values())?,
            actual,
        ),
        crate::CardinalitySpec::Dynamic { upper_bound: None } => Ok(()),
        crate::CardinalitySpec::Dynamic {
            upper_bound: Some(value),
        } => {
            let upper = crate::schema::evaluate_dimension(value, shape.parameter_values())?;
            if actual as u64 <= upper {
                Ok(())
            } else {
                Err(SnapshotValueError::PayloadCardinalityMismatchV1 {
                    path: path.clone(),
                    expected: upper,
                    actual: actual as u64,
                })
            }
        }
    }
}

fn resolved_product(
    dimensions: &[crate::DimensionExpr],
    shape: &ShapeInstance,
) -> Result<u64, SnapshotValueError> {
    let mut total = 1_u64;
    for dimension in dimensions {
        let extent = crate::schema::evaluate_dimension(dimension, shape.parameter_values())?;
        total = total
            .checked_mul(extent)
            .ok_or(crate::SemanticModelError::DimensionOverflowV1)?;
    }
    Ok(total)
}

fn order_named_values(
    fields: &[crate::SchemaField],
    values: Box<[super::NamedValueDraft]>,
    path: &SnapshotPath,
    context: &SnapshotValidationContext<'_>,
) -> Result<Vec<ValueDataDraft>, SnapshotValueError> {
    if fields.len() != values.len() {
        return Err(SnapshotValueError::AggregateFieldMismatchV1 { path: path.clone() });
    }
    let mut pending = context.try_vec_with_capacity(values.len())?;
    pending.extend(values.into_vec().into_iter().map(Some));
    let mut ordered = context.try_vec_with_capacity(fields.len())?;
    for field in fields {
        let mut matched = None;
        for (index, value) in pending.iter().enumerate() {
            if value.as_ref().is_some_and(|value| value.name == field.name) {
                if matched.is_some() {
                    return Err(SnapshotValueError::AggregateFieldMismatchV1 {
                        path: path.clone(),
                    });
                }
                matched = Some(index);
            }
        }
        let Some(index) = matched else {
            return Err(SnapshotValueError::AggregateFieldMismatchV1 { path: path.clone() });
        };
        ordered.push(pending[index].take().expect("matched record field").value);
    }
    if pending.iter().any(Option::is_some) {
        return Err(SnapshotValueError::AggregateFieldMismatchV1 { path: path.clone() });
    }
    Ok(ordered)
}

fn order_table_columns(
    columns: &[crate::SchemaField],
    values: Box<[super::TableColumnDraft]>,
    path: &SnapshotPath,
    context: &SnapshotValidationContext<'_>,
) -> Result<Vec<Box<[ValueDataDraft]>>, SnapshotValueError> {
    if columns.len() != values.len() {
        return Err(SnapshotValueError::AggregateFieldMismatchV1 { path: path.clone() });
    }
    let mut pending = context.try_vec_with_capacity(values.len())?;
    pending.extend(values.into_vec().into_iter().map(Some));
    let mut ordered = context.try_vec_with_capacity(columns.len())?;
    for column in columns {
        let mut matched = None;
        for (index, value) in pending.iter().enumerate() {
            if value
                .as_ref()
                .is_some_and(|value| value.name == column.name)
            {
                if matched.is_some() {
                    return Err(SnapshotValueError::AggregateFieldMismatchV1 {
                        path: path.clone(),
                    });
                }
                matched = Some(index);
            }
        }
        let Some(index) = matched else {
            return Err(SnapshotValueError::AggregateFieldMismatchV1 { path: path.clone() });
        };
        ordered.push(pending[index].take().expect("matched table column").values);
    }
    if pending.iter().any(Option::is_some) {
        return Err(SnapshotValueError::AggregateFieldMismatchV1 { path: path.clone() });
    }
    Ok(ordered)
}

fn data_mismatch(
    schema: &SchemaBody,
    draft: &ValueDataDraft,
    path: &SnapshotPath,
) -> SnapshotValueError {
    SnapshotValueError::SnapshotDataSchemaMismatch {
        path: path.clone(),
        expected: schema_kind(schema),
        actual: draft.kind(),
    }
}

fn data_mismatch_kind(
    schema: &SchemaBody,
    actual: super::ValueDataKind,
    path: &SnapshotPath,
) -> SnapshotValueError {
    SnapshotValueError::SnapshotDataSchemaMismatch {
        path: path.clone(),
        expected: schema_kind(schema),
        actual,
    }
}

pub(super) const fn schema_kind(schema: &SchemaBody) -> SchemaDataKind {
    match schema {
        SchemaBody::Dynamic => SchemaDataKind::Dynamic,
        SchemaBody::Bool => SchemaDataKind::Bool,
        SchemaBody::UnsignedInteger(_) => SchemaDataKind::UnsignedInteger,
        SchemaBody::SignedInteger(_) => SchemaDataKind::SignedInteger,
        SchemaBody::FloatingPoint(_) => SchemaDataKind::FloatingPoint,
        SchemaBody::Complex(_) => SchemaDataKind::Complex,
        SchemaBody::Rational64 => SchemaDataKind::Rational64,
        SchemaBody::String => SchemaDataKind::String,
        SchemaBody::Id => SchemaDataKind::Id,
        SchemaBody::Index => SchemaDataKind::Index,
        SchemaBody::Atom(_) => SchemaDataKind::Atom,
        SchemaBody::Enum { .. } => SchemaDataKind::Enum,
        SchemaBody::Option(_) => SchemaDataKind::Option,
        SchemaBody::Tuple(_) => SchemaDataKind::Tuple,
        SchemaBody::Record(_) => SchemaDataKind::Record,
        SchemaBody::Matrix { .. } => SchemaDataKind::Matrix,
        SchemaBody::Table { .. } => SchemaDataKind::Table,
        SchemaBody::Set { .. } => SchemaDataKind::Set,
        SchemaBody::Map { .. } => SchemaDataKind::Map,
        SchemaBody::ReifiedType => SchemaDataKind::ReifiedType,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot::{
        Complex32Bits, Complex64Bits, F32Bits, F64Bits, SequenceView, TableColumnDraft,
    };
    use crate::{
        DimensionExpr, DimensionLifetime, DimensionParameterDeclaration, DimensionParameterId,
        DimensionParameterOrigin, NominalKey, SchemaDraft, SchemaField, SchemaTableBuilder,
    };
    use core::cell::{Cell, RefCell};

    #[derive(Default)]
    struct RecordingConstructionAuthority {
        allocations: RefCell<Vec<(u64, u32)>>,
    }

    impl SnapshotConstructionAuthority for RecordingConstructionAuthority {
        fn admit_snapshot_allocation(
            &self,
            bytes: u64,
            alignment: u32,
        ) -> Result<(), crate::MemoryRuntimeError> {
            self.allocations.borrow_mut().push((bytes, alignment));
            Ok(())
        }

        fn allocation_object(&self) -> Option<crate::MemoryObjectId> {
            None
        }
    }

    struct FailAfterConstructionAuthority {
        remaining: Cell<usize>,
    }

    impl SnapshotConstructionAuthority for FailAfterConstructionAuthority {
        fn admit_snapshot_allocation(
            &self,
            bytes: u64,
            _alignment: u32,
        ) -> Result<(), crate::MemoryRuntimeError> {
            let remaining = self.remaining.get();
            if remaining == 0 {
                return Err(crate::MemoryRuntimeError::CapacityExceeded {
                    object: crate::MemoryObjectId::new(0),
                    requested: bytes,
                    capacity: 0,
                });
            }
            self.remaining.set(remaining - 1);
            Ok(())
        }

        fn allocation_object(&self) -> Option<crate::MemoryObjectId> {
            Some(crate::MemoryObjectId::new(0))
        }
    }

    #[test]
    fn scalar_sequence_fast_path_is_total_for_every_accepted_schema() {
        let cases = vec![
            (
                SchemaBody::UnsignedInteger(IntegerWidth::W8),
                ValueDataDraft::U8(1),
            ),
            (
                SchemaBody::UnsignedInteger(IntegerWidth::W16),
                ValueDataDraft::U16(1),
            ),
            (
                SchemaBody::UnsignedInteger(IntegerWidth::W32),
                ValueDataDraft::U32(1),
            ),
            (
                SchemaBody::UnsignedInteger(IntegerWidth::W64),
                ValueDataDraft::U64(1),
            ),
            (
                SchemaBody::UnsignedInteger(IntegerWidth::W128),
                ValueDataDraft::U128(1),
            ),
            (
                SchemaBody::SignedInteger(IntegerWidth::W8),
                ValueDataDraft::I8(-1),
            ),
            (
                SchemaBody::SignedInteger(IntegerWidth::W16),
                ValueDataDraft::I16(-1),
            ),
            (
                SchemaBody::SignedInteger(IntegerWidth::W32),
                ValueDataDraft::I32(-1),
            ),
            (
                SchemaBody::SignedInteger(IntegerWidth::W64),
                ValueDataDraft::I64(-1),
            ),
            (
                SchemaBody::SignedInteger(IntegerWidth::W128),
                ValueDataDraft::I128(-1),
            ),
            (
                SchemaBody::FloatingPoint(FloatWidth::W32),
                ValueDataDraft::F32(F32Bits::from_f32(1.25)),
            ),
            (
                SchemaBody::FloatingPoint(FloatWidth::W64),
                ValueDataDraft::F64(F64Bits::from_f64(1.25)),
            ),
            (
                SchemaBody::Complex(FloatWidth::W32),
                ValueDataDraft::Complex32(Complex32Bits::new(
                    F32Bits::from_f32(1.0),
                    F32Bits::from_f32(-1.0),
                )),
            ),
            (
                SchemaBody::Complex(FloatWidth::W64),
                ValueDataDraft::Complex64(Complex64Bits::new(
                    F64Bits::from_f64(1.0),
                    F64Bits::from_f64(-1.0),
                )),
            ),
            (
                SchemaBody::Rational64,
                ValueDataDraft::Rational64 {
                    numerator: 1,
                    denominator: 2,
                },
            ),
            (SchemaBody::Bool, ValueDataDraft::Bool(true)),
            (
                SchemaBody::String,
                ValueDataDraft::String("packed".to_owned()),
            ),
            (SchemaBody::Id, ValueDataDraft::Id(7)),
            (SchemaBody::Index, ValueDataDraft::Index(1)),
            (
                SchemaBody::Atom(NominalKey::from_bytes([7; 32])),
                ValueDataDraft::Atom,
            ),
        ];
        let (schemas, _) = SchemaTableBuilder::new().finish().unwrap().into_parts();
        let context = SnapshotValidationContext::new(&schemas);
        let path = SnapshotPath::root();

        for (schema, valid) in cases {
            assert!(scalar_sequence_schema(&schema));
            finalize_scalar_sequence(
                &schema,
                vec![valid].into_boxed_slice(),
                &path,
                ScalarSequenceElement::Matrix,
                &context,
            )
            .unwrap_or_else(|error| panic!("valid {schema:?} scalar sequence failed: {error:?}"));
            finalize_scalar_sequence(
                &schema,
                Box::new([]),
                &path,
                ScalarSequenceElement::Matrix,
                &context,
            )
            .unwrap_or_else(|error| panic!("empty {schema:?} scalar sequence failed: {error:?}"));
            let invalid = if matches!(schema, SchemaBody::Bool) {
                ValueDataDraft::U8(1)
            } else {
                ValueDataDraft::Bool(true)
            };
            assert!(
                finalize_scalar_sequence(
                    &schema,
                    vec![invalid].into_boxed_slice(),
                    &path,
                    ScalarSequenceElement::Matrix,
                    &context,
                )
                .is_err(),
                "mismatched {schema:?} scalar sequence was accepted"
            );
        }

        assert!(matches!(
            finalize_scalar_sequence(
                &SchemaBody::Index,
                vec![ValueDataDraft::Index(0)].into_boxed_slice(),
                &path,
                ScalarSequenceElement::Matrix,
                &context,
            ),
            Err(SnapshotValueError::InvalidIndexV1 { .. })
        ));
        assert!(
            finalize_scalar_sequence(
                &SchemaBody::Rational64,
                vec![ValueDataDraft::Rational64 {
                    numerator: 1,
                    denominator: 0,
                }]
                .into_boxed_slice(),
                &path,
                ScalarSequenceElement::Matrix,
                &context,
            )
            .is_err()
        );
    }

    #[test]
    fn packed_scalar_matrices_cover_id_values() {
        let schema = SchemaDraft {
            dimension_parameters: Box::new([]),
            body: SchemaBody::Matrix {
                element: Box::new(SchemaBody::Id),
                dimensions: vec![DimensionExpr::Constant(2)].into_boxed_slice(),
            },
        }
        .finalize()
        .unwrap();
        let mut builder = SchemaTableBuilder::new();
        let handle = builder.insert(schema).unwrap();
        let build = builder.finish().unwrap();
        let schema = build.resolve(handle).unwrap();
        let (schemas, _) = build.into_parts();
        let value = ValueDraft {
            schema,
            shape_values: Box::new([]),
            data: ValueDataDraft::Matrix(
                vec![ValueDataDraft::Id(7), ValueDataDraft::Id(11)].into_boxed_slice(),
            ),
        }
        .finalize(&SnapshotValidationContext::new(&schemas))
        .unwrap();
        let ValueData::Matrix(matrix) = value.data() else {
            panic!("expected Id matrix")
        };
        assert!(matches!(matrix.elements(), SequenceView::Id(&[7, 11])));
    }

    #[test]
    fn scalar_table_columns_pack_without_intermediate_value_data() {
        let row_count = 64_usize;
        let schema = SchemaDraft {
            dimension_parameters: Box::new([]),
            body: SchemaBody::Table {
                columns: vec![
                    SchemaField {
                        name: "byte".to_owned(),
                        schema: SchemaBody::UnsignedInteger(IntegerWidth::W8),
                    },
                    SchemaField {
                        name: "flag".to_owned(),
                        schema: SchemaBody::Bool,
                    },
                    SchemaField {
                        name: "text".to_owned(),
                        schema: SchemaBody::String,
                    },
                ]
                .into_boxed_slice(),
                rows: crate::CardinalitySpec::Exact(DimensionExpr::Constant(row_count as u64)),
            },
        }
        .finalize()
        .unwrap();
        let mut builder = SchemaTableBuilder::new();
        let handle = builder.insert(schema).unwrap();
        let build = builder.finish().unwrap();
        let schema = build.resolve(handle).unwrap();
        let (schemas, _) = build.into_parts();
        let authority = RecordingConstructionAuthority::default();
        let draft = ValueDraft {
            schema,
            shape_values: Box::new([]),
            data: ValueDataDraft::Table(
                vec![
                    TableColumnDraft {
                        name: "byte".to_owned(),
                        values: (0..row_count)
                            .map(|value| ValueDataDraft::U8(value as u8))
                            .collect::<Vec<_>>()
                            .into_boxed_slice(),
                    },
                    TableColumnDraft {
                        name: "flag".to_owned(),
                        values: (0..row_count)
                            .map(|value| ValueDataDraft::Bool(value % 2 == 0))
                            .collect::<Vec<_>>()
                            .into_boxed_slice(),
                    },
                    TableColumnDraft {
                        name: "text".to_owned(),
                        values: (0..row_count)
                            .map(|value| ValueDataDraft::String(format!("row-{value}")))
                            .collect::<Vec<_>>()
                            .into_boxed_slice(),
                    },
                ]
                .into_boxed_slice(),
            ),
        };
        let failing = FailAfterConstructionAuthority {
            remaining: Cell::new(1),
        };
        assert!(
            draft
                .clone()
                .finalize(
                    &SnapshotValidationContext::new(&schemas).with_construction_authority(&failing),
                )
                .is_err(),
            "a partial packed-table build must fail through construction authority"
        );
        let value = draft
            .finalize(
                &SnapshotValidationContext::new(&schemas).with_construction_authority(&authority),
            )
            .unwrap();
        let ValueData::Table(table) = value.data() else {
            panic!("expected table")
        };
        assert!(
            matches!(table.column(0), Some(SequenceView::U8(values)) if values.len() == row_count)
        );
        assert!(
            matches!(table.column(1), Some(SequenceView::Bool(values)) if values.len() == row_count)
        );
        assert!(
            matches!(table.column(2), Some(SequenceView::String(values)) if values.len() == row_count)
        );
        let expanded = (
            (row_count * core::mem::size_of::<ValueData>()) as u64,
            core::mem::align_of::<ValueData>() as u32,
        );
        assert!(!authority.allocations.borrow().contains(&expanded));
        assert!(
            authority
                .allocations
                .borrow()
                .contains(&(row_count as u64, core::mem::align_of::<u8>() as u32,))
        );
    }

    #[test]
    fn recursive_dynamic_values_share_one_admitted_schema_owner() {
        let scalar = SchemaDraft {
            dimension_parameters: Box::new([]),
            body: SchemaBody::UnsignedInteger(IntegerWidth::W64),
        }
        .finalize()
        .unwrap();
        let dynamic_matrix = SchemaDraft {
            dimension_parameters: Box::new([]),
            body: SchemaBody::Matrix {
                element: Box::new(SchemaBody::Dynamic),
                dimensions: vec![DimensionExpr::Constant(2)].into_boxed_slice(),
            },
        }
        .finalize()
        .unwrap();
        let mut builder = SchemaTableBuilder::new();
        let scalar_handle = builder.insert(scalar).unwrap();
        let matrix_handle = builder.insert(dynamic_matrix).unwrap();
        let build = builder.finish().unwrap();
        let scalar = build.resolve(scalar_handle).unwrap();
        let matrix = build.resolve(matrix_handle).unwrap();
        let (schemas, _) = build.into_parts();
        let nested = |value| {
            ValueDataDraft::Dynamic(Some(Box::new(ValueDraft {
                schema: scalar,
                shape_values: Box::new([]),
                data: ValueDataDraft::U64(value),
            })))
        };
        let authority = RecordingConstructionAuthority::default();
        let value = ValueDraft {
            schema: matrix,
            shape_values: Box::new([]),
            data: ValueDataDraft::Matrix(vec![nested(1), nested(2)].into_boxed_slice()),
        }
        .finalize(&SnapshotValidationContext::new(&schemas).with_construction_authority(&authority))
        .unwrap();
        let ValueData::Matrix(dynamic) = value.data() else {
            panic!("expected Dynamic matrix")
        };
        let SequenceView::Values(values) = dynamic.elements() else {
            panic!("expected recursively stored Dynamic values")
        };
        let owners = values
            .iter()
            .map(|value| {
                let ValueData::Dynamic(value) = value else {
                    panic!("expected Dynamic element")
                };
                value.value().unwrap().schemas().unwrap()
            })
            .collect::<Vec<_>>();
        assert!(Arc::ptr_eq(&owners[0], &owners[1]));
        let schema_clone = (
            schemas.clone_allocation_bound_bytes().unwrap(),
            core::mem::align_of::<SchemaTable>() as u32,
        );
        assert_eq!(
            authority
                .allocations
                .borrow()
                .iter()
                .filter(|allocation| **allocation == schema_clone)
                .count(),
            1,
        );
    }

    #[test]
    fn metadata_only_rebind_shares_storage_but_schema_transform_rebuilds() {
        let fixed = SchemaDraft {
            dimension_parameters: Box::new([]),
            body: SchemaBody::Matrix {
                element: Box::new(SchemaBody::String),
                dimensions: vec![DimensionExpr::Constant(1)].into_boxed_slice(),
            },
        }
        .finalize()
        .unwrap();
        let dynamic = SchemaDraft {
            dimension_parameters: vec![DimensionParameterDeclaration {
                id: DimensionParameterId::new(0),
                origin: DimensionParameterOrigin::Explicit,
                lifetime: DimensionLifetime::Turn,
                lower_bound: DimensionExpr::Constant(0),
                upper_bound: None,
            }]
            .into_boxed_slice(),
            body: SchemaBody::Matrix {
                element: Box::new(SchemaBody::String),
                dimensions: vec![DimensionExpr::Parameter(DimensionParameterId::new(0))]
                    .into_boxed_slice(),
            },
        }
        .finalize()
        .unwrap();
        let mut builder = SchemaTableBuilder::new();
        let fixed_handle = builder.insert(fixed).unwrap();
        let dynamic_handle = builder.insert(dynamic).unwrap();
        let build = builder.finish().unwrap();
        let fixed = build.resolve(fixed_handle).unwrap();
        let dynamic = build.resolve(dynamic_handle).unwrap();
        let (schemas, _) = build.into_parts();
        let value = ValueDraft {
            schema: fixed,
            shape_values: Box::new([]),
            data: ValueDataDraft::Matrix(
                vec![ValueDataDraft::String("owned".to_owned())].into_boxed_slice(),
            ),
        }
        .finalize(&SnapshotValidationContext::new(&schemas))
        .unwrap();

        let same = value.rebind(fixed, value.shape(), &schemas).unwrap();
        assert!(value.shares_frozen_storage(&same));

        let mut equivalent_builder = SchemaTableBuilder::new();
        let equivalent_handle = equivalent_builder
            .insert(schemas.get(fixed).unwrap().clone())
            .unwrap();
        let equivalent_build = equivalent_builder.finish().unwrap();
        let equivalent = equivalent_build.resolve(equivalent_handle).unwrap();
        let (equivalent_schemas, _) = equivalent_build.into_parts();
        let rebound = value
            .rebind(equivalent, value.shape(), &equivalent_schemas)
            .unwrap();
        assert!(value.shares_frozen_storage(&rebound));
        assert!(rebound.validate_against(&equivalent_schemas).is_ok());

        let dynamic_shape = schemas
            .get(dynamic)
            .unwrap()
            .instantiate_shape(vec![1].into_boxed_slice())
            .unwrap();
        let transformed = value.rebind(dynamic, &dynamic_shape, &schemas).unwrap();
        assert!(!value.shares_frozen_storage(&transformed));

        let dynamic_value = ValueDraft {
            schema: dynamic,
            shape_values: vec![1].into_boxed_slice(),
            data: ValueDataDraft::Matrix(
                vec![ValueDataDraft::String("dynamic".to_owned())].into_boxed_slice(),
            ),
        }
        .finalize(&SnapshotValidationContext::new(&schemas))
        .unwrap();
        let mut dynamic_table_builder = SchemaTableBuilder::new();
        let dynamic_table_handle = dynamic_table_builder
            .insert(schemas.get(dynamic).unwrap().clone())
            .unwrap();
        let dynamic_table_build = dynamic_table_builder.finish().unwrap();
        let dynamic_table_schema = dynamic_table_build.resolve(dynamic_table_handle).unwrap();
        let (dynamic_table, _) = dynamic_table_build.into_parts();
        let dynamic_rebound = dynamic_value
            .rebind(dynamic_table_schema, &dynamic_shape, &dynamic_table)
            .unwrap();
        assert!(dynamic_value.shares_frozen_storage(&dynamic_rebound));

        let dynamic_schema = SchemaDraft {
            dimension_parameters: Box::new([]),
            body: SchemaBody::Dynamic,
        }
        .finalize()
        .unwrap();
        let mut source_dynamic_builder = SchemaTableBuilder::new();
        let source_dynamic_handle = source_dynamic_builder
            .insert(dynamic_schema.clone())
            .unwrap();
        let source_dynamic_build = source_dynamic_builder.finish().unwrap();
        let source_dynamic = source_dynamic_build.resolve(source_dynamic_handle).unwrap();
        let (source_dynamic_table, _) = source_dynamic_build.into_parts();
        let dynamic_value = ValueDraft {
            schema: source_dynamic,
            shape_values: Box::new([]),
            data: ValueDataDraft::Dynamic(None),
        }
        .finalize(&SnapshotValidationContext::new(&source_dynamic_table))
        .unwrap();
        let mut target_dynamic_builder = SchemaTableBuilder::new();
        let target_dynamic_handle = target_dynamic_builder.insert(dynamic_schema).unwrap();
        let target_dynamic_build = target_dynamic_builder.finish().unwrap();
        let target_dynamic = target_dynamic_build.resolve(target_dynamic_handle).unwrap();
        let (target_dynamic_table, _) = target_dynamic_build.into_parts();
        let dynamic_rebound = dynamic_value
            .rebind(target_dynamic, dynamic_value.shape(), &target_dynamic_table)
            .unwrap();
        assert!(!dynamic_value.shares_frozen_storage(&dynamic_rebound));
    }

    #[test]
    fn index_snapshots_are_one_based() {
        let schema = SchemaDraft {
            dimension_parameters: Box::new([]),
            body: SchemaBody::Index,
        }
        .finalize()
        .unwrap();
        let mut builder = SchemaTableBuilder::new();
        let handle = builder.insert(schema).unwrap();
        let build = builder.finish().unwrap();
        let schema = build.resolve(handle).unwrap();
        let (schemas, _) = build.into_parts();
        let context = SnapshotValidationContext::new(&schemas);

        let draft = |value| ValueDraft {
            schema,
            shape_values: Box::new([]),
            data: ValueDataDraft::Index(value),
        };
        assert!(matches!(
            draft(0).finalize(&context),
            Err(SnapshotValueError::InvalidIndexV1 { value: 0, .. })
        ));
        assert!(draft(1).finalize(&context).is_ok());
        assert!(draft(u64::MAX).finalize(&context).is_ok());
    }

    #[test]
    fn snapshot_finalization_shares_one_fail_closed_canonicalization_budget() {
        let schema = SchemaDraft {
            dimension_parameters: Box::new([]),
            body: SchemaBody::Set {
                element: Box::new(SchemaBody::Bool),
                cardinality: crate::CardinalitySpec::Dynamic { upper_bound: None },
            },
        }
        .finalize()
        .unwrap();
        let mut builder = SchemaTableBuilder::new();
        let handle = builder.insert(schema).unwrap();
        let build = builder.finish().unwrap();
        let schema = build.resolve(handle).unwrap();
        let (schemas, _) = build.into_parts();
        let draft = || ValueDraft {
            schema,
            shape_values: Box::new([]),
            data: ValueDataDraft::Set(
                vec![ValueDataDraft::Bool(false), ValueDataDraft::Bool(true)].into_boxed_slice(),
            ),
        };

        assert!(
            draft()
                .finalize(&SnapshotValidationContext::new(&schemas))
                .is_ok()
        );
        let budget = SnapshotCanonicalizationBudget::new(0);
        assert!(matches!(
            draft().finalize(
                &SnapshotValidationContext::new(&schemas).with_canonicalization_budget(&budget),
            ),
            Err(SnapshotValueError::CanonicalizationWorkLimitExceededV1 { limit: 0 })
        ));
        assert_eq!(budget.consumed(), 0);
    }

    #[test]
    fn snapshot_finalization_charges_ordered_insertion_shifts() {
        let integer = SchemaBody::UnsignedInteger(crate::IntegerWidth::W64);
        let set = SchemaDraft {
            dimension_parameters: Box::new([]),
            body: SchemaBody::Set {
                element: Box::new(integer.clone()),
                cardinality: crate::CardinalitySpec::Dynamic { upper_bound: None },
            },
        }
        .finalize()
        .unwrap();
        let map = SchemaDraft {
            dimension_parameters: Box::new([]),
            body: SchemaBody::Map {
                key: Box::new(integer.clone()),
                value: Box::new(integer),
                cardinality: crate::CardinalitySpec::Dynamic { upper_bound: None },
            },
        }
        .finalize()
        .unwrap();
        let mut builder = SchemaTableBuilder::new();
        let set_handle = builder.insert(set).unwrap();
        let map_handle = builder.insert(map).unwrap();
        let build = builder.finish().unwrap();
        let set_schema = build.resolve(set_handle).unwrap();
        let map_schema = build.resolve(map_handle).unwrap();
        let (schemas, _) = build.into_parts();
        let set_draft = |values: &[u64]| ValueDraft {
            schema: set_schema,
            shape_values: Box::new([]),
            data: ValueDataDraft::Set(
                values
                    .iter()
                    .copied()
                    .map(ValueDataDraft::U64)
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            ),
        };
        let map_draft = |values: &[u64]| ValueDraft {
            schema: map_schema,
            shape_values: Box::new([]),
            data: ValueDataDraft::Map(
                values
                    .iter()
                    .copied()
                    .map(|value| super::super::MapEntryDraft {
                        items: vec![ValueDataDraft::U64(value), ValueDataDraft::U64(value)]
                            .into_boxed_slice(),
                    })
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            ),
        };

        for draft in [set_draft(&[4, 3, 2, 1, 0]), map_draft(&[4, 3, 2, 1, 0])] {
            let budget = SnapshotCanonicalizationBudget::new(10);
            assert!(matches!(
                draft.finalize(
                    &SnapshotValidationContext::new(&schemas).with_canonicalization_budget(&budget),
                ),
                Err(SnapshotValueError::CanonicalizationWorkLimitExceededV1 { limit: 10 })
            ));
            // The failed shift charge is rejected atomically and therefore
            // does not advance the shared meter past its last admitted value.
            assert_eq!(budget.consumed(), 9);
        }
        for draft in [set_draft(&[0, 1, 2, 3, 4]), map_draft(&[0, 1, 2, 3, 4])] {
            let budget = SnapshotCanonicalizationBudget::new(4);
            assert!(
                draft
                    .finalize(
                        &SnapshotValidationContext::new(&schemas)
                            .with_canonicalization_budget(&budget),
                    )
                    .is_ok()
            );
            assert_eq!(budget.consumed(), 4);
        }
    }

    #[test]
    fn bound_composite_preserves_table_columns_and_storage() {
        let schema = SchemaDraft {
            dimension_parameters: Box::new([]),
            body: SchemaBody::Table {
                columns: vec![
                    SchemaField {
                        name: "id".to_owned(),
                        schema: SchemaBody::String,
                    },
                    SchemaField {
                        name: "x".to_owned(),
                        schema: SchemaBody::FloatingPoint(FloatWidth::W64),
                    },
                ]
                .into_boxed_slice(),
                rows: crate::CardinalitySpec::Exact(crate::DimensionExpr::Constant(2)),
            },
        }
        .finalize()
        .unwrap();
        let mut builder = SchemaTableBuilder::new();
        let string = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::String,
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let float = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::FloatingPoint(FloatWidth::W64),
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let handle = builder.insert(schema).unwrap();
        let build = builder.finish().unwrap();
        let schema = build.resolve(handle).unwrap();
        let string = build.resolve(string).unwrap();
        let float = build.resolve(float).unwrap();
        let (schemas, _) = build.into_parts();
        let schemas = Arc::new(schemas);
        let shape = schemas
            .get(schema)
            .unwrap()
            .instantiate_shape(Box::new([]))
            .unwrap();
        let layout = |id| {
            (
                id,
                schemas
                    .get(id)
                    .unwrap()
                    .instantiate_shape(Box::new([]))
                    .unwrap(),
            )
        };
        let constructor = CompositeSnapshotConstructor::bind(
            schema,
            shape,
            &[layout(string), layout(string), layout(float), layout(float)],
            Arc::clone(&schemas),
        )
        .unwrap();
        let context = SnapshotValidationContext::with_shared_schemas(&schemas);
        let children = [
            (string, ValueDataDraft::String("c".into())),
            (string, ValueDataDraft::String("d".into())),
            (float, ValueDataDraft::F64(F64Bits::from_f64(3.0))),
            (float, ValueDataDraft::F64(F64Bits::from_f64(4.0))),
        ]
        .into_iter()
        .map(|(schema, data)| {
            ValueDraft {
                schema,
                data,
                shape_values: Box::new([]),
            }
            .finalize(&context)
            .unwrap()
        })
        .collect::<Vec<_>>()
        .into_boxed_slice();
        let rebuilt = constructor.construct(children, None).unwrap();
        let ValueData::Table(table) = rebuilt.data() else {
            panic!("rebuilt composite must remain a table");
        };
        let super::super::SequenceView::String(ids) = table.column(0).unwrap() else {
            panic!("string table column changed representation");
        };
        assert_eq!(
            ids.iter().map(|id| id.as_ref()).collect::<Vec<_>>(),
            ["c", "d"]
        );
        let super::super::SequenceView::F64(values) = table.column(1).unwrap() else {
            panic!("f64 table column changed representation");
        };
        assert_eq!(
            values
                .iter()
                .map(|value| value.to_f64())
                .collect::<Vec<_>>(),
            [3.0, 4.0]
        );
    }
}
