use std::collections::BTreeMap;
use std::mem;

use cranelift_codegen::ir::{
    AbiParam, InstBuilder, MemFlags, UserFuncName, Value, condcodes::IntCC, types,
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, default_libcall_names};
use mech_core::CellSlotId;

use super::{
    BatchedExecutionError, BatchedGpuProgram, BinaryOperation, ElementwiseOperation,
    ScalarComputation, ScalarOperand, UnaryOperation,
};

type NativeTurn = unsafe extern "C" fn(
    input_pointers: *const *const f32,
    state_pointers: *const *const f32,
    next_state_pointers: *const *mut f32,
    instances: usize,
);

struct NativeKernel {
    _module: JITModule,
    turn: NativeTurn,
}

pub struct BatchedJitCpuSession<'a> {
    program: &'a BatchedGpuProgram,
    kernel: NativeKernel,
    _inputs: BTreeMap<CellSlotId, Vec<f32>>,
    state: BTreeMap<CellSlotId, Vec<f32>>,
    next_state: BTreeMap<CellSlotId, Vec<f32>>,
    input_pointers: Vec<*const f32>,
    state_pointers: Vec<*const f32>,
    next_state_pointers: Vec<*mut f32>,
}

impl BatchedGpuProgram {
    pub fn prepare_jit_cpu(
        &self,
        inputs: &BTreeMap<String, Vec<f32>>,
    ) -> Result<BatchedJitCpuSession<'_>, BatchedExecutionError> {
        let inputs = self.expand_inputs(inputs)?;
        let state = self.initial_state();
        let next_state = state
            .iter()
            .map(|(slot, values)| (*slot, vec![0.0; values.len()]))
            .collect();
        let kernel = NativeKernel::compile(self)?;
        let input_pointers = self
            .inputs
            .iter()
            .map(|input| inputs[&input.slot].as_ptr())
            .collect();
        let mut session = BatchedJitCpuSession {
            program: self,
            kernel,
            _inputs: inputs,
            state,
            next_state,
            input_pointers,
            state_pointers: Vec::with_capacity(self.states.len()),
            next_state_pointers: Vec::with_capacity(self.states.len()),
        };
        session.refresh_state_pointers();
        Ok(session)
    }
}

impl BatchedJitCpuSession<'_> {
    pub fn dispatch_turns(&mut self, turns: u32) -> Result<(), BatchedExecutionError> {
        if turns == 0 {
            return Err(BatchedExecutionError::ZeroTurns);
        }
        for _ in 0..turns {
            self.refresh_state_pointers();
            // SAFETY: The generated function uses the exact ABI below. All pointer
            // tables and their backing f32 buffers remain live for the call, and
            // every generated access is bounded by the admitted fixed shapes and
            // the program's instance count.
            unsafe {
                (self.kernel.turn)(
                    self.input_pointers.as_ptr(),
                    self.state_pointers.as_ptr(),
                    self.next_state_pointers.as_ptr(),
                    self.program.instances as usize,
                );
            }
            mem::swap(&mut self.state, &mut self.next_state);
        }
        Ok(())
    }

    pub fn state(&self) -> &BTreeMap<CellSlotId, Vec<f32>> {
        &self.state
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
    fn compile(program: &BatchedGpuProgram) -> Result<Self, BatchedExecutionError> {
        let mut jit_builder =
            JITBuilder::with_flags(&[("opt_level", "speed")], default_libcall_names())
                .map_err(native_error)?;
        jit_builder
            .symbol("mech_jit_sinf", mech_jit_sinf as *const u8)
            .symbol("mech_jit_cosf", mech_jit_cosf as *const u8)
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
        let atan2_id = module
            .declare_function("mech_jit_atan2f", Linkage::Import, &binary_signature)
            .map_err(native_error)?;

        let pointer_type = module.target_config().pointer_type();
        let mut signature = module.make_signature();
        for _ in 0..4 {
            signature.params.push(AbiParam::new(pointer_type));
        }
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
            let exit = builder.create_block();
            builder.append_block_params_for_function_params(entry);
            builder.append_block_param(header, pointer_type);
            builder.switch_to_block(entry);

            let parameters = builder.block_params(entry).to_vec();
            let input_table = parameters[0];
            let state_table = parameters[1];
            let next_state_table = parameters[2];
            let instances = parameters[3];
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
            let zero = builder.ins().iconst(pointer_type, 0);
            builder.ins().jump(header, &[zero.into()]);

            builder.switch_to_block(header);
            let instance = builder.block_params(header)[0];
            let has_instance = builder
                .ins()
                .icmp(IntCC::UnsignedLessThan, instance, instances);
            builder.ins().brif(has_instance, body, &[], exit, &[]);

            builder.switch_to_block(body);
            let sin_ref = module.declare_func_in_func(sin_id, builder.func);
            let cos_ref = module.declare_func_in_func(cos_id, builder.func);
            let atan2_ref = module.declare_func_in_func(atan2_id, builder.func);
            let functions = MathFunctions {
                sin: sin_ref,
                cos: cos_ref,
                atan2: atan2_ref,
            };
            let mut registers = vec![None; program.register_count];
            for (index, input) in program.inputs.iter().enumerate() {
                let offset = program.register_offsets[&input.slot];
                for component in 0..input.shape.elements() {
                    registers[offset + component] = Some(load_component(
                        &mut builder,
                        input_bases[index],
                        instance,
                        input.shape.elements(),
                        component,
                        pointer_type,
                    ));
                }
            }
            for (index, state) in program.states.iter().enumerate() {
                let offset = program.register_offsets[&state.slot];
                for component in 0..state.shape.elements() {
                    registers[offset + component] = Some(load_component(
                        &mut builder,
                        state_bases[index],
                        instance,
                        state.shape.elements(),
                        component,
                        pointer_type,
                    ));
                }
            }
            for instruction in &program.instructions {
                let value = lower_computation(
                    &mut builder,
                    &instruction.computation,
                    &registers,
                    functions,
                )?;
                registers[instruction.output] = Some(value);
            }
            for (index, state) in program.states.iter().enumerate() {
                for (component, source) in state.update.iter().enumerate() {
                    let value = lower_operand(&mut builder, *source, &registers)?;
                    store_component(
                        &mut builder,
                        next_state_bases[index],
                        instance,
                        state.shape.elements(),
                        component,
                        pointer_type,
                        value,
                    );
                }
            }
            let next_instance = builder.ins().iadd_imm(instance, 1);
            builder.ins().jump(header, &[next_instance.into()]);

            builder.switch_to_block(exit);
            builder.ins().return_(&[]);
            builder.seal_all_blocks();
            builder.finalize();
        }

        module
            .define_function(function_id, &mut context)
            .map_err(native_error)?;
        module.clear_context(&mut context);
        module.finalize_definitions().map_err(native_error)?;
        let code = module.get_finalized_function(function_id);
        // SAFETY: `code` is the finalized entry point for the four-pointer
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
    atan2: cranelift_codegen::ir::FuncRef,
}

fn lower_computation(
    builder: &mut FunctionBuilder<'_>,
    computation: &ScalarComputation,
    registers: &[Option<Value>],
    functions: MathFunctions,
) -> Result<Value, BatchedExecutionError> {
    Ok(match computation {
        ScalarComputation::Copy(input) => lower_operand(builder, *input, registers)?,
        ScalarComputation::Negate(input) => {
            let value = lower_operand(builder, *input, registers)?;
            builder.ins().fneg(value)
        }
        ScalarComputation::Elementwise { operation, inputs } => {
            let values = inputs
                .iter()
                .map(|input| lower_operand(builder, *input, registers))
                .collect::<Result<Vec<_>, _>>()?;
            match operation {
                ElementwiseOperation::Binary(operation) => match operation {
                    BinaryOperation::Add => builder.ins().fadd(values[0], values[1]),
                    BinaryOperation::Subtract => builder.ins().fsub(values[0], values[1]),
                    BinaryOperation::Multiply => builder.ins().fmul(values[0], values[1]),
                    BinaryOperation::Divide => builder.ins().fdiv(values[0], values[1]),
                },
                ElementwiseOperation::Unary(operation) => match operation {
                    UnaryOperation::Sin => call_math(builder, functions.sin, &values),
                    UnaryOperation::Cos => call_math(builder, functions.cos, &values),
                },
                ElementwiseOperation::Atan2 => call_math(builder, functions.atan2, &values),
                ElementwiseOperation::Identity => values[0],
                ElementwiseOperation::Pack2 => {
                    return Err(BatchedExecutionError::Native(
                        "pack2 reached the native fixed-shape backend".to_owned(),
                    ));
                }
            }
        }
        ScalarComputation::SumProducts(terms) => {
            let mut sum = builder.ins().f32const(0.0);
            for (left, right) in terms {
                let left = lower_operand(builder, *left, registers)?;
                let right = lower_operand(builder, *right, registers)?;
                sum = builder.ins().fma(left, right, sum);
            }
            sum
        }
    })
}

fn lower_operand(
    builder: &mut FunctionBuilder<'_>,
    operand: ScalarOperand,
    registers: &[Option<Value>],
) -> Result<Value, BatchedExecutionError> {
    match operand {
        ScalarOperand::Register(register) => registers[register].ok_or_else(|| {
            BatchedExecutionError::Native(format!(
                "native lowering read register {register} before definition"
            ))
        }),
        ScalarOperand::Constant(value) => Ok(builder.ins().f32const(value)),
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

fn load_component(
    builder: &mut FunctionBuilder<'_>,
    base: Value,
    instance: Value,
    elements: usize,
    component: usize,
    pointer_type: cranelift_codegen::ir::Type,
) -> Value {
    let address = component_address(builder, base, instance, elements, component, pointer_type);
    builder
        .ins()
        .load(types::F32, MemFlags::trusted(), address, 0)
}

fn store_component(
    builder: &mut FunctionBuilder<'_>,
    base: Value,
    instance: Value,
    elements: usize,
    component: usize,
    pointer_type: cranelift_codegen::ir::Type,
    value: Value,
) {
    let address = component_address(builder, base, instance, elements, component, pointer_type);
    builder.ins().store(MemFlags::trusted(), value, address, 0);
}

fn component_address(
    builder: &mut FunctionBuilder<'_>,
    base: Value,
    instance: Value,
    elements: usize,
    component: usize,
    pointer_type: cranelift_codegen::ir::Type,
) -> Value {
    let element = builder
        .ins()
        .imul_imm(instance, i64::try_from(elements).unwrap());
    let element = builder
        .ins()
        .iadd_imm(element, i64::try_from(component).unwrap());
    let byte_offset = builder
        .ins()
        .imul_imm(element, i64::from(types::F32.bytes()));
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

extern "C" fn mech_jit_atan2f(y: f32, x: f32) -> f32 {
    y.atan2(x)
}
