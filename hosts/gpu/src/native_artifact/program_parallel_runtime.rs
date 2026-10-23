//! Scoped execution of original generated comprehension bodies. This module
//! schedules entries and transports immutable results; it evaluates no CFG.
use super::{Context, NativeProgramExecutionError as Error, invalid, amount};
use crate::native_artifact::{NativeExecutionPhase as Phase, NativeProgramExecutionLimits,
    program_abi::{self as abi, Frame}};
use mech_core::{ManagedMemoryReservation, Value};
use mech_engine::native_artifact::{NativeLibraryWorkBudget, NativeWorkerBudgetError};

#[cfg(test)]
thread_local! { pub(super) static TEST_WORKERS: std::cell::Cell<Option<usize>> = const { std::cell::Cell::new(None) }; }

#[derive(Clone)]
pub(super) enum Capture { Word(u32, u64), Value(u32, Value) }
pub(super) enum Yield { Word(u64), Value(Value) }
pub(super) struct Receipt {
    pub value: Yield,
    pub previous: u32,
    pub visits: u64,
    pub hash_regions: u64,
    pub hash_updates: u64,
    pub packed_regions: u64,
    pub packed_updates: u64,
    pub packed_ssa_regions: u64,
}
pub(super) struct Worker {
    pub region: usize,
    pub index: u64,
    pub count: u64,
    pub result: Option<Receipt>,
}
pub(super) struct Batch {
    region: usize,
    next: u64,
    count: u64,
    results: std::vec::IntoIter<Result<Receipt, Error>>,
    _reservation: ManagedMemoryReservation,
}

fn budget_error(error: NativeWorkerBudgetError, origin: mech_engine::native_artifact::NativeMatchOrigin) -> Error {
    match error {
        NativeWorkerBudgetError::Work(error) => Error::Collection { origin, error },
        NativeWorkerBudgetError::Memory(error) => error.into(),
    }
}

impl Context<'_, '_> {
    pub(super) fn parallel_helper(&mut self, site: usize, op: u64, args: [u64; 3]) -> Result<Option<u64>, Error> {
        match op {
            abi::PARALLEL_RUN => self.run_parallel(site, args),
            abi::PARALLEL_TAKE => self.take_parallel(site, args),
            abi::PARALLEL_YIELD => self.yield_parallel(site, args),
            abi::PARALLEL_ENTER => {
                let worker = self.worker.as_ref().ok_or_else(|| invalid("worker entry without owned child state"))?;
                let region = self.control.descriptor.parallel.regions.get(worker.region)
                    .ok_or_else(|| invalid("worker entry proof absent"))?;
                if site != region.begin as usize || args != [worker.region as u64, worker.index, worker.count]
                    || worker.result.is_some() || self.control.previous.is_some()
                    || self.control.current.is_some() || self.control.completed {
                    return Err(invalid("worker entry differs from constructor boundary"));
                }
                self.control.work.charge(Phase::Setup, 256)?;
                Ok(None)
            }
            _ => Err(invalid("unknown parallel entry operation")),
        }
    }

    fn run_parallel(&mut self, site: usize, args: [u64; 3]) -> Result<Option<u64>, Error> {
        let descriptor = self.control.descriptor;
        let id = usize::try_from(args[2]).map_err(|_| invalid("parallel region index overflow"))?;
        let region = descriptor.parallel.regions.get(id).ok_or_else(|| invalid("parallel region outside descriptor"))?;
        if region.initialize as usize != site || self.control.previous != Some(site as u32)
            || self.control.current != Some(region.iteration) || !self.consumed {
            return Err(invalid("parallel dispatch outside exact initialized collection"));
        }
        if self.worker.is_some() || self.activation_phase { return Ok(Some(0)); }
        if self.parallel_batch.is_some() { return Err(invalid("overlapping native parallel batches")); }
        let count = self.control.progress[region.iteration as usize * 2];
        let workers = std::thread::available_parallelism().map_or(1, usize::from).min(32);
        #[cfg(test)]
        let workers = TEST_WORKERS.with(|override_| override_.get().unwrap_or(workers)).min(32);
        if count < 2 || count > workers as u64 { return Ok(Some(0)); }
        let Some((compute, _)) = self.library.unscoped_remaining() else { return Ok(Some(0)); };
        if compute == 0 { return Ok(Some(0)); }
        if args[0] == 0 || args[1] == 0 { return Err(invalid("parallel dispatch lost checked entry")); }
        // SAFETY: the same checked generated function passes its own address
        // and its currently borrowed Frame. The synchronous outer invocation
        // retains the code owner until every scoped child has joined.
        let frame = unsafe { &*(args[1] as *const Frame) };
        let own_context = (self as *mut Self).cast::<std::ffi::c_void>();
        if frame.base.context != own_context || frame.base.version != abi::VERSION
            || frame.base.local_count < descriptor.plan.values().len() as u64
            || frame.base.locals.is_null() || frame.phase == abi::PHASE_WORKER {
            return Err(invalid("parallel dispatch passed a foreign frame"));
        }
        let memory = self.library.memory_budget().with_limits(
            self.limits.max_temporary_bytes.ok_or_else(|| invalid("parallel requires finite bytes"))?,
            self.limits.max_retained_nodes.ok_or_else(|| invalid("parallel requires finite nodes"))?,
        );
        // Scope admission includes native thread stacks and all transport
        // containers. Captured canonical payloads retain their existing shared
        // account and are never copied into an unaccounted worker heap.
        const STACK: usize = 2 * 1024 * 1024;
        // Keep a separate MiB for maintained Rust helpers/unwinding. A caller
        // may raise its normal native-stack allowance; that cannot silently
        // enlarge the bounded stack reserved for these workers.
        const CALLBACK_HEADROOM: usize = 1024 * 1024;
        if descriptor.stack > (STACK - CALLBACK_HEADROOM) as u64 { return Ok(Some(0)); }
        let count_usize = count as usize;
        let metadata = amount(count_usize, (std::mem::size_of::<Receipt>() + 1024) as u64, 1024)?
            .checked_add(amount(region.captures.len(),
                (std::mem::size_of::<Capture>() as u64) * (count + 1), 0)?)
            .ok_or_else(|| invalid("parallel metadata overflow"))?;
        self.control.work.charge(Phase::Setup, metadata.checked_mul(8)
            .and_then(|n| n.checked_add(4096)).ok_or_else(|| invalid("parallel setup overflow"))?)?;
        let reservation = match memory.reserve_capacity(metadata) { Ok(value) => value, Err(_) => return Ok(Some(0)) };
        let stacks = match memory.reserve_capacity(count.checked_mul((STACK + 65536) as u64)
            .ok_or_else(|| invalid("parallel stack admission overflow"))?) {
            Ok(value) => value, Err(_) => return Ok(Some(0)),
        };
        let mut captures = Vec::new();
        captures.try_reserve_exact(region.captures.len()).map_err(|_| invalid("parallel capture allocation"))?;
        for &slot in &region.captures {
            if descriptor.plan.values()[slot as usize].scalar_kind().is_some() {
                // SAFETY: checked slot is within this exact live local lane.
                captures.push(Capture::Word(slot, unsafe { *frame.base.locals.add(slot as usize) }));
            } else {
                captures.push(Capture::Value(slot, self.arena.copy_current(slot, &mut self.control.work)?));
            }
        }
        let origin = descriptor.plan.collections()[region.collection as usize].origin();
        let (compute, comparisons) = self.library.unscoped_remaining().ok_or_else(|| invalid("parallel scope changed during admission"))?;
        let group = self.library.reserve_worker_group_with_memory(compute, comparisons, &memory)
            .map_err(|error| budget_error(error, origin))?;
        let limits = NativeProgramExecutionLimits { execution: self.execution_limits, library: self.limits };
        // SAFETY: this address was emitted by the exact entry currently holding
        // the caller's checked JIT/linked code owner. No user callback or source
        // value supplies it, and scoped joins prevent code-owner escape.
        let entry: abi::Entry = unsafe { std::mem::transmute(args[0] as usize) };
        let outcomes = std::thread::scope(|scope| -> Result<Vec<Result<Receipt, Error>>, Error> {
            let mut handles = Vec::new();
            handles.try_reserve_exact(count_usize).map_err(|_| invalid("parallel handle allocation"))?;
            let mut results = Vec::new();
            results.try_reserve_exact(count_usize).map_err(|_| invalid("parallel result allocation"))?;
            let mut jobs = Vec::new();
            jobs.try_reserve_exact(count_usize).map_err(|_| invalid("parallel job allocation"))?;
            for index in 0..count {
                let loan = group.worker_loan_with_memory(&memory).map_err(|error| budget_error(error, origin))?;
                let mut worker_captures = Vec::new();
                worker_captures.try_reserve_exact(captures.len()).map_err(|_| invalid("worker capture allocation"))?;
                worker_captures.extend(captures.iter().cloned());
                jobs.push((index, loan, worker_captures));
            }
            let mut spawn_failed = false;
            for (index, loan, worker_captures) in jobs {
                let memory = memory.clone();
                let handle = std::thread::Builder::new().stack_size(STACK).spawn_scoped(scope, move || {
                    let library = NativeLibraryWorkBudget::from_worker_loan(loan)
                        .map_err(|error| Error::Collection { origin, error })?;
                    super::session::Prepared::run_worker(descriptor, entry, limits, &memory,
                        library, Worker { region: id, index, count, result: None }, worker_captures)
                });
                match handle {
                    Ok(handle) => handles.push(handle),
                    Err(_) => { spawn_failed = true; break; }
                }
            }
            // Always explicitly join every launched thread, including after a
            // later spawn failure. No unjoined panic may unwind through FFI.
            for handle in handles {
                results.push(handle.join().unwrap_or_else(|_| Err(invalid("native worker panicked"))));
            }
            if spawn_failed { Err(invalid("native worker creation failed")) } else { Ok(results) }
        });
        // All worker-local ledgers have retired before reconciliation. Work
        // already spent stays charged even when any worker failed.
        group.finish().map_err(|error| Error::Collection { origin, error })?;
        drop(stacks);
        let results = outcomes?;
        for result in results.iter().filter_map(|result| result.as_ref().ok()) {
            self.control.visits = self.control.visits.checked_add(result.visits).ok_or_else(|| invalid("parallel visit overflow"))?;
            self.hash_table_regions = self.hash_table_regions.checked_add(result.hash_regions).ok_or_else(|| invalid("parallel region overflow"))?;
            self.hash_table_updates = self.hash_table_updates.checked_add(result.hash_updates).ok_or_else(|| invalid("parallel update overflow"))?;
            self.packed_map_regions = self.packed_map_regions.checked_add(result.packed_regions).ok_or_else(|| invalid("parallel packed region overflow"))?;
            self.packed_map_updates = self.packed_map_updates.checked_add(result.packed_updates).ok_or_else(|| invalid("parallel packed update overflow"))?;
            self.packed_ssa_regions = self.packed_ssa_regions.checked_add(result.packed_ssa_regions).ok_or_else(|| invalid("parallel SSA region overflow"))?;
        }
        self.parallel_batch = Some(Batch { region: id, next: 0, count, results: results.into_iter(), _reservation: reservation });
        Ok(Some(1))
    }

    fn take_parallel(&mut self, site: usize, args: [u64; 3]) -> Result<Option<u64>, Error> {
        let id = usize::try_from(args[0]).map_err(|_| invalid("parallel take region overflow"))?;
        let descriptor = self.control.descriptor;
        let region = descriptor.parallel.regions.get(id).ok_or_else(|| invalid("parallel take region"))?;
        if args[1..] != [0; 2] || region.body as usize != site
            || self.control.previous != Some(site as u32) || !self.consumed {
            return Err(invalid("parallel take outside original element"));
        }
        let Some(batch) = &mut self.parallel_batch else { return Ok(Some(0)); };
        if batch.region != id || self.control.current != Some(region.iteration)
            || batch.count.checked_sub(self.control.progress[region.iteration as usize * 2]) != Some(batch.next) {
            return Err(invalid("parallel result belongs to another collection visit"));
        }
        // Surface a failed body only at its original source element. Earlier
        // pushes retain their own output/cap error ordering.
        let receipt = batch.results.next().ok_or_else(|| invalid("parallel result replay"))??;
        batch.next = batch.next.checked_add(1).ok_or_else(|| invalid("parallel result index overflow"))?;
        if !region.contains(receipt.previous) || !super::super::program_parallel_layout::successors(
            descriptor.plan.blocks()[receipt.previous as usize].terminator()).contains(&Some(region.push)) {
            return Err(invalid("parallel receipt lacks original push frontier"));
        }
        match receipt.value {
            Yield::Word(word) => self.write_borrowed_word(0, word)?,
            Yield::Value(value) => { self.arena.put(region.output, &value, &mut self.control.work)?; }
        }
        // The private receipt certifies execution of this exact body under a
        // child loan. Only its original push successor becomes admissible.
        self.control.previous = Some(receipt.previous);
        self.consumed = true;
        if self.parallel_batch.as_ref().is_some_and(|batch| batch.results.len() == 0) { self.parallel_batch = None; }
        Ok(Some(1))
    }

    fn yield_parallel(&mut self, site: usize, args: [u64; 3]) -> Result<Option<u64>, Error> {
        let worker = self.worker.as_ref().ok_or_else(|| invalid("worker yield in parent invocation"))?;
        let descriptor = self.control.descriptor;
        let region = descriptor.parallel.regions.get(worker.region).ok_or_else(|| invalid("worker proof region"))?;
        let previous = self.control.previous.ok_or_else(|| invalid("worker yield before body"))?;
        if args[1] != worker.region as u64 || args[2] != 0 || region.push as usize != site
            || worker.result.is_some() || self.control.current != Some(region.iteration)
            || !region.contains(previous)
            || !super::super::program_parallel_layout::successors(descriptor.plan.blocks()[previous as usize].terminator()).contains(&Some(region.push))
            || (self.action_required(previous as usize) && !self.consumed)
            || self.closed_tail.is_some() || self.borrowed_strings.is_some() {
            return Err(invalid("worker escaped its exact result boundary"));
        }
        let value = if descriptor.plan.values()[region.output as usize].scalar_kind().is_some() {
            Yield::Word(args[0])
        } else {
            if args[0] != 0 { return Err(invalid("managed worker result has scalar payload")); }
            Yield::Value(self.arena.copy_current(region.output, &mut self.control.work)?)
        };
        self.worker.as_mut().unwrap().result = Some(Receipt { value, previous,
            visits: self.control.visits, hash_regions: self.hash_table_regions, hash_updates: self.hash_table_updates,
            packed_regions: self.packed_map_regions, packed_updates: self.packed_map_updates,
            packed_ssa_regions: self.packed_ssa_regions });
        Ok(Some(1))
    }
}
