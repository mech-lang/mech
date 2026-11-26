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
    pub(super) fn initial_block_rows(
        &self,
        ctx: &mut CollectionExecution<'_, '_, '_, '_>,
        id: u32,
    ) -> Result<Option<u64>, Error> {
        use mech_engine::{ComprehensionKind as K, MatrixBlockRows as R};
        let descriptor = ctx.descriptor();
        Ok(
            match descriptor.metadata().plan.collections()[id as usize].kind() {
                K::MatrixHorizontalBlocks {
                    rows: R::Fixed(rows),
                } => Some(rows),
                K::MatrixHorizontalBlocks {
                    rows: R::InputMatrixElements { input },
                } => {
                    let source = descriptor
                        .metadata()
                        .plan
                        .inputs()
                        .get(input as usize)
                        .ok_or_else(|| invalid("block row input is absent"))?
                        .value();
                    Some(data::count(&ctx.arena_copy_current(source.get())?)?)
                }
                _ => None,
            },
        )
    }

    pub(super) fn finish_block_order(
        &mut self,
        ctx: &mut CollectionExecution<'_, '_, '_, '_>,
        id: u32,
    ) -> Result<Option<[u64; 2]>, Error> {
        let descriptor = ctx.descriptor();
        let definition = &descriptor.metadata().plan.collections()[id as usize];
        if !matches!(
            definition.kind(),
            mech_engine::ComprehensionKind::MatrixHorizontalBlocks { .. }
        ) {
            return Ok(None);
        }
        let builder = self.builder(id)?;
        let rows = builder.block_rows.unwrap_or(0);
        let columns = builder.block_columns;
        let count = builder.values.len() as u64;
        if mul(rows, columns)? != count {
            return Err(invalid("collected matrix block geometry differs"));
        }
        if rows > 1 && columns > 1 {
            // The admitted scalar bank is column-major. Reorder in place
            // using one byte per cycle mark; no scalar payload is cloned.
            self.account(ctx, id, 0, add(mul(count, 64)?, 256)?)?;
            check(
                definition.origin(),
                Dimension::TemporaryBytes,
                add(self.live_bytes, count)?,
                ctx.limits().max_temporary_bytes,
            )?;
            check(
                definition.origin(),
                Dimension::RetainedNodes,
                add(self.retained_nodes()?, 1)?,
                ctx.limits().max_retained_nodes,
            )?;
            let memory = ctx.library().memory_budget();
            let _nodes = memory.reserve_nodes(1)?;
            let _marks = memory.reserve_capacity(count)?;
            let count = usize::try_from(count).map_err(|_| invalid("block permutation count"))?;
            let mut visited = Vec::<u8>::new();
            visited
                .try_reserve_exact(count)
                .map_err(|_| invalid("matrix block permutation allocation"))?;
            let mut visited = admitted_capacity(visited, count, "matrix block permutation")?;
            visited.resize(count, 0);
            self.builder(id)?
                .values
                .row_major_order(rows as usize, columns as usize, &mut visited);
        }
        Ok(Some([rows, columns]))
    }

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
        if dimensions.len() != 2 {
            return Err(invalid("native matrix block must have rank two"));
        }
        let rows = value
            .shape()
            .resolve_dimension(&dimensions[0])
            .map_err(|_| invalid("matrix block rows"))?;
        let columns = value
            .shape()
            .resolve_dimension(&dimensions[1])
            .map_err(|_| invalid("matrix block columns"))?;
        if mul(rows, columns)? != additions {
            return Err(invalid("matrix block element count differs"));
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
        if builder.block_rows.is_some_and(|expected| rows != expected) {
            return Err(invalid("native matrix block row counts differ"));
        }
        let next_columns = add(builder.block_columns, columns)?;
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
        builder.values.append_row(&value, rows)?;
        builder.block_rows = Some(rows);
        builder.block_columns = next_columns;
        builder.nodes.merge(pending_nodes)?;
        builder.payload_bytes = payload_bytes;
        self.live_bytes = add(old_live, extra)?;
        Ok(())
    }
}

impl data::CollectedValues {
    fn append_row(&mut self, value: &Value, rows: u64) -> Result<(), Error> {
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
        // per element. Non-numeric banks use admitted draft slots.
        match (&mut *self, matrix.elements()) {
            (Self::U8(to), SequenceView::U8(from)) if rows <= 1 => to.extend_from_slice(from),
            (Self::U16(to), SequenceView::U16(from)) if rows <= 1 => to.extend_from_slice(from),
            (Self::U32(to), SequenceView::U32(from)) if rows <= 1 => to.extend_from_slice(from),
            (Self::U64(to), SequenceView::U64(from)) if rows <= 1 => to.extend_from_slice(from),
            (Self::Bool(to), SequenceView::Bool(from)) if rows <= 1 => to.extend_from_slice(from),
            (Self::I64(to), SequenceView::I64(from)) if rows <= 1 => to.extend_from_slice(from),
            (Self::F64(to), SequenceView::F64(from)) if rows <= 1 => to.extend_from_slice(from),
            _ => {
                for ordinal in 0..matrix.elements().len() as u64 {
                    let ordinal = if rows > 1 {
                        let columns = matrix.elements().len() as u64 / rows;
                        (ordinal % rows) * columns + ordinal / rows
                    } else {
                        ordinal
                    };
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

    fn row_major_order(&mut self, rows: usize, columns: usize, visited: &mut [u8]) {
        for start in 0..visited.len() {
            let mut cursor = start;
            while visited[cursor] == 0 {
                visited[cursor] = 1;
                let next = (cursor % rows) * columns + cursor / rows;
                match self {
                    Self::U8(values) => values.swap(start, next),
                    Self::U16(values) => values.swap(start, next),
                    Self::U32(values) => values.swap(start, next),
                    Self::U64(values) => values.swap(start, next),
                    Self::Bool(values) => values.swap(start, next),
                    Self::I64(values) => values.swap(start, next),
                    Self::F64(values) => values.swap(start, next),
                    Self::Draft(values) => values.swap(start, next),
                }
                cursor = next;
            }
        }
    }
}
