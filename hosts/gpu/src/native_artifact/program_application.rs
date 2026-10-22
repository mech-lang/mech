//! Managed canonical Object application over the existing pinned generator.
use super::application::{
    GeneratedLinkedApplication, LinkedApplicationInputs, LinkedApplicationProfile,
    NativeApplicationError, prepare_destination, render_main, write_linked_application,
};
use mech_core::FunctionCatalog;
use mech_engine::{ProgramArtifact, encode_program_artifact_bytecode_v1};
use std::path::Path;

/// Generates a runtime-only linked application using the exact same checked
/// CFG and managed-value boundary as JIT execution. The shell accepts typed
/// input drafts and emits canonical snapshots for qualification; it does not
/// implement program semantics or claim native resource-host integration.
/// Generation neither links nor executes the result.
pub fn generate_linked_program_application(
    workspace: &Path,
    directory: &Path,
    binary_name: &str,
    artifact: &ProgramArtifact,
    catalog: &FunctionCatalog,
    source_provenance: &[u8],
) -> Result<GeneratedLinkedApplication, NativeApplicationError> {
    let workspace = prepare_destination(workspace, directory, binary_name)?;
    let object =
        super::emit_program_object(artifact, catalog).map_err(NativeApplicationError::Compile)?;
    let bytecode = encode_program_artifact_bytecode_v1(artifact)
        .map_err(|error| NativeApplicationError::Encoding(format!("{error:?}")))?;
    let main = render_main(
        include_str!("templates/linked_program_main.rs.in"),
        object.identity().symbol(),
        &bytecode,
    );
    write_linked_application(
        &workspace,
        directory,
        binary_name,
        LinkedApplicationInputs {
            profile: LinkedApplicationProfile::ManagedProgram,
            schemas: artifact.schemas(),
            factory_linkage:
                mech_engine::linked_factories::NativeFactoryLinkageProfile::derive_native_image(
                    object.descriptor().plan(),
                    catalog,
                ),
            source_provenance,
            bytecode,
            object: object.bytes(),
            identity: object.identity().bytes(),
            main,
            revision: artifact.revision().as_bytes(),
            typed_ir_sha256: object.descriptor().typed_ir_sha256(),
            object_metadata: object.metadata(),
            descriptor_image: Some(
                object
                    .descriptor()
                    .encode_build_owned_image()
                    .map_err(|error| NativeApplicationError::Encoding(error.to_string()))?,
            ),
            manifest_schema: "mech.canonical-linked-program-application.v1",
            resource_image: None,
            route: "canonical-artifact-program-cranelift-object",
        },
    )
}

/// Generate a real capability-secured application for the complete original
/// canonical artifact. Source is pinned provenance only; the linked shell has
/// no parser, JIT, source-native frontend or resident body execution.
/// Generation emits files only, not a build, execution or timing result.
#[cfg(feature = "canonical-native-program-host")]
pub fn generate_linked_hosted_program_application(
    workspace: &Path,
    directory: &Path,
    binary_name: &str,
    artifact: &ProgramArtifact,
    catalog: &FunctionCatalog,
    source_provenance: &[u8],
) -> Result<GeneratedLinkedApplication, NativeApplicationError> {
    generate_hosted(
        workspace,
        directory,
        binary_name,
        artifact,
        catalog,
        source_provenance,
        false,
    )
}

/// Explicit experiment using the SAME Object, provider and phased launcher.
/// The generated binary requires a launch-time `benchmark_policy` of secured
/// or trusted; no environment switch or production default enables it.
/// Trusted bypasses resource/operation decisions for live in-scope unlimited
/// launch grants, not memory/work/schema/transaction/provider/lifecycle checks.
/// Do not distribute this benchmark binary as a secured production executable.
#[cfg(feature = "canonical-native-benchmark-policy")]
pub fn generate_linked_benchmark_program_application(
    workspace: &Path,
    directory: &Path,
    binary_name: &str,
    artifact: &ProgramArtifact,
    catalog: &FunctionCatalog,
    source_provenance: &[u8],
) -> Result<GeneratedLinkedApplication, NativeApplicationError> {
    generate_hosted(
        workspace,
        directory,
        binary_name,
        artifact,
        catalog,
        source_provenance,
        true,
    )
}

#[cfg(feature = "canonical-native-program-host")]
fn generate_hosted(
    workspace: &Path,
    directory: &Path,
    binary_name: &str,
    artifact: &ProgramArtifact,
    catalog: &FunctionCatalog,
    source_provenance: &[u8],
    benchmark_policy: bool,
) -> Result<GeneratedLinkedApplication, NativeApplicationError> {
    let workspace = prepare_destination(workspace, directory, binary_name)?;
    if !artifact.inputs().is_empty() {
        return Err(NativeApplicationError::Invalid(
            "hosted application must bind its application inputs through resources",
        ));
    }
    // Metadata only: exact contracts, original Live observations, Snapshot sends
    // and shared canonical effect order. No provider/input is observed here.
    let resources = mech_runtime::CanonicalNativeResourceMetadata::prepare(artifact, usize::MAX)
        .map_err(|error| NativeApplicationError::Encoding(format!("{error:?}")))?;
    let object =
        super::emit_program_object(artifact, catalog).map_err(NativeApplicationError::Compile)?;
    let bytecode = encode_program_artifact_bytecode_v1(artifact)
        .map_err(|error| NativeApplicationError::Encoding(format!("{error:?}")))?;
    let route = if benchmark_policy {
        "canonical-artifact-program-object-benchmark-cli"
    } else {
        "canonical-artifact-program-object-secured-cli"
    };
    let main = render_hosted_main(
        object.identity().symbol(),
        &bytecode,
        route,
        benchmark_policy,
    );
    write_linked_application(
        &workspace,
        directory,
        binary_name,
        LinkedApplicationInputs {
            profile: LinkedApplicationProfile::HostedManagedProgram { benchmark_policy },
            schemas: artifact.schemas(),
            factory_linkage:
                mech_engine::linked_factories::NativeFactoryLinkageProfile::derive_native_image(
                    object.descriptor().plan(),
                    catalog,
                ),
            source_provenance,
            bytecode,
            object: object.bytes(),
            identity: object.identity().bytes(),
            main,
            revision: artifact.revision().as_bytes(),
            typed_ir_sha256: object.descriptor().typed_ir_sha256(),
            object_metadata: object.metadata(),
            descriptor_image: Some(
                object
                    .descriptor()
                    .encode_build_owned_image()
                    .map_err(|error| NativeApplicationError::Encoding(error.to_string()))?,
            ),
            resource_image: Some(
                resources
                    .encode_build_owned_image()
                    .map_err(|error| NativeApplicationError::Encoding(error.to_string()))?,
            ),
            manifest_schema: if benchmark_policy {
                "mech.canonical-linked-benchmark-program-application.v1"
            } else {
                "mech.canonical-linked-hosted-program-application.v1"
            },
            route,
        },
    )
}

#[cfg(feature = "canonical-native-program-host")]
fn render_hosted_main(symbol: &str, bytecode: &[u8], route: &str, benchmark: bool) -> String {
    let main = render_main(
        include_str!("templates/linked_hosted_program_main.rs.in"),
        symbol,
        bytecode,
    );
    let controls = if benchmark {
        [
            "use mech_gpu::native_artifact::{NativeBenchmarkCapabilityKernel, NativeBenchmarkPolicy};",
            "benchmark_policy: String,",
            "let benchmark_policy = NativeBenchmarkPolicy::parse(&launch.benchmark_policy).map_err(diagnostic)?;",
            ".capability_kernel(NativeBenchmarkCapabilityKernel::new(benchmark_policy, launch.grants.len(), &memory).map_err(diagnostic)?)",
            "benchmark_policy.name()",
        ]
    } else {
        ["", "", "", "", "\"secured\""]
    };
    let mut main = main.replace(
        "@HOSTED_ROUTE_LITERAL@",
        &serde_json::to_string(route).unwrap(),
    );
    for (marker, replacement) in [
        "@BENCHMARK_POLICY_IMPORT@",
        "@BENCHMARK_POLICY_FIELD@",
        "@BENCHMARK_POLICY_PARSE@",
        "@BENCHMARK_POLICY_KERNEL@",
        "@HOSTED_POLICY_EXPRESSION@",
    ]
    .into_iter()
    .zip(controls)
    {
        main = main.replace(marker, replacement);
    }
    main
}
