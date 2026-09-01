use std::collections::{BTreeMap, BTreeSet};
use std::sync::mpsc::{self, Receiver, Sender};
use std::{mem, sync::Arc, thread};

#[cfg(target_arch = "aarch64")]
use core::arch::aarch64::*;
#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::__m128;

use cranelift_codegen::ir::{
    AbiParam, InstBuilder, MemFlags, StackSlotData, StackSlotKind, Type, UserFuncName, Value,
    condcodes::{FloatCC, IntCC},
    types,
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, default_libcall_names};
use mech_core::CellSlotId;
#[cfg(not(target_arch = "aarch64"))]
use wide::f32x4;
#[cfg(not(target_arch = "aarch64"))]
use wide::{CmpEq, CmpGe, CmpGt, i32x4};

use super::{
    BatchedExecutionError, BatchedFaultRecorder, BatchedIntegrityFault, BinaryOperation,
    ComparisonOperation, ElementwiseOperation, FixedShapeKernel, LogicOperation, ScalarComputation,
    ScalarInstruction, ScalarOperand, ScalarPredicate, UnaryOperation,
};

type NativeTurn = unsafe extern "C" fn(
    input_pointers: *const *const f32,
    state_pointers: *const *const f32,
    next_state_pointers: *const *mut f32,
) -> u64;

const SIMD_JIT_LANES: usize = 4;
type NativeSimdTurn = unsafe extern "C" fn(
    input_pointers: *const *const f32,
    state_pointers: *const *mut f32,
    next_state_pointers: *const *mut f32,
    start_group: usize,
    end_group: usize,
) -> u64;

struct NativeKernel {
    _module: JITModule,
    turn: NativeTurn,
}

struct NativeSimdKernel {
    _module: JITModule,
    turn: NativeSimdTurn,
}

enum ParallelWorkerCommand {
    Run { turns: u32 },
    Shutdown,
}

#[derive(Clone, Copy, Debug)]
struct ParallelWorkerResult {
    packed_fault: u64,
    fault_turn: u32,
}

struct ParallelWorker {
    command: Sender<ParallelWorkerCommand>,
    result: Receiver<ParallelWorkerResult>,
    handle: Option<thread::JoinHandle<()>>,
}

/// Long-lived workers for the SIMD/JIT CPU path. The worker owns its pointer
/// tables, so the hot loop only sends a small command and receives one fault
/// word instead of rebuilding vectors and spawning threads for every turn.
struct ParallelWorkerPool {
    workers: Vec<ParallelWorker>,
}

impl ParallelWorkerPool {
    fn new(
        turn: NativeSimdTurn,
        input_pointers: &[*const f32],
        state_pointers: &[*mut f32],
        next_state_pointers: &[*mut f32],
        groups: usize,
        workers: usize,
    ) -> Result<Self, String> {
        let input_addresses = input_pointers
            .iter()
            .map(|pointer| *pointer as usize)
            .collect::<Vec<_>>();
        let state_addresses = state_pointers
            .iter()
            .map(|pointer| *pointer as usize)
            .collect::<Vec<_>>();
        let next_state_addresses = next_state_pointers
            .iter()
            .map(|pointer| *pointer as usize)
            .collect::<Vec<_>>();
        let mut pool = Self {
            workers: Vec::with_capacity(workers),
        };

        for worker_index in 0..workers {
            let (command, commands) = mpsc::channel();
            let (results, result) = mpsc::channel();
            let input_addresses = input_addresses.clone();
            let state_addresses = state_addresses.clone();
            let next_state_addresses = next_state_addresses.clone();
            let start_group = groups * worker_index / workers;
            let end_group = groups * (worker_index + 1) / workers;
            let handle = thread::Builder::new()
                .name(format!("mech-simd-worker-{worker_index}"))
                .spawn(move || {
                    // Raw pointers are reconstructed only inside the worker;
                    // the address vectors crossing the thread boundary are
                    // integer handles whose allocations remain owned by the
                    // session for the worker's entire lifetime.
                    let input_pointers = input_addresses
                        .iter()
                        .map(|pointer| *pointer as *const f32)
                        .collect::<Vec<_>>();
                    let mut state_pointers = state_addresses
                        .iter()
                        .map(|pointer| *pointer as *mut f32)
                        .collect::<Vec<_>>();
                    let mut next_state_pointers = next_state_addresses
                        .iter()
                        .map(|pointer| *pointer as *mut f32)
                        .collect::<Vec<_>>();

                    while let Ok(command) = commands.recv() {
                        match command {
                            ParallelWorkerCommand::Run { turns } => {
                                let mut result = ParallelWorkerResult {
                                    packed_fault: 0,
                                    fault_turn: turns,
                                };
                                for turn_index in 0..turns {
                                    // SAFETY: this worker owns a disjoint SIMD
                                    // group range and all backing buffers stay
                                    // live until the pool is dropped.
                                    let packed_fault = unsafe {
                                        turn(
                                            input_pointers.as_ptr(),
                                            state_pointers.as_ptr(),
                                            next_state_pointers.as_ptr(),
                                            start_group,
                                            end_group,
                                        )
                                    };
                                    if packed_fault != 0 {
                                        result = ParallelWorkerResult {
                                            packed_fault,
                                            fault_turn: turn_index,
                                        };
                                        break;
                                    }
                                    mem::swap(&mut state_pointers, &mut next_state_pointers);
                                }
                                if results.send(result).is_err() {
                                    break;
                                }
                            }
                            ParallelWorkerCommand::Shutdown => break,
                        }
                    }
                })
                .map_err(|error| format!("failed to start SIMD worker: {error}"))?;
            pool.workers.push(ParallelWorker {
                command,
                result,
                handle: Some(handle),
            });
        }
        Ok(pool)
    }

    fn worker_count(&self) -> usize {
        self.workers.len()
    }

    fn run(&self, turns: u32) -> Result<ParallelWorkerResult, String> {
        for worker in &self.workers {
            worker
                .command
                .send(ParallelWorkerCommand::Run { turns })
                .map_err(|error| format!("SIMD worker command failed: {error}"))?;
        }
        self.workers
            .iter()
            .map(|worker| {
                worker
                    .result
                    .recv()
                    .map_err(|error| format!("SIMD worker result failed: {error}"))
            })
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .filter(|result| result.packed_fault != 0)
            .min_by_key(|result| (result.fault_turn, result.packed_fault))
            .map_or(
                Ok(ParallelWorkerResult {
                    packed_fault: 0,
                    fault_turn: turns,
                }),
                Ok,
            )
    }
}

impl Drop for ParallelWorkerPool {
    fn drop(&mut self) {
        for worker in &self.workers {
            let _ = worker.command.send(ParallelWorkerCommand::Shutdown);
        }
        for worker in &mut self.workers {
            if let Some(handle) = worker.handle.take() {
                let _ = handle.join();
            }
        }
    }
}

pub struct BatchedJitCpuSession {
    program: Arc<FixedShapeKernel>,
    kernel: NativeKernel,
    checked: bool,
    inputs: BTreeMap<CellSlotId, Vec<f32>>,
    input_broadcast: Vec<bool>,
    state: BTreeMap<CellSlotId, Vec<f32>>,
    next_state: BTreeMap<CellSlotId, Vec<f32>>,
    input_pointers: Vec<*const f32>,
    state_pointers: Vec<*const f32>,
    next_state_pointers: Vec<*mut f32>,
    faults: BatchedFaultRecorder,
}

/// Native JIT session that evaluates four independent instances in one
/// Cranelift vector body. State and input buffers use a group-major layout so
/// each matrix component is loaded and stored as one contiguous f32x4 value;
/// the public state view is materialized back to Mech's instance-major layout
/// only when read.
pub struct BatchedJitSimdCpuSession {
    program: Arc<FixedShapeKernel>,
    kernel: NativeSimdKernel,
    parallel_pool: Option<ParallelWorkerPool>,
    checked: bool,
    inputs: BTreeMap<CellSlotId, Vec<f32>>,
    packed_inputs: BTreeMap<CellSlotId, Vec<f32>>,
    input_broadcast: Vec<bool>,
    state: BTreeMap<CellSlotId, Vec<f32>>,
    packed_state: BTreeMap<CellSlotId, Vec<f32>>,
    packed_next_state: BTreeMap<CellSlotId, Vec<f32>>,
    packed_checkpoint_state: Option<BTreeMap<CellSlotId, Vec<f32>>>,
    logical_state_dirty: bool,
    input_pointers: Vec<*const f32>,
    state_pointers: Vec<*mut f32>,
    next_state_pointers: Vec<*mut f32>,
    faults: BatchedFaultRecorder,
}

impl FixedShapeKernel {
    pub fn prepare_jit_cpu(
        &self,
        inputs: &BTreeMap<String, Vec<f32>>,
    ) -> Result<BatchedJitCpuSession, BatchedExecutionError> {
        self.prepare_jit_cpu_with_validation(inputs, true)
    }

    /// Prepares a JIT session without integrity predicates. Invalid candidate
    /// state is published, so callers must only use this mode when they own
    /// equivalent validation or explicitly accept the weaker guarantee.
    pub fn prepare_jit_cpu_unchecked(
        &self,
        inputs: &BTreeMap<String, Vec<f32>>,
    ) -> Result<BatchedJitCpuSession, BatchedExecutionError> {
        self.prepare_jit_cpu_with_validation(inputs, false)
    }

    /// Prepares the lane-vectorized JIT session.  The first implementation
    /// uses four-lane f32 vectors and therefore requires a batch extent that
    /// is divisible by four; callers can use the scalar JIT for a tail until
    /// masked vector tails are added.
    pub fn prepare_jit_simd_cpu(
        &self,
        inputs: &BTreeMap<String, Vec<f32>>,
    ) -> Result<BatchedJitSimdCpuSession, BatchedExecutionError> {
        self.prepare_jit_simd_cpu_with_validation(inputs, true)
    }

    /// Prepares an unchecked lane-vectorized JIT session.
    pub fn prepare_jit_simd_cpu_unchecked(
        &self,
        inputs: &BTreeMap<String, Vec<f32>>,
    ) -> Result<BatchedJitSimdCpuSession, BatchedExecutionError> {
        self.prepare_jit_simd_cpu_with_validation(inputs, false)
    }

    fn prepare_jit_cpu_with_validation(
        &self,
        provided_inputs: &BTreeMap<String, Vec<f32>>,
        checked: bool,
    ) -> Result<BatchedJitCpuSession, BatchedExecutionError> {
        let input_broadcast = self
            .inputs
            .iter()
            .map(|input| {
                provided_inputs
                    .get(&input.name)
                    .is_some_and(|values| values.len() == input.shape.elements())
            })
            .collect::<Vec<_>>();
        let inputs = self.expand_inputs(provided_inputs)?;
        let state = self.initial_state();
        let next_state = state
            .iter()
            .map(|(slot, values)| (*slot, vec![0.0; values.len()]))
            .collect();
        let kernel = NativeKernel::compile(self, checked, &input_broadcast)?;
        let input_pointers = self
            .inputs
            .iter()
            .map(|input| inputs[&input.slot].as_ptr())
            .collect();
        let mut session = BatchedJitCpuSession {
            program: Arc::new(self.clone()),
            kernel,
            checked,
            inputs,
            input_broadcast,
            state,
            next_state,
            input_pointers,
            state_pointers: Vec::with_capacity(self.states.len()),
            next_state_pointers: Vec::with_capacity(self.states.len()),
            faults: BatchedFaultRecorder::default(),
        };
        session.refresh_state_pointers();
        Ok(session)
    }

    fn prepare_jit_simd_cpu_with_validation(
        &self,
        provided_inputs: &BTreeMap<String, Vec<f32>>,
        checked: bool,
    ) -> Result<BatchedJitSimdCpuSession, BatchedExecutionError> {
        if self.instances as usize % SIMD_JIT_LANES != 0 {
            return Err(BatchedExecutionError::Native(format!(
                "SIMD JIT requires an instance count divisible by {SIMD_JIT_LANES}, found {}",
                self.instances
            )));
        }
        let input_broadcast = self
            .inputs
            .iter()
            .map(|input| {
                provided_inputs
                    .get(&input.name)
                    .is_some_and(|values| values.len() == input.shape.elements())
            })
            .collect::<Vec<_>>();
        let inputs = self.expand_inputs(provided_inputs)?;
        let state = self.initial_state();
        let packed_inputs: BTreeMap<CellSlotId, Vec<f32>> = self
            .inputs
            .iter()
            .enumerate()
            .map(|(_index, input)| {
                let values = &inputs[&input.slot];
                let packed = pack_simd_instances(values, input.shape.elements());
                (input.slot, packed)
            })
            .collect();
        let packed_state: BTreeMap<CellSlotId, Vec<f32>> = self
            .states
            .iter()
            .map(|descriptor| {
                (
                    descriptor.slot,
                    pack_simd_instances(&state[&descriptor.slot], descriptor.shape.elements()),
                )
            })
            .collect();
        let packed_next_state = self
            .states
            .iter()
            .map(|descriptor| (descriptor.slot, vec![0.0; state[&descriptor.slot].len()]))
            .collect();
        let packed_checkpoint_state = checked.then(|| {
            packed_state
                .iter()
                .map(|(slot, values)| (*slot, vec![0.0; values.len()]))
                .collect()
        });
        let kernel = NativeSimdKernel::compile(self, checked, &input_broadcast)?;
        let input_pointers = self
            .inputs
            .iter()
            .map(|input| packed_inputs[&input.slot].as_ptr())
            .collect();
        let mut session = BatchedJitSimdCpuSession {
            program: Arc::new(self.clone()),
            kernel,
            parallel_pool: None,
            checked,
            inputs,
            packed_inputs,
            input_broadcast,
            state,
            packed_state,
            packed_next_state,
            packed_checkpoint_state,
            logical_state_dirty: false,
            input_pointers,
            state_pointers: Vec::with_capacity(self.states.len()),
            next_state_pointers: Vec::with_capacity(self.states.len()),
            faults: BatchedFaultRecorder::default(),
        };
        session.refresh_state_pointers();
        Ok(session)
    }
}

impl BatchedJitCpuSession {
    pub fn update_inputs(
        &mut self,
        updates: &BTreeMap<String, Vec<f32>>,
    ) -> Result<(), BatchedExecutionError> {
        let mut recompile = false;
        for (name, values) in updates {
            let (index, input) = self
                .program
                .inputs
                .iter()
                .enumerate()
                .find(|(_, input)| input.name == *name)
                .ok_or_else(|| BatchedExecutionError::MissingInput(name.clone()))?;
            self.inputs
                .insert(input.slot, self.program.expand_input(input, values)?);
            let broadcast = values.len() == input.shape.elements();
            if self.input_broadcast[index] != broadcast {
                self.input_broadcast[index] = broadcast;
                recompile = true;
            }
        }
        self.input_pointers = self
            .program
            .inputs
            .iter()
            .map(|input| self.inputs[&input.slot].as_ptr())
            .collect();
        if recompile {
            self.kernel =
                NativeKernel::compile(&self.program, self.checked, &self.input_broadcast)?;
        }
        Ok(())
    }

    pub fn dispatch_turns(&mut self, turns: u32) -> Result<(), BatchedExecutionError> {
        if turns == 0 {
            return Err(BatchedExecutionError::ZeroTurns);
        }
        for _ in 0..turns {
            let attempted_turn = self.faults.next_turn();
            self.refresh_state_pointers();
            // SAFETY: The generated function uses the exact ABI below. All pointer
            // tables and their backing f32 buffers remain live for the call, and
            // every generated access is bounded by the admitted fixed shapes and
            // the program's instance count.
            let packed_fault = unsafe {
                (self.kernel.turn)(
                    self.input_pointers.as_ptr(),
                    self.state_pointers.as_ptr(),
                    self.next_state_pointers.as_ptr(),
                )
            };
            if let Some(fault) = self
                .program
                .failed_packed_constraint(packed_fault, attempted_turn)
            {
                return Err(self.faults.record(fault));
            }
            mem::swap(&mut self.state, &mut self.next_state);
        }
        Ok(())
    }

    pub fn state(&self) -> &BTreeMap<CellSlotId, Vec<f32>> {
        &self.state
    }

    pub const fn fault_count(&self) -> u64 {
        self.faults.fault_count
    }

    pub const fn attempted_turns(&self) -> u64 {
        self.faults.attempted_turns()
    }

    pub fn last_fault(&self) -> Option<&BatchedIntegrityFault> {
        self.faults.last_fault.as_ref()
    }

    fn refresh_state_pointers(&mut self) {
        self.state_pointers.clear();
        self.next_state_pointers.clear();
        for state in &self.program.states {
            self.state_pointers.push(self.state[&state.slot].as_ptr());
            self.next_state_pointers
                .push(self.next_state.get_mut(&state.slot).unwrap().as_mut_ptr());
        }
    }
}

impl BatchedJitSimdCpuSession {
    pub fn update_inputs(
        &mut self,
        updates: &BTreeMap<String, Vec<f32>>,
    ) -> Result<(), BatchedExecutionError> {
        // Input replacement can move packed buffers, invalidating the raw
        // addresses owned by the persistent worker pool.
        self.parallel_pool.take();
        let mut recompile = false;
        for (name, values) in updates {
            let (index, input) = self
                .program
                .inputs
                .iter()
                .enumerate()
                .find(|(_, input)| input.name == *name)
                .ok_or_else(|| BatchedExecutionError::MissingInput(name.clone()))?;
            self.inputs
                .insert(input.slot, self.program.expand_input(input, values)?);
            let broadcast = values.len() == input.shape.elements();
            if self.input_broadcast[index] != broadcast {
                self.input_broadcast[index] = broadcast;
                recompile = true;
            }
        }
        self.packed_inputs = self
            .program
            .inputs
            .iter()
            .enumerate()
            .map(|(_index, input)| {
                let values = &self.inputs[&input.slot];
                let packed = pack_simd_instances(values, input.shape.elements());
                (input.slot, packed)
            })
            .collect();
        self.input_pointers = self
            .program
            .inputs
            .iter()
            .map(|input| self.packed_inputs[&input.slot].as_ptr())
            .collect();
        if recompile {
            self.kernel =
                NativeSimdKernel::compile(&self.program, self.checked, &self.input_broadcast)?;
        }
        Ok(())
    }

    pub fn dispatch_turns(&mut self, turns: u32) -> Result<(), BatchedExecutionError> {
        if turns == 0 {
            return Err(BatchedExecutionError::ZeroTurns);
        }
        // The single-thread path owns the pointer-table orientation. Stop a
        // pool created by the parallel path before changing that orientation.
        self.parallel_pool.take();
        for _ in 0..turns {
            let attempted_turn = self.faults.next_turn();
            // SAFETY: the generated function uses the exact ABI below. Its
            // vector loads read four contiguous lanes at fixed strides, and
            // all buffers remain live for the duration of the call.
            let packed_fault = unsafe {
                (self.kernel.turn)(
                    self.input_pointers.as_ptr(),
                    self.state_pointers.as_ptr(),
                    self.next_state_pointers.as_ptr(),
                    0,
                    self.program.instances as usize / SIMD_JIT_LANES,
                )
            };
            if let Some(fault) = self
                .program
                .failed_packed_constraint(packed_fault, attempted_turn)
            {
                return Err(self.faults.record(fault));
            }
            mem::swap(&mut self.state_pointers, &mut self.next_state_pointers);
            mem::swap(&mut self.packed_state, &mut self.packed_next_state);
            self.logical_state_dirty = true;
        }
        Ok(())
    }

    /// Dispatches the resident SIMD kernel across disjoint instance ranges.
    /// The worker pool is created on the first call and reused thereafter;
    /// checked mode still performs validation and publication once per turn.
    pub fn dispatch_turns_parallel(
        &mut self,
        turns: u32,
        workers: usize,
    ) -> Result<(), BatchedExecutionError> {
        if turns == 0 {
            return Err(BatchedExecutionError::ZeroTurns);
        }
        if workers <= 1 {
            return self.dispatch_turns(turns);
        }
        let groups = self.program.instances as usize / SIMD_JIT_LANES;
        if groups == 0 {
            return Err(BatchedExecutionError::Native(
                "parallel SIMD JIT requires at least one instance group".to_owned(),
            ));
        }
        let workers = workers.min(groups);
        self.ensure_parallel_pool(groups, workers)?;
        for _ in 0..turns {
            let attempted_turn = self.faults.next_turn();
            let worker_result = self
                .parallel_pool
                .as_ref()
                .expect("parallel worker pool initialized")
                .run(1)
                .map_err(BatchedExecutionError::Native)?;
            let packed_fault = worker_result.packed_fault;
            if let Some(fault) = self
                .program
                .failed_packed_constraint(packed_fault, attempted_turn)
            {
                // A rejected turn leaves worker-local pointer orientation
                // unspecified. Discard the pool so the published state stays
                // authoritative if the caller continues after the fault.
                self.parallel_pool.take();
                return Err(self.faults.record(fault));
            }
            mem::swap(&mut self.state_pointers, &mut self.next_state_pointers);
            mem::swap(&mut self.packed_state, &mut self.packed_next_state);
            self.logical_state_dirty = true;
        }
        Ok(())
    }

    /// Runs a checked turn block with one host synchronization at its end.
    ///
    /// Each worker validates every candidate turn in the native kernel, while
    /// the session keeps a block-start checkpoint. A successful block is
    /// published once; a fault restores that checkpoint, records the exact
    /// attempted turn, drops the worker pool (its private pointer orientation
    /// may differ after a partial block), and returns the normal structured
    /// integrity error. Intermediate turns are intentionally not observable.
    pub fn dispatch_turns_parallel_checked_fused(
        &mut self,
        turns: u32,
        workers: usize,
    ) -> Result<(), BatchedExecutionError> {
        if turns == 0 {
            return Err(BatchedExecutionError::ZeroTurns);
        }
        if !self.checked {
            return Err(BatchedExecutionError::Native(
                "checked fused parallel dispatch requires a checked session".to_owned(),
            ));
        }
        if workers == 0 {
            return Err(BatchedExecutionError::Native(
                "checked fused parallel dispatch requires at least one worker".to_owned(),
            ));
        }
        let groups = self.program.instances as usize / SIMD_JIT_LANES;
        if groups == 0 {
            return Err(BatchedExecutionError::Native(
                "parallel SIMD JIT requires at least one instance group".to_owned(),
            ));
        }
        let workers = workers.min(groups);
        self.ensure_parallel_pool(groups, workers)?;
        for descriptor in &self.program.states {
            self.packed_checkpoint_state
                .as_mut()
                .expect("checked sessions have a packed checkpoint")
                .get_mut(&descriptor.slot)
                .expect("packed checkpoint descriptor exists")
                .clone_from(&self.packed_state[&descriptor.slot]);
        }
        let block_start = self.faults.attempted_turns();
        let worker_result = match self
            .parallel_pool
            .as_ref()
            .expect("parallel worker pool initialized")
            .run(turns)
        {
            Ok(result) => result,
            Err(error) => {
                for descriptor in &self.program.states {
                    self.packed_state
                        .get_mut(&descriptor.slot)
                        .expect("packed state descriptor exists")
                        .clone_from(
                            &self
                                .packed_checkpoint_state
                                .as_ref()
                                .expect("checked sessions have a packed checkpoint")
                                [&descriptor.slot],
                        );
                }
                self.parallel_pool.take();
                return Err(BatchedExecutionError::Native(error));
            }
        };
        if let Some(fault) = self.program.failed_packed_constraint(
            worker_result.packed_fault,
            block_start.saturating_add(worker_result.fault_turn as u64 + 1),
        ) {
            for descriptor in &self.program.states {
                self.packed_state
                    .get_mut(&descriptor.slot)
                    .expect("packed state descriptor exists")
                    .clone_from(
                        &self
                            .packed_checkpoint_state
                            .as_ref()
                            .expect("checked sessions have a packed checkpoint")[&descriptor.slot],
                    );
            }
            self.parallel_pool.take();
            for _ in 0..=worker_result.fault_turn {
                self.faults.next_turn();
            }
            return Err(self.faults.record(fault));
        }
        for _ in 0..turns {
            self.faults.next_turn();
        }
        if turns % 2 == 1 {
            mem::swap(&mut self.state_pointers, &mut self.next_state_pointers);
            mem::swap(&mut self.packed_state, &mut self.packed_next_state);
        }
        self.logical_state_dirty = true;
        Ok(())
    }

    /// Runs an unchecked fixed-mode turn block as one worker-pool command.
    /// This is the CPU analogue of Futhark's fixed `main_unchecked` entry
    /// point: no integrity publication occurs inside the block, while the
    /// public session still exposes the resulting state at its boundary.
    pub fn dispatch_turns_parallel_unchecked(
        &mut self,
        turns: u32,
        workers: usize,
    ) -> Result<(), BatchedExecutionError> {
        if turns == 0 {
            return Err(BatchedExecutionError::ZeroTurns);
        }
        if self.checked {
            return Err(BatchedExecutionError::Native(
                "fixed unchecked parallel dispatch requires an unchecked session".to_owned(),
            ));
        }
        if workers == 0 {
            return Err(BatchedExecutionError::Native(
                "fixed unchecked parallel dispatch requires at least one worker".to_owned(),
            ));
        }
        let groups = self.program.instances as usize / SIMD_JIT_LANES;
        if groups == 0 {
            return Err(BatchedExecutionError::Native(
                "parallel SIMD JIT requires at least one instance group".to_owned(),
            ));
        }
        let workers = workers.min(groups);
        self.ensure_parallel_pool(groups, workers)?;
        let worker_result = self
            .parallel_pool
            .as_ref()
            .expect("parallel worker pool initialized")
            .run(turns)
            .map_err(BatchedExecutionError::Native)?;
        let packed_fault = worker_result.packed_fault;
        if let Some(fault) = self.program.failed_packed_constraint(
            packed_fault,
            self.faults.attempted_turns().saturating_add(1),
        ) {
            self.parallel_pool.take();
            return Err(self.faults.record(fault));
        }
        for _ in 0..turns {
            self.faults.next_turn();
        }
        if turns % 2 == 1 {
            mem::swap(&mut self.state_pointers, &mut self.next_state_pointers);
            mem::swap(&mut self.packed_state, &mut self.packed_next_state);
        }
        self.logical_state_dirty = true;
        Ok(())
    }

    pub fn state(&mut self) -> &BTreeMap<CellSlotId, Vec<f32>> {
        if self.logical_state_dirty {
            for descriptor in &self.program.states {
                unpack_simd_instances(
                    &self.packed_state[&descriptor.slot],
                    self.state.get_mut(&descriptor.slot).unwrap(),
                    descriptor.shape.elements(),
                );
            }
            self.logical_state_dirty = false;
        }
        &self.state
    }

    pub const fn fault_count(&self) -> u64 {
        self.faults.fault_count
    }

    pub const fn attempted_turns(&self) -> u64 {
        self.faults.attempted_turns()
    }

    pub fn last_fault(&self) -> Option<&BatchedIntegrityFault> {
        self.faults.last_fault.as_ref()
    }

    fn refresh_state_pointers(&mut self) {
        self.state_pointers.clear();
        self.next_state_pointers.clear();
        for state in &self.program.states {
            self.state_pointers
                .push(self.packed_state.get_mut(&state.slot).unwrap().as_mut_ptr());
            self.next_state_pointers.push(
                self.packed_next_state
                    .get_mut(&state.slot)
                    .unwrap()
                    .as_mut_ptr(),
            );
        }
    }

    fn ensure_parallel_pool(
        &mut self,
        groups: usize,
        workers: usize,
    ) -> Result<(), BatchedExecutionError> {
        let needs_pool = self
            .parallel_pool
            .as_ref()
            .is_none_or(|pool| pool.worker_count() != workers);
        if needs_pool {
            self.parallel_pool.take();
            self.parallel_pool = Some(
                ParallelWorkerPool::new(
                    self.kernel.turn,
                    &self.input_pointers,
                    &self.state_pointers,
                    &self.next_state_pointers,
                    groups,
                    workers,
                )
                .map_err(BatchedExecutionError::Native)?,
            );
        }
        Ok(())
    }
}

impl Drop for BatchedJitSimdCpuSession {
    fn drop(&mut self) {
        // Join workers while the JIT module and all state buffers are still
        // alive. This also makes session shutdown deterministic for hosts.
        self.parallel_pool.take();
    }
}

impl NativeKernel {
    fn compile(
        program: &FixedShapeKernel,
        checked: bool,
        input_broadcast: &[bool],
    ) -> Result<Self, BatchedExecutionError> {
        let mut jit_builder =
            JITBuilder::with_flags(&[("opt_level", "speed")], default_libcall_names())
                .map_err(native_error)?;
        jit_builder
            .symbol("mech_jit_sinf", mech_jit_sinf as *const u8)
            .symbol("mech_jit_cosf", mech_jit_cosf as *const u8)
            .symbol("mech_jit_sincos_pack", mech_jit_sincos_pack as *const u8)
            .symbol("mech_jit_sqrtf", mech_jit_sqrtf as *const u8)
            .symbol("mech_jit_ceilf", mech_jit_ceilf as *const u8)
            .symbol("mech_jit_atan2f", mech_jit_atan2f as *const u8);
        let mut module = JITModule::new(jit_builder);

        let unary_signature = {
            let mut signature = module.make_signature();
            signature.params.push(AbiParam::new(types::F32));
            signature.returns.push(AbiParam::new(types::F32));
            signature
        };
        let binary_signature = {
            let mut signature = module.make_signature();
            signature.params.push(AbiParam::new(types::F32));
            signature.params.push(AbiParam::new(types::F32));
            signature.returns.push(AbiParam::new(types::F32));
            signature
        };
        let sin_id = module
            .declare_function("mech_jit_sinf", Linkage::Import, &unary_signature)
            .map_err(native_error)?;
        let cos_id = module
            .declare_function("mech_jit_cosf", Linkage::Import, &unary_signature)
            .map_err(native_error)?;
        let sincos_id = module
            .declare_function("mech_jit_sincos_pack", Linkage::Import, &{
                let mut signature = module.make_signature();
                signature.params.push(AbiParam::new(types::F32));
                signature.returns.push(AbiParam::new(types::I64));
                signature
            })
            .map_err(native_error)?;
        let sqrt_id = module
            .declare_function("mech_jit_sqrtf", Linkage::Import, &unary_signature)
            .map_err(native_error)?;
        let ceil_id = module
            .declare_function("mech_jit_ceilf", Linkage::Import, &unary_signature)
            .map_err(native_error)?;
        let atan2_id = module
            .declare_function("mech_jit_atan2f", Linkage::Import, &binary_signature)
            .map_err(native_error)?;
        let pointer_type = module.target_config().pointer_type();
        let mut signature = module.make_signature();
        for _ in 0..3 {
            signature.params.push(AbiParam::new(pointer_type));
        }
        signature.returns.push(AbiParam::new(types::I64));
        let function_id = module
            .declare_function("mech_fixed_numeric_turn", Linkage::Local, &signature)
            .map_err(native_error)?;
        let mut context = module.make_context();
        context.func.signature = signature;
        context.func.name = UserFuncName::user(0, function_id.as_u32());
        let mut function_context = FunctionBuilderContext::new();

        {
            let mut builder = FunctionBuilder::new(&mut context.func, &mut function_context);
            let entry = builder.create_block();
            let header = builder.create_block();
            let body = builder.create_block();
            let advance = builder.create_block();
            let fault = builder.create_block();
            let exit = builder.create_block();
            builder.append_block_params_for_function_params(entry);
            builder.append_block_param(header, pointer_type);
            let dynamic_input_count = input_broadcast
                .iter()
                .filter(|broadcast| !**broadcast)
                .count();
            let loop_base_count = dynamic_input_count + program.states.len() * 2;
            for _ in 0..loop_base_count {
                builder.append_block_param(header, pointer_type);
            }
            builder.append_block_param(fault, pointer_type);
            builder.append_block_param(fault, types::I32);
            builder.switch_to_block(entry);

            let parameters = builder.block_params(entry).to_vec();
            let input_table = parameters[0];
            let state_table = parameters[1];
            let next_state_table = parameters[2];
            let pointer_bytes = i32::try_from(pointer_type.bytes()).unwrap();
            let flags = MemFlags::trusted();
            let input_bases = (0..program.inputs.len())
                .map(|index| {
                    builder.ins().load(
                        pointer_type,
                        flags,
                        input_table,
                        i32::try_from(index).unwrap() * pointer_bytes,
                    )
                })
                .collect::<Vec<_>>();
            let input_broadcast_values = program
                .inputs
                .iter()
                .enumerate()
                .map(|(index, input)| {
                    input_broadcast[index].then(|| {
                        (0..input.shape.elements())
                            .map(|component| {
                                load_component(&mut builder, input_bases[index], component)
                            })
                            .collect::<Vec<_>>()
                    })
                })
                .collect::<Vec<_>>();
            let state_bases = (0..program.states.len())
                .map(|index| {
                    builder.ins().load(
                        pointer_type,
                        flags,
                        state_table,
                        i32::try_from(index).unwrap() * pointer_bytes,
                    )
                })
                .collect::<Vec<_>>();
            let next_state_bases = (0..program.states.len())
                .map(|index| {
                    builder.ins().load(
                        pointer_type,
                        flags,
                        next_state_table,
                        i32::try_from(index).unwrap() * pointer_bytes,
                    )
                })
                .collect::<Vec<_>>();
            let constant_values = collect_constant_bits(program)
                .into_iter()
                .map(|bits| (bits, builder.ins().f32const(f32::from_bits(bits))))
                .collect::<BTreeMap<_, _>>();
            let zero = builder.ins().iconst(pointer_type, 0);
            let mut initial_loop_bases = Vec::with_capacity(loop_base_count);
            for (index, input) in program.inputs.iter().enumerate() {
                if !input_broadcast[index] {
                    initial_loop_bases.push((
                        input_bases[index],
                        i64::try_from(input.shape.elements())
                            .unwrap()
                            .checked_mul(i64::from(types::F32.bytes()))
                            .unwrap(),
                    ));
                }
            }
            for (index, state) in program.states.iter().enumerate() {
                let stride = i64::try_from(state.shape.elements())
                    .unwrap()
                    .checked_mul(i64::from(types::F32.bytes()))
                    .unwrap();
                initial_loop_bases.push((state_bases[index], stride));
            }
            for (index, state) in program.states.iter().enumerate() {
                let stride = i64::try_from(state.shape.elements())
                    .unwrap()
                    .checked_mul(i64::from(types::F32.bytes()))
                    .unwrap();
                initial_loop_bases.push((next_state_bases[index], stride));
            }
            debug_assert_eq!(initial_loop_bases.len(), loop_base_count);
            let mut initial_header_args = vec![cranelift_codegen::ir::BlockArg::Value(zero)];
            initial_header_args.extend(
                initial_loop_bases
                    .iter()
                    .map(|(base, _)| cranelift_codegen::ir::BlockArg::Value(*base)),
            );

            builder.ins().jump(header, &initial_header_args);

            builder.switch_to_block(header);
            let header_params = builder.block_params(header).to_vec();
            let instance = header_params[0];
            let has_instance = builder.ins().icmp_imm(
                IntCC::UnsignedLessThan,
                instance,
                i64::from(program.instances),
            );
            builder.ins().brif(has_instance, body, &[], exit, &[]);

            builder.switch_to_block(body);
            let sin_ref = module.declare_func_in_func(sin_id, builder.func);
            let cos_ref = module.declare_func_in_func(cos_id, builder.func);
            let sincos_ref = module.declare_func_in_func(sincos_id, builder.func);
            let sqrt_ref = module.declare_func_in_func(sqrt_id, builder.func);
            let ceil_ref = module.declare_func_in_func(ceil_id, builder.func);
            let atan2_ref = module.declare_func_in_func(atan2_id, builder.func);
            let functions = MathFunctions {
                sin: sin_ref,
                cos: cos_ref,
                sincos: sincos_ref,
                sqrt: sqrt_ref,
                ceil: ceil_ref,
                atan2: atan2_ref,
            };
            let mut registers = vec![None; program.fixed_ir().register_count];
            let mut loop_base_index = 1;
            let input_instance_bases = program
                .inputs
                .iter()
                .enumerate()
                .map(|(index, _input)| {
                    (!input_broadcast[index]).then(|| {
                        let base = header_params[loop_base_index];
                        loop_base_index += 1;
                        base
                    })
                })
                .collect::<Vec<_>>();
            for (index, input) in program.inputs.iter().enumerate() {
                let offset = program.register_offsets[&input.slot];
                for component in 0..input.shape.elements() {
                    let value = if let Some(values) = &input_broadcast_values[index] {
                        values[component]
                    } else {
                        load_component(
                            &mut builder,
                            input_instance_bases[index].unwrap(),
                            component,
                        )
                    };
                    registers[offset + component] = Some(NativeRegister::F32(value));
                }
            }
            let state_instance_bases = program
                .states
                .iter()
                .map(|_| {
                    let base = header_params[loop_base_index];
                    loop_base_index += 1;
                    base
                })
                .collect::<Vec<_>>();
            for (index, state) in program.states.iter().enumerate() {
                let offset = program.register_offsets[&state.slot];
                for component in 0..state.shape.elements() {
                    registers[offset + component] = Some(NativeRegister::F32(load_component(
                        &mut builder,
                        state_instance_bases[index],
                        component,
                    )));
                }
            }
            let next_state_instance_bases = program
                .states
                .iter()
                .map(|_| {
                    let base = header_params[loop_base_index];
                    loop_base_index += 1;
                    base
                })
                .collect::<Vec<_>>();
            debug_assert_eq!(loop_base_index, header_params.len());
            let instructions = &program.fixed_ir().instructions;
            let mut paired_outputs = BTreeSet::new();
            let mut common_values = BTreeMap::<String, NativeRegister>::new();
            for (instruction_index, instruction) in instructions.iter().enumerate() {
                if paired_outputs.remove(&instruction.output) {
                    continue;
                }
                let computation_key = format!("{:?}", instruction.computation);
                if let Some(value) = common_values.get(&computation_key).copied() {
                    registers[instruction.output] = Some(value);
                    continue;
                }
                if let Some((partner_index, current_is_sin, operand)) =
                    find_sincos_partner(instructions, instruction_index, &instruction.computation)
                {
                    let operand =
                        lower_numeric_operand(&mut builder, operand, &registers, &constant_values)?;
                    let (sin, cos) = call_sincos(&mut builder, functions.sincos, operand);
                    let current = if current_is_sin { sin } else { cos };
                    let partner = if current_is_sin { cos } else { sin };
                    registers[instruction.output] = Some(NativeRegister::F32(current));
                    registers[instructions[partner_index].output] =
                        Some(NativeRegister::F32(partner));
                    common_values.insert(computation_key, NativeRegister::F32(current));
                    common_values.insert(
                        format!("{:?}", instructions[partner_index].computation),
                        NativeRegister::F32(partner),
                    );
                    paired_outputs.insert(instructions[partner_index].output);
                    continue;
                }
                let value = lower_computation(
                    &mut builder,
                    &instruction.computation,
                    &registers,
                    functions,
                    &constant_values,
                )?;
                registers[instruction.output] = Some(value);
                common_values.insert(computation_key, value);
            }
            let constraint_result = if checked {
                let mut constraint_code = builder.ins().iconst(types::I32, 0);
                for (index, constraint) in program.constraints.iter().enumerate() {
                    let condition = lower_predicate(
                        &mut builder,
                        &constraint.predicate,
                        &registers,
                        &constant_values,
                    )?;
                    let code_is_empty = builder.ins().icmp_imm(IntCC::Equal, constraint_code, 0);
                    let failed = builder.ins().bnot(condition);
                    let record = builder.ins().band(code_is_empty, failed);
                    let code = builder.ins().iconst(types::I32, (index + 1) as i64);
                    constraint_code = builder.ins().select(record, code, constraint_code);
                }
                Some((constraint_code, constraint_code))
            } else {
                None
            };
            for (index, state) in program.states.iter().enumerate() {
                for (component, source) in state.update.iter().enumerate() {
                    let value =
                        lower_numeric_operand(&mut builder, *source, &registers, &constant_values)?;
                    store_component(
                        &mut builder,
                        next_state_instance_bases[index],
                        component,
                        value,
                    );
                }
            }

            if let Some((failed_mask, fault_code)) = constraint_result {
                let has_fault = builder.ins().icmp_imm(IntCC::NotEqual, failed_mask, 0);
                builder.ins().brif(
                    has_fault,
                    fault,
                    &[instance.into(), fault_code.into()],
                    advance,
                    &[],
                );
            } else {
                builder.ins().jump(advance, &[]);
            }

            builder.switch_to_block(advance);
            let next_instance = builder.ins().iadd_imm(instance, 1);
            let mut next_header_args = vec![cranelift_codegen::ir::BlockArg::Value(next_instance)];
            for (index, (_, stride)) in initial_loop_bases.iter().enumerate() {
                let next_base = builder.ins().iadd_imm(header_params[index + 1], *stride);
                next_header_args.push(cranelift_codegen::ir::BlockArg::Value(next_base));
            }
            builder.ins().jump(header, &next_header_args);

            builder.switch_to_block(fault);
            let fault_instance = builder.block_params(fault)[0];
            let fault_code = builder.block_params(fault)[1];
            let fault_instance = if pointer_type == types::I64 {
                fault_instance
            } else {
                builder.ins().uextend(types::I64, fault_instance)
            };
            let fault_code = builder.ins().uextend(types::I64, fault_code);
            let packed = builder.ins().ishl_imm(fault_instance, 8);
            let packed = builder.ins().bor(packed, fault_code);
            builder.ins().return_(&[packed]);

            builder.switch_to_block(exit);
            let success = builder.ins().iconst(types::I64, 0);
            builder.ins().return_(&[success]);
            builder.seal_all_blocks();
            builder.finalize();
        }

        module
            .define_function(function_id, &mut context)
            .map_err(native_error)?;
        module.clear_context(&mut context);
        module.finalize_definitions().map_err(native_error)?;
        let code = module.get_finalized_function(function_id);
        // SAFETY: `code` is the finalized entry point for the three-argument
        // signature constructed above. The module remains owned by the kernel.
        let turn = unsafe { mem::transmute::<*const u8, NativeTurn>(code) };
        Ok(Self {
            _module: module,
            turn,
        })
    }
}

impl NativeSimdKernel {
    fn compile(
        program: &FixedShapeKernel,
        checked: bool,
        input_broadcast: &[bool],
    ) -> Result<Self, BatchedExecutionError> {
        let mut jit_builder =
            JITBuilder::with_flags(&[("opt_level", "speed")], default_libcall_names())
                .map_err(native_error)?;
        jit_builder
            .symbol("mech_jit_sinf_f32x4", mech_jit_sinf_f32x4 as *const u8)
            .symbol("mech_jit_cosf_f32x4", mech_jit_cosf_f32x4 as *const u8)
            .symbol("mech_jit_sincos_f32x4", mech_jit_sincos_f32x4 as *const u8)
            .symbol("mech_jit_atan2_f32x4", mech_jit_atan2_f32x4 as *const u8);
        let mut module = JITModule::new(jit_builder);

        let pointer_type = module.target_config().pointer_type();
        let simd_unary_signature = {
            let mut signature = module.make_signature();
            #[cfg(any(target_arch = "aarch64", target_arch = "x86_64"))]
            {
                signature.params.push(AbiParam::new(types::F32X4));
                signature.returns.push(AbiParam::new(types::F32X4));
            }
            #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
            {
                signature.params.push(AbiParam::new(pointer_type));
                signature.params.push(AbiParam::new(pointer_type));
            }
            signature
        };
        let simd_binary_signature = {
            let mut signature = module.make_signature();
            #[cfg(any(target_arch = "aarch64", target_arch = "x86_64"))]
            {
                signature.params.push(AbiParam::new(types::F32X4));
                signature.params.push(AbiParam::new(types::F32X4));
                signature.returns.push(AbiParam::new(types::F32X4));
            }
            #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
            {
                signature.params.push(AbiParam::new(pointer_type));
                signature.params.push(AbiParam::new(pointer_type));
                signature.params.push(AbiParam::new(pointer_type));
            }
            signature
        };
        let simd_sincos_signature = {
            let mut signature = module.make_signature();
            #[cfg(any(target_arch = "aarch64", target_arch = "x86_64"))]
            {
                signature.params.push(AbiParam::new(types::F32X4));
                signature.params.push(AbiParam::new(pointer_type));
                signature.params.push(AbiParam::new(pointer_type));
            }
            #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
            {
                signature.params.push(AbiParam::new(pointer_type));
                signature.params.push(AbiParam::new(pointer_type));
                signature.params.push(AbiParam::new(pointer_type));
            }
            signature
        };
        let sin_simd_id = module
            .declare_function(
                "mech_jit_sinf_f32x4",
                Linkage::Import,
                &simd_unary_signature,
            )
            .map_err(native_error)?;
        let cos_simd_id = module
            .declare_function(
                "mech_jit_cosf_f32x4",
                Linkage::Import,
                &simd_unary_signature,
            )
            .map_err(native_error)?;
        let sincos_simd_id = module
            .declare_function(
                "mech_jit_sincos_f32x4",
                Linkage::Import,
                &simd_sincos_signature,
            )
            .map_err(native_error)?;
        let atan2_simd_id = module
            .declare_function(
                "mech_jit_atan2_f32x4",
                Linkage::Import,
                &simd_binary_signature,
            )
            .map_err(native_error)?;
        let mut signature = module.make_signature();
        for _ in 0..3 {
            signature.params.push(AbiParam::new(pointer_type));
        }
        signature.params.push(AbiParam::new(pointer_type));
        signature.params.push(AbiParam::new(pointer_type));
        signature.returns.push(AbiParam::new(types::I64));
        let function_id = module
            .declare_function("mech_fixed_numeric_simd_turn", Linkage::Local, &signature)
            .map_err(native_error)?;
        let mut context = module.make_context();
        context.func.signature = signature;
        context.func.name = UserFuncName::user(0, function_id.as_u32());
        let mut function_context = FunctionBuilderContext::new();

        {
            let mut builder = FunctionBuilder::new(&mut context.func, &mut function_context);
            let entry = builder.create_block();
            let header = builder.create_block();
            let body = builder.create_block();
            let advance = builder.create_block();
            let fault = builder.create_block();
            let exit = builder.create_block();
            builder.append_block_params_for_function_params(entry);
            builder.append_block_param(header, pointer_type);
            let dynamic_input_count = input_broadcast
                .iter()
                .filter(|broadcast| !**broadcast)
                .count();
            let loop_base_count = dynamic_input_count + program.states.len() * 2;
            for _ in 0..loop_base_count {
                builder.append_block_param(header, pointer_type);
            }
            builder.append_block_param(fault, pointer_type);
            builder.append_block_param(fault, types::I32);
            builder.switch_to_block(entry);

            let parameters = builder.block_params(entry).to_vec();
            let input_table = parameters[0];
            let state_table = parameters[1];
            let next_state_table = parameters[2];
            let pointer_bytes = i32::try_from(pointer_type.bytes()).unwrap();
            let flags = MemFlags::trusted();
            let input_bases = (0..program.inputs.len())
                .map(|index| {
                    builder.ins().load(
                        pointer_type,
                        flags,
                        input_table,
                        i32::try_from(index).unwrap() * pointer_bytes,
                    )
                })
                .collect::<Vec<_>>();
            let input_broadcast_values = program
                .inputs
                .iter()
                .enumerate()
                .map(|(index, input)| {
                    input_broadcast[index].then(|| {
                        (0..input.shape.elements())
                            .map(|component| {
                                let scalar = load_packed_scalar_component(
                                    &mut builder,
                                    input_bases[index],
                                    component,
                                );
                                builder.ins().splat(types::F32X4, scalar)
                            })
                            .collect::<Vec<_>>()
                    })
                })
                .collect::<Vec<_>>();
            let state_bases = (0..program.states.len())
                .map(|index| {
                    builder.ins().load(
                        pointer_type,
                        flags,
                        state_table,
                        i32::try_from(index).unwrap() * pointer_bytes,
                    )
                })
                .collect::<Vec<_>>();
            let next_state_bases = (0..program.states.len())
                .map(|index| {
                    builder.ins().load(
                        pointer_type,
                        flags,
                        next_state_table,
                        i32::try_from(index).unwrap() * pointer_bytes,
                    )
                })
                .collect::<Vec<_>>();
            let constant_values = collect_constant_bits(program)
                .into_iter()
                .map(|bits| {
                    let scalar = builder.ins().f32const(f32::from_bits(bits));
                    (bits, builder.ins().splat(types::F32X4, scalar))
                })
                .collect::<BTreeMap<_, _>>();
            let start_group = builder.block_params(entry)[3];
            let end_group = builder.block_params(entry)[4];
            let mut initial_loop_bases = Vec::with_capacity(loop_base_count);
            for (index, input) in program.inputs.iter().enumerate() {
                if !input_broadcast[index] {
                    let stride = i64::try_from(
                        input
                            .shape
                            .elements()
                            .checked_mul(SIMD_JIT_LANES)
                            .unwrap()
                            .checked_mul(types::F32.bytes() as usize)
                            .unwrap(),
                    )
                    .unwrap();
                    let offset = builder.ins().imul_imm(start_group, stride);
                    let base = builder.ins().iadd(input_bases[index], offset);
                    initial_loop_bases.push((base, stride));
                }
            }
            for (index, state) in program.states.iter().enumerate() {
                let stride = i64::try_from(
                    state
                        .shape
                        .elements()
                        .checked_mul(SIMD_JIT_LANES)
                        .unwrap()
                        .checked_mul(types::F32.bytes() as usize)
                        .unwrap(),
                )
                .unwrap();
                let offset = builder.ins().imul_imm(start_group, stride);
                let base = builder.ins().iadd(state_bases[index], offset);
                initial_loop_bases.push((base, stride));
            }
            for (index, state) in program.states.iter().enumerate() {
                let stride = i64::try_from(
                    state
                        .shape
                        .elements()
                        .checked_mul(SIMD_JIT_LANES)
                        .unwrap()
                        .checked_mul(types::F32.bytes() as usize)
                        .unwrap(),
                )
                .unwrap();
                let offset = builder.ins().imul_imm(start_group, stride);
                let base = builder.ins().iadd(next_state_bases[index], offset);
                initial_loop_bases.push((base, stride));
            }
            debug_assert_eq!(initial_loop_bases.len(), loop_base_count);
            let mut initial_header_args = vec![cranelift_codegen::ir::BlockArg::Value(start_group)];
            initial_header_args.extend(
                initial_loop_bases
                    .iter()
                    .map(|(base, _)| cranelift_codegen::ir::BlockArg::Value(*base)),
            );
            builder.ins().jump(header, &initial_header_args);

            builder.switch_to_block(header);
            let header_params = builder.block_params(header).to_vec();
            let group = header_params[0];
            let has_group = builder
                .ins()
                .icmp(IntCC::UnsignedLessThan, group, end_group);
            builder.ins().brif(has_group, body, &[], exit, &[]);

            builder.switch_to_block(body);
            let sin_simd_ref = module.declare_func_in_func(sin_simd_id, builder.func);
            let cos_simd_ref = module.declare_func_in_func(cos_simd_id, builder.func);
            let sincos_simd_ref = module.declare_func_in_func(sincos_simd_id, builder.func);
            let atan2_simd_ref = module.declare_func_in_func(atan2_simd_id, builder.func);
            let simd_functions = SimdMathFunctions {
                sin: sin_simd_ref,
                cos: cos_simd_ref,
                sincos: sincos_simd_ref,
                atan2: atan2_simd_ref,
            };
            let mut registers = vec![None; program.fixed_ir().register_count];
            let mut loop_base_index = 1;
            let input_instance_bases = program
                .inputs
                .iter()
                .enumerate()
                .map(|(index, _input)| {
                    (!input_broadcast[index]).then(|| {
                        let base = header_params[loop_base_index];
                        loop_base_index += 1;
                        base
                    })
                })
                .collect::<Vec<_>>();
            for (index, input) in program.inputs.iter().enumerate() {
                let offset = program.register_offsets[&input.slot];
                for component in 0..input.shape.elements() {
                    let value = if let Some(values) = &input_broadcast_values[index] {
                        values[component]
                    } else {
                        load_simd_component(
                            &mut builder,
                            input_instance_bases[index].unwrap(),
                            input.shape.elements(),
                            component,
                        )
                    };
                    registers[offset + component] = Some(NativeSimdRegister::F32(value));
                }
            }
            let state_instance_bases = program
                .states
                .iter()
                .map(|_| {
                    let base = header_params[loop_base_index];
                    loop_base_index += 1;
                    base
                })
                .collect::<Vec<_>>();
            for (index, state) in program.states.iter().enumerate() {
                let offset = program.register_offsets[&state.slot];
                for component in 0..state.shape.elements() {
                    registers[offset + component] =
                        Some(NativeSimdRegister::F32(load_simd_component(
                            &mut builder,
                            state_instance_bases[index],
                            state.shape.elements(),
                            component,
                        )));
                }
            }
            let next_state_instance_bases = program
                .states
                .iter()
                .map(|_| {
                    let base = header_params[loop_base_index];
                    loop_base_index += 1;
                    base
                })
                .collect::<Vec<_>>();
            debug_assert_eq!(loop_base_index, header_params.len());
            let instructions = &program.fixed_ir().instructions;
            let mut paired_outputs = BTreeSet::new();
            let mut common_values = BTreeMap::<String, NativeSimdRegister>::new();
            for (instruction_index, instruction) in instructions.iter().enumerate() {
                if paired_outputs.remove(&instruction.output) {
                    continue;
                }
                let computation_key = format!("{:?}", instruction.computation);
                if let Some(value) = common_values.get(&computation_key).copied() {
                    registers[instruction.output] = Some(value);
                    continue;
                }
                if let Some((partner_index, current_is_sin, operand)) =
                    find_sincos_partner(instructions, instruction_index, &instruction.computation)
                {
                    let value = lower_simd_numeric_operand(operand, &registers, &constant_values)?;
                    let (sin, cos) =
                        call_simd_sincos(&mut builder, simd_functions.sincos, value, pointer_type);
                    let current = if current_is_sin { sin } else { cos };
                    let partner = if current_is_sin { cos } else { sin };
                    registers[instruction.output] = Some(NativeSimdRegister::F32(current));
                    registers[instructions[partner_index].output] =
                        Some(NativeSimdRegister::F32(partner));
                    common_values.insert(computation_key, NativeSimdRegister::F32(current));
                    common_values.insert(
                        format!("{:?}", instructions[partner_index].computation),
                        NativeSimdRegister::F32(partner),
                    );
                    paired_outputs.insert(instructions[partner_index].output);
                    continue;
                }
                let value = lower_simd_computation(
                    &mut builder,
                    &instruction.computation,
                    &registers,
                    simd_functions,
                    &constant_values,
                    pointer_type,
                )?;
                registers[instruction.output] = Some(value);
                common_values.insert(computation_key, value);
            }

            let mut check_block = body;
            if checked {
                for (index, constraint) in program.constraints.iter().enumerate() {
                    // The first predicate is lowered in `body`; subsequent
                    // predicates start in the open fall-through successor of
                    // the previous constraint's final lane.
                    if index != 0 {
                        builder.switch_to_block(check_block);
                    }
                    let valid = lower_simd_predicate(
                        &mut builder,
                        &constraint.predicate,
                        &registers,
                        &constant_values,
                    )?;
                    for lane in 0..SIMD_JIT_LANES {
                        let next_check = builder.create_block();
                        if lane != 0 {
                            builder.switch_to_block(check_block);
                        }
                        let lane_valid = builder.ins().extractlane(valid, lane as u8);
                        let failed = builder.ins().icmp_imm(IntCC::Equal, lane_valid, 0);
                        let lane_group = builder.ins().imul_imm(group, SIMD_JIT_LANES as i64);
                        let lane_instance = builder.ins().iadd_imm(lane_group, lane as i64);
                        let code = builder.ins().iconst(types::I32, (index + 1) as i64);
                        builder.ins().brif(
                            failed,
                            fault,
                            &[
                                cranelift_codegen::ir::BlockArg::Value(lane_instance),
                                cranelift_codegen::ir::BlockArg::Value(code),
                            ],
                            next_check,
                            &[],
                        );
                        check_block = next_check;
                    }
                }
            }
            if checked && !program.constraints.is_empty() {
                builder.switch_to_block(check_block);
            }
            for (index, state) in program.states.iter().enumerate() {
                for (component, source) in state.update.iter().enumerate() {
                    let value = lower_simd_numeric_operand(*source, &registers, &constant_values)?;
                    store_simd_component(
                        &mut builder,
                        next_state_instance_bases[index],
                        state.shape.elements(),
                        component,
                        value,
                    );
                }
            }
            builder.ins().jump(advance, &[]);

            builder.switch_to_block(advance);
            let next_group = builder.ins().iadd_imm(group, 1);
            let mut next_header_args = vec![cranelift_codegen::ir::BlockArg::Value(next_group)];
            for (index, (_, stride)) in initial_loop_bases.iter().enumerate() {
                let next_base = builder.ins().iadd_imm(header_params[index + 1], *stride);
                next_header_args.push(cranelift_codegen::ir::BlockArg::Value(next_base));
            }
            builder.ins().jump(header, &next_header_args);

            builder.switch_to_block(fault);
            let fault_instance = builder.block_params(fault)[0];
            let fault_code = builder.block_params(fault)[1];
            let fault_instance = if pointer_type == types::I64 {
                fault_instance
            } else {
                builder.ins().uextend(types::I64, fault_instance)
            };
            let fault_code = builder.ins().uextend(types::I64, fault_code);
            let packed = builder.ins().ishl_imm(fault_instance, 8);
            let packed = builder.ins().bor(packed, fault_code);
            builder.ins().return_(&[packed]);

            builder.switch_to_block(exit);
            let success = builder.ins().iconst(types::I64, 0);
            builder.ins().return_(&[success]);
            builder.seal_all_blocks();
            builder.finalize();
        }

        module
            .define_function(function_id, &mut context)
            .map_err(|error| {
                BatchedExecutionError::Native(format!(
                    "Cranelift SIMD JIT: {error:?}\n{}",
                    context.func
                ))
            })?;
        module.clear_context(&mut context);
        module.finalize_definitions().map_err(native_error)?;
        let code = module.get_finalized_function(function_id);
        // SAFETY: `code` is the finalized entry point for the three-pointer
        // signature above. The module remains owned by the kernel.
        let turn = unsafe { mem::transmute::<*const u8, NativeSimdTurn>(code) };
        Ok(Self {
            _module: module,
            turn,
        })
    }
}

#[derive(Clone, Copy)]
struct MathFunctions {
    sin: cranelift_codegen::ir::FuncRef,
    cos: cranelift_codegen::ir::FuncRef,
    sincos: cranelift_codegen::ir::FuncRef,
    sqrt: cranelift_codegen::ir::FuncRef,
    ceil: cranelift_codegen::ir::FuncRef,
    atan2: cranelift_codegen::ir::FuncRef,
}

#[derive(Clone, Copy)]
struct SimdMathFunctions {
    sin: cranelift_codegen::ir::FuncRef,
    cos: cranelift_codegen::ir::FuncRef,
    sincos: cranelift_codegen::ir::FuncRef,
    atan2: cranelift_codegen::ir::FuncRef,
}

#[derive(Clone, Copy)]
enum NativeRegister {
    F32(Value),
    Bool(Value),
}

#[derive(Clone, Copy)]
enum NativeSimdRegister {
    F32(Value),
    Bool(Value),
}

fn collect_constant_operand(operand: ScalarOperand, constants: &mut BTreeSet<u32>) {
    if let ScalarOperand::Constant(value) = operand {
        constants.insert(value.to_bits());
    }
}

fn collect_constant_computation(computation: &ScalarComputation, constants: &mut BTreeSet<u32>) {
    match computation {
        ScalarComputation::Copy(input)
        | ScalarComputation::Negate(input)
        | ScalarComputation::Absolute(input)
        | ScalarComputation::IsFinite(input) => collect_constant_operand(*input, constants),
        ScalarComputation::Compare { left, right, .. } => {
            collect_constant_operand(*left, constants);
            collect_constant_operand(*right, constants);
        }
        ScalarComputation::Logic { inputs, .. } | ScalarComputation::Elementwise { inputs, .. } => {
            for input in inputs {
                collect_constant_operand(*input, constants);
            }
        }
        ScalarComputation::SumProducts(terms) => {
            for (left, right) in terms {
                collect_constant_operand(*left, constants);
                collect_constant_operand(*right, constants);
            }
        }
    }
}

fn collect_constant_predicate(predicate: &ScalarPredicate, constants: &mut BTreeSet<u32>) {
    match predicate {
        ScalarPredicate::Value(operand) | ScalarPredicate::IsFinite(operand) => {
            collect_constant_operand(*operand, constants)
        }
        ScalarPredicate::AbsoluteDifferenceWithin {
            left,
            right,
            tolerance,
        } => {
            collect_constant_operand(*left, constants);
            collect_constant_operand(*right, constants);
            collect_constant_operand(*tolerance, constants);
        }
        ScalarPredicate::Compare { left, right, .. } => {
            collect_constant_operand(*left, constants);
            collect_constant_operand(*right, constants);
        }
        ScalarPredicate::All(inputs) | ScalarPredicate::Logic { inputs, .. } => {
            for input in inputs {
                collect_constant_predicate(input, constants);
            }
        }
    }
}

fn collect_constant_bits(program: &FixedShapeKernel) -> BTreeSet<u32> {
    let mut constants = BTreeSet::from([0.0_f32.to_bits()]);
    for instruction in &program.fixed_ir().instructions {
        collect_constant_computation(&instruction.computation, &mut constants);
    }
    for constraint in &program.constraints {
        collect_constant_predicate(&constraint.predicate, &mut constants);
    }
    for state in &program.states {
        for source in &state.update {
            collect_constant_operand(*source, &mut constants);
        }
    }
    constants
}

fn unary_math_operand(
    computation: &ScalarComputation,
    operation: UnaryOperation,
) -> Option<ScalarOperand> {
    match computation {
        ScalarComputation::Elementwise {
            operation: ElementwiseOperation::Unary(candidate),
            inputs,
        } if *candidate == operation => inputs.first().copied(),
        _ => None,
    }
}

fn same_scalar_operand(left: ScalarOperand, right: ScalarOperand) -> bool {
    match (left, right) {
        (ScalarOperand::Register(left), ScalarOperand::Register(right)) => left == right,
        (ScalarOperand::Constant(left), ScalarOperand::Constant(right)) => {
            left.to_bits() == right.to_bits()
        }
        _ => false,
    }
}

fn find_sincos_partner(
    instructions: &[ScalarInstruction],
    instruction_index: usize,
    computation: &ScalarComputation,
) -> Option<(usize, bool, ScalarOperand)> {
    let (operation, operand) =
        if let Some(operand) = unary_math_operand(computation, UnaryOperation::Sin) {
            (UnaryOperation::Sin, operand)
        } else if let Some(operand) = unary_math_operand(computation, UnaryOperation::Cos) {
            (UnaryOperation::Cos, operand)
        } else {
            return None;
        };
    let partner_operation = match operation {
        UnaryOperation::Sin => UnaryOperation::Cos,
        UnaryOperation::Cos => UnaryOperation::Sin,
        _ => unreachable!(),
    };
    instructions
        .iter()
        .enumerate()
        .skip(instruction_index + 1)
        .find_map(|(candidate_index, candidate)| {
            let candidate_operand = unary_math_operand(&candidate.computation, partner_operation)?;
            same_scalar_operand(operand, candidate_operand).then_some((
                candidate_index,
                operation == UnaryOperation::Sin,
                operand,
            ))
        })
}

fn lower_computation(
    builder: &mut FunctionBuilder<'_>,
    computation: &ScalarComputation,
    registers: &[Option<NativeRegister>],
    functions: MathFunctions,
    constants: &BTreeMap<u32, Value>,
) -> Result<NativeRegister, BatchedExecutionError> {
    Ok(match computation {
        ScalarComputation::Copy(input) => lower_operand(builder, *input, registers, constants)?,
        ScalarComputation::Negate(input) => {
            let value = lower_numeric_operand(builder, *input, registers, constants)?;
            NativeRegister::F32(builder.ins().fneg(value))
        }
        ScalarComputation::Absolute(input) => {
            let value = lower_numeric_operand(builder, *input, registers, constants)?;
            NativeRegister::F32(builder.ins().fabs(value))
        }
        ScalarComputation::IsFinite(input) => {
            let value = lower_numeric_operand(builder, *input, registers, constants)?;
            NativeRegister::Bool(lower_is_finite(builder, value))
        }
        ScalarComputation::Compare {
            operation,
            left,
            right,
        } => {
            let left = lower_numeric_operand(builder, *left, registers, constants)?;
            let right = lower_numeric_operand(builder, *right, registers, constants)?;
            let condition = builder.ins().fcmp(
                match operation {
                    ComparisonOperation::Equal => FloatCC::Equal,
                    ComparisonOperation::NotEqual => FloatCC::NotEqual,
                    ComparisonOperation::Less => FloatCC::LessThan,
                    ComparisonOperation::Greater => FloatCC::GreaterThan,
                    ComparisonOperation::LessEqual => FloatCC::LessThanOrEqual,
                    ComparisonOperation::GreaterEqual => FloatCC::GreaterThanOrEqual,
                },
                left,
                right,
            );
            NativeRegister::Bool(condition)
        }
        ScalarComputation::Logic { operation, inputs } => {
            let left = lower_boolean_operand(builder, inputs[0], registers, constants)?;
            let condition = if *operation == LogicOperation::Not {
                builder.ins().bnot(left)
            } else {
                let right = lower_boolean_operand(builder, inputs[1], registers, constants)?;
                match operation {
                    LogicOperation::And => builder.ins().band(left, right),
                    LogicOperation::Or => builder.ins().bor(left, right),
                    LogicOperation::Xor => builder.ins().bxor(left, right),
                    LogicOperation::Not => unreachable!(),
                }
            };
            NativeRegister::Bool(condition)
        }
        ScalarComputation::Elementwise { operation, inputs } => {
            let values = inputs
                .iter()
                .map(|input| lower_numeric_operand(builder, *input, registers, constants))
                .collect::<Result<Vec<_>, _>>()?;
            NativeRegister::F32(match operation {
                ElementwiseOperation::Binary(operation) => match operation {
                    BinaryOperation::Add => builder.ins().fadd(values[0], values[1]),
                    BinaryOperation::Subtract => builder.ins().fsub(values[0], values[1]),
                    BinaryOperation::Multiply => builder.ins().fmul(values[0], values[1]),
                    BinaryOperation::Divide => builder.ins().fdiv(values[0], values[1]),
                },
                ElementwiseOperation::Unary(operation) => match operation {
                    UnaryOperation::Sin => call_math(builder, functions.sin, &values),
                    UnaryOperation::Cos => call_math(builder, functions.cos, &values),
                    UnaryOperation::Sqrt => call_math(builder, functions.sqrt, &values),
                    UnaryOperation::Ceil => call_math(builder, functions.ceil, &values),
                },
                ElementwiseOperation::Atan2 => call_math(builder, functions.atan2, &values),
                ElementwiseOperation::Identity => values[0],
            })
        }
        ScalarComputation::SumProducts(terms) => {
            return lower_sum_products(builder, terms, registers, constants);
        }
    })
}

fn lower_sum_products(
    builder: &mut FunctionBuilder<'_>,
    terms: &[(ScalarOperand, ScalarOperand)],
    registers: &[Option<NativeRegister>],
    constants: &BTreeMap<u32, Value>,
) -> Result<NativeRegister, BatchedExecutionError> {
    let mut sum = None;
    for (left, right) in terms {
        let value = match sum {
            None => {
                let left_operand = *left;
                let right_operand = *right;
                let left = lower_numeric_operand(builder, *left, registers, constants)?;
                let right = lower_numeric_operand(builder, *right, registers, constants)?;
                match (left_operand, right_operand) {
                    (ScalarOperand::Constant(value), _) if value == 1.0 => right,
                    (_, ScalarOperand::Constant(value)) if value == 1.0 => left,
                    (ScalarOperand::Constant(value), _) if value == -1.0 => {
                        builder.ins().fneg(right)
                    }
                    (_, ScalarOperand::Constant(value)) if value == -1.0 => {
                        builder.ins().fneg(left)
                    }
                    _ => builder.ins().fmul(left, right),
                }
            }
            Some(sum) => {
                let left_operand = *left;
                let right_operand = *right;
                let left = lower_numeric_operand(builder, *left, registers, constants)?;
                let right = lower_numeric_operand(builder, *right, registers, constants)?;
                match (left_operand, right_operand) {
                    (ScalarOperand::Constant(value), _) if value == 1.0 => {
                        builder.ins().fadd(sum, right)
                    }
                    (_, ScalarOperand::Constant(value)) if value == 1.0 => {
                        builder.ins().fadd(sum, left)
                    }
                    (ScalarOperand::Constant(value), _) if value == -1.0 => {
                        builder.ins().fsub(sum, right)
                    }
                    (_, ScalarOperand::Constant(value)) if value == -1.0 => {
                        builder.ins().fsub(sum, left)
                    }
                    _ => builder.ins().fma(left, right, sum),
                }
            }
        };
        sum = Some(value);
    }
    Ok(NativeRegister::F32(
        sum.unwrap_or_else(|| constants[&0.0_f32.to_bits()]),
    ))
}

fn lower_simd_computation(
    builder: &mut FunctionBuilder<'_>,
    computation: &ScalarComputation,
    registers: &[Option<NativeSimdRegister>],
    functions: SimdMathFunctions,
    constants: &BTreeMap<u32, Value>,
    pointer_type: Type,
) -> Result<NativeSimdRegister, BatchedExecutionError> {
    Ok(match computation {
        ScalarComputation::Copy(input) => lower_simd_operand(*input, registers, constants)?,
        ScalarComputation::Negate(input) => {
            let value = lower_simd_numeric_operand(*input, registers, constants)?;
            NativeSimdRegister::F32(builder.ins().fneg(value))
        }
        ScalarComputation::Absolute(input) => {
            let value = lower_simd_numeric_operand(*input, registers, constants)?;
            NativeSimdRegister::F32(builder.ins().fabs(value))
        }
        ScalarComputation::IsFinite(input) => {
            let value = lower_simd_numeric_operand(*input, registers, constants)?;
            NativeSimdRegister::Bool(lower_simd_is_finite(builder, value))
        }
        ScalarComputation::Compare {
            operation,
            left,
            right,
        } => {
            let left = lower_simd_numeric_operand(*left, registers, constants)?;
            let right = lower_simd_numeric_operand(*right, registers, constants)?;
            NativeSimdRegister::Bool(builder.ins().fcmp(
                match operation {
                    ComparisonOperation::Equal => FloatCC::Equal,
                    ComparisonOperation::NotEqual => FloatCC::NotEqual,
                    ComparisonOperation::Less => FloatCC::LessThan,
                    ComparisonOperation::Greater => FloatCC::GreaterThan,
                    ComparisonOperation::LessEqual => FloatCC::LessThanOrEqual,
                    ComparisonOperation::GreaterEqual => FloatCC::GreaterThanOrEqual,
                },
                left,
                right,
            ))
        }
        ScalarComputation::Logic { operation, inputs } => {
            let left = lower_simd_boolean_operand(inputs[0], registers, constants)?;
            let condition = if *operation == LogicOperation::Not {
                builder.ins().bnot(left)
            } else {
                let right = lower_simd_boolean_operand(inputs[1], registers, constants)?;
                match operation {
                    LogicOperation::And => builder.ins().band(left, right),
                    LogicOperation::Or => builder.ins().bor(left, right),
                    LogicOperation::Xor => builder.ins().bxor(left, right),
                    LogicOperation::Not => unreachable!(),
                }
            };
            NativeSimdRegister::Bool(condition)
        }
        ScalarComputation::Elementwise { operation, inputs } => {
            let values = inputs
                .iter()
                .map(|input| lower_simd_numeric_operand(*input, registers, constants))
                .collect::<Result<Vec<_>, _>>()?;
            NativeSimdRegister::F32(match operation {
                ElementwiseOperation::Binary(operation) => match operation {
                    BinaryOperation::Add => builder.ins().fadd(values[0], values[1]),
                    BinaryOperation::Subtract => builder.ins().fsub(values[0], values[1]),
                    BinaryOperation::Multiply => builder.ins().fmul(values[0], values[1]),
                    BinaryOperation::Divide => builder.ins().fdiv(values[0], values[1]),
                },
                ElementwiseOperation::Unary(operation) => match operation {
                    UnaryOperation::Sin => {
                        call_simd_unary_math(builder, functions.sin, values[0], pointer_type)
                    }
                    UnaryOperation::Cos => {
                        call_simd_unary_math(builder, functions.cos, values[0], pointer_type)
                    }
                    UnaryOperation::Sqrt => builder.ins().sqrt(values[0]),
                    UnaryOperation::Ceil => builder.ins().ceil(values[0]),
                },
                ElementwiseOperation::Atan2 => call_simd_binary_math(
                    builder,
                    functions.atan2,
                    values[0],
                    values[1],
                    pointer_type,
                ),
                ElementwiseOperation::Identity => values[0],
            })
        }
        ScalarComputation::SumProducts(terms) => {
            return lower_simd_sum_products(builder, terms, registers, constants);
        }
    })
}

fn lower_simd_sum_products(
    builder: &mut FunctionBuilder<'_>,
    terms: &[(ScalarOperand, ScalarOperand)],
    registers: &[Option<NativeSimdRegister>],
    constants: &BTreeMap<u32, Value>,
) -> Result<NativeSimdRegister, BatchedExecutionError> {
    let mut sum = None;
    for (left, right) in terms {
        let value = match sum {
            None => {
                let left_operand = *left;
                let right_operand = *right;
                let left = lower_simd_numeric_operand(*left, registers, constants)?;
                let right = lower_simd_numeric_operand(*right, registers, constants)?;
                match (left_operand, right_operand) {
                    (ScalarOperand::Constant(value), _) if value == 1.0 => right,
                    (_, ScalarOperand::Constant(value)) if value == 1.0 => left,
                    (ScalarOperand::Constant(value), _) if value == -1.0 => {
                        builder.ins().fneg(right)
                    }
                    (_, ScalarOperand::Constant(value)) if value == -1.0 => {
                        builder.ins().fneg(left)
                    }
                    _ => builder.ins().fmul(left, right),
                }
            }
            Some(sum) => {
                let left_operand = *left;
                let right_operand = *right;
                let left = lower_simd_numeric_operand(*left, registers, constants)?;
                let right = lower_simd_numeric_operand(*right, registers, constants)?;
                match (left_operand, right_operand) {
                    (ScalarOperand::Constant(value), _) if value == 1.0 => {
                        builder.ins().fadd(sum, right)
                    }
                    (_, ScalarOperand::Constant(value)) if value == 1.0 => {
                        builder.ins().fadd(sum, left)
                    }
                    (ScalarOperand::Constant(value), _) if value == -1.0 => {
                        builder.ins().fsub(sum, right)
                    }
                    (_, ScalarOperand::Constant(value)) if value == -1.0 => {
                        builder.ins().fsub(sum, left)
                    }
                    _ => builder.ins().fma(left, right, sum),
                }
            }
        };
        sum = Some(value);
    }
    Ok(NativeSimdRegister::F32(
        sum.unwrap_or_else(|| constants[&0.0_f32.to_bits()]),
    ))
}

fn lower_simd_predicate(
    builder: &mut FunctionBuilder<'_>,
    predicate: &ScalarPredicate,
    registers: &[Option<NativeSimdRegister>],
    constants: &BTreeMap<u32, Value>,
) -> Result<Value, BatchedExecutionError> {
    Ok(match predicate {
        ScalarPredicate::Value(operand) => {
            match lower_simd_operand(*operand, registers, constants)? {
                NativeSimdRegister::Bool(value) => value,
                NativeSimdRegister::F32(value) => {
                    let zero = constants[&0.0f32.to_bits()];
                    builder.ins().fcmp(FloatCC::NotEqual, value, zero)
                }
            }
        }
        ScalarPredicate::IsFinite(operand) => lower_simd_is_finite(builder, {
            let value = lower_simd_numeric_operand(*operand, registers, constants)?;
            value
        }),
        ScalarPredicate::AbsoluteDifferenceWithin {
            left,
            right,
            tolerance,
        } => {
            let left = lower_simd_numeric_operand(*left, registers, constants)?;
            let right = lower_simd_numeric_operand(*right, registers, constants)?;
            let tolerance = lower_simd_numeric_operand(*tolerance, registers, constants)?;
            let diff = builder.ins().fsub(left, right);
            let diff = builder.ins().fabs(diff);
            builder
                .ins()
                .fcmp(FloatCC::LessThanOrEqual, diff, tolerance)
        }
        ScalarPredicate::Compare {
            operation,
            left,
            right,
        } => {
            let left = lower_simd_numeric_operand(*left, registers, constants)?;
            let right = lower_simd_numeric_operand(*right, registers, constants)?;
            builder.ins().fcmp(
                match operation {
                    ComparisonOperation::Equal => FloatCC::Equal,
                    ComparisonOperation::NotEqual => FloatCC::NotEqual,
                    ComparisonOperation::Less => FloatCC::LessThan,
                    ComparisonOperation::Greater => FloatCC::GreaterThan,
                    ComparisonOperation::LessEqual => FloatCC::LessThanOrEqual,
                    ComparisonOperation::GreaterEqual => FloatCC::GreaterThanOrEqual,
                },
                left,
                right,
            )
        }
        ScalarPredicate::All(inputs) => {
            let mut inputs = inputs.iter();
            let mut condition = lower_simd_predicate(
                builder,
                inputs.next().expect("flattened conjunction is non-empty"),
                registers,
                constants,
            )?;
            for input in inputs {
                let next = lower_simd_predicate(builder, input, registers, constants)?;
                condition = builder.ins().band(condition, next);
            }
            condition
        }
        ScalarPredicate::Logic { operation, inputs } => {
            let left = lower_simd_predicate(builder, &inputs[0], registers, constants)?;
            if *operation == LogicOperation::Not {
                builder.ins().bnot(left)
            } else {
                let right = lower_simd_predicate(builder, &inputs[1], registers, constants)?;
                match operation {
                    LogicOperation::And => builder.ins().band(left, right),
                    LogicOperation::Or => builder.ins().bor(left, right),
                    LogicOperation::Xor => builder.ins().bxor(left, right),
                    LogicOperation::Not => unreachable!(),
                }
            }
        }
    })
}

fn lower_simd_operand(
    operand: ScalarOperand,
    registers: &[Option<NativeSimdRegister>],
    constants: &BTreeMap<u32, Value>,
) -> Result<NativeSimdRegister, BatchedExecutionError> {
    match operand {
        ScalarOperand::Register(register) => registers[register].ok_or_else(|| {
            BatchedExecutionError::Native(format!(
                "native SIMD lowering read register {register} before definition"
            ))
        }),
        ScalarOperand::Constant(value) => Ok(NativeSimdRegister::F32(
            constants.get(&value.to_bits()).copied().ok_or_else(|| {
                BatchedExecutionError::Native(format!(
                    "native SIMD lowering constant {value:?} was not hoisted"
                ))
            })?,
        )),
    }
}

fn lower_simd_numeric_operand(
    operand: ScalarOperand,
    registers: &[Option<NativeSimdRegister>],
    constants: &BTreeMap<u32, Value>,
) -> Result<Value, BatchedExecutionError> {
    match lower_simd_operand(operand, registers, constants)? {
        NativeSimdRegister::F32(value) => Ok(value),
        NativeSimdRegister::Bool(_) => Err(BatchedExecutionError::Native(
            "native SIMD numeric operation received a boolean operand".to_owned(),
        )),
    }
}

fn lower_simd_boolean_operand(
    operand: ScalarOperand,
    registers: &[Option<NativeSimdRegister>],
    constants: &BTreeMap<u32, Value>,
) -> Result<Value, BatchedExecutionError> {
    match lower_simd_operand(operand, registers, constants)? {
        NativeSimdRegister::Bool(value) => Ok(value),
        NativeSimdRegister::F32(_) => Err(BatchedExecutionError::Native(
            "native SIMD boolean operation received a numeric operand".to_owned(),
        )),
    }
}

fn pack_simd_instances(values: &[f32], elements: usize) -> Vec<f32> {
    let instances = values.len() / elements;
    let mut packed = vec![0.0; values.len()];
    for group in (0..instances).step_by(SIMD_JIT_LANES) {
        for component in 0..elements {
            for lane in 0..SIMD_JIT_LANES {
                packed[(group / SIMD_JIT_LANES) * elements * SIMD_JIT_LANES
                    + component * SIMD_JIT_LANES
                    + lane] = values[(group + lane) * elements + component];
            }
        }
    }
    packed
}

fn unpack_simd_instances(packed: &[f32], values: &mut [f32], elements: usize) {
    let instances = values.len() / elements;
    for group in (0..instances).step_by(SIMD_JIT_LANES) {
        for component in 0..elements {
            for lane in 0..SIMD_JIT_LANES {
                let packed_index = (group / SIMD_JIT_LANES) * elements * SIMD_JIT_LANES
                    + component * SIMD_JIT_LANES
                    + lane;
                let value_index = (group + lane) * elements + component;
                values[value_index] = packed[packed_index];
            }
        }
    }
}

fn simd_stack_slot(builder: &mut FunctionBuilder<'_>) -> cranelift_codegen::ir::StackSlot {
    builder.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 16, 4))
}

fn stack_address(
    builder: &mut FunctionBuilder<'_>,
    slot: cranelift_codegen::ir::StackSlot,
    pointer_type: Type,
) -> Value {
    builder.ins().stack_addr(pointer_type, slot, 0)
}

fn call_simd_unary_math(
    builder: &mut FunctionBuilder<'_>,
    function: cranelift_codegen::ir::FuncRef,
    value: Value,
    _pointer_type: Type,
) -> Value {
    #[cfg(any(target_arch = "aarch64", target_arch = "x86_64"))]
    {
        let call = builder.ins().call(function, &[value]);
        builder.inst_results(call)[0]
    }
    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
    {
        let output_slot = simd_stack_slot(builder);
        let output = stack_address(builder, output_slot, _pointer_type);
        let input_slot = simd_stack_slot(builder);
        let input = stack_address(builder, input_slot, _pointer_type);
        builder.ins().store(MemFlags::trusted(), value, input, 0);
        builder.ins().call(function, &[input, output]);
        builder
            .ins()
            .load(types::F32X4, MemFlags::trusted(), output, 0)
    }
}

fn call_simd_binary_math(
    builder: &mut FunctionBuilder<'_>,
    function: cranelift_codegen::ir::FuncRef,
    left: Value,
    right: Value,
    _pointer_type: Type,
) -> Value {
    #[cfg(any(target_arch = "aarch64", target_arch = "x86_64"))]
    {
        let call = builder.ins().call(function, &[left, right]);
        builder.inst_results(call)[0]
    }
    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
    {
        let output_slot = simd_stack_slot(builder);
        let output = stack_address(builder, output_slot, _pointer_type);
        let left_slot = simd_stack_slot(builder);
        let right_slot = simd_stack_slot(builder);
        let left_address = stack_address(builder, left_slot, _pointer_type);
        let right_address = stack_address(builder, right_slot, _pointer_type);
        builder
            .ins()
            .store(MemFlags::trusted(), left, left_address, 0);
        builder
            .ins()
            .store(MemFlags::trusted(), right, right_address, 0);
        builder
            .ins()
            .call(function, &[left_address, right_address, output]);
        builder
            .ins()
            .load(types::F32X4, MemFlags::trusted(), output, 0)
    }
}

fn lower_simd_is_finite(builder: &mut FunctionBuilder<'_>, value: Value) -> Value {
    // Ordered comparison against the largest finite f32 rejects both NaNs and
    // infinities while remaining entirely in the generated SIMD body.
    let absolute = builder.ins().fabs(value);
    let maximum = builder.ins().f32const(f32::MAX);
    let maximum = builder.ins().splat(types::F32X4, maximum);
    builder
        .ins()
        .fcmp(FloatCC::LessThanOrEqual, absolute, maximum)
}

fn load_simd_component(
    builder: &mut FunctionBuilder<'_>,
    base: Value,
    _elements: usize,
    component: usize,
) -> Value {
    let offset = i32::try_from(
        component
            .checked_mul(SIMD_JIT_LANES)
            .and_then(|offset| offset.checked_mul(types::F32.bytes() as usize))
            .unwrap(),
    )
    .unwrap();
    builder
        .ins()
        .load(types::F32X4, MemFlags::trusted(), base, offset)
}

fn load_packed_scalar_component(
    builder: &mut FunctionBuilder<'_>,
    base: Value,
    component: usize,
) -> Value {
    let offset = i32::try_from(
        component
            .checked_mul(SIMD_JIT_LANES)
            .and_then(|offset| offset.checked_mul(types::F32.bytes() as usize))
            .unwrap(),
    )
    .unwrap();
    builder
        .ins()
        .load(types::F32, MemFlags::trusted(), base, offset)
}

fn store_simd_component(
    builder: &mut FunctionBuilder<'_>,
    base: Value,
    _elements: usize,
    component: usize,
    value: Value,
) {
    let offset = i32::try_from(
        component
            .checked_mul(SIMD_JIT_LANES)
            .and_then(|offset| offset.checked_mul(types::F32.bytes() as usize))
            .unwrap(),
    )
    .unwrap();
    builder
        .ins()
        .store(MemFlags::trusted(), value, base, offset);
}

fn lower_predicate(
    builder: &mut FunctionBuilder<'_>,
    predicate: &ScalarPredicate,
    registers: &[Option<NativeRegister>],
    constants: &BTreeMap<u32, Value>,
) -> Result<Value, BatchedExecutionError> {
    Ok(match predicate {
        ScalarPredicate::Value(operand) => {
            match lower_operand(builder, *operand, registers, constants)? {
                NativeRegister::Bool(value) => value,
                NativeRegister::F32(value) => {
                    let zero = constants[&0.0f32.to_bits()];
                    builder.ins().fcmp(FloatCC::NotEqual, value, zero)
                }
            }
        }
        ScalarPredicate::IsFinite(operand) => {
            let value = lower_numeric_operand(builder, *operand, registers, constants)?;
            lower_is_finite(builder, value)
        }
        ScalarPredicate::AbsoluteDifferenceWithin {
            left,
            right,
            tolerance,
        } => {
            let left = lower_numeric_operand(builder, *left, registers, constants)?;
            let right = lower_numeric_operand(builder, *right, registers, constants)?;
            let tolerance = lower_numeric_operand(builder, *tolerance, registers, constants)?;
            let difference = builder.ins().fsub(left, right);
            let difference = builder.ins().fabs(difference);
            builder
                .ins()
                .fcmp(FloatCC::LessThanOrEqual, difference, tolerance)
        }
        ScalarPredicate::Compare {
            operation,
            left,
            right,
        } => {
            let left = lower_numeric_operand(builder, *left, registers, constants)?;
            let right = lower_numeric_operand(builder, *right, registers, constants)?;
            builder.ins().fcmp(
                match operation {
                    ComparisonOperation::Equal => FloatCC::Equal,
                    ComparisonOperation::NotEqual => FloatCC::NotEqual,
                    ComparisonOperation::Less => FloatCC::LessThan,
                    ComparisonOperation::Greater => FloatCC::GreaterThan,
                    ComparisonOperation::LessEqual => FloatCC::LessThanOrEqual,
                    ComparisonOperation::GreaterEqual => FloatCC::GreaterThanOrEqual,
                },
                left,
                right,
            )
        }
        ScalarPredicate::All(inputs) => {
            let mut inputs = inputs.iter();
            let mut condition = lower_predicate(
                builder,
                inputs.next().expect("flattened conjunction is non-empty"),
                registers,
                constants,
            )?;
            for input in inputs {
                let next = lower_predicate(builder, input, registers, constants)?;
                condition = builder.ins().band(condition, next);
            }
            condition
        }
        ScalarPredicate::Logic { operation, inputs } => {
            let left = lower_predicate(builder, &inputs[0], registers, constants)?;
            if *operation == LogicOperation::Not {
                builder.ins().bnot(left)
            } else {
                let right = lower_predicate(builder, &inputs[1], registers, constants)?;
                match operation {
                    LogicOperation::And => builder.ins().band(left, right),
                    LogicOperation::Or => builder.ins().bor(left, right),
                    LogicOperation::Xor => builder.ins().bxor(left, right),
                    LogicOperation::Not => unreachable!(),
                }
            }
        }
    })
}

fn lower_is_finite(builder: &mut FunctionBuilder<'_>, value: Value) -> Value {
    let bits = builder.ins().bitcast(types::I32, MemFlags::new(), value);
    let exponent = builder.ins().band_imm(bits, 0x7f80_0000);
    builder
        .ins()
        .icmp_imm(IntCC::NotEqual, exponent, 0x7f80_0000)
}

fn lower_operand(
    _builder: &mut FunctionBuilder<'_>,
    operand: ScalarOperand,
    registers: &[Option<NativeRegister>],
    constants: &BTreeMap<u32, Value>,
) -> Result<NativeRegister, BatchedExecutionError> {
    match operand {
        ScalarOperand::Register(register) => registers[register].ok_or_else(|| {
            BatchedExecutionError::Native(format!(
                "native lowering read register {register} before definition"
            ))
        }),
        ScalarOperand::Constant(value) => Ok(NativeRegister::F32(
            constants.get(&value.to_bits()).copied().ok_or_else(|| {
                BatchedExecutionError::Native(format!(
                    "native lowering constant {value:?} was not hoisted"
                ))
            })?,
        )),
    }
}

fn lower_numeric_operand(
    builder: &mut FunctionBuilder<'_>,
    operand: ScalarOperand,
    registers: &[Option<NativeRegister>],
    constants: &BTreeMap<u32, Value>,
) -> Result<Value, BatchedExecutionError> {
    match lower_operand(builder, operand, registers, constants)? {
        NativeRegister::F32(value) => Ok(value),
        NativeRegister::Bool(_) => Err(BatchedExecutionError::Native(
            "native numeric operation received a boolean operand".to_owned(),
        )),
    }
}

fn lower_boolean_operand(
    builder: &mut FunctionBuilder<'_>,
    operand: ScalarOperand,
    registers: &[Option<NativeRegister>],
    constants: &BTreeMap<u32, Value>,
) -> Result<Value, BatchedExecutionError> {
    match lower_operand(builder, operand, registers, constants)? {
        NativeRegister::Bool(value) => Ok(value),
        NativeRegister::F32(_) => Err(BatchedExecutionError::Native(
            "native boolean operation received a numeric operand".to_owned(),
        )),
    }
}

fn call_math(
    builder: &mut FunctionBuilder<'_>,
    function: cranelift_codegen::ir::FuncRef,
    arguments: &[Value],
) -> Value {
    let call = builder.ins().call(function, arguments);
    builder.inst_results(call)[0]
}

fn call_sincos(
    builder: &mut FunctionBuilder<'_>,
    function: cranelift_codegen::ir::FuncRef,
    value: Value,
) -> (Value, Value) {
    let call = builder.ins().call(function, &[value]);
    let packed = builder.inst_results(call)[0];
    let sin_bits = builder.ins().ireduce(types::I32, packed);
    let cos_packed = builder.ins().ushr_imm(packed, 32);
    let cos_bits = builder.ins().ireduce(types::I32, cos_packed);
    let sin = builder.ins().bitcast(types::F32, MemFlags::new(), sin_bits);
    let cos = builder.ins().bitcast(types::F32, MemFlags::new(), cos_bits);
    (sin, cos)
}

fn call_simd_sincos(
    builder: &mut FunctionBuilder<'_>,
    function: cranelift_codegen::ir::FuncRef,
    value: Value,
    _pointer_type: Type,
) -> (Value, Value) {
    #[cfg(any(target_arch = "aarch64", target_arch = "x86_64"))]
    {
        let sin_slot = simd_stack_slot(builder);
        let cos_slot = simd_stack_slot(builder);
        let sin = stack_address(builder, sin_slot, _pointer_type);
        let cos = stack_address(builder, cos_slot, _pointer_type);
        builder.ins().call(function, &[value, sin, cos]);
        let sin = builder
            .ins()
            .load(types::F32X4, MemFlags::trusted(), sin, 0);
        let cos = builder
            .ins()
            .load(types::F32X4, MemFlags::trusted(), cos, 0);
        return (sin, cos);
    }

    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
    {
        let sin_slot = simd_stack_slot(builder);
        let cos_slot = simd_stack_slot(builder);
        let sin = stack_address(builder, sin_slot, _pointer_type);
        let cos = stack_address(builder, cos_slot, _pointer_type);
        let input_slot = simd_stack_slot(builder);
        let input = stack_address(builder, input_slot, _pointer_type);
        builder.ins().store(MemFlags::trusted(), value, input, 0);
        builder.ins().call(function, &[input, sin, cos]);
        let sin = builder
            .ins()
            .load(types::F32X4, MemFlags::trusted(), sin, 0);
        let cos = builder
            .ins()
            .load(types::F32X4, MemFlags::trusted(), cos, 0);
        (sin, cos)
    }
}

fn load_component(builder: &mut FunctionBuilder<'_>, base: Value, component: usize) -> Value {
    let offset =
        i32::try_from(component.checked_mul(types::F32.bytes() as usize).unwrap()).unwrap();
    builder
        .ins()
        .load(types::F32, MemFlags::trusted(), base, offset)
}

fn store_component(builder: &mut FunctionBuilder<'_>, base: Value, component: usize, value: Value) {
    let offset =
        i32::try_from(component.checked_mul(types::F32.bytes() as usize).unwrap()).unwrap();
    builder
        .ins()
        .store(MemFlags::trusted(), value, base, offset);
}

fn native_error(error: impl std::fmt::Display) -> BatchedExecutionError {
    BatchedExecutionError::Native(format!("Cranelift JIT: {error}"))
}

extern "C" fn mech_jit_sinf(value: f32) -> f32 {
    value.sin()
}

extern "C" fn mech_jit_cosf(value: f32) -> f32 {
    value.cos()
}

/// Return both transcendental results in one ABI call. The low and high
/// halves contain the raw f32 bit patterns for sin(value) and cos(value).
extern "C" fn mech_jit_sincos_pack(value: f32) -> u64 {
    let (sin, cos) = value.sin_cos();
    u64::from(sin.to_bits()) | (u64::from(cos.to_bits()) << 32)
}

/// Vector math entry points used by the SIMD JIT. Native vector arguments
/// keep each helper on the SIMD register path on the two targets we support;
/// the pointer ABI remains the portable fallback for other architectures.
#[cfg(target_arch = "aarch64")]
#[allow(improper_ctypes_definitions)]
extern "C" fn mech_jit_sinf_f32x4(value: float32x4_t) -> float32x4_t {
    unsafe { neon_sin_cos(value).0 }
}

#[cfg(target_arch = "x86_64")]
#[allow(improper_ctypes_definitions)]
extern "C" fn mech_jit_sinf_f32x4(value: __m128) -> __m128 {
    let value: f32x4 = unsafe { mem::transmute(value) };
    let (sin, _) = simd_sin_cos(value);
    unsafe { mem::transmute(sin) }
}

#[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
extern "C" fn mech_jit_sinf_f32x4(input: *const f32, output: *mut f32) {
    let value = unsafe { f32x4::new(*(input as *const [f32; 4])) };
    let (sin, _) = simd_sin_cos(value);
    unsafe { *(output as *mut [f32; 4]) = sin.to_array() };
}

#[cfg(target_arch = "aarch64")]
#[allow(improper_ctypes_definitions)]
extern "C" fn mech_jit_cosf_f32x4(value: float32x4_t) -> float32x4_t {
    unsafe { neon_sin_cos(value).1 }
}

#[cfg(target_arch = "x86_64")]
#[allow(improper_ctypes_definitions)]
extern "C" fn mech_jit_cosf_f32x4(value: __m128) -> __m128 {
    let value: f32x4 = unsafe { mem::transmute(value) };
    let (_, cos) = simd_sin_cos(value);
    unsafe { mem::transmute(cos) }
}

#[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
extern "C" fn mech_jit_cosf_f32x4(input: *const f32, output: *mut f32) {
    let value = unsafe { f32x4::new(*(input as *const [f32; 4])) };
    let (_, cos) = simd_sin_cos(value);
    unsafe { *(output as *mut [f32; 4]) = cos.to_array() };
}

#[cfg(target_arch = "aarch64")]
#[allow(improper_ctypes_definitions)]
extern "C" fn mech_jit_sincos_f32x4(
    value: float32x4_t,
    sin_output: *mut f32,
    cos_output: *mut f32,
) {
    let (sin, cos) = unsafe { neon_sin_cos(value) };
    unsafe {
        vst1q_f32(sin_output, sin);
        vst1q_f32(cos_output, cos);
    }
}

#[cfg(target_arch = "x86_64")]
#[allow(improper_ctypes_definitions)]
extern "C" fn mech_jit_sincos_f32x4(value: __m128, sin_output: *mut f32, cos_output: *mut f32) {
    let value: f32x4 = unsafe { mem::transmute(value) };
    let (sin, cos) = simd_sin_cos(value);
    unsafe {
        *(sin_output as *mut [f32; 4]) = sin.to_array();
        *(cos_output as *mut [f32; 4]) = cos.to_array();
    }
}

#[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
extern "C" fn mech_jit_sincos_f32x4(input: *const f32, sin_output: *mut f32, cos_output: *mut f32) {
    let value = unsafe { f32x4::new(*(input as *const [f32; 4])) };
    let (sin, cos) = value.sin_cos();
    unsafe {
        *(sin_output as *mut [f32; 4]) = sin.to_array();
        *(cos_output as *mut [f32; 4]) = cos.to_array();
    }
}

#[cfg(target_arch = "aarch64")]
#[allow(improper_ctypes_definitions)]
extern "C" fn mech_jit_atan2_f32x4(y: float32x4_t, x: float32x4_t) -> float32x4_t {
    unsafe { neon_atan2(y, x) }
}

#[cfg(target_arch = "x86_64")]
#[allow(improper_ctypes_definitions)]
extern "C" fn mech_jit_atan2_f32x4(y: __m128, x: __m128) -> __m128 {
    let y: f32x4 = unsafe { mem::transmute(y) };
    let x: f32x4 = unsafe { mem::transmute(x) };
    unsafe { mem::transmute(simd_atan2(y, x)) }
}

#[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
extern "C" fn mech_jit_atan2_f32x4(y_input: *const f32, x_input: *const f32, output: *mut f32) {
    let y = unsafe { f32x4::new(*(y_input as *const [f32; 4])) };
    let x = unsafe { f32x4::new(*(x_input as *const [f32; 4])) };
    unsafe { *(output as *mut [f32; 4]) = simd_atan2(y, x).to_array() };
}

#[inline]
#[cfg(not(target_arch = "aarch64"))]
fn bitcast_f32x4_to_i32x4(value: f32x4) -> i32x4 {
    // Both wide vector types are four 32-bit lanes with the same C layout.
    // This is a representation cast, not a numeric conversion.
    unsafe { mem::transmute(value) }
}

#[inline]
#[cfg(not(target_arch = "aarch64"))]
fn bitcast_i32x4_to_f32x4(value: i32x4) -> f32x4 {
    // Both wide vector types are four 32-bit lanes with the same C layout.
    // This is a representation cast, not a numeric conversion.
    unsafe { mem::transmute(value) }
}

/// Vector sine/cosine matching `wide::f32x4::sin_cos`, evaluated directly in
/// native NEON registers instead of converting through a temporary wide value.
/// Keeping this algorithm identical preserves the existing SIMD numeric
/// contract while avoiding the conversion and temporary-array overhead.
#[cfg(target_arch = "aarch64")]
#[inline(always)]
#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn neon_sin_cos(value: float32x4_t) -> (float32x4_t, float32x4_t) {
    let dp1 = vdupq_n_f32(0.78515625_f32 * 2.0);
    let dp2 = vdupq_n_f32(2.4187564849853515625E-4_f32 * 2.0);
    let dp3 = vdupq_n_f32(3.77489497744594108E-8_f32 * 2.0);
    let p0sin = vdupq_n_f32(-1.6666654611E-1_f32);
    let p1sin = vdupq_n_f32(8.3321608736E-3_f32);
    let p2sin = vdupq_n_f32(-1.9515295891E-4_f32);
    let p0cos = vdupq_n_f32(4.166664568298827E-2_f32);
    let p1cos = vdupq_n_f32(-1.388731625493765E-3_f32);
    let p2cos = vdupq_n_f32(2.443315711809948E-5_f32);
    let two_over_pi = vdupq_n_f32((2.0 / core::f32::consts::PI) as f32);
    let half = vdupq_n_f32(0.5);
    let one = vdupq_n_f32(1.0);
    let zero = vdupq_n_f32(0.0);

    let xa = vabsq_f32(value);
    let y = vrndnq_f32(vmulq_f32(xa, two_over_pi));
    let q = vcvtnq_s32_f32(y);
    let x = vsubq_f32(
        vsubq_f32(vsubq_f32(xa, vmulq_f32(y, dp1)), vmulq_f32(y, dp2)),
        vmulq_f32(y, dp3),
    );
    let x2 = vmulq_f32(x, x);
    let x4 = vmulq_f32(x2, x2);
    let s_poly = vaddq_f32(vmulq_f32(x4, p2sin), vaddq_f32(vmulq_f32(x2, p1sin), p0sin));
    let s = vaddq_f32(vmulq_f32(s_poly, vmulq_f32(x, x2)), x);
    let c_poly = vaddq_f32(vmulq_f32(x4, p2cos), vaddq_f32(vmulq_f32(x2, p1cos), p0cos));
    let c = vaddq_f32(vmulq_f32(c_poly, x4), vsubq_f32(one, vmulq_f32(half, x2)));

    let one_i = vdupq_n_s32(1);
    let zero_i = vdupq_n_s32(0);
    let swap = vmvnq_u32(vceqq_s32(vandq_s32(q, one_i), zero_i));
    let finite = vandq_u32(vceqq_f32(xa, xa), vcleq_f32(xa, vdupq_n_f32(f32::MAX)));
    let overflow = vandq_u32(vcgtq_s32(q, vdupq_n_s32(0x2000000)), finite);
    let s = vbslq_f32(overflow, zero, s);
    let c = vbslq_f32(overflow, one, c);
    let mut sin = vbslq_f32(swap, c, s);
    let sign_sin = veorq_s32(vshlq_n_s32(q, 30), vreinterpretq_s32_f32(value));
    let sign_mask = vandq_s32(sign_sin, vreinterpretq_s32_f32(vdupq_n_f32(-0.0)));
    sin = vreinterpretq_f32_s32(veorq_s32(vreinterpretq_s32_f32(sin), sign_mask));
    let mut cos = vbslq_f32(swap, s, c);
    let sign_cos = vshlq_n_s32(vandq_s32(vaddq_s32(q, one_i), vdupq_n_s32(2)), 30);
    cos = vreinterpretq_f32_s32(veorq_s32(vreinterpretq_s32_f32(cos), sign_cos));
    (sin, cos)
}

#[cfg(target_arch = "aarch64")]
#[inline(always)]
#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn neon_atan2(y: float32x4_t, x: float32x4_t) -> float32x4_t {
    let p3 = vdupq_n_f32(8.05374449538E-2_f32);
    let p2 = vdupq_n_f32(-1.38776856032E-1_f32);
    let p1 = vdupq_n_f32(1.99777106478E-1_f32);
    let p0 = vdupq_n_f32(-3.33329491539E-1_f32);
    let one = vdupq_n_f32(1.0);
    let zero = vdupq_n_f32(0.0);
    let frac_pi_2 = vdupq_n_f32(core::f32::consts::FRAC_PI_2);
    let frac_pi_4 = vdupq_n_f32(core::f32::consts::FRAC_PI_4);
    let pi = vdupq_n_f32(core::f32::consts::PI);
    let sqrt_2 = vdupq_n_f32(core::f32::consts::SQRT_2);

    let x1 = vabsq_f32(x);
    let y1 = vabsq_f32(y);
    let swap = vcgtq_f32(y1, x1);
    let mut x2 = vbslq_f32(swap, y1, x1);
    let mut y2 = vbslq_f32(swap, x1, y1);
    let infinity = vdupq_n_f32(f32::INFINITY);
    let both_infinite = vandq_u32(vceqq_f32(x1, infinity), vceqq_f32(y1, infinity));
    // Keep the wide implementation's special handling for +/-infinity. The
    // sign of either infinite input is irrelevant to this magnitude step.
    let magnitude_mask = vdupq_n_s32(0x7fff_ffff);
    x2 = vbslq_f32(
        both_infinite,
        vreinterpretq_f32_s32(vandq_s32(vreinterpretq_s32_f32(x2), magnitude_mask)),
        x2,
    );
    y2 = vbslq_f32(
        both_infinite,
        vreinterpretq_f32_s32(vandq_s32(vreinterpretq_s32_f32(y2), magnitude_mask)),
        y2,
    );
    let t = vdivq_f32(y2, x2);
    let not_small = vcgeq_f32(t, vsubq_f32(sqrt_2, one));
    let a = vbslq_f32(not_small, vsubq_f32(t, one), t);
    let b = vbslq_f32(not_small, vaddq_f32(t, one), one);
    let s = vandq_u32(not_small, vreinterpretq_u32_f32(frac_pi_4));
    let z = vdivq_f32(a, b);
    let zz = vmulq_f32(z, z);
    let zz2 = vmulq_f32(zz, zz);
    let re_poly = vaddq_f32(vmulq_f32(zz, p3), p2);
    let re_poly = vaddq_f32(vmulq_f32(re_poly, zz2), vaddq_f32(vmulq_f32(zz, p1), p0));
    let re = vaddq_f32(
        vaddq_f32(vmulq_f32(re_poly, vmulq_f32(zz, z)), z),
        vreinterpretq_f32_u32(s),
    );
    let re = vbslq_f32(swap, vsubq_f32(frac_pi_2, re), re);
    let xy = vreinterpretq_f32_u32(vorrq_u32(
        vreinterpretq_u32_f32(x),
        vreinterpretq_u32_f32(y),
    ));
    let zero_xy = vceqq_f32(xy, zero);
    let re = vbslq_f32(zero_xy, zero, re);
    let sign_mask = vdupq_n_u32(0x8000_0000);
    let x_sign = vceqq_u32(vandq_u32(vreinterpretq_u32_f32(x), sign_mask), sign_mask);
    let re = vbslq_f32(x_sign, vsubq_f32(pi, re), re);
    let y_sign = vceqq_u32(vandq_u32(vreinterpretq_u32_f32(y), sign_mask), sign_mask);
    vbslq_f32(y_sign, vnegq_f32(re), re)
}

#[inline]
#[cfg(not(target_arch = "aarch64"))]
fn simd_sin_cos(value: f32x4) -> (f32x4, f32x4) {
    const DP1F: f32x4 = f32x4::new([0.78515625_f32 * 2.0; 4]);
    const DP2F: f32x4 = f32x4::new([2.4187564849853515625E-4_f32 * 2.0; 4]);
    const DP3F: f32x4 = f32x4::new([3.77489497744594108E-8_f32 * 2.0; 4]);
    const P0SINF: f32x4 = f32x4::new([-1.6666654611E-1_f32; 4]);
    const P1SINF: f32x4 = f32x4::new([8.3321608736E-3_f32; 4]);
    const P2SINF: f32x4 = f32x4::new([-1.9515295891E-4_f32; 4]);
    const P0COSF: f32x4 = f32x4::new([4.166664568298827E-2_f32; 4]);
    const P1COSF: f32x4 = f32x4::new([-1.388731625493765E-3_f32; 4]);
    const P2COSF: f32x4 = f32x4::new([2.443315711809948E-5_f32; 4]);
    const TWO_OVER_PI: f32x4 = f32x4::new([(2.0 / core::f32::consts::PI) as f32; 4]);
    const HALF: f32x4 = f32x4::new([0.5; 4]);
    const ONE: f32x4 = f32x4::new([1.0; 4]);
    const ZERO: f32x4 = f32x4::new([0.0; 4]);

    let dp1f = DP1F;
    let dp2f = DP2F;
    let dp3f = DP3F;
    let p0sinf = P0SINF;
    let p1sinf = P1SINF;
    let p2sinf = P2SINF;
    let p0cosf = P0COSF;
    let p1cosf = P1COSF;
    let p2cosf = P2COSF;
    let two_over_pi = TWO_OVER_PI;
    let half = HALF;
    let one = ONE;
    let zero = ZERO;

    let xa = value.abs();
    let y = (xa * two_over_pi).round();
    let q = y.round_int();
    let x = y.mul_neg_add(dp3f, y.mul_neg_add(dp2f, y.mul_neg_add(dp1f, xa)));
    let x2 = x * x;
    let x4 = x2 * x2;
    let s_poly = x4.mul_add(p2sinf, x2.mul_add(p1sinf, p0sinf));
    let s = s_poly * (x * x2) + x;
    let c_poly = x4.mul_add(p2cosf, x2.mul_add(p1cosf, p0cosf));
    let c = c_poly * x4 + half.mul_neg_add(x2, one);
    let swap = !(q & i32x4::from(1)).cmp_eq(i32x4::from(0));
    let mut overflow = bitcast_i32x4_to_f32x4(q.cmp_gt(i32x4::from(0x2000000)));
    overflow &= xa.is_finite();
    let s = overflow.blend(zero, s);
    let c = overflow.blend(one, c);
    let mut sin = bitcast_i32x4_to_f32x4(swap).blend(c, s);
    let sign_sin = (q << 30) ^ bitcast_f32x4_to_i32x4(value);
    sin = sin.flip_signs(bitcast_i32x4_to_f32x4(sign_sin));
    let mut cos = bitcast_i32x4_to_f32x4(swap).blend(s, c);
    let sign_cos = ((q + i32x4::from(1)) & i32x4::from(2)) << 30;
    cos ^= bitcast_i32x4_to_f32x4(sign_cos);
    (sin, cos)
}

#[inline]
#[cfg(not(target_arch = "aarch64"))]
fn simd_atan2(y: f32x4, x: f32x4) -> f32x4 {
    const P3ATANF: f32x4 = f32x4::new([8.05374449538E-2_f32; 4]);
    const P2ATANF: f32x4 = f32x4::new([-1.38776856032E-1_f32; 4]);
    const P1ATANF: f32x4 = f32x4::new([1.99777106478E-1_f32; 4]);
    const P0ATANF: f32x4 = f32x4::new([-3.33329491539E-1_f32; 4]);
    const ONE: f32x4 = f32x4::new([1.0; 4]);
    const ZERO: f32x4 = f32x4::new([0.0; 4]);
    const FRAC_PI_2: f32x4 = f32x4::new([core::f32::consts::FRAC_PI_2; 4]);
    const FRAC_PI_4: f32x4 = f32x4::new([core::f32::consts::FRAC_PI_4; 4]);
    const PI: f32x4 = f32x4::new([core::f32::consts::PI; 4]);
    const SQRT_2: f32x4 = f32x4::new([core::f32::consts::SQRT_2; 4]);

    let p3atanf = P3ATANF;
    let p2atanf = P2ATANF;
    let p1atanf = P1ATANF;
    let p0atanf = P0ATANF;
    let one = ONE;
    let zero = ZERO;
    let frac_pi_2 = FRAC_PI_2;
    let frac_pi_4 = FRAC_PI_4;
    let pi = PI;
    let sqrt_2 = SQRT_2;

    let x1 = x.abs();
    let y1 = y.abs();
    let swapxy = y1.cmp_gt(x1);
    let mut x2 = swapxy.blend(y1, x1);
    let mut y2 = swapxy.blend(x1, y1);
    let both_infinite = x.is_inf() & y.is_inf();
    if both_infinite.any() {
        let minus_one = -one;
        x2 = both_infinite.blend(x2 & minus_one, x2);
        y2 = both_infinite.blend(y2 & minus_one, y2);
    }
    let t = y2 / x2;
    let not_small = t.cmp_ge(sqrt_2 - one);
    let a = not_small.blend(t - one, t);
    let b = not_small.blend(t + one, one);
    let s = not_small & frac_pi_4;
    let z = a / b;
    let zz = z * z;
    let zz2 = zz * zz;
    let re = p3atanf
        .mul_add(zz, p2atanf)
        .mul_add(zz2, p1atanf.mul_add(zz, p0atanf));
    let re = re.mul_add(zz * z, z) + s;
    let re = swapxy.blend(frac_pi_2 - re, re);
    let re = ((x | y).cmp_eq(zero)).blend(zero, re);
    let re = (x.sign_bit()).blend(pi - re, re);
    (y.sign_bit()).blend(-re, re)
}

extern "C" fn mech_jit_sqrtf(value: f32) -> f32 {
    value.sqrt()
}

extern "C" fn mech_jit_ceilf(value: f32) -> f32 {
    value.ceil()
}

extern "C" fn mech_jit_atan2f(y: f32, x: f32) -> f32 {
    y.atan2(x)
}
