use libloading::Library;
use mech_core::snapshot::SequenceView;
use mech_core::{CellSlotId, DimensionExpr, ResidentShape, Value, ValueData};
use mech_engine::__resident::{ActivatedPlan, ResidentStorageClass};
use mech_engine::artifact::{
    ArtifactSource, BindingDeclaration, InitializerReference, ProgramArtifact, SlotRole,
};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

type InitFunction = unsafe extern "C" fn(*mut f64);
type TurnFunction = unsafe extern "C" fn(*const f64, *const f64, *mut f64);

pub struct AotProgram {
    _library: Library,
    initialize: InitFunction,
    turn: TurnFunction,
    state_len: usize,
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
        let generated = emit_rust(artifact, plan, input_slots)?;
        let directory = output_directory();
        fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
        let stem = format!("mech_ekf_aot_{}", std::process::id());
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
            state_len: state_len(plan)?,
            source_path,
            compile_time,
        })
    }

    pub fn turn(&self, state: &mut AotState, inputs: [f64; 4]) {
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
}

#[derive(Clone, Copy, Debug)]
struct Shape {
    rows: usize,
    columns: usize,
}

impl Shape {
    fn len(self) -> usize {
        self.rows * self.columns
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

fn state_len(plan: &ActivatedPlan) -> Result<usize, String> {
    plan.slots
        .iter()
        .filter(|slot| slot.storage == ResidentStorageClass::State)
        .try_fold(0usize, |len, slot| {
            if slot.region.kind != mech_core::ResidentValueKind::F64 {
                return Err("prototype only supports f64 state".to_string());
            }
            Ok(len.max(slot.region.offset + slot.region.len))
        })
}

fn emit_rust(
    artifact: &ProgramArtifact,
    plan: &ActivatedPlan,
    input_slots: &[CellSlotId],
) -> Result<String, String> {
    let state_len = state_len(plan)?;
    let input_by_slot = input_slots
        .iter()
        .copied()
        .enumerate()
        .map(|(ordinal, slot)| (slot, ordinal))
        .collect::<BTreeMap<_, _>>();
    let mut rust = String::new();
    writeln!(
        rust,
        "// Generated from the normal Mech ProgramArtifact. Do not edit."
    )
    .unwrap();
    writeln!(rust, "const INPUT_LEN: usize = {};", input_slots.len()).unwrap();
    writeln!(rust, "const STATE_LEN: usize = {state_len};\n").unwrap();

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
    for slot in plan
        .slots
        .iter()
        .filter(|slot| slot.storage == ResidentStorageClass::State)
    {
        let declaration = &artifact.slots()[slot.artifact_id.get() as usize];
        let Some(InitializerReference::Constant(constant)) = declaration.initializer else {
            return Err(format!(
                "state slot {} has no constant initializer",
                slot.artifact_id.get()
            ));
        };
        let values = value_elements(
            artifact
                .constants()
                .get(constant)
                .ok_or_else(|| format!("missing constant {}", constant.get()))?,
        )?;
        if values.len() != slot.region.len {
            return Err(format!(
                "initializer length mismatch for slot {}",
                slot.artifact_id.get()
            ));
        }
        for (ordinal, value) in values.iter().enumerate() {
            writeln!(
                rust,
                "    state[{}] = {value};",
                slot.region.offset + ordinal
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

    for slot in plan
        .slots
        .iter()
        .filter(|slot| slot.storage == ResidentStorageClass::State)
    {
        let elements = (0..slot.region.len)
            .map(|ordinal| format!("published[{}]", slot.region.offset + ordinal))
            .collect::<Vec<_>>();
        writeln!(
            rust,
            "    let s_{}: [f64; {}] = {};",
            slot.artifact_id.get(),
            slot.region.len,
            array(&elements),
        )
        .unwrap();
    }

    for node in artifact.nodes() {
        let output = node_output(artifact, node)?;
        let output_slot = &artifact.slots()[output.get() as usize];
        let output_shape = slot_shape(plan, output)?;
        if node.operation.module_path.as_ref() == ["resource", "read"] {
            let ordinal = input_by_slot
                .get(&output)
                .ok_or_else(|| format!("resource node {} is not a bound input", node.node.get()))?;
            writeln!(rust, "    // node {} resource/read", node.node.get()).unwrap();
            writeln!(
                rust,
                "    let v_{}: [f64; 1] = [inputs[{ordinal}]];",
                output.get()
            )
            .unwrap();
            continue;
        }
        let operands = node_inputs(artifact, plan, node)?;
        let expression = emit_operation(&node.operation.operation_name, &operands, output_shape)?;
        let variable = if output_slot.role == SlotRole::State {
            format!("next_{}", output.get())
        } else {
            format!("v_{}", output.get())
        };
        writeln!(
            rust,
            "    // node {} {}/{}",
            node.node.get(),
            node.operation.module_path.join("/"),
            node.operation.operation_name,
        )
        .unwrap();
        writeln!(
            rust,
            "    let {variable}: [f64; {}] = {expression};",
            output_shape.len(),
        )
        .unwrap();
    }

    for slot in plan
        .slots
        .iter()
        .filter(|slot| slot.storage == ResidentStorageClass::State)
    {
        for ordinal in 0..slot.region.len {
            writeln!(
                rust,
                "    candidate[{}] = next_{}[{ordinal}];",
                slot.region.offset + ordinal,
                slot.artifact_id.get(),
            )
            .unwrap();
        }
    }
    writeln!(rust, "}}").unwrap();
    Ok(rust)
}

fn node_output(
    artifact: &ProgramArtifact,
    node: &mech_engine::artifact::NodeDeclaration,
) -> Result<CellSlotId, String> {
    let outputs = &artifact.bindings()
        [node.output_bindings.start as usize..node.output_bindings.end as usize];
    let [BindingDeclaration::Output { target, .. }] = outputs else {
        return Err(format!("node {} must have one output", node.node.get()));
    };
    Ok(*target)
}

fn node_inputs(
    artifact: &ProgramArtifact,
    plan: &ActivatedPlan,
    node: &mech_engine::artifact::NodeDeclaration,
) -> Result<Vec<Operand>, String> {
    let mut inputs = artifact.bindings()
        [node.input_bindings.start as usize..node.input_bindings.end as usize]
        .iter()
        .map(|binding| match binding {
            BindingDeclaration::Input {
                port_ordinal,
                source,
                ..
            } => Ok((*port_ordinal, operand(artifact, plan, *source)?)),
            BindingDeclaration::Output { .. } => Err("output in input binding range".to_string()),
        })
        .collect::<Result<Vec<_>, String>>()?;
    inputs.sort_by_key(|(ordinal, _)| *ordinal);
    Ok(inputs.into_iter().map(|(_, operand)| operand).collect())
}

fn operand(
    artifact: &ProgramArtifact,
    plan: &ActivatedPlan,
    source: ArtifactSource,
) -> Result<Operand, String> {
    match source {
        ArtifactSource::Constant(constant) => {
            let value = artifact
                .constants()
                .get(constant)
                .ok_or_else(|| format!("missing constant {}", constant.get()))?;
            Ok(Operand {
                expression: array(&value_elements(value)?),
                shape: value_shape(artifact, value)?,
            })
        }
        ArtifactSource::Slot(slot) => {
            let declaration = &artifact.slots()[slot.get() as usize];
            Ok(Operand {
                expression: if declaration.role == SlotRole::State {
                    format!("s_{}", slot.get())
                } else {
                    format!("v_{}", slot.get())
                },
                shape: slot_shape(plan, slot)?,
            })
        }
    }
}

fn slot_shape(plan: &ActivatedPlan, slot: CellSlotId) -> Result<Shape, String> {
    let region: ResidentShape = plan
        .slots
        .get(slot.get() as usize)
        .ok_or_else(|| format!("missing resident slot {}", slot.get()))?
        .region
        .shape;
    Ok(Shape {
        rows: region.rows as usize,
        columns: region.columns as usize,
    })
}

fn value_shape(artifact: &ProgramArtifact, value: &Value) -> Result<Shape, String> {
    let schema = artifact
        .schemas()
        .entry(value.schema())
        .ok_or_else(|| "constant schema is missing".to_string())?
        .schema();
    match schema.body() {
        mech_core::SchemaBody::FloatingPoint(mech_core::FloatWidth::W64) => Ok(Shape {
            rows: 1,
            columns: 1,
        }),
        mech_core::SchemaBody::Matrix { dimensions, .. } if dimensions.len() == 2 => Ok(Shape {
            rows: evaluate_dimension(&dimensions[0], value.shape().parameter_values())? as usize,
            columns: evaluate_dimension(&dimensions[1], value.shape().parameter_values())? as usize,
        }),
        body => Err(format!("unsupported constant schema {body:?}")),
    }
}

fn evaluate_dimension(expression: &DimensionExpr, values: &[u64]) -> Result<u64, String> {
    match expression {
        DimensionExpr::Hole => Err("unresolved dimension".to_string()),
        DimensionExpr::Constant(value) => Ok(*value),
        DimensionExpr::Parameter(id) => values
            .get(id.get() as usize)
            .copied()
            .ok_or_else(|| "dimension parameter is missing".to_string()),
        DimensionExpr::Add(terms) => terms.iter().try_fold(0_u64, |sum, term| {
            sum.checked_add(evaluate_dimension(term, values)?)
                .ok_or_else(|| "dimension overflow".to_string())
        }),
        DimensionExpr::Multiply(terms) => terms.iter().try_fold(1_u64, |product, term| {
            product
                .checked_mul(evaluate_dimension(term, values)?)
                .ok_or_else(|| "dimension overflow".to_string())
        }),
        DimensionExpr::Min(terms) => terms
            .iter()
            .map(|term| evaluate_dimension(term, values))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .min()
            .ok_or_else(|| "empty minimum".to_string()),
        DimensionExpr::Max(terms) => terms
            .iter()
            .map(|term| evaluate_dimension(term, values))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .max()
            .ok_or_else(|| "empty maximum".to_string()),
    }
}

fn value_elements(value: &Value) -> Result<Vec<String>, String> {
    let bits = match value.data() {
        ValueData::F64(value) => vec![*value],
        ValueData::Matrix(matrix) => match matrix.elements() {
            SequenceView::F64(values) => values.to_vec(),
            other => return Err(format!("unsupported matrix constant {other:?}")),
        },
        other => return Err(format!("unsupported constant {other:?}")),
    };
    Ok(bits
        .into_iter()
        .map(|value| format!("f64::from_bits({}u64)", value.bits()))
        .collect())
}

fn emit_operation(name: &str, inputs: &[Operand], output: Shape) -> Result<String, String> {
    if name.starts_with("HorizontalConcatenate") {
        let mut elements = Vec::new();
        for input in inputs {
            if input.shape.rows != output.rows {
                return Err(format!("{name}: horizontal row mismatch"));
            }
            elements.extend((0..input.shape.len()).map(|index| at(input, index)));
        }
        return checked_array(name, elements, output.len());
    }
    if name.starts_with("VerticalConcatenate") {
        let mut elements = Vec::new();
        for column in 0..output.columns {
            for input in inputs {
                if input.shape.columns != output.columns {
                    return Err(format!("{name}: vertical column mismatch"));
                }
                for row in 0..input.shape.rows {
                    elements.push(at(input, row + column * input.shape.rows));
                }
            }
        }
        return checked_array(name, elements, output.len());
    }
    if name.starts_with("MatMul") {
        let [left, right] = inputs else {
            return Err(format!("{name}: expected two inputs"));
        };
        if left.shape.columns != right.shape.rows
            || output.rows != left.shape.rows
            || output.columns != right.shape.columns
        {
            return Err(format!("{name}: matrix product shape mismatch"));
        }
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
        return Ok(array(&elements));
    }
    if name.starts_with("Transpose") {
        let [input] = inputs else {
            return Err(format!("{name}: expected one input"));
        };
        let elements = (0..output.columns)
            .flat_map(|column| {
                (0..output.rows).map(move |row| at(input, column + row * input.shape.rows))
            })
            .collect::<Vec<_>>();
        return checked_array(name, elements, output.len());
    }
    if name.starts_with("Dot") {
        let [left, right] = inputs else {
            return Err(format!("{name}: expected two inputs"));
        };
        if left.shape.len() != right.shape.len() || output.len() != 1 {
            return Err(format!("{name}: dot shape mismatch"));
        }
        let sum = (0..left.shape.len())
            .map(|index| format!("{} * {}", at(left, index), at(right, index)))
            .collect::<Vec<_>>()
            .join(" + ");
        return Ok(format!("[{sum}]"));
    }
    if name.starts_with("Assign") {
        let [input] = inputs else {
            return Err(format!("{name}: expected one input"));
        };
        return checked_array(
            name,
            (0..input.shape.len())
                .map(|index| at(input, index))
                .collect(),
            output.len(),
        );
    }
    if name.starts_with("MathSin") || name.starts_with("MathCos") || name.starts_with("Negate") {
        let [input] = inputs else {
            return Err(format!("{name}: expected one input"));
        };
        let elements = (0..input.shape.len())
            .map(|index| {
                let value = at(input, index);
                if name.starts_with("MathSin") {
                    format!("({value}).sin()")
                } else if name.starts_with("MathCos") {
                    format!("({value}).cos()")
                } else {
                    format!("-({value})")
                }
            })
            .collect();
        return checked_array(name, elements, output.len());
    }
    if name.starts_with("Atan2") {
        let [left, right] = inputs else {
            return Err(format!("{name}: expected two inputs"));
        };
        return Ok(format!("[({}).atan2({})]", at(left, 0), at(right, 0)));
    }
    for (prefix, operator) in [("Add", "+"), ("Sub", "-"), ("Mul", "*"), ("Div", "/")] {
        if name.starts_with(prefix) {
            let [left, right] = inputs else {
                return Err(format!("{name}: expected two inputs"));
            };
            let elements = (0..output.len())
                .map(|index| {
                    let left_index = if left.shape.len() == 1 { 0 } else { index };
                    let right_index = if right.shape.len() == 1 { 0 } else { index };
                    format!(
                        "{} {operator} {}",
                        at(left, left_index),
                        at(right, right_index)
                    )
                })
                .collect::<Vec<_>>();
            return Ok(array(&elements));
        }
    }
    Err(format!("unsupported resident operation {name}"))
}

fn at(operand: &Operand, index: usize) -> String {
    format!("({})[{index}]", operand.expression)
}

fn checked_array(name: &str, elements: Vec<String>, expected: usize) -> Result<String, String> {
    if elements.len() != expected {
        return Err(format!(
            "{name}: generated {} elements, expected {expected}",
            elements.len(),
        ));
    }
    Ok(array(&elements))
}

fn array(elements: &[String]) -> String {
    format!("[{}]", elements.join(", "))
}
