//! A private row-vector view of canonical Map keys. The real immutable Map
//! snapshot is pinned independently of any compiler arena handle or dictionary.
use super::*;
use mech_core::{SchemaId, ShapeInstance};

#[derive(Debug)]
pub struct NativeBorrowedMapKeys {
    loan: NativeStringMatrixKeyLoan,
    schema: SchemaId,
    schema_key: SchemaKey,
    shape: ShapeInstance,
    logical: ValueFootprint,
    source_factory: std::sync::Arc<()>,
    source_limits: MemoryBudgetLimits,
    source_spent: (u64, u64),
}

#[derive(Debug)]
pub enum NativeBorrowedMapKeysChoice {
    View(NativeBorrowedMapKeys),
    Canonical(Value),
}
impl NativePreparedLibraryCall {
    #[cfg(feature = "map_ops")]
    pub fn try_borrowed_map_keys(
        &self,
        element: &NativePreparedLibraryCall,
        source: &Value,
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
        coexistence: ResourceDemand,
    ) -> Result<Option<NativeBorrowedMapKeys>, InvokeError> {
        finite_limits(self.template.origin().node(), limits).map_err(InvokeError::Binding)?;
        if work.has_active_call_limit()? {
            return Ok(None);
        }
        self.try_borrowed_map_keys_in_call(element, source, limits, work, coexistence, work.used())
    }

    #[cfg(feature = "map_ops")]
    fn try_borrowed_map_keys_in_call(
        &self,
        element: &NativePreparedLibraryCall,
        source: &Value,
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
        coexistence: ResourceDemand,
        source_start: (u64, u64),
    ) -> Result<Option<NativeBorrowedMapKeys>, InvokeError> {
        finite_limits(self.template.origin().node(), limits).map_err(InvokeError::Binding)?;
        if self.template.primitive() != Primitive::StringMapKeys
            || !element.supports_string_matrix_key_loan()
            || element.input_keys.first() != Some(&self.output_key)
            || element.template.program_revision() != self.template.program_revision()
        {
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
                {
                    return Err(InvokeError::InvalidInput);
                }
                source
                    .validate_against(&self.schemas)
                    .map_err(|_| InvokeError::InvalidInput)?;
                let ValueData::Map(map) = source.data() else {
                    return Err(InvokeError::InvalidInput);
                };
                let entries = map.entries();
                let memory = work.memory_budget();
                let source_bytes = source
                    .memory_budget_retained_bytes(&memory)
                    .ok_or(InvokeError::UnadmittedValue)?;
                let source_nodes = source
                    .memory_budget_retained_nodes(&memory)
                    .ok_or(InvokeError::UnadmittedValue)?;
                let parameters = self.template.output().dimension_parameters().len();
                let shape_bytes = add(
                    (parameters as u64)
                        .checked_mul(32)
                        .ok_or(InvokeError::InvalidInput)?,
                    512,
                )?;
                let table_bytes = (entries.len() as u64)
                    .checked_mul(core::mem::size_of::<NativeStringMatrixKeyCell>() as u64)
                    .ok_or(InvokeError::InvalidInput)?;
                let metadata_bytes = add(
                    add(table_bytes, shape_bytes)?,
                    core::mem::size_of::<NativeBorrowedMapKeys>() as u64,
                )?;
                let metadata_nodes = add(4, u64::from(!entries.is_empty()))?;
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
                // Fund all schema/shape and per-cell metadata work before any scan.
                // Copy/finalization fees vanish with the representation, while
                // logical output/cardinality/clone obligations remain below.
                context.charge(
                    add(
                        add(self.lookup_work, self.binding_work)?,
                        add(
                            add(element.lookup_work, element.binding_work)?,
                            add(
                                shape_bytes,
                                table_bytes
                                    .checked_mul(8)
                                    .ok_or(InvokeError::InvalidInput)?,
                            )?,
                        )?,
                    )?,
                    0,
                )?;
                let Some(epoch) = next_epoch(&NEXT_LOAN) else {
                    return Ok(None);
                };
                let mut source_meter = ResidentBudgetMeter::for_native_projection(work, limits)?;
                let source_footprint = budget::measure_snapshot_access_source_footprint(
                    &mut source_meter,
                    source,
                    &self.schemas,
                )?;
                let shape = mech_core::shape_for_resolved_extents(
                    self.template.output(),
                    &[1, entries.len() as u64],
                )
                .map_err(|_| InvokeError::InvalidInput)?;
                let mut payload = 0u64;
                for entry in entries {
                    let (ValueData::String(text), ValueData::U64(_)) =
                        (entry.key().data(), entry.value())
                    else {
                        return Err(InvokeError::InvalidInput);
                    };
                    payload = add(payload, text.len() as u64)?;
                }
                let logical = crate::intrinsics::map_ops::prospective(
                    crate::intrinsics::map_ops::Operation::Keys,
                    entries.len() as u64,
                    payload,
                    shape.parameter_values().len(),
                )
                .map_err(|_| InvokeError::InvalidInput)?;
                cap_error(
                    self.template.origin().node(),
                    ResourceDemand {
                        output_elements: add(coexistence.output_elements, entries.len() as u64)?,
                        cloned_bytes: add(coexistence.cloned_bytes, payload)?,
                        ..ResourceDemand::default()
                    },
                    logical.retained_bytes,
                    limits,
                )?;
                let mut selector_meter =
                    ResidentBudgetMeter::for_closed_analysis(u64::MAX, u64::MAX);
                let selector_footprint = budget::measure_native_scalar_frame_footprint(
                    &mut selector_meter,
                    element.template.inputs()[1].body(),
                    NativeScalarValue::U64(1),
                )?;
                let selector_cost = selector_meter.estimate();
                let Some(mut cells) = allocate_cells(entries.len()) else {
                    return Ok(None);
                };
                for (ordinal, entry) in entries.iter().enumerate() {
                    let ValueData::String(text) = entry.key().data() else {
                        return Err(InvokeError::InvalidInput);
                    };
                    cells[ordinal].write(packed_key_cell(
                        SequenceView::String(std::slice::from_ref(text)),
                        0,
                        text,
                        selector_footprint,
                        selector_cost,
                    )?);
                }
                // SAFETY: every cell in the exact fixed allocation was initialized.
                let cells = unsafe { cells.assume_init() };
                Ok(Some(NativeBorrowedMapKeys {
                    loan: NativeStringMatrixKeyLoan {
                        cells,
                        owner: source.clone(),
                        work: work.clone(),
                        producer: element.template.origin().node(),
                        key: element.output_key,
                        rows: 1,
                        columns: entries.len(),
                        epoch,
                        serial: 0,
                        selector_compute_work: selector_cost.compute_work(),
                        selector_comparison_work: selector_cost.comparison_work(),
                        source_footprint,
                        metadata_bytes,
                        metadata_nodes,
                        _reservation: reservation,
                        _nodes: nodes,
                    },
                    schema: contract.outputs[0].schema,
                    schema_key: self.output_key,
                    shape,
                    logical,
                    source_factory: self.cache_identity.clone(),
                    source_limits: limits,
                    source_spent: {
                        let used = work.used();
                        (
                            used.0
                                .checked_sub(source_start.0)
                                .ok_or(InvokeError::InvalidInput)?,
                            used.1
                                .checked_sub(source_start.1)
                                .ok_or(InvokeError::InvalidInput)?,
                        )
                    },
                }))
            },
        )?
    }
}
impl NativePreparedLibraryCall {
    /// ONE original selected producer visit. Immediate refusal and canonical
    /// construction share its existing call cap and cumulative account.
    #[cfg(feature = "map_ops")]
    pub fn borrowed_map_keys_or_canonical(
        &self,
        element: &NativePreparedLibraryCall,
        source: &Value,
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
        coexistence: ResourceDemand,
    ) -> Result<NativeBorrowedMapKeysChoice, InvokeError> {
        finite_limits(self.template.origin().node(), limits).map_err(InvokeError::Binding)?;
        if self.template.primitive() != Primitive::StringMapKeys {
            return Err(InvokeError::InvalidInput);
        }
        let enclosing = work.has_active_call_limit()?;
        let started = work.used();
        work.with_call_limits(
            limits.max_compute_work.unwrap(),
            limits.max_comparison_work.unwrap(),
            || {
                // Nested callers retain their original ordinary path: an enclosing
                // relative cap is not a suspendable permission after its guard ends.
                if !enclosing
                    && let Some(view) = self.try_borrowed_map_keys_in_call(
                        element,
                        source,
                        limits,
                        work,
                        coexistence,
                        started,
                    )?
                {
                    return Ok(NativeBorrowedMapKeysChoice::View(view));
                }
                let mut output = None;
                self.invoke_current(&[Some(source.clone())], &mut output, limits, work)?;
                Ok(NativeBorrowedMapKeysChoice::Canonical(
                    output.ok_or(InvokeError::InvalidInput)?,
                ))
            },
        )?
    }

    /// Materialize a previously admitted logical output, not a second source
    /// visit. Consumption is one-shot. All original logical caps remain, and
    /// only the original producer's unspent relative work allowance is usable.
    /// Intervening calls still consume the SAME cumulative invocation account.
    #[cfg(feature = "map_ops")]
    pub fn materialize_borrowed_map_keys(
        &self,
        keys: NativeBorrowedMapKeys,
        work: &NativeLibraryWorkBudget,
    ) -> Result<Value, InvokeError> {
        keys.loan.current(&keys.loan.owner, work)?;
        if self.template.primitive() != Primitive::StringMapKeys
            || !std::sync::Arc::ptr_eq(&self.cache_identity, &keys.source_factory)
            || self.output_key != keys.schema_key
        {
            return Err(InvokeError::InvalidInput);
        }
        let mut limits = keys.source_limits;
        limits.max_compute_work = Some(
            limits
                .max_compute_work
                .ok_or(InvokeError::InvalidInput)?
                .checked_sub(keys.source_spent.0)
                .ok_or(InvokeError::InvalidInput)?,
        );
        limits.max_comparison_work = Some(
            limits
                .max_comparison_work
                .ok_or(InvokeError::InvalidInput)?
                .checked_sub(keys.source_spent.1)
                .ok_or(InvokeError::InvalidInput)?,
        );
        let source = keys.loan.owner.clone();
        // Release the optional table, shape and metadata claims before the
        // ordinary allocation. The immutable source pin remains a real owner.
        drop(keys);
        let mut output = None;
        self.invoke_current(&[Some(source)], &mut output, limits, work)?;
        output.ok_or(InvokeError::InvalidInput)
    }
}

impl NativePreparedLibraryCall {
    /// Selected sequence length for this sealed row-vector view. Its genuine
    /// key snapshot stays pinned even after an unrelated arena retires a handle.
    pub fn borrowed_map_keys_length(
        &self,
        keys: &NativeBorrowedMapKeys,
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
        coexistence: ResourceDemand,
    ) -> Result<u64, InvokeError> {
        finite_limits(self.template.origin().node(), limits).map_err(InvokeError::Binding)?;
        if !self.supports_matrix_length() || self.input_keys.first() != Some(&keys.schema_key) {
            return Err(InvokeError::InvalidInput);
        }
        work.with_call_limits(
            limits.max_compute_work.unwrap(),
            limits.max_comparison_work.unwrap(),
            || {
                cap_error(
                    self.template.origin().node(),
                    ResourceDemand {
                        output_elements: 1,
                        ..ResourceDemand::default()
                    },
                    8,
                    limits,
                )?;
                keys.loan.current(&keys.loan.owner, work)?;
                let memory = work.memory_budget();
                let bytes = keys
                    .loan
                    .owner
                    .memory_budget_retained_bytes(&memory)
                    .ok_or(InvokeError::UnadmittedValue)?;
                let nodes = keys
                    .loan
                    .owner
                    .memory_budget_retained_nodes(&memory)
                    .ok_or(InvokeError::UnadmittedValue)?;
                let context = NativeProjectionIndexContext::new(work, limits, coexistence)
                    .with_added_capacity(
                        add(add(bytes, keys.loan.metadata_bytes)?, 8)?,
                        add(add(nodes, keys.loan.metadata_nodes)?, 1)?,
                    )?;
                context.check_capacity(0, 0)?;
                context.charge(add(self.lookup_work, 128)?, 1)?;
                crate::resident::pure_text::matrix_length(1, keys.loan.cells.len())
                    .map_err(InvokeError::Kernel)
            },
        )?
    }
}
impl NativeBorrowedMapKeys {
    pub fn snapshot(&self) -> &Value {
        &self.loan.owner
    }
    pub fn schema(&self) -> SchemaId {
        self.schema
    }
    pub fn schema_key(&self) -> SchemaKey {
        self.schema_key
    }
    pub fn shape(&self) -> &ShapeInstance {
        &self.shape
    }
    pub fn logical_footprint(&self) -> ValueFootprint {
        self.logical
    }
    pub fn retained_metadata_demand(&self) -> ResourceDemand {
        self.loan.retained_metadata_demand()
    }
    pub fn native_view(
        &self,
        work: &NativeLibraryWorkBudget,
    ) -> Result<NativeStringMatrixKeyView, InvokeError> {
        self.loan.native_view(&self.loan.owner, work)
    }
    pub fn admit_element(
        &mut self,
        index: u64,
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
        coexistence: ResourceDemand,
    ) -> Result<NativeStringMatrixKeyOrdinal, InvokeError> {
        // The view owns this immutable snapshot across private-frame retirement.
        // The clone is a bounded stack header; no canonical matrix is fabricated.
        let source = self.loan.owner.clone();
        self.loan
            .admit_element(&source, index, limits, work, coexistence)
    }
    pub fn with_element<T>(
        &self,
        consumer: &NativePreparedLibraryCall,
        selected: &NativeStringMatrixKeyOrdinal,
        work: &NativeLibraryWorkBudget,
        use_key: impl FnOnce(&str) -> T,
    ) -> Result<T, InvokeError> {
        self.loan
            .with_element(consumer, &self.loan.owner, selected, work, use_key)
    }
    /// Observe the canonical key ordinal only after the same current-loan,
    /// source-account and selected-consumer checks as the borrowed text.
    /// The ordinal is descriptive; it grants no mutation or call allowance.
    pub fn with_element_index<T>(
        &self,
        consumer: &NativePreparedLibraryCall,
        selected: &NativeStringMatrixKeyOrdinal,
        work: &NativeLibraryWorkBudget,
        use_key: impl FnOnce(usize, &str) -> T,
    ) -> Result<T, InvokeError> {
        self.with_element(consumer, selected, work, |text| {
            use_key(selected.ordinal, text)
        })
    }
    pub(in crate::resident) fn validate_frame_child(
        &self,
        call: &NativePreparedLibraryCall,
        index: usize,
        work: &NativeLibraryWorkBudget,
    ) -> Result<(), InvokeError> {
        self.loan.current(&self.loan.owner, work)?;
        let mech_core::ResolvedOperationContract::Declared(contract) = call.template.contract()
        else {
            return Err(InvokeError::InvalidInput);
        };
        if contract.inputs.get(index).map(|p| p.schema) != Some(self.schema)
            || call.input_keys.get(index) != Some(&self.schema_key)
        {
            return Err(InvokeError::InvalidInput);
        }
        Ok(())
    }
}

#[cfg(all(
    test,
    feature = "map_ops",
    any(feature = "full_source", feature = "full_compiler")
))]
#[path = "native_collection_borrowed_map_keys_tests.rs"]
mod tests;
