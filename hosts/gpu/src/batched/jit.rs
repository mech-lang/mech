use std::collections::BTreeMap;
use std::{mem, sync::Arc, thread};

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
    state_pointers: *const *mut f32,
    next_state_pointers: *const *mut f32,
    start: usize,
    end: usize,
) -> u64;

struct NativeKernel {
    _module: JITModule,
    turn: NativeTurn,
}

pub struct BatchedJitCpuSession {
    program: Arc<FixedShapeKernel>,
    kernel: NativeKernel,
    inputs: BTreeMap<CellSlotId, Vec<f32>>,
    state: BTreeMap<CellSlotId, Vec<f32>>,
    next_state: BTreeMap<CellSlotId, Vec<f32>>,
    state_snapshot: BTreeMap<CellSlotId, Vec<f32>>,
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
        let inputs = self.expand_inputs(inputs)?;
        let inputs = self
            .inputs
            .iter()
            .map(|input| {
                (
                    input.slot,
                    to_soa(
                        &inputs[&input.slot],
                        input.shape.elements(),
                        self.instances as usize,
                    ),
                )
            })
            .collect::<BTreeMap<_, _>>();
        let state_snapshot = self.initial_state();
        let state = state_snapshot
            .iter()
            .map(|(slot, values)| {
                let elements = self
                    .states
                    .iter()
                    .find(|state| state.slot == *slot)
                    .map(|state| state.shape.elements())
                    .expect("state slot has a fixed shape");
                (*slot, to_soa(values, elements, self.instances as usize))
            })
            .collect::<BTreeMap<_, _>>();
        let next_state = state
            .iter()
            .map(|(slot, values)| (*slot, vec![0.0; values.len()]))
            .collect();
        let kernel = NativeKernel::compile(self)?;
        let mut session = BatchedJitCpuSession {
            program: Arc::new(self.clone()),
            kernel,
            inputs,
            state,
            next_state,
            state_snapshot,
            input_pointers: Vec::new(),
            state_pointers: Vec::with_capacity(
                self.states.iter().map(|state| state.shape.elements()).sum(),
            ),
            next_state_pointers: Vec::with_capacity(
                self.states.iter().map(|state| state.shape.elements()).sum(),
            ),
            faults: BatchedFaultRecorder::default(),
        };
        session.refresh_input_pointers();
        session.refresh_state_pointers();
        Ok(session)
    }
}

impl BatchedJitCpuSession {
    pub fn update_inputs(
        &mut self,
        updates: &BTreeMap<String, Vec<f32>>,
    ) -> Result<(), BatchedExecutionError> {
        for (name, values) in updates {
            let input = self
                .program
                .inputs
                .iter()
                .find(|input| input.name == *name)
                .ok_or_else(|| BatchedExecutionError::MissingInput(name.clone()))?;
            self.inputs.insert(
                input.slot,
                to_soa(
                    &self.program.expand_input(input, values)?,
                    input.shape.elements(),
                    self.program.instances as usize,
                ),
            );
        }
        self.refresh_input_pointers();
        Ok(())
    }

    pub fn dispatch_turns(&mut self, turns: u32) -> Result<(), BatchedExecutionError> {
        if turns == 0 {
            return Err(BatchedExecutionError::ZeroTurns);
        }
        for _ in 0..turns {
            let attempted_turn = self.faults.next_turn();
            // SAFETY: The generated function uses the exact ABI below. The flat
            // pointer tables and their backing SoA f32 buffers remain live for
            // the call, and every generated access is bounded by the admitted
            // fixed shapes and the program's instance count.
            let packed_fault = unsafe {
                (self.kernel.turn)(
                    self.input_pointers.as_ptr(),
                    self.state_pointers.as_ptr(),
                    self.next_state_pointers.as_ptr(),
                    0,
                    self.program.instances as usize,
                )
            };
            if let Some(fault) = self
                .program
                .failed_packed_constraint(packed_fault, attempted_turn)
            {
                return Err(self.faults.record(fault));
            }
            mem::swap(&mut self.state, &mut self.next_state);
            // The backing vectors never move during a resident session. Swap
            // the already-materialized pointer tables along with the logical
            // ping-pong state instead of rebuilding them on every turn.
            mem::swap(&mut self.state_pointers, &mut self.next_state_pointers);
        }
        self.refresh_state_snapshot();
        Ok(())
    }

    /// Executes the same generated turn function over disjoint instance
    /// ranges. The kernel owns no shared scratch state, so workers can write
    /// directly into separate lanes of the component-major buffers. A fault
    /// still rejects the complete turn; only the lowest failing instance is
    /// reported to keep diagnostics deterministic.
    pub fn dispatch_turns_parallel(
        &mut self,
        turns: u32,
        workers: usize,
    ) -> Result<(), BatchedExecutionError> {
        if workers <= 1 {
            return self.dispatch_turns(turns);
        }
        if turns == 0 {
            return Err(BatchedExecutionError::ZeroTurns);
        }
        let instances = self.program.instances as usize;
        let workers = workers.min(instances);
        for _ in 0..turns {
            let attempted_turn = self.faults.next_turn();
            let turn = self.kernel.turn;
            let input_table = self.input_pointers.as_ptr() as usize;
            let state_table = self.state_pointers.as_ptr() as usize;
            let next_state_table = self.next_state_pointers.as_ptr() as usize;
            let mut packed_fault = 0;
            // The tables are read-only during this scope. Each worker receives
            // a disjoint [start, end) range and therefore writes disjoint
            // elements in every component buffer.
            thread::scope(|scope| {
                let mut handles = Vec::with_capacity(workers);
                for worker in 0..workers {
                    let start = instances * worker / workers;
                    let end = instances * (worker + 1) / workers;
                    handles.push(scope.spawn(move || unsafe {
                        turn(
                            input_table as *const *const f32,
                            state_table as *const *mut f32,
                            next_state_table as *const *mut f32,
                            start,
                            end,
                        )
                    }));
                }
                for handle in handles {
                    let fault = handle.join().expect("generated JIT worker must not panic");
                    if fault != 0 && (packed_fault == 0 || (fault >> 8) < (packed_fault >> 8)) {
                        packed_fault = fault;
                    }
                }
            });
            if let Some(fault) = self
                .program
                .failed_packed_constraint(packed_fault, attempted_turn)
            {
                return Err(self.faults.record(fault));
            }
            mem::swap(&mut self.state, &mut self.next_state);
            mem::swap(&mut self.state_pointers, &mut self.next_state_pointers);
        }
        self.refresh_state_snapshot();
        Ok(())
    }

    pub fn state(&self) -> &BTreeMap<CellSlotId, Vec<f32>> {
        &self.state_snapshot
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

    fn refresh_input_pointers(&mut self) {
        self.input_pointers.clear();
        for input in &self.program.inputs {
            let values = &self.inputs[&input.slot];
            for component in 0..input.shape.elements() {
                self.input_pointers
                    .push(values[component * self.program.instances as usize..].as_ptr());
            }
        }
    }

    fn refresh_state_pointers(&mut self) {
        self.state_pointers.clear();
        self.next_state_pointers.clear();
        for state in &self.program.states {
            let values = &self.state[&state.slot];
            let next_values = self.next_state.get_mut(&state.slot).unwrap();
            for component in 0..state.shape.elements() {
                self.state_pointers.push(
                    values[component * self.program.instances as usize..].as_ptr() as *mut f32,
                );
                self.next_state_pointers
                    .push(next_values[component * self.program.instances as usize..].as_mut_ptr());
            }
        }
    }

    fn refresh_state_snapshot(&mut self) {
        for state in &self.program.states {
            let values = &self.state[&state.slot];
            self.state_snapshot.insert(
                state.slot,
                from_soa(
                    values,
                    state.shape.elements(),
                    self.program.instances as usize,
                ),
            );
        }
    }
}

fn to_soa(values: &[f32], elements: usize, instances: usize) -> Vec<f32> {
    let mut result = vec![0.0; values.len()];
    for instance in 0..instances {
        for component in 0..elements {
            result[component * instances + instance] = values[instance * elements + component];
        }
    }
    result
}

fn from_soa(values: &[f32], elements: usize, instances: usize) -> Vec<f32> {
    let mut result = vec![0.0; values.len()];
    for instance in 0..instances {
        for component in 0..elements {
            result[instance * elements + component] = values[component * instances + instance];
        }
    }
    result
}

impl NativeKernel {
    fn compile(program: &FixedShapeKernel) -> Result<Self, BatchedExecutionError> {
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
        for _ in 0..5 {
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
            builder.append_block_param(fault, pointer_type);
            builder.append_block_param(fault, types::I32);
            builder.switch_to_block(entry);

            let parameters = builder.block_params(entry).to_vec();
            let input_table = parameters[0];
            let state_table = parameters[1];
            let next_state_table = parameters[2];
            let start = parameters[3];
            let end = parameters[4];
            let pointer_bytes = i32::try_from(pointer_type.bytes()).unwrap();
            let flags = MemFlags::trusted();
            let input_component_offsets =
                component_offsets(program.inputs.iter().map(|input| input.shape.elements()));
            let state_component_offsets =
                component_offsets(program.states.iter().map(|state| state.shape.elements()));
            let input_component_count: usize = program
                .inputs
                .iter()
                .map(|input| input.shape.elements())
                .sum();
            let state_component_count: usize = program
                .states
                .iter()
                .map(|state| state.shape.elements())
                .sum();
            let input_bases = (0..input_component_count)
                .map(|index| {
                    builder.ins().load(
                        pointer_type,
                        flags,
                        input_table,
                        i32::try_from(index).unwrap() * pointer_bytes,
                    )
                })
                .collect::<Vec<_>>();
            let state_bases = (0..state_component_count)
                .map(|index| {
                    builder.ins().load(
                        pointer_type,
                        flags,
                        state_table,
                        i32::try_from(index).unwrap() * pointer_bytes,
                    )
                })
                .collect::<Vec<_>>();
            let next_state_bases = (0..state_component_count)
                .map(|index| {
                    builder.ins().load(
                        pointer_type,
                        flags,
                        next_state_table,
                        i32::try_from(index).unwrap() * pointer_bytes,
                    )
                })
                .collect::<Vec<_>>();
            builder.ins().jump(header, &[start.into()]);

            builder.switch_to_block(header);
            let instance = builder.block_params(header)[0];
            let has_instance = builder.ins().icmp(IntCC::UnsignedLessThan, instance, end);
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
            for (index, input) in program.inputs.iter().enumerate() {
                let offset = program.register_offsets[&input.slot];
                for component in 0..input.shape.elements() {
                    registers[offset + component] = Some(NativeRegister::F32(load_component(
                        &mut builder,
                        input_bases[input_component_offsets[index] + component],
                        instance,
                        pointer_type,
                    )));
                }
            }
            for (index, state) in program.states.iter().enumerate() {
                let offset = program.register_offsets[&state.slot];
                for component in 0..state.shape.elements() {
                    registers[offset + component] = Some(NativeRegister::F32(load_component(
                        &mut builder,
                        state_bases[state_component_offsets[index] + component],
                        instance,
                        pointer_type,
                    )));
                }
            }
            for instruction in &program.fixed_ir().instructions {
                let value = lower_computation(
                    &mut builder,
                    &instruction.computation,
                    &registers,
                    functions,
                )?;
                registers[instruction.output] = Some(value);
            }
            let mut constraint_code = builder.ins().iconst(types::I32, 0);
            for (index, constraint) in program.constraints.iter().enumerate() {
                let condition = lower_predicate(&mut builder, &constraint.predicate, &registers)?;
                let code_is_empty = builder.ins().icmp_imm(IntCC::Equal, constraint_code, 0);
                let failed = builder.ins().bnot(condition);
                let record = builder.ins().band(code_is_empty, failed);
                let code = builder.ins().iconst(types::I32, (index + 1) as i64);
                constraint_code = builder.ins().select(record, code, constraint_code);
            }
            for (index, state) in program.states.iter().enumerate() {
                for (component, source) in state.update.iter().enumerate() {
                    let value = lower_numeric_operand(&mut builder, *source, &registers)?;
                    store_component(
                        &mut builder,
                        next_state_bases[state_component_offsets[index] + component],
                        instance,
                        pointer_type,
                        value,
                    );
                }
            }

            let has_fault = builder.ins().icmp_imm(IntCC::NotEqual, constraint_code, 0);
            builder.ins().brif(
                has_fault,
                fault,
                &[instance.into(), constraint_code.into()],
                advance,
                &[],
            );

            builder.switch_to_block(advance);
            let next_instance = builder.ins().iadd_imm(instance, 1);
            builder.ins().jump(header, &[next_instance.into()]);

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
        // SAFETY: `code` is the finalized entry point for the four-argument
        // signature constructed above. The module remains owned by the kernel.
        let turn = unsafe { mem::transmute::<*const u8, NativeTurn>(code) };
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

fn lower_computation(
    builder: &mut FunctionBuilder<'_>,
    computation: &ScalarComputation,
    registers: &[Option<NativeRegister>],
    functions: MathFunctions,
) -> Result<NativeRegister, BatchedExecutionError> {
    Ok(match computation {
        ScalarComputation::Copy(input) => lower_operand(builder, *input, registers)?,
        ScalarComputation::Negate(input) => {
            let value = lower_numeric_operand(builder, *input, registers)?;
            NativeRegister::F32(builder.ins().fneg(value))
        }
        ScalarComputation::Absolute(input) => {
            let value = lower_numeric_operand(builder, *input, registers)?;
            NativeRegister::F32(builder.ins().fabs(value))
        }
        ScalarComputation::IsFinite(input) => {
            let value = lower_numeric_operand(builder, *input, registers)?;
            NativeRegister::Bool(lower_is_finite(builder, value))
        }
        ScalarComputation::Compare {
            operation,
            left,
            right,
        } => {
            let left = lower_numeric_operand(builder, *left, registers)?;
            let right = lower_numeric_operand(builder, *right, registers)?;
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
            let left = lower_boolean_operand(builder, inputs[0], registers)?;
            let condition = if *operation == LogicOperation::Not {
                builder.ins().bnot(left)
            } else {
                let right = lower_boolean_operand(builder, inputs[1], registers)?;
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
                .map(|input| lower_numeric_operand(builder, *input, registers))
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
            let mut sum = builder.ins().f32const(0.0);
            for (left, right) in terms {
                let left_is_one = super::is_one_operand(*left);
                let right_is_one = super::is_one_operand(*right);
                let left_is_negative_one = super::is_negative_one_operand(*left);
                let right_is_negative_one = super::is_negative_one_operand(*right);
                let left = lower_numeric_operand(builder, *left, registers)?;
                let right = lower_numeric_operand(builder, *right, registers)?;
                if left_is_one {
                    sum = builder.ins().fadd(sum, right);
                } else if right_is_one {
                    sum = builder.ins().fadd(sum, left);
                } else if left_is_negative_one {
                    sum = builder.ins().fsub(sum, right);
                } else if right_is_negative_one {
                    sum = builder.ins().fsub(sum, left);
                } else {
                    sum = builder.ins().fma(left, right, sum);
                }
            }
            NativeRegister::F32(sum)
        }
    })
}

fn lower_predicate(
    builder: &mut FunctionBuilder<'_>,
    predicate: &ScalarPredicate,
    registers: &[Option<NativeRegister>],
) -> Result<Value, BatchedExecutionError> {
    Ok(match predicate {
        ScalarPredicate::Value(operand) => match lower_operand(builder, *operand, registers)? {
            NativeRegister::Bool(value) => value,
            NativeRegister::F32(value) => {
                let zero = builder.ins().f32const(0.0);
                builder.ins().fcmp(FloatCC::NotEqual, value, zero)
            }
        },
        ScalarPredicate::IsFinite(operand) => {
            let value = lower_numeric_operand(builder, *operand, registers)?;
            lower_is_finite(builder, value)
        }
        ScalarPredicate::AbsoluteDifferenceWithin {
            left,
            right,
            tolerance,
        } => {
            let left = lower_numeric_operand(builder, *left, registers)?;
            let right = lower_numeric_operand(builder, *right, registers)?;
            let tolerance = lower_numeric_operand(builder, *tolerance, registers)?;
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
            let left = lower_numeric_operand(builder, *left, registers)?;
            let right = lower_numeric_operand(builder, *right, registers)?;
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
            )?;
            for input in inputs {
                let next = lower_predicate(builder, input, registers)?;
                condition = builder.ins().band(condition, next);
            }
            condition
        }
        ScalarPredicate::Logic { operation, inputs } => {
            let left = lower_predicate(builder, &inputs[0], registers)?;
            if *operation == LogicOperation::Not {
                builder.ins().bnot(left)
            } else {
                let right = lower_predicate(builder, &inputs[1], registers)?;
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
    builder: &mut FunctionBuilder<'_>,
    operand: ScalarOperand,
    registers: &[Option<NativeRegister>],
) -> Result<NativeRegister, BatchedExecutionError> {
    match operand {
        ScalarOperand::Register(register) => registers[register].ok_or_else(|| {
            BatchedExecutionError::Native(format!(
                "native lowering read register {register} before definition"
            ))
        }),
        ScalarOperand::Constant(value) => Ok(NativeRegister::F32(builder.ins().f32const(value))),
    }
}

fn lower_numeric_operand(
    builder: &mut FunctionBuilder<'_>,
    operand: ScalarOperand,
    registers: &[Option<NativeRegister>],
) -> Result<Value, BatchedExecutionError> {
    match lower_operand(builder, operand, registers)? {
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
) -> Result<Value, BatchedExecutionError> {
    match lower_operand(builder, operand, registers)? {
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

fn component_offsets(elements: impl IntoIterator<Item = usize>) -> Vec<usize> {
    let mut offsets = Vec::new();
    let mut offset = 0;
    for count in elements {
        offsets.push(offset);
        offset += count;
    }
    offsets
}

fn load_component(
    builder: &mut FunctionBuilder<'_>,
    base: Value,
    instance: Value,
    pointer_type: cranelift_codegen::ir::Type,
) -> Value {
    let address = component_address(builder, base, instance, pointer_type);
    builder
        .ins()
        .load(types::F32, MemFlags::trusted(), address, 0)
}

fn store_component(
    builder: &mut FunctionBuilder<'_>,
    base: Value,
    instance: Value,
    pointer_type: cranelift_codegen::ir::Type,
    value: Value,
) {
    let address = component_address(builder, base, instance, pointer_type);
    builder.ins().store(MemFlags::trusted(), value, address, 0);
}

fn component_address(
    builder: &mut FunctionBuilder<'_>,
    base: Value,
    instance: Value,
    pointer_type: cranelift_codegen::ir::Type,
) -> Value {
    let byte_offset = builder
        .ins()
        .imul_imm(instance, i64::from(types::F32.bytes()));
    debug_assert_eq!(builder.func.dfg.value_type(byte_offset), pointer_type);
    builder.ins().iadd(base, byte_offset)
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
