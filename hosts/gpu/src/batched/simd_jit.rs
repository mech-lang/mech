//! Four-lane Cranelift JIT and persistent parallel workers.
//!
//! Ported from benchmark revision eedc1c75b5a780a92d3f50f094be873f93bca6b9.
//! Every dispatch validates and publishes each turn independently. Unchecked
//! execution uses a kernel whose integrity constraints were explicitly removed.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::mpsc::{self, Receiver, Sender};
use std::{mem, sync::Arc, thread};

#[cfg(target_arch = "aarch64")]
use core::arch::aarch64::{float32x4_t, vst1q_f32};
#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::{__m128, _mm_storeu_ps};

use cranelift_codegen::ir::{
    AbiParam, InstBuilder, MemFlags, StackSlotData, StackSlotKind, Type, UserFuncName, Value,
    condcodes::{FloatCC, IntCC},
    types,
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, default_libcall_names};
use mech_core::CellSlotId;
use wide::f32x4;

use super::{
    BatchedExecutionError, BatchedFaultRecorder, BatchedIntegrityFault, BinaryOperation,
    ComparisonOperation, ElementwiseOperation, FixedShapeKernel, LogicOperation, ScalarComputation,
    ScalarInstruction, ScalarOperand, ScalarPredicate, UnaryOperation,
};

const SIMD_JIT_LANES: usize = 4;
type NativeSimdTurn = unsafe extern "C" fn(
    input_pointers: *const *const f32,
    state_pointers: *const *mut f32,
    next_state_pointers: *const *mut f32,
    start_group: usize,
    end_group: usize,
) -> u64;

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
    fast_math: bool,
    inputs: BTreeMap<CellSlotId, Vec<f32>>,
    packed_inputs: BTreeMap<CellSlotId, Vec<f32>>,
    input_broadcast: Vec<bool>,
    state: BTreeMap<CellSlotId, Vec<f32>>,
    packed_state: BTreeMap<CellSlotId, Vec<f32>>,
    packed_next_state: BTreeMap<CellSlotId, Vec<f32>>,
    logical_state_dirty: bool,
    input_pointers: Vec<*const f32>,
    state_pointers: Vec<*mut f32>,
    next_state_pointers: Vec<*mut f32>,
    faults: BatchedFaultRecorder,
}

impl FixedShapeKernel {
    /// Prepare a four-lane SIMD JIT session. The batch extent must be a
    /// multiple of four. Declared integrity constraints remain enabled.
    pub fn prepare_jit_simd_cpu(
        &self,
        provided_inputs: &BTreeMap<String, Vec<f32>>,
    ) -> Result<BatchedJitSimdCpuSession, BatchedExecutionError> {
        let checked = !self.constraints.is_empty();
        let fast_math = false;
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
        let kernel = NativeSimdKernel::compile(self, checked, &input_broadcast, fast_math)?;
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
            fast_math,
            inputs,
            packed_inputs,
            input_broadcast,
            state,
            packed_state,
            packed_next_state,
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

impl BatchedJitSimdCpuSession {
    /// Validate, pack, and (if needed) compile the whole update packet before
    /// replacing resident input storage. An error leaves all inputs unchanged.
    pub fn update_inputs(
        &mut self,
        updates: &BTreeMap<String, Vec<f32>>,
    ) -> Result<(), BatchedExecutionError> {
        let mut staged = Vec::with_capacity(updates.len());
        let mut input_broadcast = self.input_broadcast.clone();
        for (name, values) in updates {
            let (index, input) = self
                .program
                .inputs
                .iter()
                .enumerate()
                .find(|(_, input)| input.name == *name)
                .ok_or_else(|| BatchedExecutionError::MissingInput(name.clone()))?;
            let expanded = self.program.expand_input(input, values)?;
            let packed = pack_simd_instances(&expanded, input.shape.elements());
            input_broadcast[index] = values.len() == input.shape.elements();
            staged.push((input.slot, expanded, packed));
        }
        let replacement = if input_broadcast != self.input_broadcast {
            Some(NativeSimdKernel::compile(
                &self.program,
                self.checked,
                &input_broadcast,
                self.fast_math,
            )?)
        } else {
            None
        };
        // Join all workers before any backing allocation or JIT module changes.
        self.parallel_pool.take();
        for (slot, expanded, packed) in staged {
            self.inputs.insert(slot, expanded);
            self.packed_inputs.insert(slot, packed);
        }
        self.input_broadcast = input_broadcast;
        if let Some(kernel) = replacement {
            self.kernel = kernel;
        }
        self.input_pointers = self
            .program
            .inputs
            .iter()
            .map(|input| self.packed_inputs[&input.slot].as_ptr())
            .collect();
        Ok(())
    }

    /// Start the reusable worker pool without advancing the state. Benchmark
    /// harnesses can keep worker creation outside the measured turn loop.
    pub fn prepare_parallel(&mut self, workers: usize) -> Result<(), BatchedExecutionError> {
        if workers == 0 {
            return Err(BatchedExecutionError::Native(
                "parallel SIMD JIT requires at least one worker".to_owned(),
            ));
        }
        let groups = self.program.instances as usize / SIMD_JIT_LANES;
        if groups == 0 {
            return Err(BatchedExecutionError::Native(
                "parallel SIMD JIT requires at least one instance group".to_owned(),
            ));
        }
        if workers == 1 {
            self.parallel_pool.take();
            return Ok(());
        }
        self.ensure_parallel_pool(groups, workers.min(groups))
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
        if workers == 0 {
            return Err(BatchedExecutionError::Native(
                "parallel SIMD JIT requires at least one worker".to_owned(),
            ));
        }
        if workers == 1 {
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
            let worker_result = match self
                .parallel_pool
                .as_ref()
                .expect("parallel worker pool initialized")
                .run(1)
            {
                Ok(result) => result,
                Err(error) => {
                    // A disconnected worker can leave peers running. Join them
                    // before callers may inspect, update, or retry the session.
                    self.parallel_pool.take();
                    return Err(BatchedExecutionError::Native(error));
                }
            };
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

impl NativeSimdKernel {
    fn compile(
        program: &FixedShapeKernel,
        checked: bool,
        input_broadcast: &[bool],
        fast_math: bool,
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
            signature.params.push(AbiParam::new(types::F32X4));
            signature.params.push(AbiParam::new(pointer_type));
            #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
            signature.params.push(AbiParam::new(pointer_type));
            signature
        };
        let simd_binary_signature = {
            let mut signature = module.make_signature();
            #[cfg(any(target_arch = "aarch64", target_arch = "x86_64"))]
            {
                signature.params.push(AbiParam::new(types::F32X4));
                signature.params.push(AbiParam::new(types::F32X4));
            }
            #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
            signature.params.push(AbiParam::new(pointer_type));
            #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
            signature.params.push(AbiParam::new(pointer_type));
            signature.params.push(AbiParam::new(pointer_type));
            signature
        };
        let simd_sincos_signature = {
            let mut signature = module.make_signature();
            #[cfg(any(target_arch = "aarch64", target_arch = "x86_64"))]
            signature.params.push(AbiParam::new(types::F32X4));
            #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
            signature.params.push(AbiParam::new(pointer_type));
            signature.params.push(AbiParam::new(pointer_type));
            signature.params.push(AbiParam::new(pointer_type));
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
            for (instruction_index, instruction) in instructions.iter().enumerate() {
                if paired_outputs.remove(&instruction.output) {
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
                    paired_outputs.insert(instructions[partner_index].output);
                    continue;
                }
                let value = lower_simd_computation(
                    &mut builder,
                    &instruction.computation,
                    &registers,
                    simd_functions,
                    &constant_values,
                    fast_math,
                    pointer_type,
                )?;
                registers[instruction.output] = Some(value);
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
        // SAFETY: `code` is the finalized entry point for the three-pointer,
        // two-range-bound ABI above. The module remains alive in this kernel.
        let turn = unsafe { mem::transmute::<*const u8, NativeSimdTurn>(code) };
        Ok(Self {
            _module: module,
            turn,
        })
    }
}

#[derive(Clone, Copy)]
struct SimdMathFunctions {
    sin: cranelift_codegen::ir::FuncRef,
    cos: cranelift_codegen::ir::FuncRef,
    sincos: cranelift_codegen::ir::FuncRef,
    atan2: cranelift_codegen::ir::FuncRef,
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

fn lower_simd_computation(
    builder: &mut FunctionBuilder<'_>,
    computation: &ScalarComputation,
    registers: &[Option<NativeSimdRegister>],
    functions: SimdMathFunctions,
    constants: &BTreeMap<u32, Value>,
    fast_math: bool,
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
            return lower_simd_sum_products(builder, terms, registers, constants, fast_math);
        }
    })
}

fn lower_simd_sum_products(
    builder: &mut FunctionBuilder<'_>,
    terms: &[(ScalarOperand, ScalarOperand)],
    registers: &[Option<NativeSimdRegister>],
    constants: &BTreeMap<u32, Value>,
    skip_zero_terms: bool,
) -> Result<NativeSimdRegister, BatchedExecutionError> {
    let mut sum = None;
    for (left, right) in terms {
        if skip_zero_terms && (is_zero_operand(*left) || is_zero_operand(*right)) {
            continue;
        }
        let value = match sum {
            None => {
                if is_one_operand(*left) {
                    lower_simd_numeric_operand(*right, registers, constants)?
                } else if is_one_operand(*right) {
                    lower_simd_numeric_operand(*left, registers, constants)?
                } else {
                    let left = lower_simd_numeric_operand(*left, registers, constants)?;
                    let right = lower_simd_numeric_operand(*right, registers, constants)?;
                    builder.ins().fmul(left, right)
                }
            }
            Some(sum) if is_one_operand(*left) => {
                let right = lower_simd_numeric_operand(*right, registers, constants)?;
                builder.ins().fadd(sum, right)
            }
            Some(sum) if is_one_operand(*right) => {
                let left = lower_simd_numeric_operand(*left, registers, constants)?;
                builder.ins().fadd(sum, left)
            }
            Some(sum) => {
                let left = lower_simd_numeric_operand(*left, registers, constants)?;
                let right = lower_simd_numeric_operand(*right, registers, constants)?;
                builder.ins().fma(left, right, sum)
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
    pointer_type: Type,
) -> Value {
    let output_slot = simd_stack_slot(builder);
    let output = stack_address(builder, output_slot, pointer_type);
    #[cfg(any(target_arch = "aarch64", target_arch = "x86_64"))]
    {
        builder.ins().call(function, &[value, output]);
        builder
            .ins()
            .load(types::F32X4, MemFlags::trusted(), output, 0)
    }
    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
    let input_slot = simd_stack_slot(builder);
    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
    let input = stack_address(builder, input_slot, pointer_type);
    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
    builder.ins().store(MemFlags::trusted(), value, input, 0);
    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
    builder.ins().call(function, &[input, output]);
    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
    builder
        .ins()
        .load(types::F32X4, MemFlags::trusted(), output, 0)
}

fn call_simd_binary_math(
    builder: &mut FunctionBuilder<'_>,
    function: cranelift_codegen::ir::FuncRef,
    left: Value,
    right: Value,
    pointer_type: Type,
) -> Value {
    let output_slot = simd_stack_slot(builder);
    let output = stack_address(builder, output_slot, pointer_type);
    #[cfg(any(target_arch = "aarch64", target_arch = "x86_64"))]
    {
        builder.ins().call(function, &[left, right, output]);
        builder
            .ins()
            .load(types::F32X4, MemFlags::trusted(), output, 0)
    }
    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
    let left_slot = simd_stack_slot(builder);
    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
    let right_slot = simd_stack_slot(builder);
    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
    let left_address = stack_address(builder, left_slot, pointer_type);
    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
    let right_address = stack_address(builder, right_slot, pointer_type);
    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
    builder
        .ins()
        .store(MemFlags::trusted(), left, left_address, 0);
    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
    builder
        .ins()
        .store(MemFlags::trusted(), right, right_address, 0);
    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
    builder
        .ins()
        .call(function, &[left_address, right_address, output]);
    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
    builder
        .ins()
        .load(types::F32X4, MemFlags::trusted(), output, 0)
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

fn is_zero_operand(operand: ScalarOperand) -> bool {
    matches!(operand, ScalarOperand::Constant(value) if value == 0.0)
}

fn is_one_operand(operand: ScalarOperand) -> bool {
    matches!(operand, ScalarOperand::Constant(value) if value == 1.0)
}

fn call_simd_sincos(
    builder: &mut FunctionBuilder<'_>,
    function: cranelift_codegen::ir::FuncRef,
    value: Value,
    pointer_type: Type,
) -> (Value, Value) {
    let sin_slot = simd_stack_slot(builder);
    let cos_slot = simd_stack_slot(builder);
    let sin = stack_address(builder, sin_slot, pointer_type);
    let cos = stack_address(builder, cos_slot, pointer_type);
    #[cfg(any(target_arch = "aarch64", target_arch = "x86_64"))]
    {
        builder.ins().call(function, &[value, sin, cos]);
        let sin_value = builder
            .ins()
            .load(types::F32X4, MemFlags::trusted(), sin, 0);
        let cos_value = builder
            .ins()
            .load(types::F32X4, MemFlags::trusted(), cos, 0);
        (sin_value, cos_value)
    }
    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
    let input_slot = simd_stack_slot(builder);
    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
    let input = stack_address(builder, input_slot, pointer_type);
    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
    builder.ins().store(MemFlags::trusted(), value, input, 0);
    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
    builder.ins().call(function, &[input, sin, cos]);
    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
    let sin = builder
        .ins()
        .load(types::F32X4, MemFlags::trusted(), sin, 0);
    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
    let cos = builder
        .ins()
        .load(types::F32X4, MemFlags::trusted(), cos, 0);
    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
    (sin, cos)
}
fn native_error(error: impl std::fmt::Display) -> BatchedExecutionError {
    BatchedExecutionError::Native(format!("Cranelift JIT: {error}"))
}

/// Vector math entry points used by the SIMD JIT. Native vector arguments
/// keep each helper on the SIMD register path on the two targets we support;
/// the pointer ABI remains the portable fallback for other architectures.
#[cfg(target_arch = "aarch64")]
#[allow(improper_ctypes_definitions)]
extern "C" fn mech_jit_sinf_f32x4(value: float32x4_t, output: *mut f32) {
    let value = unsafe { wide_from_aarch64(value) };
    unsafe { *(output as *mut [f32; 4]) = value.sin().to_array() };
}

#[cfg(target_arch = "x86_64")]
#[allow(improper_ctypes_definitions)]
extern "C" fn mech_jit_sinf_f32x4(value: __m128, output: *mut f32) {
    let value = unsafe { wide_from_x86(value) };
    unsafe { *(output as *mut [f32; 4]) = value.sin().to_array() };
}

#[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
extern "C" fn mech_jit_sinf_f32x4(input: *const f32, output: *mut f32) {
    let value = unsafe { f32x4::new(*(input as *const [f32; 4])) };
    unsafe { *(output as *mut [f32; 4]) = value.sin().to_array() };
}

#[cfg(target_arch = "aarch64")]
#[allow(improper_ctypes_definitions)]
extern "C" fn mech_jit_cosf_f32x4(value: float32x4_t, output: *mut f32) {
    let value = unsafe { wide_from_aarch64(value) };
    unsafe { *(output as *mut [f32; 4]) = value.cos().to_array() };
}

#[cfg(target_arch = "x86_64")]
#[allow(improper_ctypes_definitions)]
extern "C" fn mech_jit_cosf_f32x4(value: __m128, output: *mut f32) {
    let value = unsafe { wide_from_x86(value) };
    unsafe { *(output as *mut [f32; 4]) = value.cos().to_array() };
}

#[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
extern "C" fn mech_jit_cosf_f32x4(input: *const f32, output: *mut f32) {
    let value = unsafe { f32x4::new(*(input as *const [f32; 4])) };
    unsafe { *(output as *mut [f32; 4]) = value.cos().to_array() };
}

#[cfg(target_arch = "aarch64")]
#[allow(improper_ctypes_definitions)]
extern "C" fn mech_jit_sincos_f32x4(
    value: float32x4_t,
    sin_output: *mut f32,
    cos_output: *mut f32,
) {
    let value = unsafe { wide_from_aarch64(value) };
    let (sin, cos) = value.sin_cos();
    unsafe {
        *(sin_output as *mut [f32; 4]) = sin.to_array();
        *(cos_output as *mut [f32; 4]) = cos.to_array();
    }
}

#[cfg(target_arch = "x86_64")]
#[allow(improper_ctypes_definitions)]
extern "C" fn mech_jit_sincos_f32x4(value: __m128, sin_output: *mut f32, cos_output: *mut f32) {
    let value = unsafe { wide_from_x86(value) };
    let (sin, cos) = value.sin_cos();
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
extern "C" fn mech_jit_atan2_f32x4(y: float32x4_t, x: float32x4_t, output: *mut f32) {
    let y = unsafe { wide_from_aarch64(y) };
    let x = unsafe { wide_from_aarch64(x) };
    unsafe { *(output as *mut [f32; 4]) = y.atan2(x).to_array() };
}

#[cfg(target_arch = "x86_64")]
#[allow(improper_ctypes_definitions)]
extern "C" fn mech_jit_atan2_f32x4(y: __m128, x: __m128, output: *mut f32) {
    let y = unsafe { wide_from_x86(y) };
    let x = unsafe { wide_from_x86(x) };
    unsafe { *(output as *mut [f32; 4]) = y.atan2(x).to_array() };
}

#[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
extern "C" fn mech_jit_atan2_f32x4(y_input: *const f32, x_input: *const f32, output: *mut f32) {
    let y = unsafe { f32x4::new(*(y_input as *const [f32; 4])) };
    let x = unsafe { f32x4::new(*(x_input as *const [f32; 4])) };
    unsafe { *(output as *mut [f32; 4]) = y.atan2(x).to_array() };
}

#[cfg(target_arch = "aarch64")]
unsafe fn wide_from_aarch64(value: float32x4_t) -> f32x4 {
    let mut lanes = [0.0; 4];
    unsafe { vst1q_f32(lanes.as_mut_ptr(), value) };
    f32x4::new(lanes)
}

#[cfg(target_arch = "x86_64")]
unsafe fn wide_from_x86(value: __m128) -> f32x4 {
    let mut lanes = [0.0; 4];
    unsafe { _mm_storeu_ps(lanes.as_mut_ptr(), value) };
    f32x4::new(lanes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kernel(checked: bool, instances: usize) -> (FixedShapeKernel, BTreeMap<String, Vec<f32>>) {
        let guard = if checked {
            "valid! := candidate < 100f32\n"
        } else {
            ""
        };
        let source = format!(
            "Input updates @compute\n\
             -------------------------------------------------------------------------------\n\
             a := 1f32\nz := 2f32\n~value := 0f32\n\
             candidate := value + a + z\n{guard}value = candidate\nvalue\n"
        );
        let tree = mech_syntax::parse(&source).unwrap();
        let artifact = mech_runtime::RuntimeBuilder::new()
            .function_catalog(mech_stdlib::source_native_plan_catalog())
            .build_compiler()
            .unwrap()
            .compile_tree_artifact_with_inputs(
                &tree,
                &BTreeMap::new(),
                &BTreeSet::from(["a".to_owned(), "z".to_owned()]),
            )
            .unwrap()
            .into_artifact();
        let inputs = BTreeMap::from([
            ("a".to_owned(), vec![1.0; instances]),
            ("z".to_owned(), vec![2.0; instances]),
        ]);
        let program = crate::ComputeLowerer
            .compile_broadcast(&artifact, &inputs)
            .unwrap();
        assert_eq!(
            program.integrity_constraints().count(),
            usize::from(checked)
        );
        (program, inputs)
    }

    #[test]
    fn simd_jit_single_and_parallel_match_scalar_in_both_modes() {
        for checked in [false, true] {
            let (program, inputs) = kernel(checked, 32);
            let mut reference = program.prepare_cpu(&inputs).unwrap();
            reference.dispatch_turns(3).unwrap();
            for workers in [1, 2, 8] {
                let mut session = program.prepare_jit_simd_cpu(&inputs).unwrap();
                session.prepare_parallel(workers).unwrap();
                assert_eq!(session.attempted_turns(), 0);
                session.dispatch_turns_parallel(3, workers).unwrap();
                assert_eq!(session.state(), reference.state());
                assert_eq!(session.attempted_turns(), 3);
                assert_eq!(session.fault_count(), 0);
            }
        }
    }

    #[test]
    fn simd_jit_rejected_parallel_turn_preserves_the_entire_batch_and_recovers() {
        let (program, inputs) = kernel(true, 32);
        let mut session = program.prepare_jit_simd_cpu(&inputs).unwrap();
        session.dispatch_turns_parallel(2, 8).unwrap();
        let before = session.state().clone();
        let mut bad = vec![2.0; 32];
        bad[20] = 100.0;
        session
            .update_inputs(&BTreeMap::from([("z".to_owned(), bad)]))
            .unwrap();
        let error = session.dispatch_turns_parallel(1, 8).unwrap_err();
        assert!(matches!(error, BatchedExecutionError::Integrity(_)));
        assert_eq!(session.state(), &before);
        assert_eq!(session.attempted_turns(), 3);
        assert_eq!(session.fault_count(), 1);
        assert_eq!(session.last_fault().unwrap().instance, 20);
        assert!(session.parallel_pool.is_none());
        session.update_inputs(&inputs).unwrap();
        session.dispatch_turns_parallel(1, 8).unwrap();
        let mut reference = program.prepare_cpu(&inputs).unwrap();
        reference.dispatch_turns(3).unwrap();
        assert_eq!(session.state(), reference.state());
    }

    #[test]
    fn simd_jit_input_packets_are_atomic_and_broadcast_changes_recompile() {
        let (program, inputs) = kernel(true, 32);
        let mut session = program.prepare_jit_simd_cpu(&inputs).unwrap();
        session.prepare_parallel(8).unwrap();
        let original_pointers = session.input_pointers.clone();
        for (name, values) in [("zzz", vec![3.0]), ("z", vec![3.0; 3])] {
            let updates = BTreeMap::from([("a".to_owned(), vec![9.0]), (name.to_owned(), values)]);
            assert!(session.update_inputs(&updates).is_err());
            assert_eq!(session.input_pointers, original_pointers);
            assert!(session.parallel_pool.is_some());
        }
        session.dispatch_turns_parallel(1, 8).unwrap();
        let mut reference = program.prepare_cpu(&inputs).unwrap();
        reference.dispatch_turns(1).unwrap();
        assert_eq!(session.state(), reference.state());
        let updates = BTreeMap::from([
            ("a".to_owned(), vec![7.0]),
            ("z".to_owned(), (0..32).map(|index| index as f32).collect()),
        ]);
        session.update_inputs(&updates).unwrap();
        reference.update_inputs(&updates).unwrap();
        session.dispatch_turns_parallel(1, 8).unwrap();
        reference.dispatch_turns(1).unwrap();
        assert_eq!(session.state(), reference.state());
        // Switching between parallel and single-worker dispatch preserves the
        // published orientation of the packed ping-pong buffers.
        session.dispatch_turns(1).unwrap();
        reference.dispatch_turns(1).unwrap();
        assert_eq!(session.state(), reference.state());
    }

    #[test]
    fn simd_jit_rejects_invalid_extents_and_dispatch_counts() {
        let (program, inputs) = kernel(false, 3);
        assert!(program.prepare_jit_simd_cpu(&inputs).is_err());
        let (program, inputs) = kernel(false, 4);
        let mut session = program.prepare_jit_simd_cpu(&inputs).unwrap();
        assert!(session.prepare_parallel(0).is_err());
        assert!(session.dispatch_turns_parallel(1, 0).is_err());
        assert!(matches!(
            session.dispatch_turns_parallel(0, 8),
            Err(BatchedExecutionError::ZeroTurns)
        ));
        assert_eq!(session.attempted_turns(), 0);
    }
}
