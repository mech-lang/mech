//! Thin canonical Iteration route over the common pinned workspace generator.
use crate::native_artifact::application::{
    GeneratedLinkedApplication, LinkedApplicationInputs, LinkedApplicationProfile,
    NativeApplicationError, prepare_destination, render_main, write_linked_application,
};
use crate::native_artifact::emit_iteration_object;
use mech_core::FunctionCatalog;
use mech_engine::native_artifact::lower_scalar_iteration_artifact;
use mech_engine::{ProgramArtifact, encode_program_artifact_bytecode_v1};
use std::path::Path;

/// Emits and pins a real checked Iteration Object application. Source bytes are
/// provenance only. Generation does not run Cargo, a linker or an executable.
pub fn generate_linked_iteration_application(
    workspace: &Path,
    directory: &Path,
    binary_name: &str,
    artifact: &ProgramArtifact,
    catalog: &FunctionCatalog,
    source_provenance: &[u8],
) -> Result<GeneratedLinkedApplication, NativeApplicationError> {
    let workspace = prepare_destination(workspace, directory, binary_name)?;
    let plan = lower_scalar_iteration_artifact(artifact, catalog)
        .map_err(NativeApplicationError::Lowering)?;
    let object = emit_iteration_object(&plan).map_err(NativeApplicationError::Compile)?;
    let bytecode = encode_program_artifact_bytecode_v1(artifact)
        .map_err(|error| NativeApplicationError::Encoding(format!("{error:?}")))?;
    let main = render_main(
        include_str!("templates/linked_iteration_main.rs.in"),
        object.identity().symbol(),
        &bytecode,
    );
    write_linked_application(
        &workspace,
        directory,
        binary_name,
        LinkedApplicationInputs {
            profile: LinkedApplicationProfile::Scalar,
            schemas: artifact.schemas(),
            factory_linkage: mech_engine::linked_factories::NativeFactoryLinkageProfile::derive(
                artifact, catalog,
            ),
            source_provenance,
            bytecode,
            object: object.bytes(),
            identity: object.identity().bytes(),
            main,
            revision: artifact.revision().as_bytes(),
            typed_ir_sha256: object.descriptor().typed_ir_sha256(),
            object_metadata: object.metadata(),
            descriptor_image: None,
            resource_image: None,
            manifest_schema: "mech.canonical-linked-iteration-application.v1",
            route: "canonical-artifact-iteration-cranelift-object",
        },
    )
}
