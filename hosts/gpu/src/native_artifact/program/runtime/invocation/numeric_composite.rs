//! Publish a selected numeric tuple/record without generic turn planning.
//! The ordinary selected call remains available on every unadmitted attempt.
use super::*;
use mech_core::SchemaBody;
use mech_engine::native_artifact::{NativeIterationAction as Action, NativeScalarValue};

fn fits(value: u64, cap: Option<u64>) -> bool {
    cap.is_some_and(|limit| value <= limit)
}

// The current canonical owner already has its schema/shape witness. Observe
// only plain numeric cardinality here; constructor binding rechecks identities.
fn matrix_cells(value: &Value, schema: &mech_core::Schema) -> Option<u64> {
    let (SchemaBody::Matrix { dimensions, .. }, mech_core::ValueData::Matrix(matrix)) =
        (schema.body(), value.data())
    else {
        return None;
    };
    let [row, column] = dimensions.as_ref() else {
        return None;
    };
    let rows = value.shape().resolve_dimension(row).ok()?;
    let columns = value.shape().resolve_dimension(column).ok()?;
    // Retain the maintained resident factory's physical dimension domain.
    u32::try_from(rows).ok()?;
    u32::try_from(columns).ok()?;
    let cells = rows.checked_mul(columns)?;
    (cells == matrix.elements().len() as u64).then_some(cells)
}

impl Invocation<'_, '_, '_> {
    #[inline(never)]
    pub(super) fn numeric_composite_call(
        &mut self,
        site: usize,
        words: &[u64],
    ) -> Result<bool, Error> {
        let metadata = self.descriptor.metadata();
        let Action::LibraryCall {
            operands, output, ..
        } = metadata.plan.blocks()[site].action()
        else {
            return Err(invalid("numeric composite outside selected library action"));
        };
        let call = metadata.calls[site]
            .as_ref()
            .ok_or_else(|| invalid("numeric composite lacks selected factory"))?;
        if !call.supports_canonical_numeric_pack()
            || words.len() != operands.len()
            || metadata.plan.values()[output.get() as usize]
                .scalar_kind()
                .is_some()
            || self.state.consumed
            || self.state.control.previous != Some(site as u32)
        {
            return Err(invalid("numeric composite source selection differs"));
        }
        if self.worker.is_some()
            || self.state.scalar_path.is_some()
            || self.has_uninventoried_private_owners()
        {
            return Ok(false);
        }
        let memory = self.state.library.memory_budget();
        let mut inputs = 0u64;
        for (index, operand) in operands.iter().enumerate() {
            let ty = &metadata.plan.values()[operand.get() as usize];
            let cells = if let Some(kind) = ty.scalar_kind() {
                if NativeScalarValue::from_bits(kind, words[index]).is_none() {
                    return Ok(false);
                }
                1
            } else {
                if words[index] != 0 {
                    return Err(invalid("managed operand has a native payload"));
                }
                let Some(value) = self
                    .state
                    .arena
                    .native_entry_preview(operand.get(), &self.state.control.work)?
                else {
                    return Ok(false);
                };
                // Reuse the independently validated key at this source value
                // index; current identity and account checks still run here.
                if value.schema() != ty.schema()
                    || value.schema_key() != metadata.types[operand.get() as usize].key
                    || value.memory_budget_retained_bytes(&memory).is_none()
                {
                    return Ok(false);
                }
                let Some(cells) = matrix_cells(value, ty.definition()) else {
                    return Ok(false);
                };
                cells
            };
            let Some(count) = inputs.checked_add(cells).and_then(|n| n.checked_add(1)) else {
                return Ok(false);
            };
            inputs = count;
        }
        let Some(normalization) = call.canonical_numeric_pack_normalization() else {
            return Ok(false);
        };
        let Some((mut demand, replay)) = call.canonical_numeric_pack_source_bound(inputs, inputs)
        else {
            return Ok(false);
        };
        let Some(children_nodes) = (operands.len() as u64).checked_mul(2) else {
            return Ok(false);
        };
        let Some(metadata_nodes) = normalization
            .metadata_nodes
            .checked_add(children_nodes)
            .and_then(|n| n.checked_add(8))
        else {
            return Ok(false);
        };
        let Some(peak) = demand
            .turn_peak_bytes
            .checked_add(normalization.temporary_bytes)
        else {
            return Ok(false);
        };
        let Some(nodes) = demand.retained_nodes.checked_add(metadata_nodes) else {
            return Ok(false);
        };
        let Some(extra_work) = normalization
            .temporary_bytes
            .checked_mul(8)
            .and_then(|n| n.checked_add(normalization.compute_work))
        else {
            return Ok(false);
        };
        let Some(compute) = demand.work.compute.checked_add(extra_work) else {
            return Ok(false);
        };
        demand.turn_peak_bytes = peak;
        demand.retained_nodes = nodes;
        demand.work.compute = compute;
        let Some(credit) = compute
            .checked_add(replay)
            .and_then(|n| n.checked_add(extra_work.checked_mul(128)?))
        else {
            return Ok(false);
        };
        let Some(comparison_credit) = demand.work.comparison.checked_mul(2) else {
            return Ok(false);
        };
        let caps = self.limits;
        if !fits(demand.output_elements, caps.max_output_elements)
            || !fits(demand.persistent_bytes, caps.max_output_bytes)
            || !fits(peak, caps.max_temporary_bytes)
            || !fits(demand.cloned_bytes, caps.max_cloned_bytes)
            || !fits(nodes, caps.max_retained_nodes)
            || !fits(compute, caps.max_compute_work)
            || !fits(demand.work.comparison, caps.max_comparison_work)
            || !fits(0, caps.max_scalar_instructions)
            || !fits(0, caps.max_transfer_bytes)
            || !fits(0, caps.max_storage_buffer_bytes)
            || caps.max_storage_bindings.is_none()
            || self
                .state
                .control
                .work
                .limit()
                .saturating_sub(self.state.control.work.used())
                < credit
            || self
                .state
                .library
                .unscoped_remaining()
                .is_none_or(|(w, c)| w < credit || c < comparison_credit)
        {
            return Ok(false);
        }
        let Some(prior) = self.retained_caller_geometry(site)? else {
            return Ok(false);
        };
        if prior
            .live_bytes
            .checked_add(peak)
            .is_none_or(|n| !fits(n, caps.max_temporary_bytes))
            || prior
                .live_nodes
                .checked_add(nodes)
                .is_none_or(|n| !fits(n, caps.max_retained_nodes))
        {
            return Ok(false);
        }
        let mut reservation = match memory.reserve_capacity(peak) {
            Ok(v) => v,
            Err(mech_core::MemoryRuntimeError::BudgetExceeded { .. }) => return Ok(false),
            Err(error) => return Err(error.into()),
        };
        let probe = match memory.reserve_nodes(nodes) {
            Ok(v) => v,
            Err(mech_core::MemoryRuntimeError::BudgetExceeded { .. }) => return Ok(false),
            Err(error) => return Err(error.into()),
        };
        drop(probe);
        // The complete coexistence probe includes canonical publication. Hold
        // the actual temporary metadata population while the output is built.
        let _nodes = memory.reserve_nodes(metadata_nodes)?;
        self.state.control.work.charge(
            crate::native_artifact::NativeExecutionPhase::Node(
                metadata.plan.blocks()[site].origin().node(),
            ),
            compute,
        )?;
        self.state
            .library
            .charge(0, demand.work.comparison)
            .map_err(|error| Error::Collection {
                origin: metadata.plan.blocks()[site].origin(),
                error,
            })?;
        let mut children = Vec::with_capacity(operands.len());
        for (index, &operand) in operands.iter().enumerate() {
            let value = if metadata.plan.values()[operand.get() as usize]
                .scalar_kind()
                .is_some()
            {
                self.scalar(operand, words[index])?
            } else {
                self.state
                    .arena
                    .copy_current(operand.get(), &mut self.state.control.work)?
            };
            if value.validate_against(&metadata.schemas).is_err() {
                return Ok(false);
            }
            children.push(value);
        }
        // These are current invocation shapes, never retained input samples.
        // Every miss releases the attempt and returns to the original call;
        // the unchanged full source envelope above includes ordinary replay.
        let Ok(constructor) = call.bind_canonical_numeric_pack(children.len(), |index| {
            children
                .get(index)
                .map(|value| (value.schema(), value.shape()))
        }) else {
            return Ok(false);
        };
        let Ok(value) = constructor.construct(children.into_boxed_slice(), None) else {
            return Ok(false);
        };
        drop(constructor);
        let value = value.into_memory_budget(&mut reservation)?;
        if value.schema_key() != metadata.types[output.get() as usize].key {
            return Err(invalid("numeric composite output schema differs"));
        }
        self.state
            .arena
            .put(output.get(), &value, &mut self.state.control.work)?;
        self.state.consumed = true;
        Ok(true)
    }
}
