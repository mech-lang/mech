//! Scoped execution of original generated comprehension bodies. This module
//! schedules entries and transports immutable results; it evaluates no CFG.
use super::{Context, NativeProgramExecutionError as Error, amount, invalid};
use crate::native_artifact::{
    NativeExecutionPhase as Phase, NativeProgramExecutionLimits,
    program_abi::{self as abi, Frame},
};
use mech_core::{ManagedMemoryReservation, ManagedNodeReservation, Value};
use mech_engine::native_artifact::{NativeLibraryWorkBudget, NativeWorkerBudgetError};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};

#[path = "program_parallel_schedule.rs"]
mod schedule;

#[derive(Default)]
struct Activity {
    current: AtomicUsize,
    peak: AtomicUsize,
}
struct Active<'a>(&'a Activity);
impl Activity {
    fn enter(&self) -> Active<'_> {
        // At most the admitted bounded task count can own a guard. The scoped
        // parent reads the peak only after joining every child.
        let current = self.current.fetch_add(1, Ordering::Relaxed) + 1;
        self.peak.fetch_max(current, Ordering::Relaxed);
        Active(self)
    }
}
impl Drop for Active<'_> {
    fn drop(&mut self) {
        self.0.current.fetch_sub(1, Ordering::Relaxed);
    }
}

#[cfg(test)]
thread_local! { pub(super) static TEST_WORKERS: std::cell::Cell<Option<usize>> = const { std::cell::Cell::new(None) }; }
#[cfg(test)]
thread_local! { pub(super) static TEST_DISPATCH_ORDER: std::cell::RefCell<Vec<(usize, u64)>> = const { std::cell::RefCell::new(Vec::new()) }; }

#[derive(Clone)]
pub(super) enum Capture {
    Word(u32, u64),
    Value(u32, Value),
}
pub(super) enum Yield {
    Word(u64),
    Value(Value),
}
pub(super) struct Receipt {
    pub value: Yield,
    pub previous: u32,
    pub visits: u64,
    pub hash_regions: u64,
    pub hash_updates: u64,
    pub packed_regions: u64,
    pub direct_lookup_regions: u64,
    pub packed_updates: u64,
    pub packed_ssa_regions: u64,
    pub guarded_loop_regions: u64,
    pub guarded_call_regions: u64,
    pub frozen_map_regions: u64,
    pub frozen_map_reads: u64,
    pub worker_credit_claims: u64,
    pub worker_credit_batches: u64,
    pub worker_credit_reclaims: u64,
    pub fused_string_map_bytes: u64,
}
pub(super) struct Worker {
    pub region: usize,
    pub index: u64,
    pub count: u64,
    pub result: Option<Receipt>,
}
struct PendingRegion {
    region: usize,
    next: u64,
    count: u64,
    source: Value,
}
pub(super) struct Batch {
    regions: VecDeque<PendingRegion>,
    results: std::vec::IntoIter<Result<Receipt, Error>>,
    _reservation: ManagedMemoryReservation,
    _nodes: ManagedNodeReservation,
}
struct ReadyRegion {
    region: usize,
    count: u64,
    source: Value,
    captures: Vec<Capture>,
}

fn transport<T>(count: usize, reason: &'static str) -> Result<Vec<T>, Error> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(count)
        .map_err(|_| invalid(reason))?;
    super::collection::admitted_capacity(values, count, reason)
}

fn scheduler_nodes(workers: usize, regions: usize, captures: usize) -> Result<u64, Error> {
    // Count structural carriers/backings, not their byte size: fixed scheduler
    // containers; Ready/Pending region, source/ID and capture-vector roots;
    // each task's escrow/local-account owner, scope/thread closure, handle,
    // job, result, receipt and capture-vector roots; then Capture entries.
    // Captured/result canonical payloads keep their independent shared claims.
    amount(regions, 8, 17)?
        .checked_add(amount(workers, 16, 0)?)
        .and_then(|nodes| nodes.checked_add(amount(captures, workers as u64 + 1, 0).ok()?))
        .ok_or_else(|| invalid("parallel scheduler node count overflow"))
}

fn budget_error(
    error: NativeWorkerBudgetError,
    origin: mech_engine::native_artifact::NativeMatchOrigin,
) -> Error {
    match error {
        NativeWorkerBudgetError::Work(error) => Error::Collection { origin, error },
        NativeWorkerBudgetError::Memory(error) => error.into(),
    }
}

pub(super) fn worker_setup_fits(
    descriptor: &crate::native_artifact::NativeProgramDescriptor,
    memory: &mech_core::ManagedMemoryBudget,
    task_count: u64,
) -> Result<bool, Error> {
    let nodes = super::session::Prepared::worker_setup_nodes(descriptor)?
        .checked_mul(task_count)
        .ok_or_else(|| invalid("parallel worker setup node count overflow"))?;
    let bytes = super::session::Prepared::worker_setup_bytes(descriptor)?
        .checked_add(NativeLibraryWorkBudget::worker_account_metadata_bytes())
        .and_then(|bytes| bytes.checked_mul(task_count))
        .and_then(|bytes| bytes.checked_add(NativeLibraryWorkBudget::worker_group_metadata_bytes()))
        .ok_or_else(|| invalid("parallel worker setup byte count overflow"))?;
    // Keep both pending claims until the joint feasibility check succeeds.
    // A denied byte claim must also release the preceding node claim.
    let Ok(_nodes) = memory.reserve_nodes(nodes) else {
        return Ok(false);
    };
    let Ok(_bytes) = memory.reserve_capacity(bytes) else {
        return Ok(false);
    };
    Ok(true)
}

impl Context<'_, '_> {
    pub(super) fn parallel_helper(
        &mut self,
        site: usize,
        op: u64,
        args: [u64; 3],
    ) -> Result<Option<u64>, Error> {
        match op {
            abi::PARALLEL_RUN => self.run_parallel(site, args),
            abi::PARALLEL_TAKE => self.take_parallel(site, args),
            abi::PARALLEL_YIELD => self.yield_parallel(site, args),
            abi::PARALLEL_ENTER => {
                let worker = self
                    .worker
                    .as_ref()
                    .ok_or_else(|| invalid("worker entry without owned child state"))?;
                let region = self
                    .control
                    .descriptor
                    .metadata()
                    .parallel
                    .regions
                    .get(worker.region)
                    .ok_or_else(|| invalid("worker entry proof absent"))?;
                if site != region.begin as usize
                    || args != [worker.region as u64, worker.index, worker.count]
                    || worker.result.is_some()
                    || self.control.previous.is_some()
                    || self.control.current.is_some()
                    || self.control.completed
                {
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
        let region = descriptor
            .metadata()
            .parallel
            .regions
            .get(id)
            .ok_or_else(|| invalid("parallel region outside descriptor"))?;
        if region.initialize as usize != site
            || self.control.previous != Some(site as u32)
            || self.control.current != Some(region.iteration)
            || !self.consumed
        {
            return Err(invalid(
                "parallel dispatch outside exact initialized collection",
            ));
        }
        if self.worker.is_some() || self.activation_phase {
            return Ok(Some(0));
        }
        let count = self.control.progress[region.iteration as usize * 2];
        if let Some(batch) = &self.parallel_batch {
            let pending = batch
                .regions
                .front()
                .ok_or_else(|| invalid("empty parallel group"))?;
            if pending.region != id || pending.next != 0 || pending.count != count {
                return Err(invalid(
                    "future parallel initializer differs from queued source",
                ));
            }
            let source = self
                .arena
                .copy_current(region.source, &mut self.control.work)?;
            if !source.shares_frozen_storage(&pending.source) {
                return Err(invalid("future parallel source changed identity"));
            }
            return Ok(Some(2));
        }
        let workers = std::thread::available_parallelism()
            .map_or(1, usize::from)
            .min(32);
        #[cfg(test)]
        let workers = TEST_WORKERS
            .with(|override_| override_.get().unwrap_or(workers))
            .min(32);
        if count == 0 || count > workers as u64 || region.group.len() > workers {
            return Ok(Some(0));
        }
        let Some((compute, _)) = self.library.unscoped_remaining() else {
            return Ok(Some(0));
        };
        if compute == 0 {
            return Ok(Some(0));
        }
        if args[0] == 0 || args[1] == 0 {
            return Err(invalid("parallel dispatch lost checked entry"));
        }
        // SAFETY: the same checked generated function passes its own address
        // and its currently borrowed Frame. The synchronous outer invocation
        // retains the code owner until every scoped child has joined.
        let frame = unsafe { &*(args[1] as *const Frame) };
        let own_context = (self as *mut Self).cast::<std::ffi::c_void>();
        if frame.base.context != own_context
            || frame.base.version != abi::VERSION
            || frame.base.local_count < descriptor.metadata().plan.values().len() as u64
            || frame.base.locals.is_null()
            || frame.phase == abi::PHASE_WORKER
        {
            return Err(invalid("parallel dispatch passed a foreign frame"));
        }
        let memory = self.library.memory_budget().with_limits(
            self.limits
                .max_temporary_bytes
                .ok_or_else(|| invalid("parallel requires finite bytes"))?,
            self.limits
                .max_retained_nodes
                .ok_or_else(|| invalid("parallel requires finite nodes"))?,
        );
        // Scope admission includes native thread stacks and all transport
        // containers. Captured canonical payloads retain their existing shared
        // account and are never copied into an unaccounted worker heap.
        const STACK: usize = 2 * 1024 * 1024;
        // Keep a separate MiB for maintained Rust helpers/unwinding. A caller
        // may raise its normal native-stack allowance; that cannot silently
        // enlarge the bounded stack reserved for these workers.
        const CALLBACK_HEADROOM: usize = 1024 * 1024;
        if descriptor.stack_bytes() > (STACK - CALLBACK_HEADROOM) as u64 {
            return Ok(Some(0));
        }
        let capture_slots = region.group.iter().try_fold(0usize, |total, &member| {
            let member = descriptor
                .metadata()
                .parallel
                .regions
                .get(member as usize)
                .ok_or_else(|| invalid("parallel sibling proof outside descriptor"))?;
            total
                .checked_add(member.captures.len())
                .ok_or_else(|| invalid("parallel capture count overflow"))
        })?;
        // Admit the bounded maximum before inspecting future immutable
        // sources or allocating per-region capture/receipt containers.
        let metadata = amount(
            workers,
            (std::mem::size_of::<Receipt>()
                + std::mem::size_of::<ReadyRegion>()
                + std::mem::size_of::<PendingRegion>()
                + 1024) as u64,
            1024,
        )?
        .checked_add(amount(
            capture_slots,
            (std::mem::size_of::<Capture>() as u64) * (workers as u64 + 1),
            0,
        )?)
        .ok_or_else(|| invalid("parallel metadata overflow"))?;
        self.control.work.charge(
            Phase::Setup,
            metadata
                .checked_mul(8)
                .and_then(|n| n.checked_add(4096))
                .ok_or_else(|| invalid("parallel setup overflow"))?,
        )?;
        let reservation = match memory.reserve_capacity(metadata) {
            Ok(value) => value,
            Err(_) => return Ok(Some(0)),
        };
        // All private scheduler containers coexist under this shared structural
        // claim. Their separately admitted byte envelope is not a node count.
        let nodes = match memory.reserve_nodes(scheduler_nodes(
            workers,
            region.group.len(),
            capture_slots,
        )?) {
            Ok(value) => value,
            Err(_) => return Ok(Some(0)),
        };
        let mut ready = transport(region.group.len(), "parallel region allocation")?;
        let mut task_count = 0u64;
        for &member in &region.group {
            let member = member as usize;
            let selected = &descriptor.metadata().parallel.regions[member];
            let source = self
                .arena
                .copy_current(selected.source, &mut self.control.work)?;
            if !matches!(
                source.data(),
                mech_core::ValueData::Matrix(_) | mech_core::ValueData::Set(_)
            ) {
                return Err(invalid("parallel sibling source is not a proved sequence"));
            }
            let source_count = super::collection::source_count(&source)?;
            if member == id && source_count != count {
                return Err(invalid("parallel leader source count changed"));
            }
            task_count = task_count
                .checked_add(source_count)
                .ok_or_else(|| invalid("parallel task count overflow"))?;
            if source_count == 0 || task_count > workers as u64 {
                return Ok(Some(0));
            }
            let mut captures = transport(selected.captures.len(), "parallel capture allocation")?;
            for &slot in &selected.captures {
                if descriptor.metadata().plan.values()[slot as usize]
                    .scalar_kind()
                    .is_some()
                {
                    // SAFETY: every slot is a proved immutable capture in this
                    // exact live local lane, dominated by the leader frontier.
                    captures.push(Capture::Word(slot, unsafe {
                        *frame.base.locals.add(slot as usize)
                    }));
                } else {
                    captures.push(Capture::Value(
                        slot,
                        self.arena.copy_current(slot, &mut self.control.work)?,
                    ));
                }
            }
            ready.push(ReadyRegion {
                region: member,
                count: source_count,
                source,
                captures,
            });
        }
        if task_count < 2 {
            return Ok(Some(0));
        }
        let count_usize = task_count as usize;
        let stacks = match memory.reserve_capacity(
            task_count
                .checked_mul((STACK + 65536) as u64)
                .ok_or_else(|| invalid("parallel stack admission overflow"))?,
        ) {
            Ok(value) => value,
            Err(_) => return Ok(Some(0)),
        };
        let mut region_ids = transport(ready.len(), "parallel admission inventory allocation")?;
        region_ids.extend(ready.iter().map(|selected| selected.region));
        let Some(_parked_parent) = self.parallel_admission(&region_ids)? else {
            return Ok(Some(0));
        };
        // Decline before spawning when even the exact fixed constructor claims
        // cannot coexist. Release this feasibility reservation immediately;
        // each child still admits its own setup and later dynamic allocations.
        if !worker_setup_fits(descriptor, &memory, task_count)? {
            return Ok(Some(0));
        }
        let origin = descriptor.metadata().plan.collections()[region.collection as usize].origin();
        let (compute, comparisons) = self
            .library
            .unscoped_remaining()
            .ok_or_else(|| invalid("parallel scope changed during admission"))?;
        let group = self
            .library
            .reserve_worker_group_with_memory(compute, comparisons, &memory)
            .map_err(|error| budget_error(error, origin))?;
        let limits = NativeProgramExecutionLimits {
            execution: self.execution_limits,
            library: self.limits,
        };
        // SAFETY: this address was emitted by the exact entry currently holding
        // the caller's checked JIT/linked code owner. No user callback or source
        // value supplies it, and scoped joins prevent code-owner escape.
        let entry: abi::Entry = unsafe { std::mem::transmute(args[0] as usize) };
        let activity = Activity::default();
        let outcomes = std::thread::scope(|scope| -> Result<Vec<Result<Receipt, Error>>, Error> {
            let mut handles = transport(count_usize, "parallel handle allocation")?;
            let mut results = transport(count_usize, "parallel result allocation")?;
            let mut jobs = transport(count_usize, "parallel job allocation")?;
            for selected in &ready {
                for index in 0..selected.count {
                    let loan = group
                        .worker_loan_with_memory(&memory)
                        .map_err(|error| budget_error(error, origin))?;
                    let mut worker_captures =
                        transport(selected.captures.len(), "worker capture allocation")?;
                    worker_captures.extend(selected.captures.iter().cloned());
                    let ordinal = jobs.len();
                    let weight = super::collection::element_work_hint(&selected.source, index);
                    jobs.push((
                        weight,
                        ordinal,
                        selected.region,
                        selected.count,
                        index,
                        loan,
                        worker_captures,
                    ));
                }
            }
            // Pure mapped bodies often scale with their element's size. Start
            // larger inputs first, without inspecting their contents or changing
            // OS priorities. Scalar/tied weights keep source order. Both sorts
            // are allocation-free and bounded by the admitted <=32-task count;
            // the existing metadata*8 + 4096 setup envelope funds this work.
            // The extra scalar keys/ordinals fit the existing 1024 B/task
            // transport slack; no owning root/backing or allowance is added.
            schedule::order_by_key(&mut jobs, |(weight, ordinal, ..)| {
                (std::cmp::Reverse(*weight), *ordinal)
            });
            #[cfg(test)]
            TEST_DISPATCH_ORDER.with(|order| order.borrow_mut().clear());
            let mut spawn_failed = false;
            for (_, ordinal, region, count, index, loan, worker_captures) in jobs {
                let memory = memory.clone();
                let activity = &activity;
                let handle =
                    std::thread::Builder::new()
                        .stack_size(STACK)
                        .spawn_scoped(scope, move || {
                            let _active = activity.enter();
                            let library = NativeLibraryWorkBudget::from_worker_loan(loan)
                                .map_err(|error| Error::Collection { origin, error })?;
                            super::session::Prepared::run_worker(
                                descriptor,
                                entry,
                                limits,
                                &memory,
                                library,
                                Worker {
                                    region,
                                    index,
                                    count,
                                    result: None,
                                },
                                worker_captures,
                            )
                        });
                match handle {
                    Ok(handle) => {
                        #[cfg(test)]
                        TEST_DISPATCH_ORDER.with(|order| order.borrow_mut().push((region, index)));
                        handles.push((ordinal, handle));
                    }
                    Err(_) => {
                        spawn_failed = true;
                        break;
                    }
                }
            }
            // Always explicitly join every launched thread, including after a
            // later spawn failure. No unjoined panic may unwind through FFI.
            // Restore source order before joining/replay, even though workers
            // were dispatched in estimated-cost order. Error/push order remains
            // the original source order, not completion or scheduling order.
            schedule::order_by_key(&mut handles, |(ordinal, _)| *ordinal);
            for (_, handle) in handles {
                results.push(
                    handle
                        .join()
                        .unwrap_or_else(|_| Err(invalid("native worker panicked"))),
                );
            }
            if spawn_failed {
                Err(invalid("native worker creation failed"))
            } else {
                Ok(results)
            }
        });
        // All worker-local ledgers have retired before reconciliation. Work
        // already spent stays charged even when any worker failed.
        let (_, credit) = group
            .finish_with_credit_statistics()
            .map_err(|error| Error::Collection { origin, error })?;
        self.worker_credit_claims = self
            .worker_credit_claims
            .checked_add(credit.claims)
            .ok_or_else(|| invalid("parallel work-credit claim count overflow"))?;
        self.worker_credit_batches = self
            .worker_credit_batches
            .checked_add(credit.batches)
            .ok_or_else(|| invalid("parallel work-credit batch count overflow"))?;
        self.worker_credit_reclaims = self
            .worker_credit_reclaims
            .checked_add(credit.reclaims)
            .ok_or_else(|| invalid("parallel work-credit reclaim count overflow"))?;
        drop(stacks);
        let results = outcomes?;
        if results.iter().all(Result::is_ok) {
            self.parallel_batches = self
                .parallel_batches
                .checked_add(1)
                .ok_or_else(|| invalid("parallel batch count overflow"))?;
            self.parallel_regions = self
                .parallel_regions
                .checked_add(ready.len() as u64)
                .ok_or_else(|| invalid("parallel batch count overflow"))?;
            self.parallel_tasks = self
                .parallel_tasks
                .checked_add(task_count)
                .ok_or_else(|| invalid("parallel task count overflow"))?;
            self.parallel_peak_workers = self
                .parallel_peak_workers
                .max(activity.peak.load(Ordering::Relaxed) as u64);
        }
        for result in results.iter().filter_map(|result| result.as_ref().ok()) {
            self.control.visits = self
                .control
                .visits
                .checked_add(result.visits)
                .ok_or_else(|| invalid("parallel visit overflow"))?;
            self.hash_table_regions = self
                .hash_table_regions
                .checked_add(result.hash_regions)
                .ok_or_else(|| invalid("parallel region overflow"))?;
            self.hash_table_updates = self
                .hash_table_updates
                .checked_add(result.hash_updates)
                .ok_or_else(|| invalid("parallel update overflow"))?;
            self.packed_map_regions = self
                .packed_map_regions
                .checked_add(result.packed_regions)
                .ok_or_else(|| invalid("parallel packed region overflow"))?;
            self.direct_lookup_regions = self
                .direct_lookup_regions
                .checked_add(result.direct_lookup_regions)
                .ok_or_else(|| invalid("parallel direct lookup region overflow"))?;
            self.packed_map_updates = self
                .packed_map_updates
                .checked_add(result.packed_updates)
                .ok_or_else(|| invalid("parallel packed update overflow"))?;
            self.packed_ssa_regions = self
                .packed_ssa_regions
                .checked_add(result.packed_ssa_regions)
                .ok_or_else(|| invalid("parallel SSA region overflow"))?;
            self.guarded_loop_regions = self
                .guarded_loop_regions
                .checked_add(result.guarded_loop_regions)
                .ok_or_else(|| invalid("parallel guarded loop count overflow"))?;
            self.guarded_call_regions = self
                .guarded_call_regions
                .checked_add(result.guarded_call_regions)
                .ok_or_else(|| invalid("parallel guarded call count overflow"))?;
            self.frozen_map_regions = self
                .frozen_map_regions
                .checked_add(result.frozen_map_regions)
                .ok_or_else(|| invalid("parallel frozen region overflow"))?;
            self.frozen_map_reads = self
                .frozen_map_reads
                .checked_add(result.frozen_map_reads)
                .ok_or_else(|| invalid("parallel frozen read overflow"))?;
            self.worker_credit_claims = self
                .worker_credit_claims
                .checked_add(result.worker_credit_claims)
                .ok_or_else(|| invalid("parallel work-credit statistics overflow"))?;
            self.worker_credit_batches = self
                .worker_credit_batches
                .checked_add(result.worker_credit_batches)
                .ok_or_else(|| invalid("parallel work-credit statistics overflow"))?;
            self.worker_credit_reclaims = self
                .worker_credit_reclaims
                .checked_add(result.worker_credit_reclaims)
                .ok_or_else(|| invalid("parallel work-credit statistics overflow"))?;
            self.fused_string_map_bytes = self
                .fused_string_map_bytes
                .checked_add(result.fused_string_map_bytes)
                .ok_or_else(|| invalid("parallel joined map byte overflow"))?;
        }
        let mut pending = VecDeque::new();
        pending
            .try_reserve_exact(ready.len())
            .map_err(|_| invalid("parallel pending group allocation"))?;
        if pending.capacity() != ready.len() {
            return Err(invalid("parallel pending group exceeds admitted capacity"));
        }
        for selected in ready {
            pending.push_back(PendingRegion {
                region: selected.region,
                next: 0,
                count: selected.count,
                source: selected.source,
            });
        }
        self.parallel_batch = Some(Batch {
            regions: pending,
            results: results.into_iter(),
            _reservation: reservation,
            _nodes: nodes,
        });
        Ok(Some(1))
    }

    fn take_parallel(&mut self, site: usize, args: [u64; 3]) -> Result<Option<u64>, Error> {
        let id = usize::try_from(args[0]).map_err(|_| invalid("parallel take region overflow"))?;
        let descriptor = self.control.descriptor;
        let region = descriptor
            .metadata()
            .parallel
            .regions
            .get(id)
            .ok_or_else(|| invalid("parallel take region"))?;
        if args[1..] != [0; 2]
            || region.body as usize != site
            || self.control.previous != Some(site as u32)
            || !self.consumed
        {
            return Err(invalid("parallel take outside original element"));
        }
        let Some(batch) = &mut self.parallel_batch else {
            return Ok(Some(0));
        };
        let pending = batch
            .regions
            .front_mut()
            .ok_or_else(|| invalid("parallel group omitted region"))?;
        if pending.region != id
            || self.control.current != Some(region.iteration)
            || pending
                .count
                .checked_sub(self.control.progress[region.iteration as usize * 2])
                != Some(pending.next)
        {
            return Err(invalid(
                "parallel result belongs to another collection visit",
            ));
        }
        // Surface a failed body only at its original source element. Earlier
        // pushes retain their own output/cap error ordering.
        let receipt = batch
            .results
            .next()
            .ok_or_else(|| invalid("parallel result replay"))??;
        pending.next = pending
            .next
            .checked_add(1)
            .ok_or_else(|| invalid("parallel result index overflow"))?;
        if pending.next == pending.count {
            batch.regions.pop_front();
        }
        if !region.contains(receipt.previous)
            || !super::super::program_parallel_layout::successors(
                descriptor.metadata().plan.blocks()[receipt.previous as usize].terminator(),
            )
            .contains(&Some(region.push))
        {
            return Err(invalid("parallel receipt lacks original push frontier"));
        }
        match receipt.value {
            Yield::Word(word) => self.write_borrowed_word(0, word)?,
            Yield::Value(value) => {
                self.arena
                    .put(region.output, &value, &mut self.control.work)?;
            }
        }
        // The private receipt certifies execution of this exact body under a
        // child loan. Only its original push successor becomes admissible.
        self.control.previous = Some(receipt.previous);
        self.consumed = true;
        if self
            .parallel_batch
            .as_ref()
            .is_some_and(|batch| batch.regions.is_empty())
        {
            if self.parallel_batch.as_ref().unwrap().results.len() != 0 {
                return Err(invalid("parallel group retained extra results"));
            }
            self.parallel_batch = None;
        }
        Ok(Some(1))
    }

    fn yield_parallel(&mut self, site: usize, args: [u64; 3]) -> Result<Option<u64>, Error> {
        let worker = self
            .worker
            .as_ref()
            .ok_or_else(|| invalid("worker yield in parent invocation"))?;
        let descriptor = self.control.descriptor;
        let region = descriptor
            .metadata()
            .parallel
            .regions
            .get(worker.region)
            .ok_or_else(|| invalid("worker proof region"))?;
        let previous = self
            .control
            .previous
            .ok_or_else(|| invalid("worker yield before body"))?;
        if args[1] != worker.region as u64
            || args[2] != 0
            || region.push as usize != site
            || worker.result.is_some()
            || self.control.current != Some(region.iteration)
            || !region.contains(previous)
            || !super::super::program_parallel_layout::successors(
                descriptor.metadata().plan.blocks()[previous as usize].terminator(),
            )
            .contains(&Some(region.push))
            || (self.action_required(previous as usize) && !self.consumed)
            || self.closed_tail.is_some()
            || self.borrowed_strings.is_some()
        {
            return Err(invalid("worker escaped its exact result boundary"));
        }
        let value = if descriptor.metadata().plan.values()[region.output as usize]
            .scalar_kind()
            .is_some()
        {
            Yield::Word(args[0])
        } else {
            if args[0] != 0 {
                return Err(invalid("managed worker result has scalar payload"));
            }
            Yield::Value(
                self.arena
                    .copy_current(region.output, &mut self.control.work)?,
            )
        };
        self.worker.as_mut().unwrap().result = Some(Receipt {
            value,
            previous,
            visits: self.control.visits,
            hash_regions: self.hash_table_regions,
            hash_updates: self.hash_table_updates,
            packed_regions: self.packed_map_regions,
            packed_updates: self.packed_map_updates,
            direct_lookup_regions: self.direct_lookup_regions,
            packed_ssa_regions: self.packed_ssa_regions,
            guarded_loop_regions: self.guarded_loop_regions,
            guarded_call_regions: self.guarded_call_regions,
            frozen_map_regions: self.frozen_map_regions,
            frozen_map_reads: self.frozen_map_reads,
            worker_credit_claims: self.worker_credit_claims,
            worker_credit_batches: self.worker_credit_batches,
            worker_credit_reclaims: self.worker_credit_reclaims,
            fused_string_map_bytes: self.fused_string_map_bytes,
        });
        Ok(Some(1))
    }
}
