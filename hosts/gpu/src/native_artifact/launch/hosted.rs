//! Shared same-Object launcher; production is capability-secured.
//! No source parsing, JIT, resident graph execution or source-native fallback.
use super::program::BuildOwnedHostedBinding;
use crate::native_artifact::{NativeExecutionLimits, NativeProgramExecutionLimits};
use mech_core::{ManagedMemoryBudget, ManagedMemoryReservation, Value};
use mech_runtime::{
    CanonicalNativeResourceCompletion, CanonicalNativeResourceLimits,
    CanonicalNativeResourceMetadata, CanonicalNativeResourceRunError, ConfigValue,
    HostInstanceConfig, NativeResourceBindingError, RunResourceGrantConfig, RuntimeBuilder,
    RuntimeProgramRoute,
};
use mech_terminal::{ArgumentSnapshotCliBackend, CliBackend, CliHostFactory, StdCliBackend};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    ffi::{OsStr, OsString},
    io::{Read, Write},
    path::Path,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Grant {
    target: String,
    operations: Vec<String>,
    paths: Vec<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LibraryLimits {
    output_elements: u64,
    output_bytes: u64,
    temporary_bytes: u64,
    cloned_bytes: u64,
    retained_nodes: u64,
    compute_work: u64,
    comparison_work: u64,
}
// Separate flat schemas retain strict secured/benchmark field acceptance even
// when Cargo unifies both optional feature sets in an embedding application.
macro_rules! launch_schema {
    ($name:ident, $($extra:tt)*) => {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields, rename = "Launch")]
        struct $name {
            $($extra)*
            grants: Vec<Grant>,
            max_ports: usize,
            input_text_bytes: u64,
            output_text_bytes: u64,
            memory_bytes: u64,
            work: u64,
            transport_bytes: u64,
            native_stack_bytes: u64,
            library: LibraryLimits,
        }
    }
}
launch_schema!(Launch,);
#[cfg(feature = "canonical-native-benchmark-policy-runtime")]
launch_schema!(BenchmarkLaunch, benchmark_policy: String,);
trait LaunchPolicy: Sized {
    fn parse(bytes: &[u8]) -> Result<(Launch, Self), String>;
    fn builder(
        &self,
        grants: usize,
        memory: &ManagedMemoryBudget,
    ) -> Result<RuntimeBuilder, String>;
    fn name(&self) -> &'static str;
}
struct Secured;
impl LaunchPolicy for Secured {
    fn parse(bytes: &[u8]) -> Result<(Launch, Self), String> {
        serde_json::from_slice(bytes)
            .map(|launch| (launch, Self))
            .map_err(|error| error.to_string())
    }
    fn builder(&self, _: usize, _: &ManagedMemoryBudget) -> Result<RuntimeBuilder, String> {
        Ok(RuntimeBuilder::new())
    }
    fn name(&self) -> &'static str {
        "secured"
    }
}
#[cfg(feature = "canonical-native-benchmark-policy-runtime")]
struct Benchmark(crate::native_artifact::NativeBenchmarkPolicy);
#[cfg(feature = "canonical-native-benchmark-policy-runtime")]
impl LaunchPolicy for Benchmark {
    fn parse(bytes: &[u8]) -> Result<(Launch, Self), String> {
        let parsed: BenchmarkLaunch =
            serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
        let policy = crate::native_artifact::NativeBenchmarkPolicy::parse(&parsed.benchmark_policy)
            .map_err(diagnostic)?;
        Ok((
            Launch {
                grants: parsed.grants,
                max_ports: parsed.max_ports,
                input_text_bytes: parsed.input_text_bytes,
                output_text_bytes: parsed.output_text_bytes,
                memory_bytes: parsed.memory_bytes,
                work: parsed.work,
                transport_bytes: parsed.transport_bytes,
                native_stack_bytes: parsed.native_stack_bytes,
                library: parsed.library,
            },
            Self(policy),
        ))
    }
    fn builder(
        &self,
        grants: usize,
        memory: &ManagedMemoryBudget,
    ) -> Result<RuntimeBuilder, String> {
        Ok(RuntimeBuilder::new().capability_kernel(
            crate::native_artifact::NativeBenchmarkCapabilityKernel::new(self.0, grants, memory)
                .map_err(diagnostic)?,
        ))
    }
    fn name(&self) -> &'static str {
        self.0.name()
    }
}
/// Run a build-owned program with ordinary capability-secured CLI services.
pub fn run_hosted_program_application(
    binding: BuildOwnedHostedBinding,
    arguments: impl ExactSizeIterator<Item = OsString>,
) -> Result<(), String> {
    run::<Secured>(binding, arguments)
}
#[cfg(feature = "canonical-native-benchmark-policy-runtime")]
/// Run the separately feature-gated, explicitly trusted benchmark policy shell.
pub fn run_benchmark_program_application(
    binding: BuildOwnedHostedBinding,
    arguments: impl ExactSizeIterator<Item = OsString>,
) -> Result<(), String> {
    run::<Benchmark>(binding, arguments)
}

fn diagnostic(error: impl std::fmt::Debug) -> String {
    format!("{error:?}")
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
fn bounded(mut reader: impl Read, limit: u64) -> Result<Vec<u8>, String> {
    let acquisition = limit
        .checked_add(1)
        .ok_or("input acquisition cap overflows")?;
    let mut bytes = Vec::new();
    reader
        .by_ref()
        .take(acquisition)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() as u64 > limit {
        return Err("input exceeds its finite acquisition cap".into());
    }
    Ok(bytes)
}
fn io_error(reason: impl Into<String>) -> mech_core::MechError {
    mech_core::MechError::new(
        NativeResourceBindingError {
            reason: reason.into(),
        },
        None,
    )
}

// Keep the charge alive until the runtime (and therefore its backend) drops.
// Fields drop in order: a not-yet-transferred Vec must retire before its charge.
struct PayloadArguments {
    arguments: Vec<OsString>,
    _reservation: ManagedMemoryReservation,
}
fn argument_carrier_bytes(count: usize) -> Result<u64, String> {
    // Vec -> Arc<[OsString]> moves payload owners but allocates a second header
    // array while the Vec allocation is still live. Include Arc's strong/weak
    // counters and its padding to the element alignment on this pinned target.
    let align = std::mem::align_of::<OsString>();
    let arc_header = std::mem::size_of::<[usize; 2]>()
        .checked_add(align - 1)
        .map(|bytes| bytes & !(align - 1))
        .ok_or("payload argument header overflows")?;
    count
        .checked_mul(std::mem::size_of::<OsString>())
        .and_then(|bytes| bytes.checked_mul(2))
        .and_then(|bytes| bytes.checked_add(arc_header))
        .and_then(|bytes| u64::try_from(bytes).ok())
        .ok_or_else(|| "payload argument carriers overflow".into())
}
fn reserve_argument_headers(snapshot: &mut Vec<OsString>, count: usize) -> Result<(), String> {
    snapshot.try_reserve_exact(count).map_err(diagnostic)?;
    // Exact reservation is a minimum, not a promise about allocator rounding.
    // Never retain header capacity beyond the prepaid Vec/Arc peak.
    if snapshot.capacity() != count {
        return Err("payload argument header allocation exceeds admitted capacity".into());
    }
    Ok(())
}
fn exact_capacity<T>(values: Vec<T>, count: usize, reason: &'static str) -> Result<Vec<T>, String> {
    // The finite host-carrier bound funds count headers, not a minimum-sized
    // allocation. Drop excess backing here before it can reach the runtime.
    if values.capacity() != count {
        return Err(reason.into());
    }
    Ok(values)
}
fn bounded_arguments(
    arguments: impl ExactSizeIterator<Item = OsString>,
    limit: u64,
    memory: &ManagedMemoryBudget,
) -> Result<PayloadArguments, String> {
    // Rust/platform argv acquisition is outside this application's account.
    // Bound and fund our retained snapshot before allocation/ownership transfer;
    // do not claim that the preexisting OS/Rust argv source is RSS-budgeted.
    let count = arguments.len();
    let carrier_bytes = argument_carrier_bytes(count)?;
    let mut reservation = memory
        .reserve_capacity(carrier_bytes)
        .map_err(|error| format!("payload argument snapshot memory: {error:?}"))?;
    let mut snapshot = Vec::new();
    reserve_argument_headers(&mut snapshot, count)?;
    let mut bytes = 0u64;
    let mut capacity_bytes = 0u64;
    for argument in arguments {
        if snapshot.len() >= count {
            return Err("payload argument count changed during acquisition".into());
        }
        bytes = bytes
            .checked_add(
                u64::try_from(argument.as_encoded_bytes().len())
                    .map_err(|_| "payload argument length overflows")?,
            )
            .ok_or("payload argument aggregate length overflows")?;
        if bytes > limit {
            return Err("payload arguments exceed their finite aggregate acquisition cap".into());
        }
        capacity_bytes = capacity_bytes
            .checked_add(
                u64::try_from(argument.capacity())
                    .map_err(|_| "payload argument capacity overflows")?,
            )
            .ok_or("payload argument aggregate capacity overflows")?;
        let retained = carrier_bytes
            .checked_add(capacity_bytes)
            .ok_or("payload argument snapshot size overflows")?;
        reservation
            .resize_capacity(retained)
            .map_err(|error| format!("payload argument snapshot memory: {error:?}"))?;
        snapshot.push(argument);
    }
    if snapshot.len() != count {
        return Err("payload argument count changed during acquisition".into());
    }
    Ok(PayloadArguments {
        arguments: snapshot,
        _reservation: reservation,
    })
}

/// Only acquisition is specialized. Authorization, resource identity, Unicode
/// validation, cached snapshots, output preparation, write/flush and delivery
/// all remain the existing CLI provider/runtime implementation.
#[derive(Clone, Debug)]
struct BoundedStdin {
    limit: u64,
}
impl CliBackend for BoundedStdin {
    fn env_var(&self, name: &str) -> mech_core::MResult<Option<String>> {
        StdCliBackend.env_var(name)
    }
    fn stdin_bytes(&self) -> Option<mech_core::MResult<Vec<u8>>> {
        Some(bounded(std::io::stdin().lock(), self.limit).map_err(io_error))
    }
    fn stdin_text(&self) -> mech_core::MResult<String> {
        Err(io_error(
            "bounded backend requires the provider's raw snapshot path",
        ))
    }
    fn supports_single_use_stdin_text_capture(&self) -> bool {
        true
    }
    fn with_single_use_stdin_text_reader(
        &self,
        capture: &mut dyn FnMut(&mut dyn Read) -> mech_core::MResult<()>,
    ) -> mech_core::MResult<()> {
        capture(&mut std::io::stdin().lock())
    }
    fn write_stdout(&mut self, text: &str) -> mech_core::MResult<()> {
        StdCliBackend.write_stdout(text)
    }
    fn write_stderr(&mut self, text: &str) -> mech_core::MResult<()> {
        StdCliBackend.write_stderr(text)
    }
    fn write_stdout_bytes(&mut self, bytes: &[u8]) -> mech_core::MResult<()> {
        StdCliBackend.write_stdout_bytes(bytes)
    }
    fn write_stderr_bytes(&mut self, bytes: &[u8]) -> mech_core::MResult<()> {
        StdCliBackend.write_stderr_bytes(bytes)
    }
}
fn write_new(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| error.to_string())?;
    file.write_all(bytes).map_err(|error| error.to_string())
}
fn run<P: LaunchPolicy>(
    binding: BuildOwnedHostedBinding,
    arguments: impl ExactSizeIterator<Item = OsString>,
) -> Result<(), String> {
    let mut arguments = arguments;
    let launch_path = arguments.next().ok_or("missing launch JSON path")?;
    let report = arguments.next().ok_or("missing report path or '-'")?;
    if arguments.next().as_deref() != Some(OsStr::new("--")) {
        return Err("expected LAUNCH_JSON REPORT_OR_DASH -- [PAYLOAD_ARGUMENTS...]".into());
    }
    let launch_bytes = bounded(
        std::fs::File::open(launch_path).map_err(|error| error.to_string())?,
        64 * 1024,
    )?;
    let (launch, policy) = P::parse(&launch_bytes)?;
    launch
        .input_text_bytes
        .checked_add(1)
        .ok_or("input acquisition cap overflows")?;
    let memory = ManagedMemoryBudget::new(launch.memory_bytes);
    let mut payload_arguments = bounded_arguments(arguments, launch.input_text_bytes, &memory)?;
    let catalog = super::scalar::metadata_catalog(binding.code.catalog)?;
    let linked = binding.code.bind(&catalog).map_err(diagnostic)?;
    if !linked.descriptor().plan().inputs().is_empty() {
        return Err("hosted application has unbound ordinary turn inputs".into());
    }
    let mut library = mech_core::TargetMemoryProfile::current_resident_cpu()
        .map_err(diagnostic)?
        .limits;
    library.max_output_elements = Some(launch.library.output_elements);
    library.max_output_bytes = Some(launch.library.output_bytes);
    library.max_temporary_bytes = Some(launch.library.temporary_bytes);
    library.max_cloned_bytes = Some(launch.library.cloned_bytes);
    library.max_retained_nodes = Some(launch.library.retained_nodes);
    library.max_compute_work = Some(launch.library.compute_work);
    library.max_comparison_work = Some(launch.library.comparison_work);
    let limits = NativeProgramExecutionLimits {
        execution: NativeExecutionLimits {
            max_work: launch.work,
            max_transport_bytes: launch.transport_bytes,
            max_native_stack_bytes: launch.native_stack_bytes,
        },
        library,
    };
    let count = linked.descriptor().plan().transport_output_count();
    let effect_count = linked.descriptor().plan().effects().len();
    // Host-local result carrier counts are bounded before allocation. Native
    // Value payloads/imports remain admitted by the native memory/work account.
    // Provider raw snapshots/cache have their separate acquisition byte cap.
    let carrier_bytes = count
        .checked_mul(std::mem::size_of::<Option<Value>>())
        .and_then(|n| n.checked_add(effect_count.checked_mul(std::mem::size_of::<Value>())?))
        .and_then(|n| u64::try_from(n).ok())
        .ok_or("host result carrier size overflows")?;
    if carrier_bytes > launch.transport_bytes {
        return Err("host result carriers exceed finite transport cap".into());
    }
    let mut candidates = Vec::new();
    candidates.try_reserve_exact(count).map_err(diagnostic)?;
    let mut candidates = exact_capacity(
        candidates,
        count,
        "host output carrier allocation exceeds admitted capacity",
    )?;
    candidates.resize_with(count, || None);
    let mut effect_values = Vec::new();
    effect_values
        .try_reserve_exact(effect_count)
        .map_err(diagnostic)?;
    let mut effect_values = exact_capacity(
        effect_values,
        effect_count,
        "host effect carrier allocation exceeds admitted capacity",
    )?;

    let backend = ArgumentSnapshotCliBackend::new(
        BoundedStdin {
            limit: launch.input_text_bytes,
        },
        std::mem::take(&mut payload_arguments.arguments),
    );
    let mut builder = policy
        .builder(launch.grants.len(), &memory)?
        .host_factory(Box::new(
            CliHostFactory::with_backend(backend).map_err(diagnostic)?,
        ))
        .map_err(diagnostic)?
        .host_instance(HostInstanceConfig {
            name: "cli".into(),
            provider: "cli".into(),
            settings: ConfigValue::Map(Default::default()),
        });
    for grant in launch.grants {
        builder = builder.run_resource_grant(RunResourceGrantConfig {
            target: grant.target,
            operations: grant.operations,
            paths: grant.paths,
        });
    }
    let mut runtime = builder.build().map_err(diagnostic)?;
    // SAFETY: build.rs pins this complete resource image alongside the SAME
    // source/bytecode/Object and native descriptor. Its canonical revision and
    // schemas are checked below. No program graph is decoded at startup; fresh
    // runtime authority and port caps remain mandatory on every invocation.
    let program_revision = linked.descriptor().plan().program_revision();
    let resource_metadata = unsafe {
        CanonicalNativeResourceMetadata::decode_build_owned_image(
            binding.resource_image,
            program_revision,
            linked.descriptor().schemas().clone(),
        )
    };
    drop(catalog);
    let execution = resource_metadata
        .map_err(|error| CanonicalNativeResourceRunError::BeforeCommit {
            phase: "admission",
            error,
            cleanup: None,
        })
        .and_then(|resource_metadata| {
            runtime.with_prepared_canonical_native_resources_phased_text(
                &resource_metadata,
                CanonicalNativeResourceLimits {
                    max_ports: launch.max_ports,
                    max_input_text_bytes: launch.input_text_bytes,
                    max_output_text_bytes: launch.output_text_bytes,
                },
                // Same checked Object entry/state/work/memory survives the boundary:
                // canonical admission -> generated closed activation -> provider
                // capture -> native resume. No parser, evaluator or callee graph.
                |admission| {
                    linked
                        .prepare_hosted(admission, limits, &memory)
                        .and_then(|session| {
                            session.with_text_capture(launch.input_text_bytes, 64 * 1024)
                        })
                        .map_err(|error| mech_core::MechError::new(error, None))
                },
                |session, chunk| {
                    session
                        .capture_chunk(chunk)
                        .map_err(|error| mech_core::MechError::new(error, None))
                },
                |session, turn| {
                    let captured_lines = turn.text_capture().is_some();
                    let report = session
                        .resume_hosted_into(turn, &[], &mut candidates)
                        .map_err(|error| mech_core::MechError::new(error, None))?;
                    let first_effect = candidates
                        .len()
                        .checked_sub(effect_count)
                        .ok_or_else(|| io_error("native effect transport underflows"))?;
                    for candidate in &mut candidates[first_effect..] {
                        effect_values.push(candidate.take().ok_or_else(|| {
                            io_error("native completion omitted an effect payload")
                        })?);
                    }
                    Ok(CanonicalNativeResourceCompletion {
                        result: (report, captured_lines),
                        effects: effect_values.into_boxed_slice(),
                    })
                },
            )
        });
    // This CLI runtime supplies authority/provider/transaction services only.
    // No resident artifact is activated or evaluated by a native application.
    let route = runtime.program_execution_info();
    let no_resident_body = route.route == RuntimeProgramRoute::None
        && route.program_revision.is_none()
        && route.resident_accepted_turns == 0
        && route.resident_rejected_turns == 0;
    let shutdown = runtime.shutdown().map_err(diagnostic);
    let execution = execution.map_err(diagnostic);
    let (native, captured_lines) = match (execution, shutdown) {
        (Ok(native), Ok(())) => native,
        (Err(error), Ok(())) | (Ok(_), Err(error)) => return Err(error),
        (Err(error), Err(cleanup)) => return Err(format!("{error}; shutdown: {cleanup}")),
    };
    if !no_resident_body {
        return Err("native shell unexpectedly activated a resident body".into());
    }
    if report != "-" {
        let document = serde_json::json!({
            "schema": "mech.canonical-hosted-native-execution.v1",
            "route": binding.route,
            "policy": policy.name(),
            "program_revision": hex(program_revision.as_bytes()),
            "launch_sha256": hex(&Sha256::digest(&launch_bytes)),
            "native_aot": true,
            "hosted_phases": "admit-activate-capture-resume",
            "input_storage": if captured_lines { "bounded-lines" } else { "snapshot" },
            "source_parser": false,
            "jit": false,
            "resident_body_evaluation": false,
            "requirements": linked.descriptor().plan().resource_requirements().len(),
            "observations": linked.descriptor().plan().observations().len(),
            "effects": effect_count,
            "native_work": native.work,
            "native_comparison_work": native.comparison_work,
            "native_visits": native.visits,
            "native_transport_bytes": native.transport_bytes,
            "native_hash_table_regions": native.hash_table_regions,
            "native_hash_table_updates": native.hash_table_updates,
            "native_packed_map_regions": native.packed_map_regions,
            "native_direct_lookup_regions": native.direct_lookup_regions,
            "native_packed_map_updates": native.packed_map_updates,
            "native_packed_ssa_regions": native.packed_ssa_regions,
            "native_guarded_loop_regions": native.guarded_loop_regions,
            "native_guarded_call_regions": native.guarded_call_regions,
            "native_frozen_map_regions": native.frozen_map_regions,
            "native_frozen_map_reads": native.frozen_map_reads,
            "native_worker_credit_claims": native.worker_credit_claims,
            "native_worker_credit_batches": native.worker_credit_batches,
            "native_worker_credit_reclaims": native.worker_credit_reclaims,
            "native_fused_string_map_bytes": native.fused_string_map_bytes,
            "native_parallel_regions": native.parallel_regions,
            "native_parallel_batches": native.parallel_batches,
            "native_parallel_tasks": native.parallel_tasks,
            "native_parallel_peak_workers": native.parallel_peak_workers,
            "host_result_carrier_bytes": carrier_bytes,
            "input_acquisition_byte_cap": launch.input_text_bytes,
            "payload_argument_snapshot_reserved_bytes": payload_arguments._reservation.capacity_bytes(),
            "shutdown": "completed",
            "scope": "complete original canonical artifact; shared CLI capability/provider/transaction services; argv snapshot has an aggregate acquisition cap and same-account reservation; OS/Rust argv acquisition and stdin provider cache bytes are outside the managed account"
        });
        write_new(
            Path::new(&report),
            &serde_json::to_vec_pretty(&document).unwrap(),
        )?;
    }
    Ok(())
}
#[cfg(test)]
#[path = "hosted_tests.rs"]
mod tests;
