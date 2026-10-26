//! Bind a finite byte map and genuine output scratch for an emitted map loop.
use crate::native_artifact::program::runtime::collections::text_join::*;
use crate::native_artifact::program::runtime::invocation::CollectionExecution;
use crate::native_artifact::program::runtime::maps::ascii as ascii_map;
use mech_engine::native_artifact::NativeStringMapRegion;

#[cfg(all(test, feature = "canonical-native-program"))]
#[path = "owned_tests.rs"]
pub(in crate::native_artifact::program::runtime::collections::text_join::compiled_map) mod owned_tests;

enum Owned {
    Snapshot(mech_core::snapshot::OwnedSnapshotBytes),
    Private(mech_engine::native_artifact::NativePrivateStringBuffer),
}
impl Owned {
    fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Snapshot(owner) => owner.as_bytes(),
            Self::Private(owner) => owner.as_bytes(),
        }
    }
    fn retained_capacity_bytes(&self) -> u64 {
        match self {
            Self::Snapshot(owner) => owner.retained_capacity_bytes(),
            Self::Private(owner) => owner.retained_capacity_bytes(),
        }
    }
    fn as_mut_bytes(&mut self) -> Result<&mut [u8], Error> {
        match self {
            Self::Snapshot(owner) => Ok(owner.as_mut_bytes()),
            // A joined buffer was already transformed and validated. It must
            // travel straight to its original finish, never enter a new loan.
            Self::Private(_) => Err(invalid("private joined map reentered mutable mapping")),
        }
    }
    fn into_text_parts(self) -> Result<(String, ManagedMemoryReservation), Error> {
        match self {
            Self::Private(owner) => Ok(owner.into_parts()),
            Self::Snapshot(owner) => {
                let (bytes, charge) = owner.into_parts();
                let text = String::from_utf8(bytes)
                    .map_err(|_| invalid("compiled String map wrote invalid UTF8"))?;
                Ok((text, charge))
            }
        }
    }
}

pub(in crate::native_artifact::program::runtime::collections::text_join) struct Buffer {
    table: Vec<u8>,
    bytes: Vec<u8>,
    owned: Option<Owned>,
    reservation: ManagedMemoryReservation,
}
impl State {
    /// The same original map admissions/finish leaf over an already composed
    /// private owner. No semantic joined-source Value is installed or read.
    pub(in crate::native_artifact::program::runtime) fn prepare_joined_string_map(
        &mut self,
        ctx: &mut CollectionExecution<'_, '_, '_, '_>,
        region: &NativeStringMapRegion,
        remaining: u64,
    ) -> Result<Option<u64>, Error> {
        let Some(mapped) = ctx.take_joined_map(region)? else {
            return Ok(None);
        };
        let id = region.collection();
        self.account(ctx, id, 0, 128)?;
        let source = self.sources[region.iteration() as usize]
            .as_ref()
            .ok_or_else(|| invalid("joined String map has no initialized source"))?;
        if remaining == 0
            || remaining != mapped.count
            || source.count != remaining
            || source.byte_offset != 0
            || source.handle.is_some()
            || !matches!(source.backing, crate::native_artifact::program::runtime::collections::SourceBacking::ConsumedString)
            || mapped.processed > remaining
        {
            return Err(invalid("joined String map source count/owner differs"));
        }
        let join = self
            .builder(id)?
            .join
            .as_ref()
            .ok_or_else(|| invalid("joined String map has no TextJoin"))?;
        if join.parked
            || join.accepted != 0
            || !join.text.is_empty()
            || join.compiled_map.is_some()
            || !matches!(join.separator.data(), ValueData::String(s) if s.is_empty())
        {
            return Err(invalid("joined String map sink state differs"));
        }
        ctx.charge_input_bank_headers()?;
        if ctx.inputs().len() < 2 || ctx.inputs().iter().any(Option::is_some) {
            return Err(invalid("joined String map has stale argument owners"));
        }
        for slot in region.owned_source_slots() {
            if ctx.arena_current_retained_bytes(slot.get())?.is_some() {
                return Err(invalid(
                    "joined String map fabricated a canonical source alias",
                ));
            }
        }
        // Keep original source qualification and table/loop prices. The source
        // proof establishes one original byte per grapheme before replacements.
        self.account(ctx, id, 0, add(mul(remaining, 64)?, 256)?)?;
        let ValueData::Map(entries) = mapped.map.data() else {
            return Err(invalid("joined String map root kind"));
        };
        let entries = entries.entries().len() as u64;
        self.account(ctx, id, 0, add(mul(entries, 256)?, 1024)?)?;
        let memory = ctx.library().memory_budget();
        if mapped.owner.memory_budget() != memory
            || mapped.owner.as_bytes().len() as u64 != remaining
        {
            return Err(invalid("joined String map lost its same-account backing"));
        }
        let map_bytes = mapped
            .map
            .memory_budget_retained_bytes(&memory)
            .ok_or_else(|| invalid("joined String map root is unadmitted"))?;
        let scratch = add(mapped.owner.retained_capacity_bytes(), 256)?;
        resource(
            ctx,
            id,
            Dimension::TemporaryBytes,
            add(add(self.live_bytes, map_bytes)?, scratch)?,
            ctx.limits().max_temporary_bytes,
        )?;
        resource(
            ctx,
            id,
            Dimension::RetainedNodes,
            add(add(self.retained_nodes()?, mul(entries, 2)?)?, 6)?,
            ctx.limits().max_retained_nodes,
        )?;
        let footprint = prospective_string_value_footprint(0, false, 1, remaining)
            .map_err(|_| invalid("joined String map output footprint"))?;
        resource(
            ctx,
            id,
            Dimension::OutputBytes,
            footprint.retained_bytes,
            ctx.limits().max_output_bytes,
        )?;
        self.account(ctx, id, remaining, add(mul(remaining, 64)?, 4096)?)?;
        let reservation = memory.reserve_capacity(256)?;
        let processed = mapped.processed;
        self.builder(id)?.join.as_mut().unwrap().compiled_map = Some(Buffer {
            table: Vec::new(),
            bytes: Vec::new(),
            owned: Some(Owned::Private(mapped.owner)),
            reservation,
        });
        self.live_bytes = add(self.live_bytes, scratch)?;
        ctx.inputs_mut()[0] = None;
        ctx.inputs_mut()[1] = Some(mapped.map);
        #[cfg(all(test, feature = "canonical-native-program"))]
        owned_tests::observe(ctx, region, true, true);
        Ok(Some(processed))
    }

    pub(in crate::native_artifact::program::runtime) fn prepare_compiled_string_map(
        &mut self,
        ctx: &mut CollectionExecution<'_, '_, '_, '_>,
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
        ctx.charge_input_bank_headers()?;
        if ctx.inputs().len() < 2 || ctx.inputs().iter().any(Option::is_some) {
            return Err(invalid("compiled String map has stale argument owners"));
        }
        let source = ctx.arena_copy_current(region.source().get())?;
        let initialized = self.sources[region.iteration() as usize].as_ref().unwrap();
        let handle = ctx.arena_current_handle(region.source().get())?;
        if initialized.handle != Some(handle)
            || !matches!(&initialized.backing,crate::native_artifact::program::runtime::collections::SourceBacking::Value(owner)
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
        // Only descriptors that already fund the weak fact may use it. The
        // fresh source/generation checks and complete scan charge above stay.
        let ascii = if ctx.descriptor().metadata().window_regions.is_empty() {
            mech_engine::intrinsics::text_graphemes::is_ascii_without_crlf(text)
        } else {
            self.ascii_facts.inspect(&source)?
        };
        if !ascii {
            return Ok(None);
        }
        if text.len() as u64 != remaining {
            return Err(invalid("compiled String map count differs from source"));
        }
        let map_owner = ctx.arena_copy_current(region.map().get())?;
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
        let memory = ctx.library().memory_budget();
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
                &mut ctx.work(),
                crate::native_artifact::program::runtime::amount(
                    region.owned_source_slots().len(),
                    64,
                    512,
                )?,
            )?;
            for value in region.owned_source_slots() {
                ctx.arena_retire_current_matching(value.get(), &source)?;
            }
            let initialized = self.sources[region.iteration() as usize].as_mut().unwrap();
            let backing = core::mem::replace(
                &mut initialized.backing,
                crate::native_artifact::program::runtime::collections::SourceBacking::ConsumedString,
            );
            initialized.handle = None;
            drop(backing);
            match source.try_into_owned_string(&memory)? {
                mech_core::snapshot::StringOwnershipTransfer::Owned(owned) => {
                    (None, Some(Owned::Snapshot(owned.into_bytes())))
                }
                mech_core::snapshot::StringOwnershipTransfer::Unavailable(value) => {
                    // Other genuine owners (inputs, imports, captured aliases)
                    // keep the immutable route. Restore its new current handle;
                    // no source bytes or frozen identity changed on rejection.
                    let handle = ctx.arena_put(region.source().get(), &value)?;
                    let initialized = self.sources[region.iteration() as usize].as_mut().unwrap();
                    initialized.backing =
                        crate::native_artifact::program::runtime::collections::SourceBacking::Value(
                            value.clone(),
                        );
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
            ctx.limits().max_temporary_bytes,
        )?;
        resource(
            ctx,
            id,
            Dimension::RetainedNodes,
            add(add(self.retained_nodes()?, mul(entries, 2)?)?, 6)?,
            ctx.limits().max_retained_nodes,
        )?;
        let footprint = prospective_string_value_footprint(0, false, 1, remaining)
            .map_err(|_| invalid("compiled String map output footprint"))?;
        resource(
            ctx,
            id,
            Dimension::OutputBytes,
            footprint.retained_bytes,
            ctx.limits().max_output_bytes,
        )?;
        // One conservative work bound covers the source/table scans, optional
        // zero-fill, emitted loop, UTF8 validation and failure cleanup. Reuse
        // does not create or renew any per-operation/round work allowance.
        self.account(ctx, id, remaining, add(mul(remaining, 64)?, 4096)?)?;
        let reservation = memory.reserve_capacity(if owned.is_some() { 256 } else { scratch })?;
        let mut table = Vec::new();
        table
            .try_reserve_exact(ascii_map::BYTES)
            .map_err(|_| invalid("compiled String map table allocation"))?;
        if table.capacity() > ascii_map::BYTES {
            return Err(invalid("compiled String map table capacity"));
        }
        table.resize(ascii_map::BYTES, ascii_map::MISSING);
        for entry in map.entries() {
            let (ValueData::String(key), ValueData::String(value)) =
                (entry.key().data(), entry.value())
            else {
                unreachable!()
            };
            if key.len() == 1 && key.is_ascii() {
                table[ascii_map::HEADER + key.as_bytes()[0] as usize] = value.as_bytes()[0];
            }
        }
        ascii_map::prepare(&mut table);
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
            let pointer = owner.as_mut_bytes()?.as_mut_ptr() as u64;
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
        ctx.inputs_mut()[0] = source;
        ctx.inputs_mut()[1] = Some(map_owner);
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

    pub(in crate::native_artifact::program::runtime) fn finish_compiled_string_map(
        &mut self,
        ctx: &mut CollectionExecution<'_, '_, '_, '_>,
        region: &NativeStringMapRegion,
        accepted: u64,
        processed: u64,
    ) -> Result<(), Error> {
        let id = region.collection();
        if processed < accepted {
            // Preserve the actual selected missing-key error and origin. This
            // is one error call after the loop, never a per-character helper.
            let source = ctx.inputs()[0].clone();
            let map = ctx.inputs()[1]
                .as_ref()
                .ok_or_else(|| invalid("lost map root loan"))?
                .clone();
            let selected = ctx.descriptor().metadata().calls[region.selection_site() as usize]
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
                                .memory_budget_retained_bytes(&ctx.library().memory_budget())
                                .ok_or_else(|| invalid("unadmitted map source"))
                        })
                        .transpose()?
                        .unwrap_or(0),
                    map.memory_budget_retained_bytes(&ctx.library().memory_budget())
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
            let before = ctx.library().used();
            let result = if let Some(source) = source.as_ref() {
                selected.selected_string_map_entry(
                    &map,
                    source,
                    processed as usize..processed as usize + 1,
                    ctx.limits(),
                    &ctx.library(),
                    coexistence,
                )
            } else {
                let owner = self.builders[id as usize]
                    .as_ref()
                    .and_then(|builder| builder.join.as_ref())
                    .and_then(|join| join.compiled_map.as_ref())
                    .and_then(|buffer| buffer.owned.as_ref())
                    .ok_or_else(|| invalid("lost owned map source"))?;
                match owner {
                    Owned::Snapshot(owner) => selected.selected_owned_string_map_entry(
                        &map,
                        owner,
                        processed as usize..processed as usize + 1,
                        ctx.limits(),
                        &ctx.library(),
                        coexistence,
                    ),
                    Owned::Private(owner) => selected.selected_private_string_map_entry(
                        &map,
                        owner,
                        processed as usize..processed as usize + 1,
                        ctx.limits(),
                        &ctx.library(),
                        coexistence,
                    ),
                }
            };
            let after = ctx.library().used();
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
        if owned
            .as_ref()
            .map_or(bytes.len(), |owner| owner.as_bytes().len()) as u64
            != accepted
            || processed != accepted
        {
            return Err(invalid("compiled String map output count"));
        }
        let text = if let Some(owner) = owned {
            let (text, charge) = owner.into_text_parts()?;
            // Source ownership was checked on this SAME account, and scratch
            // arithmetic was checked before either allocation or handoff.
            if charge.budget() != reservation.budget()
                || charge
                    .capacity_bytes()
                    .checked_add(reservation.capacity_bytes())
                    .is_none()
            {
                drop(text);
                return Err(invalid("owned map source charge changed"));
            }
            reservation.merge_capacity(charge)?;
            text
        } else {
            String::from_utf8(bytes)
                .map_err(|_| invalid("compiled String map wrote invalid UTF8"))?
        };
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
