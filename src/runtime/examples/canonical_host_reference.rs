//! Secured resident reference for canonical hosted native qualification.
//! Uses the existing runtime, real CLI provider and ordinary capability policy.
//! This is not native AOT or an old-I/O-versus-secured benchmark result.
use std::{ffi::OsString, io::Read, path::Path};

use mech_runtime::{
    ConfigValue, HostInstanceConfig, MechRuntime, ResidentDurabilityPolicy, RunResourceGrantConfig,
    RuntimeBuilder, RuntimeProgramLoadOutcome, RuntimeProgramRoute,
};
use mech_terminal::{ArgumentSnapshotCliBackend, CliHostFactory, StdCliBackend};

#[derive(serde_derive::Deserialize)]
#[serde(deny_unknown_fields)]
struct LaunchGrant {
    target: String,
    operations: Vec<String>,
    paths: Vec<String>,
}

fn diagnostic(error: impl std::fmt::Debug) -> String {
    format!("{error:?}")
}

fn builder(arguments: Vec<OsString>) -> Result<RuntimeBuilder, String> {
    let backend = ArgumentSnapshotCliBackend::new(StdCliBackend, arguments);
    Ok(RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .host_factory(Box::new(
            CliHostFactory::with_backend(backend).map_err(diagnostic)?,
        ))
        .map_err(diagnostic)?
        .host_instance(HostInstanceConfig {
            name: "cli".into(),
            provider: "cli".into(),
            settings: ConfigValue::Map(Default::default()),
        }))
}

fn load_bytecode(
    runtime: &mut MechRuntime,
    mut reader: impl Read,
) -> Result<RuntimeProgramLoadOutcome, String> {
    let mut bytecode = Vec::new();
    match runtime.config().limits.max_source_bytes {
        Some(limit) => reader
            .take(limit.saturating_add(1))
            .read_to_end(&mut bytecode),
        None => reader.read_to_end(&mut bytecode),
    }
    .map_err(|error| error.to_string())?;
    // The ordinary loader owns both size admission and decoding. Reading at
    // most limit+1 also bounds acquisition before it reports an oversized
    // input; an explicit unbounded runtime configuration remains unbounded.
    runtime
        .load_bytecode_program(&bytecode, ResidentDurabilityPolicy::Volatile)
        .map_err(diagnostic)
}

fn run() -> Result<(), String> {
    let mut arguments = std::env::args_os().skip(1);
    match arguments.next().and_then(|value| value.into_string().ok()).as_deref() {
        Some("compile") => {
            let source = arguments.next().ok_or("missing source path")?;
            let destination = arguments.next().ok_or("missing bytecode path")?;
            if arguments.next().is_some() {
                return Err("compile accepts SOURCE BYTECODE only".into());
            }
            if Path::new(&destination).exists() {
                return Err("refusing to overwrite bytecode".into());
            }
            let source = std::fs::read_to_string(source).map_err(|error| error.to_string())?;
            let mut compiler = builder(Vec::new())?
                .build_compiler()
                .map_err(diagnostic)?;
            let artifact = compiler
                .compile_canonical_source(&source)
                .map_err(diagnostic)?
                .into_parts()
                .0;
            let bytecode = mech_engine::encode_program_artifact_bytecode_v1(&artifact)
                .map_err(|error| format!("{error:?}"))?;
            std::fs::write(destination, bytecode).map_err(|error| error.to_string())?;
            Ok(())
        }
        Some("run") => {
            let bytecode = arguments.next().ok_or("missing bytecode path")?;
            let grants = arguments.next().ok_or("missing grants JSON path")?;
            // A report is qualification evidence, not application output. A
            // literal '-' disables it. Payload argv has one explicit boundary.
            let report = arguments.next().ok_or("missing report path or '-' ")?;
            if arguments.next().as_deref() != Some(std::ffi::OsStr::new("--")) {
                return Err("run requires '--' before immutable payload arguments".into());
            }
            let grants: Vec<LaunchGrant> = serde_json::from_slice(
                &std::fs::read(grants).map_err(|error| error.to_string())?,
            )
            .map_err(|error| error.to_string())?;
            let mut builder = builder(arguments.collect())?;
            // Authority is always enforced by the existing runtime; there is
            // no benchmark-specific trusted bypass or alternate evaluator.
            for grant in grants {
                builder = builder.run_resource_grant(RunResourceGrantConfig {
                    target: grant.target,
                    operations: grant.operations,
                    paths: grant.paths,
                });
            }
            let mut runtime = builder.build().map_err(diagnostic)?;
            let execution = (|| {
                let input = std::fs::File::open(bytecode).map_err(|error| error.to_string())?;
                let loaded = load_bytecode(&mut runtime, input)?;
                let info = runtime.program_execution_info();
                if loaded.route != RuntimeProgramRoute::ResidentExternal
                    || info.program_revision.is_none()
                    || info.program_revision != loaded.info.program_revision
                    || info.resident_rejected_turns != 0
                    || info.resident_accepted_turns == 0
                {
                    return Err("canonical host execution did not complete an accepted turn".into());
                }
                Ok(info)
            })();
            let shutdown = runtime.shutdown().map_err(diagnostic);
            let info = match (execution, shutdown) {
                (Ok(info), Ok(())) => info,
                (Err(error), Ok(())) | (Ok(_), Err(error)) => return Err(error),
                (Err(error), Err(cleanup)) => {
                    return Err(format!("{error}; shutdown: {cleanup}"));
                }
            };
            if report != "-" {
                let report = Path::new(&report);
                if report.exists() {
                    return Err("refusing to overwrite qualification report".into());
                }
                let document = serde_json::json!({
                    "schema": "mech.canonical-host-reference.v1",
                    "policy": "secured",
                    "route": "canonical-resident-external-precompiled-bytecode",
                    "program_revision": format!("{:?}", info.program_revision),
                    "requirements": info.requirement_count,
                    "observations": info.observation_count,
                    "effects": info.effect_count,
                    "accepted": info.resident_accepted_turns,
                    "rejected": info.resident_rejected_turns,
                    "shutdown": "completed",
                    "scope": "resident reference; ordinary runtime authority, provider, evaluator and publication policy",
                    "native_aot": false
                });
                std::fs::write(report, serde_json::to_vec_pretty(&document).unwrap())
                    .map_err(|error| error.to_string())?;
            }
            Ok(())
        }
        _ => Err("usage: canonical_host_reference compile SOURCE BYTECODE | run BYTECODE GRANTS REPORT_OR_DASH -- [PAYLOAD...]".into()),
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn bytecode(source: &str) -> Vec<u8> {
        builder(Vec::new())
            .unwrap()
            .build_compiler()
            .unwrap()
            .compile_canonical_source(source)
            .unwrap()
            .bytecode()
            .to_vec()
    }

    fn runtime(limit: Option<u64>) -> MechRuntime {
        let mut config = mech_runtime::RuntimeConfig::default();
        config.limits.max_source_bytes = limit;
        builder(Vec::new()).unwrap().config(config).build().unwrap()
    }

    #[test]
    fn exact_limit_reference_matches_ordinary_bytecode_loading() {
        let bytes = bytecode("answer := 7\nanswer\n");
        let mut reference = runtime(Some(bytes.len() as u64));
        let mut ordinary = runtime(Some(bytes.len() as u64));
        let actual = load_bytecode(&mut reference, Cursor::new(&bytes)).unwrap();
        let expected = ordinary
            .load_bytecode_program(&bytes, ResidentDurabilityPolicy::Volatile)
            .unwrap();
        assert_eq!(actual.info.program_revision, expected.info.program_revision);
        assert_eq!(actual.route, expected.route);
        assert_eq!(
            actual.initial_value.format_canonical_inline(),
            expected.initial_value.format_canonical_inline()
        );
        reference.shutdown().unwrap();
        ordinary.shutdown().unwrap();
    }

    #[test]
    fn oversized_valid_bytecode_matches_denial_and_preserves_publication() {
        let small = bytecode("answer := 7\nanswer\n");
        let large = bytecode(&format!("answer := \"{}\"\nanswer\n", "x".repeat(256)));
        mech_engine::decode_program_artifact_bytecode_v1(&large).unwrap();
        let limit = large.len() as u64 - 1;
        assert!(small.len() as u64 <= limit);
        let mut reference = runtime(Some(limit));
        let mut ordinary = runtime(Some(limit));
        load_bytecode(&mut reference, Cursor::new(&small)).unwrap();
        ordinary
            .load_bytecode_program(&small, ResidentDurabilityPolicy::Volatile)
            .unwrap();
        let prior = reference.program_execution_info();
        let prior_output = reference
            .program_output_value()
            .unwrap()
            .unwrap()
            .format_canonical_inline();
        let mut input = Cursor::new(&large);
        let actual = load_bytecode(&mut reference, &mut input).unwrap_err();
        let expected = ordinary
            .load_bytecode_program(&large, ResidentDurabilityPolicy::Volatile)
            .unwrap_err();
        assert!(
            expected
                .kind_as::<mech_runtime::ResourceBudgetExceededError>()
                .is_some()
        );
        assert_eq!(actual, diagnostic(expected));
        assert_eq!(input.position(), limit + 1);
        assert_eq!(reference.program_execution_info(), prior);
        assert_eq!(ordinary.program_execution_info(), prior);
        assert_eq!(
            reference
                .program_output_value()
                .unwrap()
                .unwrap()
                .format_canonical_inline(),
            prior_output
        );
        reference.unload_active_program().unwrap();
        let recovered = load_bytecode(&mut reference, Cursor::new(&small)).unwrap();
        assert_eq!(recovered.info.program_revision, prior.program_revision);
        reference.shutdown().unwrap();
        ordinary.shutdown().unwrap();
    }

    #[test]
    fn acquisition_stops_at_limit_plus_one_and_respects_explicit_none() {
        let mut bounded = runtime(Some(64));
        let mut input = Cursor::new(vec![b'x'; 1024 * 1024]);
        let error = load_bytecode(&mut bounded, &mut input).unwrap_err();
        assert!(error.contains("source_bytes"), "{error}");
        assert_eq!(input.position(), 65);
        assert_eq!(
            bounded.program_execution_info().route,
            RuntimeProgramRoute::None
        );
        bounded.shutdown().unwrap();
        let bytes = bytecode("answer := 7\nanswer\n");
        let mut unbounded = runtime(None);
        let mut input = Cursor::new(&bytes);
        load_bytecode(&mut unbounded, &mut input).unwrap();
        assert_eq!(input.position(), bytes.len() as u64);
        unbounded.shutdown().unwrap();
    }
}
