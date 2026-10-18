//! Secured resident reference for canonical hosted native qualification.
//! Uses the existing runtime, real CLI provider and ordinary capability policy.
//! This is not native AOT or an old-I/O-versus-secured benchmark result.
use std::{
    ffi::OsString,
    io::{Read, Write},
    path::Path,
};

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

fn read_bounded(mut reader: impl Read, max_source_bytes: Option<u64>) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    match max_source_bytes {
        Some(limit) => reader.take(limit.saturating_add(1)).read_to_end(&mut bytes),
        None => reader.read_to_end(&mut bytes),
    }
    .map_err(|error| error.to_string())?;
    Ok(bytes)
}

fn compile_source(
    reader: impl Read,
    config: mech_runtime::RuntimeConfig,
) -> Result<Vec<u8>, String> {
    let bytes = read_bounded(reader, config.limits.max_source_bytes)?;
    let mut compiler = builder(Vec::new())?
        .config(config)
        .build_compiler()
        .map_err(diagnostic)?;
    Ok(compiler
        .compile_canonical_source_bytes(&bytes)
        .map_err(diagnostic)?
        .bytecode()
        .to_vec())
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| error.to_string())?;
    file.write_all(bytes).map_err(|error| error.to_string())
}

fn load_bytecode(
    runtime: &mut MechRuntime,
    reader: impl Read,
) -> Result<RuntimeProgramLoadOutcome, String> {
    let bytecode = read_bounded(reader, runtime.config().limits.max_source_bytes)?;
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
            let source = std::fs::File::open(source).map_err(|error| error.to_string())?;
            let bytecode = compile_source(source, mech_runtime::RuntimeConfig::default())?;
            write_new(Path::new(&destination), &bytecode)?;
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
                write_new(report, &serde_json::to_vec_pretty(&document).unwrap())?;
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

    #[test]
    fn source_acquisition_preserves_compiler_admission_and_utf8() {
        let source = "answer := \"é👋\"\nanswer\n";
        let mut config = mech_runtime::RuntimeConfig::default();
        config.limits.max_source_bytes = Some(source.len() as u64);
        let actual = compile_source(Cursor::new(source), config.clone()).unwrap();
        let expected = builder(Vec::new())
            .unwrap()
            .config(config.clone())
            .build_compiler()
            .unwrap()
            .compile_canonical_source(source)
            .unwrap()
            .bytecode()
            .to_vec();
        assert_eq!(actual, expected);

        config.limits.max_source_bytes = Some(64);
        let oversized = format!("-- {}\nanswer := 7\nanswer\n", "x".repeat(1024 * 1024));
        let mut reader = Cursor::new(oversized.as_bytes());
        let actual = compile_source(&mut reader, config.clone()).unwrap_err();
        assert_eq!(reader.position(), 65);
        let expected = builder(Vec::new())
            .unwrap()
            .config(config.clone())
            .build_compiler()
            .unwrap()
            .compile_canonical_source(&oversized[..65])
            .err()
            .unwrap();
        assert!(
            expected
                .kind_as::<mech_runtime::ResourceBudgetExceededError>()
                .is_some()
        );
        assert_eq!(actual, diagnostic(expected));
        assert!(compile_source(Cursor::new([0xff]), config.clone()).is_err());
        config.limits.max_source_bytes = None;
        let mut reader = Cursor::new(oversized.as_bytes());
        let actual = compile_source(&mut reader, config.clone()).unwrap_err();
        assert_eq!(reader.position(), oversized.len() as u64);
        // None removes only the source-byte cap, not independent parser limits.
        let expected = builder(Vec::new())
            .unwrap()
            .config(config.clone())
            .build_compiler()
            .unwrap()
            .compile_canonical_source(&oversized)
            .err()
            .unwrap();
        assert_eq!(actual, diagnostic(expected));
        assert_eq!(
            compile_source(Cursor::new(source), config).unwrap(),
            bytecode(source)
        );
    }

    #[test]
    fn oversized_utf8_source_is_denied_before_decoding_a_truncated_character() {
        for character in ["é", "界", "👋"] {
            // The first64 bytes are ASCII; the next character needs2..4 bytes.
            let source = format!("-- {}{character}\nanswer := 7\nanswer\n", "x".repeat(61));
            assert!(!bytecode(&source).is_empty(), "valid complete source");
            for limit in [64u64, 65, 66] {
                let mut config = mech_runtime::RuntimeConfig::default();
                config.limits.max_source_bytes = Some(limit);
                let mut reader = Cursor::new(source.as_bytes());
                let actual = compile_source(&mut reader, config.clone()).unwrap_err();
                assert_eq!(reader.position(), limit + 1);
                // The reader knows only the acquired prefix length, just like
                // bounded bytecode loading. Compare ordinary admission for
                // that same length, without requiring a valid UTF-8 prefix.
                let expected = builder(Vec::new())
                    .unwrap()
                    .config(config)
                    .build_compiler()
                    .unwrap()
                    .compile_canonical_source(&" ".repeat(limit as usize + 1))
                    .err()
                    .unwrap();
                let budget = expected
                    .kind_as::<mech_runtime::ResourceBudgetExceededError>()
                    .unwrap();
                assert_eq!(budget.resource, "source_bytes");
                assert_eq!(budget.requested, limit + 1);
                assert_eq!(budget.max, Some(limit));
                assert_eq!(actual, diagnostic(expected), "{character:?}, limit={limit}");
            }
        }
    }

    #[test]
    fn qualification_outputs_refuse_concurrent_creators_without_truncation() {
        let directory = std::env::temp_dir().join(format!(
            "mech-reference-outputs-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&directory).unwrap();
        // Both bytecode and report publication call this same create-new writer.
        for name in ["program.mecb", "report.json"] {
            let destination = directory.join(name);
            assert!(!destination.exists());
            let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
            let workers = [b"first".as_slice(), b"second".as_slice()].map(|bytes| {
                let barrier = barrier.clone();
                let destination = destination.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    (write_new(&destination, bytes), bytes)
                })
            });
            let results = workers.map(|worker| worker.join().unwrap());
            assert_eq!(
                results.iter().filter(|(result, _)| result.is_ok()).count(),
                1
            );
            let winner = results.iter().find(|(result, _)| result.is_ok()).unwrap();
            assert_eq!(std::fs::read(&destination).unwrap(), winner.1);
            assert!(write_new(&destination, b"replacement").is_err());
            assert_eq!(std::fs::read(&destination).unwrap(), winner.1);
            std::fs::remove_file(destination).unwrap();
        }
        std::fs::remove_dir(directory).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn qualification_outputs_refuse_dangling_symlinks() {
        let directory = std::env::temp_dir().join(format!(
            "mech-reference-symlinks-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&directory).unwrap();
        for name in ["program.mecb", "report.json"] {
            let destination = directory.join(name);
            let target = directory.join(format!("{name}-target"));
            std::os::unix::fs::symlink(&target, &destination).unwrap();
            assert!(!destination.exists());
            assert!(write_new(&destination, b"must not follow").is_err());
            assert!(!target.exists());
            std::fs::remove_file(destination).unwrap();
        }
        std::fs::remove_dir(directory).unwrap();
    }
}
