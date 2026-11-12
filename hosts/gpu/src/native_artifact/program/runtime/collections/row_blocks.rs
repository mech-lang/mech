//! Bounded row-block transport. Generated source bodies produce each block;
//! this boundary appends canonical scalar storage and executes no source CFG.
use super::*;
use mech_core::snapshot::SequenceView;
use mech_engine::native_artifact::NativeMatchValue;

#[cfg(test)]
thread_local! {
    static FORCE_GENERIC_PAYLOAD: core::cell::Cell<bool> = const { core::cell::Cell::new(false) };
    static PACKED_PAYLOADS: core::cell::Cell<u64> = const { core::cell::Cell::new(0) };
}

/// Only canonical packed banks whose scalar payload width is fixed qualify.
/// This replaces a payload-size pass, not a capacity/source/account check or
/// any debit. Text and generic cells retain their checked element traversal.
fn payload_bytes(value: &Value, additions: u64) -> Result<u64, Error> {
    #[cfg(test)]
    let packed = !FORCE_GENERIC_PAYLOAD.with(core::cell::Cell::get);
    #[cfg(not(test))]
    let packed = true;
    if packed {
        if let ValueData::Matrix(matrix) = value.data() {
            let width = match matrix.elements() {
                SequenceView::Bool(_) | SequenceView::U8(_) => Some(1),
                SequenceView::U16(_) => Some(2),
                SequenceView::U32(_) => Some(4),
                SequenceView::U64(_)
                | SequenceView::I64(_)
                | SequenceView::F64(_)
                | SequenceView::Index(_) => Some(8),
                _ => None,
            };
            if let Some(width) = width {
                #[cfg(test)]
                PACKED_PAYLOADS.with(|count| count.set(count.get() + 1));
                return mul(additions, width);
            }
        }
    }
    let mut bytes = 0;
    for ordinal in 0..additions {
        bytes = add(bytes, data::element(value, ordinal, 0)?.0.bytes())?;
    }
    Ok(bytes)
}

#[cfg(all(test, feature = "canonical-native-program"))]
mod tests;

impl State {
    pub(super) fn push_row_block(
        &mut self,
        ctx: &mut CollectionExecution<'_, '_, '_, '_>,
        id: u32,
        slot: NativeMatchValue,
    ) -> Result<(), Error> {
        let descriptor = ctx.descriptor();
        let collection = &descriptor.metadata().plan.collections()[id as usize];
        let ty = &descriptor.metadata().plan.values()[slot.get() as usize];
        let value = ctx.arena_copy_current(slot.get())?;
        let additions = data::count(&value)?;
        let retained = value
            .memory_budget_retained_bytes(&ctx.library().memory_budget())
            .ok_or_else(|| invalid("row block is not admitted to the shared account"))?;
        // Fund shape/alias checks and the complete payload-size pass before
        // touching elements. The canonical retained bound includes all text.
        self.account(
            ctx,
            id,
            0,
            add(
                add(mul(additions, 64)?, mul(retained, 8)?)?,
                add(mul(self.sources.len() as u64, 64)?, 256)?,
            )?,
        )?;
        let mech_core::SchemaBody::Matrix { dimensions, .. } = ty.definition().body() else {
            return Err(invalid("row block has a non-matrix schema"));
        };
        if dimensions.len() != 2
            || value
                .shape()
                .resolve_dimension(&dimensions[0])
                .map_err(|_| invalid("row block rows"))?
                != 1
        {
            return Err(invalid("native row block must have exactly one row"));
        }
        let payload = payload_bytes(&value, additions)?;
        self.account(
            ctx,
            id,
            payload,
            add(mul(payload, 16)?, mul(additions, 64)?)?,
        )?;
        let untracked = self.untracked_yield_bytes(ctx, Some(&value))?;
        let old_live = self.live_bytes;
        let retained_nodes = self.retained_nodes()?;
        let origin = collection.origin();
        let output = &descriptor.metadata().plan.values()[collection.output().get() as usize];
        let builder = self.builder(id)?;
        let additions =
            usize::try_from(additions).map_err(|_| invalid("row block count exceeds usize"))?;
        let count = builder
            .values
            .len()
            .checked_add(additions)
            .ok_or_else(|| invalid("row-block output count overflow"))?;
        let capacity = if count > builder.values.capacity() {
            builder
                .values
                .capacity()
                .checked_mul(2)
                .map(|twice| count.max(twice).max(4))
                .ok_or_else(|| invalid("row-block capacity overflow"))?
        } else {
            builder.values.capacity()
        };
        let payload_bytes = add(builder.payload_bytes, payload)?;
        let backing = builder.values.backing_bytes(capacity, payload_bytes)?;
        let growing = capacity > builder.values.capacity();
        let growth_scratch = if growing {
            mul(
                builder.values.capacity() as u64,
                builder.values.element_bytes(),
            )?
        } else {
            0
        };
        let extra = backing
            .checked_sub(builder.reservation.capacity_bytes())
            .ok_or_else(|| invalid("row-block reservation moved backwards"))?;
        check(
            origin,
            Dimension::OutputElements,
            count as u64,
            ctx.limits().max_output_elements,
        )?;
        check(
            origin,
            Dimension::OutputBytes,
            builder.values.output_bytes(
                payload_bytes,
                count as u64,
                capacity,
                output.definition().dimension_parameters().len(),
            )?,
            ctx.limits().max_output_bytes,
        )?;
        check(
            origin,
            Dimension::RetainedNodes,
            add(
                data::output_nodes(output.definition().body(), count as u64)?,
                retained_nodes,
            )?,
            ctx.limits().max_retained_nodes,
        )?;
        check(
            origin,
            Dimension::TemporaryBytes,
            add(add(add(old_live, extra)?, untracked)?, growth_scratch)?,
            ctx.limits().max_temporary_bytes,
        )?;
        charge(
            &mut ctx.work(),
            add(add(mul(extra, 16)?, mul(growth_scratch, 4)?)?, 128)?,
        )?;
        let memory = ctx.library().memory_budget();
        let pending_nodes = memory.reserve_nodes(if builder.values.packed_width().is_none() {
            additions as u64
        } else {
            0
        })?;
        let _replacement = memory.reserve_nodes(u64::from(growing))?;
        let _growth = (growth_scratch != 0)
            .then(|| memory.reserve_capacity(growth_scratch))
            .transpose()?;
        builder.reservation.resize_capacity(backing)?;
        if growing {
            builder.values.reserve(capacity)?;
        }
        builder.values.append_row(&value)?;
        builder.nodes.merge(pending_nodes)?;
        builder.payload_bytes = payload_bytes;
        self.live_bytes = add(old_live, extra)?;
        Ok(())
    }
}

impl data::CollectedValues {
    fn append_row(&mut self, value: &Value) -> Result<(), Error> {
        let ValueData::Matrix(matrix) = value.data() else {
            return Err(invalid("row-block payload is not a matrix"));
        };
        let available = self
            .capacity()
            .checked_sub(self.len())
            .ok_or_else(|| invalid("row-block capacity precedes length"))?;
        if matrix.elements().len() > available {
            return Err(invalid("row-block copy exceeds admitted capacity"));
        }
        // Exact-width numeric banks copy directly, without drafts or callbacks
        // per element. Other maintained scalar banks use admitted draft slots.
        match (&mut *self, matrix.elements()) {
            (Self::U8(to), SequenceView::U8(from)) => to.extend_from_slice(from),
            (Self::U16(to), SequenceView::U16(from)) => to.extend_from_slice(from),
            (Self::U32(to), SequenceView::U32(from)) => to.extend_from_slice(from),
            (Self::U64(to), SequenceView::U64(from)) => to.extend_from_slice(from),
            _ => {
                for ordinal in 0..matrix.elements().len() as u64 {
                    let data = match data::element(value, ordinal, 0)?.0 {
                        data::Element::Scalar(NativeScalarKind::Bool, x) => {
                            ValueDataDraft::Bool(x != 0)
                        }
                        data::Element::Scalar(NativeScalarKind::U8, x) => {
                            ValueDataDraft::U8(x as u8)
                        }
                        data::Element::Scalar(NativeScalarKind::U16, x) => {
                            ValueDataDraft::U16(x as u16)
                        }
                        data::Element::Scalar(NativeScalarKind::U32, x) => {
                            ValueDataDraft::U32(x as u32)
                        }
                        data::Element::Scalar(NativeScalarKind::U64, x) => ValueDataDraft::U64(x),
                        data::Element::Scalar(NativeScalarKind::I64, x) => {
                            ValueDataDraft::I64(x as i64)
                        }
                        data::Element::Scalar(NativeScalarKind::F64, x) => {
                            ValueDataDraft::F64(mech_core::snapshot::F64Bits::from_bits(x))
                        }
                        data::Element::Index(x) => ValueDataDraft::Index(x),
                        data::Element::Text(x) => ValueDataDraft::String(x.to_owned()),
                    };
                    self.push(data)?;
                }
            }
        }
        Ok(())
    }
}
