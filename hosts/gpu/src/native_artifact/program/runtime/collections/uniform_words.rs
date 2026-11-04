//! One admitted typed constructor for a same-plan pure collection region.
//! Range preparation, lexical entry, ordinary finish and publication stay put.
use super::{CollectionExecution, Error, SourceBacking, State, add, charge, data, invalid};

fn fits(required: u64, allowance: Option<u64>) -> bool {
    allowance.is_some_and(|limit| required <= limit)
}

impl State {
    pub(in crate::native_artifact::program::runtime) fn fill_uniform_words(
        &mut self,
        ctx: &mut CollectionExecution<'_, '_, '_, '_>,
        site: usize,
        word: u64,
        count: u64,
    ) -> Result<bool, Error> {
        let descriptor = ctx.descriptor();
        let regions = &descriptor.metadata().uniform_word_collections;
        // Fund bounded proof/builder/source lookups before inspecting them.
        let entries = regions
            .len()
            .checked_add(self.sources.len())
            .and_then(|n| n.checked_add(self.builders.len()))
            .ok_or_else(|| invalid("uniform collection metadata overflow"))?;
        charge(&mut ctx.work(), super::super::amount(entries, 128, 256)?)?;
        let region = regions
            .iter()
            .find(|region| region.header() as usize == site)
            .ok_or_else(|| invalid("uniform fill outside its exact same-plan header"))?;
        if !region.filtered() {
            return Err(invalid("full uniform source requires native fill protocol"));
        }
        let id = region.collection();
        let iteration = region.iteration() as usize;
        let source = self
            .sources
            .get(iteration)
            .and_then(Option::as_ref)
            .ok_or_else(|| invalid("uniform fill has no current selected range"))?;
        if !matches!(source.backing, SourceBacking::U64Range { start: 0 })
            || count.checked_add(1) != Some(source.count)
            || ctx.progress().get(iteration * 2) != Some(&source.count)
            || ctx.progress().get(iteration * 2 + 1) != Some(&1)
            || source.index_offset != 0
            || source.element.is_some()
        {
            return Err(invalid("uniform fill outside its fresh inclusive range"));
        }
        let width = match region.kind() {
            mech_engine::native_artifact::NativeScalarKind::U8 if word <= u8::MAX as u64 => 1,
            mech_engine::native_artifact::NativeScalarKind::U16 if word <= u16::MAX as u64 => 2,
            mech_engine::native_artifact::NativeScalarKind::U32 if word <= u32::MAX as u64 => 4,
            mech_engine::native_artifact::NativeScalarKind::U64 => 8,
            _ => {
                return Err(invalid(
                    "uniform word differs from its proved unsigned kind",
                ));
            }
        };
        let output = &descriptor.metadata().plan.values()[descriptor.metadata().plan.collections()
            [id as usize]
            .output()
            .get() as usize];
        let node_count = data::output_nodes(output.definition().body(), count)?;
        let retained = self.retained_nodes()?;
        let old_live = self.live_bytes;
        let builder = self.builder(id)?;
        if builder.values.len() != 0
            || builder.values.capacity() != 0
            || builder.reservation.capacity_bytes() != 0
            || builder.payload_bytes != 0
            || builder.values.packed_width() != Some(width)
            || builder.join.is_some()
            || builder.query.is_some()
            || builder.histogram.is_some()
        {
            return Err(invalid("uniform fill requires its fresh packed builder"));
        }
        // Pure eligibility probes only. If any full construction/publication
        // bound cannot fit, retain the original per-element CFG and its error
        // precedence. No failed kernel or allocator is caught and retried.
        let geometry = (|| {
            let host_count = usize::try_from(count).ok()?;
            let bytes = count.checked_mul(width)?;
            let envelope = descriptor
                .metadata()
                .scalar_bytes
                .checked_add(bytes.checked_mul(4)?)?
                .checked_add(4096)?;
            let construct = bytes.checked_mul(16)?.checked_add(128)?;
            let allocation = bytes.checked_mul(16)?.checked_add(128)?;
            let finish = envelope.checked_mul(16)?.checked_add(256)?;
            let total_work = construct.checked_add(allocation)?.checked_add(finish)?;
            let component_compute = builder
                .compute
                .checked_add(construct)?
                .checked_add(finish)?;
            let cloned = builder.cloned_bytes.checked_add(bytes)?;
            let temporary = old_live.checked_add(bytes)?.checked_add(envelope)?;
            let output_bytes = builder
                .values
                .output_bytes(
                    bytes,
                    count,
                    host_count,
                    output.definition().dimension_parameters().len(),
                )
                .ok()?;
            Some((
                host_count,
                bytes,
                envelope,
                construct,
                allocation,
                total_work,
                component_compute,
                cloned,
                temporary,
                output_bytes,
            ))
        })();
        let Some((
            host_count,
            bytes,
            envelope,
            construct,
            allocation,
            total_work,
            compute,
            cloned,
            temporary,
            output_bytes,
        )) = geometry
        else {
            return Ok(false);
        };
        let limits = ctx.limits();
        if !fits(count, limits.max_output_elements)
            || !fits(cloned, limits.max_cloned_bytes)
            || !fits(compute, limits.max_compute_work)
            || !fits(temporary, limits.max_temporary_bytes)
            || !fits(output_bytes, limits.max_output_bytes)
            || !fits(add(node_count, retained)?, limits.max_retained_nodes)
        {
            return Ok(false);
        }
        let remaining = {
            let meter = ctx.work();
            meter.limit().saturating_sub(meter.used())
        };
        let Some((library_remaining, _)) = ctx.library().unscoped_remaining() else {
            return Ok(false);
        };
        if total_work > remaining || total_work > library_remaining {
            return Ok(false);
        }
        let memory = ctx.library().memory_budget();
        let probe_bytes = add(bytes, envelope)?;
        let probe = memory.reserve_capacity(probe_bytes);
        match probe {
            Ok(reservation) => drop(reservation),
            Err(mech_core::MemoryRuntimeError::BudgetExceeded { .. }) => return Ok(false),
            Err(error) => return Err(error.into()),
        }
        let probe = memory.reserve_nodes(add(node_count, 1)?);
        match probe {
            Ok(reservation) => drop(reservation),
            Err(mech_core::MemoryRuntimeError::BudgetExceeded { .. }) => return Ok(false),
            Err(error) => return Err(error.into()),
        }
        self.account(ctx, id, bytes, construct)?;
        charge(&mut ctx.work(), allocation)?;
        let builder = self.builder(id)?;
        let _replacement = memory.reserve_nodes(u64::from(host_count != 0))?;
        builder.reservation.resize_capacity(bytes)?;
        builder.values.reserve(host_count)?;
        builder
            .values
            .fill_unsigned(host_count, region.kind(), word)?;
        builder.payload_bytes = bytes;
        self.live_bytes = add(old_live, bytes)?;
        Ok(true)
    }
}

/// Build-owned storage layout. The output points to an empty typed Vec's exact
/// capacity. Only generated stores initialize it; finish publishes its length.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub(in crate::native_artifact) struct Frame {
    pub output: u64,
    pub count: u64,
    pub capacity_bytes: u64,
    pub word: u64,
    pub width: u64,
    pub epoch: u64,
    pub issued: u64,
    pub next: u64,
}
pub(super) struct Loan {
    // Uninitialized capacity has len0 until the complete native loop succeeds.
    values: data::CollectedValues,
    frame: Frame,
    header: usize,
    collection: u32,
    iteration: u32,
    token: u64,
    expected_word: u64,
    source_start: u64,
    source_count: u64,
    work: mech_engine::native_artifact::NativeLibraryWorkBudget,
    reservation: mech_core::ManagedMemoryReservation,
    _metadata: mech_core::ManagedMemoryReservation,
    _nodes: mech_core::ManagedNodeReservation,
}
impl State {
    pub(in crate::native_artifact::program::runtime) fn has_uniform_fill(&self) -> bool {
        self.native_fill.is_some()
    }
    pub(in crate::native_artifact::program::runtime) fn fresh_uniform_header(
        &self,
        collection: u32,
        iteration: u32,
        progress: &[u64],
    ) -> bool {
        self.native_fill.is_none()
            && self.native_read.is_none()
            && self.current == Some(collection)
            && self
                .sources
                .get(iteration as usize)
                .and_then(Option::as_ref)
                .is_some_and(|source| {
                    matches!(source.backing, SourceBacking::U64Range { .. })
                        && progress.get(iteration as usize * 2) == Some(&source.count)
                        && progress.get(iteration as usize * 2 + 1) == Some(&1)
                        && source.index_offset == 0
                        && source.element.is_none()
                })
    }
    pub(in crate::native_artifact::program::runtime) fn prepare_uniform_words(
        &mut self,
        ctx: &mut CollectionExecution<'_, '_, '_, '_>,
        region: &mech_engine::native_artifact::NativeUniformWordCollection,
        word: u64,
        count: u64,
        prior: mech_core::ResourceDemand,
    ) -> Result<Option<u64>, Error> {
        let descriptor = ctx.descriptor();
        let metadata = descriptor.metadata();
        let entries = self
            .builders
            .len()
            .checked_add(self.sources.len())
            .ok_or_else(|| invalid("native fill metadata overflow"))?;
        charge(&mut ctx.work(), super::super::amount(entries, 128, 256)?)?;
        let iteration = region.iteration() as usize;
        let id = region.collection();
        if !self.fresh_uniform_header(id, region.iteration(), ctx.progress())
            || ctx.ledger().current != Some(region.iteration())
        {
            return Ok(None);
        }
        let source = self.sources[iteration].as_ref().unwrap();
        let SourceBacking::U64Range { start } = source.backing else {
            return Ok(None);
        };
        let original_count = if region.filtered() {
            if start != 0 {
                return Ok(None);
            }
            count.checked_add(1)
        } else {
            Some(count)
        };
        if original_count != Some(source.count) {
            return Err(invalid("native fill count differs from prepared source"));
        }
        let original_source_count = source.count;
        let width = match region.kind() {
            mech_engine::native_artifact::NativeScalarKind::U8 if word <= u8::MAX as u64 => 1,
            mech_engine::native_artifact::NativeScalarKind::U16 if word <= u16::MAX as u64 => 2,
            mech_engine::native_artifact::NativeScalarKind::U32 if word <= u32::MAX as u64 => 4,
            mech_engine::native_artifact::NativeScalarKind::U64 => 8,
            _ => return Err(invalid("native fill word differs from proved kind")),
        };
        let output = &metadata.plan.values()
            [metadata.plan.collections()[id as usize].output().get() as usize];
        let node_count = data::output_nodes(output.definition().body(), count)?;
        let retained = self.retained_nodes()?;
        let old_live = self.live_bytes;
        let builder = self.builder(id)?;
        if builder.values.len() != 0
            || builder.values.capacity() != 0
            || builder.reservation.capacity_bytes() != 0
            || builder.payload_bytes != 0
            || builder.values.packed_width() != Some(width)
            || builder.join.is_some()
            || builder.query.is_some()
            || builder.histogram.is_some()
        {
            return Ok(None);
        }
        let geometry = (|| {
            let host_count = usize::try_from(count).ok()?;
            let bytes = count.checked_mul(width)?;
            if bytes > isize::MAX as u64 {
                return None;
            }
            let metadata_bytes = core::mem::size_of::<Loan>() as u64;
            let envelope = metadata
                .scalar_bytes
                .checked_add(bytes.checked_mul(4)?)?
                .checked_add(4096)?;
            // Store WORK and fixed loan metadata are fully paid before the
            // pointer leaves Rust; no shared-account renewal occurs in CFG.
            let construct = bytes
                .checked_mul(16)?
                .checked_add(128)?
                .checked_add(metadata_bytes.checked_mul(16)?)?
                .checked_add(256)?;
            let allocation = bytes.checked_mul(16)?.checked_add(128)?;
            let finish = envelope.checked_mul(16)?.checked_add(256)?;
            let total_work = construct.checked_add(allocation)?.checked_add(finish)?;
            let compute = builder
                .compute
                .checked_add(construct)?
                .checked_add(finish)?;
            let cloned = builder.cloned_bytes.checked_add(bytes)?;
            let temporary = prior
                .turn_peak_bytes
                .checked_add(old_live)?
                .checked_add(bytes)?
                .checked_add(envelope)?
                .checked_add(metadata_bytes)?;
            let nodes = prior
                .retained_nodes
                .checked_add(retained)?
                .checked_add(node_count)?
                .checked_add(2)?;
            let output_bytes = builder
                .values
                .output_bytes(
                    bytes,
                    count,
                    host_count,
                    output.definition().dimension_parameters().len(),
                )
                .ok()?;
            Some((
                host_count,
                bytes,
                metadata_bytes,
                envelope,
                construct,
                allocation,
                total_work,
                compute,
                cloned,
                temporary,
                nodes,
                output_bytes,
            ))
        })();
        let Some((
            host_count,
            bytes,
            metadata_bytes,
            envelope,
            construct,
            allocation,
            total_work,
            compute,
            cloned,
            temporary,
            nodes,
            output_bytes,
        )) = geometry
        else {
            return Ok(None);
        };
        let limits = ctx.limits();
        if !fits(count, limits.max_output_elements)
            || !fits(cloned, limits.max_cloned_bytes)
            || !fits(compute, limits.max_compute_work)
            || !fits(temporary, limits.max_temporary_bytes)
            || !fits(output_bytes, limits.max_output_bytes)
            || !fits(nodes, limits.max_retained_nodes)
        {
            return Ok(None);
        }
        let remaining = {
            let meter = ctx.work();
            meter.limit().saturating_sub(meter.used())
        };
        let Some((library_remaining, _)) = ctx.library().unscoped_remaining() else {
            return Ok(None);
        };
        if total_work > remaining || total_work > library_remaining {
            return Ok(None);
        }
        let memory = ctx.library().memory_budget();
        // The eligibility probe precedes any allocation or source omission.
        // A later allocator failure is an atomic error, never a cold retry.
        match memory.reserve_capacity(add(add(bytes, envelope)?, metadata_bytes)?) {
            Ok(probe) => drop(probe),
            Err(mech_core::MemoryRuntimeError::BudgetExceeded { .. }) => return Ok(None),
            Err(error) => return Err(error.into()),
        }
        match memory.reserve_nodes(add(node_count, 2)?) {
            Ok(probe) => drop(probe),
            Err(mech_core::MemoryRuntimeError::BudgetExceeded { .. }) => return Ok(None),
            Err(error) => return Err(error.into()),
        }
        let reservation = match memory.reserve_capacity(bytes) {
            Ok(value) => value,
            Err(mech_core::MemoryRuntimeError::BudgetExceeded { .. }) => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        let metadata_reservation = match memory.reserve_capacity(metadata_bytes) {
            Ok(value) => value,
            Err(mech_core::MemoryRuntimeError::BudgetExceeded { .. }) => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        let nodes = match memory.reserve_nodes(2) {
            Ok(value) => value,
            Err(mech_core::MemoryRuntimeError::BudgetExceeded { .. }) => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        self.account(ctx, id, bytes, construct)?;
        charge(&mut ctx.work(), allocation)?;
        let mut values = data::CollectedValues::new(output.definition().body());
        values.reserve(host_count)?;
        let address = values.unsigned_capacity_pointer()?;
        let token = ctx
            .ledger()
            .visits
            .checked_add(1)
            .ok_or_else(|| invalid("native fill epoch exhausted"))?;
        let loan = super::utf8_read::boxed(Loan {
            values,
            frame: Frame {
                output: address,
                count,
                capacity_bytes: bytes,
                word,
                width,
                epoch: token,
                issued: token,
                next: 0,
            },
            header: region.header() as usize,
            collection: id,
            iteration: region.iteration(),
            token,
            expected_word: word,
            source_start: start,
            source_count: original_source_count,
            work: ctx.library().clone(),
            reservation,
            _metadata: metadata_reservation,
            _nodes: nodes,
        })
        .ok_or_else(|| invalid("native fill loan allocation"))?;
        let pointer = (&loan.frame as *const Frame) as u64;
        self.native_fill = Some(loan);
        Ok(Some(pointer))
    }
    pub(in crate::native_artifact::program::runtime) fn finish_uniform_words(
        &mut self,
        ctx: &mut CollectionExecution<'_, '_, '_, '_>,
        site: usize,
        pointer: u64,
        status: u64,
    ) -> Result<(), Error> {
        let mut loan = self
            .native_fill
            .take()
            .ok_or_else(|| invalid("native fill has no loan"))?;
        let current_source = self
            .sources
            .get(loan.iteration as usize)
            .and_then(Option::as_ref);
        let source_current = current_source.is_some_and(|source| {
            matches!(source.backing, SourceBacking::U64Range { start } if start == loan.source_start)
                && source.count == loan.source_count && source.index_offset == 0
                && source.element.is_none()
                && ctx.progress().get(loan.iteration as usize * 2) == Some(&loan.source_count)
                && ctx.progress().get(loan.iteration as usize * 2 + 1) == Some(&1)
        });
        if !source_current
            || !loan.work.shares_account(ctx.library())
            || ctx.ledger().visits.checked_add(1) != Some(loan.token)
            || status != 0
            || pointer != (&loan.frame as *const Frame) as u64
            || loan.header != site
            || self.current != Some(loan.collection)
            || ctx.ledger().current != Some(loan.iteration)
            || loan.frame.epoch != loan.token
            || loan.frame.issued != loan.token
            || loan.token == 0
            || loan.frame.next != loan.frame.count
            || loan.frame.word != loan.expected_word
            || loan.frame.output != loan.values.unsigned_capacity_pointer()?
            || loan.values.packed_width() != Some(loan.frame.width)
            || loan.values.capacity() as u64 != loan.frame.count
            || loan.frame.count.checked_mul(loan.frame.width)
                != Some(loan.reservation.capacity_bytes())
            || loan.frame.capacity_bytes != loan.reservation.capacity_bytes()
        {
            return Err(invalid(
                "native fill completion changed its exact admitted bank",
            ));
        }
        // SAFETY: the genuine emitted kernel wrote every element, with exact
        // capacity/width/epoch and completed cursor checked above. Unsigned
        // element types admit every bit pattern. No other helper could run.
        unsafe {
            loan.values
                .set_initialized_unsigned_len(loan.frame.count as usize)?;
        }
        let old_live = self.live_bytes;
        let builder = self.builder(loan.collection)?;
        if builder.values.len() != 0
            || builder.values.capacity() != 0
            || builder.reservation.capacity_bytes() != 0
        {
            return Err(invalid("native fill builder changed during its loan"));
        }
        let Loan {
            values,
            reservation,
            ..
        } = *loan;
        let bytes = reservation.capacity_bytes();
        builder.values = values;
        builder.reservation = reservation;
        builder.payload_bytes = bytes;
        self.live_bytes = add(old_live, bytes)?;
        Ok(())
    }
}
