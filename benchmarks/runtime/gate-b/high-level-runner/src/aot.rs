use crate::kernel_ir::{
    BinaryOperation, KernelIr, Operation, Shape, Source, UnaryOperation, ValueStorage,
};
use libloading::Library;
use mech_core::CellSlotId;
use mech_engine::__resident::ActivatedPlan;
use mech_engine::artifact::ProgramArtifact;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

type InitFunction = unsafe extern "C" fn(*mut f64);
type TurnFunction = unsafe extern "C" fn(*const f64, *const f64, *mut f64);

static OUTPUT_SERIAL: AtomicU64 = AtomicU64::new(0);

pub struct AotProgram {
    _library: Library,
    initialize: InitFunction,
    turn: TurnFunction,
    input_len: usize,
    state_len: usize,
    instruction_count: usize,
    source_path: PathBuf,
    compile_time: Duration,
}

pub struct AotState {
    buffers: [Vec<f64>; 2],
    published: usize,
}

impl AotState {
    pub fn new(program: &AotProgram) -> Self {
        let mut initial = vec![0.0; program.state_len];
        unsafe { (program.initialize)(initial.as_mut_ptr()) };
        Self {
            buffers: [initial.clone(), initial],
            published: 0,
        }
    }

    pub fn values(&self) -> &[f64] {
        &self.buffers[self.published]
    }
}

impl AotProgram {
    pub fn build(
        artifact: &ProgramArtifact,
        plan: &ActivatedPlan,
        input_slots: &[CellSlotId],
    ) -> Result<Self, String> {
        let kernel =
            KernelIr::lower(artifact, plan, input_slots).map_err(|error| error.to_string())?;
        Self::build_kernel(&kernel)
    }

    fn build_kernel(kernel: &KernelIr) -> Result<Self, String> {
        let generated = emit_rust(kernel)?;
        let directory = output_directory();
        fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
        let serial = OUTPUT_SERIAL.fetch_add(1, Ordering::Relaxed);
        let stem = format!("mech_program_aot_{}_{}", std::process::id(), serial);
        let source_path = directory.join(format!("{stem}.rs"));
        let library_path = directory.join(dynamic_library_name(&stem));
        fs::write(&source_path, generated).map_err(|error| error.to_string())?;

        let started = Instant::now();
        let output = Command::new("rustc")
            .arg(&source_path)
            .arg("--edition=2021")
            .arg("--crate-type=cdylib")
            .arg("-C")
            .arg("opt-level=3")
            .arg("-C")
            .arg("codegen-units=1")
            .arg("-C")
            .arg("panic=abort")
            .arg("-o")
            .arg(&library_path)
            .output()
            .map_err(|error| format!("could not invoke rustc: {error}"))?;
        let compile_time = started.elapsed();
        if !output.status.success() {
            return Err(format!(
                "rustc exited with {}\n{}",
                output.status,
                String::from_utf8_lossy(&output.stderr),
            ));
        }

        let library = unsafe { Library::new(&library_path) }
            .map_err(|error| format!("could not load {}: {error}", library_path.display()))?;
        let initialize = unsafe {
            *library
                .get::<InitFunction>(b"mech_aot_initialize")
                .map_err(|error| error.to_string())?
        };
        let turn = unsafe {
            *library
                .get::<TurnFunction>(b"mech_aot_turn")
                .map_err(|error| error.to_string())?
        };
        Ok(Self {
            _library: library,
            initialize,
            turn,
            input_len: kernel.input_count,
            state_len: kernel.state_len,
            instruction_count: kernel.instructions.len(),
            source_path,
            compile_time,
        })
    }

    pub fn turn(&self, state: &mut AotState, inputs: &[f64]) {
        assert_eq!(inputs.len(), self.input_len);
        assert_eq!(state.buffers[0].len(), self.state_len);
        let published = state.published;
        let candidate = 1 - published;
        let [left, right] = &mut state.buffers;
        let (input_state, output_state) = if published == 0 {
            (&*left, &mut *right)
        } else {
            (&*right, &mut *left)
        };
        unsafe {
            (self.turn)(
                inputs.as_ptr(),
                input_state.as_ptr(),
                output_state.as_mut_ptr(),
            )
        };
        state.published = candidate;
    }

    pub fn source_path(&self) -> &Path {
        &self.source_path
    }

    pub fn compile_time(&self) -> Duration {
        self.compile_time
    }

    pub fn instruction_count(&self) -> usize {
        self.instruction_count
    }
}

#[derive(Clone)]
struct Operand {
    expression: String,
    shape: Shape,
}

fn output_directory() -> PathBuf {
    std::env::var_os("MECH_AOT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("mech-aot-prototype"))
}

fn dynamic_library_name(stem: &str) -> String {
    if cfg!(target_os = "windows") {
        format!("{stem}.dll")
    } else if cfg!(target_os = "macos") {
        format!("lib{stem}.dylib")
    } else {
        format!("lib{stem}.so")
    }
}

fn emit_rust(kernel: &KernelIr) -> Result<String, String> {
    let mut rust = String::new();
    writeln!(
        rust,
        "// Generated from backend-neutral Mech numeric kernel IR. Do not edit."
    )
    .unwrap();
    writeln!(rust, "const INPUT_LEN: usize = {};", kernel.input_count).unwrap();
    writeln!(rust, "const STATE_LEN: usize = {};\n", kernel.state_len).unwrap();

    writeln!(rust, "#[no_mangle]").unwrap();
    writeln!(
        rust,
        "pub unsafe extern \"C\" fn mech_aot_initialize(state: *mut f64) {{"
    )
    .unwrap();
    writeln!(
        rust,
        "    let state = core::slice::from_raw_parts_mut(state, STATE_LEN);"
    )
    .unwrap();
    for state in &kernel.states {
        for (ordinal, bits) in state.initial_elements.iter().enumerate() {
            writeln!(
                rust,
                "    state[{}] = {};",
                state.offset + ordinal,
                float_literal(*bits),
            )
            .unwrap();
        }
    }
    writeln!(rust, "}}\n").unwrap();

    writeln!(rust, "#[no_mangle]").unwrap();
    writeln!(
        rust,
        "pub unsafe extern \"C\" fn mech_aot_turn(inputs: *const f64, published: *const f64, candidate: *mut f64) {{"
    )
    .unwrap();
    writeln!(
        rust,
        "    let inputs = core::slice::from_raw_parts(inputs, INPUT_LEN);"
    )
    .unwrap();
    writeln!(
        rust,
        "    let published = core::slice::from_raw_parts(published, STATE_LEN);"
    )
    .unwrap();
    writeln!(
        rust,
        "    let candidate = core::slice::from_raw_parts_mut(candidate, STATE_LEN);"
    )
    .unwrap();

    for state in &kernel.states {
        let elements = (0..state.initial_elements.len())
            .map(|ordinal| format!("published[{}]", state.offset + ordinal))
            .collect::<Vec<_>>();
        writeln!(
            rust,
            "    let s_{}: [f64; {}] = {};",
            state.value.get(),
            state.initial_elements.len(),
            array(&elements),
        )
        .unwrap();
    }

    for instruction in &kernel.instructions {
        let output = kernel.value(instruction.output);
        let expression = match instruction.operation {
            Operation::Input { ordinal } => format!("[inputs[{ordinal}]]"),
            operation => {
                let operands = instruction
                    .inputs
                    .iter()
                    .map(|source| operand(kernel, source))
                    .collect::<Vec<_>>();
                emit_operation(operation, &operands, output.ty.shape)?
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
            "    let {variable}: [f64; {}] = {expression};",
            output.ty.shape.len(),
        )
        .unwrap();
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
                "    candidate[{}] = {variable}[{ordinal}];",
                state.offset + ordinal,
            )
            .unwrap();
        }
    }
    writeln!(rust, "}}").unwrap();
    Ok(rust)
}

fn operand(kernel: &KernelIr, source: &Source) -> Operand {
    match source {
        Source::Constant(constant) => Operand {
            expression: array(
                &constant
                    .elements
                    .iter()
                    .map(|bits| float_literal(*bits))
                    .collect::<Vec<_>>(),
            ),
            shape: constant.ty.shape,
        },
        Source::Value(id) => {
            let value = kernel.value(*id);
            Operand {
                expression: match value.storage {
                    ValueStorage::State => format!("s_{}", id.get()),
                    ValueStorage::Temporary => format!("v_{}", id.get()),
                },
                shape: value.ty.shape,
            }
        }
    }
}

fn emit_operation(
    operation: Operation,
    inputs: &[Operand],
    output: Shape,
) -> Result<String, String> {
    match operation {
        Operation::Input { .. } => {
            Err("input operation reached numeric code generation".to_string())
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
                        .map(|inner| {
                            format!(
                                "{} * {}",
                                at(left, row + inner * left.shape.rows),
                                at(right, inner + column * right.shape.rows),
                            )
                        })
                        .collect::<Vec<_>>();
                    elements.push(terms.join(" + "));
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
            Ok(format!("[({}).atan2({})]", at(left, 0), at(right, 0)))
        }
        Operation::Binary(operation) => {
            let [left, right] = inputs else {
                unreachable!("kernel IR validated binary arity")
            };
            let operator = match operation {
                BinaryOperation::Add => "+",
                BinaryOperation::Subtract => "-",
                BinaryOperation::Multiply => "*",
                BinaryOperation::Divide => "/",
            };
            let elements = (0..output.len())
                .map(|index| {
                    let left_index = if left.shape.len() == 1 { 0 } else { index };
                    let right_index = if right.shape.len() == 1 { 0 } else { index };
                    format!(
                        "{} {operator} {}",
                        at(left, left_index),
                        at(right, right_index),
                    )
                })
                .collect::<Vec<_>>();
            Ok(array(&elements))
        }
    }
}

fn at(operand: &Operand, index: usize) -> String {
    format!("({})[{index}]", operand.expression)
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

fn float_literal(bits: u64) -> String {
    format!("f64::from_bits({bits}u64)")
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
            shape: Shape {
                rows: 2,
                columns: 3,
            },
        };
        let right = Operand {
            expression: "right".to_string(),
            shape: Shape {
                rows: 3,
                columns: 1,
            },
        };
        let expression = emit_operation(
            Operation::MatrixMultiply,
            &[left, right],
            Shape {
                rows: 2,
                columns: 1,
            },
        )
        .unwrap();
        assert_eq!(
            expression,
            "[(left)[0] * (right)[0] + (left)[2] * (right)[1] + (left)[4] * (right)[2], (left)[1] * (right)[0] + (left)[3] * (right)[1] + (left)[5] * (right)[2]]",
        );
    }
}
