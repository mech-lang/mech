use std::collections::BTreeSet;

use mech_core::{
    ApplicationRequirement, MResult, MechError, MechErrorKind,
    canonical_application_requirement_bytes,
};

pub trait ResidentExternalAuthority: core::fmt::Debug {
    fn authorize(&self, requirement: &ApplicationRequirement) -> MResult<()>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DenyAllResidentExternalAuthority;

impl ResidentExternalAuthority for DenyAllResidentExternalAuthority {
    fn authorize(&self, requirement: &ApplicationRequirement) -> MResult<()> {
        Err(denied(requirement))
    }
}

#[derive(Clone, Debug, Default)]
pub struct ExactRequirementAuthority {
    requirements: BTreeSet<Vec<u8>>,
}

impl ExactRequirementAuthority {
    pub fn new(requirements: impl IntoIterator<Item = ApplicationRequirement>) -> MResult<Self> {
        let requirements = requirements
            .into_iter()
            .map(|requirement| canonical_application_requirement_bytes(&requirement))
            .collect::<MResult<BTreeSet<_>>>()?;
        Ok(Self { requirements })
    }
}

impl ResidentExternalAuthority for ExactRequirementAuthority {
    fn authorize(&self, requirement: &ApplicationRequirement) -> MResult<()> {
        let canonical = canonical_application_requirement_bytes(requirement)?;
        if self.requirements.contains(&canonical) {
            Ok(())
        } else {
            Err(denied(requirement))
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResidentExternalRequirementDenied {
    pub requirement: ApplicationRequirement,
}

impl MechErrorKind for ResidentExternalRequirementDenied {
    fn name(&self) -> &str {
        "ResidentExternalRequirementDenied"
    }

    fn message(&self) -> String {
        format!(
            "resident external authority denied exact requirement {:?}",
            self.requirement
        )
    }
}

fn denied(requirement: &ApplicationRequirement) -> MechError {
    MechError::new(
        ResidentExternalRequirementDenied {
            requirement: requirement.clone(),
        },
        None,
    )
}
