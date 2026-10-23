//! A private storage binding for one closed, generated source loop. This module
//! never traverses the input, chooses a source branch, or computes an update.
use super::{Binding, Context, Error, State, add, invalid, storage_error};
use crate::native_artifact::{NativeExecutionPhase as Phase, owned_string_map::PublicationGeometry};
use mech_core::{ManagedMemoryReservation, ManagedNodeReservation, MemoryBudgetLimits, Value, ValueData};
use mech_engine::native_artifact::{NativeLibraryWorkBudget, NativePreparedLibraryCall,
    NativeSelectedStringSpanSelector};
use core::ffi::c_void;
use crate::native_artifact::program_packed_map::{PackedMap, BodyPrice};

#[repr(C)]
pub(super) struct Owner<'plan> {
    // Private generated-code prefix. No source Value can carry this address.
    packed_frame: u64,
    packed: Option<PackedMap>,
    packed_ssa_entered: bool,
    guarded_loop_entered: bool,
    header_site: usize,
    binding: Binding<'plan>,
    source: Value,
    range: &'plan NativePreparedLibraryCall,
    selection_site: usize,
    descriptor: &'plan crate::native_artifact::NativeProgramDescriptor,
    selector: Option<NativeSelectedStringSpanSelector>,
    selector_current: bool,
    key: Option<core::ops::Range<usize>>,
    probe: Option<crate::native_artifact::owned_map_binding::NativeProbe>,
    work: NativeLibraryWorkBudget,
    limits: MemoryBudgetLimits,
    outside_nodes: u64,
    body_work: u64,
    pub funded: u64,
    updates: u64,
    result: *mut u64,
    error: Option<Error>,
    _reservation: ManagedMemoryReservation,
    _nodes: ManagedNodeReservation,
}

impl Owner<'_> {
    fn call(&mut self, site: usize, op: u64, args: [u64; 3]) -> Result<u64, Error> {
        #[cfg(all(test, feature = "canonical-native-program"))]
        super::super::map_closed_tests::record_bound_call(op);
        if op == 13 {
            let packed = self.packed.as_ref().ok_or_else(|| invalid("packed SSA entry without owner"))?;
            if self.packed_ssa_entered || site != self.header_site || args != [0; 3]
                || self.funded != 0 || self.updates != 0
                || packed.funded() != 0 || packed.remaining() != 0 || packed.updates() != 0
            { return Err(invalid("packed SSA entry is not the first header")); }
            self.packed_ssa_entered = true;
            return Ok(0);
        }
        if op == 14 {
            let packed = self.packed.as_ref().ok_or_else(|| invalid("guarded loop entry without owner"))?;
            let ValueData::String(text) = self.source.data() else { return Err(invalid("guarded loop lost pinned String")); };
            if !self.packed_ssa_entered || self.guarded_loop_entered || site != self.header_site
                || args[0] != packed.width() || args[0] == 0 || args[0] > args[1]
                || args[1] > text.len() as u64 || args[2] != 0
                || self.funded != 0 || self.updates != 0
                || packed.funded() != 0 || packed.remaining() != 0 || packed.updates() != 0
            { return Err(invalid("guarded loop selection is not a valid first header")); }
            self.guarded_loop_entered = true;
            return Ok(0);
        }
        if op == 0 {
            if let Some(packed) = self.packed.as_mut() {
                let count = packed.refill().map_err(storage_error)?;
                self.funded = add(self.funded, count)?;
                return Ok(count);
            }
            // A finite prepayment, never a renewed allowance. Unused final
            // batch fuel stays spent. Storage debits share this SAME account.
            let remaining = self.work.unscoped_remaining()
                .ok_or_else(|| invalid("closed map entered a foreign work scope"))?;
            let count = (remaining.0 / self.body_work / 4).min(1024).max(1);
            let price = self.body_work.checked_mul(count)
                .ok_or_else(|| invalid("closed map body prepayment overflow"))?;
            self.work.charge(price, 0).map_err(super::StorageError::Work).map_err(storage_error)?;
            self.funded = add(self.funded, count)?;
            self.selector_current = false;
            self.key = None;
            self.probe = None;
            return Ok(count);
        }
        if op == 7 {
            let packed = self.packed.as_mut().ok_or_else(|| invalid("packed insert without owner"))?;
            let frame = packed.frame();
            // SAFETY: the private Box owns this frame; no Rust borrow survives
            // the synchronous generated call, which supplied these exact keys.
            let (low, high) = unsafe { ((*frame).key_low, (*frame).key_high) };
            packed.insert_missing(low, high, args[2]).map_err(storage_error)?;
            return Ok(0);
        }
        if op == 8 { return Err(storage_error(super::StorageError::Binding("missing map key"))); }
        if op == 9 { return Err(invalid("invalid generated packed-map operation")); }
        if matches!(op, 10..=12) {
            let packed = self.packed.as_ref().ok_or_else(|| invalid("packed call cap without owner"))?;
            packed.check_call(match op { 10 => 6, 11 => 3, _ => 5 }).map_err(storage_error)?;
            return Err(invalid("packed call flag denied an allowed operation"));
        }
        if op == 1 || op == 6 {
            let selector = self.work.with_call_limits(self.limits.max_compute_work.unwrap(),
                self.limits.max_comparison_work.unwrap(), || {
                    self.work.charge(self.range.selected_u64_string_span_selector_work(), 0)
                        .map_err(super::StorageError::Work).map_err(storage_error)?;
                    self.range.selected_u64_string_span_selector_reusing_shape(
                        args[0], args[1], self.selector.as_ref())
                        .map_err(|error| Error::Library { origin: self.descriptor.plan.blocks()[site].origin(), error })
                }).map_err(super::StorageError::Work).map_err(storage_error)??;
            self.selector = Some(selector);
            self.selector_current = true;
            self.key = None;
            self.probe = None;
            if op == 1 { return Ok(0); }
        }
        let ValueData::String(text) = self.source.data() else {
            return Err(invalid("closed map lost pinned String"));
        };
        if op == 2 || op == 6 {
            if !core::mem::take(&mut self.selector_current) {
                return Err(invalid("closed map selection without current range"));
            }
            let selector = self.selector.as_ref()
                .ok_or_else(|| invalid("closed map selection without range"))?;
            // Fusion moves no fallible source instruction across this pair:
            // only empty blocks/jumps intervene. Preserve the selection's
            // original diagnostic origin after the range has succeeded.
            let selection_site = if op == 6 { self.selection_site } else { site };
            let bad = || Error::Library {
                origin: self.descriptor.plan.blocks()[selection_site].origin(),
                error: mech_engine::native_artifact::NativeCollectionInvocationError::Kernel(
                    mech_core::ResidentKernelError::InvalidInput),
            };
            if selector.count() == 0 { self.key = Some(0..0); return Ok(0); }
            let start = selector.start().checked_sub(1).ok_or_else(bad)?;
            let end = start.checked_add(selector.count())
                .ok_or_else(bad)?;
            if end > text.len() as u64 {
                return Err(bad());
            }
            self.key = Some(start as usize..end as usize);
            return Ok(0);
        }
        let bytes = self.key.as_ref().ok_or_else(|| invalid("closed map operation without key"))?;
        let key = text.get(bytes.clone()).ok_or_else(|| invalid("closed map key bounds"))?;
        match op {
            3 | 4 => self.binding.native_read(key, op == 3, &mut self.probe, self.limits).map_err(storage_error),
            5 => {
                let geometry = PublicationGeometry {
                    live_bytes: self.work.memory_budget().used_bytes(),
                    live_nodes: add(self.outside_nodes,
                        self.binding.retained_nodes().map_err(storage_error)?)?,
                };
                let next = add(self.updates, 1)?;
                let result = self.binding.native_insert(key, args[2], &mut self.probe, geometry, self.limits)
                    .map_err(storage_error)?;
                self.updates = next;
                Ok(result)
            }
            _ => Err(invalid("unknown private map storage operation")),
        }
    }
}

unsafe extern "C" fn call(raw: *mut c_void, site: u64, op: u64,
    first: u64, second: u64, third: u64, result: *mut u64) -> u64
{
    use crate::native_artifact::abi::{STATUS_ABI, STATUS_ADMISSION, STATUS_OK};
    if raw.is_null() { return STATUS_ABI; }
    // SAFETY: the checked entry supplies this pointer only paired with this
    // function. Its Box, source and table remain pinned until the synchronous
    // generated loop leaves. No source Value can contain either private word.
    let owner = unsafe { &mut *raw.cast::<Owner<'_>>() };
    if owner.error.is_some() { return STATUS_ADMISSION; }
    let value = if result.is_null() || result != owner.result {
        Err(invalid("closed map result lane mismatch"))
    } else { owner.call(site as usize, op, [first, second, third]) };
    match value {
        Ok(value) => {
            // SAFETY: this is the exact admitted one-word result lane.
            unsafe { *result = value; }
            STATUS_OK
        }
        Err(error) => { owner.error = Some(error); STATUS_ADMISSION }
    }
}

impl<'plan> State<'plan> {
    fn begin_closed(&mut self, ctx: &mut Context<'plan, '_>, region: usize)
        -> Result<Option<[u64; 4]>, Error>
    {
        let proof = &self.descriptor.closed_tails.regions[region];
        let index = proof.map.ok_or_else(|| invalid("closed map missing owner proof"))?;
        if !core::ptr::eq(self.descriptor, ctx.control.descriptor)
            || self.work_identity != ctx.control.work.identity()
            || ctx.control.previous != Some(proof.tail.initialize())
            || self.last_owned_visit != Some(ctx.control.visits) || !ctx.consumed
        { return Err(invalid("closed map binding outside its consumed initializer")); }
        if self.slots[index].closed.is_some() || self.active_tables != 1
            || !self.caller.as_ref().is_some_and(|caller| caller.iteration == proof.iteration)
        { return Ok(None); }
        let Some(binding) = self.slots[index].binding.as_ref() else { return Ok(None); };
        let layout = self.descriptor.map_owner_words.as_ref().unwrap();
        let span = layout.spans().iter().find(|span| span.region() == index)
            .ok_or_else(|| invalid("closed map missing key projection"))?;
        let source = ctx.arena.copy_current(span.source().get(), &mut ctx.control.work)?;
        let ValueData::String(text) = source.data() else { return Ok(None); };
        let scan = (text.len() as u64).checked_mul(64).and_then(|n| n.checked_add(512))
            .ok_or_else(|| invalid("closed map String scan overflow"))?;
        ctx.control.work.charge(Phase::Setup, scan)?;
        if !mech_engine::intrinsics::text_graphemes::is_ascii_without_crlf(text) {
            return Ok(None);
        }
        if source.memory_budget_retained_bytes(&ctx.library.memory_budget()).is_none() {
            return Err(invalid("closed map String lacks invocation admission"));
        }
        let nodes = binding.retained_nodes().map_err(storage_error)?;
        let Some(geometry) = self.geometry(ctx, proof.tail.initialize() as usize)? else { return Ok(None); };
        let range = self.descriptor.calls[span.range_site() as usize].as_ref()
            .ok_or_else(|| invalid("closed map missing maintained range factory"))?;
        // The selected range's shape/cardinality scratch is retained for this
        // private lane instead of allocated/admitted again on every projection.
        let bytes = add((core::mem::size_of::<Owner<'plan>>() + 128) as u64,
            range.selected_u64_string_span_selector_metadata_bytes())?;
        if add(geometry.live_bytes, bytes)? > ctx.limits.max_temporary_bytes.unwrap_or(0)
            || add(geometry.live_nodes, bytes)? > ctx.limits.max_retained_nodes.unwrap_or(0)
        { return Ok(None); }
        let memory = ctx.library.memory_budget().with_limits(ctx.limits.max_temporary_bytes.unwrap(),
            ctx.limits.max_retained_nodes.unwrap());
        let private_nodes = memory.reserve_nodes(bytes)?;
        let reservation = memory.reserve_capacity(bytes)?;
        let outside_nodes = geometry.live_nodes.checked_sub(nodes)
            .and_then(|n| n.checked_add(bytes)).ok_or_else(|| invalid("closed map census overflow"))?;
        let mut packed = None;
        if let Some(packed_proof) = proof.packed.as_ref()
            && binding.native_seed_is_empty().map_err(storage_error)?
        {
            if ctx.arguments.is_null() || ctx.argument_capacity < 2 {
                return Err(invalid("packed map offset lane is too small"));
            }
            // SAFETY: the checked owning session pins this exact argument
            // lane. Generated entry wrote its two proved native scalar words;
            // copy them before begin_closed_region writes return words 0..4.
            // Computed scalars need not have a canonical arena allocation.
            let [first, last] = unsafe { [*ctx.arguments, *ctx.arguments.add(1)] };
            if (packed_proof.start_offset().is_none() && first != 0)
                || (packed_proof.end_offset().is_none() && last != 0)
            { return Err(invalid("packed map absent offset is not zero")); }
            let width = last.checked_sub(first)
                .and_then(|width| width.checked_add(u64::from(packed_proof.inclusive())));
            if let Some(width) = width.filter(|width| (1..=18).contains(width)) {
                let end = width + u64::from(!packed_proof.inclusive());
                // Shape/schema validation happens once; changed-width or
                // unrepresentable regions retain the general closed owner.
                // If setup cannot fit the call cap, decline without rejecting
                // a source branch that might never execute the range at all.
                let compute = range.selected_u64_string_span_selector_work();
                let admitted = if compute <= ctx.limits.max_compute_work.unwrap() {
                    ctx.library.with_call_limits(ctx.limits.max_compute_work.unwrap(),
                        ctx.limits.max_comparison_work.unwrap(), || {
                            ctx.library.charge(compute, 0)?;
                            Ok::<_, mech_core::ResidentKernelError>(
                                range.selected_u64_string_span_selector(1, end).is_ok())
                        }).map_err(super::StorageError::Work).map_err(storage_error)?
                        .map_err(super::StorageError::Work).map_err(storage_error)?
                } else { false };
                if admitted {
                    let region = &layout.regions()[index];
                    packed = Some(PackedMap::new_encoded(ctx.library.clone(), ctx.limits, width,
                        BodyPrice { scalar_compute: proof.compute,
                            range_compute: range.selected_u64_string_span_selector_work(),
                            reads: region.reads().len() as u64, writes: region.writes().len() as u64 },
                        outside_nodes, packed_proof.packing_domain()).map_err(storage_error)?);
                }
            }
        }
        let binding = self.slots[index].binding.take().unwrap();
        let mut owner = Box::new(Owner { packed_frame: 0, packed,
            packed_ssa_entered: false, guarded_loop_entered: false, header_site: proof.tail.header() as usize,
            binding, source, range,
            selection_site: span.selection_site() as usize, descriptor: self.descriptor,
            selector: None, selector_current: false, key: None, probe: None,
            work: ctx.library.clone(), limits: ctx.limits, outside_nodes,
            body_work: proof.compute, funded: 0, updates: 0, result: ctx.result,
            error: None, _reservation: reservation, _nodes: private_nodes });
        if let Some(packed) = owner.packed.as_mut() {
            let frame = packed.frame();
            let ValueData::String(text) = owner.source.data() else { unreachable!() };
            // SAFETY: both owners remain pinned in this Box for the entire
            // synchronous generated entry. The source cannot mutate.
            unsafe { (*frame).source_ptr = text.as_ptr() as u64; (*frame).source_len = text.len() as u64; }
            owner.packed_frame = frame as u64;
            ctx.packed_map_regions = add(ctx.packed_map_regions, 1)?;
            #[cfg(all(test, feature = "canonical-native-program"))]
            super::super::map_closed_tests::record_packed_entry();
        }
        let pointer = (&mut *owner as *mut Owner<'plan>) as u64;
        self.slots[index].closed = Some(owner);
        Ok(Some([0, 0, call as *const () as u64, pointer]))
    }
}

impl<'plan, 'a> Context<'plan, 'a> {
    pub(in crate::native_artifact::program_runtime) fn begin_closed_map(&mut self, region: usize) -> Result<Option<[u64; 4]>, Error> {
        let Some(mut state) = self.map_owners.state.take() else { return Ok(None); };
        let result = state.begin_closed(self, region);
        self.map_owners.state = Some(state);
        result
    }
    pub(in crate::native_artifact::program_runtime) fn end_closed_map(&mut self, region: usize, remaining: u64, mode: u64) -> Result<u64, Error> {
        if !matches!(mode, 0 | 2 | 3) { return Err(invalid("closed map cannot suspend")); }
        let index = self.control.descriptor.closed_tails.regions[region].map.unwrap();
        let state = self.map_owners.state.as_mut().ok_or_else(|| invalid("closed map session lost"))?;
        let owner = state.slots[index].closed.take().ok_or_else(|| invalid("closed map owner lost"))?;
        let Owner { mut binding, packed, packed_ssa_entered, guarded_loop_entered, funded, mut updates, error, .. } = *owner;
        if let Some(error) = error { state.slots[index].binding = Some(binding); return Err(error); }
        let traversals = funded.checked_sub(remaining)
            .ok_or_else(|| invalid("closed map body fuel was renewed"))?;
        let is_packed = packed.is_some();
        if let Some(packed) = packed {
            if packed.remaining() != remaining || packed.funded() != funded {
                return Err(invalid("packed map fuel differs from generated body"));
            }
            updates = packed.updates();
            if mode == 0 {
                let frozen_proof = state.descriptor.frozen_maps.regions.iter()
                    .find(|proof| proof.owner == index && proof.closed == region);
                #[cfg(test)]
                let frozen_proof = if super::frozen::TEST_DISABLE.with(std::cell::Cell::get) { None } else { frozen_proof };
                if let Some(proof) = frozen_proof {
                    if state.slots[index].frozen.is_some() { return Err(invalid("overlapping frozen map")); }
                    let map = packed.freeze().map_err(storage_error)?;
                    if map.updates() != updates || map.traversals() != traversals {
                        return Err(invalid("frozen map completion differs"));
                    }
                    state.slots[index].frozen = Some(super::frozen::FrozenSlot {
                        map, token: 0, parent: proof.parent, born_visit: self.control.visits,
                    });
                } else {
                    let (map, written, consumed) = packed.finish().map_err(storage_error)?;
                    if written != updates || consumed != traversals { return Err(invalid("packed map completion differs")); }
                    binding.install_native_map(map).map_err(storage_error)?;
                }
            }
        }
        state.slots[index].binding = Some(binding);
        self.hash_table_updates = add(self.hash_table_updates, updates)?;
        if is_packed { self.packed_map_updates = add(self.packed_map_updates, updates)?; }
        if packed_ssa_entered { self.packed_ssa_regions = add(self.packed_ssa_regions, 1)?; }
        if guarded_loop_entered { self.guarded_loop_regions = add(self.guarded_loop_regions, 1)?; }
        Ok(traversals)
    }
}
