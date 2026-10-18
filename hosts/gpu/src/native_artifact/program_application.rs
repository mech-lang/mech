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
            source_provenance,
            bytecode,
            object: object.bytes(),
            identity: object.identity().bytes(),
            main,
            revision: artifact.revision().as_bytes(),
            typed_ir_sha256: object.descriptor().typed_ir_sha256(),
            object_metadata: object.metadata(),
            manifest_schema: "mech.canonical-linked-program-application.v1",
            route: "canonical-artifact-program-cranelift-object",
        },
    )
}
