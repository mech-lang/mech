use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    fs, mem,
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
};

use cranelift_codegen::{
    ir::{
        AbiParam, InstBuilder, MemFlags, StackSlotData, StackSlotKind, Type, UserFuncName, Value,
        condcodes::{FloatCC, IntCC},
        types,
    },
    settings::{self, Configurable},
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{FuncId, Linkage, Module, default_libcall_names};
use cranelift_object::{ObjectBuilder, ObjectModule};
use mech_core::CellSlotId;

use super::{
    BatchedExecutionError, BatchedFaultRecorder, BatchedIntegrityFault, BinaryOperation,
    ComparisonOperation, ElementwiseOperation, FixedShapeKernel, LogicOperation, ScalarComputation,
    ScalarInstruction, ScalarOperand, ScalarPredicate, UnaryOperation,
    aot::{
        aot_error, artifact_key, dynamic_library_extension, link_dynamic_library, lock_cache_entry,
        object_extension,
    },
};

const SIMD_LANES: usize = 4;
const SIMD_AOT_ENTRY_POINT: &[u8] = b"mech_fixed_numeric_simd_turn\0";

type NativeSimdAotTurn = unsafe extern "C" fn(
    input_pointers: *const *const f32,
    state_pointers: *const *const f32,
    next_state_pointers: *const *mut f32,
    groups: usize,
) -> u64;

struct SimdAotKernel {
    _library: libloading::Library,
    turn: NativeSimdAotTurn,
    path: PathBuf,
    sha256: String,
}

/// A reusable four-lane Cranelift AOT library for a fixed-shape Mech region.
#[derive(Clone)]
pub struct BatchedAotSimdCpuArtifact {
    program: Arc<FixedShapeKernel>,
    kernel: Arc<SimdAotKernel>,
}

/// A resident AOT session with state packed in four-instance SIMD groups.
pub struct BatchedAotSimdCpuSession {
    program: Arc<FixedShapeKernel>,
    kernel: Arc<SimdAotKernel>,
    inputs: BTreeMap<CellSlotId, Vec<f32>>,
    state: BTreeMap<CellSlotId, Vec<f32>>,
    packed_state: BTreeMap<CellSlotId, Vec<f32>>,
    packed_next_state: BTreeMap<CellSlotId, Vec<f32>>,
    input_pointers: Vec<*const f32>,
    state_pointers: Vec<*const f32>,
    next_state_pointers: Vec<*mut f32>,
    faults: BatchedFaultRecorder,
}

impl FixedShapeKernel {
    /// Emits or reuses a four-lane AOT library in the default AOT cache.
    pub fn compile_aot_simd_cpu(&self) -> Result<BatchedAotSimdCpuArtifact, BatchedExecutionError> {
        self.compile_aot_simd_cpu_to(default_simd_cache_dir())
    }

    /// Emits or reuses a four-lane AOT library under `directory`.
    pub fn compile_aot_simd_cpu_to(
        &self,
        directory: impl AsRef<Path>,
    ) -> Result<BatchedAotSimdCpuArtifact, BatchedExecutionError> {
        if self.instances as usize % SIMD_LANES != 0 {
            return Err(BatchedExecutionError::Native(format!(
                "SIMD AOT requires an instance count divisible by {SIMD_LANES}, found {}",
                self.instances
            )));
        }
        let path = emit_simd_aot_library(self, directory.as_ref())?;
        let kernel = load_simd_aot_library(path)?;
        Ok(BatchedAotSimdCpuArtifact {
            program: Arc::new(self.clone()),
            kernel: Arc::new(kernel),
        })
    }

    /// Creates a resident four-lane session from an emitted or cached library.
    pub fn prepare_aot_simd_cpu(
        &self,
        inputs: &BTreeMap<String, Vec<f32>>,
    ) -> Result<BatchedAotSimdCpuSession, BatchedExecutionError> {
        self.compile_aot_simd_cpu()?.prepare(inputs)
    }
}

impl BatchedAotSimdCpuArtifact {
    pub(crate) fn library_sha256(&self) -> &str {
        &self.kernel.sha256
    }
    pub fn path(&self) -> &Path {
        &self.kernel.path
    }

    pub fn prepare(
        &self,
        inputs: &BTreeMap<String, Vec<f32>>,
    ) -> Result<BatchedAotSimdCpuSession, BatchedExecutionError> {
        let inputs = self.program.expand_inputs(inputs)?;
        let inputs = self
            .program
            .inputs
            .iter()
            .map(|input| {
                (
                    input.slot,
                    pack_simd_instances(&inputs[&input.slot], input.shape.elements()),
                )
            })
            .collect::<BTreeMap<_, _>>();
        let state = self.program.initial_state();
        let packed_state = self
            .program
            .states
            .iter()
            .map(|descriptor| {
                (
                    descriptor.slot,
                    pack_simd_instances(&state[&descriptor.slot], descriptor.shape.elements()),
                )
            })
            .collect::<BTreeMap<_, _>>();
        let packed_next_state = packed_state
            .iter()
            .map(|(slot, values)| (*slot, vec![0.0; values.len()]))
            .collect();
        let input_pointers = self
            .program
            .inputs
            .iter()
            .map(|input| inputs[&input.slot].as_ptr())
            .collect();
        let mut session = BatchedAotSimdCpuSession {
            program: Arc::clone(&self.program),
            kernel: Arc::clone(&self.kernel),
            inputs,
            state,
            packed_state,
            packed_next_state,
            input_pointers,
            state_pointers: Vec::with_capacity(self.program.states.len()),
            next_state_pointers: Vec::with_capacity(self.program.states.len()),
            faults: BatchedFaultRecorder::default(),
        };
        session.refresh_state_pointers();
        Ok(session)
    }
}

impl BatchedAotSimdCpuSession {
    pub fn artifact_path(&self) -> &Path {
        &self.kernel.path
    }

    pub fn update_inputs(
        &mut self,
        updates: &BTreeMap<String, Vec<f32>>,
    ) -> Result<(), BatchedExecutionError> {
        // Validate and pack every replacement before modifying a buffer.
        // An error must leave both the inputs and their raw pointer table intact.
        let replacements = updates
            .iter()
            .map(|(name, values)| {
                let input = self
                    .program
                    .inputs
                    .iter()
                    .find(|input| input.name == *name)
                    .ok_or_else(|| BatchedExecutionError::MissingInput(name.clone()))?;
                let expanded = self.program.expand_input(input, values)?;
                Ok((
                    input.slot,
                    pack_simd_instances(&expanded, input.shape.elements()),
                ))
            })
            .collect::<Result<BTreeMap<_, _>, BatchedExecutionError>>()?;
        self.inputs.extend(replacements);
        self.input_pointers = self
            .program
            .inputs
            .iter()
            .map(|input| self.inputs[&input.slot].as_ptr())
            .collect();
        Ok(())
    }

    pub fn dispatch_turns(&mut self, turns: u32) -> Result<(), BatchedExecutionError> {
        self.dispatch_turns_inner(turns, true)
    }

    /// Benchmark resident execution without converting the final packed state
    /// to a host-facing layout. Publication and integrity checks still occur
    /// on every turn. Call `read_state` before inspecting the updated host state.
    #[cfg(feature = "benchmark-probes")]
    pub fn dispatch_turns_resident(&mut self, turns: u32) -> Result<(), BatchedExecutionError> {
        self.dispatch_turns_inner(turns, false)
    }

    /// Materialize the host-facing state after resident benchmark dispatch.
    #[cfg(feature = "benchmark-probes")]
    pub fn read_state(&mut self) -> &BTreeMap<CellSlotId, Vec<f32>> {
        self.unpack_state();
        &self.state
    }

    fn dispatch_turns_inner(
        &mut self,
        turns: u32,
        readback: bool,
    ) -> Result<(), BatchedExecutionError> {
        if turns == 0 {
            return Err(BatchedExecutionError::ZeroTurns);
        }
        for _ in 0..turns {
            let attempted_turn = self.faults.next_turn();
            self.refresh_state_pointers();
            // SAFETY: buffers use the four-lane layout expected by the saved
            // kernel and remain live throughout the bounded group loop.
            let packed_fault = unsafe {
                (self.kernel.turn)(
                    self.input_pointers.as_ptr(),
                    self.state_pointers.as_ptr(),
                    self.next_state_pointers.as_ptr(),
                    self.program.instances as usize / SIMD_LANES,
                )
            };
            if let Some(fault) = self
                .program
                .failed_packed_constraint(packed_fault, attempted_turn)
            {
                if readback {
                    self.unpack_state();
                }
                return Err(self.faults.record(fault));
            }
            mem::swap(&mut self.packed_state, &mut self.packed_next_state);
        }
        if readback {
            self.unpack_state();
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
            self.state_pointers
                .push(self.packed_state[&state.slot].as_ptr());
            self.next_state_pointers.push(
                self.packed_next_state
                    .get_mut(&state.slot)
                    .unwrap()
                    .as_mut_ptr(),
            );
        }
    }

    fn unpack_state(&mut self) {
        for descriptor in &self.program.states {
            unpack_simd_instances(
                &self.packed_state[&descriptor.slot],
                descriptor.shape.elements(),
                self.state.get_mut(&descriptor.slot).unwrap(),
            );
        }
    }
}

fn default_simd_cache_dir() -> PathBuf {
    std::env::var_os("MECH_AOT_CACHE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("mech-aot-v1"))
}

fn emit_simd_aot_library(
    program: &FixedShapeKernel,
    directory: &Path,
) -> Result<PathBuf, BatchedExecutionError> {
    fs::create_dir_all(directory).map_err(aot_error)?;
    let key = artifact_key(program, b"simd4-sincos-v1", include_bytes!("aot_math.c"));
    let library_path = directory.join(format!("mech-simd-{key}.{}", dynamic_library_extension()));
    let _cache_lock = lock_cache_entry(&library_path)?;
    if library_path.is_file() {
        return Ok(library_path);
    }

    let object_path = directory.join(format!("mech-simd-{key}.{}", object_extension()));
    let helper_path = directory.join(format!("mech-simd-math-{key}.{}", object_extension()));
    let mut flag_builder = settings::builder();
    flag_builder.set("opt_level", "speed").map_err(aot_error)?;
    flag_builder.set("is_pic", "true").map_err(aot_error)?;
    let isa_builder = cranelift_native::builder().map_err(aot_error)?;
    let isa = isa_builder
        .finish(settings::Flags::new(flag_builder))
        .map_err(aot_error)?;
    let builder = ObjectBuilder::new(isa, "mech_fixed_shape_simd_aot", default_libcall_names())
        .map_err(aot_error)?;
    let mut module = ObjectModule::new(builder);
    define_simd_native_turn(&mut module, program)?;
    let bytes = module.finish().emit().map_err(aot_error)?;
    fs::write(&object_path, bytes).map_err(aot_error)?;
    compile_math_helper(&helper_path)?;
    link_dynamic_library(&[&object_path, &helper_path], &library_path)?;
    Ok(library_path)
}

fn compile_math_helper(object_path: &Path) -> Result<(), BatchedExecutionError> {
    let compiler = std::env::var_os("CC").unwrap_or_else(|| OsString::from("cc"));
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/batched/aot_math.c");
    let output = Command::new(&compiler)
        .arg("-O3")
        .arg("-fPIC")
        .arg("-c")
        .arg(&source)
        .arg("-o")
        .arg(object_path)
        .output()
        .map_err(aot_error)?;
    if output.status.success() {
        Ok(())
    } else {
        Err(aot_error(format!(
            "{} failed while compiling {}: {}",
            Path::new(&compiler).display(),
            source.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        )))
    }
}

fn load_simd_aot_library(path: PathBuf) -> Result<SimdAotKernel, BatchedExecutionError> {
    use sha2::{Digest, Sha256};
    let sha256 = Sha256::digest(fs::read(&path).map_err(aot_error)?)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    // SAFETY: this process emitted the library for the current host target and
    // retains its handle for the lifetime of the copied entry-point pointer.
    let library = unsafe { libloading::Library::new(&path) }.map_err(aot_error)?;
    let turn = unsafe {
        *library
            .get::<NativeSimdAotTurn>(SIMD_AOT_ENTRY_POINT)
            .map_err(aot_error)?
    };
    Ok(SimdAotKernel {
        _library: library,
        turn,
        path,
        sha256,
    })
}

#[derive(Clone, Copy)]
struct SimdMathFunctions {
    sin: cranelift_codegen::ir::FuncRef,
    cos: cranelift_codegen::ir::FuncRef,
    sincos: cranelift_codegen::ir::FuncRef,
    atan2: cranelift_codegen::ir::FuncRef,
}

#[derive(Clone, Copy)]
enum SimdRegister {
    F32(Value),
    Bool(Value),
}

fn define_simd_native_turn<M: Module>(
    module: &mut M,
    program: &FixedShapeKernel,
) -> Result<FuncId, BatchedExecutionError> {
    let pointer_type = module.target_config().pointer_type();
    let unary_signature = {
        let mut signature = module.make_signature();
        signature.params.push(AbiParam::new(pointer_type));
        signature.params.push(AbiParam::new(pointer_type));
        signature
    };
    let sincos_signature = {
        let mut signature = module.make_signature();
        for _ in 0..3 {
            signature.params.push(AbiParam::new(pointer_type));
        }
        signature
    };
    let atan2_signature = sincos_signature.clone();
    let sin_id = module
        .declare_function("mech_aot_sinf_f32x4", Linkage::Import, &unary_signature)
        .map_err(aot_error)?;
    let cos_id = module
        .declare_function("mech_aot_cosf_f32x4", Linkage::Import, &unary_signature)
        .map_err(aot_error)?;
    let sincos_id = module
        .declare_function("mech_aot_sincos_f32x4", Linkage::Import, &sincos_signature)
        .map_err(aot_error)?;
    let atan2_id = module
        .declare_function("mech_aot_atan2_f32x4", Linkage::Import, &atan2_signature)
        .map_err(aot_error)?;

    let mut signature = module.make_signature();
    for _ in 0..4 {
        signature.params.push(AbiParam::new(pointer_type));
    }
    signature.returns.push(AbiParam::new(types::I64));
    let function_id = module
        .declare_function("mech_fixed_numeric_simd_turn", Linkage::Export, &signature)
        .map_err(aot_error)?;
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
        let groups = parameters[3];
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
        let constants = collect_constant_bits(program)
            .into_iter()
            .map(|bits| {
                let scalar = builder.ins().f32const(f32::from_bits(bits));
                (bits, builder.ins().splat(types::F32X4, scalar))
            })
            .collect::<BTreeMap<_, _>>();
        let zero = builder.ins().iconst(pointer_type, 0);
        builder.ins().jump(header, &[zero.into()]);

        builder.switch_to_block(header);
        let group = builder.block_params(header)[0];
        let has_group = builder.ins().icmp(IntCC::UnsignedLessThan, group, groups);
        builder.ins().brif(has_group, body, &[], exit, &[]);

        builder.switch_to_block(body);
        let functions = SimdMathFunctions {
            sin: module.declare_func_in_func(sin_id, builder.func),
            cos: module.declare_func_in_func(cos_id, builder.func),
            sincos: module.declare_func_in_func(sincos_id, builder.func),
            atan2: module.declare_func_in_func(atan2_id, builder.func),
        };
        let mut registers = vec![None; program.fixed_ir().register_count];
        for (index, input) in program.inputs.iter().enumerate() {
            let register_offset = program.register_offsets[&input.slot];
            for component in 0..input.shape.elements() {
                registers[register_offset + component] =
                    Some(SimdRegister::F32(load_simd_component(
                        &mut builder,
                        input_bases[index],
                        group,
                        input.shape.elements(),
                        component,
                        pointer_type,
                    )));
            }
        }
        for (index, state) in program.states.iter().enumerate() {
            let register_offset = program.register_offsets[&state.slot];
            for component in 0..state.shape.elements() {
                registers[register_offset + component] =
                    Some(SimdRegister::F32(load_simd_component(
                        &mut builder,
                        state_bases[index],
                        group,
                        state.shape.elements(),
                        component,
                        pointer_type,
                    )));
            }
        }

        let instructions = &program.fixed_ir().instructions;
        let mut paired_outputs = BTreeSet::new();
        for (instruction_index, instruction) in instructions.iter().enumerate() {
            if paired_outputs.remove(&instruction.output) {
                continue;
            }
            if let Some((partner_index, current_is_sin, operand)) =
                find_sincos_partner(instructions, instruction_index, &instruction.computation)
            {
                let operand = lower_simd_numeric_operand(operand, &registers, &constants)?;
                let (sin, cos) =
                    call_simd_sincos(&mut builder, functions.sincos, operand, pointer_type);
                let current = if current_is_sin { sin } else { cos };
                let partner = if current_is_sin { cos } else { sin };
                registers[instruction.output] = Some(SimdRegister::F32(current));
                registers[instructions[partner_index].output] = Some(SimdRegister::F32(partner));
                paired_outputs.insert(instructions[partner_index].output);
                continue;
            }
            registers[instruction.output] = Some(lower_simd_computation(
                &mut builder,
                &instruction.computation,
                &registers,
                functions,
                &constants,
                pointer_type,
            )?);
        }

        let mut check_block = body;
        for (constraint_index, constraint) in program.constraints.iter().enumerate() {
            if constraint_index != 0 {
                builder.switch_to_block(check_block);
            }
            let valid =
                lower_simd_predicate(&mut builder, &constraint.predicate, &registers, &constants)?;
            for lane in 0..SIMD_LANES {
                let next_check = builder.create_block();
                if lane != 0 {
                    builder.switch_to_block(check_block);
                }
                let lane_valid = builder.ins().extractlane(valid, lane as u8);
                let failed = builder.ins().icmp_imm(IntCC::Equal, lane_valid, 0);
                let instance = builder.ins().imul_imm(group, SIMD_LANES as i64);
                let instance = builder.ins().iadd_imm(instance, lane as i64);
                let code = builder
                    .ins()
                    .iconst(types::I32, (constraint_index + 1) as i64);
                builder.ins().brif(
                    failed,
                    fault,
                    &[instance.into(), code.into()],
                    next_check,
                    &[],
                );
                check_block = next_check;
            }
        }
        if !program.constraints.is_empty() {
            builder.switch_to_block(check_block);
        }
        for (index, state) in program.states.iter().enumerate() {
            for (component, source) in state.update.iter().enumerate() {
                let value = lower_simd_numeric_operand(*source, &registers, &constants)?;
                store_simd_component(
                    &mut builder,
                    next_state_bases[index],
                    group,
                    state.shape.elements(),
                    component,
                    pointer_type,
                    value,
                );
            }
        }
        builder.ins().jump(advance, &[]);

        builder.switch_to_block(advance);
        let next_group = builder.ins().iadd_imm(group, 1);
        builder.ins().jump(header, &[next_group.into()]);

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
        .map_err(aot_error)?;
    module.clear_context(&mut context);
    Ok(function_id)
}

fn lower_simd_computation(
    builder: &mut FunctionBuilder<'_>,
    computation: &ScalarComputation,
    registers: &[Option<SimdRegister>],
    functions: SimdMathFunctions,
    constants: &BTreeMap<u32, Value>,
    pointer_type: Type,
) -> Result<SimdRegister, BatchedExecutionError> {
    Ok(match computation {
        ScalarComputation::Copy(input) => lower_simd_operand(*input, registers, constants)?,
        ScalarComputation::Negate(input) => SimdRegister::F32(
            builder
                .ins()
                .fneg(lower_simd_numeric_operand(*input, registers, constants)?),
        ),
        ScalarComputation::Absolute(input) => SimdRegister::F32(
            builder
                .ins()
                .fabs(lower_simd_numeric_operand(*input, registers, constants)?),
        ),
        ScalarComputation::IsFinite(input) => SimdRegister::Bool(lower_simd_is_finite(
            builder,
            lower_simd_numeric_operand(*input, registers, constants)?,
        )),
        ScalarComputation::Compare {
            operation,
            left,
            right,
        } => SimdRegister::Bool(builder.ins().fcmp(
            float_condition(*operation),
            lower_simd_numeric_operand(*left, registers, constants)?,
            lower_simd_numeric_operand(*right, registers, constants)?,
        )),
        ScalarComputation::Logic { operation, inputs } => {
            let left = lower_simd_boolean_operand(inputs[0], registers, constants)?;
            SimdRegister::Bool(if *operation == LogicOperation::Not {
                builder.ins().bnot(left)
            } else {
                let right = lower_simd_boolean_operand(inputs[1], registers, constants)?;
                lower_logic(builder, *operation, left, right)
            })
        }
        ScalarComputation::Elementwise { operation, inputs } => {
            let values = inputs
                .iter()
                .map(|input| lower_simd_numeric_operand(*input, registers, constants))
                .collect::<Result<Vec<_>, _>>()?;
            SimdRegister::F32(match operation {
                ElementwiseOperation::Binary(operation) => match operation {
                    BinaryOperation::Add => builder.ins().fadd(values[0], values[1]),
                    BinaryOperation::Subtract => builder.ins().fsub(values[0], values[1]),
                    BinaryOperation::Multiply => builder.ins().fmul(values[0], values[1]),
                    BinaryOperation::Divide => builder.ins().fdiv(values[0], values[1]),
                },
                ElementwiseOperation::Unary(operation) => match operation {
                    UnaryOperation::Sin => {
                        call_simd_unary(builder, functions.sin, values[0], pointer_type)
                    }
                    UnaryOperation::Cos => {
                        call_simd_unary(builder, functions.cos, values[0], pointer_type)
                    }
                    UnaryOperation::Sqrt => builder.ins().sqrt(values[0]),
                    UnaryOperation::Ceil => builder.ins().ceil(values[0]),
                },
                ElementwiseOperation::Atan2 => {
                    call_simd_binary(builder, functions.atan2, values[0], values[1], pointer_type)
                }
                ElementwiseOperation::Identity => values[0],
            })
        }
        ScalarComputation::SumProducts(terms) => {
            let mut sum = constants[&0.0_f32.to_bits()];
            for (left, right) in terms {
                let left = lower_simd_numeric_operand(*left, registers, constants)?;
                let right = lower_simd_numeric_operand(*right, registers, constants)?;
                sum = builder.ins().fma(left, right, sum);
            }
            SimdRegister::F32(sum)
        }
    })
}

fn lower_simd_predicate(
    builder: &mut FunctionBuilder<'_>,
    predicate: &ScalarPredicate,
    registers: &[Option<SimdRegister>],
    constants: &BTreeMap<u32, Value>,
) -> Result<Value, BatchedExecutionError> {
    Ok(match predicate {
        ScalarPredicate::Value(operand) => {
            match lower_simd_operand(*operand, registers, constants)? {
                SimdRegister::Bool(value) => value,
                SimdRegister::F32(value) => {
                    builder
                        .ins()
                        .fcmp(FloatCC::NotEqual, value, constants[&0.0_f32.to_bits()])
                }
            }
        }
        ScalarPredicate::IsFinite(operand) => lower_simd_is_finite(
            builder,
            lower_simd_numeric_operand(*operand, registers, constants)?,
        ),
        ScalarPredicate::AbsoluteDifferenceWithin {
            left,
            right,
            tolerance,
        } => {
            let left = lower_simd_numeric_operand(*left, registers, constants)?;
            let right = lower_simd_numeric_operand(*right, registers, constants)?;
            let tolerance = lower_simd_numeric_operand(*tolerance, registers, constants)?;
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
        } => builder.ins().fcmp(
            float_condition(*operation),
            lower_simd_numeric_operand(*left, registers, constants)?,
            lower_simd_numeric_operand(*right, registers, constants)?,
        ),
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
                lower_logic(builder, *operation, left, right)
            }
        }
    })
}

fn lower_simd_operand(
    operand: ScalarOperand,
    registers: &[Option<SimdRegister>],
    constants: &BTreeMap<u32, Value>,
) -> Result<SimdRegister, BatchedExecutionError> {
    match operand {
        ScalarOperand::Register(register) => registers[register].ok_or_else(|| {
            BatchedExecutionError::Native(format!(
                "SIMD AOT lowering read register {register} before definition"
            ))
        }),
        ScalarOperand::Constant(value) => constants
            .get(&value.to_bits())
            .copied()
            .map(SimdRegister::F32)
            .ok_or_else(|| {
                BatchedExecutionError::Native(format!(
                    "SIMD AOT constant {value:?} was not hoisted"
                ))
            }),
    }
}

fn lower_simd_numeric_operand(
    operand: ScalarOperand,
    registers: &[Option<SimdRegister>],
    constants: &BTreeMap<u32, Value>,
) -> Result<Value, BatchedExecutionError> {
    match lower_simd_operand(operand, registers, constants)? {
        SimdRegister::F32(value) => Ok(value),
        SimdRegister::Bool(_) => Err(BatchedExecutionError::Native(
            "SIMD AOT numeric operation received a boolean operand".to_owned(),
        )),
    }
}

fn lower_simd_boolean_operand(
    operand: ScalarOperand,
    registers: &[Option<SimdRegister>],
    constants: &BTreeMap<u32, Value>,
) -> Result<Value, BatchedExecutionError> {
    match lower_simd_operand(operand, registers, constants)? {
        SimdRegister::Bool(value) => Ok(value),
        SimdRegister::F32(_) => Err(BatchedExecutionError::Native(
            "SIMD AOT boolean operation received a numeric operand".to_owned(),
        )),
    }
}

fn lower_simd_is_finite(builder: &mut FunctionBuilder<'_>, value: Value) -> Value {
    let absolute = builder.ins().fabs(value);
    let maximum = builder.ins().f32const(f32::MAX);
    let maximum = builder.ins().splat(types::F32X4, maximum);
    builder
        .ins()
        .fcmp(FloatCC::LessThanOrEqual, absolute, maximum)
}

fn lower_logic(
    builder: &mut FunctionBuilder<'_>,
    operation: LogicOperation,
    left: Value,
    right: Value,
) -> Value {
    match operation {
        LogicOperation::And => builder.ins().band(left, right),
        LogicOperation::Or => builder.ins().bor(left, right),
        LogicOperation::Xor => builder.ins().bxor(left, right),
        LogicOperation::Not => unreachable!(),
    }
}

fn float_condition(operation: ComparisonOperation) -> FloatCC {
    match operation {
        ComparisonOperation::Equal => FloatCC::Equal,
        ComparisonOperation::NotEqual => FloatCC::NotEqual,
        ComparisonOperation::Less => FloatCC::LessThan,
        ComparisonOperation::Greater => FloatCC::GreaterThan,
        ComparisonOperation::LessEqual => FloatCC::LessThanOrEqual,
        ComparisonOperation::GreaterEqual => FloatCC::GreaterThanOrEqual,
    }
}

fn call_simd_unary(
    builder: &mut FunctionBuilder<'_>,
    function: cranelift_codegen::ir::FuncRef,
    value: Value,
    pointer_type: Type,
) -> Value {
    let input_slot = simd_stack_slot(builder);
    let output_slot = simd_stack_slot(builder);
    let input = builder.ins().stack_addr(pointer_type, input_slot, 0);
    let output = builder.ins().stack_addr(pointer_type, output_slot, 0);
    builder.ins().store(MemFlags::trusted(), value, input, 0);
    builder.ins().call(function, &[input, output]);
    builder
        .ins()
        .load(types::F32X4, MemFlags::trusted(), output, 0)
}

fn call_simd_binary(
    builder: &mut FunctionBuilder<'_>,
    function: cranelift_codegen::ir::FuncRef,
    left: Value,
    right: Value,
    pointer_type: Type,
) -> Value {
    let left_slot = simd_stack_slot(builder);
    let right_slot = simd_stack_slot(builder);
    let output_slot = simd_stack_slot(builder);
    let left_pointer = builder.ins().stack_addr(pointer_type, left_slot, 0);
    let right_pointer = builder.ins().stack_addr(pointer_type, right_slot, 0);
    let output = builder.ins().stack_addr(pointer_type, output_slot, 0);
    builder
        .ins()
        .store(MemFlags::trusted(), left, left_pointer, 0);
    builder
        .ins()
        .store(MemFlags::trusted(), right, right_pointer, 0);
    builder
        .ins()
        .call(function, &[left_pointer, right_pointer, output]);
    builder
        .ins()
        .load(types::F32X4, MemFlags::trusted(), output, 0)
}

fn call_simd_sincos(
    builder: &mut FunctionBuilder<'_>,
    function: cranelift_codegen::ir::FuncRef,
    value: Value,
    pointer_type: Type,
) -> (Value, Value) {
    let input_slot = simd_stack_slot(builder);
    let sin_slot = simd_stack_slot(builder);
    let cos_slot = simd_stack_slot(builder);
    let input = builder.ins().stack_addr(pointer_type, input_slot, 0);
    let sin = builder.ins().stack_addr(pointer_type, sin_slot, 0);
    let cos = builder.ins().stack_addr(pointer_type, cos_slot, 0);
    builder.ins().store(MemFlags::trusted(), value, input, 0);
    builder.ins().call(function, &[input, sin, cos]);
    (
        builder
            .ins()
            .load(types::F32X4, MemFlags::trusted(), sin, 0),
        builder
            .ins()
            .load(types::F32X4, MemFlags::trusted(), cos, 0),
    )
}

fn simd_stack_slot(builder: &mut FunctionBuilder<'_>) -> cranelift_codegen::ir::StackSlot {
    builder.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 16, 4))
}

fn load_simd_component(
    builder: &mut FunctionBuilder<'_>,
    base: Value,
    group: Value,
    elements: usize,
    component: usize,
    pointer_type: Type,
) -> Value {
    let address = simd_component_address(builder, base, group, elements, component, pointer_type);
    builder
        .ins()
        .load(types::F32X4, MemFlags::trusted(), address, 0)
}

fn store_simd_component(
    builder: &mut FunctionBuilder<'_>,
    base: Value,
    group: Value,
    elements: usize,
    component: usize,
    pointer_type: Type,
    value: Value,
) {
    let address = simd_component_address(builder, base, group, elements, component, pointer_type);
    builder.ins().store(MemFlags::trusted(), value, address, 0);
}

fn simd_component_address(
    builder: &mut FunctionBuilder<'_>,
    base: Value,
    group: Value,
    elements: usize,
    component: usize,
    pointer_type: Type,
) -> Value {
    let group_bytes = elements
        .checked_mul(SIMD_LANES)
        .and_then(|value| value.checked_mul(types::F32.bytes() as usize))
        .unwrap();
    let component_bytes = component
        .checked_mul(SIMD_LANES)
        .and_then(|value| value.checked_mul(types::F32.bytes() as usize))
        .unwrap();
    let offset = builder
        .ins()
        .imul_imm(group, i64::try_from(group_bytes).unwrap());
    let offset = builder
        .ins()
        .iadd_imm(offset, i64::try_from(component_bytes).unwrap());
    debug_assert_eq!(builder.func.dfg.value_type(offset), pointer_type);
    builder.ins().iadd(base, offset)
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
    let partner_operation = if operation == UnaryOperation::Sin {
        UnaryOperation::Cos
    } else {
        UnaryOperation::Sin
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

fn collect_constant_bits(program: &FixedShapeKernel) -> BTreeSet<u32> {
    let mut constants = BTreeSet::from([0.0_f32.to_bits()]);
    for instruction in &program.fixed_ir().instructions {
        collect_computation_constants(&instruction.computation, &mut constants);
    }
    for constraint in &program.constraints {
        collect_predicate_constants(&constraint.predicate, &mut constants);
    }
    for state in &program.states {
        for source in &state.update {
            collect_operand_constant(*source, &mut constants);
        }
    }
    constants
}

fn collect_operand_constant(operand: ScalarOperand, constants: &mut BTreeSet<u32>) {
    if let ScalarOperand::Constant(value) = operand {
        constants.insert(value.to_bits());
    }
}

fn collect_computation_constants(computation: &ScalarComputation, constants: &mut BTreeSet<u32>) {
    match computation {
        ScalarComputation::Copy(input)
        | ScalarComputation::Negate(input)
        | ScalarComputation::Absolute(input)
        | ScalarComputation::IsFinite(input) => collect_operand_constant(*input, constants),
        ScalarComputation::Compare { left, right, .. } => {
            collect_operand_constant(*left, constants);
            collect_operand_constant(*right, constants);
        }
        ScalarComputation::Logic { inputs, .. } | ScalarComputation::Elementwise { inputs, .. } => {
            for input in inputs {
                collect_operand_constant(*input, constants);
            }
        }
        ScalarComputation::SumProducts(terms) => {
            for (left, right) in terms {
                collect_operand_constant(*left, constants);
                collect_operand_constant(*right, constants);
            }
        }
    }
}

fn collect_predicate_constants(predicate: &ScalarPredicate, constants: &mut BTreeSet<u32>) {
    match predicate {
        ScalarPredicate::Value(input) | ScalarPredicate::IsFinite(input) => {
            collect_operand_constant(*input, constants);
        }
        ScalarPredicate::AbsoluteDifferenceWithin {
            left,
            right,
            tolerance,
        } => {
            collect_operand_constant(*left, constants);
            collect_operand_constant(*right, constants);
            collect_operand_constant(*tolerance, constants);
        }
        ScalarPredicate::Compare { left, right, .. } => {
            collect_operand_constant(*left, constants);
            collect_operand_constant(*right, constants);
        }
        ScalarPredicate::All(inputs) | ScalarPredicate::Logic { inputs, .. } => {
            for input in inputs {
                collect_predicate_constants(input, constants);
            }
        }
    }
}

fn pack_simd_instances(values: &[f32], elements: usize) -> Vec<f32> {
    let instances = values.len() / elements;
    let mut packed = vec![0.0; values.len()];
    for group in 0..instances / SIMD_LANES {
        for component in 0..elements {
            for lane in 0..SIMD_LANES {
                let instance = group * SIMD_LANES + lane;
                packed[(group * elements + component) * SIMD_LANES + lane] =
                    values[instance * elements + component];
            }
        }
    }
    packed
}

fn unpack_simd_instances(packed: &[f32], elements: usize, values: &mut [f32]) {
    let instances = values.len() / elements;
    for group in 0..instances / SIMD_LANES {
        for component in 0..elements {
            for lane in 0..SIMD_LANES {
                let instance = group * SIMD_LANES + lane;
                values[instance * elements + component] =
                    packed[(group * elements + component) * SIMD_LANES + lane];
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simd_aot_input_updates_are_atomic() {
        let (program, inputs) = super::super::aot::tests::input_update_test_kernel();
        let mut session = program.prepare_aot_simd_cpu(&inputs).unwrap();
        let original_inputs = session.inputs.clone();
        let original_pointers = session.input_pointers.clone();
        for (name, values) in [("zzz", vec![3.0]), ("z", vec![3.0; 3])] {
            let updates =
                BTreeMap::from([("a".to_owned(), vec![9.0; 4]), (name.to_owned(), values)]);
            assert!(session.update_inputs(&updates).is_err());
            assert_eq!(session.inputs, original_inputs);
            assert_eq!(session.input_pointers, original_pointers);
        }
        session.dispatch_turns(1).unwrap();
        let mut reference = program.prepare_cpu(&inputs).unwrap();
        reference.dispatch_turns(1).unwrap();
        assert_eq!(session.state(), reference.state());

        let updates = BTreeMap::from([
            ("a".to_owned(), vec![7.0]),
            ("z".to_owned(), vec![3.0, 4.0, 5.0, 6.0]),
        ]);
        session.update_inputs(&updates).unwrap();
        reference.update_inputs(&updates).unwrap();
        session.dispatch_turns(1).unwrap();
        reference.dispatch_turns(1).unwrap();
        assert_eq!(session.state(), reference.state());
    }
}
