//! Public canonical AOT adapter; no source-native frontend or execution fallback.
use super::*;
use mech_build::{CargoProjectPaths, build_cargo_project, generate_cargo_project_lockfile};
use mech_engine::{ProgramArtifact, decode_program_artifact_bytecode_v1};
use mech_gpu::native_artifact::{
    GeneratedLinkedApplication, generate_linked_hosted_program_application,
    validate_linked_application_binary_name,
};
use serde_json::{Value as Json, json};

fn native_error(error: impl std::fmt::Debug) -> MechError {
    MechError::new(
        GenericError {
            msg: format!("canonical native build: {error:?}"),
        },
        None,
    )
}
fn validate_options(options: &BuildOptions) -> MResult<()> {
    if options.emit != BuildEmit::Native || options.target.is_some() {
        return build_error(
            "canonical Cranelift emits native applications for the current host only; use --emit native without --target",
        );
    }
    if options.native_scalar || options.native_loop_fma {
        return build_error(
            "--scalar and --fp-contract-loop are unsupported by canonical Cranelift builds",
        );
    }
    if options.config_path.is_some() || options.debug || options.trace {
        return build_error(
            "canonical Cranelift builds do not accept runtime configuration, --debug or --trace; use the executable's explicit launch JSON",
        );
    }
    Ok(())
}
fn revision(artifact: &ProgramArtifact) -> String {
    artifact
        .revision()
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
fn source_record(source: &mech_runtime::CanonicalCompilationSource) -> Json {
    let document = source.document();
    json!({
        "canonical_uri": source.canonical_uri(),
        "document_revision": document.source().revision().0,
        "nominal_origin_canonical_bytes": document.nominal_origin().map(|origin| origin.canonical_bytes()),
        "nominal_package_id": document.nominal_package_id(),
        "source_utf8_bytes": document.source().to_contiguous_string().into_bytes(),
    })
}
fn provenance(
    artifact: &ProgramArtifact,
    sources: Option<&mech_runtime::CanonicalCompilationSources>,
) -> MResult<Vec<u8>> {
    let (pairing, roots, dependencies) = match sources {
        Some(sources) => (
            "retained-canonical-compilation",
            sources
                .roots()
                .iter()
                .map(source_record)
                .collect::<Vec<_>>(),
            sources
                .dependencies()
                .iter()
                .map(source_record)
                .collect::<Vec<_>>(),
        ),
        None => ("source-unpaired-bytecode", Vec::new(), Vec::new()),
    };
    serde_json::to_vec_pretty(&json!({
        "schema": "mech.canonical-native-build-provenance.v1",
        "provenance_only": true,
        "source_pairing": pairing,
        "program_revision": revision(artifact),
        "roots": roots,
        "dependencies": dependencies,
    }))
    .map_err(native_error)
}
fn generate(
    prepared: &PreparedBuildInput,
    workspace: &Path,
    directory: &Path,
    name: &str,
) -> MResult<GeneratedLinkedApplication> {
    match &prepared.origin {
        PreparedBuildOrigin::CanonicalSource {
            _artifact: artifact,
            _sources: sources,
            ..
        } => {
            let provenance = provenance(artifact, Some(sources))?;
            generate_linked_hosted_program_application(
                workspace,
                directory,
                name,
                artifact,
                sources.function_catalog(),
                &provenance,
            )
            .map_err(native_error)
        }
        PreparedBuildOrigin::SourceUnpairedBytecode => {
            // Bytecode has no authenticated original source/catalog pairing.
            // Revalidate its complete artifact against the current standard catalog.
            let artifact =
                decode_program_artifact_bytecode_v1(&prepared.bytecode).map_err(native_error)?;
            let catalog = mech_stdlib::source_catalog();
            let provenance = provenance(&artifact, None)?;
            generate_linked_hosted_program_application(
                workspace,
                directory,
                name,
                &artifact,
                &catalog,
                &provenance,
            )
            .map_err(native_error)
        }
    }
}
fn copy_project(source: &Path, destination: &Path) -> MResult<()> {
    fs::create_dir(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_dir() {
            copy_project(&entry.path(), &destination.join(entry.file_name()))?;
        } else if kind.is_file() {
            copy_exact_file(&entry.path(), &destination.join(entry.file_name()))?;
        } else {
            return build_error("refusing nonregular entry in generated canonical Cargo project");
        }
    }
    Ok(())
}
pub(super) fn run(options: BuildOptions) -> MResult<CliOutcome> {
    validate_options(&options)?;
    let bytecode_count = validate_build_bytecode_inputs(&options.paths)?;
    let name = options
        .name
        .clone()
        .unwrap_or_else(|| inferred_binary_name(&options.paths[0]));
    validate_linked_application_binary_name(&name).map_err(|error| native_error(format!(
        "canonical linked binary names must be 1..64 ASCII letters/digits/hyphens, starting with a letter: {error}"
    )))?;
    let workspace = options
        .workspace_root
        .clone()
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")))
        .canonicalize()?;
    let output = options
        .output_path
        .clone()
        .unwrap_or_else(|| default_output_path(BuildEmit::Native, &name));
    let kept_project = if options.keep_project {
        let destination = project_output_path(&output);
        refuse_existing_project_output(&destination)?;
        Some(destination)
    } else {
        None
    };
    // Preserve collision rejection before compilation, Object emission or writes.
    let (prepared, _) = prepare_build_input(&options, &name, bytecode_count, true)?;
    let temporary = tempfile::Builder::new()
        .prefix("mech-canonical-aot-")
        .tempdir()?;
    let project = generate(&prepared, &workspace, &temporary.path().join("app"), &name)?;
    // This is only a path view: Object/source pin authority stays in SAME
    // generated build.rs; the shared Cargo helpers own frozen/isolated building.
    let paths = CargoProjectPaths::new(project.root());
    let seed = fs::read(workspace.join("Cargo.lock"))?;
    generate_cargo_project_lockfile(&paths, &seed, options.offline)?;
    let artifact = build_cargo_project(
        &paths,
        &name,
        None,
        &temporary.path().join("cargo-target"),
        options.profile == BuildProfile::Release,
        options.offline,
    )?;
    copy_exact_file(&artifact.executable, &output)?;
    if let Some(destination) = kept_project {
        copy_project(project.root(), &destination)?;
    }
    println!(
        "[Output] Canonical Cranelift AOT Mech application written to: {}",
        output.display()
    );
    println!(
        "[Launch] {} LAUNCH_JSON REPORT_OR_DASH -- [PAYLOAD_ARGUMENTS...]",
        output.display()
    );
    Ok(CliOutcome::success())
}

#[cfg(test)]
#[path = "build_cranelift_tests.rs"]
mod tests;
