use std::collections::{BTreeMap, BTreeSet};
use std::{mem, sync::Arc};

use cranelift_codegen::ir::{
    AbiParam, InstBuilder, MemFlags, UserFuncName, Value,
    condcodes::{FloatCC, IntCC},
    types,
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, default_libcall_names};
use mech_core::CellSlotId;

use super::{
    BatchedExecutionError, BatchedFaultRecorder, BatchedIntegrityFault, BinaryOperation,
    ComparisonOperation, ElementwiseOperation, FixedShapeKernel, LogicOperation, ScalarComputation,
    ScalarOperand, ScalarPredicate, UnaryOperation,
};

type NativeTurn = unsafe extern "C" fn(
    input_pointers: *const *const f32,
    state_pointers: *const *const f32,
    next_state_pointers: *const *mut f32,
) -> u64;

const SIMD_JIT_LANES: usize = 4;
type NativeSimdTurn = unsafe extern "C" fn(
    input_pointers: *const *const f32,
    state_pointers: *const *const f32,
    next_state_pointers: *const *mut f32,
) -> u64;

struct NativeKernel {
    _module: JITModule,
    turn: NativeTurn,
}

struct NativeSimdKernel {
    _module: JITModule,
    turn: NativeSimdTurn,
}

pub struct BatchedJitCpuSession {
    program: Arc<FixedShapeKernel>,
    kernel: NativeKernel,
    checked: bool,
    fast_math: bool,
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
/// Cranelift vector body.  The math intrinsics remain scalar calls for now;
/// arithmetic, comparisons, loads, and stores are vectorized.  This keeps the
/// exact scalar math contract while providing a measured path for lane
/// batching before vector math is introduced.
pub struct BatchedJitSimdCpuSession {
    program: Arc<FixedShapeKernel>,
    kernel: NativeSimdKernel,
    checked: bool,
    fast_math: bool,
    inputs: BTreeMap<CellSlotId, Vec<f32>>,
    input_broadcast: Vec<bool>,
    state: BTreeMap<CellSlotId, Vec<f32>>,
    next_state: BTreeMap<CellSlotId, Vec<f32>>,
    input_pointers: Vec<*const f32>,
    state_pointers: Vec<*const f32>,
    next_state_pointers: Vec<*mut f32>,
    faults: BatchedFaultRecorder,
}

impl FixedShapeKernel {
    pub fn prepare_jit_cpu(
        &self,
        inputs: &BTreeMap<String, Vec<f32>>,
    ) -> Result<BatchedJitCpuSession, BatchedExecutionError> {
        self.prepare_jit_cpu_with_validation(inputs, true, false)
    }

    /// Prepares a JIT session without integrity predicates. Invalid candidate
    /// state is published, so callers must only use this mode when they own
    /// equivalent validation or explicitly accept the weaker guarantee.
    pub fn prepare_jit_cpu_unchecked(
        &self,
        inputs: &BTreeMap<String, Vec<f32>>,
    ) -> Result<BatchedJitCpuSession, BatchedExecutionError> {
        self.prepare_jit_cpu_with_validation(inputs, false, false)
    }

    /// Prepares a JIT session with integrity predicates disabled and algebraic
    /// zero-term elimination enabled. This mode does not preserve IEEE NaN
    /// propagation through zero products; callers must accept that weaker
    /// numeric guarantee in addition to unchecked publication.
    pub fn prepare_jit_cpu_unchecked_fast(
        &self,
        inputs: &BTreeMap<String, Vec<f32>>,
    ) -> Result<BatchedJitCpuSession, BatchedExecutionError> {
        self.prepare_jit_cpu_with_validation(inputs, false, true)
    }

    /// Prepares a checked JIT session with algebraic zero-term elimination.
    /// Rollback and all declared integrity predicates remain enabled, but
    /// IEEE NaN propagation through eliminated zero products is not preserved.
    pub fn prepare_jit_cpu_checked_fast(
        &self,
        inputs: &BTreeMap<String, Vec<f32>>,
    ) -> Result<BatchedJitCpuSession, BatchedExecutionError> {
        self.prepare_jit_cpu_with_validation(inputs, true, true)
    }

    /// Prepares the lane-vectorized JIT session.  The first implementation
    /// uses four-lane f32 vectors and therefore requires a batch extent that
    /// is divisible by four; callers can use the scalar JIT for a tail until
    /// masked vector tails are added.
    pub fn prepare_jit_simd_cpu(
        &self,
        inputs: &BTreeMap<String, Vec<f32>>,
    ) -> Result<BatchedJitSimdCpuSession, BatchedExecutionError> {
        self.prepare_jit_simd_cpu_with_validation(inputs, true, false)
    }

    /// Prepares an unchecked lane-vectorized JIT session.
    pub fn prepare_jit_simd_cpu_unchecked(
        &self,
        inputs: &BTreeMap<String, Vec<f32>>,
    ) -> Result<BatchedJitSimdCpuSession, BatchedExecutionError> {
        self.prepare_jit_simd_cpu_with_validation(inputs, false, false)
    }

    /// Prepares a checked lane-vectorized JIT session with algebraic
    /// zero-term elimination.  This has the same weakened NaN propagation as
    /// [`Self::prepare_jit_cpu_checked_fast`].
    pub fn prepare_jit_simd_cpu_checked_fast(
        &self,
        inputs: &BTreeMap<String, Vec<f32>>,
    ) -> Result<BatchedJitSimdCpuSession, BatchedExecutionError> {
        self.prepare_jit_simd_cpu_with_validation(inputs, true, true)
    }

    /// Prepares an unchecked lane-vectorized JIT session with algebraic
    /// zero-term elimination.
    pub fn prepare_jit_simd_cpu_unchecked_fast(
        &self,
        inputs: &BTreeMap<String, Vec<f32>>,
    ) -> Result<BatchedJitSimdCpuSession, BatchedExecutionError> {
        self.prepare_jit_simd_cpu_with_validation(inputs, false, true)
    }

    fn prepare_jit_cpu_with_validation(
        &self,
        provided_inputs: &BTreeMap<String, Vec<f32>>,
        checked: bool,
        fast_math: bool,
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
        let kernel = NativeKernel::compile(self, checked, &input_broadcast, fast_math)?;
        let input_pointers = self
            .inputs
            .iter()
            .map(|input| inputs[&input.slot].as_ptr())
            .collect();
        let mut session = BatchedJitCpuSession {
            program: Arc::new(self.clone()),
            kernel,
            checked,
            fast_math,
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
        fast_math: bool,
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
        let next_state = state
            .iter()
            .map(|(slot, values)| (*slot, vec![0.0; values.len()]))
            .collect();
        let kernel = NativeSimdKernel::compile(self, checked, &input_broadcast, fast_math)?;
        let input_pointers = self
            .inputs
            .iter()
            .map(|input| inputs[&input.slot].as_ptr())
            .collect();
        let mut session = BatchedJitSimdCpuSession {
            program: Arc::new(self.clone()),
            kernel,
            checked,
            fast_math,
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
            self.kernel = NativeKernel::compile(
                &self.program,
                self.checked,
                &self.input_broadcast,
                self.fast_math,
            )?;
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
            self.kernel = NativeSimdKernel::compile(
                &self.program,
                self.checked,
                &self.input_broadcast,
                self.fast_math,
            )?;
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
            // SAFETY: the generated function uses the exact ABI below. Its
            // vector loads gather four scalar instances at fixed strides, and
            // all buffers remain live for the duration of the call.
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

impl NativeKernel {
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
            .symbol("mech_jit_sinf", mech_jit_sinf as *const u8)
            .symbol("mech_jit_cosf", mech_jit_cosf as *const u8)
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
            let sqrt_ref = module.declare_func_in_func(sqrt_id, builder.func);
            let ceil_ref = module.declare_func_in_func(ceil_id, builder.func);
            let atan2_ref = module.declare_func_in_func(atan2_id, builder.func);
            let functions = MathFunctions {
                sin: sin_ref,
                cos: cos_ref,
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
            for instruction in &program.fixed_ir().instructions {
                let value = lower_computation(
                    &mut builder,
                    &instruction.computation,
                    &registers,
                    functions,
                    &constant_values,
                    fast_math,
                )?;
                registers[instruction.output] = Some(value);
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
        fast_math: bool,
    ) -> Result<Self, BatchedExecutionError> {
        let mut jit_builder =
            JITBuilder::with_flags(&[("opt_level", "speed")], default_libcall_names())
                .map_err(native_error)?;
        jit_builder
            .symbol("mech_jit_sinf", mech_jit_sinf as *const u8)
            .symbol("mech_jit_cosf", mech_jit_cosf as *const u8)
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
                                let scalar =
                                    load_component(&mut builder, input_bases[index], component);
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
            let zero = builder.ins().iconst(pointer_type, 0);
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
                    initial_loop_bases.push((input_bases[index], stride));
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
                initial_loop_bases.push((state_bases[index], stride));
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
            let group = header_params[0];
            let groups = i64::from(program.instances / SIMD_JIT_LANES as u32);
            let has_group = builder
                .ins()
                .icmp_imm(IntCC::UnsignedLessThan, group, groups);
            builder.ins().brif(has_group, body, &[], exit, &[]);

            builder.switch_to_block(body);
            let sin_ref = module.declare_func_in_func(sin_id, builder.func);
            let cos_ref = module.declare_func_in_func(cos_id, builder.func);
            let sqrt_ref = module.declare_func_in_func(sqrt_id, builder.func);
            let ceil_ref = module.declare_func_in_func(ceil_id, builder.func);
            let atan2_ref = module.declare_func_in_func(atan2_id, builder.func);
            let functions = MathFunctions {
                sin: sin_ref,
                cos: cos_ref,
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
            for instruction in &program.fixed_ir().instructions {
                let value = lower_simd_computation(
                    &mut builder,
                    &instruction.computation,
                    &registers,
                    functions,
                    &constant_values,
                    fast_math,
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
    sqrt: cranelift_codegen::ir::FuncRef,
    ceil: cranelift_codegen::ir::FuncRef,
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

fn lower_computation(
    builder: &mut FunctionBuilder<'_>,
    computation: &ScalarComputation,
    registers: &[Option<NativeRegister>],
    functions: MathFunctions,
    constants: &BTreeMap<u32, Value>,
    fast_math: bool,
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
            return lower_sum_products(builder, terms, registers, constants, fast_math);
        }
    })
}

fn lower_sum_products(
    builder: &mut FunctionBuilder<'_>,
    terms: &[(ScalarOperand, ScalarOperand)],
    registers: &[Option<NativeRegister>],
    constants: &BTreeMap<u32, Value>,
    skip_zero_terms: bool,
) -> Result<NativeRegister, BatchedExecutionError> {
    let mut sum = None;
    for (left, right) in terms {
        if skip_zero_terms && (is_zero_operand(*left) || is_zero_operand(*right)) {
            continue;
        }
        let value = match sum {
            None => {
                if skip_zero_terms && is_one_operand(*left) {
                    lower_numeric_operand(builder, *right, registers, constants)?
                } else if skip_zero_terms && is_one_operand(*right) {
                    lower_numeric_operand(builder, *left, registers, constants)?
                } else {
                    let left = lower_numeric_operand(builder, *left, registers, constants)?;
                    let right = lower_numeric_operand(builder, *right, registers, constants)?;
                    builder.ins().fmul(left, right)
                }
            }
            Some(sum) if skip_zero_terms && is_one_operand(*left) => {
                let right = lower_numeric_operand(builder, *right, registers, constants)?;
                builder.ins().fadd(sum, right)
            }
            Some(sum) if skip_zero_terms && is_one_operand(*right) => {
                let left = lower_numeric_operand(builder, *left, registers, constants)?;
                builder.ins().fadd(sum, left)
            }
            Some(sum) => {
                let left = lower_numeric_operand(builder, *left, registers, constants)?;
                let right = lower_numeric_operand(builder, *right, registers, constants)?;
                builder.ins().fma(left, right, sum)
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
    functions: MathFunctions,
    constants: &BTreeMap<u32, Value>,
    fast_math: bool,
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
                    UnaryOperation::Sin => call_simd_unary_math(builder, functions.sin, values[0]),
                    UnaryOperation::Cos => call_simd_unary_math(builder, functions.cos, values[0]),
                    UnaryOperation::Sqrt => builder.ins().sqrt(values[0]),
                    UnaryOperation::Ceil => builder.ins().ceil(values[0]),
                },
                ElementwiseOperation::Atan2 => {
                    call_simd_binary_math(builder, functions.atan2, values[0], values[1])
                }
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
                if skip_zero_terms && is_one_operand(*left) {
                    lower_simd_numeric_operand(*right, registers, constants)?
                } else if skip_zero_terms && is_one_operand(*right) {
                    lower_simd_numeric_operand(*left, registers, constants)?
                } else {
                    let left = lower_simd_numeric_operand(*left, registers, constants)?;
                    let right = lower_simd_numeric_operand(*right, registers, constants)?;
                    builder.ins().fmul(left, right)
                }
            }
            Some(sum) if skip_zero_terms && is_one_operand(*left) => {
                let right = lower_simd_numeric_operand(*right, registers, constants)?;
                builder.ins().fadd(sum, right)
            }
            Some(sum) if skip_zero_terms && is_one_operand(*right) => {
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

fn lower_simd_is_finite(builder: &mut FunctionBuilder<'_>, value: Value) -> Value {
    // Cranelift's scalar bitwise-immediate operations do not accept vector
    // values.  |x| <= f32::MAX is equivalent to the scalar finite check for
    // f32: NaN fails the ordered comparison and both infinities exceed the
    // bound.
    let absolute = builder.ins().fabs(value);
    let maximum = builder.ins().f32const(f32::MAX);
    let maximum = builder.ins().splat(types::F32X4, maximum);
    builder
        .ins()
        .fcmp(FloatCC::LessThanOrEqual, absolute, maximum)
}

fn call_simd_unary_math(
    builder: &mut FunctionBuilder<'_>,
    function: cranelift_codegen::ir::FuncRef,
    value: Value,
) -> Value {
    let zero = builder.ins().f32const(0.0);
    let mut result = builder.ins().splat(types::F32X4, zero);
    for lane in 0..SIMD_JIT_LANES {
        let scalar = builder.ins().extractlane(value, lane as u8);
        let scalar = call_math(builder, function, &[scalar]);
        result = builder.ins().insertlane(result, scalar, lane as u8);
    }
    result
}

fn call_simd_binary_math(
    builder: &mut FunctionBuilder<'_>,
    function: cranelift_codegen::ir::FuncRef,
    left: Value,
    right: Value,
) -> Value {
    let zero = builder.ins().f32const(0.0);
    let mut result = builder.ins().splat(types::F32X4, zero);
    for lane in 0..SIMD_JIT_LANES {
        let left = builder.ins().extractlane(left, lane as u8);
        let right = builder.ins().extractlane(right, lane as u8);
        let scalar = call_math(builder, function, &[left, right]);
        result = builder.ins().insertlane(result, scalar, lane as u8);
    }
    result
}

fn load_simd_component(
    builder: &mut FunctionBuilder<'_>,
    base: Value,
    elements: usize,
    component: usize,
) -> Value {
    let stride = i32::try_from(elements.checked_mul(types::F32.bytes() as usize).unwrap()).unwrap();
    let mut values = [builder.ins().f32const(0.0); SIMD_JIT_LANES];
    for (lane, value) in values.iter_mut().enumerate() {
        let lane_base = if lane == 0 {
            base
        } else {
            builder
                .ins()
                .iadd_imm(base, i64::from(stride) * lane as i64)
        };
        *value = load_component(builder, lane_base, component);
    }
    let mut vector = builder.ins().splat(types::F32X4, values[0]);
    for (lane, value) in values.into_iter().enumerate().skip(1) {
        vector = builder.ins().insertlane(vector, value, lane as u8);
    }
    vector
}

fn store_simd_component(
    builder: &mut FunctionBuilder<'_>,
    base: Value,
    elements: usize,
    component: usize,
    value: Value,
) {
    let stride = i64::try_from(elements.checked_mul(types::F32.bytes() as usize).unwrap()).unwrap();
    for lane in 0..SIMD_JIT_LANES {
        let lane_base = if lane == 0 {
            base
        } else {
            builder.ins().iadd_imm(base, stride * lane as i64)
        };
        let scalar = builder.ins().extractlane(value, lane as u8);
        store_component(builder, lane_base, component, scalar);
    }
}

fn is_zero_operand(operand: ScalarOperand) -> bool {
    matches!(operand, ScalarOperand::Constant(value) if value == 0.0)
}

fn is_one_operand(operand: ScalarOperand) -> bool {
    matches!(operand, ScalarOperand::Constant(value) if value == 1.0)
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

extern "C" fn mech_jit_sqrtf(value: f32) -> f32 {
    value.sqrt()
}

extern "C" fn mech_jit_ceilf(value: f32) -> f32 {
    value.ceil()
}

extern "C" fn mech_jit_atan2f(y: f32, x: f32) -> f32 {
    y.atan2(x)
}
