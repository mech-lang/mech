use core::cmp::Ordering;

use mech_core::{
    ApplicationRequirement, ApplicationRequirementId, canonical_application_requirement_bytes,
    compare_application_requirements, validate_application_requirement,
};

use super::ArtifactBuildError;

/// Immutable, canonical authority for every application requirement used by
/// nodes in a semantic program artifact.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApplicationRequirementTable {
    entries: Box<[ApplicationRequirement]>,
}

impl ApplicationRequirementTable {
    pub fn empty() -> Self {
        Self {
            entries: Box::new([]),
        }
    }

    pub fn from_canonical_entries(
        entries: Vec<ApplicationRequirement>,
    ) -> Result<Self, ArtifactBuildError> {
        for requirement in &entries {
            validate_application_requirement(requirement)?;
            canonical_application_requirement_bytes(requirement)?;
        }
        if entries
            .windows(2)
            .any(|pair| compare_application_requirements(&pair[0], &pair[1]) != Ordering::Less)
        {
            return Err(ArtifactBuildError::NonCanonicalApplicationRequirementTable);
        }
        Ok(Self {
            entries: entries.into_boxed_slice(),
        })
    }

    pub fn get(&self, id: ApplicationRequirementId) -> Option<&ApplicationRequirement> {
        self.entries.get(id.get() as usize)
    }

    pub fn iter(
        &self,
    ) -> impl ExactSizeIterator<Item = (ApplicationRequirementId, &ApplicationRequirement)> {
        self.entries
            .iter()
            .enumerate()
            .map(|(index, requirement)| (ApplicationRequirementId::new(index as u32), requirement))
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for ApplicationRequirementTable {
    fn default() -> Self {
        Self::empty()
    }
}
