//! Thin canonical Match route over the common pinned workspace generator.
use super::application::{
    GeneratedLinkedApplication, LinkedApplicationInputs, NativeApplicationError,
    prepare_destination, render_main, write_linked_application,
};
use super::emit_match_object;
use mech_core::FunctionCatalog;
use mech_engine::native_artifact::lower_scalar_match_artifact;
use mech_engine::{ProgramArtifact, encode_program_artifact_bytecode_v1};
use std::path::Path;

/// Emits and pins a real checked Match Object application. Source bytes are
/// provenance only. Generation does not run Cargo, a linker or an executable.
pub fn generate_linked_match_application(
    workspace: &Path,
    directory: &Path,
    binary_name: &str,
    artifact: &ProgramArtifact,
    catalog: &FunctionCatalog,
    source_provenance: &[u8],
) -> Result<GeneratedLinkedApplication, NativeApplicationError> {
    let workspace = prepare_destination(workspace, directory, binary_name)?;
    let plan =
        lower_scalar_match_artifact(artifact, catalog).map_err(NativeApplicationError::Lowering)?;
    let object = emit_match_object(&plan).map_err(NativeApplicationError::Compile)?;
    let bytecode = encode_program_artifact_bytecode_v1(artifact)
        .map_err(|error| NativeApplicationError::Encoding(format!("{error:?}")))?;
    let main = render_main(
        include_str!("templates/linked_match_main.rs.in"),
        object.identity().symbol(),
        &bytecode,
    );
    write_linked_application(
        &workspace,
        directory,
        binary_name,
        LinkedApplicationInputs {
            source_provenance,
            bytecode,
            object: object.bytes(),
            identity: object.identity().bytes(),
            main,
            revision: artifact.revision().as_bytes(),
            typed_ir_sha256: object.descriptor().typed_ir_sha256(),
            object_metadata: object.metadata(),
            manifest_schema: "mech.canonical-linked-match-application.v1",
            route: "canonical-artifact-match-cranelift-object",
        },
    )
}
