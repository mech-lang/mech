use super::kernel_ir::{
    BatchLayoutKind, BinaryOperation, ElementType, KernelIr, Operation, Source, UnaryOperation,
    ValueId,
};
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fmt::Write as _;

const THREADS_PER_BLOCK: usize = 256;

#[derive(Clone, Copy)]
enum ScalarType {
    F32,
    F64,
}

impl ScalarType {
    fn mlir(self) -> &'static str {
        match self {
            Self::F32 => "f32",
            Self::F64 => "f64",
        }
    }

    fn constant(self, f64_bits: u64) -> (u64, String) {
        match self {
            Self::F32 => {
                let bits = (f64::from_bits(f64_bits) as f32).to_bits();
                (bits as u64, format!("arith.constant 0x{bits:08X} : f32"))
            }
            Self::F64 => (f64_bits, format!("arith.constant 0x{f64_bits:016X} : f64")),
        }
    }
}

struct Emitter {
    body: String,
    next_value: usize,
    constants: BTreeMap<(u8, u64), String>,
    indent: &'static str,
    state_type: String,
    scalar: ScalarType,
}

impl Emitter {
    fn new(indent: &'static str, state_len: usize, scalar: ScalarType) -> Self {
        Self {
            body: String::new(),
            next_value: 0,
            constants: BTreeMap::new(),
            indent,
            state_type: format!("memref<{state_len}x{}>", scalar.mlir()),
            scalar,
        }
    }

    fn value(&mut self) -> String {
        let value = format!("%v{}", self.next_value);
        self.next_value += 1;
        value
    }

    fn operation(&mut self, operation: impl AsRef<str>) -> String {
        let value = self.value();
        writeln!(self.body, "{}{value} = {}", self.indent, operation.as_ref()).unwrap();
        value
    }

    fn constant(&mut self, element: ElementType, bits: u64) -> String {
        let (kind, cache_bits, operation) = match element {
            ElementType::F64 => {
                let (cache_bits, operation) = self.scalar.constant(bits);
                let kind = match self.scalar {
                    ScalarType::F32 => 0,
                    ScalarType::F64 => 1,
                };
                (kind, cache_bits, operation)
            }
            ElementType::Index => (2, bits, format!("arith.constant {bits} : index")),
        };
        if let Some(value) = self.constants.get(&(kind, cache_bits)) {
            return value.clone();
        }
        let value = self.operation(operation);
        self.constants.insert((kind, cache_bits), value.clone());
        value
    }

    fn index(&mut self, value: usize) -> String {
        self.constant(ElementType::Index, value as u64)
    }

    fn offset_index(&mut self, base: &str, offset: usize) -> String {
        if offset == 0 {
            base.to_owned()
        } else {
            let offset = self.index(offset);
            self.operation(format!("arith.addi {base}, {offset} : index"))
        }
    }

    fn load_at(&mut self, index: &str) -> String {
        self.operation(format!("memref.load %state[{index}] : {}", self.state_type))
    }

    fn store_at(&mut self, value: &str, index: &str) {
        writeln!(
            self.body,
            "{}memref.store {value}, %state[{index}] : {}",
            self.indent, self.state_type
        )
        .unwrap();
    }

    fn unary(&mut self, operation: &str, value: &str) -> String {
        self.operation(format!("{operation} {value} : {}", self.scalar.mlir()))
    }

    fn binary(&mut self, operation: &str, left: &str, right: &str) -> String {
        self.operation(format!(
            "{operation} {left}, {right} : {}",
            self.scalar.mlir()
        ))
    }
}

pub(super) fn emit_mlir(kernel: &KernelIr) -> Result<String, String> {
    emit_mlir_with_scalar(kernel, ScalarType::F64)
}

pub(super) fn emit_mlir_f32(kernel: &KernelIr) -> Result<String, String> {
    emit_mlir_with_scalar(kernel, ScalarType::F32)
}

fn emit_mlir_with_scalar(kernel: &KernelIr, scalar: ScalarType) -> Result<String, String> {
    let batch_len = validate_kernel(kernel)?;

    let mut mlir = String::new();
    writeln!(
        mlir,
        "// Generated from backend-neutral Mech numeric kernel IR. Do not edit."
    )
    .unwrap();
    writeln!(mlir, "module attributes {{gpu.container_module}} {{").unwrap();
    emit_length_function(&mut mlir, "mech_state_len", kernel.state_len);
    emit_length_function(&mut mlir, "mech_batch_len", batch_len);
    emit_initialize(&mut mlir, kernel, scalar)?;
    emit_gpu_kernel(&mut mlir, kernel, batch_len, scalar)?;
    emit_launch(&mut mlir, kernel.state_len, batch_len, scalar);
    writeln!(mlir, "}}").unwrap();
    Ok(mlir)
}

pub(super) fn validate_kernel(kernel: &KernelIr) -> Result<usize, String> {
    if kernel.input_len != 0 || !kernel.inputs.is_empty() {
        return Err(
            "GPU MLIR step 1 accepts resident state only; host inputs are not implemented yet"
                .to_owned(),
        );
    }
    for instruction in &kernel.instructions {
        if !matches!(
            instruction.operation,
            Operation::Broadcast
                | Operation::Assign
                | Operation::Unary(UnaryOperation::Negate)
                | Operation::Binary(BinaryOperation::Add)
                | Operation::Binary(BinaryOperation::Subtract)
                | Operation::Binary(BinaryOperation::Multiply)
                | Operation::Binary(BinaryOperation::Divide)
        ) {
            return Err(format!(
                "GPU MLIR node {} `{}` uses unsupported operation {:?}; step 1 supports broadcast, assignment, negate, add, subtract, multiply, and divide",
                instruction.node, instruction.operation_name, instruction.operation
            ));
        }
        for source in &instruction.inputs {
            let Source::Constant(constant) = source else {
                continue;
            };
            if constant.ty.element != ElementType::F64 {
                return Err(format!(
                    "GPU MLIR node {} `{}` has a non-f64 constant",
                    instruction.node, instruction.operation_name
                ));
            }
            require_uniform(
                &constant.elements,
                &format!("node {} lane constant", instruction.node),
            )?;
        }
    }
    let batch = kernel.batch.ok_or_else(|| {
        "GPU MLIR lowering requires a materialized row-vector state so one lane maps to one GPU thread"
            .to_owned()
    })?;
    if batch.kind != BatchLayoutKind::MaterializedLaneVectors {
        return Err("GPU MLIR lowering does not yet accept outer-lifted kernels".to_owned());
    }
    if kernel.states.is_empty() {
        return Err("GPU MLIR lowering requires at least one resident state vector".to_owned());
    }
    let mut lane_states = 0usize;
    for state in &kernel.states {
        let ty = kernel.value(state.value).ty;
        if ty.element != ElementType::F64 || (ty.shape.len() != batch.len && ty.shape.len() != 1) {
            return Err(format!(
                "GPU MLIR state {} must be an f64 scalar or lane vector of length {}",
                state.value.get(),
                batch.len
            ));
        }
        if ty.shape.len() == batch.len {
            lane_states += 1;
        } else if kernel.state_is_written(state.value)
            && !kernel.state_is_identity_preserved(state.value)
        {
            return Err(format!(
                "GPU MLIR scalar state {} is modified by the particle kernel; scalar control state may only use an identity assignment to declare host-owned retention",
                state.value.get(),
            ));
        }
    }
    if lane_states == 0 {
        return Err("GPU MLIR lowering requires at least one resident lane vector".to_owned());
    }
    let referenced_values = referenced_values(kernel);
    for activation in kernel
        .activations
        .iter()
        .filter(|activation| referenced_values.contains(&activation.value))
    {
        if kernel.value(activation.value).ty.element != ElementType::F64 {
            return Err(format!(
                "GPU MLIR activation {} is not f64",
                activation.value.get()
            ));
        }
        require_uniform(
            &activation.elements,
            &format!("activation {}", activation.value.get()),
        )?;
    }
    Ok(batch.len)
}

pub(super) fn referenced_values(kernel: &KernelIr) -> BTreeSet<ValueId> {
    kernel
        .instructions
        .iter()
        .flat_map(|instruction| instruction.inputs.iter())
        .filter_map(|source| match source {
            Source::Value(value) => Some(*value),
            Source::Constant(_) => None,
        })
        .collect()
}

fn emit_length_function(mlir: &mut String, name: &str, len: usize) {
    writeln!(mlir, "  func.func @{name}() -> i64 {{").unwrap();
    writeln!(mlir, "    %len = arith.constant {len} : i64").unwrap();
    writeln!(mlir, "    return %len : i64").unwrap();
    writeln!(mlir, "  }}\n").unwrap();
}

fn emit_initialize(mlir: &mut String, kernel: &KernelIr, scalar: ScalarType) -> Result<(), String> {
    let state_type = format!("memref<{}x{}>", kernel.state_len, scalar.mlir());
    writeln!(
        mlir,
        "  func.func @mech_initialize(%state: {state_type}) attributes {{llvm.emit_c_interface}} {{"
    )
    .unwrap();
    let mut emitter = Emitter::new("    ", kernel.state_len, scalar);
    for state in &kernel.states {
        let uniform = state
            .initial_elements
            .first()
            .copied()
            .filter(|first| state.initial_elements.iter().all(|value| value == first));
        if let Some(bits) = uniform.filter(|_| state.initial_elements.len() > 1) {
            let start = emitter.index(state.offset);
            let end = emitter.index(state.offset + state.initial_elements.len());
            let step = emitter.index(1);
            let value = emitter.constant(ElementType::F64, bits);
            writeln!(
                emitter.body,
                "    scf.for %init = {start} to {end} step {step} {{"
            )
            .unwrap();
            writeln!(
                emitter.body,
                "      memref.store {value}, %state[%init] : {state_type}"
            )
            .unwrap();
            writeln!(emitter.body, "    }}").unwrap();
        } else {
            for (ordinal, bits) in state.initial_elements.iter().enumerate() {
                let index = emitter.index(state.offset + ordinal);
                let value = emitter.constant(ElementType::F64, *bits);
                emitter.store_at(&value, &index);
            }
        }
    }
    mlir.push_str(&emitter.body);
    writeln!(mlir, "    return").unwrap();
    writeln!(mlir, "  }}\n").unwrap();
    Ok(())
}

fn emit_gpu_kernel(
    mlir: &mut String,
    kernel: &KernelIr,
    batch_len: usize,
    scalar: ScalarType,
) -> Result<(), String> {
    let state_type = format!("memref<{}x{}>", kernel.state_len, scalar.mlir());
    writeln!(mlir, "  gpu.module @mech_kernels {{").unwrap();
    writeln!(
        mlir,
        "    gpu.func @mech_turn(%state: {state_type}) kernel attributes {{spirv.entry_point_abi = #spirv.entry_point_abi<>}} {{"
    )
    .unwrap();
    mlir.push_str(
        r#"      %block = gpu.block_id x
      %width = gpu.block_dim x
      %thread = gpu.thread_id x
      %base = arith.muli %block, %width : index
      %lane = arith.addi %base, %thread : index
"#,
    );
    writeln!(mlir, "      %limit = arith.constant {batch_len} : index").unwrap();
    writeln!(
        mlir,
        "      %active = arith.cmpi ult, %lane, %limit : index"
    )
    .unwrap();
    writeln!(mlir, "      scf.if %active {{").unwrap();

    let mut emitter = Emitter::new("        ", kernel.state_len, scalar);
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
        values.insert(activation.value, emitter.constant(ElementType::F64, bits));
    }
    for state in &kernel.states {
        let index = if kernel.value(state.value).ty.shape == super::kernel_ir::Shape::SCALAR {
            emitter.index(state.offset)
        } else {
            emitter.offset_index("%lane", state.offset)
        };
        let value = emitter.load_at(&index);
        values.insert(state.value, value);
    }
    for instruction in &kernel.instructions {
        writeln!(
            emitter.body,
            "        // node {} {}",
            instruction.node, instruction.operation_name
        )
        .unwrap();
        let value = emit_instruction(&mut emitter, &values, instruction)?;
        values.insert(instruction.output, value);
    }
    for state in kernel.states.iter().filter(|state| {
        kernel.state_is_written(state.value)
            && kernel.value(state.value).ty.shape != super::kernel_ir::Shape::SCALAR
    }) {
        let value = values.get(&state.value).ok_or_else(|| {
            format!(
                "state {} has no value after GPU lowering",
                state.value.get()
            )
        })?;
        let index = emitter.offset_index("%lane", state.offset);
        emitter.store_at(value, &index);
    }
    mlir.push_str(&emitter.body);
    writeln!(mlir, "      }}").unwrap();
    writeln!(mlir, "      gpu.return").unwrap();
    writeln!(mlir, "    }}").unwrap();
    writeln!(mlir, "  }}\n").unwrap();
    Ok(())
}

fn emit_instruction(
    emitter: &mut Emitter,
    values: &BTreeMap<ValueId, String>,
    instruction: &super::kernel_ir::Instruction,
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
            Ok(emitter.unary("arith.negf", &value))
        }
        Operation::Binary(operation) if operation != BinaryOperation::Power => {
            let [left, right] = operands.as_slice() else {
                return Err(format!(
                    "GPU MLIR node {} has invalid binary arity",
                    instruction.node
                ));
            };
            let operation = match operation {
                BinaryOperation::Add => "arith.addf",
                BinaryOperation::Subtract => "arith.subf",
                BinaryOperation::Multiply => "arith.mulf",
                BinaryOperation::Divide => "arith.divf",
                BinaryOperation::Power => unreachable!(),
            };
            Ok(emitter.binary(operation, left, right))
        }
        operation => Err(format!(
            "GPU MLIR node {} `{}` uses unsupported operation {operation:?}; step 1 supports broadcast, assignment, negate, add, subtract, multiply, and divide",
            instruction.node, instruction.operation_name
        )),
    }
}

fn source_value(
    emitter: &mut Emitter,
    values: &BTreeMap<ValueId, String>,
    source: &Source,
) -> Result<String, String> {
    match source {
        Source::Value(value) => values.get(value).cloned().ok_or_else(|| {
            format!(
                "value {} is unavailable during GPU MLIR lowering",
                value.get()
            )
        }),
        Source::Constant(constant) => {
            if constant.ty.element != ElementType::F64 {
                return Err("GPU MLIR lane constant is not f64".to_owned());
            }
            let bits = require_uniform(&constant.elements, "lane constant")?;
            Ok(emitter.constant(ElementType::F64, bits))
        }
    }
}

pub(super) fn only_operand(operands: &[String], node: u32) -> Result<String, String> {
    let [value] = operands else {
        return Err(format!("GPU MLIR node {node} has invalid unary arity"));
    };
    Ok(value.clone())
}

pub(super) fn require_uniform(elements: &[u64], label: &str) -> Result<u64, String> {
    let Some(first) = elements.first().copied() else {
        return Err(format!("GPU MLIR {label} is empty"));
    };
    if elements.iter().any(|element| *element != first) {
        return Err(format!("GPU MLIR {label} is not lane-uniform"));
    }
    Ok(first)
}

fn emit_launch(mlir: &mut String, state_len: usize, batch_len: usize, scalar: ScalarType) {
    let blocks = batch_len.div_ceil(THREADS_PER_BLOCK);
    let state_type = format!("memref<{state_len}x{}>", scalar.mlir());
    writeln!(
        mlir,
        "  func.func @mech_launch(%state: {state_type}) attributes {{llvm.emit_c_interface}} {{"
    )
    .unwrap();
    writeln!(mlir, "    %blocks = arith.constant {blocks} : index").unwrap();
    writeln!(
        mlir,
        "    %threads = arith.constant {THREADS_PER_BLOCK} : index"
    )
    .unwrap();
    writeln!(mlir, "    %one = arith.constant 1 : index").unwrap();
    writeln!(mlir, "    gpu.launch_func @mech_kernels::@mech_turn").unwrap();
    writeln!(mlir, "        blocks in (%blocks, %one, %one)").unwrap();
    writeln!(mlir, "        threads in (%threads, %one, %one)").unwrap();
    writeln!(mlir, "        args(%state : {state_type})").unwrap();
    writeln!(mlir, "    return").unwrap();
    writeln!(mlir, "  }}").unwrap();
}
