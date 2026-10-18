//! Secured resident reference for canonical hosted native qualification.
//! Uses the existing runtime, real CLI provider and ordinary capability policy.
//! This is not native AOT or an old-I/O-versus-secured benchmark result.
use std::{ffi::OsString, path::Path};

use mech_runtime::{
    ConfigValue, HostInstanceConfig, ResidentDurabilityPolicy, RunResourceGrantConfig,
    RuntimeBuilder, RuntimeProgramRoute,
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
                let bytecode = std::fs::read(bytecode).map_err(|error| error.to_string())?;
                let artifact = mech_engine::decode_program_artifact_bytecode_v1(&bytecode)
                    .map_err(|error| format!("{error:?}"))?;
                let revision = artifact.revision();
                let loaded = runtime
                    .load_compiled_program(artifact, ResidentDurabilityPolicy::Volatile)
                    .map_err(diagnostic)?;
                let info = runtime.program_execution_info();
                if loaded.route != RuntimeProgramRoute::ResidentExternal
                    || info.program_revision != Some(revision)
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
