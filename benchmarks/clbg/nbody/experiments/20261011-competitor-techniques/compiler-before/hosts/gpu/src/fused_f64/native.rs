use super::{FusedProgram, Op, Operand, element_groups};
use cranelift_codegen::ir::{
    AbiParam, ConstantData, Endianness, InstBuilder, MemFlags, Value,
    condcodes::{FloatCC, IntCC},
    types,
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{FuncId, Linkage, Module, default_libcall_names};
use std::collections::{BTreeMap, BTreeSet};

type NativeLoop = unsafe extern "C" fn(*mut f64, usize);
type NativeObserve = unsafe extern "C" fn(*const f64, *mut f64);
pub struct JitKernel {
    _module: JITModule,
    run: NativeLoop,
    state_len: usize,
    output: Option<(NativeObserve, usize)>,
}

extern "C" fn mech_fused_pow(x: f64, y: f64) -> f64 {
    x.powf(y)
}

impl JitKernel {
    pub fn compile(program: &FusedProgram) -> Result<Self, String> {
        Self::compile_with_simd(program, false)
    }

    pub fn compile_with_simd(program: &FusedProgram, simd: bool) -> Result<Self, String> {
        Self::compile_internal(program, simd, None)
    }

    pub fn compile_with_output(
        program: &FusedProgram,
        output: &str,
        simd: bool,
    ) -> Result<Self, String> {
        Self::compile_internal(program, simd, Some(output))
    }

    fn compile_internal(
        program: &FusedProgram,
        simd: bool,
        output: Option<&str>,
    ) -> Result<Self, String> {
        let mut jit = JITBuilder::with_flags(&[("opt_level", "speed")], default_libcall_names())
            .map_err(error)?;
        jit.symbol("mech_fused_pow", mech_fused_pow as *const u8);
        let mut module = JITModule::new(jit);
        let function = define_program(program, &mut module, simd)?;
        let observation = output
            .map(|name| {
                Ok::<_, String>((
                    define_observer(program, &mut module, name)?,
                    program.output_len(name)?,
                ))
            })
            .transpose()?;
        module.finalize_definitions().map_err(error)?;
        let address = module.get_finalized_function(function);
        // SAFETY: The compiled signature is exactly NativeLoop; the owning
        // JITModule is retained for every invocation of its executable memory.
        let run = unsafe { std::mem::transmute::<*const u8, NativeLoop>(address) };
        let output = observation.map(|(function, len)| {
            let address = module.get_finalized_function(function);
            // SAFETY: define_observer emits exactly NativeObserve. The same
            // owning JITModule retains its executable memory for every call.
            let observe = unsafe { std::mem::transmute::<*const u8, NativeObserve>(address) };
            (observe, len)
        });
        Ok(Self {
            _module: module,
            run,
            state_len: program.initial.len(),
            output,
        })
    }

    pub fn run(&self, state: &mut [f64], turns: usize) {
        assert_eq!(state.len(), self.state_len);
        // SAFETY: The slice has the admitted size, is exclusively borrowed,
        // and generated accesses use only constant indices within that size.
        unsafe { (self.run)(state.as_mut_ptr(), turns) };
    }

    /// Evaluate the selected pure output from the current state without
    /// committing any state update or advancing the simulation.
    pub fn observe(&self, state: &[f64]) -> Result<Vec<f64>, String> {
        assert_eq!(state.len(), self.state_len);
        let (observe, len) = self.output.ok_or("kernel has no selected output")?;
        let mut output = vec![0.0; len];
        // SAFETY: Both slices have the admitted lengths, input is immutable,
        // output is exclusively borrowed, and generated offsets are bounded.
        unsafe { observe(state.as_ptr(), output.as_mut_ptr()) };
        Ok(output)
    }
}

fn value(operand: Operand, registers: &[Value], builder: &mut FunctionBuilder<'_>) -> Value {
    match operand {
        Operand::Constant(x) => builder.ins().f64const(x),
        Operand::Register(index) => registers[index],
    }
}

type PackedValues = BTreeMap<((u8, u64), (u8, u64)), Value>;
type LaneOrigins = BTreeMap<usize, (Value, u8)>;

fn shuffle_pair(a: (Value, u8), b: (Value, u8), builder: &mut FunctionBuilder<'_>) -> Value {
    if a.0 == b.0 && a.1 == 0 && b.1 == 1 {
        return a.0;
    }
    // Canonicalize a single-vector shuffle to indices in its first input.
    // A repeated lane is then recognized as a broadcast by Cranelift's
    // indexed-FMA lowering, rather than materialized in another register.
    let second_base = if a.0 == b.0 { 0 } else { 16 };
    let mask = (a.1 * 8..a.1 * 8 + 8)
        .chain(second_base + b.1 * 8..second_base + b.1 * 8 + 8)
        .collect::<Vec<_>>();
    let mask = builder.func.dfg.immediates.push(ConstantData::from(mask));
    let flags = MemFlags::new().with_endianness(Endianness::Little);
    let a = builder.ins().bitcast(types::I8X16, flags, a.0);
    let b = builder.ins().bitcast(types::I8X16, flags, b.0);
    let vector = builder.ins().shuffle(a, b, mask);
    builder.ins().bitcast(types::F64X2, flags, vector)
}

fn packed_pair(
    first: Operand,
    second: Operand,
    registers: &[Value],
    packed: &mut PackedValues,
    origins: &LaneOrigins,
    builder: &mut FunctionBuilder<'_>,
) -> Value {
    let key = (operand_key(first), operand_key(second));
    if let Some(vector) = packed.get(&key) {
        return *vector;
    }
    let origin = |operand| match operand {
        Operand::Register(index) => origins.get(&index).copied(),
        Operand::Constant(_) => None,
    };
    let vector = if let (Operand::Constant(a), Operand::Constant(b)) = (first, second) {
        let bytes = a
            .to_bits()
            .to_le_bytes()
            .into_iter()
            .chain(b.to_bits().to_le_bytes())
            .collect::<Vec<_>>();
        let constant = builder.func.dfg.constants.insert(ConstantData::from(bytes));
        builder.ins().vconst(types::F64X2, constant)
    } else if let (Some(a), Some(b)) = (origin(first), origin(second)) {
        shuffle_pair(a, b, builder)
    } else if operand_key(first) == operand_key(second) {
        let scalar = value(first, registers, builder);
        builder.ins().splat(types::F64X2, scalar)
    } else {
        let first = value(first, registers, builder);
        let second = value(second, registers, builder);
        let vector = builder.ins().scalar_to_vector(types::F64X2, first);
        builder.ins().insertlane(vector, second, 1)
    };
    packed.insert(key, vector);
    vector
}

fn error(error: impl std::fmt::Display) -> String {
    error.to_string()
}

pub(super) fn compile_object(program: &FusedProgram, simd: bool) -> Result<Vec<u8>, String> {
    compile_object_with_output(program, simd, None)
}

pub(super) fn compile_object_with_output(
    program: &FusedProgram,
    simd: bool,
    output: Option<&str>,
) -> Result<Vec<u8>, String> {
    crate::batched::aot::emit_native_object("mech_fused_f64", |module| {
        define_program(program, module, simd).map_err(crate::BatchedExecutionError::Native)?;
        if let Some(output) = output {
            define_observer(program, module, output)
                .map_err(crate::BatchedExecutionError::Native)?;
        }
        Ok(())
    })
    .map_err(error)
}

fn define_observer<M: Module>(
    program: &FusedProgram,
    module: &mut M,
    output: &str,
) -> Result<FuncId, String> {
    let values = &program
        .outputs
        .get(output)
        .ok_or_else(|| format!("unknown output: {output}"))?
        .values;
    let pointer = module.target_config().pointer_type();
    let mut signature = module.make_signature();
    signature
        .params
        .extend([AbiParam::new(pointer), AbiParam::new(pointer)]);
    let function = module
        .declare_function("mech_observe", Linkage::Export, &signature)
        .map_err(error)?;
    let mut pow_signature = module.make_signature();
    pow_signature
        .params
        .extend([AbiParam::new(types::F64), AbiParam::new(types::F64)]);
    pow_signature.returns.push(AbiParam::new(types::F64));
    let pow = module
        .declare_function("mech_fused_pow", Linkage::Import, &pow_signature)
        .map_err(error)?;
    let mut context = module.make_context();
    context.func.signature = signature;
    let mut function_context = FunctionBuilderContext::new();
    {
        let mut builder = FunctionBuilder::new(&mut context.func, &mut function_context);
        let entry = builder.create_block();
        builder.append_block_params_for_function_params(entry);
        builder.switch_to_block(entry);
        let state = builder.block_params(entry)[0];
        let output = builder.block_params(entry)[1];
        let pow_ref = module.declare_func_in_func(pow, builder.func);
        let mut registers = (0..program.initial.len())
            .map(|i| {
                builder
                    .ins()
                    .load(types::F64, MemFlags::new(), state, (i * 8) as i32)
            })
            .collect::<Vec<_>>();
        let live = program.live_instructions(values);
        for (index, instruction) in program.instructions.iter().enumerate() {
            if !live.contains(&index) {
                registers.push(builder.ins().f64const(0.0));
                continue;
            }
            let inputs = instruction
                .inputs
                .iter()
                .map(|x| value(*x, &registers, &mut builder))
                .collect::<Vec<_>>();
            registers.push(arithmetic(instruction.op, &inputs, &mut builder, pow_ref));
        }
        for (i, operand) in values.iter().enumerate() {
            let scalar = value(*operand, &registers, &mut builder);
            builder
                .ins()
                .store(MemFlags::new(), scalar, output, (i * 8) as i32);
        }
        builder.ins().return_(&[]);
        builder.seal_all_blocks();
        builder.finalize();
    }
    module
        .define_function(function, &mut context)
        .map_err(|error| format!("{error:?}"))?;
    Ok(function)
}

fn simd_state_groups(program: &FusedProgram) -> Vec<Vec<usize>> {
    program
        .states
        .iter()
        .zip(&program.state_shapes)
        .flat_map(|((_, offset, _), (rows, columns))| {
            let mut groups = Vec::new();
            let mut tails = Vec::new();
            for group in element_groups(*rows, *columns) {
                for chunk in group.chunks(2) {
                    let indices = chunk.iter().map(|i| offset + i).collect::<Vec<_>>();
                    if program.pack_state_tails && indices.len() == 1 {
                        tails.push(indices[0]);
                    } else {
                        groups.push(indices);
                    }
                }
            }
            groups.extend(tails.chunks(2).map(|chunk| chunk.to_vec()));
            groups
        })
        .collect()
}

fn define_program<M: Module>(
    program: &FusedProgram,
    module: &mut M,
    simd: bool,
) -> Result<FuncId, String> {
    let pointer = module.target_config().pointer_type();
    let mut signature = module.make_signature();
    signature
        .params
        .extend([AbiParam::new(pointer), AbiParam::new(pointer)]);
    let function = module
        .declare_function("mech_turns", Linkage::Export, &signature)
        .map_err(error)?;
    let mut pow_signature = module.make_signature();
    pow_signature
        .params
        .extend([AbiParam::new(types::F64), AbiParam::new(types::F64)]);
    pow_signature.returns.push(AbiParam::new(types::F64));
    let pow = module
        .declare_function("mech_fused_pow", Linkage::Import, &pow_signature)
        .map_err(error)?;
    let mut context = module.make_context();
    context.func.signature = signature;
    let mut function_context = FunctionBuilderContext::new();
    {
        let mut builder = FunctionBuilder::new(&mut context.func, &mut function_context);
        let entry = builder.create_block();
        let header = builder.create_block();
        let body = builder.create_block();
        let exit = builder.create_block();
        builder.append_block_params_for_function_params(entry);
        builder.append_block_param(header, pointer);
        let state_groups = if simd {
            simd_state_groups(program)
        } else {
            (0..program.initial.len()).map(|i| vec![i]).collect()
        };
        for chunk in &state_groups {
            builder.append_block_param(
                header,
                if chunk.len() == 2 {
                    types::F64X2
                } else {
                    types::F64
                },
            );
        }
        builder.switch_to_block(entry);
        let state = builder.block_params(entry)[0];
        let turns = builder.block_params(entry)[1];
        let mut initial = vec![turns];
        for chunk in &state_groups {
            let a = builder
                .ins()
                .load(types::F64, MemFlags::new(), state, (chunk[0] * 8) as i32);
            initial.push(if chunk.len() == 2 {
                let b =
                    builder
                        .ins()
                        .load(types::F64, MemFlags::new(), state, (chunk[1] * 8) as i32);
                let vector = builder.ins().scalar_to_vector(types::F64X2, a);
                builder.ins().insertlane(vector, b, 1)
            } else {
                a
            });
        }
        let mut input_values = BTreeMap::new();
        for input in &program.inputs {
            for i in input.offset..input.offset + input.rows * input.columns {
                input_values.insert(
                    i,
                    builder
                        .ins()
                        .load(types::F64, MemFlags::new(), state, (i * 8) as i32),
                );
            }
        }
        builder.ins().jump(
            header,
            &initial.iter().copied().map(Into::into).collect::<Vec<_>>(),
        );
        builder.seal_block(entry);
        builder.switch_to_block(header);
        let remaining = builder.block_params(header)[0];
        let states = builder.block_params(header)[1..].to_vec();
        let done = builder.ins().icmp_imm(IntCC::Equal, remaining, 0);
        builder.ins().brif(done, exit, &[], body, &[]);
        builder.switch_to_block(body);
        builder.seal_block(body);
        let pow_ref = module.declare_func_in_func(pow, builder.func);
        let mut registers = vec![remaining; program.initial.len()];
        let mut packed = BTreeMap::new();
        let mut origins = BTreeMap::new();
        for (state, chunk) in states.iter().zip(&state_groups) {
            if chunk.len() == 2 {
                registers[chunk[0]] = builder.ins().extractlane(*state, 0);
                registers[chunk[1]] = builder.ins().extractlane(*state, 1);
                origins.insert(chunk[0], (*state, 0));
                origins.insert(chunk[1], (*state, 1));
                packed.insert(
                    (
                        operand_key(Operand::Register(chunk[0])),
                        operand_key(Operand::Register(chunk[1])),
                    ),
                    *state,
                );
            } else {
                registers[chunk[0]] = *state;
            }
        }
        for (index, value) in input_values {
            registers[index] = value;
        }
        let zero = builder.ins().f64const(0.0);
        registers.resize(program.initial.len() + program.instructions.len(), zero);
        let roots = state_groups
            .iter()
            .flatten()
            .map(|i| program.updates[*i])
            .collect::<Vec<_>>();
        emit_instructions(
            program,
            &roots,
            simd,
            &mut registers,
            &mut packed,
            &mut origins,
            &mut BTreeSet::new(),
            &mut builder,
            pow_ref,
        );
        let next_turns = builder.ins().iadd_imm(remaining, -1);
        let mut next = vec![next_turns];
        for chunk in &state_groups {
            next.push(if chunk.len() == 2 {
                packed_pair(
                    program.updates[chunk[0]],
                    program.updates[chunk[1]],
                    &registers,
                    &mut packed,
                    &origins,
                    &mut builder,
                )
            } else {
                value(program.updates[chunk[0]], &registers, &mut builder)
            });
        }
        builder.ins().jump(
            header,
            &next.iter().copied().map(Into::into).collect::<Vec<_>>(),
        );
        builder.seal_block(header);
        builder.switch_to_block(exit);
        builder.seal_block(exit);
        for (value, chunk) in states.iter().zip(&state_groups) {
            for (lane, index) in chunk.iter().enumerate() {
                let scalar = if chunk.len() == 2 {
                    builder.ins().extractlane(*value, lane as u8)
                } else {
                    *value
                };
                builder
                    .ins()
                    .store(MemFlags::new(), scalar, state, (index * 8) as i32);
            }
        }
        builder.ins().return_(&[]);
        builder.finalize();
    }
    module
        .define_function(function, &mut context)
        .map_err(|error| format!("{error:?}"))?;
    Ok(function)
}

#[allow(clippy::too_many_arguments)]
fn emit_instructions(
    program: &FusedProgram,
    roots: &[Operand],
    simd: bool,
    registers: &mut Vec<Value>,
    packed: &mut PackedValues,
    origins: &mut LaneOrigins,
    emitted: &mut BTreeSet<usize>,
    builder: &mut FunctionBuilder<'_>,
    pow_ref: cranelift_codegen::ir::FuncRef,
) {
    let order = program.instruction_order(&roots, simd);
    let mut cursor = 0;
    while cursor < order.len() {
        let index = order[cursor];
        if emitted.contains(&index) {
            cursor += 1;
            continue;
        }
        let instruction = &program.instructions[index];
        if simd
            && program.simd_pairs.contains(&index)
            && order.get(cursor + 1) == Some(&(index + 1))
            && !emitted.contains(&(index + 1))
            && instruction.op != Op::Pow
            && let Some(second) = program.instructions.get(index + 1)
            && instruction.op == second.op
            && !second
                .inputs
                .iter()
                .any(|x| matches!(x, Operand::Register(r) if *r == program.initial.len()+index))
        {
            let inputs = instruction
                .inputs
                .iter()
                .zip(&second.inputs)
                .map(|(a, b)| packed_pair(*a, *b, registers, packed, origins, builder))
                .collect::<Vec<_>>();
            let vector = arithmetic(instruction.op, &inputs, builder, pow_ref);
            let first_index = program.initial.len() + index;
            registers[first_index] = builder.ins().extractlane(vector, 0);
            registers[first_index + 1] = builder.ins().extractlane(vector, 1);
            origins.insert(first_index, (vector, 0));
            origins.insert(first_index + 1, (vector, 1));
            packed.insert(
                (
                    operand_key(Operand::Register(first_index)),
                    operand_key(Operand::Register(first_index + 1)),
                ),
                vector,
            );
            emitted.extend([index, index + 1]);
            cursor += 2;
            continue;
        }
        let inputs = instruction
            .inputs
            .iter()
            .map(|operand| value(*operand, registers, builder))
            .collect::<Vec<_>>();
        let result = arithmetic(instruction.op, &inputs, builder, pow_ref);
        registers[program.initial.len() + index] = result;
        emitted.insert(index);
        cursor += 1;
    }
}

fn operand_key(operand: Operand) -> (u8, u64) {
    match operand {
        Operand::Constant(x) => (0, x.to_bits()),
        Operand::Register(index) => (1, index as u64),
    }
}

fn arithmetic(
    op: Op,
    inputs: &[Value],
    builder: &mut FunctionBuilder<'_>,
    pow: cranelift_codegen::ir::FuncRef,
) -> Value {
    match op {
        Op::Add => builder.ins().fadd(inputs[0], inputs[1]),
        Op::Sub => builder.ins().fsub(inputs[0], inputs[1]),
        Op::Mul => builder.ins().fmul(inputs[0], inputs[1]),
        Op::Div => builder.ins().fdiv(inputs[0], inputs[1]),
        Op::Neg => builder.ins().fneg(inputs[0]),
        Op::Sqrt => builder.ins().sqrt(inputs[0]),
        Op::InverseCubeSqrt => {
            let root = builder.ins().sqrt(inputs[1]);
            let denominator = builder.ins().fmul(inputs[1], root);
            builder.ins().fdiv(inputs[0], denominator)
        }
        Op::Fma => builder.ins().fma(inputs[0], inputs[1], inputs[2]),
        Op::Fmsub => {
            let negative = builder.ins().fneg(inputs[2]);
            builder.ins().fma(inputs[0], inputs[1], negative)
        }
        Op::Fnmadd => {
            let negative = builder.ins().fneg(inputs[0]);
            builder.ins().fma(negative, inputs[1], inputs[2])
        }
        Op::And => builder.ins().fmul(inputs[0], inputs[1]),
        Op::Or => builder.ins().fmax(inputs[0], inputs[1]),
        Op::Not | Op::Lt | Op::Le | Op::Gt | Op::Ge | Op::Eq | Op::Ne => {
            let ty = builder.func.dfg.value_type(inputs[0]);
            let zero = builder.ins().f64const(0.0);
            let one = builder.ins().f64const(1.0);
            let (zero, one) = if ty == types::F64X2 {
                (builder.ins().splat(ty, zero), builder.ins().splat(ty, one))
            } else {
                (zero, one)
            };
            let condition = match op {
                Op::Not | Op::Eq => FloatCC::Equal,
                Op::Ne => FloatCC::NotEqual,
                Op::Lt => FloatCC::LessThan,
                Op::Le => FloatCC::LessThanOrEqual,
                Op::Gt => FloatCC::GreaterThan,
                Op::Ge => FloatCC::GreaterThanOrEqual,
                _ => unreachable!(),
            };
            let right = if op == Op::Not { zero } else { inputs[1] };
            let mask = builder.ins().fcmp(condition, inputs[0], right);
            if ty == types::F64X2 {
                let mask = builder.ins().bitcast(ty, MemFlags::new(), mask);
                builder.ins().bitselect(mask, one, zero)
            } else {
                builder.ins().select(mask, one, zero)
            }
        }
        Op::RefinedInverseCube => {
            let ty = builder.func.dfg.value_type(inputs[1]);
            let vector = ty == types::F64X2;
            let q = inputs[1];
            let narrow = if vector {
                builder.ins().fvdemote(q)
            } else {
                builder.ins().fdemote(types::F32, q)
            };
            let root = builder.ins().sqrt(narrow);
            let one = builder.ins().f32const(1.0);
            let one = if vector {
                builder.ins().splat(types::F32X4, one)
            } else {
                one
            };
            let seed = builder.ins().fdiv(one, root);
            let mut seed = if vector {
                builder.ins().fvpromote_low(seed)
            } else {
                builder.ins().fpromote(types::F64, seed)
            };
            let half = builder.ins().f64const(0.5);
            let three_halves = builder.ins().f64const(1.5);
            let half = if vector {
                builder.ins().splat(ty, half)
            } else {
                half
            };
            let three_halves = if vector {
                builder.ins().splat(ty, three_halves)
            } else {
                three_halves
            };
            let half_q = builder.ins().fmul(half, q);
            for _ in 0..2 {
                let factor = builder.ins().fmul(half_q, seed);
                let factor = builder.ins().fmul(factor, seed);
                let correction = builder.ins().fsub(three_halves, factor);
                seed = builder.ins().fmul(seed, correction);
            }
            let square = builder.ins().fmul(seed, seed);
            let cube = builder.ins().fmul(seed, square);
            builder.ins().fmul(inputs[0], cube)
        }
        Op::Pow => {
            let call = builder.ins().call(pow, inputs);
            builder.inst_results(call)[0]
        }
    }
}

type NativeMap = unsafe extern "C" fn(*const f64, *mut f64);
type NativeBitmapMap = unsafe extern "C" fn(*const f64) -> u64;

enum MapFunction {
    Numeric(NativeMap),
    Bitmap(NativeBitmapMap),
}

/// Keep predicates as integer masks inside a native map. The public numeric
/// observation ABI still returns canonical 0.0/1.0 Boolean values. Rebuilding
/// just the Boolean DAG lets Cranelift remove the unused floating-point Boolean
/// conversions emitted by the shared numeric instruction lowering.
#[allow(clippy::too_many_arguments)]
fn boolean_mask(
    operand: Operand,
    program: &FusedProgram,
    simd: bool,
    emitted: &BTreeSet<usize>,
    registers: &[Value],
    packed: &mut PackedValues,
    origins: &LaneOrigins,
    masks: &mut BTreeMap<usize, Value>,
    builder: &mut FunctionBuilder<'_>,
) -> Value {
    let Operand::Register(register) = operand else {
        let Operand::Constant(value) = operand else {
            unreachable!()
        };
        return builder
            .ins()
            .iconst(types::I64, if value == 0.0 { 0 } else { -1 });
    };
    if let Some(mask) = masks.get(&register) {
        return *mask;
    }
    let instruction = &program.instructions[register - program.initial.len()];
    let mask = match instruction.op {
        Op::And | Op::Or | Op::Not => {
            let left = boolean_mask(
                instruction.inputs[0],
                program,
                simd,
                emitted,
                registers,
                packed,
                origins,
                masks,
                builder,
            );
            if instruction.op == Op::Not {
                builder.ins().bnot(left)
            } else {
                let right = boolean_mask(
                    instruction.inputs[1],
                    program,
                    simd,
                    emitted,
                    registers,
                    packed,
                    origins,
                    masks,
                    builder,
                );
                if instruction.op == Op::And {
                    builder.ins().band(left, right)
                } else {
                    builder.ins().bor(left, right)
                }
            }
        }
        Op::Lt | Op::Le | Op::Gt | Op::Ge | Op::Eq | Op::Ne => {
            let condition = match instruction.op {
                Op::Lt => FloatCC::LessThan,
                Op::Le => FloatCC::LessThanOrEqual,
                Op::Gt => FloatCC::GreaterThan,
                Op::Ge => FloatCC::GreaterThanOrEqual,
                Op::Eq => FloatCC::Equal,
                Op::Ne => FloatCC::NotEqual,
                _ => unreachable!(),
            };
            let index = register - program.initial.len();
            let first = if program.simd_pairs.contains(&index) {
                Some(index)
            } else {
                index
                    .checked_sub(1)
                    .filter(|i| program.simd_pairs.contains(i))
            };
            let available = |operand: &Operand| match operand {
                Operand::Register(register) if *register >= program.initial.len() => {
                    emitted.contains(&(register - program.initial.len()))
                }
                _ => true,
            };
            if simd
                && let Some(first) = first
                && program.instructions[first].inputs.iter().all(available)
                && program.instructions[first + 1].inputs.iter().all(available)
            {
                let a = &program.instructions[first];
                let b = &program.instructions[first + 1];
                let args = a
                    .inputs
                    .iter()
                    .zip(&b.inputs)
                    .map(|(a, b)| packed_pair(*a, *b, registers, packed, origins, builder))
                    .collect::<Vec<_>>();
                let vector = builder.ins().fcmp(condition, args[0], args[1]);
                let first_register = program.initial.len() + first;
                masks.insert(first_register, builder.ins().extractlane(vector, 0));
                masks.insert(first_register + 1, builder.ins().extractlane(vector, 1));
                masks[&register]
            } else {
                let left = value(instruction.inputs[0], registers, builder);
                let right = value(instruction.inputs[1], registers, builder);
                let condition = builder.ins().fcmp(condition, left, right);
                let yes = builder.ins().iconst(types::I64, -1);
                let no = builder.ins().iconst(types::I64, 0);
                builder.ins().select(condition, yes, no)
            }
        }
        _ => unreachable!("admitted Boolean output must have a Boolean producer"),
    };
    masks.insert(register, mask);
    mask
}

/// A native reset/iterate/observe map, with immutable f64 input parameters.
pub struct MapKernel {
    _module: JITModule,
    map: MapFunction,
    inputs: usize,
    outputs: usize,
}

impl MapKernel {
    pub fn compile(
        program: &FusedProgram,
        stop: &str,
        output: &str,
        simd: bool,
    ) -> Result<Self, String> {
        Self::compile_internal(program, stop, output, simd, false)
    }

    /// Return up to 64 Boolean output lanes through a single native register.
    pub fn compile_bitmap(
        program: &FusedProgram,
        stop: &str,
        output: &str,
        simd: bool,
    ) -> Result<Self, String> {
        Self::compile_internal(program, stop, output, simd, true)
    }

    fn compile_internal(
        program: &FusedProgram,
        stop: &str,
        output: &str,
        simd: bool,
        bitmap: bool,
    ) -> Result<Self, String> {
        let mut jit = JITBuilder::with_flags(&[("opt_level", "speed")], default_libcall_names())
            .map_err(error)?;
        jit.symbol("mech_fused_pow", mech_fused_pow as *const u8);
        let mut module = JITModule::new(jit);
        let function = define_map(program, &mut module, stop, output, simd, bitmap)?;
        module.finalize_definitions().map_err(error)?;
        let address = module.get_finalized_function(function);
        // SAFETY: define_map emits the selected numeric or packed ABI. The JITModule
        // owns the executable memory throughout every checked slice call.
        let map = unsafe {
            if bitmap {
                MapFunction::Bitmap(std::mem::transmute::<*const u8, NativeBitmapMap>(address))
            } else {
                MapFunction::Numeric(std::mem::transmute::<*const u8, NativeMap>(address))
            }
        };
        Ok(Self {
            _module: module,
            map,
            inputs: program
                .inputs
                .iter()
                .map(|input| input.rows * input.columns)
                .sum(),
            outputs: program.output_len(output)?,
        })
    }

    pub fn evaluate(&self, input: &[f64]) -> Result<Vec<f64>, String> {
        if input.len() != self.inputs {
            return Err("native map input length mismatch".into());
        }
        let mut output = vec![0.0; self.outputs];
        // SAFETY: The exact admitted input/output lengths bound all constant
        // accesses, and generated code writes only the exclusive output slice.
        unsafe {
            match self.map {
                MapFunction::Numeric(map) => map(input.as_ptr(), output.as_mut_ptr()),
                MapFunction::Bitmap(map) => {
                    let bits = map(input.as_ptr());
                    for (i, value) in output.iter_mut().enumerate() {
                        *value = ((bits >> (self.outputs - 1 - i)) & 1) as f64;
                    }
                }
            }
        }
        Ok(output)
    }
}

pub(super) fn compile_map_object(
    program: &FusedProgram,
    stop: &str,
    output: &str,
    simd: bool,
    bitmap: bool,
) -> Result<Vec<u8>, String> {
    crate::batched::aot::emit_native_object("mech_fused_map", |module| {
        define_map(program, module, stop, output, simd, bitmap)
            .map_err(crate::BatchedExecutionError::Native)?;
        Ok(())
    })
    .map_err(error)
}

fn define_map<M: Module>(
    program: &FusedProgram,
    module: &mut M,
    stop: &str,
    output: &str,
    simd: bool,
    bitmap: bool,
) -> Result<FuncId, String> {
    if program.output_len(stop)? != 1 || !program.output_is_boolean(stop) {
        return Err("native map termination must be a scalar Boolean output".into());
    }
    let stop_roots = &program.outputs[stop].values;
    let output_roots = &program
        .outputs
        .get(output)
        .ok_or("unknown native map output")?
        .values;
    if bitmap && (!program.output_is_boolean(output) || !(1..=64).contains(&output_roots.len())) {
        return Err("native bitmap map requires 1 through 64 Boolean output lanes".into());
    }
    let pointer = module.target_config().pointer_type();
    let mut signature = module.make_signature();
    signature.params.push(AbiParam::new(pointer));
    if bitmap {
        signature.returns.push(AbiParam::new(types::I64));
    } else {
        signature.params.push(AbiParam::new(pointer));
    }
    let function = module
        .declare_function(
            if bitmap {
                "mech_map_bits"
            } else {
                "mech_map_batch"
            },
            Linkage::Export,
            &signature,
        )
        .map_err(error)?;
    let mut pow_signature = module.make_signature();
    pow_signature
        .params
        .extend([AbiParam::new(types::F64), AbiParam::new(types::F64)]);
    pow_signature.returns.push(AbiParam::new(types::F64));
    let pow = module
        .declare_function("mech_fused_pow", Linkage::Import, &pow_signature)
        .map_err(error)?;
    let state_groups = if simd {
        simd_state_groups(program)
    } else {
        program
            .states
            .iter()
            .zip(&program.state_shapes)
            .flat_map(|((_, offset, _), (rows, columns))| {
                (0..rows * columns).map(move |i| vec![offset + i])
            })
            .collect::<Vec<_>>()
    };
    let update_roots = state_groups
        .iter()
        .flatten()
        .map(|i| program.updates[*i])
        .collect::<Vec<_>>();
    let mut context = module.make_context();
    context.func.signature = signature;
    let mut function_context = FunctionBuilderContext::new();
    {
        let mut builder = FunctionBuilder::new(&mut context.func, &mut function_context);
        let entry = builder.create_block();
        let header = builder.create_block();
        let body = builder.create_block();
        let exit = builder.create_block();
        builder.append_block_params_for_function_params(entry);
        for chunk in &state_groups {
            builder.append_block_param(
                header,
                if chunk.len() == 2 {
                    types::F64X2
                } else {
                    types::F64
                },
            );
        }
        builder.switch_to_block(entry);
        let input_pointer = builder.block_params(entry)[0];
        let output_pointer = (!bitmap).then(|| builder.block_params(entry)[1]);
        let zero = builder.ins().f64const(0.0);
        let mut registers = vec![zero; program.initial.len() + program.instructions.len()];
        let mut input_offset = 0;
        for input in &program.inputs {
            for i in input.offset..input.offset + input.rows * input.columns {
                registers[i] =
                    builder
                        .ins()
                        .load(types::F64, MemFlags::new(), input_pointer, input_offset);
                input_offset += 8;
            }
        }
        let mut packed = BTreeMap::new();
        let mut origins = BTreeMap::new();
        let initial_values = state_groups
            .iter()
            .map(|chunk| {
                if chunk.len() == 2 {
                    packed_pair(
                        Operand::Constant(program.initial[chunk[0]]),
                        Operand::Constant(program.initial[chunk[1]]),
                        &registers,
                        &mut packed,
                        &origins,
                        &mut builder,
                    )
                } else {
                    builder.ins().f64const(program.initial[chunk[0]])
                }
            })
            .collect::<Vec<_>>();
        let pow_ref = module.declare_func_in_func(pow, builder.func);
        let initial_args = initial_values
            .iter()
            .copied()
            .map(Into::into)
            .collect::<Vec<_>>();
        builder.ins().jump(header, &initial_args);
        builder.seal_block(entry);
        builder.switch_to_block(header);
        let state_values = builder.block_params(header).to_vec();
        packed.clear();
        origins.clear();
        for (state, chunk) in state_values.iter().zip(&state_groups) {
            if chunk.len() == 2 {
                for (lane, index) in chunk.iter().enumerate() {
                    registers[*index] = builder.ins().extractlane(*state, lane as u8);
                    origins.insert(*index, (*state, lane as u8));
                }
                packed.insert(
                    (
                        operand_key(Operand::Register(chunk[0])),
                        operand_key(Operand::Register(chunk[1])),
                    ),
                    *state,
                );
            } else {
                registers[chunk[0]] = *state;
            }
        }
        let mut emitted = BTreeSet::new();
        emit_instructions(
            program,
            stop_roots,
            simd,
            &mut registers,
            &mut packed,
            &mut origins,
            &mut emitted,
            &mut builder,
            pow_ref,
        );
        let mut masks = BTreeMap::new();
        let stop_value = boolean_mask(
            stop_roots[0],
            program,
            simd,
            &emitted,
            &registers,
            &mut packed,
            &origins,
            &mut masks,
            &mut builder,
        );
        let done = builder.ins().icmp_imm(IntCC::NotEqual, stop_value, 0);
        builder.ins().brif(done, exit, &[], body, &[]);
        let header_registers = registers.clone();
        let header_packed = packed.clone();
        let header_origins = origins.clone();
        let header_emitted = emitted.clone();
        builder.switch_to_block(body);
        builder.seal_block(body);
        emit_instructions(
            program,
            &update_roots,
            simd,
            &mut registers,
            &mut packed,
            &mut origins,
            &mut emitted,
            &mut builder,
            pow_ref,
        );
        let next = state_groups
            .iter()
            .map(|chunk| {
                if chunk.len() == 2 {
                    packed_pair(
                        program.updates[chunk[0]],
                        program.updates[chunk[1]],
                        &registers,
                        &mut packed,
                        &origins,
                        &mut builder,
                    )
                } else {
                    value(program.updates[chunk[0]], &registers, &mut builder)
                }
            })
            .collect::<Vec<_>>();
        builder.ins().jump(
            header,
            &next.iter().copied().map(Into::into).collect::<Vec<_>>(),
        );
        builder.seal_block(header);
        builder.switch_to_block(exit);
        builder.seal_block(exit);
        // Exit is reached directly from the header. Body-only values do not
        // dominate it, so continue output lowering from the header's environment.
        registers = header_registers;
        packed = header_packed;
        origins = header_origins;
        emitted = header_emitted;
        emit_instructions(
            program,
            output_roots,
            simd,
            &mut registers,
            &mut packed,
            &mut origins,
            &mut emitted,
            &mut builder,
            pow_ref,
        );
        let mut bits = builder.ins().iconst(types::I64, 0);
        for (i, operand) in output_roots.iter().enumerate() {
            let result = if program.output_is_boolean(output) {
                let mask = boolean_mask(
                    *operand,
                    program,
                    simd,
                    &emitted,
                    &registers,
                    &mut packed,
                    &origins,
                    &mut masks,
                    &mut builder,
                );
                if bitmap {
                    let bit = builder
                        .ins()
                        .band_imm(mask, 1_i64 << (output_roots.len() - 1 - i));
                    bits = builder.ins().bor(bits, bit);
                    continue;
                }
                let bit = builder.ins().band_imm(mask, 1);
                builder.ins().fcvt_from_uint(types::F64, bit)
            } else {
                value(*operand, &registers, &mut builder)
            };
            builder.ins().store(
                MemFlags::new(),
                result,
                output_pointer.unwrap(),
                (i * 8) as i32,
            );
        }
        builder.ins().return_(if bitmap {
            std::slice::from_ref(&bits)
        } else {
            &[]
        });
        builder.finalize();
    }
    module
        .define_function(function, &mut context)
        .map_err(|error| format!("{error:?}"))?;
    Ok(function)
}

pub(super) fn compile_grid_object(
    program: &FusedProgram,
    stop: &str,
    output: &str,
    simd: bool,
    width: usize,
    column: usize,
    row: usize,
) -> Result<Vec<u8>, String> {
    crate::batched::aot::emit_native_object("mech_native_grid", |module| {
        define_grid(program,module,stop,output,simd,width,column,row).map_err(crate::BatchedExecutionError::Native)?;
        Ok(())
    }).map_err(error)
}

fn define_grid<M:Module>(program:&FusedProgram,module:&mut M,stop:&str,output:&str,simd:bool,width:usize,column:usize,row:usize)->Result<FuncId,String>{
        let map = define_map(program, module, stop, output, simd, true)
            .map_err(error)?;
        let lanes = program
            .output_len(output)
            .map_err(error)?;
        let input_count = program
            .inputs
            .iter()
            .map(|i| i.rows * i.columns)
            .sum::<usize>();
        let column_end=column.checked_add(lanes).ok_or("grid input layout overflow")?;
        if input_count!=lanes+2 || width>=input_count || row>=input_count || column_end>input_count || width==row || (column..column_end).contains(&width) || (column..column_end).contains(&row) {
            return Err("grid inputs must be disjoint width, row and column lanes in the packed input ABI".into());
        }
        let ptr = module.target_config().pointer_type();
        let mut ctx = module.make_context();
        ctx.func.signature.params = vec![AbiParam::new(ptr); 3];
        let id = module
            .declare_function("mech_grid_row", Linkage::Export, &ctx.func.signature)
            .map_err(error)?;
        let map = module.declare_func_in_func(map, &mut ctx.func);
        let mut fbctx = FunctionBuilderContext::new();
        let mut b = FunctionBuilder::new(&mut ctx.func, &mut fbctx);
        let entry = b.create_block();
        let body = b.create_block();
        let exit = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.append_block_param(body, ptr);
        b.switch_to_block(entry);
        let p = b.block_params(entry).to_vec();
        let slot = b.create_sized_stack_slot(cranelift_codegen::ir::StackSlotData::new(
            cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
            (input_count * 8) as u32,
            3,
        ));
        let inputs = b.ins().stack_addr(ptr, slot, 0);
        let wf = b.ins().fcvt_from_uint(types::F64, p[0]);
        let rf = b.ins().fcvt_from_uint(types::F64, p[1]);
        b.ins()
            .store(MemFlags::new(), wf, inputs, (width * 8) as i32);
        b.ins().store(MemFlags::new(), rf, inputs, (row * 8) as i32);
        let zero = b.ins().iconst(ptr, 0);
        let more = b.ins().icmp_imm(IntCC::UnsignedGreaterThan, p[0], 0);
        b.ins().brif(more, body, &[zero.into()], exit, &[]);
        b.switch_to_block(body);
        let x = b.block_params(body)[0];
        for lane in 0..lanes {
            let xi = b.ins().iadd_imm(x, lane as i64);
            let xf = b.ins().fcvt_from_uint(types::F64, xi);
            b.ins()
                .store(MemFlags::new(), xf, inputs, ((column + lane) * 8) as i32);
        }
        let call = b.ins().call(map, &[inputs]);
        let bits = b.inst_results(call)[0];
        if lanes == 8 {
            let remaining = b.ins().isub(p[0], x);
            let tail = b.ins().icmp_imm(IntCC::UnsignedLessThan, remaining, 8);
            let eight = b.ins().iconst(ptr, 8);
            let shift = b.ins().isub(eight, remaining);
            let shift = b.ins().select(tail, shift, zero);
            let mask = b.ins().iconst(types::I64, 255);
            let mask = b.ins().ishl(mask, shift);
            let bits = b.ins().band(bits, mask);
            let byte = b.ins().ireduce(types::I8, bits);
            let byte_index = b.ins().ushr_imm(x, 3);
            let address = b.ins().iadd(p[2], byte_index);
            b.ins().store(MemFlags::new(), byte, address, 0);
        } else {
            for lane in 0..lanes {
                let index = b.ins().iadd_imm(x, lane as i64);
                let valid = b.ins().icmp(IntCC::UnsignedLessThan, index, p[0]);
                let store = b.create_block();
                let next = b.create_block();
                b.ins().brif(valid, store, &[], next, &[]);
                b.switch_to_block(store);
                let value = b.ins().ushr_imm(bits, (lanes - lane - 1) as i64);
                let value = b.ins().band_imm(value, 1);
                let bit_offset = b.ins().band_imm(index, 7);
                let seven = b.ins().iconst(ptr, 7);
                let shift = b.ins().isub(seven, bit_offset);
                let value = b.ins().ishl(value, shift);
                let value = b.ins().ireduce(types::I8, value);
                let byte_index = b.ins().ushr_imm(index, 3);
                let addr = b.ins().iadd(p[2], byte_index);
                let old = b.ins().load(types::I8, MemFlags::new(), addr, 0);
                let value = b.ins().bor(old, value);
                b.ins().store(MemFlags::new(), value, addr, 0);
                b.ins().jump(next, &[]);
                b.switch_to_block(next);
            }
        }
        let next = b.ins().iadd_imm(x, lanes as i64);
        let more = b.ins().icmp(IntCC::UnsignedLessThan, next, p[0]);
        b.ins().brif(more, body, &[next.into()], exit, &[]);
        b.switch_to_block(exit);
        b.ins().return_(&[]);
        b.seal_all_blocks();
        b.finalize();
        module
            .define_function(id, &mut ctx)
            .map_err(error)?;
        Ok(id)
}

/// Source-defined grid row traversal, emitted identically for JIT and AOT.
pub struct GridKernel {
    _module:JITModule,
    row:unsafe extern "C" fn(u64,u64,*mut u8),
}
impl GridKernel {
    pub fn compile(program:&FusedProgram,stop:&str,output:&str,simd:bool,width:usize,column:usize,row:usize)->Result<Self,String>{
        let mut builder=JITBuilder::with_flags(&[("opt_level","speed")],default_libcall_names()).map_err(error)?;
        builder.symbol("mech_fused_pow",mech_fused_pow as *const u8);
        let mut module=JITModule::new(builder);
        let function=define_grid(program,&mut module,stop,output,simd,width,column,row)?;
        module.finalize_definitions().map_err(error)?;
        // SAFETY: define_grid emits this exact synchronous row ABI.
        let row=unsafe{std::mem::transmute::<*const u8,unsafe extern "C" fn(u64,u64,*mut u8)>(module.get_finalized_function(function))};
        Ok(Self{_module:module,row})
    }
    #[cfg(feature="native-build")]
    pub fn rows(&self,width:usize,height:usize)->Result<Vec<u8>,String>{
        use rayon::prelude::*;
        let stride=width.div_ceil(8);
        if stride==0{return Ok(vec![]);}
        let len=stride.checked_mul(height).filter(|&n|n<=isize::MAX as usize).ok_or("grid exceeds addressable storage")?;
        let mut bytes=vec![0;len];let address=self.row as usize;
        bytes.par_chunks_mut(stride).enumerate().for_each(|(row,output)|{
            // SAFETY: the completed module stays alive and immutable until all
            // scoped tasks join; this pure row ABI writes only its disjoint row.
            let run=unsafe{std::mem::transmute::<usize,unsafe extern "C" fn(u64,u64,*mut u8)>(address)};
            unsafe{run(width as u64,row as u64,output.as_mut_ptr())};
        });Ok(bytes)
    }
    pub fn row(&self,width:usize,index:usize,output:&mut[u8])->Result<(),String>{
        if output.len()!=width.div_ceil(8){return Err("grid row output size mismatch".into());}
        output.fill(0);
        // SAFETY: exact output size bounds every source-derived packed write.
        unsafe{(self.row)(width as u64,index as u64,output.as_mut_ptr())};Ok(())
    }
}
