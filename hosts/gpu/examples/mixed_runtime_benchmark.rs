use std::env;
use std::hint::black_box;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use mech_core::{GenericError, LegacyValue, MResult, MechError, Ref};
use mech_gpu::{GpuRuntimeBackend, GpuRuntimeHostSettings, GpuRuntimeResourceProvider};
use mech_runtime::{
    PreparedRuntimeEffect, ResourcePathCapability, RuntimeAfterCommitEffect, RuntimeBuilder,
    RuntimeEffectMetadata, RuntimeEffectSource, RuntimeEventKind, RuntimeHostInput,
    RuntimeHostInputDriver, RuntimeHostInputSource, RuntimeHostInputValue, RuntimeIngress,
    RuntimeResourceProvider, RuntimeResourceReadRequest, RuntimeResourceWriteIntent,
    RuntimeResourceWritePreflightRequest, RuntimeResourceWriteRequest,
    provider_defined_effect_contract,
};

const APP_SOURCE: &str = include_str!("../../../examples/mixed-cpu-gpu-particles/app.mec");
const KERNEL_SOURCE: &str = include_str!("../../../examples/mixed-cpu-gpu-particles/kernel.mec");
const PARTICLE_COUNT_DECLARATION: &str = "particle-count := 4096f32";
const CLOCK_BASE: &str = "timer://clock/tick";
const GPU_BASE: &str = "gpu://particles/kernel";
const CONSOLE_BASE: &str = "console://console/output";

#[derive(Clone, Copy, Debug)]
enum RequestedBackend {
    All,
    Wgpu,
    Cpu,
}

impl RequestedBackend {
    fn parse(value: &str) -> Self {
        match value {
            "all" => Self::All,
            "wgpu" => Self::Wgpu,
            "cpu" => Self::Cpu,
            _ => panic!("backend must be `all`, `wgpu`, or `cpu`"),
        }
    }
}

fn main() {
    let backend = RequestedBackend::parse(&env::args().nth(1).unwrap_or_else(|| "all".to_owned()));
    let particles = argument(2, 4_096, "particle count");
    let warmup_turns = argument(3, 5, "warmup turns");
    let measured_turns = argument(4, 50, "measured turns");

    println!("mixed runtime benchmark");
    println!("source: examples/mixed-cpu-gpu-particles/app.mec + kernel.mec");
    println!("particles: {particles}");
    println!("warmup turns: {warmup_turns}");
    println!("measured turns: {measured_turns}");
    println!("GPU measurement synchronizes once after the measured batch; no readback");

    if matches!(backend, RequestedBackend::All | RequestedBackend::Cpu) {
        match run_lane(
            GpuRuntimeBackend::Cpu,
            particles,
            warmup_turns,
            measured_turns,
        ) {
            Ok(profile) => print_profile("cpu", profile),
            Err(error) => println!("cpu error: {error}"),
        }
    }
    if matches!(backend, RequestedBackend::All | RequestedBackend::Wgpu) {
        match run_lane(
            GpuRuntimeBackend::Wgpu,
            particles,
            warmup_turns,
            measured_turns,
        ) {
            Ok(profile) => print_profile("wgpu", profile),
            Err(error) => println!("wgpu unavailable: {error}"),
        }
    }
}

fn argument(index: usize, default: usize, name: &str) -> usize {
    env::args()
        .nth(index)
        .map(|argument| {
            argument
                .parse::<usize>()
                .unwrap_or_else(|_| panic!("{name} must be an integer"))
        })
        .unwrap_or(default)
        .max(1)
}

struct LaneProfile {
    adapter: String,
    dispatch_elements: u64,
    install_and_initial_dispatch: Duration,
    first_pulse: Duration,
    submitted: Duration,
    synchronized: Duration,
    completed: Duration,
    turns: usize,
    particles: usize,
}

fn run_lane(
    backend: GpuRuntimeBackend,
    particles: usize,
    warmup_turns: usize,
    measured_turns: usize,
) -> Result<LaneProfile, String> {
    let kernel = SpecializedKernel::new(particles)?;
    let provider = GpuRuntimeResourceProvider::new(
        "particles",
        GpuRuntimeHostSettings {
            source: kernel.path.clone(),
            backend,
            turns_per_dispatch: 1,
            inputs: [
                ("force-x".to_owned(), 0.0),
                ("force-y".to_owned(), 0.0),
                ("force-enabled".to_owned(), 0.0),
                ("dt".to_owned(), 0.016),
            ]
            .into_iter()
            .collect(),
        },
    );

    let install_started = Instant::now();
    let mut runtime = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .resource_provider(Box::new(BenchmarkClockProvider))
        .resource_provider(Box::new(BenchmarkConsoleProvider))
        .resource_provider(Box::new(provider.clone()))
        .input_driver(BenchmarkClockDriver::default())
        .build()
        .map_err(mech_error)?;
    grant_program_resources(&mut runtime)?;
    let mut context = runtime.runtime_context().map_err(mech_error)?;
    runtime
        .run_string_with_context(&mut context, APP_SOURCE)
        .map_err(mech_error)?;
    let install_and_initial_dispatch = install_started.elapsed();
    let initial_dispatches = provider.dispatched_turns().map_err(mech_error)?;
    if initial_dispatches == 0 {
        return Err(format!(
            "{} did not complete the initial kernel dispatch: {}",
            provider.adapter_name().map_err(mech_error)?,
            latest_delivery_failure(&runtime)
        ));
    }

    let source = RuntimeHostInputSource::new(CLOCK_BASE, "tick").map_err(mech_error)?;
    let mut tick = 1_u64;
    let first_pulse_started = Instant::now();
    apply_tick(&mut runtime, &mut context, &source, tick)?;
    provider.synchronize().map_err(mech_error)?;
    let first_pulse = first_pulse_started.elapsed();

    for _ in 0..warmup_turns {
        tick += 1;
        apply_tick(&mut runtime, &mut context, &source, tick)?;
    }
    provider.synchronize().map_err(mech_error)?;

    let measured_started = Instant::now();
    for _ in 0..measured_turns {
        tick += 1;
        apply_tick(&mut runtime, &mut context, &source, tick)?;
    }
    let submitted = measured_started.elapsed();
    let synchronized = provider.synchronize().map_err(mech_error)?;
    let completed = measured_started.elapsed();
    let adapter = provider.adapter_name().map_err(mech_error)?;
    let dispatch_elements = provider
        .dispatch_elements()
        .map_err(mech_error)?
        .ok_or_else(|| "kernel did not report its dispatch size".to_owned())?;
    if dispatch_elements != particles as u64 {
        return Err(format!(
            "compiled kernel has {dispatch_elements} elements, requested {particles}"
        ));
    }
    let dispatched_turns = provider.dispatched_turns().map_err(mech_error)?;
    let expected_turns = initial_dispatches + 1 + warmup_turns as u64 + measured_turns as u64;
    if dispatched_turns != expected_turns {
        return Err(format!(
            "kernel completed {dispatched_turns} dispatches, expected {expected_turns}: {}",
            latest_delivery_failure(&runtime)
        ));
    }

    Ok(LaneProfile {
        adapter,
        dispatch_elements,
        install_and_initial_dispatch,
        first_pulse,
        submitted,
        synchronized,
        completed,
        turns: measured_turns,
        particles,
    })
}

fn latest_delivery_failure(runtime: &mech_runtime::MechRuntime) -> String {
    runtime
        .list_events(None)
        .ok()
        .and_then(|events| {
            events.into_iter().rev().find_map(|event| match event.kind {
                RuntimeEventKind::EffectDeliveryFailed { message, .. } => Some(message),
                _ => None,
            })
        })
        .unwrap_or_else(|| "no delivery failure was recorded".to_owned())
}

fn apply_tick(
    runtime: &mut mech_runtime::MechRuntime,
    context: &mut mech_runtime::RuntimeContext,
    source: &RuntimeHostInputSource,
    tick: u64,
) -> Result<(), String> {
    let outcome = runtime
        .apply_host_input_with_context(
            context,
            RuntimeHostInput::single(source.clone(), RuntimeHostInputValue::F64(tick as f64)),
        )
        .map_err(mech_error)?;
    if outcome.turn.is_none() {
        return Err("clock update did not advance the reactive graph".to_owned());
    }
    black_box(outcome);
    Ok(())
}

fn print_profile(name: &str, profile: LaneProfile) {
    let per_turn = profile.completed.as_secs_f64() / profile.turns as f64;
    let throughput = profile.particles as f64 / per_turn;
    println!("\nbackend: {name}");
    println!("  executor: {}", profile.adapter);
    println!(
        "  compiled dispatch elements: {}",
        profile.dispatch_elements
    );
    println!(
        "  program install + initial dispatch: {:.3} ms",
        milliseconds(profile.install_and_initial_dispatch)
    );
    println!(
        "  first pulse after install: {:.3} ms",
        milliseconds(profile.first_pulse)
    );
    println!(
        "  measured submission batch: {:.3} ms",
        milliseconds(profile.submitted)
    );
    println!(
        "  final synchronization: {:.3} ms",
        milliseconds(profile.synchronized)
    );
    println!(
        "  completed batch: {:.3} ms",
        milliseconds(profile.completed)
    );
    println!("  completed managed turn: {:.3} ms", per_turn * 1_000.0);
    println!("  completed turns/s: {:.3}", 1.0 / per_turn);
    println!(
        "  completed particle-turns/s: {:.3} M",
        throughput / 1_000_000.0
    );
}

fn milliseconds(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}

fn grant_program_resources(runtime: &mut mech_runtime::MechRuntime) -> Result<(), String> {
    grant(runtime, CLOCK_BASE, "read", "tick")?;
    for path in ["adapter", "turns", "dispatch-ms"] {
        grant(runtime, GPU_BASE, "read", path)?;
    }
    for path in [
        "input/force-x",
        "input/force-y",
        "input/force-enabled",
        "input/dt",
        "turn",
    ] {
        grant(runtime, GPU_BASE, "write", path)?;
    }
    grant(runtime, CONSOLE_BASE, "write", "line")
}

fn grant(
    runtime: &mut mech_runtime::MechRuntime,
    base_uri: &str,
    operation: &str,
    path: &str,
) -> Result<(), String> {
    let subject = runtime
        .runtime_context()
        .map_err(mech_error)?
        .subject()
        .to_owned();
    let capability = ResourcePathCapability::exact(
        runtime.next_capability_id(),
        subject,
        base_uri,
        [operation],
        path,
    )
    .map_err(mech_error)?;
    runtime
        .grant_capability(Arc::new(capability))
        .map(|_| ())
        .map_err(mech_error)
}

struct SpecializedKernel {
    path: PathBuf,
}

impl SpecializedKernel {
    fn new(particles: usize) -> Result<Self, String> {
        if !KERNEL_SOURCE.contains(PARTICLE_COUNT_DECLARATION) {
            return Err("kernel particle count declaration changed".to_owned());
        }
        let replacement = format!("particle-count := {particles}f32");
        let source = KERNEL_SOURCE.replacen(PARTICLE_COUNT_DECLARATION, &replacement, 1);
        let path = env::temp_dir().join(format!(
            "mech-mixed-particles-{}-{particles}.mec",
            std::process::id()
        ));
        std::fs::write(&path, source).map_err(|error| error.to_string())?;
        Ok(Self { path })
    }
}

impl Drop for SpecializedKernel {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

#[derive(Debug)]
struct BenchmarkClockProvider;

#[derive(Debug, Default)]
struct BenchmarkClockDriver {
    live: bool,
}

impl RuntimeHostInputDriver for BenchmarkClockDriver {
    fn drives(&self, source: &RuntimeHostInputSource) -> bool {
        source.base_uri() == CLOCK_BASE && source.path() == "tick"
    }

    fn attach(&mut self, _ingress: RuntimeIngress) -> MResult<()> {
        Ok(())
    }

    fn start(&mut self) -> MResult<()> {
        self.live = true;
        Ok(())
    }

    fn stop(&mut self) -> MResult<()> {
        self.live = false;
        Ok(())
    }

    fn is_live(&self) -> bool {
        self.live
    }
}

impl RuntimeResourceProvider for BenchmarkClockProvider {
    fn scheme(&self) -> &str {
        "timer"
    }

    fn base_uris(&self) -> Vec<String> {
        vec![CLOCK_BASE.to_owned()]
    }

    fn plan_read(&self, request: RuntimeResourceReadRequest) -> MResult<LegacyValue> {
        clock_value(request)
    }

    fn read(&self, request: RuntimeResourceReadRequest) -> MResult<LegacyValue> {
        clock_value(request)
    }
}

fn clock_value(request: RuntimeResourceReadRequest) -> MResult<LegacyValue> {
    if request.base_uri == CLOCK_BASE && request.path == "tick" {
        Ok(LegacyValue::F64(Ref::new(0.0)))
    } else {
        Err(benchmark_error("unknown benchmark clock resource"))
    }
}

#[derive(Debug)]
struct BenchmarkConsoleProvider;

impl RuntimeResourceProvider for BenchmarkConsoleProvider {
    fn scheme(&self) -> &str {
        "console"
    }

    fn base_uris(&self) -> Vec<String> {
        vec![CONSOLE_BASE.to_owned()]
    }

    fn read(&self, _request: RuntimeResourceReadRequest) -> MResult<LegacyValue> {
        Err(benchmark_error("benchmark console is write-only"))
    }

    fn semantic_write_contract(
        &self,
        intent: RuntimeResourceWriteIntent,
    ) -> Option<&'static mech_core::OperationContractDeclaration> {
        (intent == RuntimeResourceWriteIntent::Send).then(provider_defined_effect_contract)
    }

    fn preflight_write(&self, request: RuntimeResourceWritePreflightRequest) -> MResult<()> {
        if request.base_uri == CONSOLE_BASE
            && request.path == "line"
            && request.intent == RuntimeResourceWriteIntent::Send
        {
            Ok(())
        } else {
            Err(benchmark_error("unknown benchmark console resource"))
        }
    }

    fn prepare_write(
        &self,
        request: RuntimeResourceWriteRequest,
    ) -> MResult<PreparedRuntimeEffect> {
        self.preflight_write(RuntimeResourceWritePreflightRequest {
            base_uri: request.base_uri.clone(),
            path: request.path,
            context_name: request.context_name,
            operation: request.operation,
            intent: request.intent,
        })?;
        Ok(PreparedRuntimeEffect::AfterCommit(Box::new(
            BenchmarkConsoleEffect,
        )))
    }
}

#[derive(Debug)]
struct BenchmarkConsoleEffect;

impl RuntimeAfterCommitEffect for BenchmarkConsoleEffect {
    fn metadata(&self) -> RuntimeEffectMetadata {
        RuntimeEffectMetadata::new(
            RuntimeEffectSource::ResourceProvider {
                scheme: "console".to_owned(),
            },
            "write",
        )
        .with_resource(CONSOLE_BASE)
    }

    fn deliver(&mut self) -> MResult<()> {
        black_box(());
        Ok(())
    }
}

fn benchmark_error(message: impl Into<String>) -> MechError {
    MechError::new(
        GenericError {
            msg: message.into(),
        },
        None,
    )
}

fn mech_error(error: MechError) -> String {
    format!("{error:?}")
}
