use super::kernel_ir::{
    BinaryOperation, ElementType, Instruction, KernelIr, Operation, Shape, Source, UnaryOperation,
    ValueId,
};
use super::mlir_gpu_codegen::{only_operand, referenced_values, require_uniform, validate_kernel};
use std::collections::BTreeMap;
use std::fmt::Write as _;

const THREADS_PER_WORKGROUP: usize = 256;

#[derive(Clone, Copy)]
enum InitializationMode {
    Device,
    Host,
}

struct SpirvEmitter {
    body: String,
    next_value: usize,
    constants: BTreeMap<(u8, u64), String>,
    state_pointer_type: String,
}

impl SpirvEmitter {
    fn new(state_len: usize) -> Self {
        Self {
            body: String::new(),
            next_value: 0,
            constants: BTreeMap::new(),
            state_pointer_type: state_pointer_type(state_len),
        }
    }

    fn operation(&mut self, operation: impl AsRef<str>) -> String {
        let value = format!("%v{}", self.next_value);
        self.next_value += 1;
        writeln!(self.body, "    {value} = {}", operation.as_ref()).unwrap();
        value
    }

    fn f32_constant(&mut self, f64_bits: u64) -> String {
        let bits = (f64::from_bits(f64_bits) as f32).to_bits();
        if let Some(value) = self.constants.get(&(0, bits as u64)) {
            return value.clone();
        }
        let value = self.operation(format!("spirv.Constant 0x{bits:08X} : f32"));
        self.constants.insert((0, bits as u64), value.clone());
        value
    }

    fn i32_constant(&mut self, number: usize) -> Result<String, String> {
        let number = i32::try_from(number)
            .map_err(|_| format!("SPIR-V GPU index {number} exceeds the i32 address range"))?;
        if let Some(value) = self.constants.get(&(1, number as u64)) {
            return Ok(value.clone());
        }
        let value = self.operation(format!("spirv.Constant {number} : i32"));
        self.constants.insert((1, number as u64), value.clone());
        Ok(value)
    }

    fn offset_index(&mut self, base: &str, offset: usize) -> Result<String, String> {
        if offset == 0 {
            Ok(base.to_owned())
        } else {
            let offset = self.i32_constant(offset)?;
            Ok(self.operation(format!("spirv.IAdd {base}, {offset} : i32")))
        }
    }

    fn element_pointer(&mut self, index: &str) -> Result<String, String> {
        let zero = self.i32_constant(0)?;
        Ok(self.operation(format!(
            "spirv.AccessChain %state[{zero}, {index}] : {}, i32, i32 -> !spirv.ptr<f32, StorageBuffer>",
            self.state_pointer_type
        )))
    }

    fn load_at(&mut self, index: &str) -> Result<String, String> {
        let pointer = self.element_pointer(index)?;
        Ok(self.operation(format!("spirv.Load \"StorageBuffer\" {pointer} : f32")))
    }

    fn store_at(&mut self, value: &str, index: &str) -> Result<(), String> {
        let pointer = self.element_pointer(index)?;
        writeln!(
            self.body,
            "    spirv.Store \"StorageBuffer\" {pointer}, {value} : f32"
        )
        .unwrap();
        Ok(())
    }

    fn unary(&mut self, operation: &str, value: &str) -> String {
        self.operation(format!("{operation} {value} : f32"))
    }

    fn binary(&mut self, operation: &str, left: &str, right: &str) -> String {
        self.operation(format!("{operation} {left}, {right} : f32"))
    }
}

pub(super) fn emit_spirv_mlir_f32(kernel: &KernelIr) -> Result<String, String> {
    emit_spirv_mlir_f32_with_initialization(kernel, InitializationMode::Device)
}

pub(super) fn emit_spirv_mlir_f32_host_initialized(kernel: &KernelIr) -> Result<String, String> {
    emit_spirv_mlir_f32_with_initialization(kernel, InitializationMode::Host)
}

fn emit_spirv_mlir_f32_with_initialization(
    kernel: &KernelIr,
    initialization: InitializationMode,
) -> Result<String, String> {
    let batch_len = validate_kernel(kernel)?;
    i32::try_from(kernel.state_len)
        .map_err(|_| "SPIR-V GPU state exceeds the i32 address range".to_owned())?;
    i32::try_from(batch_len)
        .map_err(|_| "SPIR-V GPU batch exceeds the i32 invocation range".to_owned())?;
    if matches!(initialization, InitializationMode::Device)
        && kernel.states.iter().any(|state| {
            kernel.value(state.value).ty.shape == Shape::SCALAR
                || !state
                    .initial_elements
                    .windows(2)
                    .all(|pair| pair[0] == pair[1])
        })
    {
        return Err(
            "SPIR-V device initialization requires lane-uniform vector state and no scalar control state; use the host-initialized Metal target"
                .to_owned(),
        );
    }

    let state_pointer = state_pointer_type(kernel.state_len);
    let mut mlir = String::new();
    writeln!(
        mlir,
        "// Generated from backend-neutral Mech numeric kernel IR. Do not edit."
    )
    .unwrap();
    writeln!(
        mlir,
        "// Relaxed precision profile: Mech f64 state and constants lower to f32."
    )
    .unwrap();
    writeln!(mlir, "// mech.state_len = {}", kernel.state_len).unwrap();
    writeln!(mlir, "// mech.batch_len = {batch_len}").unwrap();
    emit_state_layout_metadata(&mut mlir, kernel);
    writeln!(
        mlir,
        "// mech.initialization = {}",
        match initialization {
            InitializationMode::Device => "device",
            InitializationMode::Host => "host",
        }
    )
    .unwrap();
    writeln!(
        mlir,
        "spirv.module @mech_kernels Logical GLSL450 attributes {{spirv.target_env = #spirv.target_env<#spirv.vce<v1.3, [Shader], []>, api=Vulkan, #spirv.resource_limits<>>}} {{"
    )
    .unwrap();
    writeln!(
        mlir,
        "  spirv.GlobalVariable @global_invocation_id built_in(\"GlobalInvocationId\") : !spirv.ptr<vector<3xi32>, Input>"
    )
    .unwrap();
    writeln!(
        mlir,
        "  spirv.GlobalVariable @mech_state bind(0, 0) : {state_pointer}"
    )
    .unwrap();

    if matches!(initialization, InitializationMode::Device) {
        emit_initialize(&mut mlir, kernel)?;
    }
    emit_turn(&mut mlir, kernel)?;

    let entries: &[&str] = match initialization {
        InitializationMode::Device => &["mech_initialize", "mech_turn"],
        InitializationMode::Host => &["mech_turn"],
    };
    for entry in entries {
        writeln!(
            mlir,
            "  spirv.EntryPoint \"GLCompute\" @{entry}, @global_invocation_id"
        )
        .unwrap();
        writeln!(
            mlir,
            "  spirv.ExecutionMode @{entry} \"LocalSize\", {THREADS_PER_WORKGROUP}, 1, 1"
        )
        .unwrap();
    }
    writeln!(mlir, "}}").unwrap();
    Ok(mlir)
}

fn emit_state_layout_metadata(mlir: &mut String, kernel: &KernelIr) {
    let lane_offsets = kernel
        .states
        .iter()
        .filter(|state| kernel.value(state.value).ty.shape != Shape::SCALAR)
        .map(|state| state.offset.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let scalar_offsets = kernel
        .states
        .iter()
        .filter(|state| kernel.value(state.value).ty.shape == Shape::SCALAR)
        .map(|state| state.offset.to_string())
        .collect::<Vec<_>>()
        .join(",");
    writeln!(mlir, "// mech.lane_state_offsets = {lane_offsets}").unwrap();
    writeln!(mlir, "// mech.scalar_state_offsets = {scalar_offsets}").unwrap();
}

fn emit_initialize(mlir: &mut String, kernel: &KernelIr) -> Result<(), String> {
    emit_function_header(mlir, "mech_initialize", kernel.state_len);
    let mut emitter = SpirvEmitter::new(kernel.state_len);
    for state in &kernel.states {
        if kernel.value(state.value).ty.shape == Shape::SCALAR {
            return Err(format!(
                "SPIR-V device initialization cannot safely initialize scalar control state {}; use the host-initialized Metal target",
                state.value.get(),
            ));
        }
        let bits = require_uniform(
            &state.initial_elements,
            &format!("state {} initializer", state.value.get()),
        )?;
        let index = emitter.offset_index("%lane", state.offset)?;
        let value = emitter.f32_constant(bits);
        emitter.store_at(&value, &index)?;
    }
    mlir.push_str(&emitter.body);
    writeln!(mlir, "    spirv.Return").unwrap();
    writeln!(mlir, "  }}").unwrap();
    Ok(())
}

fn emit_turn(mlir: &mut String, kernel: &KernelIr) -> Result<(), String> {
    emit_function_header(mlir, "mech_turn", kernel.state_len);
    let mut emitter = SpirvEmitter::new(kernel.state_len);
    let mut values = BTreeMap::<ValueId, String>::new();
    let referenced_values = referenced_values(kernel);
    for activation in kernel
        .activations
        .iter()
        .filter(|activation| referenced_values.contains(&activation.value))
    {
        let bits = require_uniform(
            &activation.elements,
            &format!("activation {}", activation.value.get()),
        )?;
        values.insert(activation.value, emitter.f32_constant(bits));
    }
    for state in &kernel.states {
        let index = if kernel.value(state.value).ty.shape == Shape::SCALAR {
            emitter.i32_constant(state.offset)?
        } else {
            emitter.offset_index("%lane", state.offset)?
        };
        let value = emitter.load_at(&index)?;
        values.insert(state.value, value);
    }
    for instruction in &kernel.instructions {
        writeln!(
            emitter.body,
            "    // node {} {}",
            instruction.node, instruction.operation_name
        )
        .unwrap();
        let value = emit_instruction(&mut emitter, &values, instruction)?;
        values.insert(instruction.output, value);
    }
    for state in kernel.states.iter().filter(|state| {
        kernel.state_is_written(state.value) && kernel.value(state.value).ty.shape != Shape::SCALAR
    }) {
        let value = values.get(&state.value).ok_or_else(|| {
            format!(
                "state {} has no value after SPIR-V lowering",
                state.value.get()
            )
        })?;
        let index = emitter.offset_index("%lane", state.offset)?;
        emitter.store_at(value, &index)?;
    }
    mlir.push_str(&emitter.body);
    writeln!(mlir, "    spirv.Return").unwrap();
    writeln!(mlir, "  }}").unwrap();
    Ok(())
}

fn emit_function_header(mlir: &mut String, name: &str, state_len: usize) {
    writeln!(mlir, "  spirv.func @{name}() \"None\" {{").unwrap();
    writeln!(
        mlir,
        "    %state = spirv.mlir.addressof @mech_state : {}",
        state_pointer_type(state_len)
    )
    .unwrap();
    writeln!(
        mlir,
        "    %global_addr = spirv.mlir.addressof @global_invocation_id : !spirv.ptr<vector<3xi32>, Input>"
    )
    .unwrap();
    writeln!(
        mlir,
        "    %global = spirv.Load \"Input\" %global_addr : vector<3xi32>"
    )
    .unwrap();
    writeln!(
        mlir,
        "    %lane = spirv.CompositeExtract %global[0 : i32] : vector<3xi32>"
    )
    .unwrap();
}

fn emit_instruction(
    emitter: &mut SpirvEmitter,
    values: &BTreeMap<ValueId, String>,
    instruction: &Instruction,
) -> Result<String, String> {
    let operands = instruction
        .inputs
        .iter()
        .map(|source| source_value(emitter, values, source))
        .collect::<Result<Vec<_>, _>>()?;
    match instruction.operation {
        Operation::Broadcast | Operation::Assign => only_operand(&operands, instruction.node),
        Operation::Unary(UnaryOperation::Negate) => {
            let value = only_operand(&operands, instruction.node)?;
            Ok(emitter.unary("spirv.FNegate", &value))
        }
        Operation::Binary(operation) if operation != BinaryOperation::Power => {
            let [left, right] = operands.as_slice() else {
                return Err(format!(
                    "SPIR-V MLIR node {} has invalid binary arity",
                    instruction.node
                ));
            };
            let operation = match operation {
                BinaryOperation::Add => "spirv.FAdd",
                BinaryOperation::Subtract => "spirv.FSub",
                BinaryOperation::Multiply => "spirv.FMul",
                BinaryOperation::Divide => "spirv.FDiv",
                BinaryOperation::Power => unreachable!(),
            };
            Ok(emitter.binary(operation, left, right))
        }
        operation => Err(format!(
            "SPIR-V MLIR node {} `{}` uses unsupported operation {operation:?}",
            instruction.node, instruction.operation_name
        )),
    }
}

fn source_value(
    emitter: &mut SpirvEmitter,
    values: &BTreeMap<ValueId, String>,
    source: &Source,
) -> Result<String, String> {
    match source {
        Source::Value(value) => values.get(value).cloned().ok_or_else(|| {
            format!(
                "value {} is unavailable during SPIR-V MLIR lowering",
                value.get()
            )
        }),
        Source::Constant(constant) => {
            if constant.ty.element != ElementType::F64 {
                return Err("SPIR-V MLIR lane constant is not f64".to_owned());
            }
            let bits = require_uniform(&constant.elements, "lane constant")?;
            Ok(emitter.f32_constant(bits))
        }
    }
}

fn state_pointer_type(state_len: usize) -> String {
    format!(
        "!spirv.ptr<!spirv.struct<(!spirv.array<{state_len} x f32, stride=4> [0])>, StorageBuffer>"
    )
}
