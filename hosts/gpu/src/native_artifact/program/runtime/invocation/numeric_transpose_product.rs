//! One source-proved transpose consumed by its sole ordered product.
//! Geometry and complete old source obligations are admitted before mutation.
use super::*;
use crate::native_artifact::iteration_abi::Progress;
use mech_engine::native_artifact::{NativeIterationAction as Action, NativeMatchAction as Scalar};

impl Invocation<'_, '_, '_> {
    pub(super) fn prepare_transpose_product(
        &mut self,
        site: usize,
        args: [u64; 3],
    ) -> Result<Option<u64>, Error> {
        let descriptor = self.descriptor;
        let m = descriptor.metadata();
        let view = m
            .numeric_binary
            .product_view(site)
            .ok_or_else(|| invalid("transpose/product outside its source proof"))?;
        if args != [self.arguments as u64, 2, 0]
            || self.argument_capacity < 2
            || self.arguments.is_null()
            || self.state.consumed
            || self.numeric_binary.is_some()
            || self.state.control.previous != Some(site as u32)
            || self.state.control.current != view.parent
        {
            return Err(invalid("transpose/product outside its fresh exact entry"));
        }
        if self.worker.is_some()
            || self.state.scalar_path.is_some()
            || self.numeric_prepaid.is_some()
            || self.has_uninventoried_private_owners()
            || std::iter::once(site as u32)
                .chain(view.continuation.iter().copied())
                .any(|at| {
                    let visit = &m.visits[at as usize];
                    visit.progress != Progress::Normal
                        || visit.complete
                        || visit.arithmetic
                        || visit.reject
                        || visit.integrity
                        || m.map_admission.group(at as usize).is_some()
                })
        {
            return Ok(Some(0));
        }
        // The same session-owned disjoint initialized managed argument lane.
        let words = unsafe { core::slice::from_raw_parts(self.arguments, 2) };
        if words != [0, 0] {
            return Err(invalid("transpose/product managed inputs have native bits"));
        }
        let memory = self.state.library.memory_budget();
        let mut dimensions = [[0usize; 2]; 2];
        for (at, &id) in view.operands.iter().enumerate() {
            let ty = &m.plan.values()[id as usize];
            let Some(value) = self
                .state
                .arena
                .native_entry_preview(id, &self.state.control.work)?
            else {
                return Ok(Some(0));
            };
            if value.schema() != ty.schema()
                || value.schema_key() != ty.schema_key()
                || value.memory_budget_retained_bytes(&memory).is_none()
            {
                return Ok(Some(0));
            }
            let Some(axes) = dims(value, ty.definition()) else {
                return Ok(Some(0));
            };
            // Preserve the ordinary product's NaN payload/fallback path. The
            // transpose itself is exact-bit and is replayed normally on refusal.
            if cells(value).is_none_or(|cells| cells.iter().any(|cell| cell.to_f64().is_nan())) {
                return Ok(Some(0));
            }
            dimensions[at] = axes;
        }
        let source_dimensions = dimensions[view.transposed_operand];
        let transpose = m.calls[site]
            .as_ref()
            .ok_or_else(|| invalid("transpose/product producer factory missing"))?;
        let consumer = m.calls[view.product as usize]
            .as_ref()
            .ok_or_else(|| invalid("transpose/product consumer factory missing"))?;
        let Some([tr, tc, transpose_count]) =
            transpose.f64_matrix_transpose_geometry(source_dimensions)
        else {
            return Ok(Some(0));
        };
        let Ok(tr_usize) = usize::try_from(tr) else {
            return Ok(Some(0));
        };
        let Ok(tc_usize) = usize::try_from(tc) else {
            return Ok(Some(0));
        };
        dimensions[view.transposed_operand] = [tr_usize, tc_usize];
        let Some([rows, columns, inner]) =
            consumer.f64_matrix_product_geometry(dimensions[0], dimensions[1])
        else {
            return Ok(Some(0));
        };
        let Some(count) = rows.checked_mul(columns) else {
            return Ok(Some(0));
        };
        let Some(input_count) = dimensions.iter().try_fold(0u64, |sum, axes| {
            sum.checked_add((axes[0] as u64).checked_mul(axes[1] as u64)?)
        }) else {
            return Ok(Some(0));
        };
        let Some((producer_demand, producer_replay)) =
            transpose.f64_matrix_transpose_source_bound(transpose_count)
        else {
            return Ok(Some(0));
        };
        let Some((consumer_demand, consumer_replay)) =
            consumer.f64_matrix_product_source_bound(input_count, count, inner)
        else {
            return Ok(Some(0));
        };
        let producer = m
            .numeric_binary
            .at(site)
            .ok_or_else(|| invalid("transpose/product source operation missing"))?;
        for definition in [
            transpose.template().output(),
            m.plan.values()[producer.output as usize].definition(),
        ] {
            if mech_core::shape_for_resolved_extents(definition, &[tr, tc]).is_err() {
                return Ok(Some(0));
            }
        }
        if mech_core::shape_for_resolved_extents(consumer.template().output(), &[rows, columns])
            .is_err()
        {
            return Ok(Some(0));
        }
        let Ok(shape) = mech_core::shape_for_resolved_extents(
            m.plan.values()[view.output as usize].definition(),
            &[rows, columns],
        ) else {
            return Ok(Some(0));
        };
        let Some(visit_work) = view
            .continuation
            .iter()
            .try_fold(0u64, |sum, &at| sum.checked_add(m.visits[at as usize].work))
        else {
            return Ok(Some(0));
        };
        let copies = view
            .continuation
            .iter()
            .filter(|&&at| {
                matches!(
                    m.plan.blocks()[at as usize].action(),
                    Action::Scalar(Scalar::Copy { .. })
                )
            })
            .count() as u64;
        // Source copy_current is32; put costs64 plus four times the complete
        // retained owner bound. Copies between the two operations cost96.
        // Charge these omitted source obligations using the genuine selected
        // producer's persistent bound. No intermediate owner is constructed.
        let Some(elided_owners) = producer_demand
            .persistent_bytes
            .checked_mul(4)
            .and_then(|n| n.checked_add(96))
            .and_then(|n| {
                copies
                    .checked_mul(96)
                    .and_then(|copies| n.checked_add(copies))
            })
        else {
            return Ok(Some(0));
        };
        let Some(compute) = producer_demand
            .work
            .compute
            .checked_add(consumer_demand.work.compute)
            .and_then(|n| n.checked_add(visit_work))
            .and_then(|n| n.checked_add(elided_owners))
        else {
            return Ok(Some(0));
        };
        let Some(comparison) = producer_demand
            .work
            .comparison
            .checked_add(consumer_demand.work.comparison)
        else {
            return Ok(Some(0));
        };
        let Some(credit) = compute
            .checked_add(producer_replay)
            .and_then(|n| n.checked_add(consumer_replay))
        else {
            return Ok(Some(0));
        };
        let Some(comparison_credit) = comparison.checked_mul(2) else {
            return Ok(Some(0));
        };
        let Some(temporary) = producer_demand
            .turn_peak_bytes
            .checked_add(consumer_demand.turn_peak_bytes)
            .and_then(|n| n.checked_add(producer_demand.persistent_bytes))
        else {
            return Ok(Some(0));
        };
        let Some(nodes) = producer_demand
            .retained_nodes
            .checked_add(consumer_demand.retained_nodes)
        else {
            return Ok(Some(0));
        };
        let Some(cloned) = producer_demand
            .cloned_bytes
            .checked_add(consumer_demand.cloned_bytes)
        else {
            return Ok(Some(0));
        };
        let caps = self.limits;
        if !fits(count.max(transpose_count), caps.max_output_elements)
            || !fits(
                producer_demand
                    .persistent_bytes
                    .max(consumer_demand.persistent_bytes),
                caps.max_output_bytes,
            )
            || !fits(temporary, caps.max_temporary_bytes)
            || !fits(nodes, caps.max_retained_nodes)
            || !fits(cloned, caps.max_cloned_bytes)
            || !fits(compute, caps.max_compute_work)
            || !fits(comparison, caps.max_comparison_work)
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
            return Ok(Some(0));
        }
        let Some(prior) = self.retained_caller_geometry(site)? else {
            return Ok(Some(0));
        };
        if prior
            .live_bytes
            .checked_add(temporary)
            .is_none_or(|n| !fits(n, caps.max_temporary_bytes))
            || prior
                .live_nodes
                .checked_add(nodes)
                .is_none_or(|n| !fits(n, caps.max_retained_nodes))
        {
            return Ok(Some(0));
        }
        let reservation = match memory.reserve_capacity(temporary) {
            Ok(value) => value,
            Err(mech_core::MemoryRuntimeError::BudgetExceeded { .. }) => return Ok(Some(0)),
            Err(error) => return Err(error.into()),
        };
        let node_probe = match memory.reserve_nodes(nodes) {
            Ok(value) => value,
            Err(mech_core::MemoryRuntimeError::BudgetExceeded { .. }) => return Ok(Some(0)),
            Err(error) => return Err(error.into()),
        };
        drop(node_probe);
        let nodes = memory.reserve_nodes(3)?;
        let Ok(length) = usize::try_from(count) else {
            return Ok(Some(0));
        };
        if core::alloc::Layout::array::<F64Bits>(length).is_err() {
            return Ok(Some(0));
        }
        self.state.control.work.charge(
            crate::native_artifact::NativeExecutionPhase::Node(
                m.plan.blocks()[site].origin().node(),
            ),
            compute,
        )?;
        self.state
            .library
            .charge(0, comparison)
            .map_err(|error| Error::Collection {
                origin: m.plan.blocks()[site].origin(),
                error,
            })?;
        let mut inputs = [None, None];
        for (at, &id) in view.operands.iter().enumerate() {
            let value = self
                .state
                .arena
                .copy_current(id, &mut self.state.control.work)?;
            if value.validate_against(&m.schemas).is_err() {
                return Ok(Some(0));
            }
            inputs[at] = Some(value);
        }
        let mut pending = Box::new(Pending {
            frame: NumericBinaryFrame {
                left: 0,
                right: 0,
                output: 0,
                count,
                columns,
                inner,
                left_mode: 1,
                right_mode: 1,
                left_scalar: 0,
                right_scalar: 0,
            },
            issued: NumericBinaryFrame::default(),
            view_end: Some((view.product, self.state.control.visits)),
            site,
            output: view.output,
            inputs,
            cells: vec![F64Bits::from_bits(0); length],
            shape,
            reservation,
            _nodes: nodes,
        });
        pending.frame.left = cells(pending.inputs[0].as_ref().unwrap())
            .ok_or_else(|| invalid("transpose/product lost its left pin"))?
            .as_ptr() as u64;
        pending.frame.right = cells(pending.inputs[1].as_ref().unwrap())
            .ok_or_else(|| invalid("transpose/product lost its right pin"))?
            .as_ptr() as u64;
        pending.frame.output = pending.cells.as_mut_ptr() as u64;
        pending.issued = pending.frame;
        let pointer = &pending.frame as *const NumericBinaryFrame as u64;
        #[cfg(test)]
        {
            let mut intermediate_present = false;
            for &value in &view.elided_values {
                intermediate_present |= self
                    .state
                    .arena
                    .native_entry_preview(value, &self.state.control.work)?
                    .is_some();
            }
            super::super::super::composition_tests::observe::transpose_product(
                site,
                transpose_count,
                count,
                intermediate_present,
            );
        }
        self.numeric_binary = Some(pending);
        Ok(Some(pointer))
    }
}
