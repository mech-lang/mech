//! Schema-aware mutable program cells.
//!
//! A [`ValueCell`] owns identity and schema metadata while its exact typed
//! backing remains private. Canonical snapshots are the only universal value
//! representation exposed by this module.

use crate::{
    CardinalitySpec, DimensionExpr, FloatWidth, FunctionMatrixElement,
    FunctionMatrixRepresentation, FunctionMatrixStoragePattern, FunctionRuntimeType,
    FunctionValueRepresentation, IntegerWidth, MResult, MechError, MechErrorKind, MemoryDomain,
    Ref, ResolvedType, Schema, SchemaBody, SchemaId, SchemaKey, SchemaTable, SchemaTableBuilder,
    ShapeInstance, SnapshotValueError, TypeConstraintFailure, TypeResolutionError, Value,
    ValueData, ValueDataDraft, ValueDraft,
};
use core::{any::Any, any::type_name, cell, fmt};

#[cfg(feature = "matrix")]
use crate::snapshot::SequenceView;
use crate::snapshot::SnapshotValidationContext;

#[cfg(all(feature = "no_std", feature = "string"))]
use alloc::string::ToString;
#[cfg(feature = "no_std")]
use alloc::{boxed::Box, rc::Rc, string::String, vec::Vec};
#[cfg(not(feature = "no_std"))]
use std::{boxed::Box, rc::Rc, string::String, vec::Vec};

/// Stable logical identity of a canonical mutable cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CanonicalCellId(u64);

impl CanonicalCellId {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

mod canonical_cell_sealed {
    use super::*;

    pub trait Sealed: FunctionRuntimeType + Clone + 'static {
        fn snapshot_bound(
            &self,
            schema: SchemaId,
            shape: &ShapeInstance,
            schemas: &SchemaTable,
        ) -> MResult<Value>;

        fn replace_bound(&mut self, value: &Value) -> MResult<()>;

        fn initialize_planned_fixed(
            &self,
            _frame: &mut crate::KernelMemoryFrame<'_>,
            object: crate::PlanObjectKey,
        ) -> MResult<()> {
            Err(managed_host_shape_error(
                object,
                "indirect payload requires its admitted builder",
            ))
        }

        fn representation(schema: &SchemaBody) -> FunctionValueRepresentation {
            let _ = schema;
            Self::REPRESENTATION
        }

        fn matrix_extents(&self) -> Option<(usize, usize)> {
            None
        }
    }
}

/// An exact typed backing that can safely live behind a canonical value cell.
///
/// This trait is sealed. Universal values and aggregate containers cannot be
/// used as exact cell backings.
pub trait CanonicalCellBacking:
    canonical_cell_sealed::Sealed + FunctionRuntimeType + Clone + 'static
{
}

impl<T> CanonicalCellBacking for T where
    T: canonical_cell_sealed::Sealed + FunctionRuntimeType + Clone + 'static
{
}

pub(crate) struct CellRecord {
    pub(crate) identity: CanonicalCellId,
    pub(crate) schema: SchemaId,
    pub(crate) schema_key: SchemaKey,
    pub(crate) schemas: Rc<SchemaTable>,
    publication_locked: cell::Cell<bool>,
    publication_shape: cell::RefCell<Option<ShapeInstance>>,
    published: cell::RefCell<PublishedCellState>,
}

struct PublishedCellState {
    shape: ShapeInstance,
    version: crate::PublishedValueVersion,
    storage: CellStorageBinding,
}

/// Physical backing authority for one published logical cell value during the
/// first stable-record cutover. Managed variants are added with the planned
/// payload realization checkpoint, once their adapters can be constructed.
#[derive(Clone)]
enum CellStorageBinding {
    ManagedHost {
        owner: MemoryDomain,
        storage: Rc<dyn ErasedCellStorage>,
    },
    ManagedCanonical {
        owner: MemoryDomain,
        storage: Rc<dyn ErasedCellStorage>,
    },
    #[expect(
        dead_code,
        reason = "backend binding construction is completed by the R6 backend cutover checkpoint"
    )]
    ManagedDevice {
        owner: MemoryDomain,
        storage: Rc<dyn ErasedCellStorage>,
    },
    PinnedExternal(Rc<dyn ErasedCellStorage>),
}

impl CellStorageBinding {
    fn adapter(&self) -> &Rc<dyn ErasedCellStorage> {
        match self {
            Self::ManagedHost { storage, .. }
            | Self::ManagedCanonical { storage, .. }
            | Self::ManagedDevice { storage, .. } => storage,
            Self::PinnedExternal(storage) => storage,
        }
    }

    fn owner(&self) -> Option<&MemoryDomain> {
        match self {
            Self::ManagedHost { owner, .. }
            | Self::ManagedCanonical { owner, .. }
            | Self::ManagedDevice { owner, .. } => Some(owner),
            Self::PinnedExternal(_) => None,
        }
    }
}

#[derive(Clone)]
pub(crate) struct CellBinding {
    record: Rc<CellRecord>,
    /// Planning-time topology for composites assembled from live canonical
    /// cells. The canonical snapshot remains the runtime value authority.
    pub(crate) compiler_children: Option<Rc<[ValueCell]>>,
}

impl core::ops::Deref for CellBinding {
    type Target = CellRecord;

    fn deref(&self) -> &Self::Target {
        self.record.as_ref()
    }
}

impl CellBinding {
    fn managed(
        identity: CanonicalCellId,
        schema: SchemaId,
        schema_key: SchemaKey,
        shape: ShapeInstance,
        schemas: Rc<SchemaTable>,
        owner: MemoryDomain,
        storage: CellStorageBinding,
    ) -> Self {
        debug_assert_eq!(storage.owner().map(MemoryDomain::id), Some(owner.id()));
        Self {
            record: Rc::new(CellRecord {
                identity,
                schema,
                schema_key,
                schemas,
                publication_locked: cell::Cell::new(false),
                publication_shape: cell::RefCell::new(None),
                published: cell::RefCell::new(PublishedCellState {
                    shape,
                    version: crate::PublishedValueVersion::initial(),
                    storage,
                }),
            }),
            compiler_children: None,
        }
    }

    fn pinned_external(
        identity: CanonicalCellId,
        schema: SchemaId,
        schema_key: SchemaKey,
        shape: ShapeInstance,
        schemas: Rc<SchemaTable>,
        storage: Rc<dyn ErasedCellStorage>,
    ) -> Self {
        Self {
            record: Rc::new(CellRecord {
                identity,
                schema,
                schema_key,
                schemas,
                publication_locked: cell::Cell::new(false),
                publication_shape: cell::RefCell::new(None),
                published: cell::RefCell::new(PublishedCellState {
                    shape,
                    version: crate::PublishedValueVersion::initial(),
                    storage: CellStorageBinding::PinnedExternal(storage),
                }),
            }),
            compiler_children: None,
        }
    }

    fn shape(&self) -> cell::Ref<'_, ShapeInstance> {
        if self.publication_locked.get() {
            return cell::Ref::map(self.publication_shape.borrow(), |shape| {
                shape
                    .as_ref()
                    .expect("ready publication retains its pre-commit shape")
            });
        }
        cell::Ref::map(self.published.borrow(), |published| &published.shape)
    }

    fn try_shape(&self, access: CellAccess) -> MResult<cell::Ref<'_, ShapeInstance>> {
        if self.publication_locked.get() {
            return self
                .publication_shape
                .try_borrow()
                .map(|shape| {
                    cell::Ref::map(shape, |shape| {
                        shape
                            .as_ref()
                            .expect("ready publication retains its pre-commit shape")
                    })
                })
                .map_err(|_| borrow_conflict(access));
        }
        self.published
            .try_borrow()
            .map(|published| cell::Ref::map(published, |published| &published.shape))
            .map_err(|_| borrow_conflict(access))
    }

    fn storage(&self) -> MResult<Rc<dyn ErasedCellStorage>> {
        if self.publication_locked.get() {
            return Err(MechError::from(
                crate::MemoryRuntimeError::PublicationInProgress,
            ));
        }
        let published = self
            .published
            .try_borrow()
            .map_err(|_| borrow_conflict(CellAccess::Snapshot))?;
        if let Some(owner) = published.storage.owner() {
            owner.ensure_open().map_err(MechError::from)?;
        }
        Ok(published.storage.adapter().clone())
    }

    fn publication_version(&self) -> crate::PublishedValueVersion {
        self.published.borrow().version
    }

    fn memory_domain(&self) -> Option<MemoryDomain> {
        self.published.borrow().storage.owner().cloned()
    }
}

pub(crate) trait ErasedCellStorage {
    fn as_any(&self) -> &dyn Any;
    fn representation(&self, schema: &SchemaBody) -> FunctionValueRepresentation;
    fn snapshot(
        &self,
        schema: SchemaId,
        shape: &ShapeInstance,
        schemas: &SchemaTable,
    ) -> MResult<Value>;
    fn replace(&self, value: &Value) -> MResult<()>;
    fn preflight_replace(&self) -> MResult<()>;
    fn capabilities(&self) -> crate::StorageCapabilityDescriptor;
    fn detached_clone(&self) -> MResult<DetachedCellStorage>;
    fn same_storage(&self, other: &dyn ErasedCellStorage) -> bool;
    fn borrow_state(&self) -> CellBorrowState;
    fn managed_host_binding(&self) -> Option<ManagedHostCellBinding> {
        None
    }
}

#[derive(Clone)]
pub(crate) struct ManagedHostCellBinding {
    pub(crate) realized: crate::RealizedMemoryPlan,
    pub(crate) object: crate::PlanObjectKey,
    pub(crate) region: crate::MemoryAccessRegion,
}

pub(crate) struct DetachedCellStorage {
    pub identity: CanonicalCellId,
    pub storage: Rc<dyn ErasedCellStorage>,
}

pub(crate) struct PreparedManagedCellBinding {
    pub(crate) cell: ValueCell,
    pub(crate) expected_version: crate::PublishedValueVersion,
    expected_storage: Rc<dyn ErasedCellStorage>,
    previous_shape: Option<ShapeInstance>,
    pub(crate) next_shape: Option<ShapeInstance>,
    next_storage: Option<CellStorageBinding>,
    pub(crate) changed: bool,
}

pub(crate) struct StagedManagedCellUpdate {
    pub domain: MemoryDomain,
    pub realized: crate::RealizedMemoryPlan,
    pub candidate: crate::CellPublicationCandidate,
}

struct ExactCellStorage<T> {
    reference: Ref<T>,
}

struct ManagedHostCellStorage {
    owner: MemoryDomain,
    realized: crate::RealizedMemoryPlan,
    object: crate::PlanObjectKey,
    region: crate::MemoryAccessRegion,
    representation: FunctionValueRepresentation,
}

/// Immutable canonical payload published behind a stable logical cell. A
/// replacement installs a new storage owner atomically; no mutable payload
/// `Ref` survives publication and detached snapshots share the frozen root.
struct ManagedCanonicalCellStorage {
    owner: MemoryDomain,
    realized: crate::RealizedMemoryPlan,
    object: crate::PlanObjectKey,
    payload: crate::PlanObjectKey,
    value: Value,
    representation: FunctionValueRepresentation,
}

fn planned_payload_object(
    owner: &MemoryDomain,
    realized: &crate::RealizedMemoryPlan,
    header: crate::PlanObjectKey,
) -> MResult<crate::PlanObjectKey> {
    planned_payload_object_optional(owner, realized, header)?
        .ok_or_else(|| managed_host_shape_error(header, "canonical header has no payload envelope"))
}

fn planned_payload_object_optional(
    owner: &MemoryDomain,
    realized: &crate::RealizedMemoryPlan,
    header: crate::PlanObjectKey,
) -> MResult<Option<crate::PlanObjectKey>> {
    let allocations: &[crate::AllocationPlan] = if let Some(plan) = realized.owned_value_plan() {
        &plan.allocations
    } else {
        #[cfg(feature = "functions")]
        {
            let Some(plan) = realized.call_plan() else {
                return Ok(None);
            };
            &plan.allocations
        }
        #[cfg(not(feature = "functions"))]
        {
            return Ok(None);
        }
    };
    let header_plan = allocations
        .iter()
        .find(|allocation| allocation.id == header.object())
        .ok_or_else(|| {
            managed_host_shape_error(header, "canonical header is absent from its plan")
        })?;
    let payload = allocations.iter().find(|allocation| {
        allocation.role == crate::AllocationRole::VariablePayload
            && allocation.owner == header_plan.owner
            && allocation.lifetime == header_plan.lifetime
    });
    payload
        .map(|payload| owner.plan_object_key(realized.revision(), payload.id))
        .transpose()
        .map_err(MechError::from)
}

fn realize_managed_canonical_value(
    owner: &MemoryDomain,
    value: Value,
    schemas: &SchemaTable,
) -> MResult<(
    crate::RealizedMemoryPlan,
    crate::PlanObjectKey,
    crate::PlanObjectKey,
    crate::MemoryAccessRegion,
    Value,
)> {
    owner.ensure_open().map_err(MechError::from)?;
    value.validate_against(schemas).map_err(snapshot_failure)?;
    let schema = schemas.get(value.schema()).cloned().ok_or_else(|| {
        snapshot_failure(SnapshotValueError::UnknownSnapshotSchema {
            schema: value.schema(),
        })
    })?;
    let descriptor = crate::ResolvedValueDescriptor::from_schema(schema, value.shape().clone())
        .map_err(MechError::from)?;
    let elements = descriptor
        .current_extents()
        .map_err(MechError::from)?
        .iter()
        .try_fold(1_u64, |total, extent| total.checked_mul(*extent))
        .ok_or_else(|| {
            MechError::new(
                crate::MemoryPlanError::ArithmeticOverflow {
                    field: "owned canonical element count",
                },
                None,
            )
            .with_compiler_loc()
        })?;
    let footprint = value.retained_footprint(schemas).map_err(|_| {
        MechError::from(crate::MemoryRuntimeError::CandidateValidationFailed {
            object: None,
            reason: "canonical value footprint is invalid for its declared schema".into(),
        })
    })?;
    let target = crate::TargetMemoryProfile::current_direct_host()
        .map_err(|error| MechError::new(error, None).with_compiler_loc())?;
    let storage = crate::physical_storage_descriptor(
        FunctionValueRepresentation::AnyValue,
        &target,
        crate::MemoryLifetime::Activation,
    );
    let plan = crate::plan_owned_value_memory(crate::ValueLayoutPlanningRequest {
        descriptor: &descriptor,
        storage: &storage,
        witness: crate::MemoryFootprintWitness::Known(crate::CurrentMemoryFootprint {
            logical_elements: elements,
            payload_bytes: footprint.retained_bytes,
            encoded_bytes: footprint.encoded_bytes,
            retained_nodes: footprint.node_count,
            shape_parameter_count: descriptor.shape().parameter_values().len() as u64,
            ..crate::CurrentMemoryFootprint::default()
        }),
        target: &target,
    })
    .map_err(|error| MechError::new(error, None).with_compiler_loc())?;
    let object_id = plan.allocations[0].id;
    let region = crate::memory_runtime::planned_value_access_region(&plan.value)?;
    let realized = owner.realize_owned_value_plan(plan)?;
    let object = owner.plan_object_key(realized.revision(), object_id)?;
    let payload = planned_payload_object(owner, &realized, object)?;
    let allocator = owner.planned_allocator(&realized, payload)?;
    let ownership = allocator
        .admit_frozen_snapshot(footprint.retained_bytes, footprint.node_count)
        .map_err(MechError::from)?;
    let value = value.into_retained_payload_ticket(ownership);
    let initialized = realized.binding(object)?.required_initialization_bytes();
    realized.record_initialized(object, initialized)?;
    Ok((realized, object, payload, region, value))
}

impl ManagedHostCellStorage {
    fn snapshot_with_authority(
        &self,
        cell: Option<&ValueCell>,
        schema: SchemaId,
        shape: &ShapeInstance,
        schemas: &SchemaTable,
    ) -> MResult<Value> {
        self.owner.ensure_open().map_err(MechError::from)?;
        let _scope = self
            .owner
            .enter_cell_read_scope()
            .map_err(MechError::from)?;
        let request = crate::CallAccessRequest {
            object: self.object,
            mode: crate::MemoryAccessMode::Read,
            region: self.region,
        };
        let prepared = if let Some(cell) = cell {
            self.owner
                .prepare_cell_access(&self.realized, cell, request, false)
        } else {
            self.owner.prepare_call(&self.realized, &[request])
        }
        .map_err(MechError::from)?;
        let frame = self
            .owner
            .acquire_call(&self.realized, &prepared)
            .map_err(MechError::from)?;
        let data = snapshot_managed_host_data(&frame, self.object, self.representation)?;
        drop(frame);
        finalize_draft(schema, shape, schemas, data)
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum CellBorrowState {
    Available,
    Borrowed,
}

impl<T: CanonicalCellBacking> ErasedCellStorage for ExactCellStorage<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn representation(&self, schema: &SchemaBody) -> FunctionValueRepresentation {
        T::representation(schema)
    }

    fn snapshot(
        &self,
        schema: SchemaId,
        shape: &ShapeInstance,
        schemas: &SchemaTable,
    ) -> MResult<Value> {
        self.reference
            .try_borrow()
            .map_err(|_| borrow_conflict(CellAccess::Snapshot))?
            .snapshot_bound(schema, shape, schemas)
    }

    fn replace(&self, value: &Value) -> MResult<()> {
        self.reference
            .try_borrow_mut()
            .map_err(|_| borrow_conflict(CellAccess::Replace))?
            .replace_bound(value)
    }

    fn preflight_replace(&self) -> MResult<()> {
        self.reference
            .try_borrow_mut()
            .map(|_| ())
            .map_err(|_| borrow_conflict(CellAccess::Replace))
    }

    fn capabilities(&self) -> crate::StorageCapabilityDescriptor {
        crate::runtime_storage::actual_backing_capabilities(T::REPRESENTATION)
    }

    fn detached_clone(&self) -> MResult<DetachedCellStorage> {
        let value = self
            .reference
            .try_borrow()
            .map_err(|_| borrow_conflict(CellAccess::Snapshot))?
            .clone();
        let reference = Ref::new(value);
        let identity = reference.reactive_cell_id();
        Ok(DetachedCellStorage {
            identity,
            storage: Rc::new(Self { reference }),
        })
    }

    fn same_storage(&self, other: &dyn ErasedCellStorage) -> bool {
        other
            .as_any()
            .downcast_ref::<Self>()
            .is_some_and(|other| self.reference.same_handle(&other.reference))
    }

    fn borrow_state(&self) -> CellBorrowState {
        if self.reference.try_borrow().is_ok() {
            CellBorrowState::Available
        } else {
            CellBorrowState::Borrowed
        }
    }
}

impl ErasedCellStorage for ManagedHostCellStorage {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn representation(&self, _: &SchemaBody) -> FunctionValueRepresentation {
        self.representation
    }

    fn snapshot(
        &self,
        schema: SchemaId,
        shape: &ShapeInstance,
        schemas: &SchemaTable,
    ) -> MResult<Value> {
        self.snapshot_with_authority(None, schema, shape, schemas)
    }

    fn replace(&self, _: &Value) -> MResult<()> {
        Err(MechError::from(
            crate::MemoryRuntimeError::CandidateValidationFailed {
                object: Some(self.object.object()),
                reason: "managed host storage changes require a planned publication".into(),
            },
        ))
    }

    fn preflight_replace(&self) -> MResult<()> {
        self.owner.ensure_open().map_err(MechError::from)
    }

    fn capabilities(&self) -> crate::StorageCapabilityDescriptor {
        crate::runtime_storage::actual_backing_capabilities(self.representation)
    }

    fn detached_clone(&self) -> MResult<DetachedCellStorage> {
        Err(MechError::from(
            crate::MemoryRuntimeError::CandidateValidationFailed {
                object: Some(self.object.object()),
                reason: "managed storage is detached through its canonical snapshot".into(),
            },
        ))
    }

    fn same_storage(&self, other: &dyn ErasedCellStorage) -> bool {
        other.as_any().downcast_ref::<Self>().is_some_and(|other| {
            self.owner.id() == other.owner.id()
                && self.realized.revision() == other.realized.revision()
                && self.object == other.object
        })
    }

    fn borrow_state(&self) -> CellBorrowState {
        CellBorrowState::Available
    }

    fn managed_host_binding(&self) -> Option<ManagedHostCellBinding> {
        Some(ManagedHostCellBinding {
            realized: self.realized.clone(),
            object: self.object,
            region: self.region,
        })
    }
}

impl ErasedCellStorage for ManagedCanonicalCellStorage {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn representation(&self, schema: &SchemaBody) -> FunctionValueRepresentation {
        let _ = schema;
        self.representation
    }

    fn snapshot(
        &self,
        schema: SchemaId,
        shape: &ShapeInstance,
        schemas: &SchemaTable,
    ) -> MResult<Value> {
        self.value
            .rebind(schema, shape, schemas)
            .map_err(snapshot_failure)
    }

    fn replace(&self, _: &Value) -> MResult<()> {
        Err(MechError::from(
            crate::MemoryRuntimeError::CandidateValidationFailed {
                object: None,
                reason: "managed canonical storage changes require atomic binding publication"
                    .into(),
            },
        ))
    }

    fn preflight_replace(&self) -> MResult<()> {
        Ok(())
    }

    fn capabilities(&self) -> crate::StorageCapabilityDescriptor {
        // The published payload is one immutable, recursively canonical root
        // regardless of the exact language representation retained for
        // factory matching. In particular, an admitted dynamic matrix root is
        // not the abstract `AnyStorage` physical representation.
        crate::runtime_storage::actual_backing_capabilities(FunctionValueRepresentation::AnyValue)
    }

    fn detached_clone(&self) -> MResult<DetachedCellStorage> {
        Err(MechError::from(
            crate::MemoryRuntimeError::CandidateValidationFailed {
                object: Some(self.object.object()),
                reason: "managed canonical storage is detached through its owning session".into(),
            },
        ))
    }

    fn same_storage(&self, other: &dyn ErasedCellStorage) -> bool {
        other.as_any().downcast_ref::<Self>().is_some_and(|other| {
            self.owner.id() == other.owner.id()
                && self.realized.revision() == other.realized.revision()
                && self.object == other.object
                && self.payload == other.payload
        })
    }

    fn borrow_state(&self) -> CellBorrowState {
        CellBorrowState::Available
    }
}

pub(crate) fn snapshot_managed_host_data(
    frame: &crate::KernelMemoryFrame<'_>,
    object: crate::PlanObjectKey,
    representation: FunctionValueRepresentation,
) -> MResult<ValueDataDraft> {
    macro_rules! scalar {
        ($type:ty, $variant:ident, $map:expr) => {
            frame
                .with_object_value_view::<$type, _>(object, |view| {
                    view.get_column_major(0)
                        .map(|value| ValueDataDraft::$variant(($map)(value)))
                })
                .map_err(MechError::from)?
                .ok_or_else(|| managed_host_shape_error(object, "scalar storage is empty"))
        };
    }
    #[cfg(feature = "matrix")]
    macro_rules! matrix {
        ($type:ty, $variant:ident, $map:expr) => {
            frame
                .with_object_value_view::<$type, _>(object, |view| {
                    let mut values = Vec::with_capacity(view.len());
                    for row in 0..view.rows() {
                        for column in 0..view.columns() {
                            let value = view.get(row, column).ok_or_else(|| {
                                managed_host_shape_error(
                                    object,
                                    "matrix coordinate is out of bounds",
                                )
                            })?;
                            values.push(ValueDataDraft::$variant(($map)(value)));
                        }
                    }
                    Ok::<_, MechError>(ValueDataDraft::Matrix(values.into_boxed_slice()))
                })
                .map_err(MechError::from)?
        };
    }
    match representation {
        #[cfg(feature = "u8")]
        FunctionValueRepresentation::U8 => scalar!(u8, U8, |value| value),
        #[cfg(feature = "u16")]
        FunctionValueRepresentation::U16 => scalar!(u16, U16, |value| value),
        #[cfg(feature = "u32")]
        FunctionValueRepresentation::U32 => scalar!(u32, U32, |value| value),
        #[cfg(feature = "u64")]
        FunctionValueRepresentation::U64 => scalar!(u64, U64, |value| value),
        #[cfg(feature = "u128")]
        FunctionValueRepresentation::U128 => scalar!(u128, U128, |value| value),
        #[cfg(feature = "i8")]
        FunctionValueRepresentation::I8 => scalar!(i8, I8, |value| value),
        #[cfg(feature = "i16")]
        FunctionValueRepresentation::I16 => scalar!(i16, I16, |value| value),
        #[cfg(feature = "i32")]
        FunctionValueRepresentation::I32 => scalar!(i32, I32, |value| value),
        #[cfg(feature = "i64")]
        FunctionValueRepresentation::I64 => scalar!(i64, I64, |value| value),
        #[cfg(feature = "i128")]
        FunctionValueRepresentation::I128 => scalar!(i128, I128, |value| value),
        #[cfg(feature = "f32")]
        FunctionValueRepresentation::F32 => scalar!(f32, F32, |value| {
            crate::snapshot::F32Bits::from_f32(value)
        }),
        #[cfg(feature = "f64")]
        FunctionValueRepresentation::F64 => scalar!(f64, F64, |value| {
            crate::snapshot::F64Bits::from_f64(value)
        }),
        #[cfg(feature = "bool")]
        FunctionValueRepresentation::Bool => scalar!(bool, Bool, |value| value),
        FunctionValueRepresentation::Id => {
            scalar!(
                crate::memory_runtime::ManagedId,
                Id,
                |value: crate::memory_runtime::ManagedId| value.0
            )
        }
        #[cfg(feature = "complex")]
        FunctionValueRepresentation::C64 => scalar!(crate::C64, Complex64, |value: crate::C64| {
            crate::snapshot::Complex64Bits::new(
                crate::snapshot::F64Bits::from_f64(value.0.re),
                crate::snapshot::F64Bits::from_f64(value.0.im),
            )
        }),
        #[cfg(feature = "rational")]
        FunctionValueRepresentation::R64 => frame
            .with_object_value_view::<crate::R64, _>(object, |view| {
                view.get_column_major(0)
                    .map(|value| ValueDataDraft::Rational64 {
                        numerator: *value.numer(),
                        denominator: value.denom().unsigned_abs(),
                    })
            })
            .map_err(MechError::from)?
            .ok_or_else(|| managed_host_shape_error(object, "scalar storage is empty")),
        FunctionValueRepresentation::Index => scalar!(usize, Index, |value| value as u64),
        #[cfg(feature = "matrix")]
        FunctionValueRepresentation::Matrix { element, .. } => match element {
            FunctionMatrixElement::Index => matrix!(usize, Index, |value| value as u64),
            #[cfg(feature = "u8")]
            FunctionMatrixElement::U8 => matrix!(u8, U8, |value| value),
            #[cfg(feature = "u16")]
            FunctionMatrixElement::U16 => matrix!(u16, U16, |value| value),
            #[cfg(feature = "u32")]
            FunctionMatrixElement::U32 => matrix!(u32, U32, |value| value),
            #[cfg(feature = "u64")]
            FunctionMatrixElement::U64 => matrix!(u64, U64, |value| value),
            #[cfg(feature = "u128")]
            FunctionMatrixElement::U128 => matrix!(u128, U128, |value| value),
            #[cfg(feature = "i8")]
            FunctionMatrixElement::I8 => matrix!(i8, I8, |value| value),
            #[cfg(feature = "i16")]
            FunctionMatrixElement::I16 => matrix!(i16, I16, |value| value),
            #[cfg(feature = "i32")]
            FunctionMatrixElement::I32 => matrix!(i32, I32, |value| value),
            #[cfg(feature = "i64")]
            FunctionMatrixElement::I64 => matrix!(i64, I64, |value| value),
            #[cfg(feature = "i128")]
            FunctionMatrixElement::I128 => matrix!(i128, I128, |value| value),
            #[cfg(feature = "f32")]
            FunctionMatrixElement::F32 => matrix!(f32, F32, |value| {
                crate::snapshot::F32Bits::from_f32(value)
            }),
            #[cfg(feature = "f64")]
            FunctionMatrixElement::F64 => matrix!(f64, F64, |value| {
                crate::snapshot::F64Bits::from_f64(value)
            }),
            #[cfg(feature = "bool")]
            FunctionMatrixElement::Bool => matrix!(bool, Bool, |value| value),
            #[cfg(feature = "complex")]
            FunctionMatrixElement::C64 => {
                matrix!(crate::C64, Complex64, |value: crate::C64| {
                    crate::snapshot::Complex64Bits::new(
                        crate::snapshot::F64Bits::from_f64(value.0.re),
                        crate::snapshot::F64Bits::from_f64(value.0.im),
                    )
                })
            }
            #[cfg(feature = "rational")]
            FunctionMatrixElement::R64 => frame
                .with_object_value_view::<crate::R64, _>(object, |view| {
                    let mut values = Vec::with_capacity(view.len());
                    for row in 0..view.rows() {
                        for column in 0..view.columns() {
                            let value = view.get(row, column).ok_or_else(|| {
                                managed_host_shape_error(
                                    object,
                                    "matrix coordinate is out of bounds",
                                )
                            })?;
                            values.push(ValueDataDraft::Rational64 {
                                numerator: *value.numer(),
                                denominator: value.denom().unsigned_abs(),
                            });
                        }
                    }
                    Ok::<_, MechError>(ValueDataDraft::Matrix(values.into_boxed_slice()))
                })
                .map_err(MechError::from)?,
            _ => Err(managed_host_shape_error(
                object,
                "the fixed-width managed matrix codec is not installed for this element",
            )),
        },
        _ => Err(managed_host_shape_error(
            object,
            "the fixed-width managed codec is not installed for this representation",
        )),
    }
}

fn managed_host_shape_error(object: crate::PlanObjectKey, reason: &'static str) -> MechError {
    MechError::from(crate::MemoryRuntimeError::InvalidLayout {
        object: Some(object.object()),
        size: 0,
        alignment: 1,
        reason,
    })
}

pub(crate) fn initialize_managed_object_from_value(
    frame: &mut crate::KernelMemoryFrame<'_>,
    object: crate::PlanObjectKey,
    representation: FunctionValueRepresentation,
    value: &Value,
) -> MResult<()> {
    macro_rules! scalar {
        ($type:ty, $variant:ident, $map:expr) => {
            scalar!(@checked $type, $variant, |value| Ok::<$type, MechError>(($map)(value)))
        };
        (@checked $type:ty, $variant:ident, $map:expr) => {{
            let ValueData::$variant(value) = value.data() else {
                return Err(managed_host_shape_error(
                    object,
                    "canonical scalar data disagrees with its physical representation",
                ));
            };
            let value: $type = ($map)(*value)?;
            frame.with_object_init_view::<$type, _>(object, |output| {
                if output.len() != 1 {
                    return Err(managed_host_shape_error(
                        object,
                        "scalar object does not contain exactly one logical element",
                    ));
                }
                output.try_fill_column_major(|_| Ok(value))
            })
        }};
    }
    #[cfg(feature = "matrix")]
    macro_rules! matrix {
        ($type:ty, $variant:ident, $map:expr) => {
            matrix!(@checked $type, $variant, |value| Ok::<$type, MechError>(($map)(value)))
        };
        (@checked $type:ty, $variant:ident, $map:expr) => {{
            let ValueData::Matrix(matrix) = value.data() else {
                return Err(managed_host_shape_error(
                    object,
                    "canonical matrix data disagrees with its physical representation",
                ));
            };
            let SequenceView::$variant(values) = matrix.elements() else {
                return Err(managed_host_shape_error(
                    object,
                    "canonical matrix element storage disagrees with its physical representation",
                ));
            };
            frame.with_object_init_view::<$type, _>(object, |output| {
                if output.len() != values.len() {
                    return Err(managed_host_shape_error(
                        object,
                        "canonical matrix length disagrees with its planned geometry",
                    ));
                }
                let rows = output.rows();
                let columns = output.columns();
                output.try_fill_column_major(|index| {
                    let row = index % rows;
                    let column = index / rows;
                    ($map)(values[row * columns + column])
                })
            })
        }};
    }

    match representation {
        #[cfg(feature = "u8")]
        FunctionValueRepresentation::U8 => scalar!(u8, U8, |value| value),
        #[cfg(feature = "u16")]
        FunctionValueRepresentation::U16 => scalar!(u16, U16, |value| value),
        #[cfg(feature = "u32")]
        FunctionValueRepresentation::U32 => scalar!(u32, U32, |value| value),
        #[cfg(feature = "u64")]
        FunctionValueRepresentation::U64 => scalar!(u64, U64, |value| value),
        #[cfg(feature = "u128")]
        FunctionValueRepresentation::U128 => scalar!(u128, U128, |value| value),
        #[cfg(feature = "i8")]
        FunctionValueRepresentation::I8 => scalar!(i8, I8, |value| value),
        #[cfg(feature = "i16")]
        FunctionValueRepresentation::I16 => scalar!(i16, I16, |value| value),
        #[cfg(feature = "i32")]
        FunctionValueRepresentation::I32 => scalar!(i32, I32, |value| value),
        #[cfg(feature = "i64")]
        FunctionValueRepresentation::I64 => scalar!(i64, I64, |value| value),
        #[cfg(feature = "i128")]
        FunctionValueRepresentation::I128 => scalar!(i128, I128, |value| value),
        #[cfg(feature = "f32")]
        FunctionValueRepresentation::F32 => scalar!(f32, F32, |value: crate::snapshot::F32Bits| {
            value.to_f32()
        }),
        #[cfg(feature = "f64")]
        FunctionValueRepresentation::F64 => scalar!(f64, F64, |value: crate::snapshot::F64Bits| {
            value.to_f64()
        }),
        #[cfg(feature = "bool")]
        FunctionValueRepresentation::Bool => scalar!(bool, Bool, |value| value),
        FunctionValueRepresentation::Id => {
            scalar!(crate::memory_runtime::ManagedId, Id, |value: u64| {
                crate::memory_runtime::ManagedId(value)
            })
        }
        FunctionValueRepresentation::Index => scalar!(@checked usize, Index, |value| {
            usize::try_from(value).map_err(|_| managed_host_shape_error(object, "canonical index exceeds the host index range"))
        }),
        #[cfg(feature = "complex")]
        FunctionValueRepresentation::C64 => {
            let ValueData::Complex64(value) = value.data() else {
                return Err(managed_host_shape_error(
                    object,
                    "canonical C64 data is invalid",
                ));
            };
            let value = crate::C64::new(value.real().to_f64(), value.imaginary().to_f64());
            frame.with_object_init_view::<crate::C64, _>(object, |output| {
                output.try_fill_column_major(|_| Ok(value))
            })
        }
        #[cfg(feature = "rational")]
        FunctionValueRepresentation::R64 => {
            let ValueData::Rational64(value) = value.data() else {
                return Err(managed_host_shape_error(
                    object,
                    "canonical R64 data is invalid",
                ));
            };
            let denominator = i64::try_from(value.denominator()).map_err(|_| {
                managed_host_shape_error(object, "rational denominator exceeds i64")
            })?;
            let value = crate::R64::new(value.numerator(), denominator);
            frame.with_object_init_view::<crate::R64, _>(object, |output| {
                output.try_fill_column_major(|_| Ok(value))
            })
        }
        #[cfg(feature = "matrix")]
        FunctionValueRepresentation::Matrix { element, .. } => match element {
            FunctionMatrixElement::Index => matrix!(@checked usize, Index, |value| {
                usize::try_from(value).map_err(|_| managed_host_shape_error(object, "canonical index exceeds the host index range"))
            }),
            #[cfg(feature = "u8")]
            FunctionMatrixElement::U8 => matrix!(u8, U8, |value| value),
            #[cfg(feature = "u16")]
            FunctionMatrixElement::U16 => matrix!(u16, U16, |value| value),
            #[cfg(feature = "u32")]
            FunctionMatrixElement::U32 => matrix!(u32, U32, |value| value),
            #[cfg(feature = "u64")]
            FunctionMatrixElement::U64 => matrix!(u64, U64, |value| value),
            #[cfg(feature = "u128")]
            FunctionMatrixElement::U128 => matrix!(u128, U128, |value| value),
            #[cfg(feature = "i8")]
            FunctionMatrixElement::I8 => matrix!(i8, I8, |value| value),
            #[cfg(feature = "i16")]
            FunctionMatrixElement::I16 => matrix!(i16, I16, |value| value),
            #[cfg(feature = "i32")]
            FunctionMatrixElement::I32 => matrix!(i32, I32, |value| value),
            #[cfg(feature = "i64")]
            FunctionMatrixElement::I64 => matrix!(i64, I64, |value| value),
            #[cfg(feature = "i128")]
            FunctionMatrixElement::I128 => matrix!(i128, I128, |value| value),
            #[cfg(feature = "f32")]
            FunctionMatrixElement::F32 => {
                matrix!(f32, F32, |value: crate::snapshot::F32Bits| value.to_f32())
            }
            #[cfg(feature = "f64")]
            FunctionMatrixElement::F64 => {
                matrix!(f64, F64, |value: crate::snapshot::F64Bits| value.to_f64())
            }
            #[cfg(feature = "bool")]
            FunctionMatrixElement::Bool => matrix!(bool, Bool, |value| value),
            #[cfg(feature = "complex")]
            FunctionMatrixElement::C64 => matrix!(
                crate::C64,
                Complex64,
                |value: crate::snapshot::Complex64Bits| {
                    crate::C64::new(value.real().to_f64(), value.imaginary().to_f64())
                }
            ),
            #[cfg(feature = "rational")]
            FunctionMatrixElement::R64 => {
                let ValueData::Matrix(matrix) = value.data() else {
                    return Err(managed_host_shape_error(
                        object,
                        "canonical matrix is invalid",
                    ));
                };
                let SequenceView::Rational64(values) = matrix.elements() else {
                    return Err(managed_host_shape_error(
                        object,
                        "canonical rational matrix element storage is invalid",
                    ));
                };
                frame.with_object_init_view::<crate::R64, _>(object, |output| {
                    if output.len() != values.len() {
                        return Err(managed_host_shape_error(
                            object,
                            "canonical matrix length disagrees with its planned geometry",
                        ));
                    }
                    let rows = output.rows();
                    let columns = output.columns();
                    output.try_fill_column_major(|index| {
                        let value = &values[(index % rows) * columns + index / rows];
                        let denominator = i64::try_from(value.denominator()).map_err(|_| {
                            managed_host_shape_error(object, "rational denominator exceeds i64")
                        })?;
                        Ok(crate::R64::new(value.numerator(), denominator))
                    })
                })
            }
            _ => Err(managed_host_shape_error(
                object,
                "the managed input codec is not installed for this matrix element",
            )),
        },
        _ => Err(managed_host_shape_error(
            object,
            "the managed input codec is not installed for this representation",
        )),
    }
}

/// Initializes only the logical elements of an admitted fixed-width output.
/// Spare capacity and stride gaps remain uninitialized.
#[cfg(feature = "functions")]
fn initialize_planned_default(
    frame: &mut crate::KernelMemoryFrame<'_>,
    object: crate::PlanObjectKey,
    slot: crate::PlannedSlotKind,
) -> MResult<()> {
    use crate::{FloatWidth, IntegerWidth, PlannedSlotKind, ScalarMemoryKind};
    macro_rules! fill {
        ($type:ty, $value:expr) => {
            frame.with_object_init_view::<$type, _>(object, |output| {
                output.try_fill_column_major(|_| Ok($value))
            })
        };
    }
    match slot {
        PlannedSlotKind::FixedScalar(kind) => match kind {
            #[cfg(feature = "bool")]
            ScalarMemoryKind::Bool => fill!(bool, false),
            ScalarMemoryKind::Unsigned(IntegerWidth::W8) => fill!(u8, 0),
            ScalarMemoryKind::Unsigned(IntegerWidth::W16) => fill!(u16, 0),
            ScalarMemoryKind::Unsigned(IntegerWidth::W32) => fill!(u32, 0),
            ScalarMemoryKind::Unsigned(IntegerWidth::W64) => fill!(u64, 0),
            ScalarMemoryKind::Unsigned(IntegerWidth::W128) => fill!(u128, 0),
            ScalarMemoryKind::Signed(IntegerWidth::W8) => fill!(i8, 0),
            ScalarMemoryKind::Signed(IntegerWidth::W16) => fill!(i16, 0),
            ScalarMemoryKind::Signed(IntegerWidth::W32) => fill!(i32, 0),
            ScalarMemoryKind::Signed(IntegerWidth::W64) => fill!(i64, 0),
            ScalarMemoryKind::Signed(IntegerWidth::W128) => fill!(i128, 0),
            ScalarMemoryKind::Floating(FloatWidth::W32) => fill!(f32, 0.0),
            ScalarMemoryKind::Floating(FloatWidth::W64) => fill!(f64, 0.0),
            ScalarMemoryKind::Id => fill!(
                crate::memory_runtime::ManagedId,
                crate::memory_runtime::ManagedId(0)
            ),
            ScalarMemoryKind::Index => fill!(usize, 1),
            #[cfg(feature = "complex")]
            ScalarMemoryKind::Complex(FloatWidth::W64) => {
                fill!(crate::C64, crate::C64::new(0.0, 0.0))
            }
            #[cfg(feature = "rational")]
            ScalarMemoryKind::Rational64 => fill!(crate::R64, crate::R64::new(0, 1)),
            _ => Err(managed_host_shape_error(
                object,
                "fixed output requires a supported sealed initialization codec",
            )),
        },
        _ => Err(managed_host_shape_error(
            object,
            "indirect output requires its admitted canonical builder",
        )),
    }
}

#[cfg(feature = "functions")]
pub(crate) fn value_from_managed_object(
    owner: &MemoryDomain,
    realized: &crate::RealizedMemoryPlan,
    object: crate::PlanObjectKey,
    region: crate::MemoryAccessRegion,
    representation: FunctionValueRepresentation,
    schema: SchemaId,
    shape: &ShapeInstance,
    schemas: &SchemaTable,
) -> MResult<Value> {
    ManagedHostCellStorage {
        owner: owner.clone(),
        realized: realized.clone(),
        object,
        region,
        representation,
    }
    .snapshot(schema, shape, schemas)
}

/// An opaque, schema-aware mutable program location.
///
/// Exact backing extraction is intentionally crate-private:
///
/// ```compile_fail
/// use mech_core::ValueCell;
///
/// fn expose(cell: &ValueCell) {
///     let _ = cell.try_ref::<f64>();
/// }
/// ```
#[derive(Clone)]
pub struct ValueCell {
    pub(crate) binding: CellBinding,
}

impl ValueCell {
    #[cfg(feature = "functions")]
    pub(crate) fn has_managed_canonical_storage(&self) -> MResult<bool> {
        let published = self
            .binding
            .published
            .try_borrow()
            .map_err(|_| borrow_conflict(CellAccess::Snapshot))?;
        Ok(matches!(
            published.storage,
            CellStorageBinding::ManagedCanonical { .. }
        ))
    }

    #[cfg(feature = "functions")]
    pub(crate) fn allocate_call_output_in(
        owner: &MemoryDomain,
        plan: Rc<crate::CallMemoryPlan>,
        representation: FunctionValueRepresentation,
    ) -> MResult<Self> {
        if plan.outputs.len() != 1 {
            return Err(
                MechError::new(crate::MemoryPlanError::DescriptorMismatch, None)
                    .with_compiler_loc(),
            );
        }
        let realized = owner.prepare_call_memory_realization(plan.clone())?;
        let output = &plan.outputs[0];
        let object = owner.plan_object_key(realized.revision(), output.object)?;
        let prepared = owner.prepare_owned_initialization(
            &realized,
            crate::CallAccessRequest {
                object,
                mode: crate::MemoryAccessMode::Write,
                region: crate::memory_runtime::planned_value_access_region(&output.value)?,
            },
        )?;
        {
            let _scope =
                owner.enter_realized_plan_point(&realized, crate::MemoryPlanPoint::new(0))?;
            let mut frame = owner.acquire_call(&realized, &prepared)?;
            initialize_planned_default(&mut frame, object, output.value.storage.planned_slot())?;
        }
        Self::allocate_planned(
            owner,
            &output.descriptor,
            representation,
            &realized,
            object,
            &output.value,
        )
    }

    /// Constructs one owned logical cell over an initialized R5 plan object.
    /// This boundary installs storage ownership; it never allocates backing
    /// from a runtime representation alone.
    pub fn allocate_planned(
        owner: &MemoryDomain,
        descriptor: &crate::ResolvedValueDescriptor,
        representation: FunctionValueRepresentation,
        realized: &crate::RealizedMemoryPlan,
        object: crate::PlanObjectKey,
        layout: &crate::ValueLayoutPlan,
    ) -> MResult<Self> {
        owner.ensure_open()?;
        if realized.domain() != owner.id() {
            return Err(crate::MemoryRuntimeError::WrongMemoryDomain {
                expected: owner.id(),
                actual: realized.domain(),
            }
            .into());
        }
        realized.binding(object)?;
        let region = crate::memory_runtime::planned_value_access_region(layout)?;
        let mut builder = SchemaTableBuilder::new();
        let handle = builder
            .insert(descriptor.schema().clone())
            .map_err(MechError::from)?;
        let build = builder.finish().map_err(MechError::from)?;
        let schema = build.resolve(handle).map_err(MechError::from)?;
        let cell = Self {
            binding: CellBinding::managed(
                crate::types::next_canonical_cell_id()?,
                schema,
                descriptor.schema().key(),
                descriptor.shape().clone(),
                Rc::new(build.table),
                owner.clone(),
                CellStorageBinding::ManagedHost {
                    owner: owner.clone(),
                    storage: Rc::new(ManagedHostCellStorage {
                        owner: owner.clone(),
                        realized: realized.clone(),
                        object,
                        region,
                        representation,
                    }),
                },
            ),
        };
        cell.validate_storage_contract()?;
        cell.snapshot()?;
        Ok(cell)
    }

    pub fn resolved_descriptor(&self) -> MResult<crate::ResolvedValueDescriptor> {
        let schemas = self.schema_table();
        let schema = schemas
            .find_by_key(self.schema_key())
            .and_then(|id| schemas.get(id))
            .cloned()
            .ok_or_else(|| {
                MechError::from(TypeResolutionError::incompatible(
                    "value cell descriptor",
                    TypeConstraintFailure::InvalidScheme {
                        reason: "the cell schema is absent from its canonical schema table".into(),
                    },
                ))
            })?;
        let shape = self.binding.try_shape(CellAccess::Snapshot)?.clone();
        crate::ResolvedValueDescriptor::from_schema(schema, shape).map_err(MechError::from)
    }

    /// Measures the currently published semantic value for R5/R6 live
    /// footprint resolution. This walks an immutable snapshot without
    /// rebuilding its canonical tree.
    #[cfg(feature = "functions")]
    pub fn current_memory_footprint(&self) -> MResult<crate::CurrentMemoryFootprint> {
        let descriptor = self.resolved_descriptor()?;
        let logical_elements = descriptor
            .current_extents()
            .map_err(MechError::from)?
            .iter()
            .try_fold(1_u64, |product, extent| product.checked_mul(*extent))
            .ok_or_else(|| {
                MechError::new(
                    crate::MemoryPlanError::ArithmeticOverflow {
                        field: "live value logical elements",
                    },
                    None,
                )
                .with_compiler_loc()
            })?;
        let snapshot = self.snapshot()?;
        let retained = snapshot
            .retained_footprint(self.schema_table().as_ref())
            .map_err(|error| {
                MechError::new(
                    crate::GenericError {
                        msg: format!("unable to measure live value: {error:?}"),
                    },
                    None,
                )
                .with_compiler_loc()
            })?;
        Ok(crate::CurrentMemoryFootprint {
            logical_elements,
            payload_bytes: retained.retained_bytes,
            encoded_bytes: retained.encoded_bytes,
            retained_nodes: retained.node_count,
            shape_parameter_count: descriptor.shape().parameter_values().len() as u64,
            ..crate::CurrentMemoryFootprint::default()
        })
    }

    /// Computes a conservative prospective footprint for a set assembled
    /// from borrowed immutable values. Duplicate canonical keys may make the
    /// finalized set smaller; no candidate payload is cloned or allocated by
    /// this planning traversal.
    #[cfg(feature = "functions")]
    pub fn prospective_set_memory_footprint(
        &self,
        values: impl IntoIterator<Item = MResult<Value>>,
    ) -> MResult<crate::CurrentMemoryFootprint> {
        let schemas = self.schema_table();
        let schema = schemas.get(self.schema()).ok_or_else(|| {
            snapshot_failure(SnapshotValueError::UnknownSnapshotSchema {
                schema: self.schema(),
            })
        })?;
        let SchemaBody::Set { element, .. } = schema.body() else {
            return Err(backing_mismatch::<Value>(self.representation()));
        };
        let mut elements = 0_u64;
        let mut encoded = 8_u64;
        let mut retained = u64::try_from(core::mem::size_of::<Value>())
            .ok()
            .and_then(|bytes| {
                (self.shape().parameter_values().len() as u64)
                    .checked_mul(core::mem::size_of::<u64>() as u64)
                    .and_then(|shape| bytes.checked_add(shape))
            })
            .and_then(|bytes| bytes.checked_add(core::mem::size_of::<ValueData>() as u64))
            .ok_or_else(|| {
                MechError::new(
                    crate::MemoryPlanError::ArithmeticOverflow {
                        field: "prospective set root bytes",
                    },
                    None,
                )
                .with_compiler_loc()
            })?;
        let mut nodes = 2_u64;
        for value in values {
            let value = value?;
            let value_schemas = value.schemas().ok_or_else(|| {
                MechError::new(crate::ValueSchemaContextUnavailable, None).with_compiler_loc()
            })?;
            let value_schema = value_schemas.get(value.schema()).ok_or_else(|| {
                snapshot_failure(SnapshotValueError::UnknownSnapshotSchema {
                    schema: value.schema(),
                })
            })?;
            if value_schema.body() != element.as_ref() {
                return Err(backing_mismatch::<Value>(self.representation()));
            }
            let footprint = crate::snapshot::canonical_data_retained_footprint(
                element,
                value.data(),
            )
            .map_err(|error| {
                MechError::new(
                    crate::GenericError {
                        msg: format!("unable to measure prospective set element: {error:?}"),
                    },
                    None,
                )
                .with_compiler_loc()
            })?;
            elements = elements.checked_add(1).ok_or_else(|| {
                MechError::new(
                    crate::MemoryPlanError::ArithmeticOverflow {
                        field: "prospective set elements",
                    },
                    None,
                )
                .with_compiler_loc()
            })?;
            encoded = encoded
                .checked_add(footprint.encoded_bytes)
                .ok_or_else(|| {
                    MechError::new(
                        crate::MemoryPlanError::ArithmeticOverflow {
                            field: "prospective set encoded bytes",
                        },
                        None,
                    )
                    .with_compiler_loc()
                })?;
            retained = retained
                .checked_add(core::mem::size_of::<crate::snapshot::CanonicalKeyValue>() as u64)
                .and_then(|bytes| bytes.checked_add(footprint.retained_bytes))
                .ok_or_else(|| {
                    MechError::new(
                        crate::MemoryPlanError::ArithmeticOverflow {
                            field: "prospective set retained bytes",
                        },
                        None,
                    )
                    .with_compiler_loc()
                })?;
            nodes = nodes.checked_add(footprint.node_count).ok_or_else(|| {
                MechError::new(
                    crate::MemoryPlanError::ArithmeticOverflow {
                        field: "prospective set retained nodes",
                    },
                    None,
                )
                .with_compiler_loc()
            })?;
        }
        Ok(crate::CurrentMemoryFootprint {
            logical_elements: elements,
            payload_bytes: retained,
            encoded_bytes: encoded,
            retained_nodes: nodes,
            shape_parameter_count: self.shape().parameter_values().len() as u64,
            ..crate::CurrentMemoryFootprint::default()
        })
    }

    pub fn validate_descriptor(&self, expected: &crate::ResolvedValueDescriptor) -> MResult<()> {
        let actual = self.resolved_descriptor()?;
        if actual != *expected {
            return Err(MechError::new(
                ExternalCellDescriptorMismatch {
                    expected: expected.resolved_type().semantic_name(),
                    actual: actual.resolved_type().semantic_name(),
                },
                None,
            )
            .with_compiler_loc());
        }
        self.validate_storage_contract()
    }

    /// Resolves this cell through its own canonical schema and current shape.
    /// The pure type system never imports physical cell or storage types.
    pub fn resolved_type(&self) -> MResult<ResolvedType> {
        let schemas = self.schema_table();
        let schema = schemas
            .find_by_key(self.schema_key())
            .and_then(|id| schemas.get(id))
            .ok_or_else(|| {
                MechError::from(TypeResolutionError::incompatible(
                    "source expression",
                    TypeConstraintFailure::InvalidScheme {
                        reason: "the cell schema is absent from its canonical schema table".into(),
                    },
                ))
            })?;
        ResolvedType::from_schema(schema, &self.shape()).map_err(MechError::from)
    }

    /// Resolves the extents owned by the cell's top-level aggregate schema.
    ///
    /// This is a boundary adapter for validating a semantically resolved
    /// result against a physical cell whose backing uses a less precise
    /// dynamic schema. Pure type-system modules remain independent of cells
    /// and storage representations.
    pub fn current_top_level_extents(&self) -> MResult<Box<[u64]>> {
        let schemas = self.schema_table();
        let schema = schemas
            .find_by_key(self.schema_key())
            .and_then(|id| schemas.get(id))
            .ok_or_else(|| {
                MechError::from(TypeResolutionError::incompatible(
                    "source expression",
                    TypeConstraintFailure::InvalidScheme {
                        reason: "the cell schema is absent from its canonical schema table".into(),
                    },
                ))
            })?;
        let shape = self.shape();
        let mut dimensions: Vec<&crate::DimensionExpr> = Vec::new();
        match schema.body() {
            SchemaBody::Matrix {
                dimensions: matrix_dimensions,
                ..
            } => dimensions.extend(matrix_dimensions.iter()),
            SchemaBody::Table {
                rows: CardinalitySpec::Exact(rows),
                ..
            } => dimensions.push(rows),
            SchemaBody::Set {
                cardinality: CardinalitySpec::Exact(cardinality),
                ..
            }
            | SchemaBody::Map {
                cardinality: CardinalitySpec::Exact(cardinality),
                ..
            } => dimensions.push(cardinality),
            _ => {}
        }
        dimensions
            .into_iter()
            .map(|dimension| shape.resolve_dimension(dimension).map_err(MechError::from))
            .collect::<MResult<Vec<_>>>()
            .map(Vec::into_boxed_slice)
    }

    /// Constructs the canonical empty-tuple value used as the output of an
    /// effect that does not otherwise return a value.
    pub fn unit() -> Self {
        Self::from_inferred_value_data(
            SchemaBody::Tuple(Vec::new().into_boxed_slice()),
            ValueDataDraft::Tuple(Vec::new().into_boxed_slice()),
        )
        .expect("the canonical unit schema and value are valid")
    }

    /// Constructs a standalone canonical cell from an exact scalar backing.
    pub fn from_exact<T>(value: T) -> MResult<Self>
    where
        T: CanonicalCellBacking,
    {
        Self::from_exact_in(&MemoryDomain::new().map_err(MechError::from)?, value)
    }

    /// Constructs an owned exact cell in an existing execution session.
    pub fn from_exact_in<T>(owner: &MemoryDomain, value: T) -> MResult<Self>
    where
        T: CanonicalCellBacking,
    {
        let matrix_extents = canonical_cell_sealed::Sealed::matrix_extents(&value);
        Self::from_inferred_ref_in(owner, Ref::new(value), matrix_extents)
    }

    /// Constructs a standalone canonical matrix cell from an exact backing.
    ///
    /// The supplied logical extents are validated when the first snapshot is
    /// captured; the backing remains private behind the cell binding.
    #[cfg(feature = "matrix")]
    pub fn from_exact_matrix_ref<T>(reference: Ref<T>, rows: usize, columns: usize) -> MResult<Self>
    where
        T: CanonicalCellBacking,
    {
        Self::from_inferred_ref_in(
            &MemoryDomain::new().map_err(MechError::from)?,
            reference,
            Some((rows, columns)),
        )
    }

    /// Constructs a fresh exact backing for a declared runtime output.
    ///
    /// Source specialization uses the runtime factory signature as the
    /// authority for storage representation while the operation supplies the
    /// resolved logical matrix dimensions. No erased universal value or
    /// mutable universal handle is involved in output construction.
    fn allocate_backing_for_representation_in(
        owner: &MemoryDomain,
        representation: FunctionValueRepresentation,
        _matrix_dimensions: Option<(usize, usize)>,
    ) -> MResult<Self> {
        macro_rules! scalar {
            ($value:expr) => {
                return Self::from_exact_in(owner, $value)
            };
        }
        match representation {
            #[cfg(feature = "u8")]
            FunctionValueRepresentation::U8 => scalar!(0_u8),
            #[cfg(feature = "u16")]
            FunctionValueRepresentation::U16 => scalar!(0_u16),
            #[cfg(feature = "u32")]
            FunctionValueRepresentation::U32 => scalar!(0_u32),
            #[cfg(feature = "u64")]
            FunctionValueRepresentation::U64 => scalar!(0_u64),
            #[cfg(feature = "u128")]
            FunctionValueRepresentation::U128 => scalar!(0_u128),
            #[cfg(feature = "i8")]
            FunctionValueRepresentation::I8 => scalar!(0_i8),
            #[cfg(feature = "i16")]
            FunctionValueRepresentation::I16 => scalar!(0_i16),
            #[cfg(feature = "i32")]
            FunctionValueRepresentation::I32 => scalar!(0_i32),
            #[cfg(feature = "i64")]
            FunctionValueRepresentation::I64 => scalar!(0_i64),
            #[cfg(feature = "i128")]
            FunctionValueRepresentation::I128 => scalar!(0_i128),
            #[cfg(feature = "f32")]
            FunctionValueRepresentation::F32 => scalar!(0.0_f32),
            #[cfg(feature = "f64")]
            FunctionValueRepresentation::F64 => scalar!(0.0_f64),
            #[cfg(feature = "complex")]
            FunctionValueRepresentation::C64 => scalar!(crate::C64::new(0.0, 0.0)),
            #[cfg(feature = "rational")]
            FunctionValueRepresentation::R64 => scalar!(crate::R64::new(0, 1)),
            #[cfg(feature = "bool")]
            FunctionValueRepresentation::Bool => scalar!(false),
            #[cfg(feature = "string")]
            FunctionValueRepresentation::String => scalar!(String::new()),
            FunctionValueRepresentation::Index => scalar!(1_usize),
            #[cfg(feature = "matrix")]
            FunctionValueRepresentation::Matrix { element, storage } => {
                let dimensions = _matrix_dimensions.ok_or_else(|| {
                    MechError::new(
                        ValueCellOutputConstructionUnsupported {
                            representation,
                            reason: "matrix dimensions were not supplied".into(),
                        },
                        None,
                    )
                    .with_compiler_loc()
                })?;
                macro_rules! matrix_element {
                    ($type:ty, $default:expr) => {
                        return default_matrix_cell_in::<$type>(owner, storage, dimensions, $default)
                    };
                }
                match element {
                    FunctionMatrixElement::Index => matrix_element!(usize, 1_usize),
                    #[cfg(feature = "bool")]
                    FunctionMatrixElement::Bool => matrix_element!(bool, false),
                    #[cfg(feature = "string")]
                    FunctionMatrixElement::String => matrix_element!(String, String::new()),
                    #[cfg(feature = "u8")]
                    FunctionMatrixElement::U8 => matrix_element!(u8, 0_u8),
                    #[cfg(feature = "u16")]
                    FunctionMatrixElement::U16 => matrix_element!(u16, 0_u16),
                    #[cfg(feature = "u32")]
                    FunctionMatrixElement::U32 => matrix_element!(u32, 0_u32),
                    #[cfg(feature = "u64")]
                    FunctionMatrixElement::U64 => matrix_element!(u64, 0_u64),
                    #[cfg(feature = "u128")]
                    FunctionMatrixElement::U128 => matrix_element!(u128, 0_u128),
                    #[cfg(feature = "i8")]
                    FunctionMatrixElement::I8 => matrix_element!(i8, 0_i8),
                    #[cfg(feature = "i16")]
                    FunctionMatrixElement::I16 => matrix_element!(i16, 0_i16),
                    #[cfg(feature = "i32")]
                    FunctionMatrixElement::I32 => matrix_element!(i32, 0_i32),
                    #[cfg(feature = "i64")]
                    FunctionMatrixElement::I64 => matrix_element!(i64, 0_i64),
                    #[cfg(feature = "i128")]
                    FunctionMatrixElement::I128 => matrix_element!(i128, 0_i128),
                    #[cfg(feature = "f32")]
                    FunctionMatrixElement::F32 => matrix_element!(f32, 0.0_f32),
                    #[cfg(feature = "f64")]
                    FunctionMatrixElement::F64 => matrix_element!(f64, 0.0_f64),
                    #[cfg(feature = "complex")]
                    FunctionMatrixElement::C64 => {
                        matrix_element!(crate::C64, crate::C64::new(0.0, 0.0))
                    }
                    #[cfg(feature = "rational")]
                    FunctionMatrixElement::R64 => {
                        matrix_element!(crate::R64, crate::R64::new(0, 1))
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        Err(MechError::new(
            ValueCellOutputConstructionUnsupported {
                representation,
                reason: "no exact canonical backing exists for this representation".into(),
            },
            None,
        )
        .with_compiler_loc())
    }

    #[cfg(all(test, feature = "f64", feature = "matrixd"))]
    fn test_backing_for_representation(
        representation: FunctionValueRepresentation,
        matrix_dimensions: Option<(usize, usize)>,
    ) -> MResult<Self> {
        let owner = MemoryDomain::new().map_err(MechError::from)?;
        Self::allocate_backing_for_representation_in(&owner, representation, matrix_dimensions)
    }

    pub fn allocate_for_descriptor(
        descriptor: &crate::ResolvedValueDescriptor,
        representation: FunctionValueRepresentation,
    ) -> MResult<Self> {
        let owner = MemoryDomain::new().map_err(MechError::from)?;
        Self::allocate_for_descriptor_in(&owner, descriptor, representation)
    }

    pub(crate) fn allocate_for_descriptor_in(
        owner: &MemoryDomain,
        descriptor: &crate::ResolvedValueDescriptor,
        representation: FunctionValueRepresentation,
    ) -> MResult<Self> {
        let capabilities = crate::runtime_storage::actual_backing_capabilities(representation);
        validate_storage_compatibility(descriptor.schema(), descriptor.shape(), &capabilities)?;
        if capabilities.topology == crate::StorageTopology::CanonicalValue {
            let data = initial_data_for_descriptor(descriptor)?;
            return Self::from_resolved_descriptor_data_in(owner, descriptor, data);
        }
        let extents = descriptor.current_extents().map_err(MechError::from)?;
        let dimensions = if matches!(representation, FunctionValueRepresentation::Matrix { .. }) {
            let [rows, columns] = extents.as_ref() else {
                return Err(MechError::new(
                    ValueCellOutputConstructionUnsupported {
                        representation,
                        reason: format!(
                            "matrix output descriptor has {} current extents",
                            extents.len()
                        ),
                    },
                    None,
                )
                .with_compiler_loc());
            };
            Some((
                usize::try_from(*rows).map_err(|_| {
                    MechError::new(
                        ValueCellOutputConstructionUnsupported {
                            representation,
                            reason: format!("row extent {rows} exceeds the host index range"),
                        },
                        None,
                    )
                    .with_compiler_loc()
                })?,
                usize::try_from(*columns).map_err(|_| {
                    MechError::new(
                        ValueCellOutputConstructionUnsupported {
                            representation,
                            reason: format!("column extent {columns} exceeds the host index range"),
                        },
                        None,
                    )
                    .with_compiler_loc()
                })?,
            ))
        } else {
            None
        };
        let allocated =
            Self::allocate_backing_for_representation_in(owner, representation, dimensions)?;
        let mut builder = SchemaTableBuilder::new();
        let handle = builder
            .insert(descriptor.schema().clone())
            .map_err(MechError::from)?;
        let build = builder.finish().map_err(MechError::from)?;
        let schema = build.resolve(handle).map_err(MechError::from)?;
        let identity = allocated.binding.identity;
        let owner = allocated.memory_domain().ok_or_else(|| {
            MechError::from(crate::MemoryRuntimeError::CandidateValidationFailed {
                object: None,
                reason: "owned output allocation has no memory session".into(),
            })
        })?;
        let storage = allocated
            .binding
            .published
            .try_borrow()
            .map_err(|_| borrow_conflict(CellAccess::Snapshot))?
            .storage
            .clone();
        if matches!(storage, CellStorageBinding::PinnedExternal(_)) {
            return Err(MechError::from(
                crate::MemoryRuntimeError::CandidateValidationFailed {
                    object: None,
                    reason: "owned output allocation resolved to pinned external storage".into(),
                },
            ));
        }
        let cell = Self {
            binding: CellBinding::managed(
                identity,
                schema,
                descriptor.schema().key(),
                descriptor.shape().clone(),
                Rc::new(build.table),
                owner,
                storage,
            ),
        };
        cell.validate_storage_contract()?;
        cell.snapshot()?;
        if cell.resolved_descriptor()? != *descriptor {
            return Err(MechError::new(
                ExternalCellDescriptorMismatch {
                    expected: descriptor.resolved_type().semantic_name(),
                    actual: cell.resolved_type()?.semantic_name(),
                },
                None,
            )
            .with_compiler_loc());
        }
        Ok(cell)
    }

    pub fn from_ref<T>(
        reference: Ref<T>,
        schema: SchemaId,
        shape: ShapeInstance,
        schemas: Rc<SchemaTable>,
    ) -> MResult<Self>
    where
        T: CanonicalCellBacking,
    {
        let schema_key = schemas
            .entry(schema)
            .map(|entry| entry.key())
            .ok_or_else(|| {
                snapshot_failure(SnapshotValueError::UnknownSnapshotSchema { schema })
            })?;
        let mut registration = reference
            .cell_registration()
            .try_borrow_mut()
            .map_err(|_| borrow_conflict(CellAccess::Replace))?;
        if let Some(record) = registration.upgrade() {
            if record.schema_key != schema_key {
                return Err(MechError::new(
                    ValueCellSchemaMismatch {
                        expected: record.schema_key,
                        actual: schema_key,
                    },
                    None,
                )
                .with_compiler_loc());
            }
            let cell = Self {
                binding: CellBinding {
                    record,
                    compiler_children: None,
                },
            };
            if *cell.shape() != shape {
                return Err(MechError::new(
                    ValueCellShapeMismatch {
                        expected: cell.shape().parameter_values().to_vec().into_boxed_slice(),
                        actual: shape.parameter_values().to_vec().into_boxed_slice(),
                    },
                    None,
                )
                .with_compiler_loc());
            }
            cell.snapshot()?;
            return Ok(cell);
        }
        let identity = reference.reactive_cell_id();
        let cell = Self {
            binding: CellBinding::pinned_external(
                identity,
                schema,
                schema_key,
                shape,
                schemas,
                Rc::new(ExactCellStorage {
                    reference: reference.clone(),
                }),
            ),
        };
        cell.validate_storage_contract()?;
        cell.snapshot()?;
        *registration = Rc::downgrade(&cell.binding.record);
        Ok(cell)
    }

    pub fn from_value(value: Value, schemas: Rc<SchemaTable>) -> MResult<Self> {
        let value = rebind_value(value, schemas.as_ref())?;
        Self::from_bound_value_in(
            &MemoryDomain::new().map_err(MechError::from)?,
            value,
            schemas,
        )
    }

    fn from_bound_value_in(
        owner: &MemoryDomain,
        value: Value,
        schemas: Rc<SchemaTable>,
    ) -> MResult<Self> {
        let representation = representation_for_schema(
            schemas
                .get(value.schema())
                .expect("canonical schema remains present")
                .body(),
        );
        Self::from_bound_value_with_representation_in(owner, value, schemas, representation)
    }

    fn from_bound_value_with_representation_in(
        owner: &MemoryDomain,
        value: Value,
        schemas: Rc<SchemaTable>,
        representation: FunctionValueRepresentation,
    ) -> MResult<Self> {
        let schema = value.schema();
        let schema_key = value.schema_key();
        let shape = value.shape().clone();
        debug_assert_eq!(
            schemas.entry(schema).map(|entry| entry.key()),
            Some(schema_key),
            "canonical value must retain its originating schema table"
        );
        let (realized, object, payload, _, value) =
            realize_managed_canonical_value(owner, value, &schemas)?;
        let identity = crate::types::next_canonical_cell_id()?;
        let cell = Self {
            binding: CellBinding::managed(
                identity,
                schema,
                schema_key,
                shape,
                schemas,
                owner.clone(),
                CellStorageBinding::ManagedCanonical {
                    owner: owner.clone(),
                    storage: Rc::new(ManagedCanonicalCellStorage {
                        owner: owner.clone(),
                        realized,
                        object,
                        payload,
                        representation,
                        value,
                    }),
                },
            ),
        };
        cell.validate_storage_contract()?;
        cell.snapshot()?;
        Ok(cell)
    }

    /// Creates a mutable cell from a detached canonical value and the schema
    /// context retained by that value.
    pub fn from_snapshot(value: Value) -> MResult<Self> {
        Self::from_snapshot_in(&MemoryDomain::new().map_err(MechError::from)?, value)
    }

    pub fn from_snapshot_in(owner: &MemoryDomain, value: Value) -> MResult<Self> {
        let schemas = value.schemas().ok_or_else(|| {
            MechError::new(ValueSchemaContextUnavailable, None).with_compiler_loc()
        })?;
        value.validate_against(&schemas).map_err(snapshot_failure)?;
        Self::from_bound_value_in(owner, value, Rc::new((*schemas).clone()))
    }

    /// Imports an owned value into an explicit execution session.
    ///
    /// This is a copy boundary, not mutable cross-domain sharing. Values that
    /// already belong to the requested session retain their logical identity;
    /// other values are snapshotted and reconstructed under the destination
    /// owner before they enter an executable program.
    pub fn import_owned_in(&self, owner: &MemoryDomain) -> MResult<Self> {
        if self
            .memory_domain()
            .is_some_and(|current| current.id() == owner.id())
        {
            return Ok(self.clone());
        }
        Self::from_runtime_value_in(owner, self.snapshot()?, self.schema_table())
    }

    /// Constructs an empty standalone set whose element schema is closed and
    /// whose current cardinality may change without changing schema identity.
    pub fn empty_dynamic_set(element: SchemaBody) -> MResult<Self> {
        Self::from_inferred_value_data(
            SchemaBody::Set {
                element: Box::new(element),
                cardinality: CardinalitySpec::Dynamic { upper_bound: None },
            },
            ValueDataDraft::Set(Vec::new().into_boxed_slice()),
        )
    }

    /// Constructs an empty table whose row extent can vary without replacing
    /// the table cell.
    pub fn empty_dynamic_table(columns: Box<[crate::SchemaField]>) -> MResult<Self> {
        let data = columns
            .iter()
            .map(|column| crate::snapshot::TableColumnDraft {
                name: column.name.clone(),
                values: Box::new([]),
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();
        Self::from_inferred_value_data(
            SchemaBody::Table {
                columns,
                rows: CardinalitySpec::Dynamic { upper_bound: None },
            },
            ValueDataDraft::Table(data),
        )
    }

    /// Constructs an empty map whose entry extent can vary without replacing
    /// the map cell.
    pub fn empty_dynamic_map(key: SchemaBody, value: SchemaBody) -> MResult<Self> {
        Self::from_inferred_value_data(
            SchemaBody::Map {
                key: Box::new(key),
                value: Box::new(value),
                cardinality: CardinalitySpec::Dynamic { upper_bound: None },
            },
            ValueDataDraft::Map(Box::new([])),
        )
    }

    /// Constructs a dense matrix whose dimensions are turn-scoped schema
    /// parameters. Subsequent replacements may change those dimensions while
    /// retaining the same schema and cell identity.
    pub fn dynamic_matrix(
        element: SchemaBody,
        dimensions: Box<[u64]>,
        elements: Box<[ValueDataDraft]>,
    ) -> MResult<Self> {
        let owner = MemoryDomain::new().map_err(MechError::from)?;
        Self::dynamic_matrix_in(&owner, element, dimensions, elements)
    }

    /// Constructs a dynamic matrix inside an existing execution session.
    pub fn dynamic_matrix_in(
        owner: &MemoryDomain,
        element: SchemaBody,
        dimensions: Box<[u64]>,
        elements: Box<[ValueDataDraft]>,
    ) -> MResult<Self> {
        let (schema, shape, schemas) = dynamic_matrix_schema(element, dimensions)?;
        let value = finalize_draft(
            schema,
            &shape,
            schemas.as_ref(),
            ValueDataDraft::Matrix(elements),
        )?;
        Self::from_runtime_value_in(owner, value, schemas)
    }

    /// Constructs a turn-varying rank-two matrix with a resizable matrix
    /// backing even when its initial shape happens to be a row or column.
    /// Dynamic kernel outputs use this when either axis may change later.
    #[doc(hidden)]
    pub fn dynamic_rank_matrix(
        element: SchemaBody,
        dimensions: Box<[u64]>,
        elements: Box<[ValueDataDraft]>,
    ) -> MResult<Self> {
        let (schema, shape, schemas) = dynamic_matrix_schema(element, dimensions)?;
        let value = finalize_draft(
            schema,
            &shape,
            schemas.as_ref(),
            ValueDataDraft::Matrix(elements),
        )?;
        let owner = MemoryDomain::new().map_err(MechError::from)?;
        #[cfg(all(feature = "matrix", feature = "matrixd"))]
        if let ValueData::Matrix(matrix) = value.data()
            && let Some(cell) = dynamic_matrix_cell(
                &owner,
                matrix.elements(),
                schema,
                &shape,
                schemas.clone(),
                true,
            )?
        {
            return Ok(cell);
        }
        Self::from_bound_value_in(&owner, value, schemas)
    }

    /// Constructs a standalone canonical cell from a closed schema body and
    /// matching canonical data draft.
    pub fn from_schema_data(body: SchemaBody, data: ValueDataDraft) -> MResult<Self> {
        Self::from_inferred_value_data(body, data)
    }

    /// Constructs canonical storage from an already validated semantic
    /// descriptor. The descriptor is checked again after finalization so
    /// callers cannot attach resolved type metadata to incompatible data.
    pub fn from_resolved_descriptor_data(
        descriptor: &crate::ResolvedValueDescriptor,
        data: ValueDataDraft,
    ) -> MResult<Self> {
        let owner = MemoryDomain::new().map_err(MechError::from)?;
        Self::from_resolved_descriptor_data_in(&owner, descriptor, data)
    }

    /// Constructs canonical storage for a resolved descriptor inside an
    /// existing execution session. Runtime specialization uses this path so
    /// indirect outputs do not quietly create a second per-cell domain.
    pub(crate) fn from_resolved_descriptor_data_in(
        owner: &MemoryDomain,
        descriptor: &crate::ResolvedValueDescriptor,
        data: ValueDataDraft,
    ) -> MResult<Self> {
        let mut builder = SchemaTableBuilder::new();
        let handle = builder
            .insert(descriptor.schema().clone())
            .map_err(MechError::from)?;
        let build = builder.finish().map_err(MechError::from)?;
        let schema = build.resolve(handle).map_err(MechError::from)?;
        let value = finalize_draft(schema, descriptor.shape(), &build.table, data)?;
        let cell = Self::from_runtime_value_in(owner, value, Rc::new(build.table))?;
        cell.validate_descriptor(descriptor)?;
        Ok(cell)
    }

    /// Constructs a detached heterogeneous tuple from canonical child cells.
    /// Child schemas are closed before embedding so no table-local schema id
    /// escapes into the new tuple's schema arena.
    pub fn tuple_from_cells(cells: &[Self]) -> MResult<Self> {
        let elements = cells
            .iter()
            .map(Self::closed_schema_body)
            .collect::<MResult<Vec<_>>>()?;
        let body = SchemaBody::Tuple(elements.clone().into_boxed_slice());
        let (schema, shape, schemas) = merged_schema(body, cells.iter())?;
        let values = cells
            .iter()
            .zip(&elements)
            .map(|(cell, expected)| {
                canonical_cell_draft_for_schema(cell, expected, schemas.as_ref())
            })
            .collect::<MResult<Vec<_>>>()?;
        let value = finalize_draft(
            schema,
            &shape,
            schemas.as_ref(),
            ValueDataDraft::Tuple(values.into_boxed_slice()),
        )?;
        let mut tuple = Self::from_runtime_value(value, schemas)?;
        tuple.binding.compiler_children = Some(cells.to_vec().into());
        Ok(tuple)
    }

    /// Rebuilds this tuple from current child-cell values in its existing
    /// merged schema arena.
    pub fn rebuild_tuple_cells(&self, cells: &[Self]) -> MResult<Value> {
        let SchemaBody::Tuple(elements) = self.closed_schema_body()? else {
            return Err(aggregate_rebuild_unsupported(self, "tuple"));
        };
        if elements.len() != cells.len() {
            return Err(aggregate_rebuild_arity(
                self,
                "tuple",
                elements.len(),
                cells.len(),
            ));
        }
        let values = cells
            .iter()
            .zip(&elements)
            .map(|(cell, expected)| {
                canonical_cell_draft_for_schema(cell, expected, self.binding.schemas.as_ref())
            })
            .collect::<MResult<Vec<_>>>()?;
        self.rebuild_data_draft(ValueDataDraft::Tuple(values.into_boxed_slice()))
    }

    /// Constructs a canonical record from named child cells in one schema
    /// arena, including concrete schemas retained below dynamic children.
    pub fn record_from_cells(fields: &[(String, Self)]) -> MResult<Self> {
        let schema_fields = fields
            .iter()
            .map(|(name, cell)| {
                Ok(crate::SchemaField {
                    name: name.clone(),
                    schema: cell.closed_schema_body()?,
                })
            })
            .collect::<MResult<Vec<_>>>()?;
        let body = SchemaBody::Record(schema_fields.clone().into_boxed_slice());
        let (schema, shape, schemas) = merged_schema(body, fields.iter().map(|(_, cell)| cell))?;
        let data = record_cell_fields_draft(fields, &schema_fields, schemas.as_ref())?;
        let value = finalize_draft(schema, &shape, schemas.as_ref(), data)?;
        Self::from_runtime_value(value, schemas)
    }

    /// Rebuilds this record from current named child-cell values in its
    /// existing merged schema arena.
    pub fn rebuild_record_cells(&self, fields: &[(String, Self)]) -> MResult<Value> {
        let SchemaBody::Record(schema_fields) = self.closed_schema_body()? else {
            return Err(aggregate_rebuild_unsupported(self, "record"));
        };
        if schema_fields.len() != fields.len() {
            return Err(aggregate_rebuild_arity(
                self,
                "record",
                schema_fields.len(),
                fields.len(),
            ));
        }
        let data = record_cell_fields_draft(fields, &schema_fields, self.binding.schemas.as_ref())?;
        self.rebuild_data_draft(data)
    }

    /// Constructs a canonical table from source cells while retaining every
    /// concrete schema needed by dynamic (`*`) columns in one schema arena.
    pub fn table_from_cell_columns(
        columns: Box<[(crate::SchemaField, Box<[Self]>)]>,
        rows: CardinalitySpec,
    ) -> MResult<Self> {
        let body = SchemaBody::Table {
            columns: columns
                .iter()
                .map(|(field, _)| field.clone())
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            rows,
        };
        let cells = columns
            .iter()
            .flat_map(|(_, values)| values.iter())
            .collect::<Vec<_>>();
        let (schema, shape, schemas) = merged_schema(body, cells.iter().copied())?;
        let data = table_cell_columns_draft(&columns, schemas.as_ref())?;
        let value = finalize_draft(schema, &shape, schemas.as_ref(), data)?;
        Self::from_runtime_value(value, schemas)
    }

    /// Rebuilds this table with current source-cell values using its existing
    /// merged arena. This preserves both table identity and dynamic children.
    pub fn rebuild_table_cell_columns(&self, columns: &[(String, Box<[Self]>)]) -> MResult<Value> {
        let SchemaBody::Table {
            columns: schema_columns,
            ..
        } = self.closed_schema_body()?
        else {
            return Err(MechError::new(
                ValueCellOutputConstructionUnsupported {
                    representation: self.representation(),
                    reason: "table-cell reconstruction requires a canonical table".into(),
                },
                None,
            )
            .with_compiler_loc());
        };
        if schema_columns.len() != columns.len() {
            return Err(MechError::new(
                ValueCellOutputConstructionUnsupported {
                    representation: self.representation(),
                    reason: format!(
                        "table schema has {} columns but {} columns were supplied",
                        schema_columns.len(),
                        columns.len(),
                    ),
                },
                None,
            )
            .with_compiler_loc());
        }
        let columns = schema_columns
            .into_vec()
            .into_iter()
            .zip(columns)
            .map(|(field, (name, values))| {
                if field.name != *name {
                    return Err(MechError::new(
                        ValueCellOutputConstructionUnsupported {
                            representation: self.representation(),
                            reason: format!(
                                "table schema column {} does not match supplied column {name}",
                                field.name,
                            ),
                        },
                        None,
                    )
                    .with_compiler_loc());
                }
                Ok((field, values.clone()))
            })
            .collect::<MResult<Vec<_>>>()?;
        let data = table_cell_columns_draft(&columns, self.binding.schemas.as_ref())?;
        self.rebuild_data_draft(data)
    }

    /// Constructs a row-major dynamic matrix from homogeneous canonical child
    /// cells. The output keeps one cell identity while later turns may change
    /// its dimensions through [`ValueCell::replace`].
    pub fn dynamic_matrix_from_cells(rows: usize, columns: usize, cells: &[Self]) -> MResult<Self> {
        if rows.saturating_mul(columns) != cells.len() {
            return Err(MechError::new(
                ValueCellOutputConstructionUnsupported {
                    representation: FunctionValueRepresentation::AnyValue,
                    reason: format!(
                        "matrix dimensions require {} elements but {} were supplied",
                        rows.saturating_mul(columns),
                        cells.len()
                    ),
                },
                None,
            )
            .with_compiler_loc());
        }
        let Some(first) = cells.first() else {
            return Err(MechError::new(
                ValueCellOutputConstructionUnsupported {
                    representation: FunctionValueRepresentation::AnyValue,
                    reason: "an empty matrix requires an explicit element schema".into(),
                },
                None,
            )
            .with_compiler_loc());
        };
        let element = first.closed_schema_body()?;
        let mut values = Vec::with_capacity(cells.len());
        for cell in cells {
            let candidate = cell.closed_schema_body()?;
            if candidate != element {
                return Err(MechError::new(
                    ValueCellOutputConstructionUnsupported {
                        representation: cell.representation(),
                        reason: "matrix elements must share one canonical schema".into(),
                    },
                    None,
                )
                .with_compiler_loc());
            }
            values.push(canonical_cell_draft(cell)?);
        }
        let owner = first.memory_domain().ok_or_else(|| {
            MechError::from(crate::MemoryRuntimeError::CandidateValidationFailed {
                object: None,
                reason: "owned matrix element has no memory session".into(),
            })
        })?;
        Self::dynamic_matrix_in(
            &owner,
            element,
            vec![rows as u64, columns as u64].into_boxed_slice(),
            values.into_boxed_slice(),
        )
    }

    /// Constructs a matrix whose schema is the already-resolved semantic
    /// output type. Direct source specializers use this boundary adapter when
    /// the result retains a compound dimension relation such as a sum of
    /// concatenated axes. Runtime storage remains selected independently.
    #[doc(hidden)]
    pub fn matrix_from_resolved_type_cells(
        resolved: &ResolvedType,
        rows: usize,
        columns: usize,
        cells: &[Self],
        schema_sources: &[Self],
    ) -> MResult<Self> {
        if rows.saturating_mul(columns) != cells.len() {
            return Err(MechError::new(
                ValueCellOutputConstructionUnsupported {
                    representation: FunctionValueRepresentation::AnyValue,
                    reason: format!(
                        "matrix dimensions require {} elements but {} were supplied",
                        rows.saturating_mul(columns),
                        cells.len()
                    ),
                },
                None,
            )
            .with_compiler_loc());
        }
        let crate::KindExpr::Matrix { dimensions, .. } = resolved.kind() else {
            return Err(MechError::from(TypeResolutionError::incompatible(
                "resolved matrix output",
                TypeConstraintFailure::StructuralMismatch {
                    expected: "matrix".into(),
                    actual: resolved.semantic_name(),
                },
            )));
        };
        let element = if let Some(first) = cells.first() {
            first.closed_schema_body()?
        } else if let Some(element) = schema_sources.iter().find_map(|source| {
            let SchemaBody::Matrix { element, .. } = source.closed_schema_body().ok()? else {
                return None;
            };
            Some(*element)
        }) {
            element
        } else {
            return Err(MechError::new(
                ValueCellOutputConstructionUnsupported {
                    representation: FunctionValueRepresentation::AnyValue,
                    reason: "an empty resolved matrix requires an element schema template".into(),
                },
                None,
            )
            .with_compiler_loc());
        };
        let mut values = Vec::with_capacity(cells.len());
        for cell in cells {
            if cell.closed_schema_body()? != element {
                return Err(MechError::new(
                    ValueCellOutputConstructionUnsupported {
                        representation: cell.representation(),
                        reason: "matrix elements must share one canonical schema".into(),
                    },
                    None,
                )
                .with_compiler_loc());
            }
            values.push(canonical_cell_draft(cell)?);
        }
        let draft = crate::SchemaDraft {
            dimension_parameters: resolved.dimension_parameters().to_vec().into_boxed_slice(),
            body: SchemaBody::Matrix {
                element: Box::new(element),
                dimensions: dimensions.clone(),
            },
        };
        let (schema, shape, schemas) = merged_resolved_matrix_schema(
            draft,
            vec![rows as u64, columns as u64].into_boxed_slice(),
            schema_sources,
        )?;
        let value = finalize_draft(
            schema,
            &shape,
            schemas.as_ref(),
            ValueDataDraft::Matrix(values.into_boxed_slice()),
        )?;
        let owner = cells
            .first()
            .or_else(|| schema_sources.first())
            .and_then(ValueCell::memory_domain)
            .ok_or_else(|| {
                MechError::from(crate::MemoryRuntimeError::CandidateValidationFailed {
                    object: None,
                    reason: "resolved matrix output has no owning memory session".into(),
                })
            })?;
        Self::from_runtime_value_in(&owner, value, schemas)
    }

    /// Returns this cell's schema with every shape parameter resolved to its
    /// current concrete extent. The returned body is safe to embed in a
    /// standalone derived-output schema.
    pub fn closed_schema_body(&self) -> MResult<SchemaBody> {
        let schema = self
            .binding
            .schemas
            .get(self.binding.schema)
            .expect("value-cell schema remains present");
        close_schema_body(schema.body(), &self.binding.shape())
    }

    /// Returns detached canonical tuple element cells, or `None` when this is
    /// not a tuple. Child schemas are closed against the parent's current
    /// shape before the cells are constructed.
    pub fn tuple_elements(&self) -> MResult<Option<Vec<Self>>> {
        let SchemaBody::Tuple(schemas) = self.closed_schema_body()? else {
            return Ok(None);
        };
        let draft = self
            .snapshot()?
            .canonical_data_draft()
            .map_err(snapshot_failure)?;
        let ValueDataDraft::Tuple(values) = draft else {
            unreachable!("validated tuple schema retains tuple data")
        };
        child_cells(schemas.into_vec(), values.into_vec()).map(Some)
    }

    /// Returns tuple child cells while retaining identities captured during
    /// canonical tuple assembly. Source destructuring uses this narrow path
    /// to keep reactive topology; ordinary value inspection remains detached.
    #[doc(hidden)]
    pub fn reactive_tuple_elements(&self) -> MResult<Option<Vec<Self>>> {
        let SchemaBody::Tuple(_) = self.closed_schema_body()? else {
            return Ok(None);
        };
        match &self.binding.compiler_children {
            Some(children) => Ok(Some(children.to_vec())),
            None => self.tuple_elements(),
        }
    }

    #[cfg(feature = "semantic-compiler")]
    pub(crate) fn compiler_composite_children(&self) -> Option<&[Self]> {
        self.binding.compiler_children.as_deref()
    }

    /// Returns detached canonical matrix element cells in row-major order, or
    /// `None` when this is not a matrix.
    pub fn matrix_elements(&self) -> MResult<Option<Vec<Self>>> {
        let SchemaBody::Matrix { element, .. } = self.closed_schema_body()? else {
            return Ok(None);
        };
        let draft = self
            .snapshot()?
            .canonical_data_draft()
            .map_err(snapshot_failure)?;
        let ValueDataDraft::Matrix(values) = draft else {
            unreachable!("validated matrix schema retains matrix data")
        };
        values
            .into_vec()
            .into_iter()
            .map(|value| Self::from_schema_data((*element).clone(), value))
            .collect::<MResult<Vec<_>>>()
            .map(Some)
    }

    /// Returns detached canonical set element cells in canonical key order,
    /// or `None` when this is not a set.
    pub fn set_element_cells(&self) -> MResult<Option<Vec<Self>>> {
        let SchemaBody::Set { element, .. } = self.closed_schema_body()? else {
            return Ok(None);
        };
        let draft = self
            .snapshot()?
            .canonical_data_draft()
            .map_err(snapshot_failure)?;
        let ValueDataDraft::Set(values) = draft else {
            unreachable!("validated set schema retains set data")
        };
        values
            .into_vec()
            .into_iter()
            .map(|value| Self::from_schema_data((*element).clone(), value))
            .collect::<MResult<Vec<_>>>()
            .map(Some)
    }

    /// Reconstructs the private exact backing used by typed function ports
    /// while retaining canonical schema and shape metadata. Aggregate values
    /// remain backed by immutable [`Value`] data.
    pub(crate) fn from_runtime_value(value: Value, schemas: Rc<SchemaTable>) -> MResult<Self> {
        let owner = MemoryDomain::new().map_err(MechError::from)?;
        Self::from_runtime_value_in(&owner, value, schemas)
    }

    /// Reconstructs runtime storage inside an existing execution session.
    /// Standalone convenience constructors delegate here with a fresh owner;
    /// program specialization supplies its shared owner explicitly.
    pub(crate) fn from_runtime_value_in(
        owner: &MemoryDomain,
        value: Value,
        schemas: Rc<SchemaTable>,
    ) -> MResult<Self> {
        let value = rebind_value(value, schemas.as_ref())?;
        let schema = value.schema();
        let shape = value.shape().clone();
        macro_rules! scalar {
            ($value:expr) => {
                return Self::from_owned_ref_in(owner, Ref::new($value), schema, shape, schemas)
            };
        }
        match value.data() {
            #[cfg(feature = "u8")]
            ValueData::U8(value) => scalar!(*value),
            #[cfg(feature = "u16")]
            ValueData::U16(value) => scalar!(*value),
            #[cfg(feature = "u32")]
            ValueData::U32(value) => scalar!(*value),
            #[cfg(feature = "u64")]
            ValueData::U64(value) => scalar!(*value),
            #[cfg(feature = "u128")]
            ValueData::U128(value) => scalar!(*value),
            #[cfg(feature = "i8")]
            ValueData::I8(value) => scalar!(*value),
            #[cfg(feature = "i16")]
            ValueData::I16(value) => scalar!(*value),
            #[cfg(feature = "i32")]
            ValueData::I32(value) => scalar!(*value),
            #[cfg(feature = "i64")]
            ValueData::I64(value) => scalar!(*value),
            #[cfg(feature = "i128")]
            ValueData::I128(value) => scalar!(*value),
            #[cfg(feature = "f32")]
            ValueData::F32(value) => scalar!(value.to_f32()),
            #[cfg(feature = "f64")]
            ValueData::F64(value) => scalar!(value.to_f64()),
            #[cfg(feature = "complex")]
            ValueData::Complex64(value) => scalar!(crate::C64::new(
                value.real().to_f64(),
                value.imaginary().to_f64(),
            )),
            #[cfg(feature = "rational")]
            ValueData::Rational64(value) => {
                if let Ok(denominator) = i64::try_from(value.denominator()) {
                    scalar!(crate::R64::new(value.numerator(), denominator));
                }
            }
            #[cfg(feature = "bool")]
            ValueData::Bool(value) => scalar!(*value),
            #[cfg(feature = "string")]
            ValueData::String(value) => scalar!(value.to_string()),
            ValueData::Index(value) => {
                if let Ok(value) = usize::try_from(*value) {
                    scalar!(value);
                }
            }
            #[cfg(all(feature = "matrix", feature = "matrixd"))]
            ValueData::Matrix(matrix) => {
                if let Some(cell) = dynamic_matrix_cell(
                    owner,
                    matrix.elements(),
                    schema,
                    &shape,
                    schemas.clone(),
                    false,
                )? {
                    return Ok(cell);
                }
            }
            _ => {}
        }
        Self::from_bound_value_in(owner, value, schemas)
    }

    /// Registers an explicitly caller-owned compatibility value without
    /// transferring its storage into the managed session. Ordinary owned
    /// constructors never use this path; it exists for embedding boundaries
    /// whose borrow behavior is itself part of the public contract.
    pub fn from_external_ref<T>(
        reference: Ref<T>,
        matrix_extents: Option<(usize, usize)>,
    ) -> MResult<Self>
    where
        T: CanonicalCellBacking,
    {
        if let (FunctionValueRepresentation::Matrix { element, storage }, Some((rows, columns))) =
            (T::REPRESENTATION, matrix_extents)
            && let FunctionMatrixStoragePattern::Exact(storage) = storage
            && matches!(
                storage,
                FunctionMatrixRepresentation::RowVectorD
                    | FunctionMatrixRepresentation::VectorD
                    | FunctionMatrixRepresentation::MatrixD
            )
        {
            let element = schema_body_for_matrix_element(element)
                .ok_or_else(|| backing_mismatch::<T>(T::REPRESENTATION))?;
            let (schema, shape, schemas) = match storage {
                FunctionMatrixRepresentation::RowVectorD => {
                    dynamic_row_vector_schema(element, rows as u64, columns as u64)?
                }
                FunctionMatrixRepresentation::VectorD => {
                    dynamic_column_vector_schema(element, rows as u64, columns as u64)?
                }
                FunctionMatrixRepresentation::MatrixD => dynamic_matrix_schema(
                    element,
                    vec![rows as u64, columns as u64].into_boxed_slice(),
                )?,
                _ => unreachable!("dynamic matrix representations were matched above"),
            };
            return Self::from_ref(reference, schema, shape, schemas);
        }
        let body = schema_body_for_representation(T::REPRESENTATION, matrix_extents)
            .ok_or_else(|| backing_mismatch::<T>(T::REPRESENTATION))?;
        let (schema, shape, schemas) = standalone_schema(body)?;
        Self::from_ref(reference, schema, shape, schemas)
    }

    #[cfg(test)]
    pub(crate) fn from_inferred_ref<T>(
        reference: Ref<T>,
        matrix_extents: Option<(usize, usize)>,
    ) -> MResult<Self>
    where
        T: CanonicalCellBacking,
    {
        Self::from_external_ref(reference, matrix_extents)
    }

    pub(crate) fn from_inferred_ref_in<T>(
        owner: &MemoryDomain,
        reference: Ref<T>,
        matrix_extents: Option<(usize, usize)>,
    ) -> MResult<Self>
    where
        T: CanonicalCellBacking,
    {
        if let (FunctionValueRepresentation::Matrix { element, storage }, Some((rows, columns))) =
            (T::REPRESENTATION, matrix_extents)
            && let FunctionMatrixStoragePattern::Exact(storage) = storage
            && matches!(
                storage,
                FunctionMatrixRepresentation::RowVectorD
                    | FunctionMatrixRepresentation::VectorD
                    | FunctionMatrixRepresentation::MatrixD
            )
        {
            let element = schema_body_for_matrix_element(element)
                .ok_or_else(|| backing_mismatch::<T>(T::REPRESENTATION))?;
            let (schema, shape, schemas) = match storage {
                FunctionMatrixRepresentation::RowVectorD => {
                    dynamic_row_vector_schema(element, rows as u64, columns as u64)?
                }
                FunctionMatrixRepresentation::VectorD => {
                    dynamic_column_vector_schema(element, rows as u64, columns as u64)?
                }
                FunctionMatrixRepresentation::MatrixD => dynamic_matrix_schema(
                    element,
                    vec![rows as u64, columns as u64].into_boxed_slice(),
                )?,
                _ => unreachable!("dynamic matrix representations were matched above"),
            };
            return Self::from_owned_ref_in(owner, reference, schema, shape, schemas);
        }
        let body = schema_body_for_representation(T::REPRESENTATION, matrix_extents)
            .ok_or_else(|| backing_mismatch::<T>(T::REPRESENTATION))?;
        let (schema, shape, schemas) = standalone_schema(body)?;
        Self::from_owned_ref_in(owner, reference, schema, shape, schemas)
    }

    /// Imports an exact decoded backing into an existing managed session while
    /// retaining the schema and shape declared by the artifact. The temporary
    /// `Ref<T>` is consumed during construction; it is never installed as a
    /// pinned-external cell binding.
    #[cfg(all(feature = "matrix", feature = "program"))]
    pub(crate) fn from_decoded_exact_backing_in<T>(
        owner: &MemoryDomain,
        reference: Ref<T>,
        schema: SchemaId,
        shape: ShapeInstance,
        schemas: Rc<SchemaTable>,
    ) -> MResult<Self>
    where
        T: CanonicalCellBacking,
    {
        Self::from_owned_ref_in(owner, reference, schema, shape, schemas)
    }

    fn from_owned_ref_in<T>(
        owner: &MemoryDomain,
        reference: Ref<T>,
        schema: SchemaId,
        shape: ShapeInstance,
        schemas: Rc<SchemaTable>,
    ) -> MResult<Self>
    where
        T: CanonicalCellBacking,
    {
        let target = crate::TargetMemoryProfile::current_direct_host()
            .map_err(|error| MechError::new(error, None).with_compiler_loc())?;
        let physical = crate::physical_storage_descriptor(
            T::REPRESENTATION,
            &target,
            crate::MemoryLifetime::Activation,
        );
        if matches!(physical.slot, crate::PlannedSlotKind::FixedScalar(_)) {
            let descriptor = crate::ResolvedValueDescriptor::from_schema(
                schemas.get(schema).cloned().ok_or_else(|| {
                    snapshot_failure(SnapshotValueError::UnknownSnapshotSchema { schema })
                })?,
                shape,
            )
            .map_err(MechError::from)?;
            let elements = descriptor
                .current_extents()
                .map_err(MechError::from)?
                .iter()
                .try_fold(1_u64, |size, extent| size.checked_mul(*extent))
                .ok_or_else(|| {
                    MechError::from(crate::MemoryRuntimeError::IdentityExhausted {
                        identity: "owned value element count",
                    })
                })?;
            let witness = crate::MemoryFootprintWitness::Known(crate::CurrentMemoryFootprint {
                logical_elements: elements,
                shape_parameter_count: descriptor.shape().parameter_values().len() as u64,
                ..crate::CurrentMemoryFootprint::default()
            });
            let plan = crate::plan_owned_value_memory(crate::ValueLayoutPlanningRequest {
                descriptor: &descriptor,
                storage: &physical,
                witness,
                target: &target,
            })
            .map_err(|error| MechError::new(error, None).with_compiler_loc())?;
            let object_id = plan.allocations[0].id;
            let realized = owner.realize_owned_value_plan(plan)?;
            let plan = realized
                .owned_value_plan()
                .expect("owned realization retains its R5 plan");
            let object = owner.plan_object_key(realized.revision(), object_id)?;
            let prepared = owner.prepare_owned_initialization(
                &realized,
                crate::CallAccessRequest {
                    object,
                    mode: crate::MemoryAccessMode::Write,
                    region: crate::memory_runtime::planned_value_access_region(&plan.value)?,
                },
            )?;
            {
                let value = reference
                    .try_borrow()
                    .map_err(|_| borrow_conflict(CellAccess::Snapshot))?;
                let mut frame = owner.acquire_call(&realized, &prepared)?;
                value.initialize_planned_fixed(&mut frame, object)?;
            }
            return Self::allocate_planned(
                owner,
                &descriptor,
                T::REPRESENTATION,
                &realized,
                object,
                &plan.value,
            );
        }
        // Indirect owned values are finalized before they enter the stable
        // logical cell. Keeping the constructor's temporary `Ref<T>` here
        // would make ordinary String and canonical matrix cells a disguised
        // pinned-external path. Snapshot once, then publish the immutable
        // canonical root behind the managed binding.
        let value = ExactCellStorage { reference }.snapshot(schema, &shape, &schemas)?;
        Self::from_bound_value_with_representation_in(owner, value, schemas, T::REPRESENTATION)
    }

    pub(crate) fn from_inferred_value_data(
        body: SchemaBody,
        data: ValueDataDraft,
    ) -> MResult<Self> {
        let (schema, shape, schemas) = standalone_schema(body)?;
        let value = finalize_draft(schema, &shape, &schemas, data)?;
        Self::from_runtime_value(value, schemas)
    }

    pub fn schema(&self) -> SchemaId {
        self.binding.schema
    }

    pub fn schema_key(&self) -> SchemaKey {
        self.binding.schema_key
    }

    pub fn shape(&self) -> cell::Ref<'_, ShapeInstance> {
        self.binding.shape()
    }

    #[cfg(feature = "functions")]
    pub(crate) fn accepts_published_shape(&self, next: &ShapeInstance) -> bool {
        let Some(schema) = self.binding.schemas.get(self.binding.schema) else {
            return false;
        };
        shape_change_allowed(schema, &self.binding.shape(), next)
    }

    pub(crate) fn schema_table(&self) -> Rc<SchemaTable> {
        self.binding.schemas.clone()
    }

    pub fn representation(&self) -> FunctionValueRepresentation {
        let schema = self
            .binding
            .schemas
            .get(self.binding.schema)
            .expect("value-cell schema remains present");
        self.binding
            .storage()
            .expect("value-cell published storage remains borrowable")
            .representation(schema.body())
    }

    pub fn type_memory_contract(&self) -> MResult<crate::TypeMemoryContract> {
        let schema = self
            .binding
            .schemas
            .get(self.binding.schema)
            .ok_or_else(|| {
                snapshot_failure(SnapshotValueError::UnknownSnapshotSchema {
                    schema: self.binding.schema,
                })
            })?;
        Ok(schema.type_memory_contract()?)
    }

    pub fn resolved_type_memory_contract(&self) -> MResult<crate::ResolvedTypeMemoryContract> {
        let schema = self
            .binding
            .schemas
            .get(self.binding.schema)
            .ok_or_else(|| {
                snapshot_failure(SnapshotValueError::UnknownSnapshotSchema {
                    schema: self.binding.schema,
                })
            })?;
        let shape = self.binding.try_shape(CellAccess::Snapshot)?.clone();
        Ok(schema.resolved_type_memory_contract(&shape)?)
    }

    pub fn storage_capabilities(&self) -> crate::StorageCapabilityDescriptor {
        self.binding
            .storage()
            .expect("value-cell published storage remains borrowable")
            .capabilities()
    }

    /// Rechecks the mandatory schema/storage compatibility invariant.
    pub fn validate_storage_contract(&self) -> MResult<()> {
        let schema = self
            .binding
            .schemas
            .get(self.binding.schema)
            .ok_or_else(|| {
                snapshot_failure(SnapshotValueError::UnknownSnapshotSchema {
                    schema: self.binding.schema,
                })
            })?;
        let shape = self.binding.try_shape(CellAccess::Snapshot)?.clone();
        validate_storage_compatibility(schema, &shape, &self.binding.storage()?.capabilities())
    }

    /// Describes whether this cell's schema permits its resolved extents to
    /// change while the cell identity remains stable.
    pub fn extent_evolution(&self) -> crate::ExtentEvolution {
        self.binding
            .schemas
            .get(self.binding.schema)
            .expect("value-cell schema remains present")
            .extent_evolution()
    }

    pub fn snapshot(&self) -> MResult<Value> {
        let shape = self.binding.shape().clone();
        let storage = self.binding.storage()?;
        if let Some(managed) = storage.as_any().downcast_ref::<ManagedHostCellStorage>() {
            managed.snapshot_with_authority(
                Some(self),
                self.binding.schema,
                &shape,
                &self.binding.schemas,
            )
        } else {
            storage.snapshot(self.binding.schema, &shape, &self.binding.schemas)
        }
    }

    /// Clones this cell's exact backing into a new, independent mutable cell.
    ///
    /// Schema, shape, and storage representation are retained while physical
    /// cell identity is deliberately fresh. Source specialization uses this
    /// for full-write outputs whose representation mirrors an input.
    pub fn detached_clone(&self) -> MResult<Self> {
        let storage = self.binding.storage()?;
        if let Some(managed) = storage
            .as_any()
            .downcast_ref::<ManagedCanonicalCellStorage>()
        {
            let cell = Self {
                binding: CellBinding::managed(
                    crate::types::next_canonical_cell_id()?,
                    self.binding.schema,
                    self.binding.schema_key,
                    self.binding.try_shape(CellAccess::Snapshot)?.clone(),
                    self.binding.schemas.clone(),
                    managed.owner.clone(),
                    CellStorageBinding::ManagedCanonical {
                        owner: managed.owner.clone(),
                        storage: storage.clone(),
                    },
                ),
            };
            cell.validate_storage_contract()?;
            return Ok(cell);
        }
        if let Some(owner) = self.memory_domain() {
            return Self::from_snapshot_in(&owner, self.snapshot()?);
        }
        let detached = storage.detached_clone()?;
        let shape = self.binding.try_shape(CellAccess::Snapshot)?.clone();
        let cell = Self {
            binding: CellBinding::pinned_external(
                detached.identity,
                self.binding.schema,
                self.binding.schema_key,
                shape,
                self.binding.schemas.clone(),
                detached.storage,
            ),
        };
        cell.validate_storage_contract()?;
        cell.snapshot()?;
        Ok(cell)
    }

    pub fn replace(&self, value: &Value) -> MResult<()> {
        if let Some(staged) = self.stage_managed_replacement(value)? {
            let prepared = staged
                .domain
                .prepare_cell_publication(&staged.realized, vec![staged.candidate])?;
            staged.domain.ready_cell_publication(prepared)?.commit();
            return Ok(());
        }
        if value.schema_key() != self.binding.schema_key {
            return Err(MechError::new(
                ValueCellSchemaMismatch {
                    expected: self.binding.schema_key,
                    actual: value.schema_key(),
                },
                None,
            )
            .with_compiler_loc());
        }
        let value = rebind_value(value.clone(), self.binding.schemas.as_ref())?;
        debug_assert_eq!(value.schema(), self.binding.schema);
        let mut published = self
            .binding
            .published
            .try_borrow_mut()
            .map_err(|_| borrow_conflict(CellAccess::Replace))?;
        if let Some(owner) = published.storage.owner() {
            owner.ensure_open().map_err(MechError::from)?;
        }
        let current_shape = published.shape.clone();
        let schema = self
            .binding
            .schemas
            .get(self.binding.schema)
            .expect("value-cell schema remains present");
        if !shape_change_allowed(schema, &current_shape, value.shape()) {
            return Err(MechError::new(
                ValueCellShapeMismatch {
                    expected: current_shape.parameter_values().to_vec().into_boxed_slice(),
                    actual: value.shape().parameter_values().to_vec().into_boxed_slice(),
                },
                None,
            )
            .with_compiler_loc());
        }
        value
            .validate_against(&self.binding.schemas)
            .map_err(snapshot_failure)?;
        let next_version = published
            .version
            .checked_successor("published value version")
            .map_err(|error| MechError::new(error, None).with_compiler_loc())?;
        published.storage.adapter().replace(&value)?;
        published.shape = value.shape().clone();
        published.version = next_version;
        Ok(())
    }

    /// Materializes a replacement only in the unpublished region selected
    /// by this value's R5 transaction. Callers can collect several candidates
    /// before obtaining a single publication gate.
    pub(crate) fn stage_managed_replacement(
        &self,
        value: &Value,
    ) -> MResult<Option<StagedManagedCellUpdate>> {
        let storage = self.binding.storage()?;
        if value.schema_key() != self.schema_key() {
            return Err(MechError::new(
                ValueCellSchemaMismatch {
                    expected: self.schema_key(),
                    actual: value.schema_key(),
                },
                None,
            )
            .with_compiler_loc());
        }
        let value = if value.schema() == self.schema() {
            value
                .validate_against(self.binding.schemas.as_ref())
                .map_err(snapshot_failure)?;
            value.clone()
        } else {
            rebind_value(value.clone(), self.binding.schemas.as_ref())?
        };
        let current = self.binding.try_shape(CellAccess::Replace)?;
        let schema = self
            .binding
            .schemas
            .get(self.schema())
            .expect("cell schema remains present");
        if !shape_change_allowed(schema, &current, value.shape()) {
            return Err(MechError::new(
                ValueCellShapeMismatch {
                    expected: current.parameter_values().to_vec().into_boxed_slice(),
                    actual: value.shape().parameter_values().to_vec().into_boxed_slice(),
                },
                None,
            )
            .with_compiler_loc());
        }
        drop(current);
        value
            .validate_against(&self.binding.schemas)
            .map_err(snapshot_failure)?;
        if let Some(managed) = storage
            .as_any()
            .downcast_ref::<ManagedCanonicalCellStorage>()
        {
            let (realized, object, _, region, value) =
                realize_managed_canonical_value(&managed.owner, value, &self.binding.schemas)?;
            let binding = realized.binding(object)?;
            return Ok(Some(StagedManagedCellUpdate {
                domain: managed.owner.clone(),
                realized,
                candidate: crate::CellPublicationCandidate {
                    cell: self.clone(),
                    object,
                    binding,
                    region,
                    value,
                    changed: true,
                },
            }));
        }
        let Some(managed) = storage.as_any().downcast_ref::<ManagedHostCellStorage>() else {
            return Ok(None);
        };
        let target = managed
            .realized
            .transactions()
            .iter()
            .find_map(|transaction| {
                let (current, staged) = match transaction {
                    crate::TransactionRequirement::StageAndSwap { current, staged } => {
                        (*current, *staged)
                    }
                    crate::TransactionRequirement::DoubleBuffer { current, next } => {
                        (*current, *next)
                    }
                    _ => return None,
                };
                if managed.object.object() == current {
                    Some(staged)
                } else if managed.object.object() == staged {
                    Some(current)
                } else {
                    None
                }
            })
            .ok_or_else(|| {
                managed_host_shape_error(
                    managed.object,
                    "cell update has no admitted transaction stage",
                )
            })?;
        let object = managed
            .owner
            .plan_object_key(managed.realized.revision(), target)?;
        let region = match managed.region {
            crate::MemoryAccessRegion::Rectangle {
                offset_bytes,
                row_stride_bytes,
                column_stride_bytes,
                element_bytes,
                ..
            } => {
                let descriptor = crate::ResolvedValueDescriptor::from_schema(
                    schema.clone(),
                    value.shape().clone(),
                )
                .map_err(MechError::from)?;
                let extents = descriptor.current_extents().map_err(MechError::from)?;
                let [rows, columns] = extents.as_ref() else {
                    return Err(managed_host_shape_error(
                        object,
                        "matrix replacement must have rank two",
                    ));
                };
                let capacity = managed.realized.binding(object)?.capacity_bytes();
                if (*rows != 0 && *columns != 0)
                    && (row_stride_bytes == 0
                        || column_stride_bytes == 0
                        || *rows > column_stride_bytes / row_stride_bytes
                        || *columns > capacity / column_stride_bytes)
                {
                    if managed.realized.owned_value_plan().is_some()
                        || managed.realized.has_call_plan()
                    {
                        return self
                            .stage_owned_relocation(managed, descriptor, value)
                            .map(Some);
                    }
                    return Err(MechError::from(
                        crate::MemoryRuntimeError::CapacityExceeded {
                            object: object.object(),
                            requested: rows
                                .checked_mul(*columns)
                                .and_then(|elements| elements.checked_mul(element_bytes))
                                .unwrap_or(u64::MAX),
                            capacity,
                        },
                    ));
                }
                crate::MemoryAccessRegion::Rectangle {
                    offset_bytes,
                    rows: *rows,
                    columns: *columns,
                    row_stride_bytes,
                    column_stride_bytes,
                    element_bytes,
                }
            }
            region => region,
        };
        {
            let _scope = managed
                .owner
                .enter_plan_point(crate::MemoryPlanPoint::new(0))?;
            let prepared = managed.owner.prepare_cell_access(
                &managed.realized,
                self,
                crate::CallAccessRequest {
                    object,
                    mode: crate::MemoryAccessMode::Write,
                    region,
                },
                true,
            )?;
            let mut frame = managed.owner.acquire_call(&managed.realized, &prepared)?;
            initialize_managed_object_from_value(
                &mut frame,
                object,
                managed.representation,
                &value,
            )?;
        }
        Ok(Some(StagedManagedCellUpdate {
            domain: managed.owner.clone(),
            realized: managed.realized.clone(),
            candidate: crate::CellPublicationCandidate {
                cell: self.clone(),
                object,
                binding: managed.realized.binding(object)?,
                region,
                value,
                changed: true,
            },
        }))
    }

    fn stage_owned_relocation(
        &self,
        managed: &ManagedHostCellStorage,
        descriptor: crate::ResolvedValueDescriptor,
        value: Value,
    ) -> MResult<StagedManagedCellUpdate> {
        #[cfg(feature = "functions")]
        let call_relocation = if let Some(call) = managed.realized.call_plan() {
            // A published call output can subsequently be changed through the
            // ordinary ValueCell API before the same FunctionInstance runs
            // again. Detach that logical value into a newly admitted
            // single-value realization; the bound instance will observe the
            // changed shape and replan its call on the next acquisition.
            let published = managed.object.object();
            let current = call
                .transactions
                .iter()
                .find_map(|transaction| match transaction {
                    crate::TransactionRequirement::StageAndSwap { current, staged }
                    | crate::TransactionRequirement::DoubleBuffer {
                        current,
                        next: staged,
                    } if *staged == published => Some(*current),
                    _ => None,
                })
                .unwrap_or(published);
            let output = call
                .outputs
                .iter()
                .position(|output| output.object == current)
                .ok_or_else(|| {
                    managed_host_shape_error(
                        managed.object,
                        "published call output has no retained R5 storage authority",
                    )
                })?;
            Some((call.output_storage[output].clone(), call.target.clone()))
        } else {
            None
        };
        #[cfg(not(feature = "functions"))]
        let call_relocation: Option<(
            crate::PhysicalStorageDescriptor,
            crate::TargetMemoryProfile,
        )> = None;

        let (storage, target) = if let Some(previous) = managed.realized.owned_value_plan() {
            (previous.storage.clone(), previous.target.clone())
        } else if let Some(authority) = call_relocation {
            authority
        } else {
            return Err(managed_host_shape_error(
                managed.object,
                "whole-cell growth requires retained R5 storage authority",
            ));
        };
        let elements = descriptor
            .current_extents()
            .map_err(MechError::from)?
            .iter()
            .try_fold(1_u64, |total, extent| total.checked_mul(*extent))
            .ok_or_else(|| {
                MechError::new(
                    crate::MemoryPlanError::ArithmeticOverflow {
                        field: "owned replacement elements",
                    },
                    None,
                )
                .with_compiler_loc()
            })?;
        let witness = crate::MemoryFootprintWitness::Known(crate::CurrentMemoryFootprint {
            logical_elements: elements,
            shape_parameter_count: descriptor.shape().parameter_values().len() as u64,
            ..crate::CurrentMemoryFootprint::default()
        });
        let plan = crate::plan_owned_value_memory(crate::ValueLayoutPlanningRequest {
            descriptor: &descriptor,
            storage: &storage,
            witness,
            target: &target,
        })
        .map_err(|error| MechError::new(error, None).with_compiler_loc())?;
        let object_id = plan.allocations[0].id;
        let region = crate::memory_runtime::planned_value_access_region(&plan.value)?;
        let realized = managed.owner.realize_owned_value_plan(plan)?;
        let object = managed
            .owner
            .plan_object_key(realized.revision(), object_id)?;
        let prepared = managed.owner.prepare_owned_initialization(
            &realized,
            crate::CallAccessRequest {
                object,
                mode: crate::MemoryAccessMode::Write,
                region,
            },
        )?;
        {
            let mut frame = managed.owner.acquire_call(&realized, &prepared)?;
            initialize_managed_object_from_value(
                &mut frame,
                object,
                managed.representation,
                &value,
            )?;
        }
        let binding = realized.binding(object)?;
        Ok(StagedManagedCellUpdate {
            domain: managed.owner.clone(),
            realized,
            candidate: crate::CellPublicationCandidate {
                cell: self.clone(),
                object,
                binding,
                region,
                value,
                changed: true,
            },
        })
    }

    pub(crate) fn prepare_managed_binding(
        &self,
        owner: &MemoryDomain,
        realized: &crate::RealizedMemoryPlan,
        object: crate::PlanObjectKey,
        region: crate::MemoryAccessRegion,
        value: &Value,
        changed: bool,
    ) -> MResult<PreparedManagedCellBinding> {
        let cell_owner = self.binding.memory_domain().ok_or_else(|| {
            MechError::from(crate::MemoryRuntimeError::CandidateValidationFailed {
                object: None,
                reason: "pinned external cells require an explicit fixed-copy publication adapter"
                    .into(),
            })
        })?;
        if cell_owner.id() != owner.id() {
            return Err(MechError::from(
                crate::MemoryRuntimeError::WrongMemoryDomain {
                    expected: cell_owner.id(),
                    actual: owner.id(),
                },
            ));
        }
        owner.ensure_open().map_err(MechError::from)?;
        if value.schema_key() != self.binding.schema_key {
            return Err(MechError::new(
                ValueCellSchemaMismatch {
                    expected: self.binding.schema_key,
                    actual: value.schema_key(),
                },
                None,
            )
            .with_compiler_loc());
        }
        let next = if value.schema() == self.binding.schema {
            value
                .validate_against(self.binding.schemas.as_ref())
                .map_err(snapshot_failure)?;
            value.clone()
        } else {
            rebind_value(value.clone(), self.binding.schemas.as_ref())?
        };
        let published = self
            .binding
            .published
            .try_borrow()
            .map_err(|_| borrow_conflict(CellAccess::Replace))?;
        let schema = self
            .binding
            .schemas
            .get(self.binding.schema)
            .expect("value-cell schema remains present");
        if !shape_change_allowed(schema, &published.shape, next.shape()) {
            return Err(MechError::new(
                ValueCellShapeMismatch {
                    expected: published
                        .shape
                        .parameter_values()
                        .to_vec()
                        .into_boxed_slice(),
                    actual: next.shape().parameter_values().to_vec().into_boxed_slice(),
                },
                None,
            )
            .with_compiler_loc());
        }
        next.validate_against(&self.binding.schemas)
            .map_err(snapshot_failure)?;
        let next_shape = next.shape().clone();
        let representation = self.representation();
        realized.binding(object).map_err(MechError::from)?;
        let next_storage = match planned_payload_object_optional(owner, realized, object)? {
            Some(payload) => CellStorageBinding::ManagedCanonical {
                owner: owner.clone(),
                storage: Rc::new(ManagedCanonicalCellStorage {
                    owner: owner.clone(),
                    realized: realized.clone(),
                    object,
                    payload,
                    value: next.clone(),
                    representation,
                }),
            },
            None => CellStorageBinding::ManagedHost {
                owner: owner.clone(),
                storage: Rc::new(ManagedHostCellStorage {
                    owner: owner.clone(),
                    realized: realized.clone(),
                    object,
                    region,
                    representation,
                }),
            },
        };
        Ok(PreparedManagedCellBinding {
            cell: self.clone(),
            expected_version: published.version,
            expected_storage: published.storage.adapter().clone(),
            previous_shape: Some(published.shape.clone()),
            next_shape: Some(next_shape),
            next_storage: Some(next_storage),
            changed,
        })
    }

    pub(crate) fn lock_publication(
        &self,
        prepared: &mut PreparedManagedCellBinding,
    ) -> MResult<()> {
        if self.binding.publication_locked.get() {
            return Err(MechError::from(
                crate::MemoryRuntimeError::PublicationInProgress,
            ));
        }
        let mut publication_shape = self
            .binding
            .publication_shape
            .try_borrow_mut()
            .map_err(|_| borrow_conflict(CellAccess::Replace))?;
        let valid = match self.binding.published.try_borrow_mut() {
            Ok(published) => {
                let valid = published.version == prepared.expected_version
                    && Rc::ptr_eq(published.storage.adapter(), &prepared.expected_storage);
                if valid {
                    *publication_shape = prepared.previous_shape.take();
                }
                valid
            }
            Err(_) => return Err(borrow_conflict(CellAccess::Replace)),
        };
        if !valid {
            return Err(MechError::from(
                crate::MemoryRuntimeError::CandidateValidationFailed {
                    object: None,
                    reason: "cell publication version changed after preparation".into(),
                },
            ));
        }
        self.binding.publication_locked.set(true);
        Ok(())
    }

    pub(crate) fn unlock_publication(&self) {
        self.binding.publication_locked.set(false);
        if let Ok(mut shape) = self.binding.publication_shape.try_borrow_mut() {
            *shape = None;
        }
    }

    pub(crate) fn install_managed_binding(
        &self,
        prepared: &mut PreparedManagedCellBinding,
        version: crate::PublishedValueVersion,
    ) {
        debug_assert!(self.same_logical_cell(&prepared.cell));
        let mut published = self.binding.published.borrow_mut();
        core::mem::swap(
            &mut published.storage,
            prepared
                .next_storage
                .as_mut()
                .expect("ready publication owns one candidate storage"),
        );
        core::mem::swap(
            &mut published.shape,
            prepared
                .next_shape
                .as_mut()
                .expect("ready publication owns one candidate shape"),
        );
        published.version = version;
    }

    /// Verifies that the cell can be mutably borrowed for a later atomic
    /// replacement without changing its identity.
    pub fn preflight_replace(&self) -> MResult<()> {
        let published = self
            .binding
            .published
            .try_borrow_mut()
            .map_err(|_| borrow_conflict(CellAccess::Replace))?;
        if let Some(owner) = published.storage.owner() {
            owner.ensure_open().map_err(MechError::from)?;
        }
        published.storage.adapter().preflight_replace()
    }

    pub fn memory_domain(&self) -> Option<MemoryDomain> {
        self.binding.memory_domain()
    }

    pub(crate) fn managed_host_binding(&self) -> MResult<Option<ManagedHostCellBinding>> {
        Ok(self.binding.storage()?.managed_host_binding())
    }

    #[cfg(feature = "functions")]
    pub(crate) fn requires_planned_import(
        &self,
        realized: &crate::RealizedMemoryPlan,
    ) -> MResult<bool> {
        Ok(self.managed_host_binding()?.is_none_or(|live| {
            live.realized.domain() != realized.domain()
                || live.realized.revision() != realized.revision()
        }))
    }

    pub fn same_logical_cell(&self, other: &Self) -> bool {
        self.binding.identity == other.binding.identity
    }

    pub fn same_storage(&self, other: &Self) -> bool {
        let Ok(storage) = self.binding.storage() else {
            return false;
        };
        let Ok(other_storage) = other.binding.storage() else {
            return false;
        };
        storage.same_storage(other_storage.as_ref())
    }

    /// Compatibility spelling for physical storage identity.
    ///
    /// New code should choose `same_logical_cell` or `same_storage`
    /// explicitly. This method retains its existing physical-storage meaning.
    pub fn same_cell(&self, other: &Self) -> bool {
        self.same_storage(other)
    }

    pub fn reactive_cell_id(&self) -> CanonicalCellId {
        self.binding.identity
    }

    /// Monotonic semantic publication version shared by every clone of this
    /// logical cell. Physical relocation alone does not advance it.
    pub fn published_version(&self) -> crate::PublishedValueVersion {
        self.binding.publication_version()
    }

    #[cfg(test)]
    pub(crate) fn test_with_identity_and_payload(
        identity_source: &Self,
        storage_source: &Self,
    ) -> MResult<Self> {
        let mut binding = CellBinding::pinned_external(
            identity_source.binding.identity,
            storage_source.binding.schema,
            storage_source.binding.schema_key,
            storage_source.binding.shape().clone(),
            storage_source.binding.schemas.clone(),
            storage_source.binding.storage()?,
        );
        binding.compiler_children = storage_source.binding.compiler_children.clone();
        Ok(Self { binding })
    }

    #[cfg(feature = "functions")]
    pub(crate) fn same_exact_ref<T: 'static>(&self, reference: &Ref<T>) -> bool {
        self.binding
            .storage()
            .ok()
            .and_then(|storage| {
                storage
                    .as_any()
                    .downcast_ref::<ExactCellStorage<T>>()
                    .map(|storage| storage.reference.same_handle(reference))
            })
            .unwrap_or(false)
    }

    #[cfg(feature = "semantic-compiler")]
    pub(crate) fn compiler_identity(&self) -> usize {
        self.binding.identity.get() as usize
    }

    /// Compares canonical schema, shape, and payload without considering cell
    /// identity or relying on either cell's local schema ids.
    pub fn snapshot_eq(&self, other: &Self) -> MResult<bool> {
        let left = self.snapshot()?;
        let right = other.snapshot()?;
        left.snapshot_eq(
            self.binding.schemas.as_ref(),
            &right,
            other.binding.schemas.as_ref(),
        )
        .map_err(snapshot_failure)
    }

    /// Compares two canonical key values using the schema-directed ordering
    /// rules used by sets and maps.
    pub fn key_eq(&self, other: &Self) -> MResult<bool> {
        let left = self.snapshot()?;
        let right = other.snapshot()?;
        left.key_cmp(
            self.binding.schemas.as_ref(),
            &right,
            other.binding.schemas.as_ref(),
        )
        .map(|ordering| ordering == core::cmp::Ordering::Equal)
        .map_err(snapshot_failure)
    }

    #[cfg(feature = "functions")]
    pub(crate) fn set_contains(&self, candidate: &Self) -> MResult<bool> {
        let set = self.snapshot()?;
        let candidate_value = candidate.snapshot()?;
        set.set_contains(
            self.binding.schemas.as_ref(),
            &candidate_value,
            candidate.binding.schemas.as_ref(),
        )
        .map_err(snapshot_failure)
    }

    #[cfg(feature = "functions")]
    pub(crate) fn set_elements_after_insert(&self, candidate: &Self) -> MResult<Box<[ValueData]>> {
        let set = self.snapshot()?;
        let candidate_value = candidate.snapshot()?;
        set.set_elements_after_insert(
            self.binding.schemas.as_ref(),
            &candidate_value,
            candidate.binding.schemas.as_ref(),
        )
        .map_err(snapshot_failure)
    }

    #[cfg(feature = "functions")]
    pub(crate) fn set_elements(&self) -> MResult<Box<[ValueData]>> {
        let snapshot = self.snapshot()?;
        let Some(set) = snapshot.set_view() else {
            return Err(backing_mismatch::<Value>(self.representation()));
        };
        Ok(set
            .elements()
            .iter()
            .map(|value| value.data().clone())
            .collect::<Vec<_>>()
            .into_boxed_slice())
    }

    #[cfg(feature = "functions")]
    pub(crate) fn set_element_drafts(&self) -> MResult<Box<[ValueDataDraft]>> {
        self.snapshot()?
            .set_element_drafts(self.binding.schemas.as_ref())
            .map_err(snapshot_failure)
    }

    #[cfg(feature = "functions")]
    pub(crate) fn set_elements_after_remove(&self, candidate: &Self) -> MResult<Box<[ValueData]>> {
        let set = self.snapshot()?;
        let candidate_value = candidate.snapshot()?;
        set.set_elements_after_remove(
            self.binding.schemas.as_ref(),
            &candidate_value,
            candidate.binding.schemas.as_ref(),
        )
        .map_err(snapshot_failure)
    }

    #[cfg(feature = "functions")]
    pub(crate) fn set_union_elements(&self, other: &Self) -> MResult<Box<[ValueData]>> {
        self.set_binary_elements(other, Value::set_union_elements)
    }

    #[cfg(feature = "functions")]
    pub(crate) fn set_intersection_elements(&self, other: &Self) -> MResult<Box<[ValueData]>> {
        self.set_binary_elements(other, Value::set_intersection_elements)
    }

    #[cfg(feature = "functions")]
    pub(crate) fn set_difference_elements(&self, other: &Self) -> MResult<Box<[ValueData]>> {
        self.set_binary_elements(other, Value::set_difference_elements)
    }

    #[cfg(feature = "functions")]
    pub(crate) fn set_symmetric_difference_elements(
        &self,
        other: &Self,
    ) -> MResult<Box<[ValueData]>> {
        self.set_binary_elements(other, Value::set_symmetric_difference_elements)
    }

    #[cfg(feature = "functions")]
    pub(crate) fn set_relation(
        &self,
        other: &Self,
        relation: crate::SetValueRelation,
    ) -> MResult<bool> {
        let left = self.snapshot()?;
        let right = other.snapshot()?;
        left.set_relation(
            self.binding.schemas.as_ref(),
            &right,
            other.binding.schemas.as_ref(),
            relation,
        )
        .map_err(snapshot_failure)
    }

    #[cfg(feature = "functions")]
    fn set_binary_elements(
        &self,
        other: &Self,
        operation: fn(
            &Value,
            &SchemaTable,
            &Value,
            &SchemaTable,
        ) -> Result<Box<[ValueData]>, SnapshotValueError>,
    ) -> MResult<Box<[ValueData]>> {
        let left = self.snapshot()?;
        let right = other.snapshot()?;
        operation(
            &left,
            self.binding.schemas.as_ref(),
            &right,
            other.binding.schemas.as_ref(),
        )
        .map_err(snapshot_failure)
    }

    #[cfg(feature = "functions")]
    pub(crate) fn rebuild_set(&self, elements: Box<[ValueData]>) -> MResult<Value> {
        let schema = self
            .binding
            .schemas
            .get(self.binding.schema)
            .expect("value-cell schema remains present");
        let SchemaBody::Set { element, .. } = schema.body() else {
            return Err(backing_mismatch::<Value>(self.representation()));
        };
        let drafts = elements
            .iter()
            .map(|value| crate::snapshot::canonical_snapshot_data_draft(element, value))
            .collect::<Result<Vec<_>, _>>()
            .map_err(snapshot_failure)?
            .into_boxed_slice();
        self.rebuild_set_drafts(drafts)
    }

    #[cfg(feature = "functions")]
    pub(crate) fn rebuild_set_drafts(&self, elements: Box<[ValueDataDraft]>) -> MResult<Value> {
        let data = ValueDataDraft::Set(elements);
        let schema = self
            .binding
            .schemas
            .get(self.binding.schema)
            .expect("value-cell schema remains present");
        let current_shape = self.binding.try_shape(CellAccess::Snapshot)?.clone();
        let shape = output_shape_for_data(schema, &data, &[], Some(&current_shape))?;
        ValueDraft {
            schema: self.binding.schema,
            shape_values: shape.parameter_values().to_vec().into_boxed_slice(),
            data,
        }
        .finalize(&SnapshotValidationContext::new(
            self.binding.schemas.as_ref(),
        ))
        .map_err(snapshot_failure)
    }

    /// Rebuilds this matrix's canonical value for new resolved dimensions.
    /// Dynamic dimensions may change; fixed dimensions remain enforced.
    pub fn rebuild_matrix_drafts(
        &self,
        dimensions: Box<[u64]>,
        elements: Box<[ValueDataDraft]>,
    ) -> MResult<Value> {
        let schema = self
            .binding
            .schemas
            .get(self.binding.schema)
            .expect("value-cell schema remains present");
        let SchemaBody::Matrix {
            dimensions: declared_dimensions,
            ..
        } = schema.body()
        else {
            return Err(backing_mismatch::<Value>(self.representation()));
        };
        if declared_dimensions.len() != dimensions.len() {
            return Err(MechError::new(
                ValueCellShapeMismatch {
                    expected: declared_dimensions
                        .iter()
                        .filter_map(|dimension| {
                            self.binding.shape().resolve_dimension(dimension).ok()
                        })
                        .collect::<Vec<_>>()
                        .into_boxed_slice(),
                    actual: dimensions,
                },
                None,
            )
            .with_compiler_loc());
        }

        let shape = matrix_shape_for_extents(schema, &dimensions, None)?;
        ValueDraft {
            schema: self.binding.schema,
            shape_values: shape.parameter_values().to_vec().into_boxed_slice(),
            data: ValueDataDraft::Matrix(elements),
        }
        .finalize(&SnapshotValidationContext::new(
            self.binding.schemas.as_ref(),
        ))
        .map_err(snapshot_failure)
    }

    /// Rebuilds canonical data against this cell's schema while retaining the
    /// cell's schema identity. Dynamic set, map, and table extents are
    /// validated by their declared cardinality policy.
    pub fn rebuild_data_draft(&self, data: ValueDataDraft) -> MResult<Value> {
        finalize_draft(
            self.binding.schema,
            &self.binding.shape(),
            self.binding.schemas.as_ref(),
            data,
        )
    }

    #[cfg(feature = "functions")]
    pub(crate) fn try_ref<T: 'static>(&self) -> MResult<Ref<T>> {
        let storage = self.binding.storage()?;
        let exact = storage
            .as_any()
            .downcast_ref::<ExactCellStorage<T>>()
            .map(|storage| storage.reference.clone());
        exact.ok_or_else(|| {
            MechError::new(
                ValueCellBackingMismatch {
                    expected: type_name::<T>().into(),
                    representation: self.representation(),
                },
                None,
            )
            .with_compiler_loc()
        })
    }
}

fn initial_data_for_descriptor(
    descriptor: &crate::ResolvedValueDescriptor,
) -> MResult<ValueDataDraft> {
    initial_data_for_schema(descriptor.schema().body(), descriptor.shape())
}

fn initial_data_for_schema(schema: &SchemaBody, shape: &ShapeInstance) -> MResult<ValueDataDraft> {
    use crate::snapshot::{Complex32Bits, Complex64Bits, F32Bits, F64Bits};
    Ok(match schema {
        SchemaBody::Dynamic => ValueDataDraft::Dynamic(None),
        SchemaBody::Bool => ValueDataDraft::Bool(false),
        SchemaBody::UnsignedInteger(crate::IntegerWidth::W8) => ValueDataDraft::U8(0),
        SchemaBody::UnsignedInteger(crate::IntegerWidth::W16) => ValueDataDraft::U16(0),
        SchemaBody::UnsignedInteger(crate::IntegerWidth::W32) => ValueDataDraft::U32(0),
        SchemaBody::UnsignedInteger(crate::IntegerWidth::W64) => ValueDataDraft::U64(0),
        SchemaBody::UnsignedInteger(crate::IntegerWidth::W128) => ValueDataDraft::U128(0),
        SchemaBody::SignedInteger(crate::IntegerWidth::W8) => ValueDataDraft::I8(0),
        SchemaBody::SignedInteger(crate::IntegerWidth::W16) => ValueDataDraft::I16(0),
        SchemaBody::SignedInteger(crate::IntegerWidth::W32) => ValueDataDraft::I32(0),
        SchemaBody::SignedInteger(crate::IntegerWidth::W64) => ValueDataDraft::I64(0),
        SchemaBody::SignedInteger(crate::IntegerWidth::W128) => ValueDataDraft::I128(0),
        SchemaBody::FloatingPoint(crate::FloatWidth::W32) => {
            ValueDataDraft::F32(F32Bits::from_f32(0.0))
        }
        SchemaBody::FloatingPoint(crate::FloatWidth::W64) => {
            ValueDataDraft::F64(F64Bits::from_f64(0.0))
        }
        SchemaBody::Complex(crate::FloatWidth::W32) => ValueDataDraft::Complex32(
            Complex32Bits::new(F32Bits::from_f32(0.0), F32Bits::from_f32(0.0)),
        ),
        SchemaBody::Complex(crate::FloatWidth::W64) => ValueDataDraft::Complex64(
            Complex64Bits::new(F64Bits::from_f64(0.0), F64Bits::from_f64(0.0)),
        ),
        SchemaBody::Rational64 => ValueDataDraft::Rational64 {
            numerator: 0,
            denominator: 1,
        },
        SchemaBody::String => ValueDataDraft::String(String::new()),
        SchemaBody::Id => ValueDataDraft::Id(0),
        SchemaBody::Index => ValueDataDraft::Index(1),
        SchemaBody::Atom(_) => ValueDataDraft::Atom,
        SchemaBody::Enum { variants, .. } => {
            let variant = variants.first().ok_or_else(|| {
                MechError::new(
                    ValueCellOutputConstructionUnsupported {
                        representation: FunctionValueRepresentation::Enum,
                        reason: "an enum output has no variant".into(),
                    },
                    None,
                )
                .with_compiler_loc()
            })?;
            ValueDataDraft::Enum(crate::snapshot::EnumDraft {
                ordinal: 0,
                payload: variant
                    .payload
                    .as_ref()
                    .map(|payload| initial_data_for_schema(payload, shape).map(Box::new))
                    .transpose()?,
            })
        }
        SchemaBody::Option(_) => ValueDataDraft::Option(crate::snapshot::OptionDraft {
            present: false,
            value: None,
        }),
        SchemaBody::Tuple(elements) => ValueDataDraft::Tuple(
            elements
                .iter()
                .map(|element| initial_data_for_schema(element, shape))
                .collect::<MResult<Vec<_>>>()?
                .into_boxed_slice(),
        ),
        SchemaBody::Record(fields) => ValueDataDraft::Record(
            fields
                .iter()
                .map(|field| {
                    Ok(crate::snapshot::NamedValueDraft {
                        name: field.name.clone(),
                        value: initial_data_for_schema(&field.schema, shape)?,
                    })
                })
                .collect::<MResult<Vec<_>>>()?
                .into_boxed_slice(),
        ),
        SchemaBody::Matrix {
            element,
            dimensions,
        } => {
            let element_count = dimensions.iter().try_fold(1_u64, |count, dimension| {
                count
                    .checked_mul(
                        shape
                            .resolve_dimension(dimension)
                            .map_err(MechError::from)?,
                    )
                    .ok_or_else(|| MechError::from(crate::SemanticModelError::DimensionOverflowV1))
            })?;
            let element_count = usize::try_from(element_count)
                .map_err(|_| MechError::from(crate::SemanticModelError::DimensionOverflowV1))?;
            let initial = initial_data_for_schema(element, shape)?;
            ValueDataDraft::Matrix(vec![initial; element_count].into_boxed_slice())
        }
        SchemaBody::Table { columns, rows } => {
            let row_count = exact_or_empty_extent(rows, shape)?;
            ValueDataDraft::Table(
                columns
                    .iter()
                    .map(|column| {
                        Ok(crate::snapshot::TableColumnDraft {
                            name: column.name.clone(),
                            values: vec![
                                initial_data_for_schema(&column.schema, shape)?;
                                row_count
                            ]
                            .into_boxed_slice(),
                        })
                    })
                    .collect::<MResult<Vec<_>>>()?
                    .into_boxed_slice(),
            )
        }
        SchemaBody::Set { cardinality, .. } => {
            require_empty_collection_extent(cardinality, shape, "set")?;
            ValueDataDraft::Set(Box::new([]))
        }
        SchemaBody::Map { cardinality, .. } => {
            require_empty_collection_extent(cardinality, shape, "map")?;
            ValueDataDraft::Map(Box::new([]))
        }
        SchemaBody::ReifiedType => ValueDataDraft::Type(crate::snapshot::ReifiedTypeDraft::Kind {
            kind: crate::KindExpr::Hole,
            dimension_parameters: Box::new([]),
        }),
    })
}

fn exact_or_empty_extent(extent: &CardinalitySpec, shape: &ShapeInstance) -> MResult<usize> {
    match extent {
        CardinalitySpec::Exact(expression) => usize::try_from(
            shape
                .resolve_dimension(expression)
                .map_err(MechError::from)?,
        )
        .map_err(|_| MechError::from(crate::SemanticModelError::DimensionOverflowV1)),
        CardinalitySpec::Dynamic { .. } => Ok(0),
    }
}

fn require_empty_collection_extent(
    extent: &CardinalitySpec,
    shape: &ShapeInstance,
    family: &'static str,
) -> MResult<()> {
    let count = exact_or_empty_extent(extent, shape)?;
    if count == 0 {
        Ok(())
    } else {
        Err(MechError::new(
            ValueCellOutputConstructionUnsupported {
                representation: FunctionValueRepresentation::AnyValue,
                reason: format!(
                    "an exact {family} output with {count} entries requires operation-provided data"
                ),
            },
            None,
        )
        .with_compiler_loc())
    }
}

fn canonical_cell_draft(cell: &ValueCell) -> MResult<ValueDataDraft> {
    cell.snapshot()?
        .canonical_data_draft()
        .map_err(snapshot_failure)
}

fn aggregate_rebuild_unsupported(cell: &ValueCell, expected: &'static str) -> MechError {
    MechError::new(
        ValueCellOutputConstructionUnsupported {
            representation: cell.representation(),
            reason: format!("{expected}-cell reconstruction requires a canonical {expected}"),
        },
        None,
    )
    .with_compiler_loc()
}

fn aggregate_rebuild_arity(
    cell: &ValueCell,
    aggregate: &'static str,
    expected: usize,
    actual: usize,
) -> MechError {
    MechError::new(
        ValueCellOutputConstructionUnsupported {
            representation: cell.representation(),
            reason: format!(
                "{aggregate} schema has {expected} children but {actual} children were supplied"
            ),
        },
        None,
    )
    .with_compiler_loc()
}

fn merged_schema<'a>(
    body: SchemaBody,
    cells: impl IntoIterator<Item = &'a ValueCell>,
) -> MResult<(SchemaId, ShapeInstance, Rc<SchemaTable>)> {
    let schema = crate::SchemaDraft {
        dimension_parameters: Vec::new().into_boxed_slice(),
        body,
    }
    .finalize()
    .map_err(|error| snapshot_failure(error.into()))?;
    let shape = schema
        .instantiate_shape(Vec::new().into_boxed_slice())
        .map_err(|error| snapshot_failure(error.into()))?;
    let mut builder = crate::SchemaTableBuilder::new();
    let handle = builder.insert(schema)?;
    for cell in cells {
        for entry in cell.binding.schemas.entries() {
            builder.insert(entry.schema().clone())?;
        }
    }
    let build = builder.finish()?;
    let schema = build.resolve(handle)?;
    Ok((schema, shape, Rc::new(build.table)))
}

#[cfg(feature = "functions")]
fn output_shape_for_data(
    schema: &crate::Schema,
    data: &ValueDataDraft,
    current_extents: &[u64],
    seed: Option<&ShapeInstance>,
) -> MResult<ShapeInstance> {
    if matches!(schema.body(), SchemaBody::Matrix { .. }) {
        return matrix_shape_for_extents(schema, current_extents, seed);
    }
    let mut values = if let Some(seed) = seed {
        schema
            .instantiate_shape(seed.parameter_values().to_vec().into_boxed_slice())
            .map_err(MechError::from)?
            .parameter_values()
            .to_vec()
    } else {
        let mut values = vec![0; schema.dimension_parameters().len()];
        for (index, parameter) in schema.dimension_parameters().iter().enumerate() {
            values[index] = crate::evaluate_dimension(parameter.lower_bound(), &values[..index])
                .map_err(MechError::from)?;
        }
        values
    };
    assign_data_extent_witness(schema.body(), data, &mut values)?;
    schema
        .instantiate_shape(values.into_boxed_slice())
        .map_err(MechError::from)
}

#[cfg(feature = "functions")]
fn assign_data_extent_witness(
    schema: &SchemaBody,
    data: &ValueDataDraft,
    values: &mut [u64],
) -> MResult<()> {
    match (schema, data) {
        (
            SchemaBody::Set {
                element,
                cardinality,
            },
            ValueDataDraft::Set(elements),
        ) => {
            assign_extent_witness(cardinality, elements.len() as u64, values)?;
            for element_data in elements {
                assign_data_extent_witness(element, element_data, values)?;
            }
        }
        (
            SchemaBody::Map {
                key,
                value,
                cardinality,
            },
            ValueDataDraft::Map(entries),
        ) => {
            assign_extent_witness(cardinality, entries.len() as u64, values)?;
            for entry in entries {
                if let [key_data, value_data] = entry.items.as_ref() {
                    assign_data_extent_witness(key, key_data, values)?;
                    assign_data_extent_witness(value, value_data, values)?;
                }
            }
        }
        (
            SchemaBody::Table {
                columns,
                rows: row_extent,
            },
            ValueDataDraft::Table(column_data),
        ) => {
            let row_count = column_data
                .first()
                .map(|column| column.values.len() as u64)
                .unwrap_or(0);
            assign_extent_witness(row_extent, row_count, values)?;
            for (column, data) in columns.iter().zip(column_data) {
                for value in &data.values {
                    assign_data_extent_witness(&column.schema, value, values)?;
                }
            }
        }
        (SchemaBody::Option(element), ValueDataDraft::Option(option)) => {
            if let Some(data) = option.value.as_deref() {
                assign_data_extent_witness(element, data, values)?;
            }
        }
        (SchemaBody::Tuple(elements), ValueDataDraft::Tuple(data)) => {
            for (element, data) in elements.iter().zip(data) {
                assign_data_extent_witness(element, data, values)?;
            }
        }
        (SchemaBody::Record(fields), ValueDataDraft::Record(data)) => {
            for (field, data) in fields.iter().zip(data) {
                assign_data_extent_witness(&field.schema, &data.value, values)?;
            }
        }
        _ => {}
    }
    Ok(())
}

#[cfg(feature = "functions")]
fn assign_extent_witness(
    extent: &CardinalitySpec,
    cardinality: u64,
    values: &mut [u64],
) -> MResult<()> {
    match extent {
        CardinalitySpec::Exact(expression) => {
            assign_dimension_witness(expression, cardinality, values)
        }
        CardinalitySpec::Dynamic {
            upper_bound: Some(upper),
        } if crate::evaluate_dimension(upper, values).map_err(MechError::from)? < cardinality => {
            assign_dimension_witness(upper, cardinality, values)
        }
        CardinalitySpec::Dynamic { .. } => Ok(()),
    }
}

fn merged_resolved_matrix_schema(
    draft: crate::SchemaDraft,
    dimensions: Box<[u64]>,
    cells: &[ValueCell],
) -> MResult<(SchemaId, ShapeInstance, Rc<SchemaTable>)> {
    let schema = draft
        .finalize()
        .map_err(|error| snapshot_failure(error.into()))?;
    let shape = matrix_shape_for_extents(&schema, &dimensions, None)?;
    let mut builder = crate::SchemaTableBuilder::new();
    let handle = builder.insert(schema)?;
    for cell in cells {
        for entry in cell.binding.schemas.entries() {
            builder.insert(entry.schema().clone())?;
        }
    }
    let build = builder.finish()?;
    let schema = build.resolve(handle)?;
    Ok((schema, shape, Rc::new(build.table)))
}

fn matrix_shape_for_extents(
    schema: &crate::Schema,
    dimensions: &[u64],
    seed: Option<&ShapeInstance>,
) -> MResult<ShapeInstance> {
    let SchemaBody::Matrix {
        dimensions: declared,
        ..
    } = schema.body()
    else {
        unreachable!("matrix shape resolution requires a matrix schema")
    };
    if declared.len() != dimensions.len() {
        return Err(MechError::new(
            ValueCellShapeMismatch {
                expected: declared
                    .iter()
                    .map(|_| u64::MAX)
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
                actual: dimensions.to_vec().into_boxed_slice(),
            },
            None,
        )
        .with_compiler_loc());
    }

    let mut values = seed
        .filter(|shape| shape.parameter_values().len() == schema.dimension_parameters().len())
        .map(|shape| shape.parameter_values().to_vec())
        .unwrap_or_else(|| vec![0; schema.dimension_parameters().len()]);
    for (index, parameter) in schema.dimension_parameters().iter().enumerate() {
        let lower = crate::evaluate_dimension(parameter.lower_bound(), &values[..index])
            .map_err(MechError::from)?;
        if values[index] < lower {
            values[index] = lower;
        }
    }
    for (expression, target) in declared.iter().zip(dimensions.iter().copied()) {
        assign_dimension_witness(expression, target, &mut values)?;
    }
    let shape = schema
        .instantiate_shape(values.into_boxed_slice())
        .map_err(MechError::from)?;
    let matches = declared
        .iter()
        .zip(dimensions)
        .all(|(expression, expected)| shape.resolve_dimension(expression) == Ok(*expected));
    if matches {
        Ok(shape)
    } else {
        Err(MechError::new(
            ValueCellShapeMismatch {
                expected: declared
                    .iter()
                    .map(|expression| shape.resolve_dimension(expression).unwrap_or(u64::MAX))
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
                actual: dimensions.to_vec().into_boxed_slice(),
            },
            None,
        )
        .with_compiler_loc())
    }
}

fn assign_dimension_witness(
    expression: &DimensionExpr,
    target: u64,
    values: &mut [u64],
) -> MResult<()> {
    match expression {
        DimensionExpr::Constant(expected) if *expected == target => Ok(()),
        DimensionExpr::Parameter(parameter) => {
            let Some(value) = values.get_mut(parameter.get() as usize) else {
                return Err(MechError::from(
                    crate::SemanticModelError::UnknownDimensionParameterV1 { id: *parameter },
                ));
            };
            *value = target;
            Ok(())
        }
        DimensionExpr::Add(operands) => {
            let Some((selected_index, selected)) = operands
                .iter()
                .enumerate()
                .find(|(_, operand)| dimension_has_parameter(operand))
            else {
                let actual =
                    crate::evaluate_dimension(expression, values).map_err(MechError::from)?;
                return (actual == target).then_some(()).ok_or_else(|| {
                    MechError::new(
                        ValueCellShapeMismatch {
                            expected: vec![actual].into_boxed_slice(),
                            actual: vec![target].into_boxed_slice(),
                        },
                        None,
                    )
                    .with_compiler_loc()
                });
            };
            if operands.is_empty() {
                return (target == 0).then_some(()).ok_or_else(|| {
                    MechError::from(crate::SemanticModelError::DimensionOverflowV1)
                });
            }
            let rest = operands
                .iter()
                .enumerate()
                .filter(|(index, _)| *index != selected_index)
                .try_fold(0_u64, |sum, (_, operand)| {
                    crate::evaluate_dimension(operand, values)
                        .map_err(MechError::from)
                        .and_then(|value| {
                            sum.checked_add(value).ok_or_else(|| {
                                MechError::from(crate::SemanticModelError::DimensionOverflowV1)
                            })
                        })
                })?;
            let selected_target = target
                .checked_sub(rest)
                .ok_or_else(|| MechError::from(crate::SemanticModelError::DimensionOverflowV1))?;
            assign_dimension_witness(selected, selected_target, values)
        }
        DimensionExpr::Multiply(operands) => {
            if target == 0 {
                let actual =
                    crate::evaluate_dimension(expression, values).map_err(MechError::from)?;
                if actual == 0 {
                    return Ok(());
                }
            }
            let adjustable = operands
                .iter()
                .enumerate()
                .filter(|(_, operand)| dimension_has_parameter(operand))
                .map(|(index, _)| index)
                .collect::<Vec<_>>();
            let Some(selected_index) = adjustable.first().copied() else {
                return dimension_witness_mismatch(expression, target, values);
            };
            for index in adjustable.into_iter().skip(1) {
                if crate::evaluate_dimension(&operands[index], values).map_err(MechError::from)?
                    == 0
                {
                    assign_dimension_witness(&operands[index], 1, values)?;
                }
            }
            let rest = operands
                .iter()
                .enumerate()
                .filter(|(index, _)| *index != selected_index)
                .try_fold(1_u64, |product, (_, operand)| {
                    crate::evaluate_dimension(operand, values)
                        .map_err(MechError::from)
                        .and_then(|value| {
                            product.checked_mul(value).ok_or_else(|| {
                                MechError::from(crate::SemanticModelError::DimensionOverflowV1)
                            })
                        })
                })?;
            if rest == 0 || target % rest != 0 {
                return dimension_witness_mismatch(expression, target, values);
            }
            assign_dimension_witness(&operands[selected_index], target / rest, values)
        }
        DimensionExpr::Min(operands) => {
            for operand in operands {
                let actual = crate::evaluate_dimension(operand, values).map_err(MechError::from)?;
                if actual < target {
                    assign_dimension_witness(operand, target, values)?;
                }
            }
            dimension_witness_mismatch(expression, target, values)
        }
        DimensionExpr::Max(operands) => {
            if operands.iter().any(|operand| {
                crate::evaluate_dimension(operand, values).is_ok_and(|actual| actual > target)
            }) {
                return dimension_witness_mismatch(expression, target, values);
            }
            let Some(selected) = operands
                .iter()
                .find(|operand| dimension_has_parameter(operand))
            else {
                return dimension_witness_mismatch(expression, target, values);
            };
            assign_dimension_witness(selected, target, values)
        }
        _ => dimension_witness_mismatch(expression, target, values),
    }
}

fn dimension_witness_mismatch(
    expression: &DimensionExpr,
    target: u64,
    values: &[u64],
) -> MResult<()> {
    let actual = crate::evaluate_dimension(expression, values).map_err(MechError::from)?;
    if actual == target {
        Ok(())
    } else {
        Err(MechError::new(
            ValueCellShapeMismatch {
                expected: vec![actual].into_boxed_slice(),
                actual: vec![target].into_boxed_slice(),
            },
            None,
        )
        .with_compiler_loc())
    }
}

fn dimension_has_parameter(expression: &DimensionExpr) -> bool {
    match expression {
        DimensionExpr::Parameter(_) => true,
        DimensionExpr::Add(operands)
        | DimensionExpr::Multiply(operands)
        | DimensionExpr::Min(operands)
        | DimensionExpr::Max(operands) => operands.iter().any(dimension_has_parameter),
        DimensionExpr::Hole | DimensionExpr::Constant(_) => false,
    }
}

fn table_cell_columns_draft(
    columns: &[(crate::SchemaField, Box<[ValueCell]>)],
    schemas: &SchemaTable,
) -> MResult<ValueDataDraft> {
    columns
        .iter()
        .map(|(field, values)| {
            Ok(crate::snapshot::TableColumnDraft {
                name: field.name.clone(),
                values: values
                    .iter()
                    .map(|cell| canonical_cell_draft_for_schema(cell, &field.schema, schemas))
                    .collect::<MResult<Vec<_>>>()?
                    .into_boxed_slice(),
            })
        })
        .collect::<MResult<Vec<_>>>()
        .map(|columns| ValueDataDraft::Table(columns.into_boxed_slice()))
}

fn record_cell_fields_draft(
    fields: &[(String, ValueCell)],
    schema_fields: &[crate::SchemaField],
    schemas: &SchemaTable,
) -> MResult<ValueDataDraft> {
    fields
        .iter()
        .zip(schema_fields)
        .map(|((name, cell), field)| {
            if *name != field.name {
                return Err(MechError::new(
                    ValueCellOutputConstructionUnsupported {
                        representation: FunctionValueRepresentation::Record,
                        reason: format!(
                            "record schema field {} does not match supplied field {name}",
                            field.name,
                        ),
                    },
                    None,
                )
                .with_compiler_loc());
            }
            Ok(crate::snapshot::NamedValueDraft {
                name: name.clone(),
                value: canonical_cell_draft_for_schema(cell, &field.schema, schemas)?,
            })
        })
        .collect::<MResult<Vec<_>>>()
        .map(|fields| ValueDataDraft::Record(fields.into_boxed_slice()))
}

fn canonical_cell_draft_for_schema(
    cell: &ValueCell,
    expected: &SchemaBody,
    schemas: &SchemaTable,
) -> MResult<ValueDataDraft> {
    let actual = cell.closed_schema_body()?;
    if matches!(expected, SchemaBody::Dynamic) {
        let snapshot = cell.snapshot()?;
        let concrete = match snapshot.data() {
            ValueData::Dynamic(value) => {
                let Some(value) = value.value() else {
                    return Ok(ValueDataDraft::Dynamic(None));
                };
                value
            }
            _ => &snapshot,
        };
        let schema = schemas.find_by_key(concrete.schema_key()).ok_or_else(|| {
            MechError::new(
                ValueCellOutputConstructionUnsupported {
                    representation: cell.representation(),
                    reason: format!(
                        "dynamic child schema {:?} is absent from the aggregate schema arena",
                        concrete.schema_key(),
                    ),
                },
                None,
            )
            .with_compiler_loc()
        })?;
        let shape = concrete.shape().clone();
        let concrete = concrete
            .rebind(schema, &shape, schemas)
            .map_err(snapshot_failure)?;
        let data = concrete.canonical_data_draft().map_err(snapshot_failure)?;
        return Ok(ValueDataDraft::Dynamic(Some(Box::new(ValueDraft {
            schema,
            shape_values: shape.parameter_values().to_vec().into_boxed_slice(),
            data,
        }))));
    }
    if actual != *expected {
        return Err(MechError::new(
            ValueCellOutputConstructionUnsupported {
                representation: cell.representation(),
                reason: format!("aggregate child expected schema {expected:?}, found {actual:?}",),
            },
            None,
        )
        .with_compiler_loc());
    }
    let snapshot = cell.snapshot()?;
    let schema = schemas.find_by_key(snapshot.schema_key()).ok_or_else(|| {
        MechError::new(
            ValueCellOutputConstructionUnsupported {
                representation: cell.representation(),
                reason: format!(
                    "aggregate child schema {:?} is absent from the aggregate schema arena",
                    snapshot.schema_key(),
                ),
            },
            None,
        )
        .with_compiler_loc()
    })?;
    let shape = snapshot.shape().clone();
    snapshot
        .rebind(schema, &shape, schemas)
        .map_err(snapshot_failure)?
        .canonical_data_draft()
        .map_err(snapshot_failure)
}

fn child_cells(schemas: Vec<SchemaBody>, values: Vec<ValueDataDraft>) -> MResult<Vec<ValueCell>> {
    schemas
        .into_iter()
        .zip(values)
        .map(|(schema, value)| ValueCell::from_schema_data(schema, value))
        .collect()
}

#[cfg(feature = "matrix")]
fn default_matrix_cell_in<T>(
    owner: &MemoryDomain,
    storage: FunctionMatrixStoragePattern,
    dimensions: (usize, usize),
    default: T,
) -> MResult<ValueCell>
where
    T: CanonicalMatrixElementBacking,
{
    let (rows, columns) = dimensions;
    #[allow(
        unused_macros,
        reason = "exact matrix constructors are feature-selected below"
    )]
    macro_rules! exact {
        ($matrix:expr, $expected_rows:expr, $expected_columns:expr) => {{
            if (rows, columns) != ($expected_rows, $expected_columns) {
                return Err(MechError::new(
                    ValueCellOutputConstructionUnsupported {
                        representation: FunctionValueRepresentation::Matrix {
                            element: crate::matrix_element_for_representation(T::REPRESENTATION),
                            storage,
                        },
                        reason: format!(
                            "declared storage is {}x{}, resolved output is {}x{}",
                            $expected_rows, $expected_columns, rows, columns,
                        ),
                    },
                    None,
                )
                .with_compiler_loc());
            }
            return ValueCell::from_inferred_ref_in(
                owner,
                Ref::new($matrix),
                Some((rows, columns)),
            );
        }};
    }
    match storage {
        #[cfg(feature = "matrix1")]
        FunctionMatrixStoragePattern::Exact(FunctionMatrixRepresentation::Matrix1) => {
            exact!(crate::Matrix1::from_element(default), 1, 1)
        }
        #[cfg(feature = "matrix2")]
        FunctionMatrixStoragePattern::Exact(FunctionMatrixRepresentation::Matrix2) => {
            exact!(crate::Matrix2::from_element(default), 2, 2)
        }
        #[cfg(feature = "matrix3")]
        FunctionMatrixStoragePattern::Exact(FunctionMatrixRepresentation::Matrix3) => {
            exact!(crate::Matrix3::from_element(default), 3, 3)
        }
        #[cfg(feature = "matrix4")]
        FunctionMatrixStoragePattern::Exact(FunctionMatrixRepresentation::Matrix4) => {
            exact!(crate::Matrix4::from_element(default), 4, 4)
        }
        #[cfg(feature = "matrix2x3")]
        FunctionMatrixStoragePattern::Exact(FunctionMatrixRepresentation::Matrix2x3) => {
            exact!(crate::Matrix2x3::from_element(default), 2, 3)
        }
        #[cfg(feature = "matrix3x2")]
        FunctionMatrixStoragePattern::Exact(FunctionMatrixRepresentation::Matrix3x2) => {
            exact!(crate::Matrix3x2::from_element(default), 3, 2)
        }
        #[cfg(feature = "row_vector2")]
        FunctionMatrixStoragePattern::Exact(FunctionMatrixRepresentation::RowVector2) => {
            exact!(crate::RowVector2::from_element(default), 1, 2)
        }
        #[cfg(feature = "row_vector3")]
        FunctionMatrixStoragePattern::Exact(FunctionMatrixRepresentation::RowVector3) => {
            exact!(crate::RowVector3::from_element(default), 1, 3)
        }
        #[cfg(feature = "row_vector4")]
        FunctionMatrixStoragePattern::Exact(FunctionMatrixRepresentation::RowVector4) => {
            exact!(crate::RowVector4::from_element(default), 1, 4)
        }
        #[cfg(feature = "vector2")]
        FunctionMatrixStoragePattern::Exact(FunctionMatrixRepresentation::Vector2) => {
            exact!(crate::Vector2::from_element(default), 2, 1)
        }
        #[cfg(feature = "vector3")]
        FunctionMatrixStoragePattern::Exact(FunctionMatrixRepresentation::Vector3) => {
            exact!(crate::Vector3::from_element(default), 3, 1)
        }
        #[cfg(feature = "vector4")]
        FunctionMatrixStoragePattern::Exact(FunctionMatrixRepresentation::Vector4) => {
            exact!(crate::Vector4::from_element(default), 4, 1)
        }
        #[cfg(feature = "row_vectord")]
        FunctionMatrixStoragePattern::Exact(FunctionMatrixRepresentation::RowVectorD) => {
            if rows != 1 {
                return Err(MechError::new(
                    ValueCellOutputConstructionUnsupported {
                        representation: FunctionValueRepresentation::Matrix {
                            element: crate::matrix_element_for_representation(T::REPRESENTATION),
                            storage,
                        },
                        reason: format!("row-vector output requires one row, found {rows}"),
                    },
                    None,
                )
                .with_compiler_loc());
            }
            ValueCell::from_inferred_ref_in(
                owner,
                Ref::new(crate::RowDVector::from_element(columns, default)),
                Some((rows, columns)),
            )
        }
        #[cfg(feature = "vectord")]
        FunctionMatrixStoragePattern::Exact(FunctionMatrixRepresentation::VectorD) => {
            if columns != 1 {
                return Err(MechError::new(
                    ValueCellOutputConstructionUnsupported {
                        representation: FunctionValueRepresentation::Matrix {
                            element: crate::matrix_element_for_representation(T::REPRESENTATION),
                            storage,
                        },
                        reason: format!(
                            "column-vector output requires one column, found {columns}"
                        ),
                    },
                    None,
                )
                .with_compiler_loc());
            }
            ValueCell::from_inferred_ref_in(
                owner,
                Ref::new(crate::DVector::from_element(rows, default)),
                Some((rows, columns)),
            )
        }
        #[cfg(feature = "matrixd")]
        FunctionMatrixStoragePattern::Exact(FunctionMatrixRepresentation::MatrixD) => {
            ValueCell::from_inferred_ref_in(
                owner,
                Ref::new(crate::DMatrix::from_element(rows, columns, default)),
                Some((rows, columns)),
            )
        }
        _ => Err(MechError::new(
            ValueCellOutputConstructionUnsupported {
                representation: FunctionValueRepresentation::Matrix {
                    element: crate::matrix_element_for_representation(T::REPRESENTATION),
                    storage,
                },
                reason: "output factory requires an exact matrix storage representation".into(),
            },
            None,
        )
        .with_compiler_loc()),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValueCellOutputConstructionUnsupported {
    pub representation: FunctionValueRepresentation,
    pub reason: String,
}

impl MechErrorKind for ValueCellOutputConstructionUnsupported {
    fn name(&self) -> &str {
        "ValueCellOutputConstructionUnsupported"
    }

    fn message(&self) -> String {
        format!(
            "cannot construct canonical output backing for {:?}: {}",
            self.representation, self.reason,
        )
    }
}

fn shape_change_allowed(
    schema: &crate::Schema,
    current: &ShapeInstance,
    next: &ShapeInstance,
) -> bool {
    let current = current.parameter_values();
    let next = next.parameter_values();
    current.len() == next.len()
        && schema
            .dimension_parameters()
            .iter()
            .zip(current.iter().zip(next))
            .all(|(parameter, (current, next))| {
                parameter.lifetime() == crate::DimensionLifetime::Turn || current == next
            })
}

#[cfg(all(feature = "matrix", feature = "matrixd"))]
fn dynamic_matrix_cell(
    owner: &MemoryDomain,
    elements: SequenceView<'_>,
    schema: SchemaId,
    shape: &ShapeInstance,
    schemas: Rc<SchemaTable>,
    preserve_dynamic_rank: bool,
) -> MResult<Option<ValueCell>> {
    // Dynamic vector specializations are optional; the rank-preservation
    // decision remains part of this shared constructor in matrix-only builds.
    let _ = preserve_dynamic_rank;
    let Some(entry) = schemas.entry(schema) else {
        return Ok(None);
    };
    let SchemaBody::Matrix { dimensions, .. } = entry.schema().body() else {
        return Ok(None);
    };
    let [rows, columns] = dimensions.as_ref() else {
        return Ok(None);
    };
    #[cfg(feature = "row_vectord")]
    let row_axis_is_invariant_one = matches!(rows, DimensionExpr::Constant(1));
    #[cfg(feature = "vectord")]
    let column_axis_is_invariant_one = matches!(columns, DimensionExpr::Constant(1));
    let (Ok(rows), Ok(columns)) = (
        shape
            .resolve_dimension(rows)
            .ok()
            .and_then(|value| usize::try_from(value).ok())
            .ok_or(()),
        shape
            .resolve_dimension(columns)
            .ok()
            .and_then(|value| usize::try_from(value).ok())
            .ok_or(()),
    ) else {
        return Ok(None);
    };
    macro_rules! matrix {
        ($values:expr) => {{
            #[cfg(feature = "row_vectord")]
            if !preserve_dynamic_rank && row_axis_is_invariant_one {
                let backing = crate::RowDVector::from_row_slice($values);
                return ValueCell::from_owned_ref_in(
                    owner,
                    Ref::new(backing),
                    schema,
                    shape.clone(),
                    schemas,
                )
                .map(Some);
            }
            #[cfg(feature = "vectord")]
            if !preserve_dynamic_rank && column_axis_is_invariant_one {
                let backing = crate::DVector::from_column_slice($values);
                return ValueCell::from_owned_ref_in(
                    owner,
                    Ref::new(backing),
                    schema,
                    shape.clone(),
                    schemas,
                )
                .map(Some);
            }
            let backing = crate::DMatrix::from_row_slice(rows, columns, $values);
            return ValueCell::from_owned_ref_in(
                owner,
                Ref::new(backing),
                schema,
                shape.clone(),
                schemas,
            )
            .map(Some);
        }};
    }
    match elements {
        #[cfg(feature = "u8")]
        SequenceView::U8(values) => matrix!(values),
        #[cfg(feature = "u16")]
        SequenceView::U16(values) => matrix!(values),
        #[cfg(feature = "u32")]
        SequenceView::U32(values) => matrix!(values),
        #[cfg(feature = "u64")]
        SequenceView::U64(values) => matrix!(values),
        #[cfg(feature = "u128")]
        SequenceView::U128(values) => matrix!(values),
        #[cfg(feature = "i8")]
        SequenceView::I8(values) => matrix!(values),
        #[cfg(feature = "i16")]
        SequenceView::I16(values) => matrix!(values),
        #[cfg(feature = "i32")]
        SequenceView::I32(values) => matrix!(values),
        #[cfg(feature = "i64")]
        SequenceView::I64(values) => matrix!(values),
        #[cfg(feature = "i128")]
        SequenceView::I128(values) => matrix!(values),
        #[cfg(feature = "f32")]
        SequenceView::F32(values) => {
            let values = values
                .iter()
                .map(|value| value.to_f32())
                .collect::<Vec<_>>();
            matrix!(&values)
        }
        #[cfg(feature = "f64")]
        SequenceView::F64(values) => {
            let values = values
                .iter()
                .map(|value| value.to_f64())
                .collect::<Vec<_>>();
            matrix!(&values)
        }
        #[cfg(feature = "complex")]
        SequenceView::Complex64(values) => {
            let values = values
                .iter()
                .map(|value| crate::C64::new(value.real().to_f64(), value.imaginary().to_f64()))
                .collect::<Vec<_>>();
            matrix!(&values)
        }
        #[cfg(feature = "rational")]
        SequenceView::Rational64(values) => {
            let Some(values) = values
                .iter()
                .map(|value| {
                    i64::try_from(value.denominator())
                        .ok()
                        .map(|denominator| crate::R64::new(value.numerator(), denominator))
                })
                .collect::<Option<Vec<_>>>()
            else {
                return Ok(None);
            };
            matrix!(&values)
        }
        #[cfg(feature = "bool")]
        SequenceView::Bool(values) => matrix!(values),
        #[cfg(feature = "string")]
        SequenceView::String(values) => {
            let values = values
                .iter()
                .map(|value| value.to_string())
                .collect::<Vec<_>>();
            matrix!(&values)
        }
        SequenceView::Index(values) => {
            let Some(values) = values
                .iter()
                .map(|value| usize::try_from(*value).ok())
                .collect::<Option<Vec<_>>>()
            else {
                return Ok(None);
            };
            matrix!(&values)
        }
        _ => Ok(None),
    }
}

impl fmt::Debug for ValueCell {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let storage = self.binding.storage();
        formatter
            .debug_struct("ValueCell")
            .field("schema_key", &self.binding.schema_key)
            .field("shape", &self.binding.shape())
            .field("representation", &self.representation())
            .field(
                "borrow_state",
                &storage.as_ref().map(|storage| storage.borrow_state()),
            )
            .finish()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CellAccess {
    Snapshot,
    Replace,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValueCellBorrowConflict {
    pub access: CellAccess,
}

impl MechErrorKind for ValueCellBorrowConflict {
    fn name(&self) -> &str {
        "ValueCellBorrowConflict"
    }

    fn message(&self) -> String {
        format!(
            "canonical value cell is already borrowed during {:?}",
            self.access
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValueCellSchemaMismatch {
    pub expected: SchemaKey,
    pub actual: SchemaKey,
}

impl MechErrorKind for ValueCellSchemaMismatch {
    fn name(&self) -> &str {
        "ValueCellSchemaMismatch"
    }

    fn message(&self) -> String {
        "replacement value has a different canonical schema".into()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValueCellShapeMismatch {
    pub expected: Box<[u64]>,
    pub actual: Box<[u64]>,
}

impl MechErrorKind for ValueCellShapeMismatch {
    fn name(&self) -> &str {
        "ValueCellShapeMismatch"
    }

    fn message(&self) -> String {
        format!(
            "replacement value has shape {:?}, expected {:?}",
            self.actual, self.expected
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValueCellBackingMismatch {
    pub expected: String,
    pub representation: FunctionValueRepresentation,
}

impl MechErrorKind for ValueCellBackingMismatch {
    fn name(&self) -> &str {
        "ValueCellBackingMismatch"
    }

    fn message(&self) -> String {
        format!(
            "canonical value cell representation {:?} does not use exact backing {}",
            self.representation, self.expected
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ValueCellSnapshotFailure {
    pub error: SnapshotValueError,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValueCellStorageContractViolation {
    pub schema: SchemaKey,
    pub reason: crate::StorageCompatibilityError,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExternalCellDescriptorMismatch {
    pub expected: String,
    pub actual: String,
}

impl MechErrorKind for ExternalCellDescriptorMismatch {
    fn name(&self) -> &str {
        "ExternalCellDescriptorMismatch"
    }

    fn message(&self) -> String {
        format!(
            "external cell descriptor expected {}, received {}",
            self.expected, self.actual
        )
    }
}

impl MechErrorKind for ValueCellStorageContractViolation {
    fn name(&self) -> &str {
        "ValueCellStorageContractViolation"
    }

    fn message(&self) -> String {
        format!(
            "value-cell storage does not satisfy schema {:?}: {}",
            self.schema, self.reason
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValueSchemaContextUnavailable;

impl MechErrorKind for ValueSchemaContextUnavailable {
    fn name(&self) -> &str {
        "ValueSchemaContextUnavailable"
    }

    fn message(&self) -> String {
        "canonical value does not retain the schema table required to create a value cell".into()
    }
}

impl MechErrorKind for ValueCellSnapshotFailure {
    fn name(&self) -> &str {
        "ValueCellSnapshotFailure"
    }

    fn message(&self) -> String {
        format!("canonical value cell snapshot failed: {:?}", self.error)
    }
}

fn validate_storage_compatibility(
    schema: &Schema,
    shape: &ShapeInstance,
    capabilities: &crate::StorageCapabilityDescriptor,
) -> MResult<()> {
    crate::check_schema_storage_compatibility(schema, shape, capabilities).map_err(|error| {
        match error {
            crate::SchemaStorageCompatibilityError::Semantic(error) => error.into(),
            crate::SchemaStorageCompatibilityError::Storage(reason) => MechError::new(
                ValueCellStorageContractViolation {
                    schema: schema.key(),
                    reason,
                },
                None,
            )
            .with_compiler_loc(),
        }
    })
}

pub(crate) fn borrow_conflict(access: CellAccess) -> MechError {
    MechError::new(ValueCellBorrowConflict { access }, None).with_compiler_loc()
}

fn snapshot_failure(error: SnapshotValueError) -> MechError {
    MechError::new(ValueCellSnapshotFailure { error }, None).with_compiler_loc()
}

pub(crate) fn finalize_draft(
    schema: SchemaId,
    shape: &ShapeInstance,
    schemas: &SchemaTable,
    data: ValueDataDraft,
) -> MResult<Value> {
    ValueDraft {
        schema,
        shape_values: shape.parameter_values().to_vec().into_boxed_slice(),
        data,
    }
    .finalize(&SnapshotValidationContext::new(schemas))
    .map_err(snapshot_failure)
}

#[doc(hidden)]
pub trait CanonicalMatrixElementBacking:
    canonical_matrix_element_sealed::Sealed
    + FunctionRuntimeType
    + Clone
    + fmt::Debug
    + PartialEq
    + 'static
{
    fn data_draft(&self) -> ValueDataDraft;
    fn from_data(data: &ValueData) -> Option<Self>;

    #[cfg(feature = "matrix")]
    fn from_sequence(values: SequenceView<'_>, index: usize) -> Option<Self>;
}

mod canonical_matrix_element_sealed {
    pub trait Sealed: Sized {
        fn initialize_dense(
            _frame: &mut crate::KernelMemoryFrame<'_>,
            object: crate::PlanObjectKey,
            _rows: usize,
            _columns: usize,
            _element: impl FnMut(usize, usize) -> Self,
        ) -> crate::MResult<()> {
            Err(super::managed_host_shape_error(
                object,
                "indirect matrix elements require their admitted builder",
            ))
        }
    }
}

fn initialize_fixed_dense<T: crate::ManagedElement>(
    frame: &mut crate::KernelMemoryFrame<'_>,
    object: crate::PlanObjectKey,
    rows: usize,
    columns: usize,
    mut element: impl FnMut(usize, usize) -> T,
) -> MResult<()> {
    frame.with_object_init_view::<T, _>(object, |output| {
        if output.rows() != rows || output.columns() != columns {
            return Err(managed_host_shape_error(
                object,
                "owned ingress shape differs from its planned view",
            ));
        }
        output.try_fill_column_major(|index| Ok(element(index % rows, index / rows)))
    })
}

macro_rules! fixed_element_ingress {
    () => {
        fn initialize_dense(
            frame: &mut crate::KernelMemoryFrame<'_>,
            object: crate::PlanObjectKey,
            rows: usize,
            columns: usize,
            element: impl FnMut(usize, usize) -> Self,
        ) -> MResult<()> {
            initialize_fixed_dense(frame, object, rows, columns, element)
        }
    };
}

macro_rules! fixed_scalar_ingress {
    () => {
        fn initialize_planned_fixed(
            &self,
            frame: &mut crate::KernelMemoryFrame<'_>,
            object: crate::PlanObjectKey,
        ) -> MResult<()> {
            initialize_fixed_dense(frame, object, 1, 1, |_, _| *self)
        }
    };
}

macro_rules! scalar_backing {
    ($type:ty, $feature:literal, $draft:ident, $data:ident, $sequence:ident, $legacy:ident, $matrix_legacy:ident) => {
        #[cfg(feature = $feature)]
        impl CanonicalMatrixElementBacking for $type {
            fn data_draft(&self) -> ValueDataDraft {
                ValueDataDraft::$draft(*self)
            }

            fn from_data(data: &ValueData) -> Option<Self> {
                match data {
                    ValueData::$data(value) => Some(*value),
                    _ => None,
                }
            }

            #[cfg(feature = "matrix")]
            fn from_sequence(values: SequenceView<'_>, index: usize) -> Option<Self> {
                match values {
                    SequenceView::$sequence(values) => values.get(index).copied(),
                    _ => None,
                }
            }
        }

        #[cfg(feature = $feature)]
        impl canonical_matrix_element_sealed::Sealed for $type {
            fixed_element_ingress!();
        }

        #[cfg(feature = $feature)]
        impl canonical_cell_sealed::Sealed for $type {
            fixed_scalar_ingress!();
            fn snapshot_bound(
                &self,
                schema: SchemaId,
                shape: &ShapeInstance,
                schemas: &SchemaTable,
            ) -> MResult<Value> {
                finalize_draft(schema, shape, schemas, self.data_draft())
            }

            fn replace_bound(&mut self, value: &Value) -> MResult<()> {
                let replacement = Self::from_data(value.data())
                    .ok_or_else(|| backing_mismatch::<Self>(Self::REPRESENTATION))?;
                *self = replacement;
                Ok(())
            }
        }
    };
}

scalar_backing!(u8, "u8", U8, U8, U8, U8, MatrixU8);
scalar_backing!(u16, "u16", U16, U16, U16, U16, MatrixU16);
scalar_backing!(u32, "u32", U32, U32, U32, U32, MatrixU32);
scalar_backing!(u64, "u64", U64, U64, U64, U64, MatrixU64);
scalar_backing!(u128, "u128", U128, U128, U128, U128, MatrixU128);
scalar_backing!(i8, "i8", I8, I8, I8, I8, MatrixI8);
scalar_backing!(i16, "i16", I16, I16, I16, I16, MatrixI16);
scalar_backing!(i32, "i32", I32, I32, I32, I32, MatrixI32);
scalar_backing!(i64, "i64", I64, I64, I64, I64, MatrixI64);
scalar_backing!(i128, "i128", I128, I128, I128, I128, MatrixI128);
scalar_backing!(bool, "bool", Bool, Bool, Bool, Bool, MatrixBool);

macro_rules! float_backing {
    ($type:ty, $feature:literal, $draft:ident, $data:ident, $sequence:ident, $bits:ty, $from:ident, $to:ident, $legacy:ident, $matrix_legacy:ident) => {
        #[cfg(feature = $feature)]
        impl CanonicalMatrixElementBacking for $type {
            fn data_draft(&self) -> ValueDataDraft {
                ValueDataDraft::$draft(<$bits>::$from(*self))
            }

            fn from_data(data: &ValueData) -> Option<Self> {
                match data {
                    ValueData::$data(value) => Some(value.$to()),
                    _ => None,
                }
            }

            #[cfg(feature = "matrix")]
            fn from_sequence(values: SequenceView<'_>, index: usize) -> Option<Self> {
                match values {
                    SequenceView::$sequence(values) => values.get(index).map(|value| value.$to()),
                    _ => None,
                }
            }
        }

        #[cfg(feature = $feature)]
        impl canonical_matrix_element_sealed::Sealed for $type {
            fixed_element_ingress!();
        }

        #[cfg(feature = $feature)]
        impl canonical_cell_sealed::Sealed for $type {
            fixed_scalar_ingress!();
            fn snapshot_bound(
                &self,
                schema: SchemaId,
                shape: &ShapeInstance,
                schemas: &SchemaTable,
            ) -> MResult<Value> {
                finalize_draft(schema, shape, schemas, self.data_draft())
            }

            fn replace_bound(&mut self, value: &Value) -> MResult<()> {
                let replacement = Self::from_data(value.data())
                    .ok_or_else(|| backing_mismatch::<Self>(Self::REPRESENTATION))?;
                *self = replacement;
                Ok(())
            }
        }
    };
}

float_backing!(
    f32,
    "f32",
    F32,
    F32,
    F32,
    crate::snapshot::F32Bits,
    from_f32,
    to_f32,
    F32,
    MatrixF32
);
float_backing!(
    f64,
    "f64",
    F64,
    F64,
    F64,
    crate::snapshot::F64Bits,
    from_f64,
    to_f64,
    F64,
    MatrixF64
);

impl CanonicalMatrixElementBacking for usize {
    fn data_draft(&self) -> ValueDataDraft {
        ValueDataDraft::Index(*self as u64)
    }

    fn from_data(data: &ValueData) -> Option<Self> {
        match data {
            ValueData::Index(value) => usize::try_from(*value).ok(),
            _ => None,
        }
    }

    #[cfg(feature = "matrix")]
    fn from_sequence(values: SequenceView<'_>, index: usize) -> Option<Self> {
        match values {
            SequenceView::Index(values) => {
                values.get(index).and_then(|value| (*value).try_into().ok())
            }
            _ => None,
        }
    }
}

impl canonical_matrix_element_sealed::Sealed for usize {
    fixed_element_ingress!();
}

impl canonical_cell_sealed::Sealed for usize {
    fixed_scalar_ingress!();
    fn snapshot_bound(
        &self,
        schema: SchemaId,
        shape: &ShapeInstance,
        schemas: &SchemaTable,
    ) -> MResult<Value> {
        finalize_draft(schema, shape, schemas, self.data_draft())
    }

    fn replace_bound(&mut self, value: &Value) -> MResult<()> {
        *self = Self::from_data(value.data())
            .ok_or_else(|| backing_mismatch::<Self>(Self::REPRESENTATION))?;
        Ok(())
    }
}

#[cfg(feature = "string")]
impl CanonicalMatrixElementBacking for String {
    fn data_draft(&self) -> ValueDataDraft {
        ValueDataDraft::String(self.clone())
    }

    fn from_data(data: &ValueData) -> Option<Self> {
        match data {
            ValueData::String(value) => Some(value.to_string()),
            _ => None,
        }
    }

    #[cfg(feature = "matrix")]
    fn from_sequence(values: SequenceView<'_>, index: usize) -> Option<Self> {
        match values {
            SequenceView::String(values) => values.get(index).map(|value| value.to_string()),
            _ => None,
        }
    }
}

#[cfg(feature = "string")]
impl canonical_matrix_element_sealed::Sealed for String {}

#[cfg(feature = "string")]
impl canonical_cell_sealed::Sealed for String {
    fn snapshot_bound(
        &self,
        schema: SchemaId,
        shape: &ShapeInstance,
        schemas: &SchemaTable,
    ) -> MResult<Value> {
        finalize_draft(schema, shape, schemas, self.data_draft())
    }

    fn replace_bound(&mut self, value: &Value) -> MResult<()> {
        *self = Self::from_data(value.data())
            .ok_or_else(|| backing_mismatch::<Self>(Self::REPRESENTATION))?;
        Ok(())
    }
}

#[cfg(feature = "complex")]
impl CanonicalMatrixElementBacking for crate::C64 {
    fn data_draft(&self) -> ValueDataDraft {
        ValueDataDraft::Complex64(crate::snapshot::Complex64Bits::new(
            crate::snapshot::F64Bits::from_f64(self.0.re),
            crate::snapshot::F64Bits::from_f64(self.0.im),
        ))
    }

    fn from_data(data: &ValueData) -> Option<Self> {
        match data {
            ValueData::Complex64(value) => Some(crate::C64::new(
                value.real().to_f64(),
                value.imaginary().to_f64(),
            )),
            _ => None,
        }
    }

    #[cfg(feature = "matrix")]
    fn from_sequence(values: SequenceView<'_>, index: usize) -> Option<Self> {
        match values {
            SequenceView::Complex64(values) => values
                .get(index)
                .map(|value| crate::C64::new(value.real().to_f64(), value.imaginary().to_f64())),
            _ => None,
        }
    }
}

#[cfg(feature = "complex")]
impl canonical_matrix_element_sealed::Sealed for crate::C64 {
    fixed_element_ingress!();
}

#[cfg(feature = "complex")]
impl canonical_cell_sealed::Sealed for crate::C64 {
    fixed_scalar_ingress!();
    fn snapshot_bound(
        &self,
        schema: SchemaId,
        shape: &ShapeInstance,
        schemas: &SchemaTable,
    ) -> MResult<Value> {
        finalize_draft(schema, shape, schemas, self.data_draft())
    }

    fn replace_bound(&mut self, value: &Value) -> MResult<()> {
        *self = Self::from_data(value.data())
            .ok_or_else(|| backing_mismatch::<Self>(Self::REPRESENTATION))?;
        Ok(())
    }
}

#[cfg(feature = "rational")]
impl CanonicalMatrixElementBacking for crate::R64 {
    fn data_draft(&self) -> ValueDataDraft {
        ValueDataDraft::Rational64 {
            numerator: *self.numer(),
            denominator: *self.denom() as u64,
        }
    }

    fn from_data(data: &ValueData) -> Option<Self> {
        match data {
            ValueData::Rational64(value) => i64::try_from(value.denominator())
                .ok()
                .map(|denominator| crate::R64::new(value.numerator(), denominator)),
            _ => None,
        }
    }

    #[cfg(feature = "matrix")]
    fn from_sequence(values: SequenceView<'_>, index: usize) -> Option<Self> {
        match values {
            SequenceView::Rational64(values) => values.get(index).and_then(|value| {
                i64::try_from(value.denominator())
                    .ok()
                    .map(|denominator| crate::R64::new(value.numerator(), denominator))
            }),
            _ => None,
        }
    }
}

#[cfg(feature = "rational")]
impl canonical_matrix_element_sealed::Sealed for crate::R64 {
    fixed_element_ingress!();
}

#[cfg(feature = "rational")]
impl canonical_cell_sealed::Sealed for crate::R64 {
    fixed_scalar_ingress!();
    fn snapshot_bound(
        &self,
        schema: SchemaId,
        shape: &ShapeInstance,
        schemas: &SchemaTable,
    ) -> MResult<Value> {
        finalize_draft(schema, shape, schemas, self.data_draft())
    }

    fn replace_bound(&mut self, value: &Value) -> MResult<()> {
        *self = Self::from_data(value.data())
            .ok_or_else(|| backing_mismatch::<Self>(Self::REPRESENTATION))?;
        Ok(())
    }
}

#[cfg(feature = "matrix")]
fn matrix_snapshot<T>(
    matrix: &impl CanonicalMatrix<T>,
    schema: SchemaId,
    shape: &ShapeInstance,
    schemas: &SchemaTable,
) -> MResult<Value>
where
    T: CanonicalMatrixElementBacking,
{
    validate_matrix_shape(matrix, schema, shape, schemas)?;
    let mut elements = Vec::with_capacity(matrix.rows().saturating_mul(matrix.cols()));
    for row in 0..matrix.rows() {
        for column in 0..matrix.cols() {
            elements.push(matrix.element(row, column).data_draft());
        }
    }
    finalize_draft(
        schema,
        shape,
        schemas,
        ValueDataDraft::Matrix(elements.into_boxed_slice()),
    )
}

#[cfg(feature = "matrix")]
fn matrix_replace<T>(matrix: &mut impl CanonicalMatrix<T>, value: &Value) -> MResult<()>
where
    T: CanonicalMatrixElementBacking,
{
    let ValueData::Matrix(replacement) = value.data() else {
        return Err(backing_mismatch::<T>(T::REPRESENTATION));
    };
    let schemas = value
        .schemas()
        .ok_or_else(|| MechError::new(ValueSchemaContextUnavailable, None).with_compiler_loc())?;
    let Some(SchemaBody::Matrix { dimensions, .. }) =
        schemas.get(value.schema()).map(|schema| schema.body())
    else {
        return Err(backing_mismatch::<T>(T::REPRESENTATION));
    };
    let [rows, columns] = dimensions.as_ref() else {
        return Err(backing_mismatch::<T>(T::REPRESENTATION));
    };
    let rows = usize::try_from(
        value
            .shape()
            .resolve_dimension(rows)
            .map_err(|error| snapshot_failure(error.into()))?,
    )
    .map_err(|_| backing_mismatch::<T>(T::REPRESENTATION))?;
    let columns = usize::try_from(
        value
            .shape()
            .resolve_dimension(columns)
            .map_err(|error| snapshot_failure(error.into()))?,
    )
    .map_err(|_| backing_mismatch::<T>(T::REPRESENTATION))?;
    let expected = rows.saturating_mul(columns);
    let values = replacement.elements();
    let mut elements = Vec::with_capacity(expected);
    for index in 0..expected {
        elements.push(
            T::from_sequence(values, index)
                .ok_or_else(|| backing_mismatch::<T>(T::REPRESENTATION))?,
        );
    }
    matrix
        .replace_elements(rows, columns, elements)
        .ok_or_else(|| {
            MechError::new(
                ValueCellShapeMismatch {
                    expected: vec![matrix.rows() as u64, matrix.cols() as u64].into_boxed_slice(),
                    actual: vec![rows as u64, columns as u64].into_boxed_slice(),
                },
                None,
            )
            .with_compiler_loc()
        })
}

#[cfg(feature = "matrix")]
trait CanonicalMatrix<T>: Sized {
    fn rows(&self) -> usize;
    fn cols(&self) -> usize;
    fn element(&self, row: usize, column: usize) -> &T;
    fn replace_elements(&mut self, rows: usize, columns: usize, elements: Vec<T>) -> Option<()>;
}

#[cfg(feature = "matrix")]
fn validate_matrix_shape<T>(
    matrix: &impl CanonicalMatrix<T>,
    schema: SchemaId,
    shape: &ShapeInstance,
    schemas: &SchemaTable,
) -> MResult<()> {
    let Some(SchemaBody::Matrix { dimensions, .. }) =
        schemas.get(schema).map(|schema| schema.body())
    else {
        return Err(backing_mismatch::<T>(FunctionValueRepresentation::AnyValue));
    };
    if dimensions.len() != 2
        || shape.resolve_dimension(&dimensions[0]).ok() != Some(matrix.rows() as u64)
        || shape.resolve_dimension(&dimensions[1]).ok() != Some(matrix.cols() as u64)
    {
        return Err(MechError::new(
            ValueCellShapeMismatch {
                expected: shape.parameter_values().to_vec().into_boxed_slice(),
                actual: vec![matrix.rows() as u64, matrix.cols() as u64].into_boxed_slice(),
            },
            None,
        )
        .with_compiler_loc());
    }
    Ok(())
}

#[cfg(feature = "matrix")]
macro_rules! matrix_backing {
    ($type:ident, $feature:literal) => {
        #[cfg(feature = $feature)]
        impl<T> CanonicalMatrix<T> for crate::$type<T>
        where
            T: CanonicalMatrixElementBacking,
        {
            fn rows(&self) -> usize {
                self.nrows()
            }

            fn cols(&self) -> usize {
                self.ncols()
            }

            fn element(&self, row: usize, column: usize) -> &T {
                &self[(row, column)]
            }

            fn replace_elements(
                &mut self,
                rows: usize,
                columns: usize,
                elements: Vec<T>,
            ) -> Option<()> {
                if rows != self.nrows() || columns != self.ncols() {
                    return None;
                }
                let mut replaced = self.clone();
                for (index, element) in elements.into_iter().enumerate() {
                    replaced[(index / columns, index % columns)] = element;
                }
                *self = replaced;
                Some(())
            }
        }

        #[cfg(feature = $feature)]
        impl<T> canonical_cell_sealed::Sealed for crate::$type<T>
        where
            T: CanonicalMatrixElementBacking,
            crate::$type<T>: FunctionRuntimeType,
        {
            fn initialize_planned_fixed(
                &self,
                frame: &mut crate::KernelMemoryFrame<'_>,
                object: crate::PlanObjectKey,
            ) -> MResult<()> {
                T::initialize_dense(frame, object, self.nrows(), self.ncols(), |row, column| {
                    self[(row, column)].clone()
                })
            }
            fn snapshot_bound(
                &self,
                schema: SchemaId,
                shape: &ShapeInstance,
                schemas: &SchemaTable,
            ) -> MResult<Value> {
                matrix_snapshot(self, schema, shape, schemas)
            }

            fn replace_bound(&mut self, value: &Value) -> MResult<()> {
                matrix_replace(self, value)
            }

            fn matrix_extents(&self) -> Option<(usize, usize)> {
                Some((self.nrows(), self.ncols()))
            }
        }
    };
}

#[cfg(feature = "matrix")]
matrix_backing!(Matrix1, "matrix1");
#[cfg(feature = "matrix")]
matrix_backing!(Matrix2, "matrix2");
#[cfg(feature = "matrix")]
matrix_backing!(Matrix3, "matrix3");
#[cfg(feature = "matrix")]
matrix_backing!(Matrix4, "matrix4");
#[cfg(feature = "matrix")]
matrix_backing!(Matrix2x3, "matrix2x3");
#[cfg(feature = "matrix")]
matrix_backing!(Matrix3x2, "matrix3x2");
#[cfg(feature = "matrix")]
matrix_backing!(RowVector2, "row_vector2");
#[cfg(feature = "matrix")]
matrix_backing!(RowVector3, "row_vector3");
#[cfg(feature = "matrix")]
matrix_backing!(RowVector4, "row_vector4");
#[cfg(feature = "matrix")]
matrix_backing!(Vector2, "vector2");
#[cfg(feature = "matrix")]
matrix_backing!(Vector3, "vector3");
#[cfg(feature = "matrix")]
matrix_backing!(Vector4, "vector4");
#[cfg(feature = "matrix")]
macro_rules! dynamic_matrix_storage {
    ($type:ident, $feature:literal) => {
        #[cfg(feature = $feature)]
        impl<T> canonical_cell_sealed::Sealed for crate::$type<T>
        where
            T: CanonicalMatrixElementBacking,
            crate::$type<T>: FunctionRuntimeType,
        {
            fn initialize_planned_fixed(
                &self,
                frame: &mut crate::KernelMemoryFrame<'_>,
                object: crate::PlanObjectKey,
            ) -> MResult<()> {
                T::initialize_dense(frame, object, self.nrows(), self.ncols(), |row, column| {
                    self[(row, column)].clone()
                })
            }
            fn snapshot_bound(
                &self,
                schema: SchemaId,
                shape: &ShapeInstance,
                schemas: &SchemaTable,
            ) -> MResult<Value> {
                matrix_snapshot(self, schema, shape, schemas)
            }

            fn replace_bound(&mut self, value: &Value) -> MResult<()> {
                matrix_replace(self, value)
            }

            fn matrix_extents(&self) -> Option<(usize, usize)> {
                Some((self.nrows(), self.ncols()))
            }
        }
    };
}

#[cfg(feature = "row_vectord")]
impl<T: CanonicalMatrixElementBacking> CanonicalMatrix<T> for crate::RowDVector<T> {
    fn rows(&self) -> usize {
        self.nrows()
    }

    fn cols(&self) -> usize {
        self.ncols()
    }

    fn element(&self, row: usize, column: usize) -> &T {
        &self[(row, column)]
    }

    fn replace_elements(&mut self, rows: usize, columns: usize, elements: Vec<T>) -> Option<()> {
        if rows != 1 {
            return None;
        }
        *self = crate::RowDVector::from_row_slice(&elements[..columns]);
        Some(())
    }
}

#[cfg(feature = "vectord")]
impl<T: CanonicalMatrixElementBacking> CanonicalMatrix<T> for crate::DVector<T> {
    fn rows(&self) -> usize {
        self.nrows()
    }

    fn cols(&self) -> usize {
        self.ncols()
    }

    fn element(&self, row: usize, column: usize) -> &T {
        &self[(row, column)]
    }

    fn replace_elements(&mut self, rows: usize, columns: usize, elements: Vec<T>) -> Option<()> {
        if columns != 1 || rows != elements.len() {
            return None;
        }
        *self = crate::DVector::from_vec(elements);
        Some(())
    }
}

#[cfg(feature = "matrixd")]
impl<T: CanonicalMatrixElementBacking> CanonicalMatrix<T> for crate::DMatrix<T> {
    fn rows(&self) -> usize {
        self.nrows()
    }

    fn cols(&self) -> usize {
        self.ncols()
    }

    fn element(&self, row: usize, column: usize) -> &T {
        &self[(row, column)]
    }

    fn replace_elements(&mut self, rows: usize, columns: usize, elements: Vec<T>) -> Option<()> {
        *self = crate::DMatrix::from_row_slice(rows, columns, &elements);
        Some(())
    }
}

#[cfg(feature = "matrix")]
dynamic_matrix_storage!(RowDVector, "row_vectord");
#[cfg(feature = "matrix")]
dynamic_matrix_storage!(DVector, "vectord");
#[cfg(feature = "matrix")]
dynamic_matrix_storage!(DMatrix, "matrixd");

impl canonical_cell_sealed::Sealed for Value {
    fn snapshot_bound(
        &self,
        schema: SchemaId,
        shape: &ShapeInstance,
        schemas: &SchemaTable,
    ) -> MResult<Value> {
        self.validate_against(schemas).map_err(snapshot_failure)?;
        if self.schema() != schema || self.shape() != shape {
            return Err(MechError::new(
                ValueCellShapeMismatch {
                    expected: shape.parameter_values().to_vec().into_boxed_slice(),
                    actual: self.shape().parameter_values().to_vec().into_boxed_slice(),
                },
                None,
            )
            .with_compiler_loc());
        }
        Ok(self.clone())
    }

    fn replace_bound(&mut self, value: &Value) -> MResult<()> {
        *self = value.clone();
        Ok(())
    }

    fn representation(schema: &SchemaBody) -> FunctionValueRepresentation {
        representation_for_schema(schema)
    }
}

fn representation_for_schema(schema: &SchemaBody) -> FunctionValueRepresentation {
    match schema {
        SchemaBody::Dynamic => FunctionValueRepresentation::AnyValue,
        SchemaBody::UnsignedInteger(IntegerWidth::W8) => FunctionValueRepresentation::U8,
        SchemaBody::UnsignedInteger(IntegerWidth::W16) => FunctionValueRepresentation::U16,
        SchemaBody::UnsignedInteger(IntegerWidth::W32) => FunctionValueRepresentation::U32,
        SchemaBody::UnsignedInteger(IntegerWidth::W64) => FunctionValueRepresentation::U64,
        SchemaBody::UnsignedInteger(IntegerWidth::W128) => FunctionValueRepresentation::U128,
        SchemaBody::SignedInteger(IntegerWidth::W8) => FunctionValueRepresentation::I8,
        SchemaBody::SignedInteger(IntegerWidth::W16) => FunctionValueRepresentation::I16,
        SchemaBody::SignedInteger(IntegerWidth::W32) => FunctionValueRepresentation::I32,
        SchemaBody::SignedInteger(IntegerWidth::W64) => FunctionValueRepresentation::I64,
        SchemaBody::SignedInteger(IntegerWidth::W128) => FunctionValueRepresentation::I128,
        SchemaBody::FloatingPoint(FloatWidth::W32) => FunctionValueRepresentation::F32,
        SchemaBody::FloatingPoint(FloatWidth::W64) => FunctionValueRepresentation::F64,
        SchemaBody::Complex(FloatWidth::W64) => FunctionValueRepresentation::C64,
        SchemaBody::Rational64 => FunctionValueRepresentation::R64,
        SchemaBody::String => FunctionValueRepresentation::String,
        SchemaBody::Bool => FunctionValueRepresentation::Bool,
        SchemaBody::Id => FunctionValueRepresentation::Id,
        SchemaBody::Index => FunctionValueRepresentation::Index,
        SchemaBody::Matrix { element, .. } => FunctionValueRepresentation::Matrix {
            element: matrix_element_for_schema(element),
            storage: FunctionMatrixStoragePattern::AnyStorage,
        },
        SchemaBody::Atom(_) => FunctionValueRepresentation::Atom,
        SchemaBody::Enum { .. } => FunctionValueRepresentation::Enum,
        SchemaBody::Record(_) => FunctionValueRepresentation::Record,
        SchemaBody::Map { .. } => FunctionValueRepresentation::Map,
        SchemaBody::Set { .. } => FunctionValueRepresentation::Set,
        SchemaBody::Table { .. } => FunctionValueRepresentation::Table,
        SchemaBody::Tuple(_) => FunctionValueRepresentation::Tuple,
        SchemaBody::ReifiedType => FunctionValueRepresentation::Kind,
        SchemaBody::Option(_) | SchemaBody::Complex(FloatWidth::W32) => {
            FunctionValueRepresentation::AnyValue
        }
    }
}

fn matrix_element_for_schema(schema: &SchemaBody) -> FunctionMatrixElement {
    match representation_for_schema(schema) {
        FunctionValueRepresentation::U8 => FunctionMatrixElement::U8,
        FunctionValueRepresentation::U16 => FunctionMatrixElement::U16,
        FunctionValueRepresentation::U32 => FunctionMatrixElement::U32,
        FunctionValueRepresentation::U64 => FunctionMatrixElement::U64,
        FunctionValueRepresentation::U128 => FunctionMatrixElement::U128,
        FunctionValueRepresentation::I8 => FunctionMatrixElement::I8,
        FunctionValueRepresentation::I16 => FunctionMatrixElement::I16,
        FunctionValueRepresentation::I32 => FunctionMatrixElement::I32,
        FunctionValueRepresentation::I64 => FunctionMatrixElement::I64,
        FunctionValueRepresentation::I128 => FunctionMatrixElement::I128,
        FunctionValueRepresentation::F32 => FunctionMatrixElement::F32,
        FunctionValueRepresentation::F64 => FunctionMatrixElement::F64,
        FunctionValueRepresentation::C64 => FunctionMatrixElement::C64,
        FunctionValueRepresentation::R64 => FunctionMatrixElement::R64,
        FunctionValueRepresentation::String => FunctionMatrixElement::String,
        FunctionValueRepresentation::Bool => FunctionMatrixElement::Bool,
        FunctionValueRepresentation::Index => FunctionMatrixElement::Index,
        _ => FunctionMatrixElement::Value,
    }
}

fn backing_mismatch<T>(representation: FunctionValueRepresentation) -> MechError {
    MechError::new(
        ValueCellBackingMismatch {
            expected: type_name::<T>().into(),
            representation,
        },
        None,
    )
    .with_compiler_loc()
}

fn schema_body_for_representation(
    representation: FunctionValueRepresentation,
    matrix_extents: Option<(usize, usize)>,
) -> Option<SchemaBody> {
    Some(match representation {
        FunctionValueRepresentation::U8 => SchemaBody::UnsignedInteger(IntegerWidth::W8),
        FunctionValueRepresentation::U16 => SchemaBody::UnsignedInteger(IntegerWidth::W16),
        FunctionValueRepresentation::U32 => SchemaBody::UnsignedInteger(IntegerWidth::W32),
        FunctionValueRepresentation::U64 => SchemaBody::UnsignedInteger(IntegerWidth::W64),
        FunctionValueRepresentation::U128 => SchemaBody::UnsignedInteger(IntegerWidth::W128),
        FunctionValueRepresentation::I8 => SchemaBody::SignedInteger(IntegerWidth::W8),
        FunctionValueRepresentation::I16 => SchemaBody::SignedInteger(IntegerWidth::W16),
        FunctionValueRepresentation::I32 => SchemaBody::SignedInteger(IntegerWidth::W32),
        FunctionValueRepresentation::I64 => SchemaBody::SignedInteger(IntegerWidth::W64),
        FunctionValueRepresentation::I128 => SchemaBody::SignedInteger(IntegerWidth::W128),
        FunctionValueRepresentation::F32 => SchemaBody::FloatingPoint(FloatWidth::W32),
        FunctionValueRepresentation::F64 => SchemaBody::FloatingPoint(FloatWidth::W64),
        FunctionValueRepresentation::C64 => SchemaBody::Complex(FloatWidth::W64),
        FunctionValueRepresentation::R64 => SchemaBody::Rational64,
        FunctionValueRepresentation::String => SchemaBody::String,
        FunctionValueRepresentation::Bool => SchemaBody::Bool,
        FunctionValueRepresentation::Id => SchemaBody::Id,
        FunctionValueRepresentation::Index => SchemaBody::Index,
        FunctionValueRepresentation::Matrix { element, .. } => {
            let (rows, columns) = matrix_extents?;
            SchemaBody::Matrix {
                element: Box::new(schema_body_for_matrix_element(element)?),
                dimensions: vec![
                    crate::DimensionExpr::Constant(rows as u64),
                    crate::DimensionExpr::Constant(columns as u64),
                ]
                .into_boxed_slice(),
            }
        }
        _ => return None,
    })
}

fn schema_body_for_matrix_element(element: FunctionMatrixElement) -> Option<SchemaBody> {
    Some(match element {
        FunctionMatrixElement::U8 => SchemaBody::UnsignedInteger(IntegerWidth::W8),
        FunctionMatrixElement::U16 => SchemaBody::UnsignedInteger(IntegerWidth::W16),
        FunctionMatrixElement::U32 => SchemaBody::UnsignedInteger(IntegerWidth::W32),
        FunctionMatrixElement::U64 => SchemaBody::UnsignedInteger(IntegerWidth::W64),
        FunctionMatrixElement::U128 => SchemaBody::UnsignedInteger(IntegerWidth::W128),
        FunctionMatrixElement::I8 => SchemaBody::SignedInteger(IntegerWidth::W8),
        FunctionMatrixElement::I16 => SchemaBody::SignedInteger(IntegerWidth::W16),
        FunctionMatrixElement::I32 => SchemaBody::SignedInteger(IntegerWidth::W32),
        FunctionMatrixElement::I64 => SchemaBody::SignedInteger(IntegerWidth::W64),
        FunctionMatrixElement::I128 => SchemaBody::SignedInteger(IntegerWidth::W128),
        FunctionMatrixElement::F32 => SchemaBody::FloatingPoint(FloatWidth::W32),
        FunctionMatrixElement::F64 => SchemaBody::FloatingPoint(FloatWidth::W64),
        FunctionMatrixElement::C64 => SchemaBody::Complex(FloatWidth::W64),
        FunctionMatrixElement::R64 => SchemaBody::Rational64,
        FunctionMatrixElement::String => SchemaBody::String,
        FunctionMatrixElement::Bool => SchemaBody::Bool,
        FunctionMatrixElement::Index => SchemaBody::Index,
        FunctionMatrixElement::Value => return None,
    })
}

pub(crate) fn close_schema_body(body: &SchemaBody, shape: &ShapeInstance) -> MResult<SchemaBody> {
    fn dimension(expression: &DimensionExpr, shape: &ShapeInstance) -> MResult<DimensionExpr> {
        shape
            .resolve_dimension(expression)
            .map(DimensionExpr::Constant)
            .map_err(|error| snapshot_failure(error.into()))
    }

    fn close_extent(value: &CardinalitySpec, shape: &ShapeInstance) -> MResult<CardinalitySpec> {
        Ok(match value {
            CardinalitySpec::Exact(value) => CardinalitySpec::Exact(dimension(value, shape)?),
            CardinalitySpec::Dynamic { upper_bound } => CardinalitySpec::Dynamic {
                upper_bound: upper_bound
                    .as_ref()
                    .map(|value| dimension(value, shape))
                    .transpose()?,
            },
        })
    }

    Ok(match body {
        SchemaBody::Dynamic => SchemaBody::Dynamic,
        SchemaBody::Bool => SchemaBody::Bool,
        SchemaBody::UnsignedInteger(width) => SchemaBody::UnsignedInteger(*width),
        SchemaBody::SignedInteger(width) => SchemaBody::SignedInteger(*width),
        SchemaBody::FloatingPoint(width) => SchemaBody::FloatingPoint(*width),
        SchemaBody::Complex(width) => SchemaBody::Complex(*width),
        SchemaBody::Rational64 => SchemaBody::Rational64,
        SchemaBody::String => SchemaBody::String,
        SchemaBody::Id => SchemaBody::Id,
        SchemaBody::Index => SchemaBody::Index,
        SchemaBody::Atom(key) => SchemaBody::Atom(*key),
        SchemaBody::Enum { key, variants } => SchemaBody::Enum {
            key: *key,
            variants: variants
                .iter()
                .map(|variant| {
                    Ok(crate::EnumVariantSchema {
                        name: variant.name.clone(),
                        payload: variant
                            .payload
                            .as_ref()
                            .map(|payload| close_schema_body(payload, shape))
                            .transpose()?,
                    })
                })
                .collect::<MResult<Vec<_>>>()?
                .into_boxed_slice(),
        },
        SchemaBody::Option(element) => {
            SchemaBody::Option(Box::new(close_schema_body(element, shape)?))
        }
        SchemaBody::Tuple(elements) => SchemaBody::Tuple(
            elements
                .iter()
                .map(|element| close_schema_body(element, shape))
                .collect::<MResult<Vec<_>>>()?
                .into_boxed_slice(),
        ),
        SchemaBody::Record(fields) => SchemaBody::Record(
            fields
                .iter()
                .map(|field| {
                    Ok(crate::SchemaField {
                        name: field.name.clone(),
                        schema: close_schema_body(&field.schema, shape)?,
                    })
                })
                .collect::<MResult<Vec<_>>>()?
                .into_boxed_slice(),
        ),
        SchemaBody::Matrix {
            element,
            dimensions,
        } => SchemaBody::Matrix {
            element: Box::new(close_schema_body(element, shape)?),
            dimensions: dimensions
                .iter()
                .map(|value| dimension(value, shape))
                .collect::<MResult<Vec<_>>>()?
                .into_boxed_slice(),
        },
        SchemaBody::Table { columns, rows } => SchemaBody::Table {
            columns: columns
                .iter()
                .map(|field| {
                    Ok(crate::SchemaField {
                        name: field.name.clone(),
                        schema: close_schema_body(&field.schema, shape)?,
                    })
                })
                .collect::<MResult<Vec<_>>>()?
                .into_boxed_slice(),
            rows: close_extent(rows, shape)?,
        },
        SchemaBody::Set {
            element,
            cardinality: value,
        } => SchemaBody::Set {
            element: Box::new(close_schema_body(element, shape)?),
            cardinality: close_extent(value, shape)?,
        },
        SchemaBody::Map {
            key,
            value,
            cardinality,
        } => SchemaBody::Map {
            key: Box::new(close_schema_body(key, shape)?),
            value: Box::new(close_schema_body(value, shape)?),
            cardinality: close_extent(cardinality, shape)?,
        },
        SchemaBody::ReifiedType => SchemaBody::ReifiedType,
    })
}

fn rebind_value(value: Value, schemas: &SchemaTable) -> MResult<Value> {
    let schema = schemas.find_by_key(value.schema_key()).ok_or_else(|| {
        snapshot_failure(SnapshotValueError::SnapshotSchemaTableMismatch {
            schema: value.schema(),
            expected: value.schema_key(),
            actual: schemas.entry(value.schema()).map(|entry| entry.key()),
        })
    })?;
    value
        .rebind(schema, value.shape(), schemas)
        .map_err(snapshot_failure)
}

fn dynamic_matrix_schema(
    element: SchemaBody,
    dimensions: Box<[u64]>,
) -> MResult<(SchemaId, ShapeInstance, Rc<SchemaTable>)> {
    let declarations = dimensions
        .iter()
        .enumerate()
        .map(|(index, _)| crate::DimensionParameterDeclaration {
            id: crate::DimensionParameterId::new(index as u32),
            origin: crate::DimensionParameterOrigin::Inferred,
            lifetime: crate::DimensionLifetime::Turn,
            lower_bound: DimensionExpr::Constant(0),
            upper_bound: None,
        })
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let body = SchemaBody::Matrix {
        element: Box::new(element),
        dimensions: (0..dimensions.len())
            .map(|index| DimensionExpr::Parameter(crate::DimensionParameterId::new(index as u32)))
            .collect::<Vec<_>>()
            .into_boxed_slice(),
    };
    let schema = crate::SchemaDraft {
        dimension_parameters: declarations,
        body,
    }
    .finalize()
    .map_err(|error| snapshot_failure(error.into()))?;
    let shape = schema
        .instantiate_shape(dimensions)
        .map_err(|error| snapshot_failure(error.into()))?;
    let mut builder = crate::SchemaTableBuilder::new();
    let handle = builder
        .insert(schema)
        .map_err(|error| snapshot_failure(error.into()))?;
    let build = builder
        .finish()
        .map_err(|error| snapshot_failure(error.into()))?;
    let schema = build
        .resolve(handle)
        .map_err(|error| snapshot_failure(error.into()))?;
    Ok((schema, shape, Rc::new(build.table)))
}

fn dynamic_row_vector_schema(
    element: SchemaBody,
    rows: u64,
    columns: u64,
) -> MResult<(SchemaId, ShapeInstance, Rc<SchemaTable>)> {
    if rows != 1 {
        return Err(MechError::new(
            ValueCellShapeMismatch {
                expected: vec![1, columns].into_boxed_slice(),
                actual: vec![rows, columns].into_boxed_slice(),
            },
            None,
        )
        .with_compiler_loc());
    }
    dynamic_vector_schema(
        element,
        vec![
            DimensionExpr::Constant(1),
            DimensionExpr::Parameter(crate::DimensionParameterId::new(0)),
        ]
        .into_boxed_slice(),
        columns,
    )
}

fn dynamic_column_vector_schema(
    element: SchemaBody,
    rows: u64,
    columns: u64,
) -> MResult<(SchemaId, ShapeInstance, Rc<SchemaTable>)> {
    if columns != 1 {
        return Err(MechError::new(
            ValueCellShapeMismatch {
                expected: vec![rows, 1].into_boxed_slice(),
                actual: vec![rows, columns].into_boxed_slice(),
            },
            None,
        )
        .with_compiler_loc());
    }
    dynamic_vector_schema(
        element,
        vec![
            DimensionExpr::Parameter(crate::DimensionParameterId::new(0)),
            DimensionExpr::Constant(1),
        ]
        .into_boxed_slice(),
        rows,
    )
}

fn dynamic_vector_schema(
    element: SchemaBody,
    dimensions: Box<[DimensionExpr]>,
    current: u64,
) -> MResult<(SchemaId, ShapeInstance, Rc<SchemaTable>)> {
    let schema = crate::SchemaDraft {
        dimension_parameters: vec![crate::DimensionParameterDeclaration {
            id: crate::DimensionParameterId::new(0),
            origin: crate::DimensionParameterOrigin::Inferred,
            lifetime: crate::DimensionLifetime::Turn,
            lower_bound: DimensionExpr::Constant(0),
            upper_bound: None,
        }]
        .into_boxed_slice(),
        body: SchemaBody::Matrix {
            element: Box::new(element),
            dimensions,
        },
    }
    .finalize()
    .map_err(|error| snapshot_failure(error.into()))?;
    let shape = schema
        .instantiate_shape(vec![current].into_boxed_slice())
        .map_err(|error| snapshot_failure(error.into()))?;
    let mut builder = SchemaTableBuilder::new();
    let handle = builder.insert(schema).map_err(MechError::from)?;
    let build = builder.finish().map_err(MechError::from)?;
    let schema = build.resolve(handle).map_err(MechError::from)?;
    Ok((schema, shape, Rc::new(build.table)))
}

fn standalone_schema(body: SchemaBody) -> MResult<(SchemaId, ShapeInstance, Rc<SchemaTable>)> {
    let schema = crate::SchemaDraft {
        dimension_parameters: Vec::new().into_boxed_slice(),
        body,
    }
    .finalize()
    .map_err(|error| snapshot_failure(error.into()))?;
    let shape = schema
        .instantiate_shape(Vec::new().into_boxed_slice())
        .map_err(|error| snapshot_failure(error.into()))?;
    let mut builder = crate::SchemaTableBuilder::new();
    let handle = builder
        .insert(schema)
        .map_err(|error| snapshot_failure(error.into()))?;
    let build = builder
        .finish()
        .map_err(|error| snapshot_failure(error.into()))?;
    let schema = build
        .resolve(handle)
        .map_err(|error| snapshot_failure(error.into()))?;
    Ok((schema, shape, Rc::new(build.table)))
}

#[cfg(all(test, any(feature = "f64", feature = "u8", feature = "string")))]
mod tests {
    use super::*;
    #[cfg(any(feature = "f64", feature = "u8"))]
    use crate::DimensionExpr;
    #[cfg(all(feature = "f64", feature = "matrix"))]
    use crate::{DimensionLifetime, DimensionParameterId, DimensionParameterOrigin};
    use crate::{DimensionParameterDeclaration, SchemaDraft, SchemaTableBuilder};

    struct TestSchema {
        id: SchemaId,
        shape: ShapeInstance,
        schemas: Rc<SchemaTable>,
    }

    fn test_schema(
        body: SchemaBody,
        dimensions: Box<[DimensionParameterDeclaration]>,
        shape_values: &[u64],
    ) -> TestSchema {
        let schema = SchemaDraft {
            dimension_parameters: dimensions,
            body,
        }
        .finalize()
        .unwrap();
        let shape = schema
            .instantiate_shape(shape_values.to_vec().into_boxed_slice())
            .unwrap();
        let mut builder = SchemaTableBuilder::new();
        let handle = builder.insert(schema).unwrap();
        let build = builder.finish().unwrap();
        let id = build.resolve(handle).unwrap();
        TestSchema {
            id,
            shape,
            schemas: Rc::new(build.table),
        }
    }

    #[cfg(feature = "f64")]
    fn f64_schema() -> TestSchema {
        test_schema(
            SchemaBody::FloatingPoint(FloatWidth::W64),
            Vec::new().into_boxed_slice(),
            &[],
        )
    }

    #[cfg(feature = "f64")]
    fn f64_value(schema: &TestSchema, value: f64) -> Value {
        finalize_draft(
            schema.id,
            &schema.shape,
            &schema.schemas,
            ValueDataDraft::F64(crate::snapshot::F64Bits::from_f64(value)),
        )
        .unwrap()
    }

    #[cfg(all(feature = "f64", feature = "matrix"))]
    fn matrix_schema(rows: u64, columns: u64) -> TestSchema {
        let dimensions = [
            DimensionParameterDeclaration {
                id: DimensionParameterId::new(0),
                origin: DimensionParameterOrigin::Inferred,
                lifetime: DimensionLifetime::Activation,
                lower_bound: DimensionExpr::Constant(0),
                upper_bound: None,
            },
            DimensionParameterDeclaration {
                id: DimensionParameterId::new(1),
                origin: DimensionParameterOrigin::Inferred,
                lifetime: DimensionLifetime::Activation,
                lower_bound: DimensionExpr::Constant(0),
                upper_bound: None,
            },
        ];
        test_schema(
            SchemaBody::Matrix {
                element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
                dimensions: vec![
                    DimensionExpr::Parameter(DimensionParameterId::new(0)),
                    DimensionExpr::Parameter(DimensionParameterId::new(1)),
                ]
                .into_boxed_slice(),
            },
            dimensions.into(),
            &[rows, columns],
        )
    }

    #[cfg(feature = "f64")]
    #[test]
    fn cloned_and_separate_cells_have_explicit_identity_and_structural_value_equality() {
        let schema = f64_schema();
        let first = ValueCell::from_ref(
            Ref::new(7.0_f64),
            schema.id,
            schema.shape.clone(),
            schema.schemas.clone(),
        )
        .unwrap();
        let clone = first.clone();
        let separate =
            ValueCell::from_ref(Ref::new(7.0_f64), schema.id, schema.shape, schema.schemas)
                .unwrap();
        let detached = first.detached_clone().unwrap();

        assert!(first.same_logical_cell(&clone));
        assert!(first.same_storage(&clone));
        assert!(first.same_cell(&clone));
        assert!(!first.same_logical_cell(&separate));
        assert!(!first.same_storage(&separate));
        assert!(!first.same_cell(&separate));
        assert!(first.snapshot_eq(&separate).unwrap());
        assert!(!first.same_logical_cell(&detached));
        assert!(!first.same_storage(&detached));
        assert!(first.snapshot_eq(&detached).unwrap());

        let same_identity_detached_storage =
            ValueCell::test_with_identity_and_payload(&first, &detached).unwrap();
        assert!(first.same_logical_cell(&same_identity_detached_storage));
        assert!(!first.same_storage(&same_identity_detached_storage));

        let different_identity_shared_storage =
            ValueCell::test_with_identity_and_payload(&detached, &first).unwrap();
        assert!(!first.same_logical_cell(&different_identity_shared_storage));
        assert!(first.same_storage(&different_identity_shared_storage));

        for other in [
            &clone,
            &separate,
            &detached,
            &same_identity_detached_storage,
            &different_identity_shared_storage,
        ] {
            assert_eq!(first.same_cell(other), first.same_storage(other));
        }
    }

    fn assert_replace_borrow_conflict_preserves_value<T>(reference: Ref<T>, schema: TestSchema)
    where
        T: CanonicalCellBacking,
    {
        let cell = ValueCell::from_ref(
            reference.clone(),
            schema.id,
            schema.shape.clone(),
            schema.schemas.clone(),
        )
        .unwrap();
        let before = cell.detached_clone().unwrap();
        let replacement = before.snapshot().unwrap();
        let held = reference.borrow_mut();
        assert!(cell.replace(&replacement).is_err());
        drop(held);
        assert!(cell.snapshot_eq(&before).unwrap());
        assert!(
            cell.storage_capabilities()
                .publication
                .preserves_previous_on_failure
        );
    }

    #[test]
    fn declared_atomic_publication_preserves_representative_backings_on_borrow_conflict() {
        #[cfg(feature = "f64")]
        {
            let scalar_schema = f64_schema();
            assert_replace_borrow_conflict_preserves_value(Ref::new(1.25_f64), scalar_schema);

            let canonical_schema = f64_schema();
            let canonical = f64_value(&canonical_schema, 1.25);
            assert_replace_borrow_conflict_preserves_value(Ref::new(canonical), canonical_schema);
        }

        #[cfg(feature = "string")]
        assert_replace_borrow_conflict_preserves_value(
            Ref::new("before".to_owned()),
            test_schema(SchemaBody::String, Box::new([]), &[]),
        );

        #[cfg(all(feature = "f64", feature = "matrixd"))]
        assert_replace_borrow_conflict_preserves_value(
            Ref::new(crate::DMatrix::<f64>::zeros(2, 3)),
            matrix_schema(2, 3),
        );

        #[cfg(all(feature = "f64", feature = "matrix2"))]
        assert_replace_borrow_conflict_preserves_value(
            Ref::new(crate::Matrix2::<f64>::zeros()),
            matrix_schema(2, 2),
        );
    }

    #[cfg(feature = "f64")]
    #[test]
    fn exact_scalar_snapshot_and_replacement_preserve_the_original_ref() {
        let schema = f64_schema();
        let reference = Ref::new(1.25_f64);
        let alias = reference.clone();
        let cell = ValueCell::from_ref(
            reference,
            schema.id,
            schema.shape.clone(),
            schema.schemas.clone(),
        )
        .unwrap();

        assert!(matches!(
            cell.snapshot().unwrap().data(),
            ValueData::F64(value) if value.to_f64() == 1.25
        ));
        cell.replace(&f64_value(&schema, 9.5)).unwrap();
        assert_eq!(*alias.borrow(), 9.5);
        assert!(cell.try_ref::<f64>().unwrap().same_handle(&alias));
    }

    #[cfg(all(feature = "f64", feature = "matrix2", feature = "matrixd"))]
    #[test]
    fn fixed_and_dynamic_matrix_snapshots_are_row_major_and_keep_exact_handles() {
        let fixed_schema = matrix_schema(2, 2);
        let fixed = Ref::new(crate::Matrix2::new(1.0, 2.0, 3.0, 4.0));
        let fixed_cell = ValueCell::from_ref(
            fixed.clone(),
            fixed_schema.id,
            fixed_schema.shape,
            fixed_schema.schemas,
        )
        .unwrap();
        let ValueData::Matrix(fixed_value) = fixed_cell.snapshot().unwrap().data().clone() else {
            panic!("fixed matrix snapshot")
        };
        assert!(matches!(
            fixed_value.elements(),
            SequenceView::F64(values)
                if values.iter().map(|value| value.to_f64()).collect::<Vec<_>>()
                    == vec![1.0, 2.0, 3.0, 4.0]
        ));
        assert!(
            fixed_cell
                .try_ref::<crate::Matrix2<f64>>()
                .unwrap()
                .same_handle(&fixed)
        );

        let dynamic_schema = matrix_schema(2, 3);
        let dynamic = Ref::new(crate::DMatrix::from_row_slice(
            2,
            3,
            &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
        ));
        let dynamic_cell = ValueCell::from_ref(
            dynamic.clone(),
            dynamic_schema.id,
            dynamic_schema.shape,
            dynamic_schema.schemas,
        )
        .unwrap();
        assert!(matches!(
            dynamic_cell.snapshot().unwrap().data(),
            ValueData::Matrix(matrix)
                if matches!(matrix.elements(), SequenceView::F64(values) if values.len() == 6)
        ));
        assert!(
            dynamic_cell
                .try_ref::<crate::DMatrix<f64>>()
                .unwrap()
                .same_handle(&dynamic)
        );
    }

    #[cfg(all(feature = "complex", feature = "rational", feature = "matrixd"))]
    #[test]
    fn canonical_complex_and_rational_matrices_retain_typed_managed_storage() {
        let complex = ValueCell::dynamic_matrix(
            SchemaBody::Complex(crate::FloatWidth::W64),
            vec![2, 2].into_boxed_slice(),
            (1..=4)
                .map(|value| {
                    ValueDataDraft::Complex64(crate::snapshot::Complex64Bits::new(
                        crate::snapshot::F64Bits::from_f64(f64::from(value)),
                        crate::snapshot::F64Bits::from_f64(0.0),
                    ))
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        )
        .unwrap();
        assert_eq!(
            complex.representation(),
            <crate::DMatrix<crate::C64> as FunctionRuntimeType>::REPRESENTATION,
        );
        assert!(matches!(
            complex.snapshot().unwrap().data(),
            ValueData::Matrix(matrix)
                if matches!(matrix.elements(), SequenceView::Complex64(values) if values.len() == 4)
        ));

        let rational = ValueCell::dynamic_matrix(
            SchemaBody::Rational64,
            vec![2, 2].into_boxed_slice(),
            (1..=4)
                .map(|numerator| ValueDataDraft::Rational64 {
                    numerator,
                    denominator: 1,
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        )
        .unwrap();
        assert_eq!(
            rational.representation(),
            <crate::DMatrix<crate::R64> as FunctionRuntimeType>::REPRESENTATION,
        );
        assert!(matches!(
            rational.snapshot().unwrap().data(),
            ValueData::Matrix(matrix)
                if matches!(matrix.elements(), SequenceView::Rational64(values) if values.len() == 4)
        ));
    }

    #[cfg(all(feature = "f64", feature = "matrix2", feature = "matrixd"))]
    #[test]
    fn declared_output_representations_construct_exact_managed_backings() {
        let scalar =
            ValueCell::test_backing_for_representation(FunctionValueRepresentation::F64, None)
                .unwrap();
        assert_eq!(scalar.representation(), FunctionValueRepresentation::F64);
        assert!(matches!(
            scalar.snapshot().unwrap().data(),
            ValueData::F64(value) if value.to_f64() == 0.0
        ));

        let fixed = ValueCell::test_backing_for_representation(
            <crate::Matrix2<f64> as FunctionRuntimeType>::REPRESENTATION,
            Some((2, 2)),
        )
        .unwrap();
        assert_eq!(
            fixed.representation(),
            <crate::Matrix2<f64> as FunctionRuntimeType>::REPRESENTATION,
        );
        assert_eq!(fixed.current_top_level_extents().unwrap().as_ref(), &[2, 2]);
        assert!(matches!(
            fixed.snapshot().unwrap().data(),
            ValueData::Matrix(matrix)
                if matches!(matrix.elements(), SequenceView::F64(values)
                    if values.len() == 4 && values.iter().all(|value| value.to_f64() == 0.0))
        ));

        let dynamic = ValueCell::test_backing_for_representation(
            <crate::DMatrix<f64> as FunctionRuntimeType>::REPRESENTATION,
            Some((2, 3)),
        )
        .unwrap();
        assert_eq!(
            dynamic.representation(),
            <crate::DMatrix<f64> as FunctionRuntimeType>::REPRESENTATION,
        );
        assert_eq!(
            dynamic.current_top_level_extents().unwrap().as_ref(),
            &[2, 3]
        );
        assert!(matches!(
            dynamic.snapshot().unwrap().data(),
            ValueData::Matrix(matrix)
                if matches!(matrix.elements(), SequenceView::F64(values)
                    if values.len() == 6 && values.iter().all(|value| value.to_f64() == 0.0))
        ));

        let error = ValueCell::test_backing_for_representation(
            <crate::Matrix2<f64> as FunctionRuntimeType>::REPRESENTATION,
            Some((2, 3)),
        )
        .unwrap_err();
        assert_eq!(error.kind_name(), "ValueCellOutputConstructionUnsupported");
    }

    #[cfg(all(
        feature = "f64",
        feature = "functions",
        feature = "matrix",
        feature = "matrixd",
        feature = "string"
    ))]
    #[test]
    fn descriptor_outputs_keep_the_requested_session_across_storage_topologies() {
        let owner = MemoryDomain::new().unwrap();
        let scalar_schema = f64_schema();
        let scalar_descriptor = crate::ResolvedValueDescriptor::from_schema(
            scalar_schema.schemas.get(scalar_schema.id).unwrap().clone(),
            scalar_schema.shape,
        )
        .unwrap();
        let scalar = ValueCell::allocate_for_descriptor_in(
            &owner,
            &scalar_descriptor,
            FunctionValueRepresentation::F64,
        )
        .unwrap();

        let string_schema = test_schema(SchemaBody::String, Box::new([]), &[]);
        let string_descriptor = crate::ResolvedValueDescriptor::from_schema(
            string_schema.schemas.get(string_schema.id).unwrap().clone(),
            string_schema.shape,
        )
        .unwrap();
        let string = ValueCell::allocate_for_descriptor_in(
            &owner,
            &string_descriptor,
            FunctionValueRepresentation::String,
        )
        .unwrap();

        let matrix_schema = matrix_schema(2, 3);
        let matrix_descriptor = crate::ResolvedValueDescriptor::from_schema(
            matrix_schema.schemas.get(matrix_schema.id).unwrap().clone(),
            matrix_schema.shape,
        )
        .unwrap();
        let matrix = ValueCell::allocate_for_descriptor_in(
            &owner,
            &matrix_descriptor,
            <crate::DMatrix<f64> as FunctionRuntimeType>::REPRESENTATION,
        )
        .unwrap();

        for output in [&scalar, &string, &matrix] {
            assert_eq!(output.memory_domain().unwrap().id(), owner.id());
        }
        let string_value = string.snapshot().unwrap();
        assert!(matches!(string_value.data(), ValueData::String(value) if value.is_empty()));
        assert_eq!(
            matrix.current_top_level_extents().unwrap().as_ref(),
            &[2, 3]
        );
    }

    #[cfg(all(feature = "f64", feature = "functions", feature = "vectord"))]
    #[test]
    fn exact_matrix_values_infer_their_canonical_extents() {
        let cell = ValueCell::from_exact(crate::DVector::from_vec(vec![1.0_f64, 2.0, 3.0]))
            .expect("a dynamic vector provides its own matrix extents");

        assert_eq!(
            cell.representation(),
            <crate::DVector<f64> as FunctionRuntimeType>::REPRESENTATION,
        );
        assert!(matches!(
            cell.snapshot().unwrap().data(),
            ValueData::Matrix(matrix)
                if matches!(matrix.elements(), SequenceView::F64(values)
                    if values.iter().map(|value| value.to_f64()).collect::<Vec<_>>() == [1.0, 2.0, 3.0])
        ));
        assert_eq!(cell.current_top_level_extents().unwrap().as_ref(), &[3, 1]);
    }

    #[cfg(feature = "f64")]
    #[test]
    fn canonical_value_cells_snapshot_and_replace_without_changing_identity() {
        let schema = f64_schema();
        let original = f64_value(&schema, 2.0);
        let cell = ValueCell::from_value(original.clone(), schema.schemas.clone()).unwrap();
        let alias = cell.clone();
        let expected = ValueCell::from_value(original, schema.schemas.clone()).unwrap();

        assert!(cell.snapshot_eq(&expected).unwrap());
        cell.replace(&f64_value(&schema, 3.0)).unwrap();
        assert!(cell.same_cell(&alias));
        assert!(matches!(
            alias.snapshot().unwrap().data(),
            ValueData::F64(value) if value.to_f64() == 3.0
        ));
    }

    #[cfg(feature = "f64")]
    #[test]
    fn value_cell_key_equality_uses_canonical_float_keys() {
        let left = ValueCell::from_schema_data(
            SchemaBody::FloatingPoint(FloatWidth::W64),
            ValueDataDraft::F64(crate::snapshot::F64Bits::from_bits(0x7ff0_0000_0000_0001)),
        )
        .unwrap();
        let right = ValueCell::from_schema_data(
            SchemaBody::FloatingPoint(FloatWidth::W64),
            ValueDataDraft::F64(crate::snapshot::F64Bits::from_bits(0xfff8_0000_0000_0042)),
        )
        .unwrap();
        assert!(!left.snapshot_eq(&right).unwrap());
        assert!(left.key_eq(&right).unwrap());

        let negative_zero = ValueCell::from_schema_data(
            SchemaBody::FloatingPoint(FloatWidth::W64),
            ValueDataDraft::F64(crate::snapshot::F64Bits::from_f64(-0.0)),
        )
        .unwrap();
        let positive_zero = ValueCell::from_schema_data(
            SchemaBody::FloatingPoint(FloatWidth::W64),
            ValueDataDraft::F64(crate::snapshot::F64Bits::from_f64(0.0)),
        )
        .unwrap();
        assert!(negative_zero.key_eq(&positive_zero).unwrap());
    }

    #[cfg(feature = "string")]
    #[test]
    fn canonical_values_rebind_by_schema_definition_across_reordered_tables() {
        let source = test_schema(SchemaBody::String, Vec::new().into_boxed_slice(), &[]);
        let value = finalize_draft(
            source.id,
            &source.shape,
            &source.schemas,
            ValueDataDraft::String("shared".into()),
        )
        .unwrap();
        let replacement = finalize_draft(
            source.id,
            &source.shape,
            &source.schemas,
            ValueDataDraft::String("replacement".into()),
        )
        .unwrap();

        let mut target = SchemaTableBuilder::new();
        target
            .insert(
                SchemaDraft {
                    dimension_parameters: Vec::new().into_boxed_slice(),
                    body: SchemaBody::Bool,
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        target
            .insert(
                SchemaDraft {
                    dimension_parameters: Vec::new().into_boxed_slice(),
                    body: SchemaBody::String,
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let target = Rc::new(target.finish().unwrap().table);
        let target_schema = target.find_by_key(value.schema_key()).unwrap();
        assert_ne!(source.id, target_schema);

        let cell = ValueCell::from_value(value, target.clone()).unwrap();
        assert_eq!(cell.schema(), target_schema);
        assert!(matches!(
            cell.snapshot().unwrap().data(),
            ValueData::String(value) if value.as_ref() == "shared"
        ));

        cell.replace(&replacement).unwrap();
        assert_eq!(cell.schema(), target_schema);
        assert!(matches!(
            cell.snapshot().unwrap().data(),
            ValueData::String(value) if value.as_ref() == "replacement"
        ));
    }

    #[cfg(all(feature = "f64", feature = "string"))]
    #[test]
    fn dynamic_table_and_record_children_rebind_colliding_local_schema_ids() {
        let text = ValueCell::from_exact("none".to_owned()).unwrap();
        let number = ValueCell::from_exact(4.0_f64).unwrap();
        assert_eq!(text.schema(), number.schema());
        assert_ne!(text.schema_key(), number.schema_key());

        let columns = vec![(
            crate::SchemaField {
                name: "value".into(),
                schema: SchemaBody::Dynamic,
            },
            vec![text.clone(), number.clone()].into_boxed_slice(),
        )]
        .into_boxed_slice();
        let table = ValueCell::table_from_cell_columns(
            columns,
            CardinalitySpec::Exact(DimensionExpr::Constant(2)),
        )
        .unwrap();
        let record = ValueCell::record_from_cells(&[("table".into(), table.clone())]).unwrap();

        let ValueData::Record(record) = record.snapshot().unwrap().data().clone() else {
            panic!("record data")
        };
        let ValueData::Table(table_value) = &record.fields()[0] else {
            panic!("table field")
        };
        let SequenceView::Values(values) = table_value.column(0).unwrap() else {
            panic!("dynamic table column")
        };
        assert!(matches!(
            values[0],
            ValueData::Dynamic(ref value)
                if matches!(value.value().unwrap().data(), ValueData::String(text) if text.as_ref() == "none")
        ));
        assert!(matches!(
            values[1],
            ValueData::Dynamic(ref value)
                if matches!(value.value().unwrap().data(), ValueData::F64(number) if number.to_f64() == 4.0)
        ));

        text.replace(
            &ValueCell::from_exact("changed".to_owned())
                .unwrap()
                .snapshot()
                .unwrap(),
        )
        .unwrap();
        let replacement = table
            .rebuild_table_cell_columns(&[("value".into(), vec![text, number].into_boxed_slice())])
            .unwrap();
        let alias = table.clone();
        table.replace(&replacement).unwrap();
        assert!(table.same_cell(&alias));
        assert!(matches!(
            table.snapshot().unwrap().data(),
            ValueData::Table(value)
                if matches!(value.column(0), Some(SequenceView::Values(values))
                    if matches!(&values[0], ValueData::Dynamic(value)
                        if matches!(value.value().unwrap().data(), ValueData::String(text) if text.as_ref() == "changed")))
        ));

        let reversed = ValueCell::table_from_cell_columns(
            vec![(
                crate::SchemaField {
                    name: "value".into(),
                    schema: SchemaBody::Dynamic,
                },
                vec![
                    ValueCell::from_exact(4.0_f64).unwrap(),
                    ValueCell::from_exact("none".to_owned()).unwrap(),
                ]
                .into_boxed_slice(),
            )]
            .into_boxed_slice(),
            CardinalitySpec::Exact(DimensionExpr::Constant(2)),
        )
        .unwrap();
        assert!(reversed.snapshot().is_ok());
    }

    #[cfg(all(feature = "f64", feature = "matrixd"))]
    #[test]
    fn concrete_matrix_shapes_rebind_between_dynamic_and_fixed_schema_definitions() {
        let source = ValueCell::test_backing_for_representation(
            <crate::DMatrix<f64> as FunctionRuntimeType>::REPRESENTATION,
            Some((4, 1)),
        )
        .unwrap()
        .snapshot()
        .unwrap();
        let target = test_schema(
            SchemaBody::Matrix {
                element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
                dimensions: vec![DimensionExpr::Constant(4), DimensionExpr::Constant(1)]
                    .into_boxed_slice(),
            },
            Vec::new().into_boxed_slice(),
            &[],
        );

        let rebound = source
            .rebind(target.id, &target.shape, &target.schemas)
            .unwrap();
        assert_eq!(rebound.schema(), target.id);
        assert_eq!(
            rebound.schema_key(),
            target.schemas.entry(target.id).unwrap().key()
        );
        assert!(matches!(
            rebound.data(),
            ValueData::Matrix(matrix) if matrix.elements().len() == 4
        ));
    }

    #[cfg(all(feature = "u8", feature = "string"))]
    #[test]
    fn dynamic_table_and_map_extents_preserve_cell_identity_across_turns() {
        use crate::snapshot::{MapEntryDraft, TableColumnDraft};

        let table = ValueCell::empty_dynamic_table(
            vec![crate::SchemaField {
                name: "value".into(),
                schema: SchemaBody::UnsignedInteger(IntegerWidth::W8),
            }]
            .into_boxed_slice(),
        )
        .unwrap();
        let table_alias = table.clone();
        for values in [vec![1_u8, 2, 3], vec![9], vec![]] {
            let replacement = ValueCell::from_schema_data(
                table.closed_schema_body().unwrap(),
                ValueDataDraft::Table(
                    vec![TableColumnDraft {
                        name: "value".into(),
                        values: values
                            .into_iter()
                            .map(ValueDataDraft::U8)
                            .collect::<Vec<_>>()
                            .into_boxed_slice(),
                    }]
                    .into_boxed_slice(),
                ),
            )
            .unwrap()
            .snapshot()
            .unwrap();
            table.replace(&replacement).unwrap();
            assert!(table.same_cell(&table_alias));
        }

        let map = ValueCell::empty_dynamic_map(
            SchemaBody::UnsignedInteger(IntegerWidth::W8),
            SchemaBody::String,
        )
        .unwrap();
        let map_alias = map.clone();
        for entries in [vec![(1_u8, "one"), (2, "two")], vec![(2, "two")], vec![]] {
            let replacement = ValueCell::from_schema_data(
                map.closed_schema_body().unwrap(),
                ValueDataDraft::Map(
                    entries
                        .into_iter()
                        .map(|(key, value)| MapEntryDraft {
                            items: vec![
                                ValueDataDraft::U8(key),
                                ValueDataDraft::String(value.into()),
                            ]
                            .into_boxed_slice(),
                        })
                        .collect::<Vec<_>>()
                        .into_boxed_slice(),
                ),
            )
            .unwrap()
            .snapshot()
            .unwrap();
            map.replace(&replacement).unwrap();
            assert!(map.same_cell(&map_alias));
        }
    }

    #[cfg(feature = "f64")]
    #[test]
    fn turn_scoped_matrix_extents_change_without_replacing_the_cell() {
        let cell = ValueCell::dynamic_matrix(
            SchemaBody::FloatingPoint(FloatWidth::W64),
            vec![0, 0].into_boxed_slice(),
            Box::new([]),
        )
        .unwrap();
        let alias = cell.clone();

        for (dimensions, values) in [
            (vec![1, 3], vec![1.0, 2.0, 3.0]),
            (vec![2, 1], vec![4.0, 5.0]),
            (vec![0, 0], vec![]),
        ] {
            let next = cell
                .rebuild_matrix_drafts(
                    dimensions.clone().into_boxed_slice(),
                    values
                        .into_iter()
                        .map(|value| ValueDataDraft::F64(crate::snapshot::F64Bits::from_f64(value)))
                        .collect::<Vec<_>>()
                        .into_boxed_slice(),
                )
                .unwrap();
            cell.replace(&next).unwrap();
            assert!(cell.same_cell(&alias));
            assert_eq!(cell.shape().parameter_values(), dimensions.as_slice());
        }
    }

    #[cfg(feature = "f64")]
    #[test]
    fn detached_dynamic_cells_have_independent_payload_and_shape_state() {
        let source = ValueCell::dynamic_matrix(
            SchemaBody::FloatingPoint(FloatWidth::W64),
            vec![1, 1].into_boxed_slice(),
            vec![ValueDataDraft::F64(crate::snapshot::F64Bits::from_f64(1.0))].into_boxed_slice(),
        )
        .unwrap();
        let before = source.detached_clone().unwrap();
        let detached = source.detached_clone().unwrap();
        let replacement = detached
            .rebuild_matrix_drafts(
                vec![1, 2].into_boxed_slice(),
                vec![
                    ValueDataDraft::F64(crate::snapshot::F64Bits::from_f64(2.0)),
                    ValueDataDraft::F64(crate::snapshot::F64Bits::from_f64(3.0)),
                ]
                .into_boxed_slice(),
            )
            .unwrap();

        detached.replace(&replacement).unwrap();

        assert_eq!(source.shape().parameter_values(), &[1, 1]);
        assert_eq!(detached.shape().parameter_values(), &[1, 2]);
        assert!(source.snapshot_eq(&before).unwrap());
        assert!(!source.same_cell(&detached));
    }

    #[cfg(feature = "string")]
    #[test]
    fn detached_canonical_cells_keep_managed_ownership_and_mutate_independently() {
        let source = ValueCell::from_exact("source".to_owned()).unwrap();
        let detached = source.detached_clone().unwrap();
        assert!(source.has_managed_canonical_storage().unwrap());
        assert!(detached.has_managed_canonical_storage().unwrap());
        assert!(source.memory_domain().is_some());
        assert!(detached.memory_domain().is_some());
        assert!(!source.same_logical_cell(&detached));
        assert!(source.same_storage(&detached));

        let replacement = detached
            .rebuild_data_draft(ValueDataDraft::String("a much larger replacement".into()))
            .unwrap();
        detached.replace(&replacement).unwrap();
        assert!(!source.same_storage(&detached));
        assert!(matches!(
            detached.snapshot().unwrap().data(),
            ValueData::String(value) if value.as_ref() == "a much larger replacement"
        ));
        assert!(matches!(
            source.snapshot().unwrap().data(),
            ValueData::String(value) if value.as_ref() == "source"
        ));

        let tuple = ValueCell::tuple_from_cells(&[source.clone(), detached.clone()]).unwrap();
        let tuple_clone = tuple.detached_clone().unwrap();
        assert!(tuple_clone.has_managed_canonical_storage().unwrap());
        assert!(tuple_clone.memory_domain().is_some());
        let tuple_replacement = tuple_clone
            .rebuild_tuple_cells(&[detached.clone(), source.clone()])
            .unwrap();
        tuple_clone.replace(&tuple_replacement).unwrap();
        assert!(!tuple.same_logical_cell(&tuple_clone));
        assert!(!tuple.snapshot_eq(&tuple_clone).unwrap());
    }

    #[cfg(feature = "f64")]
    #[test]
    fn staged_dynamic_replacement_changes_shape_only_when_committed() {
        let sink = ValueCell::dynamic_matrix(
            SchemaBody::FloatingPoint(FloatWidth::W64),
            vec![1, 1].into_boxed_slice(),
            vec![ValueDataDraft::F64(crate::snapshot::F64Bits::from_f64(1.0))].into_boxed_slice(),
        )
        .unwrap();
        let alias = sink.clone();
        let before = sink.detached_clone().unwrap();
        let replacement = sink
            .rebuild_matrix_drafts(
                vec![1, 2].into_boxed_slice(),
                vec![
                    ValueDataDraft::F64(crate::snapshot::F64Bits::from_f64(2.0)),
                    ValueDataDraft::F64(crate::snapshot::F64Bits::from_f64(3.0)),
                ]
                .into_boxed_slice(),
            )
            .unwrap();
        let expected = sink.detached_clone().unwrap();
        expected.replace(&replacement).unwrap();

        let abandoned = sink
            .stage_managed_replacement(&replacement)
            .unwrap()
            .unwrap();
        assert_eq!(sink.shape().parameter_values(), &[1, 1]);
        assert!(sink.snapshot_eq(&before).unwrap());
        drop(abandoned);
        assert_eq!(sink.shape().parameter_values(), &[1, 1]);
        assert!(sink.snapshot_eq(&before).unwrap());

        let committed = sink
            .stage_managed_replacement(&replacement)
            .unwrap()
            .unwrap();
        assert_eq!(sink.shape().parameter_values(), &[1, 1]);
        assert!(sink.snapshot_eq(&before).unwrap());
        let prepared = committed
            .domain
            .prepare_cell_publication(&committed.realized, vec![committed.candidate])
            .unwrap();
        crate::PreparedCellPublicationBatch::new(vec![prepared])
            .unwrap()
            .ready()
            .unwrap()
            .commit();

        assert!(sink.same_cell(&alias));
        assert_eq!(sink.shape().parameter_values(), &[1, 2]);
        assert!(sink.snapshot_eq(&expected).unwrap());
    }

    #[cfg(feature = "f64")]
    #[test]
    fn held_shape_borrows_make_direct_and_staged_replacement_atomic() {
        let cell = ValueCell::dynamic_matrix(
            SchemaBody::FloatingPoint(FloatWidth::W64),
            vec![1, 1].into_boxed_slice(),
            vec![ValueDataDraft::F64(crate::snapshot::F64Bits::from_f64(1.0))].into_boxed_slice(),
        )
        .unwrap();
        let replacement = cell
            .rebuild_matrix_drafts(
                vec![1, 2].into_boxed_slice(),
                vec![
                    ValueDataDraft::F64(crate::snapshot::F64Bits::from_f64(2.0)),
                    ValueDataDraft::F64(crate::snapshot::F64Bits::from_f64(3.0)),
                ]
                .into_boxed_slice(),
            )
            .unwrap();
        let before = cell.detached_clone().unwrap();

        let shape = cell.shape();
        let direct = cell.replace(&replacement).unwrap_err();
        assert_eq!(
            direct.kind_name(),
            "ValueCellBorrowConflict",
            "unexpected direct replacement error: {direct:?}"
        );
        assert_eq!(shape.parameter_values(), &[1, 1]);
        assert!(cell.snapshot_eq(&before).unwrap());
        drop(shape);

        let shape = cell.shape();
        let staged = cell
            .stage_managed_replacement(&replacement)
            .unwrap()
            .expect("managed replacement stages outside published storage");
        let prepared = staged
            .domain
            .prepare_cell_publication(&staged.realized, vec![staged.candidate])
            .unwrap();
        let ready = match staged.domain.ready_cell_publication(prepared) {
            Ok(_) => panic!("the final publication gate must reject a held shape borrow"),
            Err(error) => error,
        };
        assert_eq!(ready.kind_name(), "ValueCellBorrowConflict");
        assert_eq!(shape.parameter_values(), &[1, 1]);
        assert!(cell.snapshot_eq(&before).unwrap());
    }

    #[cfg(all(feature = "f64", feature = "matrixd"))]
    #[test]
    fn exact_dynamic_matrix_backing_resizes_without_replacing_its_handle() {
        let backing = Ref::new(crate::DMatrix::<f64>::zeros(0, 0));
        let alias = backing.clone();
        let cell = ValueCell::from_inferred_ref(backing, Some((0, 0))).unwrap();
        let schema_key = cell.schema_key();

        for (dimensions, values) in [
            (vec![2, 2], vec![1.0, 2.0, 3.0, 4.0]),
            (vec![1, 3], vec![5.0, 6.0, 7.0]),
            (vec![0, 0], vec![]),
        ] {
            let next = cell
                .rebuild_matrix_drafts(
                    dimensions.clone().into_boxed_slice(),
                    values
                        .iter()
                        .copied()
                        .map(|value| ValueDataDraft::F64(crate::snapshot::F64Bits::from_f64(value)))
                        .collect::<Vec<_>>()
                        .into_boxed_slice(),
                )
                .unwrap();
            cell.replace(&next).unwrap();

            assert_eq!(cell.schema_key(), schema_key);
            assert_eq!(cell.shape().parameter_values(), dimensions.as_slice());
            assert!(
                cell.try_ref::<crate::DMatrix<f64>>()
                    .unwrap()
                    .same_handle(&alias)
            );
            assert_eq!(
                *alias.borrow(),
                crate::DMatrix::from_row_slice(
                    dimensions[0] as usize,
                    dimensions[1] as usize,
                    &values,
                )
            );
        }
    }

    #[cfg(feature = "u8")]
    #[test]
    fn exact_and_bounded_dynamic_collection_extents_are_enforced() {
        let values = |count: u8| {
            (0..count)
                .map(ValueDataDraft::U8)
                .collect::<Vec<_>>()
                .into_boxed_slice()
        };
        let exact = ValueCell::from_schema_data(
            SchemaBody::Set {
                element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
                cardinality: CardinalitySpec::Exact(DimensionExpr::Constant(2)),
            },
            ValueDataDraft::Set(values(1)),
        )
        .unwrap_err();
        assert!(exact.kind_message().contains("Cardinality"), "{exact:?}");

        let bounded = ValueCell::from_schema_data(
            SchemaBody::Set {
                element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
                cardinality: CardinalitySpec::Dynamic {
                    upper_bound: Some(DimensionExpr::Constant(2)),
                },
            },
            ValueDataDraft::Set(values(3)),
        )
        .unwrap_err();
        assert!(
            bounded.kind_message().contains("Cardinality"),
            "{bounded:?}"
        );
    }

    #[cfg(feature = "u8")]
    #[test]
    fn rebuilding_a_dynamic_collection_preserves_its_bound_shape_witnesses() {
        let dimensions = [
            DimensionParameterDeclaration {
                id: crate::DimensionParameterId::new(0),
                origin: crate::DimensionParameterOrigin::Inferred,
                lifetime: crate::DimensionLifetime::Turn,
                lower_bound: DimensionExpr::Constant(0),
                upper_bound: None,
            },
            DimensionParameterDeclaration {
                id: crate::DimensionParameterId::new(1),
                origin: crate::DimensionParameterOrigin::Inferred,
                lifetime: crate::DimensionLifetime::Turn,
                lower_bound: DimensionExpr::Constant(0),
                upper_bound: None,
            },
        ];
        let schema = test_schema(
            SchemaBody::Set {
                element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
                cardinality: CardinalitySpec::Dynamic {
                    upper_bound: Some(DimensionExpr::Add(
                        vec![
                            DimensionExpr::Parameter(crate::DimensionParameterId::new(0)),
                            DimensionExpr::Parameter(crate::DimensionParameterId::new(1)),
                        ]
                        .into_boxed_slice(),
                    )),
                },
            },
            dimensions.into(),
            &[2, 2],
        );
        let value = finalize_draft(
            schema.id,
            &schema.shape,
            &schema.schemas,
            ValueDataDraft::Set(
                vec![ValueDataDraft::U8(1), ValueDataDraft::U8(2)].into_boxed_slice(),
            ),
        )
        .unwrap();
        let cell = ValueCell::from_runtime_value(value, schema.schemas).unwrap();
        let descriptor = cell.resolved_descriptor().unwrap();

        let next = cell
            .rebuild_set_drafts(
                vec![
                    ValueDataDraft::U8(1),
                    ValueDataDraft::U8(2),
                    ValueDataDraft::U8(3),
                ]
                .into_boxed_slice(),
            )
            .unwrap();
        cell.replace(&next).unwrap();

        assert_eq!(cell.shape().parameter_values(), &[2, 2]);
        assert_eq!(cell.resolved_descriptor().unwrap(), descriptor);
    }

    #[cfg(feature = "u8")]
    #[test]
    fn exact_and_dynamic_collection_schemas_are_semantically_distinct() {
        let element = Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8));
        let exact = ValueCell::from_schema_data(
            SchemaBody::Set {
                element: element.clone(),
                cardinality: CardinalitySpec::Exact(DimensionExpr::Constant(0)),
            },
            ValueDataDraft::Set(Box::new([])),
        )
        .unwrap();
        let dynamic = ValueCell::from_schema_data(
            SchemaBody::Set {
                element,
                cardinality: CardinalitySpec::Dynamic { upper_bound: None },
            },
            ValueDataDraft::Set(Box::new([])),
        )
        .unwrap();

        assert_ne!(exact.schema_key(), dynamic.schema_key());
        assert!(!exact.snapshot_eq(&dynamic).unwrap());
    }

    #[cfg(feature = "u8")]
    #[test]
    fn exact_collection_values_rebind_into_compatible_dynamic_extent_schemas() {
        let exact = ValueCell::from_schema_data(
            SchemaBody::Set {
                element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
                cardinality: CardinalitySpec::Exact(DimensionExpr::Constant(2)),
            },
            ValueDataDraft::Set(
                vec![ValueDataDraft::U8(1), ValueDataDraft::U8(2)].into_boxed_slice(),
            ),
        )
        .unwrap()
        .snapshot()
        .unwrap();
        let target = test_schema(
            SchemaBody::Set {
                element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
                cardinality: CardinalitySpec::Dynamic {
                    upper_bound: Some(DimensionExpr::Constant(3)),
                },
            },
            Vec::new().into_boxed_slice(),
            &[],
        );

        let rebound = exact
            .rebind(target.id, &target.shape, &target.schemas)
            .unwrap();
        assert_eq!(rebound.schema(), target.id);
        assert_eq!(
            rebound.schema_key(),
            target.schemas.entry(target.id).unwrap().key()
        );
        assert!(matches!(
            rebound.data(),
            ValueData::Set(elements) if elements.elements().len() == 2
        ));

        let exact_map = ValueCell::from_schema_data(
            SchemaBody::Map {
                key: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
                value: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
                cardinality: CardinalitySpec::Exact(DimensionExpr::Constant(1)),
            },
            ValueDataDraft::Map(
                vec![crate::snapshot::MapEntryDraft {
                    items: vec![ValueDataDraft::U8(1), ValueDataDraft::U8(2)].into_boxed_slice(),
                }]
                .into_boxed_slice(),
            ),
        )
        .unwrap()
        .snapshot()
        .unwrap();
        let map_target = test_schema(
            SchemaBody::Map {
                key: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
                value: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
                cardinality: CardinalitySpec::Dynamic {
                    upper_bound: Some(DimensionExpr::Constant(2)),
                },
            },
            Vec::new().into_boxed_slice(),
            &[],
        );
        let rebound_map = exact_map
            .rebind(map_target.id, &map_target.shape, &map_target.schemas)
            .unwrap();
        assert!(matches!(
            rebound_map.data(),
            ValueData::Map(entries) if entries.entries().len() == 1
        ));
    }

    #[cfg(feature = "f64")]
    #[test]
    fn replacement_checks_schema_and_shape_before_mutating() {
        let scalar_schema = f64_schema();
        let scalar = Ref::new(1.0_f64);
        let cell = ValueCell::from_ref(
            scalar.clone(),
            scalar_schema.id,
            scalar_schema.shape.clone(),
            scalar_schema.schemas.clone(),
        )
        .unwrap();
        let index_schema = test_schema(SchemaBody::Index, Vec::new().into_boxed_slice(), &[]);
        let index = finalize_draft(
            index_schema.id,
            &index_schema.shape,
            &index_schema.schemas,
            ValueDataDraft::Index(2),
        )
        .unwrap();
        assert!(
            cell.replace(&index)
                .unwrap_err()
                .kind_as::<ValueCellSchemaMismatch>()
                .is_some()
        );
        assert_eq!(*scalar.borrow(), 1.0);
    }

    #[cfg(all(feature = "f64", feature = "matrixd"))]
    #[test]
    fn replacement_rejects_a_different_shape_without_resizing_the_backing() {
        let two_by_two = matrix_schema(2, 2);
        let backing = Ref::new(crate::DMatrix::from_element(2, 2, 1.0));
        let cell = ValueCell::from_ref(
            backing.clone(),
            two_by_two.id,
            two_by_two.shape,
            two_by_two.schemas.clone(),
        )
        .unwrap();
        let replacement = ValueDraft {
            schema: two_by_two.id,
            shape_values: vec![1, 4].into_boxed_slice(),
            data: ValueDataDraft::Matrix(
                vec![ValueDataDraft::F64(crate::snapshot::F64Bits::from_f64(2.0)); 4]
                    .into_boxed_slice(),
            ),
        }
        .finalize(&SnapshotValidationContext::new(&two_by_two.schemas))
        .unwrap();

        assert!(
            cell.replace(&replacement)
                .unwrap_err()
                .kind_as::<ValueCellShapeMismatch>()
                .is_some()
        );
        assert_eq!(backing.borrow().shape(), (2, 2));
    }

    #[cfg(feature = "f64")]
    #[test]
    fn borrow_conflicts_are_structured_and_debug_never_exposes_payload_or_address() {
        let schema = f64_schema();
        let backing = Ref::new(12345.625_f64);
        let cell =
            ValueCell::from_ref(backing.clone(), schema.id, schema.shape, schema.schemas).unwrap();
        let available = format!("{cell:?}");
        assert!(!available.contains("12345.625"));
        assert!(!available.contains("0x"));

        let _borrow = backing.borrow_mut();
        let error = cell.snapshot().unwrap_err();
        assert_eq!(
            error.kind_as::<ValueCellBorrowConflict>().unwrap().access,
            CellAccess::Snapshot
        );
        let borrowed = format!("{cell:?}");
        assert!(borrowed.contains("Borrowed"));
        assert!(!borrowed.contains("12345.625"));
        assert!(!borrowed.contains("0x"));
    }
}
