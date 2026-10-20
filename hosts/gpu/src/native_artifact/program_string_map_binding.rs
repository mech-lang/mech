//! Bind a finite byte map and genuine output scratch for an emitted map loop.
use super::*;
use mech_engine::native_artifact::NativeStringMapRegion;

pub(super) struct Buffer {
    table: Vec<u16>,
    bytes: Vec<u8>,
    reservation: ManagedMemoryReservation,
}
impl State {
    pub(in crate::native_artifact::program_runtime) fn prepare_compiled_string_map(
        &mut self,
        ctx: &mut Context<'_, '_>,
        region: &NativeStringMapRegion,
        remaining: u64,
    ) -> Result<Option<[u64; 5]>, Error> {
        let id = region.collection();
        self.account(ctx, id, 0, 128)?;
        let source = self.sources[region.iteration() as usize]
            .as_ref()
            .ok_or_else(|| invalid("compiled String map has no source"))?;
        // Only the first nonempty header may scan representations or borrow
        // the map. Empty sources retain the original lazy map semantics.
        if remaining == 0 || remaining != source.count || source.byte_offset != 0 {
            return Ok(None);
        }
        let join = self
            .builder(id)?
            .join
            .as_ref()
            .ok_or_else(|| invalid("compiled String map has no TextJoin"))?;
        if join.parked || join.accepted != 0 || !join.text.is_empty() || join.compiled_map.is_some()
        {
            return Err(invalid("compiled String map has stale sink state"));
        }
        if !matches!(join.separator.data(),ValueData::String(separator) if separator.is_empty()) {
            return Ok(None);
        }
        charge(
            &mut ctx.control.work,
            super::super::super::amount(ctx.call_inputs.values().len(), 16, 128)?,
        )?;
        if ctx.call_inputs.values().len() < 2
            || ctx.call_inputs.values().iter().any(Option::is_some)
        {
            return Err(invalid("compiled String map has stale argument owners"));
        }
        let source = ctx
            .arena
            .copy_current(region.source().get(), &mut ctx.control.work)?;
        let initialized = self.sources[region.iteration() as usize].as_ref().unwrap();
        let handle = ctx
            .arena
            .current_handle(region.source().get(), &mut ctx.control.work)?;
        if initialized.handle != Some(handle)
            || !matches!(&initialized.backing,super::super::SourceBacking::Value(owner)
                if owner.shares_frozen_storage(&source))
        {
            return Err(invalid(
                "compiled String map source generation changed after initialization",
            ));
        }
        let ValueData::String(text) = source.data() else {
            return Err(invalid("compiled String map source kind"));
        };
        self.account(ctx, id, 0, add(mul(text.len() as u64, 64)?, 256)?)?;
        if !mech_engine::intrinsics::text_graphemes::is_ascii_without_crlf(text) {
            return Ok(None);
        }
        if text.len() as u64 != remaining {
            return Err(invalid("compiled String map count differs from source"));
        }
        let map_owner = ctx
            .arena
            .copy_current(region.map().get(), &mut ctx.control.work)?;
        let ValueData::Map(map) = map_owner.data() else {
            return Err(invalid("compiled String map root kind"));
        };
        let entries = map.entries().len() as u64;
        self.account(ctx, id, 0, add(mul(entries, 256)?, 1024)?)?;
        // A one-byte source can only select one-byte ASCII keys. Other keys
        // cannot match; a relevant non-byte result keeps the ordinary CFG.
        for entry in map.entries() {
            let (ValueData::String(key), ValueData::String(value)) =
                (entry.key().data(), entry.value())
            else {
                return Err(invalid("compiled String map entry kind"));
            };
            if key.len() == 1 && key.is_ascii() && (value.len() != 1 || !value.is_ascii()) {
                return Ok(None);
            }
        }
        let memory = ctx.library.memory_budget();
        let source_bytes = source
            .memory_budget_retained_bytes(&memory)
            .ok_or_else(|| invalid("unadmitted String map source"))?;
        let map_bytes = map_owner
            .memory_budget_retained_bytes(&memory)
            .ok_or_else(|| invalid("unadmitted String map root"))?;
        let scratch = add(remaining, 256)?;
        resource(
            ctx,
            id,
            Dimension::TemporaryBytes,
            add(
                add(add(self.live_bytes, source_bytes)?, map_bytes)?,
                scratch,
            )?,
            ctx.limits.max_temporary_bytes,
        )?;
        resource(
            ctx,
            id,
            Dimension::RetainedNodes,
            add(add(self.retained_nodes()?, mul(entries, 2)?)?, 6)?,
            ctx.limits.max_retained_nodes,
        )?;
        let footprint = prospective_string_value_footprint(0, false, 1, remaining)
            .map_err(|_| invalid("compiled String map output footprint"))?;
        resource(
            ctx,
            id,
            Dimension::OutputBytes,
            footprint.retained_bytes,
            ctx.limits.max_output_bytes,
        )?;
        // Full source scan, table scans, zero-fill, emitted loop, UTF8
        // validation and failure cleanup are prepaid on the same finite meter.
        self.account(ctx, id, remaining, add(mul(remaining, 64)?, 4096)?)?;
        let reservation = memory.reserve_capacity(scratch)?;
        let mut table = Vec::new();
        table
            .try_reserve_exact(128)
            .map_err(|_| invalid("compiled String map table allocation"))?;
        if table.capacity() > 128 {
            return Err(invalid("compiled String map table capacity"));
        }
        table.resize(128, 256u16);
        for entry in map.entries() {
            let (ValueData::String(key), ValueData::String(value)) =
                (entry.key().data(), entry.value())
            else {
                unreachable!()
            };
            if key.len() == 1 && key.is_ascii() {
                table[key.as_bytes()[0] as usize] = u16::from(value.as_bytes()[0]);
            }
        }
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(text.len())
            .map_err(|_| invalid("compiled String map output allocation"))?;
        if bytes.capacity() > text.len() {
            return Err(invalid("compiled String map output capacity"));
        }
        bytes.resize(text.len(), 0);
        let words = [
            text.as_ptr() as u64,
            remaining,
            table.as_ptr() as u64,
            bytes.as_mut_ptr() as u64,
            0,
        ];
        self.builder(id)?.join.as_mut().unwrap().compiled_map = Some(Buffer {
            table,
            bytes,
            reservation,
        });
        self.live_bytes = add(self.live_bytes, scratch)?;
        ctx.call_inputs.values_mut()[0] = Some(source);
        ctx.call_inputs.values_mut()[1] = Some(map_owner);
        Ok(Some(words))
    }

    pub(in crate::native_artifact::program_runtime) fn finish_compiled_string_map(
        &mut self,
        ctx: &mut Context<'_, '_>,
        region: &NativeStringMapRegion,
        accepted: u64,
        processed: u64,
    ) -> Result<(), Error> {
        let id = region.collection();
        if processed < accepted {
            // Preserve the actual selected missing-key error and origin. This
            // is one error call after the loop, never a per-character helper.
            let source = ctx.call_inputs.values()[0]
                .as_ref()
                .ok_or_else(|| invalid("lost map source loan"))?
                .clone();
            let map = ctx.call_inputs.values()[1]
                .as_ref()
                .ok_or_else(|| invalid("lost map root loan"))?
                .clone();
            let selected = ctx.control.descriptor.calls[region.selection_site() as usize]
                .as_ref()
                .ok_or_else(|| invalid("compiled String map lost selected factory"))?;
            let builder = self.builder(id)?;
            let prior = (builder.compute, builder.projection_comparison);
            let bytes = add(
                self.live_bytes,
                add(
                    source
                        .memory_budget_retained_bytes(&ctx.library.memory_budget())
                        .ok_or_else(|| invalid("unadmitted map source"))?,
                    map.memory_budget_retained_bytes(&ctx.library.memory_budget())
                        .ok_or_else(|| invalid("unadmitted map root"))?,
                )?,
            )?;
            let ValueData::Map(entries) = map.data() else {
                return Err(invalid("map owner kind"));
            };
            let coexistence = mech_core::ResourceDemand {
                turn_peak_bytes: bytes,
                retained_nodes: add(
                    add(
                        self.retained_nodes()?,
                        mul(entries.entries().len() as u64, 2)?,
                    )?,
                    6,
                )?,
                cloned_bytes: self.builder(id)?.cloned_bytes,
                work: mech_core::WorkDemand {
                    compute: prior.0,
                    comparison: prior.1,
                    ..Default::default()
                },
                ..Default::default()
            };
            let before = ctx.library.used();
            let result = selected.selected_string_map_entry(
                &map,
                &source,
                processed as usize..processed as usize + 1,
                ctx.limits,
                &ctx.library,
                coexistence,
            );
            let after = ctx.library.used();
            let builder = self.builder(id)?;
            builder.compute = add(
                prior.0,
                after
                    .0
                    .checked_sub(before.0)
                    .ok_or_else(|| invalid("map work moved backwards"))?,
            )?;
            builder.projection_comparison = add(
                prior.1,
                after
                    .1
                    .checked_sub(before.1)
                    .ok_or_else(|| invalid("map comparison moved backwards"))?,
            )?;
            return match result {
                Err(error) => Err(Error::Library {
                    origin: selected.template().origin(),
                    error,
                }),
                Ok(_) => Err(invalid(
                    "compiled String map disagrees with selected lookup",
                )),
            };
        }
        let builder = self.builder(id)?;
        let join = builder
            .join
            .as_mut()
            .ok_or_else(|| invalid("compiled String map lost TextJoin"))?;
        let Buffer {
            table,
            bytes,
            mut reservation,
        } = join
            .compiled_map
            .take()
            .ok_or_else(|| invalid("compiled String map lost scratch"))?;
        if bytes.len() as u64 != accepted || processed != accepted {
            return Err(invalid("compiled String map output count"));
        }
        let text = String::from_utf8(bytes)
            .map_err(|_| invalid("compiled String map wrote invalid UTF8"))?;
        drop(table);
        drop(reservation.split_capacity(256)?);
        join.text = text;
        join.accepted = accepted;
        builder.payload_bytes = accepted;
        builder.reservation = reservation;
        self.live_bytes = self
            .live_bytes
            .checked_sub(256)
            .ok_or_else(|| invalid("compiled map table accounting"))?;
        self.sources[region.iteration() as usize]
            .as_mut()
            .unwrap()
            .byte_offset = accepted as usize;
        Ok(())
    }
}
