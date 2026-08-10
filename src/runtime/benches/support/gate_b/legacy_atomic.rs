use std::collections::VecDeque;
use std::sync::Arc;

use mech_core::structures::Matrix as MechMatrix;
use mech_core::{
    FunctionCatalog, FunctionCatalogBuilder, FunctionExport, FunctionExposure, FunctionSpecializer,
    GenericError, InitialSolvePolicy, LegacyValue, MResult, MechError, MechFunction,
    MechFunctionImpl, Ref,
};
use mech_runtime::{
    InMemoryStore, MechRuntime, ResourcePathCapability, RuntimeBuilder, RuntimeContext,
    RuntimeHostInput, RuntimeHostInputDriver, RuntimeHostInputSource, RuntimeHostInputUpdate,
    RuntimeHostInputValue, RuntimeIngress, RuntimeResourceProvider, RuntimeResourceReadRequest,
    SequentialIdGenerator,
};

use super::contract::{
    EkfState, SCALED_INSTANCES, quantized_trajectory_hash, reference_trajectory, trace,
};
use super::full_write::{ELEMENTS, SIDE, coefficients, initial_values, write_next};
use super::raw_kernel;

const EKF_BASE_URI: &str = "bench://gate-b/ekf";
const FULL_WRITE_BASE_URI: &str = "bench://gate-b/full-write";
const EKF_PATHS: [&str; 4] = ["v", "omega", "z-range", "z-bearing"];

fn f64_ref(value: &LegacyValue) -> Ref<f64> {
    match value {
        LegacyValue::F64(value) => value.clone(),
        LegacyValue::MutableReference(value) => match &*value.borrow() {
            LegacyValue::F64(value) => value.clone(),
            other => panic!("expected Gate B f64 reference, got {other:?}"),
        },
        other => panic!("expected Gate B f64 argument, got {other:?}"),
    }
}

struct LegacyEkfFunction {
    velocity: Ref<f64>,
    angular_velocity: Ref<f64>,
    measured_range: Ref<f64>,
    measured_bearing: Ref<f64>,
    output: MechMatrix<f64>,
}

impl MechFunctionImpl for LegacyEkfFunction {
    fn solve_result(&self) -> MResult<()> {
        let MechMatrix::DMatrix(output) = &self.output else {
            unreachable!("3x4 Gate B output uses dynamic column-major storage")
        };
        let current = {
            let output = output.borrow();
            EkfState::from_values(output.as_slice())
        };
        let input = super::contract::EkfInput {
            velocity: *self.velocity.borrow(),
            angular_velocity: *self.angular_velocity.borrow(),
            measured_range: *self.measured_range.borrow(),
            measured_bearing: *self.measured_bearing.borrow(),
        };
        let candidate = raw_kernel::step(current, input).map_err(|error| {
            MechError::new(
                GenericError {
                    msg: format!("Gate B EKF integrity failure: {}", error.reason),
                },
                None,
            )
        })?;
        output
            .borrow_mut()
            .as_mut_slice()
            .copy_from_slice(&candidate.values());
        Ok(())
    }

    fn initial_solve_policy(&self) -> InitialSolvePolicy {
        InitialSolvePolicy::PreserveSpecializedOutput
    }

    fn out(&self) -> LegacyValue {
        LegacyValue::MatrixF64(self.output.clone())
    }

    fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
        Ok(self.reactive_output_values())
    }

    fn to_string(&self) -> String {
        "GateBEkfLegacyAtomic".to_string()
    }
}

struct LegacyEkfSpecializer;

impl FunctionSpecializer for LegacyEkfSpecializer {
    fn specialize(&self, arguments: &[LegacyValue]) -> MResult<Box<dyn MechFunction>> {
        let [velocity, angular_velocity, measured_range, measured_bearing] = arguments else {
            panic!("Gate B EKF expects four f64 arguments")
        };
        Ok(Box::new(LegacyEkfFunction {
            velocity: f64_ref(velocity),
            angular_velocity: f64_ref(angular_velocity),
            measured_range: f64_ref(measured_range),
            measured_bearing: f64_ref(measured_bearing),
            output: MechMatrix::from_vec(EkfState::INITIAL.values().to_vec(), 3, 4),
        }))
    }
}

struct LegacyFullWriteFunction {
    input: Ref<f64>,
    output: MechMatrix<f64>,
    coefficient: Box<[f64]>,
}

impl MechFunctionImpl for LegacyFullWriteFunction {
    fn solve_result(&self) -> MResult<()> {
        let MechMatrix::DMatrix(output) = &self.output else {
            unreachable!("64x64 Gate B output uses dynamic column-major storage")
        };
        let mut next = vec![0.0; ELEMENTS];
        {
            let current = output.borrow();
            write_next(
                current.as_slice(),
                &self.coefficient,
                *self.input.borrow(),
                &mut next,
            );
        }
        output.borrow_mut().as_mut_slice().copy_from_slice(&next);
        Ok(())
    }

    fn initial_solve_policy(&self) -> InitialSolvePolicy {
        InitialSolvePolicy::PreserveSpecializedOutput
    }

    fn out(&self) -> LegacyValue {
        LegacyValue::MatrixF64(self.output.clone())
    }

    fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
        Ok(self.reactive_output_values())
    }

    fn to_string(&self) -> String {
        "GateBFullWriteLegacyAtomic".to_string()
    }
}

struct LegacyFullWriteSpecializer;

impl FunctionSpecializer for LegacyFullWriteSpecializer {
    fn specialize(&self, arguments: &[LegacyValue]) -> MResult<Box<dyn MechFunction>> {
        let [input] = arguments else {
            panic!("Gate B full-write control expects one f64 argument")
        };
        Ok(Box::new(LegacyFullWriteFunction {
            input: f64_ref(input),
            output: MechMatrix::from_vec(initial_values(), SIDE, SIDE),
            coefficient: coefficients().into_boxed_slice(),
        }))
    }
}

fn catalog() -> Arc<FunctionCatalog> {
    let mut builder = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_runtime(&mut builder)
        .expect("engine intrinsic runtime fragment must be valid");
    mech_engine::install_intrinsic_source(&mut builder)
        .expect("engine intrinsic source fragment must be valid");
    let ekf_operation = builder
        .insert_specializer("gate-b/ekf", Arc::new(LegacyEkfSpecializer))
        .expect("Gate B EKF specializer must be unique");
    builder
        .insert_export(FunctionExport {
            operation: ekf_operation,
            canonical_name: "gate-b/ekf".to_string(),
            module: None,
            item: None,
            exposure: FunctionExposure::Prelude,
        })
        .expect("Gate B EKF source export");
    let full_write_operation = builder
        .insert_specializer("gate-b/full-write", Arc::new(LegacyFullWriteSpecializer))
        .expect("Gate B full-write specializer must be unique");
    builder
        .insert_export(FunctionExport {
            operation: full_write_operation,
            canonical_name: "gate-b/full-write".to_string(),
            module: None,
            item: None,
            exposure: FunctionExposure::Prelude,
        })
        .expect("Gate B full-write source export");
    Arc::new(builder.build().expect("Gate B source catalog"))
}

#[derive(Debug)]
struct GateBInputProvider;

#[derive(Debug, Default)]
struct GateBInputDriver;

impl RuntimeHostInputDriver for GateBInputDriver {
    fn drives(&self, source: &RuntimeHostInputSource) -> bool {
        (source.base_uri() == EKF_BASE_URI && EKF_PATHS.contains(&source.path()))
            || (source.base_uri() == FULL_WRITE_BASE_URI && source.path() == "input")
    }

    fn attach(&mut self, _ingress: RuntimeIngress) -> MResult<()> {
        Ok(())
    }

    fn start(&mut self) -> MResult<()> {
        Ok(())
    }

    fn stop(&mut self) -> MResult<()> {
        Ok(())
    }

    fn is_live(&self) -> bool {
        true
    }
}

impl RuntimeResourceProvider for GateBInputProvider {
    fn scheme(&self) -> &str {
        "bench"
    }

    fn base_uris(&self) -> Vec<String> {
        vec![EKF_BASE_URI.to_string(), FULL_WRITE_BASE_URI.to_string()]
    }

    fn read(&self, request: RuntimeResourceReadRequest) -> MResult<LegacyValue> {
        self.plan_read(request)
    }

    fn plan_read(&self, request: RuntimeResourceReadRequest) -> MResult<LegacyValue> {
        let ekf = request.base_uri == EKF_BASE_URI && EKF_PATHS.contains(&request.path.as_str());
        let full_write =
            request.base_uri == FULL_WRITE_BASE_URI && request.path.as_str() == "input";
        if ekf || full_write {
            return Ok(LegacyValue::F64(Ref::new(0.0)));
        }
        Err(MechError::new(
            GenericError {
                msg: format!(
                    "missing Gate B benchmark input {} / {}",
                    request.base_uri, request.path
                ),
            },
            None,
        ))
    }
}

fn runtime() -> MechRuntime {
    RuntimeBuilder::new()
        .function_catalog(catalog())
        .id_generator(SequentialIdGenerator::starting_at(1))
        .store(InMemoryStore::new())
        .resource_provider(Box::new(GateBInputProvider))
        .input_driver(GateBInputDriver)
        .build()
        .expect("Gate B legacy runtime")
}

fn grant_read(runtime: &mut MechRuntime, base_uri: &str, path: &str) {
    let subject = runtime
        .runtime_context()
        .expect("Gate B runtime context")
        .subject()
        .to_string();
    let capability = ResourcePathCapability::exact(
        runtime.next_capability_id(),
        subject,
        base_uri,
        ["read"],
        path,
    )
    .expect("Gate B exact read capability");
    runtime
        .grant_capability(Arc::new(capability))
        .expect("Gate B read grant");
}

fn ekf_packet_sources() -> [RuntimeHostInputSource; 4] {
    EKF_PATHS.map(|path| {
        RuntimeHostInputSource::new(EKF_BASE_URI, path).expect("Gate B EKF input source")
    })
}

fn ekf_packet(
    sources: &[RuntimeHostInputSource; 4],
    input: super::contract::EkfInput,
) -> RuntimeHostInput {
    RuntimeHostInput::new(vec![
        RuntimeHostInputUpdate {
            source: sources[0].clone(),
            value: RuntimeHostInputValue::F64(input.velocity),
        },
        RuntimeHostInputUpdate {
            source: sources[1].clone(),
            value: RuntimeHostInputValue::F64(input.angular_velocity),
        },
        RuntimeHostInputUpdate {
            source: sources[2].clone(),
            value: RuntimeHostInputValue::F64(input.measured_range),
        },
        RuntimeHostInputUpdate {
            source: sources[3].clone(),
            value: RuntimeHostInputValue::F64(input.measured_bearing),
        },
    ])
    .expect("Gate B four-update packet")
}

fn matrix_values(value: LegacyValue) -> Vec<f64> {
    let LegacyValue::MatrixF64(MechMatrix::DMatrix(matrix)) = value else {
        panic!("expected Gate B dynamic f64 matrix output")
    };
    matrix.borrow().as_slice().to_vec()
}

pub struct LegacyEkfFixture {
    runtime: MechRuntime,
    context: RuntimeContext,
    packets: VecDeque<RuntimeHostInput>,
    outputs: Vec<String>,
}

impl LegacyEkfFixture {
    pub fn new(instances: usize) -> Self {
        assert!(SCALED_INSTANCES.contains(&instances));
        let mut runtime = runtime();
        for path in EKF_PATHS {
            grant_read(&mut runtime, EKF_BASE_URI, path);
        }
        let mut source = format!(
            "@input := {EKF_BASE_URI}{{:read(v), :read(omega), :read(z-range), :read(z-bearing)}}\n"
        );
        let outputs = (0..instances)
            .map(|index| format!("ekf-{index}"))
            .collect::<Vec<_>>();
        for output in &outputs {
            source.push_str(&format!(
                "{output} := gate-b/ekf(@input/v, @input/omega, @input/z-range, @input/z-bearing)\n"
            ));
        }
        let mut activation_context = runtime.runtime_context().expect("Gate B runtime context");
        runtime
            .run_string_with_context(&mut activation_context, &source)
            .expect("activate Gate B legacy EKF source");
        let context = activation_context;
        let sources = ekf_packet_sources();
        let packets = trace()
            .iter()
            .copied()
            .map(|input| ekf_packet(&sources, input))
            .collect();
        Self {
            runtime,
            context,
            packets,
            outputs,
        }
    }

    pub fn run_episode(&mut self) {
        while let Some(packet) = self.packets.pop_front() {
            self.runtime
                .apply_host_input_with_context(&mut self.context, packet)
                .expect("Gate B ordinary atomic turn");
        }
    }

    pub fn run_and_validate_every_turn(&mut self) -> String {
        let mut trajectory = Vec::with_capacity(super::contract::EPISODE_LENGTH);
        for (turn, expected) in reference_trajectory().iter().enumerate() {
            let packet = self.packets.pop_front().expect("Gate B packet");
            self.runtime
                .apply_host_input_with_context(&mut self.context, packet)
                .unwrap_or_else(|error| {
                    panic!("Gate B ordinary atomic turn {}: {error:?}", turn + 1)
                });
            let states = self.states();
            for actual in &states {
                super::contract::assert_state_close(*actual, *expected, turn + 1);
            }
            trajectory.push(states[0]);
        }
        quantized_trajectory_hash(&trajectory)
    }

    pub fn states(&self) -> Vec<EkfState> {
        self.outputs
            .iter()
            .map(|output| {
                let snapshot = self
                    .runtime
                    .root_symbol_value(output)
                    .expect("Gate B legacy output");
                EkfState::from_values(&matrix_values(snapshot.to_value()))
            })
            .collect()
    }
}

pub struct LegacyFullWriteFixture {
    runtime: MechRuntime,
    context: RuntimeContext,
    packets: VecDeque<RuntimeHostInput>,
}

impl LegacyFullWriteFixture {
    pub fn new() -> Self {
        let mut runtime = runtime();
        grant_read(&mut runtime, FULL_WRITE_BASE_URI, "input");
        let mut activation_context = runtime.runtime_context().expect("Gate B runtime context");
        runtime
            .run_string_with_context(
                &mut activation_context,
                &format!(
                    "@input := {FULL_WRITE_BASE_URI}{{:read(input)}}\nfull-write := gate-b/full-write(@input/input)"
                ),
            )
            .expect("activate Gate B legacy full-write source");
        let context = activation_context;
        let source = RuntimeHostInputSource::new(FULL_WRITE_BASE_URI, "input")
            .expect("Gate B full-write source");
        let packets = trace()
            .iter()
            .map(|input| {
                RuntimeHostInput::single(source.clone(), RuntimeHostInputValue::F64(input.velocity))
            })
            .collect();
        Self {
            runtime,
            context,
            packets,
        }
    }

    pub fn run_episode(&mut self) {
        while let Some(packet) = self.packets.pop_front() {
            self.runtime
                .apply_host_input_with_context(&mut self.context, packet)
                .expect("Gate B legacy full-write turn");
        }
    }

    pub fn published(&self) -> Vec<f64> {
        let snapshot = self
            .runtime
            .root_symbol_value("full-write")
            .expect("Gate B legacy full-write output");
        matrix_values(snapshot.to_value())
    }
}
