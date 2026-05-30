use std::sync::Arc;

use mech_core::{MResult, MechError, MechErrorKind};

use crate::{Capability, CapabilityId, MechRuntime, RuntimeCapabilityGrantSpec};

#[derive(Clone, Debug, Default)]
pub struct RuntimeCapabilityGrantRegistry {
    grants: Vec<RuntimeCapabilityGrant>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeCapabilityGrant {
    pub subject: String,
    pub resource: String,
    pub operations: Vec<String>,
    pub paths: Vec<String>,
}

impl RuntimeCapabilityGrantRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_spec(&mut self, spec: RuntimeCapabilityGrantSpec) -> MResult<()> {
        self.add_grant(RuntimeCapabilityGrant {
            subject: spec.subject,
            resource: spec.resource,
            operations: spec.operations,
            paths: spec.paths,
        })
    }

    pub fn add_grant(&mut self, grant: RuntimeCapabilityGrant) -> MResult<()> {
        validate_grant(&grant)?;
        self.grants.push(grant);
        Ok(())
    }

    pub fn allows(&self, subject: &str, resource: &str, operation: &str, path: &str) -> bool {
        self.grants.iter().any(|grant| {
            grant.subject == subject
                && grant.resource == resource
                && grant.operations.iter().any(|allowed| allowed == operation)
                && grant
                    .paths
                    .iter()
                    .any(|allowed| grant_path_matches(allowed, path))
        })
    }

    pub fn is_empty(&self) -> bool {
        self.grants.is_empty()
    }

    pub fn len(&self) -> usize {
        self.grants.len()
    }
}

fn validate_grant(grant: &RuntimeCapabilityGrant) -> MResult<()> {
    if grant.subject.is_empty() {
        return invalid_grant("subject cannot be empty");
    }
    if grant.resource.is_empty() {
        return invalid_grant("resource cannot be empty");
    }
    if grant
        .operations
        .iter()
        .any(|operation| operation.is_empty())
    {
        return invalid_grant("operation cannot be empty");
    }
    if grant.paths.iter().any(|path| path.is_empty()) {
        return invalid_grant("path cannot be empty");
    }
    Ok(())
}

fn invalid_grant(reason: impl Into<String>) -> MResult<()> {
    Err(MechError::new(
        RuntimeCapabilityGrantInvalid {
            reason: reason.into(),
        },
        None,
    ))
}

fn grant_path_matches(grant_path: &str, requested_path: &str) -> bool {
    if grant_path == "*" || grant_path == requested_path {
        return true;
    }
    if let Some(prefix) = grant_path.strip_suffix("/*") {
        return requested_path.starts_with(&format!("{}/", prefix));
    }
    false
}

#[derive(Debug, Clone)]
pub struct RuntimeCapabilityGrantInvalid {
    pub reason: String,
}

impl MechErrorKind for RuntimeCapabilityGrantInvalid {
    fn name(&self) -> &str {
        "RuntimeCapabilityGrantInvalid"
    }

    fn message(&self) -> String {
        format!("invalid runtime capability grant: {}", self.reason)
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeCapabilityGrantDenied {
    pub subject: String,
    pub resource: String,
    pub operation: String,
    pub path: String,
}

impl MechErrorKind for RuntimeCapabilityGrantDenied {
    fn name(&self) -> &str {
        "RuntimeCapabilityGrantDenied"
    }

    fn message(&self) -> String {
        format!(
            "subject `{}` is not granted `{}` on `{}` path `{}`",
            self.subject, self.operation, self.resource, self.path,
        )
    }
}

pub trait RuntimeCapabilityGrantInput {
    type Output;

    fn apply(self, runtime: &mut MechRuntime) -> MResult<Self::Output>;
}

impl RuntimeCapabilityGrantInput for RuntimeCapabilityGrant {
    type Output = ();

    fn apply(self, runtime: &mut MechRuntime) -> MResult<Self::Output> {
        runtime.add_resource_capability_grant(self)
    }
}

impl<T> RuntimeCapabilityGrantInput for Arc<T>
where
    T: Capability + 'static,
{
    type Output = CapabilityId;

    fn apply(self, runtime: &mut MechRuntime) -> MResult<Self::Output> {
        let mut context = runtime.runtime_context()?;
        runtime.grant_capability_with_context(&mut context, self)
    }
}

impl RuntimeCapabilityGrantInput for Arc<dyn Capability> {
    type Output = CapabilityId;

    fn apply(self, runtime: &mut MechRuntime) -> MResult<Self::Output> {
        let mut context = runtime.runtime_context()?;
        runtime.grant_capability_with_context(&mut context, self)
    }
}
