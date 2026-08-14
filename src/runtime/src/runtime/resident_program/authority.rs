use std::collections::BTreeSet;

use mech_core::{
    ApplicationRequirement, MResult, MechError, MechErrorKind,
    canonical_application_requirement_bytes,
};
use mech_engine::ProgramArtifact;

use crate::{
    CapabilityId, CapabilityRequest, ResidentExternalAuthority, RuntimeAuthorityScope,
    RuntimeCapabilityOperation, RuntimeResourceKey, runtime::MechRuntime,
};

use super::{ResidentRouteFailureClass, route_failure};

#[derive(Clone, Debug)]
pub(crate) struct ResidentAdmissionGrant {
    requirement: ApplicationRequirement,
    request: CapabilityRequest,
    capability: CapabilityId,
    authority_scope: RuntimeAuthorityScope,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ResidentAdmissionProof {
    grants: Box<[ResidentAdmissionGrant]>,
    requirements: BTreeSet<Vec<u8>>,
}

impl ResidentExternalAuthority for ResidentAdmissionProof {
    fn authorize(&self, requirement: &ApplicationRequirement) -> MResult<()> {
        let canonical = canonical_application_requirement_bytes(requirement)?;
        if self.requirements.contains(&canonical) {
            Ok(())
        } else {
            Err(MechError::new(
                ResidentGrantInvalid {
                    reason: "requirement is absent from the admitted resident grant set".to_owned(),
                },
                None,
            ))
        }
    }
}

impl ResidentAdmissionProof {
    pub(crate) fn revalidate(&self, runtime: &MechRuntime) -> MResult<()> {
        for grant in &self.grants {
            if !grant.authority_scope.contains(grant.capability) {
                return Err(denied(
                    "stored authority scope no longer contains its capability",
                ));
            }
            let capability = runtime
                .get_capability(grant.capability)?
                .ok_or_else(|| denied("stored capability is no longer available"))?;
            let max_uses = crate::runtime::extension::invoke_extension_value(
                "capability",
                "max_uses",
                || capability.max_uses(),
            )?;
            if max_uses.is_some() {
                return Err(denied(
                    "finite-use capabilities cannot authorize a resident session",
                ));
            }
            let mut context = runtime.runtime_context()?;
            context.authority = RuntimeAuthorityScope::allow_list([grant.capability]);
            let selected = runtime
                .preview_capability_for_execution(&context, &grant.request)
                .map_err(|error| {
                    denied(format!(
                        "resident capability revalidation failed for {:?}: {error:?}",
                        grant.requirement,
                    ))
                })?;
            if selected != grant.capability {
                return Err(denied(
                    "resident capability revalidation selected a different capability",
                ));
            }
        }
        Ok(())
    }
}

impl MechRuntime {
    pub(crate) fn build_resident_authority(
        &self,
        artifact: &ProgramArtifact,
    ) -> MResult<ResidentAdmissionProof> {
        let context = self.runtime_context()?;
        let mut grants = Vec::with_capacity(artifact.requirements().len());
        let mut requirements = BTreeSet::new();
        for (_, requirement) in artifact.requirements().iter() {
            let ApplicationRequirement::Resource(resource) = requirement else {
                return Err(route_failure(
                    ResidentRouteFailureClass::SemanticUnsupported,
                    "resident host-function requirements are unsupported",
                ));
            };
            let key = RuntimeResourceKey::new(&resource.base_uri, &resource.path)?;
            let operation = RuntimeCapabilityOperation::from_name(resource.operation.clone())?;
            let request = CapabilityRequest::from_keys(
                &context.subject,
                operation.name(),
                key.capability_resource(),
            );
            let capability = self
                .preview_capability_for_execution(&context, &request)
                .map_err(|error| {
                    route_failure(
                        ResidentRouteFailureClass::AuthorizationDenied,
                        format!("resident requirement authorization denied: {error:?}"),
                    )
                })?;
            let admitted = self
                .get_capability(capability)?
                .ok_or_else(|| denied("preview selected an unavailable capability"))?;
            let max_uses = crate::runtime::extension::invoke_extension_value(
                "capability",
                "max_uses",
                || admitted.max_uses(),
            )?;
            if max_uses.is_some() {
                return Err(route_failure(
                    ResidentRouteFailureClass::AuthorizationDenied,
                    "finite-use capabilities cannot authorize a resident session",
                ));
            }
            requirements.insert(canonical_application_requirement_bytes(requirement)?);
            grants.push(ResidentAdmissionGrant {
                requirement: requirement.clone(),
                request,
                capability,
                authority_scope: context.authority.clone(),
            });
        }
        let grant_set = ResidentAdmissionProof {
            grants: grants.into_boxed_slice(),
            requirements,
        };
        grant_set.revalidate(self)?;
        Ok(grant_set)
    }

    pub(crate) fn revalidate_active_resident_grants(&self) -> MResult<()> {
        let super::ActiveProgramExecution::ResidentExternal(execution) = &self.active_program
        else {
            return Ok(());
        };
        execution.grants.revalidate(self)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResidentGrantInvalid {
    pub reason: String,
}

impl MechErrorKind for ResidentGrantInvalid {
    fn name(&self) -> &str {
        "ResidentGrantInvalid"
    }

    fn message(&self) -> String {
        self.reason.clone()
    }
}

fn denied(reason: impl Into<String>) -> MechError {
    route_failure(ResidentRouteFailureClass::AuthorizationDenied, reason)
}
