use mech_core::{ApplicationRequirement, MResult};
use mech_engine::ProgramArtifact;

use crate::{
    ExactRequirementAuthority, RuntimeCapabilityOperation, RuntimeResourceKey, runtime::MechRuntime,
};

use super::{ResidentRouteFailureClass, route_failure};

impl MechRuntime {
    pub(crate) fn build_resident_authority(
        &mut self,
        artifact: &ProgramArtifact,
    ) -> MResult<ExactRequirementAuthority> {
        let mut context = self.runtime_context()?;
        let mut authorized = Vec::with_capacity(artifact.requirements().len());
        for (_, requirement) in artifact.requirements().iter() {
            let ApplicationRequirement::Resource(request) = requirement else {
                return Err(route_failure(
                    ResidentRouteFailureClass::SemanticUnsupported,
                    "resident host-function requirements are unsupported",
                ));
            };
            let key = RuntimeResourceKey::new(&request.base_uri, &request.path)?;
            let operation = RuntimeCapabilityOperation::from_name(request.operation.clone())?;
            self.authorize_resource_with_context(&mut context, &operation, &key)
                .map_err(|error| {
                    route_failure(
                        ResidentRouteFailureClass::AuthorizationDenied,
                        format!("resident requirement authorization denied: {error:?}"),
                    )
                })?;
            authorized.push(requirement.clone());
        }
        ExactRequirementAuthority::new(authorized)
    }
}
