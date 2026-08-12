use super::kernel_ir::{
    BatchLayoutKind, BinaryOperation, ElementType, KernelIr, Operation, Shape, Source,
    UnaryOperation, ValueStorage,
};
use std::collections::BTreeSet;
use std::fmt::Write as _;

#[derive(Clone)]
struct Operand {
    expression: String,
    shape: Shape,
    element: ElementType,
    known_elements: Option<Vec<u64>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OptimizationMode {
    // Preserve the explicit graph's IEEE operations for publishable candidates.
    Strict,
    // The single-buffer tier may reassociate fixed-shape finite numeric work.
    Fast,
}

pub(super) fn emit_rust(kernel: &KernelIr) -> Result<String, String> {
    emit_rust_with_precision(kernel, FloatPrecision::F64)
}

pub(super) fn emit_rust_f32(kernel: &KernelIr) -> Result<String, String> {
    emit_rust_with_precision(kernel, FloatPrecision::F32)
}

#[derive(Clone, Copy)]
enum FloatPrecision {
    F32,
    F64,
}

fn emit_rust_with_precision(
    kernel: &KernelIr,
    precision: FloatPrecision,
) -> Result<String, String> {
    let mut rust = String::new();
    writeln!(
        rust,
        "// Generated from backend-neutral Mech numeric kernel IR. Do not edit."
    )
    .unwrap();
    writeln!(rust, "pub const INPUT_LEN: usize = {};", kernel.input_len).unwrap();
    writeln!(rust, "pub const STATE_LEN: usize = {};\n", kernel.state_len).unwrap();
    let referenced_values = kernel
        .instructions
        .iter()
        .flat_map(|instruction| instruction.inputs.iter())
        .filter_map(|source| match source {
            Source::Value(value) => Some(*value),
            Source::Constant(_) => None,
        })
        .collect::<BTreeSet<_>>();
    for activation in kernel
        .activations
        .iter()
        .filter(|activation| referenced_values.contains(&activation.value))
    {
        let ty = kernel.value(activation.value).ty;
        let elements = activation
            .elements
            .iter()
            .map(|value| literal(ty.element, *value, precision))
            .collect::<Vec<_>>();
        writeln!(
            rust,
            "const A_{}: [{}; {}] = {};",
            activation.value.get(),
            rust_type(ty.element, precision),
            ty.shape.len(),
            array(&elements),
        )
        .unwrap();
    }
    writeln!(rust).unwrap();

    writeln!(
        rust,
        "pub fn initialize(state: &mut [{}]) {{",
        float_type(precision),
    )
    .unwrap();
    writeln!(rust, "    assert_eq!(state.len(), STATE_LEN);").unwrap();
    for state in &kernel.states {
        if matches!(
            kernel.batch,
            Some(batch) if batch.kind == BatchLayoutKind::OuterLift
        ) {
            let batch_len = kernel.batch.unwrap().len;
            for (component, bits) in state.initial_elements.iter().enumerate() {
                let start = state.offset + component * batch_len;
                writeln!(
                    rust,
                    "    state[{start}..{}].fill({});",
                    start + batch_len,
                    float_literal(*bits, precision),
                )
                .unwrap();
            }
        } else if state
            .initial_elements
            .windows(2)
            .all(|pair| pair[0] == pair[1])
        {
            writeln!(
                rust,
                "    state[{}..{}].fill({});",
                state.offset,
                state.offset + state.initial_elements.len(),
                float_literal(state.initial_elements[0], precision),
            )
            .unwrap();
        } else {
            for (ordinal, bits) in state.initial_elements.iter().enumerate() {
                writeln!(
                    rust,
                    "    state[{}] = {};",
                    state.offset + ordinal,
                    float_literal(*bits, precision),
                )
                .unwrap();
            }
        }
    }
    writeln!(rust, "}}\n").unwrap();

    writeln!(rust, "// Strict candidate-producing entry point.").unwrap();
    writeln!(rust, "#[inline(never)]").unwrap();
    writeln!(
        rust,
        "pub fn turn(inputs: &[{0}], published: &[{0}], candidate: &mut [{0}]) {{",
        float_type(precision),
    )
    .unwrap();
    writeln!(rust, "    assert_eq!(inputs.len(), INPUT_LEN);").unwrap();
    writeln!(rust, "    assert_eq!(published.len(), STATE_LEN);").unwrap();
    writeln!(rust, "    assert_eq!(candidate.len(), STATE_LEN);").unwrap();

    match kernel.batch {
        Some(batch) if batch.kind == BatchLayoutKind::MaterializedLaneVectors => {
            emit_materialized_batched_turn(
                &mut rust,
                kernel,
                batch.len,
                "published",
                "candidate",
                OptimizationMode::Strict,
                precision,
            )?;
        }
        Some(batch) => emit_outer_lifted_turn(
            &mut rust,
            kernel,
            batch.len,
            "published",
            "candidate",
            OptimizationMode::Strict,
            precision,
        )?,
        None => emit_scalarized_turn(
            &mut rust,
            kernel,
            "published",
            "candidate",
            OptimizationMode::Strict,
            precision,
        )?,
    }
    writeln!(rust, "}}").unwrap();

    writeln!(
        rust,
        "// Relaxed in-place entry point for the explicitly selected fast tier."
    )
    .unwrap();
    writeln!(rust, "#[inline(always)]").unwrap();
    writeln!(
        rust,
        "\npub fn turn_in_place(inputs: &[{0}], state: &mut [{0}]) {{",
        float_type(precision),
    )
    .unwrap();
    writeln!(rust, "    assert_eq!(inputs.len(), INPUT_LEN);").unwrap();
    writeln!(rust, "    assert_eq!(state.len(), STATE_LEN);").unwrap();
    match kernel.batch {
        Some(batch) if batch.kind == BatchLayoutKind::MaterializedLaneVectors => {
            emit_materialized_batched_turn(
                &mut rust,
                kernel,
                batch.len,
                "state",
                "state",
                OptimizationMode::Fast,
                precision,
            )?;
        }
        Some(batch) => emit_outer_lifted_turn(
            &mut rust,
            kernel,
            batch.len,
            "state",
            "state",
            OptimizationMode::Fast,
            precision,
        )?,
        None => emit_scalarized_turn(
            &mut rust,
            kernel,
            "state",
            "state",
            OptimizationMode::Fast,
            precision,
        )?,
    }
    writeln!(rust, "}}").unwrap();
    Ok(rust)
}

fn emit_scalarized_turn(
    rust: &mut String,
    kernel: &KernelIr,
    published: &str,
    candidate: &str,
    mode: OptimizationMode,
    precision: FloatPrecision,
) -> Result<(), String> {
    for state in &kernel.states {
        let elements = (0..state.initial_elements.len())
            .map(|ordinal| format!("{published}[{}]", state.offset + ordinal))
            .collect::<Vec<_>>();
        writeln!(
            rust,
            "    let s_{}: [{}; {}] = {};",
            state.value.get(),
            float_type(precision),
            state.initial_elements.len(),
            array(&elements),
        )
        .unwrap();
    }

    let mut written_states = BTreeSet::new();
    for instruction in &kernel.instructions {
        let output = kernel.value(instruction.output);
        let expression = match instruction.operation {
            Operation::Input { ordinal } => {
                let input = kernel.input(ordinal);
                let elements = (0..input.len)
                    .map(|element| format!("inputs[{}]", input.offset + element))
                    .collect::<Vec<_>>();
                array(&elements)
            }
            operation => {
                let operands = instruction
                    .inputs
                    .iter()
                    .map(|source| operand(kernel, source, &written_states, precision))
                    .collect::<Vec<_>>();
                emit_operation(operation, &operands, output.ty.shape, mode)?
            }
        };
        let variable = if output.storage == ValueStorage::State {
            format!("next_{}", instruction.output.get())
        } else {
            format!("v_{}", instruction.output.get())
        };
        writeln!(
            rust,
            "    // node {} {}",
            instruction.node, instruction.operation_name,
        )
        .unwrap();
        writeln!(
            rust,
            "    let {variable}: [{}; {}] = {expression};",
            rust_type(output.ty.element, precision),
            output.ty.shape.len(),
        )
        .unwrap();
        if output.storage == ValueStorage::State {
            written_states.insert(instruction.output);
        }
    }

    for state in &kernel.states {
        let variable = if kernel.state_is_written(state.value) {
            format!("next_{}", state.value.get())
        } else {
            format!("s_{}", state.value.get())
        };
        for ordinal in 0..state.initial_elements.len() {
            writeln!(
                rust,
                "    {candidate}[{}] = {variable}[{ordinal}];",
                state.offset + ordinal,
            )
            .unwrap();
        }
    }
    Ok(())
}

fn emit_materialized_batched_turn(
    rust: &mut String,
    kernel: &KernelIr,
    batch_len: usize,
    published: &str,
    candidate: &str,
    mode: OptimizationMode,
    precision: FloatPrecision,
) -> Result<(), String> {
    writeln!(rust, "    const BATCH_LEN: usize = {batch_len};").unwrap();
    writeln!(rust, "    for lane in 0..BATCH_LEN {{").unwrap();
    for state in &kernel.states {
        writeln!(
            rust,
            "        let s_{} = {published}[{} + lane];",
            state.value.get(),
            state.offset,
        )
        .unwrap();
    }
    let mut written_states = BTreeSet::new();
    for instruction in &kernel.instructions {
        let output = kernel.value(instruction.output);
        let expression = match instruction.operation {
            Operation::Input { ordinal } => {
                let input = kernel.input(ordinal);
                if input.len == 1 {
                    format!("inputs[{}]", input.offset)
                } else {
                    format!("inputs[{} + lane]", input.offset)
                }
            }
            Operation::Broadcast | Operation::Assign => {
                let [input] = instruction.inputs.as_slice() else {
                    return Err(format!(
                        "node {} has invalid lane unary arity",
                        instruction.node,
                    ));
                };
                lane_operand(kernel, input, &written_states, batch_len, precision)?
            }
            Operation::Unary(operation) => {
                let [input] = instruction.inputs.as_slice() else {
                    return Err(format!(
                        "node {} has invalid lane unary arity",
                        instruction.node,
                    ));
                };
                let value = lane_operand(kernel, input, &written_states, batch_len, precision)?;
                match operation {
                    UnaryOperation::Sin => format!("({value}).sin()"),
                    UnaryOperation::Cos => format!("({value}).cos()"),
                    UnaryOperation::Negate => format!("-({value})"),
                }
            }
            Operation::Atan2 => {
                let [left, right] = instruction.inputs.as_slice() else {
                    return Err(format!(
                        "node {} has invalid lane atan2 arity",
                        instruction.node,
                    ));
                };
                format!(
                    "({}).atan2({})",
                    lane_operand(kernel, left, &written_states, batch_len, precision)?,
                    lane_operand(kernel, right, &written_states, batch_len, precision)?,
                )
            }
            Operation::Binary(operation) => {
                let [left, right] = instruction.inputs.as_slice() else {
                    return Err(format!(
                        "node {} has invalid lane binary arity",
                        instruction.node,
                    ));
                };
                let left = lane_operand(kernel, left, &written_states, batch_len, precision)?;
                let right = lane_operand(kernel, right, &written_states, batch_len, precision)?;
                match operation {
                    BinaryOperation::Add => format!("{left} + {right}"),
                    BinaryOperation::Subtract => format!("{left} - {right}"),
                    BinaryOperation::Multiply => format!("{left} * {right}"),
                    BinaryOperation::Divide => format!("{left} / {right}"),
                    BinaryOperation::Power => emit_power(
                        &left,
                        &right,
                        known_scalar_f64(kernel, right_source(instruction)?),
                        mode,
                    ),
                }
            }
            operation => {
                return Err(format!(
                    "node {} operation {operation:?} is not lane-wise",
                    instruction.node,
                ));
            }
        };
        let variable = if output.storage == ValueStorage::State {
            format!("next_{}", instruction.output.get())
        } else {
            format!("v_{}", instruction.output.get())
        };
        writeln!(
            rust,
            "        // node {} {}",
            instruction.node, instruction.operation_name,
        )
        .unwrap();
        writeln!(
            rust,
            "        let {variable}: {} = {expression};",
            float_type(precision),
        )
        .unwrap();
        if output.storage == ValueStorage::State {
            written_states.insert(instruction.output);
        }
    }
    for state in &kernel.states {
        let variable = if kernel.state_is_written(state.value) {
            format!("next_{}", state.value.get())
        } else {
            format!("s_{}", state.value.get())
        };
        writeln!(
            rust,
            "        {candidate}[{} + lane] = {variable};",
            state.offset,
        )
        .unwrap();
    }
    writeln!(rust, "    }}").unwrap();
    Ok(())
}

fn emit_outer_lifted_turn(
    rust: &mut String,
    kernel: &KernelIr,
    batch_len: usize,
    published: &str,
    candidate: &str,
    mode: OptimizationMode,
    precision: FloatPrecision,
) -> Result<(), String> {
    writeln!(rust, "    const BATCH_LEN: usize = {batch_len};").unwrap();
    writeln!(rust, "    for lane in 0..BATCH_LEN {{").unwrap();
    for state in &kernel.states {
        let elements = (0..state.initial_elements.len())
            .map(|component| {
                format!(
                    "{published}[{} + {component} * BATCH_LEN + lane]",
                    state.offset,
                )
            })
            .collect::<Vec<_>>();
        writeln!(
            rust,
            "        let s_{}: [{}; {}] = {};",
            state.value.get(),
            float_type(precision),
            state.initial_elements.len(),
            array(&elements),
        )
        .unwrap();
    }

    let mut written_states = BTreeSet::new();
    for instruction in &kernel.instructions {
        let output = kernel.value(instruction.output);
        let expression = match instruction.operation {
            Operation::Input { ordinal } => {
                let input = kernel.input(ordinal);
                let elements = (0..input.len)
                    .map(|component| {
                        if input.per_lane {
                            format!("inputs[{} + {component} * BATCH_LEN + lane]", input.offset,)
                        } else {
                            format!("inputs[{}]", input.offset + component)
                        }
                    })
                    .collect::<Vec<_>>();
                array(&elements)
            }
            operation => {
                let operands = instruction
                    .inputs
                    .iter()
                    .map(|source| operand(kernel, source, &written_states, precision))
                    .collect::<Vec<_>>();
                emit_operation(operation, &operands, output.ty.shape, mode)?
            }
        };
        let variable = if output.storage == ValueStorage::State {
            format!("next_{}", instruction.output.get())
        } else {
            format!("v_{}", instruction.output.get())
        };
        writeln!(
            rust,
            "        // node {} {}",
            instruction.node, instruction.operation_name,
        )
        .unwrap();
        writeln!(
            rust,
            "        let {variable}: [{}; {}] = {expression};",
            rust_type(output.ty.element, precision),
            output.ty.shape.len(),
        )
        .unwrap();
        if output.storage == ValueStorage::State {
            written_states.insert(instruction.output);
        }
    }

    for state in &kernel.states {
        let variable = if kernel.state_is_written(state.value) {
            format!("next_{}", state.value.get())
        } else {
            format!("s_{}", state.value.get())
        };
        for component in 0..state.initial_elements.len() {
            writeln!(
                rust,
                "        {candidate}[{} + {component} * BATCH_LEN + lane] = {variable}[{component}];",
                state.offset,
            )
            .unwrap();
        }
    }
    writeln!(rust, "    }}").unwrap();
    Ok(())
}

fn lane_operand(
    kernel: &KernelIr,
    source: &Source,
    written_states: &BTreeSet<super::kernel_ir::ValueId>,
    batch_len: usize,
    precision: FloatPrecision,
) -> Result<String, String> {
    match source {
        Source::Constant(constant) => {
            let Some(value) = constant.elements.first().copied() else {
                return Err("lane constant is empty".to_string());
            };
            if constant.elements.iter().any(|element| *element != value) {
                return Err("lane constant is not uniform".to_string());
            }
            Ok(literal(constant.ty.element, value, precision))
        }
        Source::Value(id) => Ok(match kernel.value(*id).storage {
            ValueStorage::Activation => {
                if kernel.value(*id).ty.shape.len() == 1 {
                    format!("A_{}[0]", id.get())
                } else if kernel.value(*id).ty.shape.len() == batch_len {
                    format!("A_{}[lane]", id.get())
                } else {
                    return Err(format!(
                        "activation slot {} cannot be lane-lifted",
                        id.get()
                    ));
                }
            }
            ValueStorage::State if written_states.contains(id) => format!("next_{}", id.get()),
            ValueStorage::State => format!("s_{}", id.get()),
            ValueStorage::Temporary => format!("v_{}", id.get()),
        }),
    }
}

fn operand(
    kernel: &KernelIr,
    source: &Source,
    written_states: &BTreeSet<super::kernel_ir::ValueId>,
    precision: FloatPrecision,
) -> Operand {
    match source {
        Source::Constant(constant) => Operand {
            expression: array(
                &constant
                    .elements
                    .iter()
                    .map(|value| literal(constant.ty.element, *value, precision))
                    .collect::<Vec<_>>(),
            ),
            shape: constant.ty.shape,
            element: constant.ty.element,
            known_elements: Some(constant.elements.clone()),
        },
        Source::Value(id) => {
            let value = kernel.value(*id);
            Operand {
                expression: match value.storage {
                    ValueStorage::Activation => format!("A_{}", id.get()),
                    ValueStorage::State if written_states.contains(id) => {
                        format!("next_{}", id.get())
                    }
                    ValueStorage::State => format!("s_{}", id.get()),
                    ValueStorage::Temporary => format!("v_{}", id.get()),
                },
                shape: value.ty.shape,
                element: value.ty.element,
                known_elements: if value.storage == ValueStorage::Activation {
                    kernel
                        .activations
                        .iter()
                        .find(|activation| activation.value == *id)
                        .map(|activation| activation.elements.clone())
                } else {
                    None
                },
            }
        }
    }
}

fn emit_operation(
    operation: Operation,
    inputs: &[Operand],
    output: Shape,
    mode: OptimizationMode,
) -> Result<String, String> {
    match operation {
        Operation::Input { .. } => {
            Err("input operation reached numeric code generation".to_string())
        }
        Operation::Broadcast => {
            let [input] = inputs else {
                unreachable!("kernel IR validated broadcast arity")
            };
            Ok(array(&vec![at(input, 0); output.len()]))
        }
        Operation::HorizontalConcatenate => {
            let elements = inputs
                .iter()
                .flat_map(|input| (0..input.shape.len()).map(|index| at(input, index)))
                .collect::<Vec<_>>();
            checked_array(elements, output.len())
        }
        Operation::VerticalConcatenate => {
            let mut elements = Vec::new();
            for column in 0..output.columns {
                for input in inputs {
                    for row in 0..input.shape.rows {
                        elements.push(at(input, row + column * input.shape.rows));
                    }
                }
            }
            checked_array(elements, output.len())
        }
        Operation::MatrixMultiply => {
            let [left, right] = inputs else {
                unreachable!("kernel IR validated matrix multiplication arity")
            };
            let mut elements = Vec::with_capacity(output.len());
            for column in 0..output.columns {
                for row in 0..output.rows {
                    let terms = (0..left.shape.columns)
                        .filter_map(|inner| {
                            multiply_term(
                                left,
                                row + inner * left.shape.rows,
                                right,
                                inner + column * right.shape.rows,
                                mode,
                            )
                        })
                        .collect::<Vec<_>>();
                    elements.push(if terms.is_empty() {
                        "0.0".to_string()
                    } else {
                        terms.join(" + ")
                    });
                }
            }
            Ok(array(&elements))
        }
        Operation::Transpose => {
            let [input] = inputs else {
                unreachable!("kernel IR validated transpose arity")
            };
            checked_array(
                (0..output.columns)
                    .flat_map(|column| {
                        (0..output.rows).map(move |row| at(input, column + row * input.shape.rows))
                    })
                    .collect(),
                output.len(),
            )
        }
        Operation::Dot => {
            let [left, right] = inputs else {
                unreachable!("kernel IR validated dot arity")
            };
            let sum = (0..left.shape.len())
                .map(|index| format!("{} * {}", at(left, index), at(right, index)))
                .collect::<Vec<_>>()
                .join(" + ");
            Ok(format!("[{sum}]"))
        }
        Operation::Assign => {
            let [input] = inputs else {
                unreachable!("kernel IR validated assignment arity")
            };
            checked_array(
                (0..input.shape.len())
                    .map(|index| at(input, index))
                    .collect(),
                output.len(),
            )
        }
        Operation::Unary(operation) => {
            let [input] = inputs else {
                unreachable!("kernel IR validated unary arity")
            };
            let elements = (0..input.shape.len())
                .map(|index| {
                    let value = at(input, index);
                    match operation {
                        UnaryOperation::Sin => format!("({value}).sin()"),
                        UnaryOperation::Cos => format!("({value}).cos()"),
                        UnaryOperation::Negate => format!("-({value})"),
                    }
                })
                .collect();
            checked_array(elements, output.len())
        }
        Operation::Atan2 => {
            let [left, right] = inputs else {
                unreachable!("kernel IR validated atan2 arity")
            };
            let elements = (0..output.len())
                .map(|index| {
                    let left_index = if left.shape.len() == 1 { 0 } else { index };
                    let right_index = if right.shape.len() == 1 { 0 } else { index };
                    format!(
                        "({}).atan2({})",
                        at(left, left_index),
                        at(right, right_index),
                    )
                })
                .collect::<Vec<_>>();
            Ok(array(&elements))
        }
        Operation::Binary(operation) => {
            let [left, right] = inputs else {
                unreachable!("kernel IR validated binary arity")
            };
            let elements = (0..output.len())
                .map(|index| {
                    let left_index = if left.shape.len() == 1 { 0 } else { index };
                    let right_index = if right.shape.len() == 1 { 0 } else { index };
                    let known_exponent = known_f64(right, right_index);
                    let left = at(left, left_index);
                    let right = at(right, right_index);
                    match operation {
                        BinaryOperation::Add => format!("{left} + {right}"),
                        BinaryOperation::Subtract => format!("{left} - {right}"),
                        BinaryOperation::Multiply => format!("{left} * {right}"),
                        BinaryOperation::Divide => format!("{left} / {right}"),
                        BinaryOperation::Power => emit_power(&left, &right, known_exponent, mode),
                    }
                })
                .collect::<Vec<_>>();
            Ok(array(&elements))
        }
        Operation::MultiplyRows => {
            let [matrix, vector] = inputs else {
                unreachable!("kernel IR validated row multiplication arity")
            };
            let elements = (0..output.len())
                .map(|index| {
                    format!(
                        "{} * {}",
                        at(matrix, index),
                        at(vector, index % output.rows),
                    )
                })
                .collect::<Vec<_>>();
            Ok(array(&elements))
        }
        Operation::SumColumns => {
            let [input] = inputs else {
                unreachable!("kernel IR validated column reduction arity")
            };
            let elements = (0..input.shape.rows)
                .map(|row| {
                    (0..input.shape.columns)
                        .map(|column| at(input, row + column * input.shape.rows))
                        .collect::<Vec<_>>()
                        .join(" + ")
                })
                .collect::<Vec<_>>();
            checked_array(elements, output.len())
        }
        Operation::Gather1D => {
            let [source, indices] = inputs else {
                unreachable!("kernel IR validated gather arity")
            };
            let elements = (0..output.len())
                .map(|ordinal| dynamic_at(source, &zero_based(indices, ordinal)))
                .collect::<Vec<_>>();
            Ok(array(&elements))
        }
        Operation::RowsAllColumns => {
            let [source, indices] = inputs else {
                unreachable!("kernel IR validated row selection arity")
            };
            let mut elements = Vec::with_capacity(output.len());
            for column in 0..source.shape.columns {
                for ordinal in 0..indices.shape.len() {
                    let row = zero_based(indices, ordinal);
                    elements.push(dynamic_at(
                        source,
                        &format!("({row}) + {}", column * source.shape.rows),
                    ));
                }
            }
            Ok(array(&elements))
        }
        Operation::AddIndexedRows | Operation::SubtractIndexedRows => {
            let [base, source, indices] = inputs else {
                unreachable!("kernel IR validated indexed row update arity")
            };
            let operator = if operation == Operation::AddIndexedRows {
                "+="
            } else {
                "-="
            };
            let mut block = format!("{{ let mut result = {};", base.expression);
            for occurrence in 0..indices.shape.len() {
                let row = zero_based(indices, occurrence);
                for column in 0..output.columns {
                    let target = format!("({row}) + {}", column * output.rows);
                    let source_index = occurrence + column * source.shape.rows;
                    write!(
                        block,
                        " result[{target}] {operator} {};",
                        at(source, source_index),
                    )
                    .unwrap();
                }
            }
            block.push_str(" result }");
            Ok(block)
        }
    }
}

fn right_source(instruction: &super::kernel_ir::Instruction) -> Result<&Source, String> {
    instruction
        .inputs
        .get(1)
        .ok_or_else(|| format!("node {} has no right operand", instruction.node))
}

fn known_scalar_f64(kernel: &KernelIr, source: &Source) -> Option<f64> {
    let (element, elements) = match source {
        Source::Constant(constant) => (constant.ty.element, constant.elements.as_slice()),
        Source::Value(id) => {
            let value = kernel.value(*id);
            let activation = kernel
                .activations
                .iter()
                .find(|activation| activation.value == *id)?;
            (value.ty.element, activation.elements.as_slice())
        }
    };
    if element != ElementType::F64 {
        return None;
    }
    let bits = *elements.first()?;
    elements
        .iter()
        .all(|element| *element == bits)
        .then(|| f64::from_bits(bits))
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

fn emit_power(
    base: &str,
    exponent: &str,
    known_exponent: Option<f64>,
    mode: OptimizationMode,
) -> String {
    if mode == OptimizationMode::Fast {
        match known_exponent {
            Some(value) if value == 2.0 => {
                return format!("{{ let x = {base}; x * x }}");
            }
            Some(value) if value == 3.0 => {
                return format!("{{ let x = {base}; x * x * x }}");
            }
            Some(value) if value == 0.5 => {
                return format!("({base}).sqrt()");
            }
            Some(value) if value == -0.5 => {
                return format!("1.0 / ({base}).sqrt()");
            }
            Some(value) if value == -1.0 => {
                return format!("1.0 / ({base})");
            }
            Some(value) if value == -1.5 => {
                return format!("{{ let x = {base}; 1.0 / (x * x.sqrt()) }}");
            }
            Some(value) if value == -2.0 => {
                return format!("{{ let x = {base}; 1.0 / (x * x) }}");
            }
            _ => {}
        }
    }
    format!("({base}).powf({exponent})")
}

fn multiply_term(
    left: &Operand,
    left_index: usize,
    right: &Operand,
    right_index: usize,
    mode: OptimizationMode,
) -> Option<String> {
    let left_expression = at(left, left_index);
    let right_expression = at(right, right_index);
    if mode == OptimizationMode::Fast {
        if known_f64(left, left_index).is_some_and(|value| value == 0.0)
            || known_f64(right, right_index).is_some_and(|value| value == 0.0)
        {
            return None;
        }
        if known_f64(left, left_index) == Some(1.0) {
            return Some(right_expression);
        }
        if known_f64(right, right_index) == Some(1.0) {
            return Some(left_expression);
        }
        if known_f64(left, left_index) == Some(-1.0) {
            return Some(format!("-({right_expression})"));
        }
        if known_f64(right, right_index) == Some(-1.0) {
            return Some(format!("-({left_expression})"));
        }
    }
    Some(format!("{left_expression} * {right_expression}"))
}

fn at(operand: &Operand, index: usize) -> String {
    format!("({})[{index}]", operand.expression)
}

fn dynamic_at(operand: &Operand, index: &str) -> String {
    format!("({})[{index}]", operand.expression)
}

fn zero_based(indices: &Operand, ordinal: usize) -> String {
    debug_assert_eq!(indices.element, ElementType::Index);
    format!("(({})[{ordinal}] as usize - 1)", indices.expression)
}

fn checked_array(elements: Vec<String>, expected: usize) -> Result<String, String> {
    if elements.len() != expected {
        return Err(format!(
            "kernel IR/codegen mismatch: generated {} elements, expected {expected}",
            elements.len(),
        ));
    }
    Ok(array(&elements))
}

fn float_literal(bits: u64, precision: FloatPrecision) -> String {
    match precision {
        FloatPrecision::F32 => {
            let bits = (f64::from_bits(bits) as f32).to_bits();
            format!("f32::from_bits({bits}u32)")
        }
        FloatPrecision::F64 => format!("f64::from_bits({bits}u64)"),
    }
}

fn literal(element: ElementType, value: u64, precision: FloatPrecision) -> String {
    match element {
        ElementType::F64 => float_literal(value, precision),
        ElementType::Index => format!("{value}u64"),
    }
}

fn rust_type(element: ElementType, precision: FloatPrecision) -> &'static str {
    match element {
        ElementType::F64 => float_type(precision),
        ElementType::Index => "u64",
    }
}

fn float_type(precision: FloatPrecision) -> &'static str {
    match precision {
        FloatPrecision::F32 => "f32",
        FloatPrecision::F64 => "f64",
    }
}

fn array(elements: &[String]) -> String {
    format!("[{}]", elements.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matrix_multiply_codegen_uses_column_major_shapes() {
        let left = Operand {
            expression: "left".to_string(),
            element: ElementType::F64,
            shape: Shape {
                rows: 2,
                columns: 3,
            },
            known_elements: None,
        };
        let right = Operand {
            expression: "right".to_string(),
            element: ElementType::F64,
            shape: Shape {
                rows: 3,
                columns: 1,
            },
            known_elements: None,
        };
        let expression = emit_operation(
            Operation::MatrixMultiply,
            &[left, right],
            Shape {
                rows: 2,
                columns: 1,
            },
            OptimizationMode::Strict,
        )
        .unwrap();
        assert_eq!(
            expression,
            "[(left)[0] * (right)[0] + (left)[2] * (right)[1] + (left)[4] * (right)[2], (left)[1] * (right)[0] + (left)[3] * (right)[1] + (left)[5] * (right)[2]]",
        );
    }

    #[test]
    fn fast_power_codegen_specializes_negative_three_halves() {
        assert_eq!(
            emit_power("x", "exponent", Some(-1.5), OptimizationMode::Fast),
            "{ let x = x; 1.0 / (x * x.sqrt()) }",
        );
        assert_eq!(
            emit_power("x", "exponent", Some(-1.5), OptimizationMode::Strict),
            "(x).powf(exponent)",
        );
    }

    #[test]
    fn fast_matrix_multiply_elides_constant_zero_and_one_terms() {
        let left = Operand {
            expression: "left".to_string(),
            element: ElementType::F64,
            shape: Shape {
                rows: 2,
                columns: 2,
            },
            known_elements: Some(vec![
                1.0_f64.to_bits(),
                0.0_f64.to_bits(),
                0.0_f64.to_bits(),
                1.0_f64.to_bits(),
            ]),
        };
        let right = Operand {
            expression: "right".to_string(),
            element: ElementType::F64,
            shape: Shape {
                rows: 2,
                columns: 1,
            },
            known_elements: None,
        };
        let expression = emit_operation(
            Operation::MatrixMultiply,
            &[left, right],
            Shape {
                rows: 2,
                columns: 1,
            },
            OptimizationMode::Fast,
        )
        .unwrap();
        assert_eq!(expression, "[(right)[0], (right)[1]]");
    }
}
