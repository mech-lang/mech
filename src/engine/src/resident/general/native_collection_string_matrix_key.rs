//! One admitted immutable String-matrix loan for native Map-key WORK.
//!
//! The table is an explicit build-owned C layout, never the private Box<str>
//! representation. It pins ONE actual canonical source and contains no String
//! Value, selector Value, visit permit, renewed budget or owned key payload.
use super::{InvokeError, NativePreparedLibraryCall, finite_limits};
use crate::{
    native_artifact::{NativeCollectionPrimitive as Primitive, NativeScalarValue},
    resident::budget::{
        self, NativeLibraryWorkBudget, ResidentBudgetMeter,
        native_turn_plan_cache::NativeProjectionIndexContext,
    },
};
use mech_core::{
    IntegerWidth, ManagedMemoryReservation, ManagedNodeReservation, MemoryBudgetLimits,
    ResourceDemand, SchemaBody, SchemaKey, Value, ValueData,
    snapshot::{SequenceView, ValueFootprint},
};
use std::sync::atomic::{AtomicU64, Ordering};
use std::{alloc::Layout, mem::MaybeUninit};

#[cfg(all(test, any(feature = "full_source", feature = "full_compiler")))]
#[path = "native_collection_string_matrix_key_tests.rs"]
mod tests;

static NEXT_LOAN: AtomicU64 = AtomicU64::new(1);

fn next_epoch(counter: &AtomicU64) -> Option<u64> {
    // One bounded attempt: contention is an optional optimization miss, never
    // an unmetered retry loop. The caller prepays its fixed setup inspection.
    let current = counter.load(Ordering::Relaxed);
    let next = current.checked_add(1)?;
    counter
        .compare_exchange(current, next, Ordering::Relaxed, Ordering::Relaxed)
        .ok()
}

// A fixed slice allocation has exactly the admitted layout. In particular it
// cannot over-reserve a Vec or allocate a second table while shrinking one.
fn allocate_cells(len: usize) -> Option<Box<[MaybeUninit<NativeStringMatrixKeyCell>]>> {
    if len == 0 {
        return Some(Box::default());
    }
    let layout = Layout::array::<MaybeUninit<NativeStringMatrixKeyCell>>(len).ok()?;
    // SAFETY: caller admitted this exact nonzero array layout. The global
    // allocator is also used by Box's eventual destructor; MaybeUninit makes
    // every partially initialized failure path safe to drop without reading.
    let pointer =
        unsafe { std::alloc::alloc(layout) }.cast::<MaybeUninit<NativeStringMatrixKeyCell>>();
    let pointer = std::ptr::NonNull::new(pointer)?;
    // SAFETY: this fresh allocation has precisely `len` elements of the same
    // layout/alignment, is uniquely owned, and was allocated with Global.
    Some(unsafe { Box::from_raw(std::ptr::slice_from_raw_parts_mut(pointer.as_ptr(), len)) })
}

/// Read-only physical row-major cells. Logical scalar output/clone obligations
/// remain canonical even though no scalar String is physically constructed.
/// Generated code must retain the loan, prove its current epoch, check bounds
/// and caps, and debit the SAME invocation before consuming a cell.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct NativeStringMatrixKeyCell {
    pub address: u64,
    pub bytes: u64,
    pub logical_output_bytes: u64,
    pub logical_cloned_bytes: u64,
    pub selection_compute_work: u64,
    pub selection_comparison_work: u64,
    pub logical_temporary_bytes: u64,
    pub logical_extra_nodes: u64,
}

/// Descriptive native geometry, not authority to visit or publish anything.
/// `cells` points to `count` NativeStringMatrixKeyCell records while the loan
/// remains alive. Linear source selectors are one-based and column-major.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct NativeStringMatrixKeyView {
    pub cells: u64,
    pub count: u64,
    pub rows: u64,
    pub columns: u64,
    pub epoch: u64,
    pub selector_compute_work: u64,
    pub selector_comparison_work: u64,
}

/// A reference-model current producer result, not a Value or reusable fee
/// credit. Native CFG can keep this ordinal in SSA instead of calling Rust.
#[derive(Debug)]
pub struct NativeStringMatrixKeyOrdinal {
    loan: u64,
    serial: u64,
    ordinal: usize,
}

#[derive(Debug)]
pub struct NativeStringMatrixKeyLoan {
    // Drop table before source pin and the claims paying for both metadata.
    cells: Box<[NativeStringMatrixKeyCell]>,
    owner: Value,
    work: NativeLibraryWorkBudget,
    producer: mech_core::NodeId,
    key: SchemaKey,
    rows: usize,
    columns: usize,
    epoch: u64,
    serial: u64,
    selector_compute_work: u64,
    selector_comparison_work: u64,
    source_footprint: ValueFootprint,
    metadata_bytes: u64,
    metadata_nodes: u64,
    _reservation: ManagedMemoryReservation,
    _nodes: ManagedNodeReservation,
}

// Canonical clones preserve this Arc-backed ShapeInstance. Rebound wrappers
// with a distinct shape allocation conservatively miss; never scan an arbitrary
// parameter tape before a current-source validation has been funded.
fn same_shape_identity(
    actual: &mech_core::ShapeInstance,
    pinned: &mech_core::ShapeInstance,
) -> bool {
    core::ptr::eq(actual, pinned)
}

fn add(a: u64, b: u64) -> Result<u64, InvokeError> {
    a.checked_add(b).ok_or(InvokeError::InvalidInput)
}
fn cap_error(
    node: mech_core::NodeId,
    demand: ResourceDemand,
    bytes: u64,
    limits: MemoryBudgetLimits,
) -> Result<(), InvokeError> {
    if let Some(violation) = mech_core::first_call_memory_budget_violation(
        mech_core::MemoryObjectOwner::NodeOutput { node, port: 0 },
        demand,
        bytes,
        0,
        limits,
    ) {
        return Err(InvokeError::Plan(
            mech_core::MemoryPlanError::TargetLimitExceeded { violation },
        ));
    }
    Ok(())
}

fn packed_key_cell(
    values: SequenceView<'_>,
    ordinal: usize,
    text: &str,
    selector_footprint: ValueFootprint,
    selector_cost: budget::KernelCostEstimate,
) -> Result<NativeStringMatrixKeyCell, InvokeError> {
    let mut meter = ResidentBudgetMeter::for_closed_analysis(u64::MAX, u64::MAX);
    let mut footprint = ValueFootprint::default();
    let mut finalization = 0;
    crate::resident::numeric::selected_sequence_footprint_with_finalization(
        &mut footprint,
        &mut finalization,
        &mut meter,
        &SchemaBody::String,
        values,
        ordinal,
    )?;
    let output = budget::projected_canonical_value_footprint(footprint, 0)?;
    let cost = meter.estimate();
    let logical_extra_nodes = add(
        add(
            selector_footprint
                .node_count
                .checked_mul(2)
                .ok_or(InvokeError::InvalidInput)?,
            footprint.node_count,
        )?,
        output
            .node_count
            .checked_mul(2)
            .ok_or(InvokeError::InvalidInput)?,
    )?;
    let containers = add(
        core::mem::size_of::<mech_core::ValueDataDraft>() as u64,
        core::mem::size_of::<Value>() as u64,
    )?;
    let logical_temporary_bytes = add(
        add(
            add(
                output
                    .retained_bytes
                    .checked_mul(2)
                    .ok_or(InvokeError::InvalidInput)?,
                containers,
            )?,
            selector_footprint.retained_bytes,
        )?,
        add(
            add(selector_cost.temporary_bytes(), cost.temporary_bytes())?,
            core::mem::size_of::<usize>() as u64,
        )?,
    )?;
    Ok(NativeStringMatrixKeyCell {
        address: text.as_ptr() as u64,
        bytes: text.len() as u64,
        logical_output_bytes: output.retained_bytes,
        logical_cloned_bytes: add(selector_footprint.retained_bytes, footprint.retained_bytes)?,
        selection_compute_work: add(add(cost.compute_work(), finalization)?, 2)?,
        selection_comparison_work: add(cost.comparison_work(), finalization)?,
        logical_temporary_bytes,
        logical_extra_nodes,
    })
}

impl NativePreparedLibraryCall {
    /// The constructor-private selected factory/template is resolved again on
    /// image restoration; a lookalike contract or cache headroom is no witness.
    pub fn supports_string_matrix_key_loan(&self) -> bool {
        let [source, selector] = self.template.inputs() else {
            return false;
        };
        self.template.primitive() == Primitive::MatrixScalarAccess
            && matches!(source.body(), SchemaBody::Matrix { element, dimensions }
                if element.as_ref() == &SchemaBody::String && dimensions.len() == 2)
            && selector.body() == &SchemaBody::UnsignedInteger(IntegerWidth::W64)
            && selector.dimension_parameters().is_empty()
            && self.template.output().body() == &SchemaBody::String
            && self.template.output().dimension_parameters().is_empty()
    }

    /// Setup only. `coexistence` contains every OTHER current canonical/private
    /// owner and cache; this method adds the complete source plus actual table
    /// residency. A storage/headroom miss declines before table allocation.
    /// The full source/schema traversal is paid once at this loan boundary.
    /// This new native plan intentionally removes generic per-read binding,
    /// turn-plan and immutable source/cache walks; it does not promise their
    /// old total fee. Each logical scalar selection still pays stored measured
    /// selector/element work and retains exact output/clone caps below.
    pub fn try_string_matrix_key_loan(
        &self,
        source: &Value,
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
        coexistence: ResourceDemand,
    ) -> Result<Option<NativeStringMatrixKeyLoan>, InvokeError> {
        finite_limits(self.template.origin().node(), limits).map_err(InvokeError::Binding)?;
        if !self.supports_string_matrix_key_loan() {
            return Ok(None);
        }
        work.with_call_limits(
            limits.max_compute_work.unwrap(),
            limits.max_comparison_work.unwrap(),
            || {
                let mech_core::ResolvedOperationContract::Declared(contract) =
                    self.template.contract()
                else {
                    return Err(InvokeError::InvalidInput);
                };
                if source.schema() != contract.inputs[0].schema
                    || source.schema_key() != self.input_keys[0]
                    || source.shape().parameter_values().len()
                        != self.template.inputs()[0].dimension_parameters().len()
                {
                    return Err(InvokeError::InvalidInput);
                }
                // Core's validate_against is a constant-size schema-entry/key
                // comparison on this already validated immutable Value. It does
                // not inspect matrix cells, text, shape-expression trees or tokens.
                source
                    .validate_against(&self.schemas)
                    .map_err(|_| InvokeError::InvalidInput)?;
                let memory = work.memory_budget();
                let source_bytes = source
                    .memory_budget_retained_bytes(&memory)
                    .ok_or(InvokeError::UnadmittedValue)?;
                let source_nodes = source
                    .memory_budget_retained_nodes(&memory)
                    .ok_or(InvokeError::UnadmittedValue)?;
                let ValueData::Matrix(matrix) = source.data() else {
                    return Err(InvokeError::InvalidInput);
                };
                let SequenceView::String(strings) = matrix.elements() else {
                    return Ok(None);
                };
                let table_bytes = (strings.len() as u64)
                    .checked_mul(core::mem::size_of::<NativeStringMatrixKeyCell>() as u64)
                    .ok_or(InvokeError::InvalidInput)?;
                let metadata_bytes = add(
                    table_bytes,
                    core::mem::size_of::<NativeStringMatrixKeyLoan>() as u64,
                )?;
                let metadata_nodes = 1 + u64::from(!strings.is_empty());
                let context = NativeProjectionIndexContext::new(work, limits, coexistence)
                    .with_added_capacity(source_bytes, source_nodes)?;
                if context
                    .check_capacity(metadata_bytes, metadata_nodes)
                    .is_err()
                {
                    return Ok(None);
                }
                let reservation = match memory.reserve_capacity(metadata_bytes) {
                    Ok(v) => v,
                    Err(_) => return Ok(None),
                };
                let nodes = match memory.reserve_nodes(metadata_nodes) {
                    Ok(v) => v,
                    Err(_) => return Ok(None),
                };
                // A single checked nonce claim is bounded even under concurrent
                // loan setup. A contention/exhaustion miss keeps this paid prefix
                // but drops both reservations and grants no native authority.
                context.charge(128, 0)?;
                let Some(epoch) = next_epoch(&NEXT_LOAN) else {
                    return Ok(None);
                };
                // Prepay construction of each fixed-size descriptor before the
                // metadata loop/allocation. This is physical setup work, not an
                // invented per-item String fee or a publication permit. The once-only
                // established binding_work envelope additionally funds the
                // complete declared schema/DimensionExpr geometry traversal
                // (lookup_work alone covers only shape-tape header reads).
                context.charge(
                    add(
                        add(self.lookup_work, self.binding_work)?,
                        table_bytes
                            .checked_mul(8)
                            .ok_or(InvokeError::InvalidInput)?,
                    )?,
                    0,
                )?;
                let mut source_meter = ResidentBudgetMeter::for_native_projection(work, limits)?;
                let source_footprint = budget::measure_snapshot_access_source_footprint(
                    &mut source_meter,
                    source,
                    &self.schemas,
                )?;
                let (rows, columns) = crate::resident::numeric::snapshot_matrix_dimensions(
                    source,
                    self.template.inputs()[0].body(),
                )?;
                if rows.checked_mul(columns) != Some(strings.len()) {
                    return Err(InvokeError::InvalidInput);
                }
                let mut selector_meter =
                    ResidentBudgetMeter::for_closed_analysis(u64::MAX, u64::MAX);
                let selector_footprint = budget::measure_native_scalar_frame_footprint(
                    &mut selector_meter,
                    self.template.inputs()[1].body(),
                    NativeScalarValue::U64(1),
                )?;
                let selector_cost = selector_meter.estimate();
                let Some(mut cells) = allocate_cells(strings.len()) else {
                    return Ok(None);
                };
                for (ordinal, text) in strings.iter().enumerate() {
                    cells[ordinal].write(packed_key_cell(
                        matrix.elements(),
                        ordinal,
                        text,
                        selector_footprint,
                        selector_cost,
                    )?);
                }
                // SAFETY: exactly one write occurred for every source ordinal
                // in 0..strings.len(), which is the fixed allocation's length.
                let cells = unsafe { cells.assume_init() };

                Ok(Some(NativeStringMatrixKeyLoan {
                    cells,
                    owner: source.clone(),
                    work: work.clone(),
                    producer: self.template.origin().node(),
                    key: self.output_key,
                    rows,
                    columns,
                    epoch,
                    serial: 0,
                    selector_compute_work: selector_cost.compute_work(),
                    selector_comparison_work: selector_cost.comparison_work(),
                    source_footprint,
                    metadata_bytes,
                    metadata_nodes,
                    _reservation: reservation,
                    _nodes: nodes,
                }))
            },
        )?
    }
}

impl NativeStringMatrixKeyLoan {
    pub fn retained_metadata_demand(&self) -> ResourceDemand {
        ResourceDemand {
            turn_peak_bytes: self.metadata_bytes,
            retained_nodes: self.metadata_nodes,
            ..ResourceDemand::default()
        }
    }
    pub fn key_schema_key(&self) -> SchemaKey {
        self.key
    }
    pub fn source_footprint(&self) -> ValueFootprint {
        self.source_footprint
    }

    fn current(&self, source: &Value, work: &NativeLibraryWorkBudget) -> Result<(), InvokeError> {
        if self.epoch == 0
            || !self.work.shares_account(work)
            || source.schema() != self.owner.schema()
            || source.schema_key() != self.owner.schema_key()
            || !same_shape_identity(source.shape(), self.owner.shape())
            || !source.shares_frozen_storage(&self.owner)
            || source
                .memory_budget_retained_bytes(&work.memory_budget())
                .is_none()
            || source
                .memory_budget_retained_nodes(&work.memory_budget())
                .is_none()
        {
            return Err(InvokeError::InvalidInput);
        }
        Ok(())
    }

    /// Once-checked setup geometry for an uninterrupted native WORK region.
    /// Close/revalidate after every cold boundary that can change current
    /// source/scope/account; retain this loan until all borrowed Map keys have
    /// been published or retired. A table epoch never pins an Arena generation.
    pub fn native_view(
        &self,
        source: &Value,
        work: &NativeLibraryWorkBudget,
    ) -> Result<NativeStringMatrixKeyView, InvokeError> {
        self.current(source, work)?;
        Ok(NativeStringMatrixKeyView {
            cells: self.cells.as_ptr() as u64,
            count: self.cells.len() as u64,
            rows: self.rows as u64,
            columns: self.columns as u64,
            epoch: self.epoch,
            selector_compute_work: self.selector_compute_work,
            selector_comparison_work: self.selector_comparison_work,
        })
    }

    /// Maintained Rust differential/reference boundary. Emitted native code
    /// uses the same cells, order and checked debits directly, without a Rust
    /// callback for each lookup. Coexistence excludes this source/loan only.
    pub fn admit_element(
        &mut self,
        source: &Value,
        index: u64,
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
        coexistence: ResourceDemand,
    ) -> Result<NativeStringMatrixKeyOrdinal, InvokeError> {
        finite_limits(self.producer, limits).map_err(InvokeError::Binding)?;
        self.current(source, work)?;
        self.serial = match self.serial.checked_add(1) {
            Some(serial) => serial,
            None => {
                // A failed producer can never resurrect an old ordinal, even
                // at exhaustion. Poison the loan rather than wrapping a ticket.
                self.epoch = 0;
                return Err(InvokeError::InvalidInput);
            }
        };
        work.with_call_limits(
            limits.max_compute_work.unwrap(),
            limits.max_comparison_work.unwrap(),
            || {
                let source_bytes = source
                    .memory_budget_retained_bytes(&work.memory_budget())
                    .ok_or(InvokeError::UnadmittedValue)?;
                let source_nodes = source
                    .memory_budget_retained_nodes(&work.memory_budget())
                    .ok_or(InvokeError::UnadmittedValue)?;
                let context = NativeProjectionIndexContext::new(work, limits, coexistence)
                    .with_added_capacity(
                        add(source_bytes, self.metadata_bytes)?,
                        add(source_nodes, self.metadata_nodes)?,
                    )?;
                context.check_capacity(0, 0)?;
                context.charge(self.selector_compute_work, self.selector_comparison_work)?;
                // The maintained matrix cost checks the scalar count before
                // traversing/checking its one-based selected ordinal.
                cap_error(
                    self.producer,
                    ResourceDemand {
                        output_elements: add(coexistence.output_elements, 1)?,
                        ..ResourceDemand::default()
                    },
                    0,
                    limits,
                )?;
                let linear =
                    crate::resident::numeric::available_scalar_index(index, self.cells.len())?;
                let ordinal = if self.rows == 1 || self.columns == 1 {
                    linear
                } else {
                    (linear % self.rows) * self.columns + linear / self.rows
                };
                let cell = self.cells.get(ordinal).ok_or(InvokeError::InvalidInput)?;
                context.charge(cell.selection_compute_work, cell.selection_comparison_work)?;
                cap_error(
                    self.producer,
                    ResourceDemand {
                        output_elements: add(coexistence.output_elements, 1)?,
                        cloned_bytes: add(coexistence.cloned_bytes, cell.logical_cloned_bytes)?,
                        ..ResourceDemand::default()
                    },
                    cell.logical_output_bytes,
                    limits,
                )?;
                context.check_capacity(cell.logical_temporary_bytes, cell.logical_extra_nodes)?;
                Ok(NativeStringMatrixKeyOrdinal {
                    loan: self.epoch,
                    serial: self.serial,
                    ordinal,
                })
            },
        )?
    }

    /// Borrow only for a genuine same-schema selected String→Map-key use.
    /// The closure cannot export a borrowed lifetime. Map lookup/insertion,
    /// missing-key errors and allocation stay in their original maintained
    /// consumer; this metadata grants no permission to invoke that operation.
    pub fn with_element<T>(
        &self,
        consumer: &NativePreparedLibraryCall,
        source: &Value,
        selected: &NativeStringMatrixKeyOrdinal,
        work: &NativeLibraryWorkBudget,
        use_key: impl FnOnce(&str) -> T,
    ) -> Result<T, InvokeError> {
        self.current(source, work)?;
        let inputs = consumer.template.inputs();
        if selected.loan != self.epoch
            || selected.serial != self.serial
            || !matches!(
                consumer.template.primitive(),
                Primitive::MapContainsKey | Primitive::MapKeyAccess | Primitive::MapInsert
            )
            || inputs.len() < 2
            || consumer.input_keys.get(1) != Some(&self.key)
            || inputs[1].body() != &SchemaBody::String
            || !inputs[1].dimension_parameters().is_empty()
            || !matches!(inputs[0].body(),SchemaBody::Map {key,value,..} if key.as_ref()==&SchemaBody::String && value.as_ref()==&SchemaBody::UnsignedInteger(IntegerWidth::W64))
        {
            return Err(InvokeError::InvalidInput);
        }
        let text = match self.owner.data() {
            ValueData::Matrix(matrix) => {
                let SequenceView::String(strings) = matrix.elements() else {
                    return Err(InvokeError::InvalidInput);
                };
                strings.get(selected.ordinal).map(AsRef::as_ref)
            }
            ValueData::Map(map) => {
                match map.entries().get(selected.ordinal).map(|e| e.key().data()) {
                    Some(ValueData::String(text)) => Some(text.as_ref()),
                    _ => None,
                }
            }
            _ => None,
        }
        .ok_or(InvokeError::InvalidInput)?;
        Ok(use_key(text))
    }
}

#[path = "native_collection_borrowed_map_keys.rs"]
mod map_keys;
pub use map_keys::{NativeBorrowedMapKeys, NativeBorrowedMapKeysChoice};
