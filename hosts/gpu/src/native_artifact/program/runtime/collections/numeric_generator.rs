//! One exact synchronous generator loan, with initialized private F64 banks.
//! A decline discards all private bytes; ordinary source owns every error.
use super::*;
use crate::native_artifact::program::prepare::numeric_generators::{
    banks::{self, Frame, MAX_INPUTS, MatrixView},
    funding::Selected,
};
use mech_core::snapshot::{F64Bits, SequenceView};
use std::cell::UnsafeCell;

struct Input {
    value: u32,
    handle: NativeValueHandle,
    owner: Value,
}
pub(super) struct Loan {
    frame: UnsafeCell<Frame>,
    expected: Frame,
    cells: Vec<F64Bits>,
    scratch: Vec<F64Bits>,
    inputs: [Option<Input>; MAX_INPUTS],
    header: usize,
    collection: u32,
    iteration: u32,
    start: u64,
    compute: u64,
    cloned: u64,
    work: mech_engine::native_artifact::NativeLibraryWorkBudget,
    reservation: ManagedMemoryReservation,
    _nodes: ManagedNodeReservation,
}
fn fits(n: u64, cap: Option<u64>) -> bool {
    cap.is_some_and(|cap| n <= cap)
}
fn allocate(count: u64) -> Result<Vec<F64Bits>, Error> {
    let count = usize::try_from(count).map_err(|_| invalid("generator bank exceeds host"))?;
    let mut cells = Vec::new();
    cells
        .try_reserve_exact(count)
        .map_err(|_| invalid("generator bank allocation"))?;
    let mut cells = admitted_capacity(cells, count, "generator bank capacity differs")?;
    cells.resize(count, F64Bits::from_bits(0));
    Ok(cells)
}
fn input_view(owner: &Value, extents: [u64; 2]) -> Option<MatrixView> {
    // The enclosing loan checked the exact same-plan fixed schema/key and
    // validated this genuine canonical owner before borrowing these cells.
    let ValueData::Matrix(matrix) = owner.data() else {
        return None;
    };
    let SequenceView::F64(cells) = matrix.elements() else {
        return None;
    };
    let count = extents[0].checked_mul(extents[1])?;
    (count == cells.len() as u64).then_some(MatrixView {
        input: cells.as_ptr() as u64,
        count,
        rows: extents[0],
        columns: extents[1],
    })
}
impl State {
    pub(in crate::native_artifact::program::runtime) fn has_numeric_generator(&self) -> bool {
        self.native_generator.is_some()
    }
    pub(in crate::native_artifact::program::runtime) fn prepare_numeric_generator(
        &mut self,
        ctx: &mut CollectionExecution<'_, '_, '_, '_>,
        selected: &Selected,
        prior: mech_core::ResourceDemand,
    ) -> Result<Option<u64>, Error> {
        let region = &selected.region;
        let budget = &selected.budget;
        if !self.fresh_numeric_header(region.collection, region.iteration, ctx.progress())
            || ctx.ledger().current != Some(region.iteration)
        {
            return Ok(None);
        }
        let source = self.sources[region.iteration as usize].as_ref().unwrap();
        if !matches!(source.backing, SourceBacking::U64Range { start } if start == region.start)
            || source.count != region.count
        {
            return Ok(None);
        }
        let metadata = ctx.descriptor().metadata();
        let output_definition = metadata.plan.values()[region.output as usize].definition();
        let count = region.extents[0]
            .checked_mul(region.extents[1])
            .ok_or_else(|| invalid("generator result size overflow"))?;
        let bytes = mul(count, 8)?;
        let scratch_bytes = mul(selected.banks.elements, 8)?;
        let private = add(add(bytes, scratch_bytes)?, size_of::<Loan>() as u64)?;
        let physical = add(private, budget.temporary)?;
        let retained = self.retained_nodes()?;
        let old_live = self.live_bytes;
        let builder = self.builder(region.collection)?;
        if builder.values.len() != 0
            || builder.values.capacity() != 0
            || builder.reservation.capacity_bytes() != 0
            || builder.payload_bytes != 0
            || builder.block_rows.is_row_major()
            || builder.block_columns != 0
            || builder
                .block_rows
                .rows()
                .is_some_and(|n| n != region.extents[0])
            || builder.values.packed_width() != Some(8)
            || builder.join.is_some()
            || builder.query.is_some()
            || builder.histogram.is_some()
        {
            return Ok(None);
        }
        let output_bound = builder.values.output_bytes(
            bytes,
            count,
            count as usize,
            output_definition.dimension_parameters().len(),
        )?;
        let compute = add(
            add(add(builder.compute, budget.compute)?, budget.setup)?,
            budget.native,
        )?;
        let cloned = add(builder.cloned_bytes, budget.cloned)?;
        let temporary = add(add(prior.turn_peak_bytes, old_live)?, physical)?;
        let nodes = add(add(prior.retained_nodes, retained)?, budget.nodes)?;
        let caps = ctx.limits();
        if !fits(budget.elements, caps.max_output_elements)
            || !fits(output_bound.max(budget.output), caps.max_output_bytes)
            || !fits(temporary, caps.max_temporary_bytes)
            || !fits(cloned, caps.max_cloned_bytes)
            || !fits(nodes, caps.max_retained_nodes)
            || !fits(compute, caps.max_compute_work)
            || !fits(budget.comparison, caps.max_comparison_work)
            || !fits(0, caps.max_scalar_instructions)
            || !fits(0, caps.max_transfer_bytes)
            || !fits(0, caps.max_storage_buffer_bytes)
            || caps.max_storage_bindings.is_none()
        {
            return Ok(None);
        }
        let admission = add(
            add(add(budget.setup, budget.native)?, budget.compute)?,
            budget.replay,
        )?;
        let remaining = {
            let w = ctx.work();
            w.limit().saturating_sub(w.used())
        };
        if remaining < admission
            || ctx
                .library()
                .unscoped_remaining()
                .is_none_or(|(w, c)| w < admission || c < budget.comparison.saturating_mul(2))
        {
            return Ok(None);
        }
        let memory = ctx.library().memory_budget();
        let reservation = match memory.reserve_capacity(physical) {
            Ok(v) => v,
            Err(mech_core::MemoryRuntimeError::BudgetExceeded { .. }) => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        let probe = match memory.reserve_nodes(budget.nodes) {
            Ok(v) => v,
            Err(mech_core::MemoryRuntimeError::BudgetExceeded { .. }) => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        drop(probe);
        let nodes = memory.reserve_nodes(4)?;
        // The exact header paid the cheap inventory; the complete source
        // envelope now pays every fallible owner/schema/shape query and all
        // initialized staging. Full ordinary replay credit remains untouched.
        charge(&mut ctx.work(), budget.setup)?;
        let mut inputs: [Option<Input>; MAX_INPUTS] = std::array::from_fn(|_| None);
        let mut views = [MatrixView::default(); MAX_INPUTS];
        for (at, source) in selected.banks.inputs.iter().enumerate() {
            if !ctx.arena_entry_is_current(source.value)? {
                return Ok(None);
            }
            let handle = ctx.arena_current_handle(source.value)?;
            let owner = ctx.arena_get(handle)?;
            let ty = &metadata.types[source.value as usize];
            if owner.schema() != ty.schema
                || owner.schema_key() != ty.key
                || owner.memory_budget_retained_bytes(&memory).is_none()
                || owner.memory_budget_retained_nodes(&memory).is_none()
                || owner.validate_against(&metadata.schemas).is_err()
            {
                return Ok(None);
            }
            let Some(view) = input_view(owner, source.extents) else {
                return Ok(None);
            };
            views[at] = view;
            inputs[at] = Some(Input {
                value: source.value,
                handle,
                owner: owner.clone(),
            });
        }
        let mut cells = allocate(count)?;
        let mut scratch = allocate(selected.banks.elements)?;
        let epoch = ctx
            .ledger()
            .visits
            .checked_add(1)
            .ok_or_else(|| invalid("generator epoch exhausted"))?;
        // Native instructions and failure checks are paid before the call.
        charge(&mut ctx.work(), budget.native)?;
        let frame = Frame {
            output: cells.as_mut_ptr() as u64,
            output_elements: count,
            scratch: scratch.as_mut_ptr() as u64,
            scratch_elements: scratch.len() as u64,
            epoch,
            issued: epoch,
            completed: 0,
            source_count: region.count,
            inputs: views,
            ..Default::default()
        };
        let loan = super::utf8_read::boxed(Loan {
            frame: UnsafeCell::new(frame),
            expected: frame,
            cells,
            scratch,
            inputs,
            header: region.header as usize,
            collection: region.collection,
            iteration: region.iteration,
            start: region.start,
            compute: budget.compute,
            cloned: budget.cloned,
            work: ctx.library().clone(),
            reservation,
            _nodes: nodes,
        })
        .ok_or_else(|| invalid("generator loan allocation"))?;
        // The native body alone mutates this synchronous cell. No Rust Frame
        // reference crosses its execution; the independent shadow is immutable.
        let pointer = loan.frame.get() as u64;
        self.native_generator = Some(loan);
        Ok(Some(pointer))
    }
    pub(in crate::native_artifact::program::runtime) fn finish_numeric_generator(
        &mut self,
        ctx: &mut CollectionExecution<'_, '_, '_, '_>,
        selected: &Selected,
        pointer: u64,
        status: u64,
    ) -> Result<bool, Error> {
        let mut loan = self
            .native_generator
            .take()
            .ok_or_else(|| invalid("generator finish has no loan"))?;
        let region = &selected.region;
        // Generated code has returned and no concurrent worker can own this
        // loan. Read the completed cell once, without lending a shared Frame.
        let frame = unsafe { *loan.frame.get() };
        let mut actual = frame;
        actual.completed = 0;
        actual.traversals = [0; banks::MAX_LOOPS];
        if status > 1
            || pointer != loan.frame.get() as u64
            || actual != loan.expected
            || loan.header != region.header as usize
            || loan.collection != region.collection
            || loan.iteration != region.iteration
            || self.current != Some(loan.collection)
            || ctx.ledger().current != Some(loan.iteration)
            || ctx.ledger().visits.checked_add(1) != Some(loan.expected.epoch)
            || !loan.work.shares_account(ctx.library())
            || loan.reservation.budget() != ctx.library().memory_budget()
            || frame.output != loan.cells.as_mut_ptr() as u64
            || frame.scratch != loan.scratch.as_mut_ptr() as u64
            || loan.cells.len() as u64 != loan.expected.output_elements
            || loan.cells.capacity() != loan.cells.len()
            || loan.scratch.len() as u64 != loan.expected.scratch_elements
            || loan.scratch.capacity() != loan.scratch.len()
            || frame
                .traversals
                .iter()
                .zip(selected.budget.maximum)
                .any(|(&n, cap)| n > cap)
        {
            return Err(invalid("generator changed its exact admitted loan"));
        }
        let source = self.sources[loan.iteration as usize]
            .as_ref()
            .ok_or_else(|| invalid("generator source disappeared"))?;
        if !matches!(source.backing, SourceBacking::U64Range { start } if start == loan.start)
            || source.count != loan.expected.source_count
            || source.index_offset != 0
            || source.element.is_some()
            || ctx.progress().get(loan.iteration as usize * 2) != Some(&loan.expected.source_count)
            || ctx.progress().get(loan.iteration as usize * 2 + 1) != Some(&1)
        {
            return Err(invalid("generator original source frontier changed"));
        }
        for (at, pin) in loan
            .inputs
            .iter()
            .enumerate()
            .filter_map(|(at, pin)| pin.as_ref().map(|pin| (at, pin)))
        {
            if ctx.arena_current_handle(pin.value)? != pin.handle
                || !ctx.arena_get(pin.handle)?.shares_frozen_storage(&pin.owner)
                || input_view(&pin.owner, selected.banks.inputs[at].extents)
                    != Some(loan.expected.inputs[at])
            {
                return Err(invalid("generator immutable input changed"));
            }
        }
        if status == 1 {
            if frame.completed != 0 || frame.traversals != [0; banks::MAX_LOOPS] {
                return Err(invalid("declined generator claimed completion"));
            }
            return Ok(false);
        }
        if frame.completed != region.count || frame.traversals[0] != region.count + 1 {
            return Err(invalid("generator success is incomplete"));
        }
        let builder = self.builder(loan.collection)?;
        if builder.values.len() != 0
            || builder.values.capacity() != 0
            || builder.reservation.capacity_bytes() != 0
            || builder.payload_bytes != 0
            || builder.block_columns != 0
            || builder.block_rows.is_row_major()
        {
            return Err(invalid("generator builder changed during loan"));
        }
        self.account(ctx, loan.collection, loan.cloned, loan.compute)?;
        ctx.library()
            .charge(0, selected.budget.comparison)
            .map_err(
                |error| crate::native_artifact::NativeExecutionError::LibraryWork {
                    phase: Phase::Completion,
                    error,
                },
            )?;
        let Loan {
            cells,
            scratch,
            inputs,
            mut reservation,
            _nodes,
            ..
        } = *loan;
        let bytes = mul(cells.len() as u64, 8)?;
        let output_reservation = reservation.split_capacity(bytes)?;
        // Private scratch and pins retire before their coexistence credit.
        drop(scratch);
        drop(inputs);
        drop(reservation);
        drop(_nodes);
        let old_live = self.live_bytes;
        let builder = self.builder(region.collection)?;
        builder.values = data::CollectedValues::F64(cells);
        builder.reservation = output_reservation;
        builder.payload_bytes = bytes;
        builder.block_rows = BlockRows::RowMajor(region.extents[0]);
        builder.block_columns = region.extents[1];
        self.live_bytes = add(old_live, bytes)?;
        Ok(true)
    }
}
