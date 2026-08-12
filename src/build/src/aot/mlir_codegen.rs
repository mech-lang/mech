use super::kernel_ir::{
    BatchLayoutKind, BinaryOperation, ElementType, KernelIr, Operation, Shape, Source,
    UnaryOperation, ValueId,
};
use std::collections::BTreeMap;
use std::fmt::Write as _;

#[derive(Clone)]
struct Operand {
    elements: Vec<String>,
    shape: Shape,
    element: ElementType,
    known_elements: Option<Vec<u64>>,
}

struct Emitter {
    body: String,
    next_value: usize,
    constants: BTreeMap<(u8, u64), String>,
}

impl Emitter {
    fn new() -> Self {
        Self {
            body: String::new(),
            next_value: 0,
            constants: BTreeMap::new(),
        }
    }

    fn value(&mut self) -> String {
        let value = format!("%v{}", self.next_value);
        self.next_value += 1;
        value
    }

    fn operation(&mut self, operation: impl AsRef<str>) -> String {
        let value = self.value();
        writeln!(self.body, "    {value} = {}", operation.as_ref()).unwrap();
        value
    }

    fn constant(&mut self, element: ElementType, bits: u64) -> String {
        let kind = match element {
            ElementType::F64 => 0,
            ElementType::Index => 1,
        };
        if let Some(value) = self.constants.get(&(kind, bits)) {
            return value.clone();
        }
        let operation = match element {
            ElementType::F64 => format!("arith.constant 0x{bits:016X} : f64"),
            ElementType::Index => format!("arith.constant {bits} : index"),
        };
        let value = self.operation(operation);
        self.constants.insert((kind, bits), value.clone());
        value
    }

    fn f64(&mut self, value: f64) -> String {
        self.constant(ElementType::F64, value.to_bits())
    }

    fn index(&mut self, value: usize) -> String {
        self.constant(ElementType::Index, value as u64)
    }

    fn load(&mut self, buffer: &str, offset: usize) -> String {
        let index = self.index(offset);
        self.load_at(buffer, &index)
    }

    fn load_at(&mut self, buffer: &str, index: &str) -> String {
        self.operation(format!("memref.load {buffer}[{index}] : memref<?xf64>"))
    }

    fn store(&mut self, value: &str, buffer: &str, offset: usize) {
        let index = self.index(offset);
        self.store_at(value, buffer, &index);
    }

    fn store_at(&mut self, value: &str, buffer: &str, index: &str) {
        writeln!(
            self.body,
            "    memref.store {value}, {buffer}[{index}] : memref<?xf64>"
        )
        .unwrap();
    }

    fn unary(&mut self, operation: &str, value: &str) -> String {
        self.operation(format!("{operation} {value} : f64"))
    }

    fn binary(&mut self, operation: &str, left: &str, right: &str) -> String {
        self.operation(format!("{operation} {left}, {right} : f64"))
    }

    fn offset_index(&mut self, base: &str, offset: usize) -> String {
        if offset == 0 {
            base.to_owned()
        } else {
            let offset = self.index(offset);
            self.operation(format!("arith.addi {base}, {offset} : index"))
        }
    }
}

pub(super) fn emit_mlir(kernel: &KernelIr) -> Result<String, String> {
    if matches!(
        kernel.batch,
        Some(batch) if batch.kind == BatchLayoutKind::OuterLift
    ) {
        return Err(
            "MLIR prototype does not yet accept an outer-lifted fixed-shape kernel".to_owned(),
        );
    }
    if let Some(state) = kernel
        .states
        .iter()
        .find(|state| kernel.value(state.value).ty.element != ElementType::F64)
    {
        return Err(format!(
            "MLIR prototype state {} is not f64",
            state.value.get()
        ));
    }

    let mut mlir = String::new();
    writeln!(
        mlir,
        "// Generated from backend-neutral Mech numeric kernel IR. Do not edit."
    )
    .unwrap();
    writeln!(mlir, "module {{").unwrap();
    emit_length_function(&mut mlir, "mech_input_len", kernel.input_len);
    emit_length_function(&mut mlir, "mech_state_len", kernel.state_len);
    emit_initialize(&mut mlir, kernel);
    emit_turn_implementation(&mut mlir, kernel)?;
    mlir.push_str(
        r#"
  func.func @mech_turn_fast(%inputs: memref<?xf64>, %state: memref<?xf64>) attributes {llvm.emit_c_interface} {
    func.call @mech_turn_fast_impl(%inputs, %state) : (memref<?xf64>, memref<?xf64>) -> ()
    return
  }
"#,
    );
    emit_run_function(&mut mlir, kernel)?;
    writeln!(mlir, "}}").unwrap();
    Ok(mlir)
}

fn emit_length_function(mlir: &mut String, name: &str, len: usize) {
    writeln!(mlir, "  func.func @{name}() -> i64 {{").unwrap();
    writeln!(mlir, "    %len = arith.constant {len} : i64").unwrap();
    writeln!(mlir, "    return %len : i64").unwrap();
    writeln!(mlir, "  }}\n").unwrap();
}

fn emit_initialize(mlir: &mut String, kernel: &KernelIr) {
    writeln!(
        mlir,
        "  func.func @mech_initialize(%state: memref<?xf64>) attributes {{llvm.emit_c_interface}} {{"
    )
    .unwrap();
    let mut emitter = Emitter::new();
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
            emitter.store_at(&value, "%state", "%init");
            writeln!(emitter.body, "    }}").unwrap();
        } else {
            for (ordinal, bits) in state.initial_elements.iter().enumerate() {
                let value = emitter.constant(ElementType::F64, *bits);
                emitter.store(&value, "%state", state.offset + ordinal);
            }
        }
    }
    mlir.push_str(&emitter.body);
    writeln!(mlir, "    return").unwrap();
    writeln!(mlir, "  }}\n").unwrap();
}

fn emit_turn_implementation(mlir: &mut String, kernel: &KernelIr) -> Result<(), String> {
    writeln!(
        mlir,
        "  func.func private @mech_turn_fast_impl(%inputs: memref<?xf64>, %state: memref<?xf64>) attributes {{llvm.linkage = #llvm.linkage<internal>, passthrough = [\"alwaysinline\"]}} {{"
    )
    .unwrap();
    if let Some(batch) = kernel.batch {
        debug_assert_eq!(batch.kind, BatchLayoutKind::MaterializedLaneVectors);
        emit_materialized_batched_turn(mlir, kernel, batch.len)?;
    } else {
        emit_scalarized_turn(mlir, kernel)?;
    }
    writeln!(mlir, "  }}").unwrap();
    Ok(())
}

fn emit_scalarized_turn(mlir: &mut String, kernel: &KernelIr) -> Result<(), String> {
    let mut emitter = Emitter::new();
    let mut values = BTreeMap::<ValueId, Operand>::new();

    for activation in &kernel.activations {
        let ty = kernel.value(activation.value).ty;
        let elements = activation
            .elements
            .iter()
            .map(|bits| emitter.constant(ty.element, *bits))
            .collect();
        values.insert(
            activation.value,
            Operand {
                elements,
                shape: ty.shape,
                element: ty.element,
                known_elements: Some(activation.elements.clone()),
            },
        );
    }
    for state in &kernel.states {
        let ty = kernel.value(state.value).ty;
        let elements = (0..state.initial_elements.len())
            .map(|ordinal| emitter.load("%state", state.offset + ordinal))
            .collect();
        values.insert(
            state.value,
            Operand {
                elements,
                shape: ty.shape,
                element: ty.element,
                known_elements: None,
            },
        );
    }

    for instruction in &kernel.instructions {
        writeln!(
            emitter.body,
            "    // node {} {}",
            instruction.node, instruction.operation_name
        )
        .unwrap();
        let output = kernel.value(instruction.output);
        if output.ty.element != ElementType::F64 {
            return Err(format!(
                "node {} has unsupported non-f64 MLIR output",
                instruction.node
            ));
        }
        let result = match instruction.operation {
            Operation::Input { ordinal } => {
                let input = kernel.input(ordinal);
                Operand {
                    elements: (0..input.len)
                        .map(|element| emitter.load("%inputs", input.offset + element))
                        .collect(),
                    shape: output.ty.shape,
                    element: output.ty.element,
                    known_elements: None,
                }
            }
            operation => {
                let operands = instruction
                    .inputs
                    .iter()
                    .map(|source| source_operand(&mut emitter, &values, source))
                    .collect::<Result<Vec<_>, _>>()?;
                emit_operation(&mut emitter, operation, &operands, output.ty.shape)?
            }
        };
        if result.elements.len() != output.ty.shape.len() {
            return Err(format!(
                "node {} MLIR lowering produced {} elements for shape {}x{}",
                instruction.node,
                result.elements.len(),
                output.ty.shape.rows,
                output.ty.shape.columns,
            ));
        }
        values.insert(instruction.output, result);
    }

    for state in &kernel.states {
        let value = values
            .get(&state.value)
            .ok_or_else(|| format!("state {} has no MLIR value", state.value.get()))?;
        for (ordinal, element) in value.elements.iter().enumerate() {
            emitter.store(element, "%state", state.offset + ordinal);
        }
    }
    mlir.push_str(&emitter.body);
    writeln!(mlir, "    return").unwrap();
    Ok(())
}

fn emit_materialized_batched_turn(
    mlir: &mut String,
    kernel: &KernelIr,
    batch_len: usize,
) -> Result<(), String> {
    let mut emitter = Emitter::new();
    let start = emitter.index(0);
    let end = emitter.index(batch_len);
    let step = emitter.index(1);
    writeln!(
        emitter.body,
        "    scf.for %lane = {start} to {end} step {step} {{"
    )
    .unwrap();
    let mut state_values = Vec::new();
    for state in &kernel.states {
        let index = emitter.offset_index("%lane", state.offset);
        let value = emitter.load_at("%state", &index);
        state_values.push((state.value, value));
    }
    let next = emit_materialized_lane(&mut emitter, kernel, "%lane", &state_values)?;
    for (state, value) in kernel.states.iter().zip(next) {
        let index = emitter.offset_index("%lane", state.offset);
        emitter.store_at(&value, "%state", &index);
    }
    writeln!(emitter.body, "    }}").unwrap();
    mlir.push_str(&emitter.body);
    writeln!(mlir, "    return").unwrap();
    Ok(())
}

fn emit_materialized_lane(
    emitter: &mut Emitter,
    kernel: &KernelIr,
    lane: &str,
    state_values: &[(ValueId, String)],
) -> Result<Vec<String>, String> {
    let mut values = BTreeMap::<ValueId, Operand>::new();
    for activation in &kernel.activations {
        let ty = kernel.value(activation.value).ty;
        let Some(bits) = activation.elements.first().copied() else {
            return Err(format!("activation {} is empty", activation.value.get()));
        };
        if activation.elements.iter().any(|element| *element != bits) {
            return Err(format!(
                "MLIR materialized batch activation {} is not lane-uniform",
                activation.value.get()
            ));
        }
        values.insert(
            activation.value,
            Operand {
                elements: vec![emitter.constant(ty.element, bits)],
                shape: Shape::SCALAR,
                element: ty.element,
                known_elements: Some(vec![bits]),
            },
        );
    }
    for (state, value) in state_values {
        values.insert(
            *state,
            Operand {
                elements: vec![value.clone()],
                shape: Shape::SCALAR,
                element: ElementType::F64,
                known_elements: None,
            },
        );
    }

    for instruction in &kernel.instructions {
        writeln!(
            emitter.body,
            "    // node {} {}",
            instruction.node, instruction.operation_name
        )
        .unwrap();
        let result = match instruction.operation {
            Operation::Input { ordinal } => {
                let input = kernel.input(ordinal);
                let index = if input.len == 1 {
                    emitter.index(input.offset)
                } else {
                    emitter.offset_index(lane, input.offset)
                };
                Operand {
                    elements: vec![emitter.load_at("%inputs", &index)],
                    shape: Shape::SCALAR,
                    element: ElementType::F64,
                    known_elements: None,
                }
            }
            operation => {
                let operands = instruction
                    .inputs
                    .iter()
                    .map(|source| lane_source_operand(emitter, &values, source))
                    .collect::<Result<Vec<_>, _>>()?;
                emit_operation(emitter, operation, &operands, Shape::SCALAR)?
            }
        };
        if result.elements.len() != 1 {
            return Err(format!(
                "node {} produced {} values in a materialized MLIR lane",
                instruction.node,
                result.elements.len()
            ));
        }
        values.insert(instruction.output, result);
    }

    kernel
        .states
        .iter()
        .map(|state| {
            values
                .get(&state.value)
                .map(|value| value.elements[0].clone())
                .ok_or_else(|| format!("state {} has no MLIR lane value", state.value.get()))
        })
        .collect()
}

fn lane_source_operand(
    emitter: &mut Emitter,
    values: &BTreeMap<ValueId, Operand>,
    source: &Source,
) -> Result<Operand, String> {
    match source {
        Source::Constant(constant) => {
            let Some(bits) = constant.elements.first().copied() else {
                return Err("MLIR lane constant is empty".to_owned());
            };
            if constant.elements.iter().any(|element| *element != bits) {
                return Err("MLIR lane constant is not uniform".to_owned());
            }
            Ok(Operand {
                elements: vec![emitter.constant(constant.ty.element, bits)],
                shape: Shape::SCALAR,
                element: constant.ty.element,
                known_elements: Some(vec![bits]),
            })
        }
        Source::Value(id) => values
            .get(id)
            .cloned()
            .ok_or_else(|| format!("value {} is unavailable during MLIR lowering", id.get())),
    }
}

fn emit_run_function(mlir: &mut String, kernel: &KernelIr) -> Result<(), String> {
    if matches!(
        kernel.batch,
        Some(batch) if batch.kind == BatchLayoutKind::MaterializedLaneVectors && batch.len % 8 == 0
    ) {
        emit_temporally_tiled_run(mlir, kernel, kernel.batch.unwrap().len)
    } else {
        mlir.push_str(
            r#"
  func.func @mech_run_fast(%inputs: memref<?xf64>, %state: memref<?xf64>, %turns: index) attributes {llvm.emit_c_interface} {
    %c0 = arith.constant 0 : index
    %c1 = arith.constant 1 : index
    scf.for %turn = %c0 to %turns step %c1 {
      func.call @mech_turn_fast_impl(%inputs, %state) : (memref<?xf64>, memref<?xf64>) -> ()
    }
    return
  }
"#,
        );
        Ok(())
    }
}

fn emit_temporally_tiled_run(
    mlir: &mut String,
    kernel: &KernelIr,
    batch_len: usize,
) -> Result<(), String> {
    writeln!(
        mlir,
        "\n  func.func @mech_run_fast(%inputs: memref<?xf64>, %state: memref<?xf64>, %turns: index) attributes {{llvm.emit_c_interface}} {{"
    )
    .unwrap();
    let mut emitter = Emitter::new();
    let start = emitter.index(0);
    let end = emitter.index(batch_len);
    let lane_step = emitter.index(8);
    let one = emitter.index(1);
    writeln!(
        emitter.body,
        "    scf.for %lane = {start} to {end} step {lane_step} {{"
    )
    .unwrap();
    let mut initial = Vec::new();
    for state in &kernel.states {
        let index = emitter.offset_index("%lane", state.offset);
        initial.push(emitter.operation(format!(
            "vector.load %state[{index}] : memref<?xf64>, vector<8xf64>"
        )));
    }
    let arguments = initial
        .iter()
        .enumerate()
        .map(|(ordinal, value)| format!("%iter{ordinal} = {value}"))
        .collect::<Vec<_>>()
        .join(", ");
    let result_types = vec!["vector<8xf64>"; initial.len()].join(", ");
    writeln!(
        emitter.body,
        "    %resident:{} = scf.for %turn = {start} to %turns step {one} iter_args({arguments}) -> ({result_types}) {{",
        initial.len()
    )
    .unwrap();

    let state_values = kernel
        .states
        .iter()
        .enumerate()
        .map(|(ordinal, state)| (state.value, format!("%iter{ordinal}")))
        .collect::<Vec<_>>();
    let next = emit_materialized_vector_pair(&mut emitter, kernel, "%lane", &state_values)?;
    writeln!(
        emitter.body,
        "    scf.yield {} : {result_types}",
        next.join(", ")
    )
    .unwrap();
    writeln!(emitter.body, "    }}").unwrap();

    for (state_ordinal, state) in kernel.states.iter().enumerate() {
        let index = emitter.offset_index("%lane", state.offset);
        writeln!(
            emitter.body,
            "    vector.store %resident#{state_ordinal}, %state[{index}] : memref<?xf64>, vector<8xf64>"
        )
        .unwrap();
    }
    writeln!(emitter.body, "    }}").unwrap();
    mlir.push_str(&emitter.body);
    writeln!(mlir, "    return").unwrap();
    writeln!(mlir, "  }}").unwrap();
    Ok(())
}

#[derive(Clone)]
struct VectorOperand {
    value: String,
    known: Option<f64>,
}

fn emit_materialized_vector_pair(
    emitter: &mut Emitter,
    kernel: &KernelIr,
    lane: &str,
    state_values: &[(ValueId, String)],
) -> Result<Vec<String>, String> {
    let mut values = BTreeMap::<ValueId, VectorOperand>::new();
    for activation in &kernel.activations {
        let Some(bits) = activation.elements.first().copied() else {
            return Err(format!("activation {} is empty", activation.value.get()));
        };
        if activation.elements.iter().any(|element| *element != bits) {
            return Err(format!(
                "MLIR materialized batch activation {} is not lane-uniform",
                activation.value.get()
            ));
        }
        values.insert(
            activation.value,
            vector_constant(emitter, f64::from_bits(bits)),
        );
    }
    for (state, value) in state_values {
        values.insert(
            *state,
            VectorOperand {
                value: value.clone(),
                known: None,
            },
        );
    }

    for instruction in &kernel.instructions {
        writeln!(
            emitter.body,
            "    // vector node {} {}",
            instruction.node, instruction.operation_name
        )
        .unwrap();
        let result = match instruction.operation {
            Operation::Input { ordinal } => {
                let input = kernel.input(ordinal);
                if input.len == 1 {
                    let scalar = emitter.load("%inputs", input.offset);
                    VectorOperand {
                        value: emitter.operation(format!("vector.splat {scalar} : vector<8xf64>")),
                        known: None,
                    }
                } else {
                    let index = emitter.offset_index(lane, input.offset);
                    VectorOperand {
                        value: emitter.operation(format!(
                            "vector.load %inputs[{index}] : memref<?xf64>, vector<8xf64>"
                        )),
                        known: None,
                    }
                }
            }
            Operation::Broadcast | Operation::Assign => {
                vector_input(&values, instruction.inputs.first(), instruction.node)?
            }
            Operation::Unary(operation) => {
                let input = vector_input(&values, instruction.inputs.first(), instruction.node)?;
                let operation = match operation {
                    UnaryOperation::Sin => "math.sin",
                    UnaryOperation::Cos => "math.cos",
                    UnaryOperation::Negate => "arith.negf",
                };
                VectorOperand {
                    value: emitter
                        .operation(format!("{operation} {} : vector<8xf64>", input.value)),
                    known: None,
                }
            }
            Operation::Atan2 => {
                let [left, right] = instruction.inputs.as_slice() else {
                    return Err(format!("node {} has invalid atan2 arity", instruction.node));
                };
                let left = vector_source(emitter, &values, left)?;
                let right = vector_source(emitter, &values, right)?;
                VectorOperand {
                    value: emitter.operation(format!(
                        "math.atan2 {}, {} : vector<8xf64>",
                        left.value, right.value
                    )),
                    known: None,
                }
            }
            Operation::Binary(operation) => {
                let [left, right] = instruction.inputs.as_slice() else {
                    return Err(format!(
                        "node {} has invalid binary arity",
                        instruction.node
                    ));
                };
                let left = vector_source(emitter, &values, left)?;
                let right = vector_source(emitter, &values, right)?;
                if operation == BinaryOperation::Power {
                    vector_power(emitter, &left, &right)
                } else {
                    let operation = match operation {
                        BinaryOperation::Add => "arith.addf",
                        BinaryOperation::Subtract => "arith.subf",
                        BinaryOperation::Multiply => "arith.mulf",
                        BinaryOperation::Divide => "arith.divf",
                        BinaryOperation::Power => unreachable!(),
                    };
                    VectorOperand {
                        value: emitter.operation(format!(
                            "{operation} {}, {} : vector<8xf64>",
                            left.value, right.value
                        )),
                        known: None,
                    }
                }
            }
            operation => {
                return Err(format!(
                    "node {} operation {operation:?} is not vector lane-wise",
                    instruction.node
                ));
            }
        };
        values.insert(instruction.output, result);
    }

    kernel
        .states
        .iter()
        .map(|state| {
            values
                .get(&state.value)
                .map(|value| value.value.clone())
                .ok_or_else(|| format!("state {} has no MLIR vector value", state.value.get()))
        })
        .collect()
}

fn vector_input(
    values: &BTreeMap<ValueId, VectorOperand>,
    input: Option<&Source>,
    node: u32,
) -> Result<VectorOperand, String> {
    let input = input.ok_or_else(|| format!("node {node} has invalid unary arity"))?;
    match input {
        Source::Value(id) => values
            .get(id)
            .cloned()
            .ok_or_else(|| format!("value {} is unavailable during MLIR lowering", id.get())),
        Source::Constant(_) => Err(format!(
            "node {node} has an unexpected constant-only unary operation"
        )),
    }
}

fn vector_source(
    emitter: &mut Emitter,
    values: &BTreeMap<ValueId, VectorOperand>,
    source: &Source,
) -> Result<VectorOperand, String> {
    match source {
        Source::Value(id) => values
            .get(id)
            .cloned()
            .ok_or_else(|| format!("value {} is unavailable during MLIR lowering", id.get())),
        Source::Constant(constant) => {
            if constant.ty.element != ElementType::F64 {
                return Err("MLIR vector constant is not f64".to_owned());
            }
            let Some(bits) = constant.elements.first().copied() else {
                return Err("MLIR vector constant is empty".to_owned());
            };
            if constant.elements.iter().any(|element| *element != bits) {
                return Err("MLIR vector constant is not lane-uniform".to_owned());
            }
            Ok(vector_constant(emitter, f64::from_bits(bits)))
        }
    }
}

fn vector_constant(emitter: &mut Emitter, value: f64) -> VectorOperand {
    VectorOperand {
        value: emitter.operation(format!(
            "arith.constant dense<0x{:016X}> : vector<8xf64>",
            value.to_bits()
        )),
        known: Some(value),
    }
}

fn vector_power(
    emitter: &mut Emitter,
    base: &VectorOperand,
    exponent: &VectorOperand,
) -> VectorOperand {
    let binary = |emitter: &mut Emitter, operation: &str, left: &str, right: &str| {
        emitter.operation(format!("{operation} {left}, {right} : vector<8xf64>"))
    };
    let value = match exponent.known {
        Some(2.0) => binary(emitter, "arith.mulf", &base.value, &base.value),
        Some(3.0) => {
            let square = binary(emitter, "arith.mulf", &base.value, &base.value);
            binary(emitter, "arith.mulf", &square, &base.value)
        }
        Some(0.5) => emitter.operation(format!("math.sqrt {} : vector<8xf64>", base.value)),
        Some(-1.0) => {
            let one = vector_constant(emitter, 1.0);
            binary(emitter, "arith.divf", &one.value, &base.value)
        }
        _ => binary(emitter, "math.powf", &base.value, &exponent.value),
    };
    VectorOperand { value, known: None }
}

fn source_operand(
    emitter: &mut Emitter,
    values: &BTreeMap<ValueId, Operand>,
    source: &Source,
) -> Result<Operand, String> {
    match source {
        Source::Constant(constant) => Ok(Operand {
            elements: constant
                .elements
                .iter()
                .map(|bits| emitter.constant(constant.ty.element, *bits))
                .collect(),
            shape: constant.ty.shape,
            element: constant.ty.element,
            known_elements: Some(constant.elements.clone()),
        }),
        Source::Value(id) => values
            .get(id)
            .cloned()
            .ok_or_else(|| format!("value {} is unavailable during MLIR lowering", id.get())),
    }
}

fn emit_operation(
    emitter: &mut Emitter,
    operation: Operation,
    inputs: &[Operand],
    output: Shape,
) -> Result<Operand, String> {
    let elements = match operation {
        Operation::Input { .. } => {
            return Err("input operation reached MLIR operation lowering".to_owned());
        }
        Operation::Broadcast => {
            let [input] = inputs else {
                return Err("broadcast has invalid MLIR arity".to_owned());
            };
            vec![input.elements[0].clone(); output.len()]
        }
        Operation::HorizontalConcatenate => inputs
            .iter()
            .flat_map(|input| input.elements.iter().cloned())
            .collect(),
        Operation::VerticalConcatenate => {
            let mut elements = Vec::new();
            for column in 0..output.columns {
                for input in inputs {
                    for row in 0..input.shape.rows {
                        elements.push(input.elements[row + column * input.shape.rows].clone());
                    }
                }
            }
            elements
        }
        Operation::MatrixMultiply => {
            let [left, right] = inputs else {
                return Err("matrix multiplication has invalid MLIR arity".to_owned());
            };
            let mut elements = Vec::with_capacity(output.len());
            for column in 0..output.columns {
                for row in 0..output.rows {
                    let mut terms = Vec::new();
                    for inner in 0..left.shape.columns {
                        if let Some(term) = multiply_term(
                            emitter,
                            left,
                            row + inner * left.shape.rows,
                            right,
                            inner + column * right.shape.rows,
                        ) {
                            terms.push(term);
                        }
                    }
                    elements.push(sum(emitter, &terms));
                }
            }
            elements
        }
        Operation::Transpose => {
            let [input] = inputs else {
                return Err("transpose has invalid MLIR arity".to_owned());
            };
            (0..output.columns)
                .flat_map(|column| {
                    (0..output.rows)
                        .map(move |row| input.elements[column + row * input.shape.rows].clone())
                })
                .collect()
        }
        Operation::Dot => {
            let [left, right] = inputs else {
                return Err("dot has invalid MLIR arity".to_owned());
            };
            let products = left
                .elements
                .iter()
                .zip(&right.elements)
                .map(|(left, right)| emitter.binary("arith.mulf", left, right))
                .collect::<Vec<_>>();
            vec![sum(emitter, &products)]
        }
        Operation::Assign => {
            let [input] = inputs else {
                return Err("assignment has invalid MLIR arity".to_owned());
            };
            input.elements.clone()
        }
        Operation::Unary(operation) => {
            let [input] = inputs else {
                return Err("unary operation has invalid MLIR arity".to_owned());
            };
            input
                .elements
                .iter()
                .map(|value| match operation {
                    UnaryOperation::Sin => emitter.unary("math.sin", value),
                    UnaryOperation::Cos => emitter.unary("math.cos", value),
                    UnaryOperation::Negate => emitter.unary("arith.negf", value),
                })
                .collect()
        }
        Operation::Atan2 => {
            let [left, right] = inputs else {
                return Err("atan2 has invalid MLIR arity".to_owned());
            };
            (0..output.len())
                .map(|index| {
                    let left = &left.elements[if left.shape.len() == 1 { 0 } else { index }];
                    let right = &right.elements[if right.shape.len() == 1 { 0 } else { index }];
                    emitter.binary("math.atan2", left, right)
                })
                .collect()
        }
        Operation::Binary(operation) => {
            let [left, right] = inputs else {
                return Err("binary operation has invalid MLIR arity".to_owned());
            };
            (0..output.len())
                .map(|index| {
                    let left_index = if left.shape.len() == 1 { 0 } else { index };
                    let right_index = if right.shape.len() == 1 { 0 } else { index };
                    let left_value = &left.elements[left_index];
                    let right_value = &right.elements[right_index];
                    match operation {
                        BinaryOperation::Add => {
                            emitter.binary("arith.addf", left_value, right_value)
                        }
                        BinaryOperation::Subtract => {
                            emitter.binary("arith.subf", left_value, right_value)
                        }
                        BinaryOperation::Multiply => {
                            emitter.binary("arith.mulf", left_value, right_value)
                        }
                        BinaryOperation::Divide => {
                            emitter.binary("arith.divf", left_value, right_value)
                        }
                        BinaryOperation::Power => emit_power(
                            emitter,
                            left_value,
                            right_value,
                            known_f64(right, right_index),
                        ),
                    }
                })
                .collect()
        }
        Operation::MultiplyRows => {
            let [matrix, vector] = inputs else {
                return Err("row multiplication has invalid MLIR arity".to_owned());
            };
            (0..output.len())
                .map(|index| {
                    emitter.binary(
                        "arith.mulf",
                        &matrix.elements[index],
                        &vector.elements[index % output.rows],
                    )
                })
                .collect()
        }
        Operation::SumColumns => {
            let [input] = inputs else {
                return Err("column sum has invalid MLIR arity".to_owned());
            };
            (0..input.shape.rows)
                .map(|row| {
                    let terms = (0..input.shape.columns)
                        .map(|column| input.elements[row + column * input.shape.rows].clone())
                        .collect::<Vec<_>>();
                    sum(emitter, &terms)
                })
                .collect()
        }
        Operation::Gather1D => {
            let [source, indices] = inputs else {
                return Err("gather has invalid MLIR arity".to_owned());
            };
            (0..output.len())
                .map(|ordinal| {
                    let index = known_one_based_index(indices, ordinal)?;
                    source.elements.get(index).cloned().ok_or_else(|| {
                        format!("MLIR gather index {} exceeds source length", index + 1)
                    })
                })
                .collect::<Result<Vec<_>, String>>()?
        }
        Operation::RowsAllColumns => {
            let [source, indices] = inputs else {
                return Err("row selection has invalid MLIR arity".to_owned());
            };
            let mut elements = Vec::with_capacity(output.len());
            for column in 0..source.shape.columns {
                for ordinal in 0..indices.shape.len() {
                    let row = known_one_based_index(indices, ordinal)?;
                    elements.push(source.elements[row + column * source.shape.rows].clone());
                }
            }
            elements
        }
        Operation::AddIndexedRows | Operation::SubtractIndexedRows => {
            let [base, source, indices] = inputs else {
                return Err("indexed row update has invalid MLIR arity".to_owned());
            };
            let mut elements = base.elements.clone();
            for occurrence in 0..indices.shape.len() {
                let row = known_one_based_index(indices, occurrence)?;
                for column in 0..output.columns {
                    let target = row + column * output.rows;
                    let source_index = occurrence + column * source.shape.rows;
                    elements[target] = emitter.binary(
                        if operation == Operation::AddIndexedRows {
                            "arith.addf"
                        } else {
                            "arith.subf"
                        },
                        &elements[target],
                        &source.elements[source_index],
                    );
                }
            }
            elements
        }
    };

    if elements.len() != output.len() {
        return Err(format!(
            "MLIR operation {operation:?} produced {} elements, expected {}",
            elements.len(),
            output.len()
        ));
    }
    Ok(Operand {
        elements,
        shape: output,
        element: ElementType::F64,
        known_elements: None,
    })
}

fn emit_power(
    emitter: &mut Emitter,
    base: &str,
    exponent: &str,
    known_exponent: Option<f64>,
) -> String {
    match known_exponent {
        Some(value) if value == 2.0 => emitter.binary("arith.mulf", base, base),
        Some(value) if value == 3.0 => {
            let square = emitter.binary("arith.mulf", base, base);
            emitter.binary("arith.mulf", &square, base)
        }
        Some(value) if value == 0.5 => emitter.unary("math.sqrt", base),
        Some(value) if value == -0.5 => {
            let one = emitter.f64(1.0);
            let root = emitter.unary("math.sqrt", base);
            emitter.binary("arith.divf", &one, &root)
        }
        Some(value) if value == -1.0 => {
            let one = emitter.f64(1.0);
            emitter.binary("arith.divf", &one, base)
        }
        Some(value) if value == -1.5 => {
            let one = emitter.f64(1.0);
            let root = emitter.unary("math.sqrt", base);
            let denominator = emitter.binary("arith.mulf", base, &root);
            emitter.binary("arith.divf", &one, &denominator)
        }
        Some(value) if value == -2.0 => {
            let one = emitter.f64(1.0);
            let square = emitter.binary("arith.mulf", base, base);
            emitter.binary("arith.divf", &one, &square)
        }
        _ => emitter.binary("math.powf", base, exponent),
    }
}

fn multiply_term(
    emitter: &mut Emitter,
    left: &Operand,
    left_index: usize,
    right: &Operand,
    right_index: usize,
) -> Option<String> {
    let left_value = &left.elements[left_index];
    let right_value = &right.elements[right_index];
    if known_f64(left, left_index).is_some_and(|value| value == 0.0)
        || known_f64(right, right_index).is_some_and(|value| value == 0.0)
    {
        return None;
    }
    if known_f64(left, left_index) == Some(1.0) {
        return Some(right_value.clone());
    }
    if known_f64(right, right_index) == Some(1.0) {
        return Some(left_value.clone());
    }
    if known_f64(left, left_index) == Some(-1.0) {
        return Some(emitter.unary("arith.negf", right_value));
    }
    if known_f64(right, right_index) == Some(-1.0) {
        return Some(emitter.unary("arith.negf", left_value));
    }
    Some(emitter.binary("arith.mulf", left_value, right_value))
}

fn sum(emitter: &mut Emitter, terms: &[String]) -> String {
    let Some((first, rest)) = terms.split_first() else {
        return emitter.f64(0.0);
    };
    rest.iter().fold(first.clone(), |sum, term| {
        emitter.binary("arith.addf", &sum, term)
    })
}

fn known_f64(operand: &Operand, index: usize) -> Option<f64> {
    if operand.element != ElementType::F64 {
        return None;
    }
    operand
        .known_elements
        .as_ref()?
        .get(index)
        .copied()
        .map(f64::from_bits)
}

fn known_one_based_index(operand: &Operand, index: usize) -> Result<usize, String> {
    if operand.element != ElementType::Index {
        return Err("MLIR prototype requires index-typed gather indices".to_owned());
    }
    let value = *operand
        .known_elements
        .as_ref()
        .and_then(|elements| elements.get(index))
        .ok_or_else(|| "MLIR prototype requires compile-time gather indices".to_owned())?;
    usize::try_from(value)
        .ok()
        .and_then(|value| value.checked_sub(1))
        .ok_or_else(|| format!("invalid one-based MLIR index {value}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn specialized_power_uses_structured_math_operations() {
        let mut emitter = Emitter::new();
        let result = emit_power(&mut emitter, "%x", "%exponent", Some(-1.5));
        assert!(emitter.body.contains("math.sqrt %x : f64"));
        assert!(emitter.body.contains("arith.mulf %x"));
        assert!(emitter.body.contains("arith.divf"));
        assert!(!emitter.body.contains("math.powf"));
        assert!(result.starts_with("%v"));
    }
}
