//! Bind a finite byte map and genuine output scratch for an emitted map loop.
use super::*;
use mech_engine::native_artifact::NativeStringMapRegion;

#[cfg(all(test, feature = "canonical-native-program"))]
#[path = "program_string_map_owned_tests.rs"]
mod owned_tests;

pub(super) struct Buffer {
    table: Vec<u16>,
    bytes: Vec<u8>,
    owned: Option<mech_core::snapshot::OwnedSnapshotBytes>,
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
        let length = text.len();
        let (source, mut owned) = if region.owned_source_slots().is_empty() {
            (Some(source), None)
        } else {
            // These are complete-use-proved slots, not an arena-wide alias
            // sweep. Retire only this CURRENT frozen allocation and invalidate
            // its handles before trying to consume its sole remaining owner.
            charge(
                &mut ctx.control.work,
                super::super::super::amount(region.owned_source_slots().len(), 64, 512)?,
            )?;
            for value in region.owned_source_slots() {
                ctx.arena
                    .retire_current_matching(value.get(), &source, &mut ctx.control.work)?;
            }
            let initialized = self.sources[region.iteration() as usize].as_mut().unwrap();
            let backing = core::mem::replace(
                &mut initialized.backing,
                super::super::SourceBacking::ConsumedString,
            );
            initialized.handle = None;
            drop(backing);
            match source.try_into_owned_string(&memory)? {
                mech_core::snapshot::StringOwnershipTransfer::Owned(owned) => {
                    (None, Some(owned.into_bytes()))
                }
                mech_core::snapshot::StringOwnershipTransfer::Unavailable(value) => {
                    // Other genuine owners (inputs, imports, captured aliases)
                    // keep the immutable route. Restore its new current handle;
                    // no source bytes or frozen identity changed on rejection.
                    let handle =
                        ctx.arena
                            .put(region.source().get(), &value, &mut ctx.control.work)?;
                    let initialized = self.sources[region.iteration() as usize].as_mut().unwrap();
                    initialized.backing = super::super::SourceBacking::Value(value.clone());
                    initialized.handle = Some(handle);
                    (Some(value), None)
                }
            }
        };
        let backing = owned
            .as_ref()
            .map_or(remaining, |owner| owner.retained_capacity_bytes());
        let scratch = add(backing, 256)?;
        let outside_source = if owned.is_some() { 0 } else { source_bytes };
        resource(
            ctx,
            id,
            Dimension::TemporaryBytes,
            add(
                add(add(self.live_bytes, outside_source)?, map_bytes)?,
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
        // One conservative work bound covers the source/table scans, optional
        // zero-fill, emitted loop, UTF8 validation and failure cleanup. Reuse
        // does not create or renew any per-operation/round work allowance.
        self.account(ctx, id, remaining, add(mul(remaining, 64)?, 4096)?)?;
        let reservation = memory.reserve_capacity(if owned.is_some() { 256 } else { scratch })?;
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
        if owned.is_none() {
            bytes
                .try_reserve_exact(length)
                .map_err(|_| invalid("compiled String map output allocation"))?;
            if bytes.capacity() > length {
                return Err(invalid("compiled String map output capacity"));
            }
            bytes.resize(length, 0);
        }
        // The aliasing case derives BOTH pointers from the exclusive mutable
        // byte slice. No immutable String/Value loan remains across execution.
        let (input, output) = if let Some(owner) = owned.as_mut() {
            let pointer = owner.as_mut_bytes().as_mut_ptr() as u64;
            (pointer, pointer)
        } else {
            let Some(ValueData::String(text)) = source.as_ref().map(Value::data) else {
                return Err(invalid("compiled String map lost frozen source"));
            };
            (text.as_ptr() as u64, bytes.as_mut_ptr() as u64)
        };
        let words = [input, remaining, table.as_ptr() as u64, output, 0];
        self.builder(id)?.join.as_mut().unwrap().compiled_map = Some(Buffer {
            table,
            bytes,
            owned,
            reservation,
        });
        self.live_bytes = add(self.live_bytes, scratch)?;
        ctx.call_inputs.values_mut()[0] = source;
        ctx.call_inputs.values_mut()[1] = Some(map_owner);
        #[cfg(all(test, feature = "canonical-native-program"))]
        owned_tests::observe(
            ctx,
            region,
            input == output,
            self.builder(id)?
                .join
                .as_ref()
                .unwrap()
                .compiled_map
                .as_ref()
                .unwrap()
                .owned
                .is_some(),
        );
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
            let source = ctx.call_inputs.values()[0].clone();
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
                        .as_ref()
                        .map(|source| {
                            source
                                .memory_budget_retained_bytes(&ctx.library.memory_budget())
                                .ok_or_else(|| invalid("unadmitted map source"))
                        })
                        .transpose()?
                        .unwrap_or(0),
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
            let result = if let Some(source) = source.as_ref() {
                selected.selected_string_map_entry(
                    &map,
                    source,
                    processed as usize..processed as usize + 1,
                    ctx.limits,
                    &ctx.library,
                    coexistence,
                )
            } else {
                let owner = self.builders[id as usize]
                    .as_ref()
                    .and_then(|builder| builder.join.as_ref())
                    .and_then(|join| join.compiled_map.as_ref())
                    .and_then(|buffer| buffer.owned.as_ref())
                    .ok_or_else(|| invalid("lost owned map source"))?;
                selected.selected_owned_string_map_entry(
                    &map,
                    owner,
                    processed as usize..processed as usize + 1,
                    ctx.limits,
                    &ctx.library,
                    coexistence,
                )
            };
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
            owned,
            mut reservation,
        } = join
            .compiled_map
            .take()
            .ok_or_else(|| invalid("compiled String map lost scratch"))?;
        let bytes = if let Some(owner) = owned {
            let (bytes, charge) = owner.into_parts();
            // Source ownership was checked on this SAME account, and scratch
            // arithmetic was checked before either allocation or handoff.
            if charge.budget() != reservation.budget()
                || charge
                    .capacity_bytes()
                    .checked_add(reservation.capacity_bytes())
                    .is_none()
            {
                drop(bytes);
                return Err(invalid("owned map source charge changed"));
            }
            reservation.merge_capacity(charge)?;
            bytes
        } else {
            bytes
        };
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
