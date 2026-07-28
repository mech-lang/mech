use std::sync::Arc;

use super::super::MechRuntime;
use crate::{
    BasicCapability, BasicConstraints, BasicOperation, BasicResource, BasicSubject, CapabilityId,
    ResourcePathCapability, ResourcePathScope, RuntimeCapabilityOperation, SharedCapabilityKernel,
};

pub(crate) fn grant_read(runtime: &mut MechRuntime, resource: &str, path: &str) {
    let subject = runtime.runtime_context().unwrap().subject;
    grant_resource(
        runtime,
        &subject,
        resource,
        RuntimeCapabilityOperation::Read,
        &[path],
    );
}

pub(crate) fn grant_write(runtime: &mut MechRuntime, resource: &str, path: &str) {
    let subject = runtime.runtime_context().unwrap().subject;
    grant_resource(
        runtime,
        &subject,
        resource,
        RuntimeCapabilityOperation::Write,
        &[path],
    );
}

pub(crate) fn grant_read_to(
    runtime: &mut MechRuntime,
    subject: &str,
    resource: &str,
    path: &str,
) -> CapabilityId {
    grant_resource(
        runtime,
        subject,
        resource,
        RuntimeCapabilityOperation::Read,
        &[path],
    )
}

pub(crate) fn grant_write_to(
    runtime: &mut MechRuntime,
    subject: &str,
    resource: &str,
    path: &str,
) -> CapabilityId {
    grant_resource(
        runtime,
        subject,
        resource,
        RuntimeCapabilityOperation::Write,
        &[path],
    )
}

pub(crate) fn grant_resource(
    runtime: &mut MechRuntime,
    subject: &str,
    resource: &str,
    operation: RuntimeCapabilityOperation,
    paths: &[&str],
) -> CapabilityId {
    let scopes = paths.iter().map(|path| {
        if *path == "*" {
            ResourcePathScope::Wildcard
        } else if let Some(prefix) = path.strip_suffix("/*") {
            ResourcePathScope::Prefix(prefix.to_string())
        } else {
            ResourcePathScope::Exact((*path).to_string())
        }
    });
    let capability = ResourcePathCapability::new(
        runtime.next_capability_id(),
        subject,
        resource,
        [operation.name()],
        scopes,
    )
    .unwrap();
    runtime.grant_capability(Arc::new(capability)).unwrap()
}

pub(crate) fn grant_host_call(
    runtime: &mut MechRuntime,
    id: CapabilityId,
    name: &str,
) -> CapabilityId {
    grant_host_call_with_constraints(runtime, id, name, None)
}

pub(crate) fn grant_host_call_with_limit(
    runtime: &mut MechRuntime,
    id: CapabilityId,
    name: &str,
    max_uses: u64,
) -> CapabilityId {
    grant_host_call_with_constraints(runtime, id, name, Some(max_uses))
}

fn grant_host_call_with_constraints(
    runtime: &mut MechRuntime,
    id: CapabilityId,
    name: &str,
    max_uses: Option<u64>,
) -> CapabilityId {
    let subject = runtime.runtime_context().unwrap().subject;
    let resource = if name.starts_with("host:") {
        name.to_string()
    } else {
        format!("host:{name}")
    };
    let mut capability = BasicCapability::new(
        id,
        &BasicSubject::new(subject),
        &BasicResource::new(resource),
        [BasicOperation::new("call")],
    );
    if let Some(max_uses) = max_uses {
        capability =
            capability.with_constraints(BasicConstraints::default().with_max_uses(max_uses));
    }
    runtime.grant_capability(Arc::new(capability)).unwrap()
}

#[derive(Clone, Debug)]
pub(crate) struct CapabilityUseProbe {
    kernel: SharedCapabilityKernel,
    capability: CapabilityId,
}

impl CapabilityUseProbe {
    pub(crate) fn new(kernel: SharedCapabilityKernel, capability: CapabilityId) -> Self {
        Self { kernel, capability }
    }

    pub(crate) fn committed_uses(&self) -> u64 {
        self.kernel.successful_uses_for_test(self.capability)
    }
}
