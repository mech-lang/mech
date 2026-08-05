use mech_core::{ExecutionResourceRequest, MResult};
use mech_runtime::{RunResourceGrantConfig, RuntimeCapabilityOperation, RuntimeResourceKey};

#[cfg(test)]
use mech_runtime::{
    RuntimeResourceReadRequest, RuntimeResourceWriteIntent, RuntimeResourceWritePreflightRequest,
};

use crate::{
    error::{NativeBuildErrorKind, native_build_error},
    plan::{PlannedResourceGrantKey, PlannedResourceOwner, PlannedResourceRequest},
};

use super::ownership::ResolvedResourceOwner;

pub(crate) fn planned_resource_request(
    request: &ExecutionResourceRequest,
) -> PlannedResourceRequest {
    PlannedResourceRequest {
        base_uri: request.base_uri.clone(),
        path: request.path.clone(),
        context_name: request.context_name.clone(),
        operation: request.operation.clone(),
        intent: request.intent,
        delivery: request.delivery,
    }
}

pub(crate) fn planned_resource_grant(
    request: &PlannedResourceRequest,
    owner: &PlannedResourceOwner,
) -> PlannedResourceGrantKey {
    PlannedResourceGrantKey {
        host_instance: owner.host_instance.clone(),
        host_context: owner.host_context.clone(),
        operation: request.operation.clone(),
        path: request.path.clone(),
    }
}

fn execution_resource_grant(
    request: &ExecutionResourceRequest,
    owner: &ResolvedResourceOwner<'_, '_>,
) -> PlannedResourceGrantKey {
    planned_resource_grant(&planned_resource_request(request), &owner.planned_owner())
}

pub(crate) fn runtime_resource_grant_target(grant: &PlannedResourceGrantKey) -> String {
    format!("{}/{}", grant.host_instance, grant.host_context)
}

pub(crate) fn runtime_resource_grant(grant: &PlannedResourceGrantKey) -> RunResourceGrantConfig {
    RunResourceGrantConfig {
        target: runtime_resource_grant_target(grant),
        operations: vec![grant.operation.clone()],
        paths: vec![grant.path.clone()],
    }
}

/// Tests whether one normalized configured grant authorizes the exact
/// structured operation and path selected during trusted owner resolution.
pub(crate) fn grant_covers_resource(
    grant: &RunResourceGrantConfig,
    planned: &PlannedResourceGrantKey,
) -> bool {
    grant.target == runtime_resource_grant_target(planned)
        && grant
            .operations
            .binary_search_by(|operation| operation.as_str().cmp(&planned.operation))
            .is_ok()
        && grant
            .paths
            .iter()
            .any(|scope| resource_path_scope_matches(scope, &planned.path))
}

pub(crate) fn validate_resource_authorization(
    request: &ExecutionResourceRequest,
    owner: &ResolvedResourceOwner<'_, '_>,
    run_grants: &[RunResourceGrantConfig],
) -> MResult<PlannedResourceGrantKey> {
    let context = owner.context;
    let planned_grant = execution_resource_grant(request, owner);
    let runtime_target = runtime_resource_grant_target(&planned_grant);

    if !context
        .operations
        .iter()
        .any(|operation| operation == &request.operation)
    {
        return Err(native_build_error(
            NativeBuildErrorKind::NativeContextOperationInvalid {
                target: runtime_target,
                operation: request.operation.clone(),
            },
            None,
        ));
    }

    let key = RuntimeResourceKey::new(&request.base_uri, &request.path).map_err(|_| {
        native_build_error(
            NativeBuildErrorKind::NativeResourcePathInvalid {
                target: runtime_target.clone(),
                path: request.path.clone(),
            },
            None,
        )
    })?;
    if key.base_uri != request.base_uri || key.path != request.path {
        return Err(native_build_error(
            NativeBuildErrorKind::NativeResourcePathInvalid {
                target: runtime_target.clone(),
                path: request.path.clone(),
            },
            None,
        ));
    }

    RuntimeCapabilityOperation::from_name(request.operation.clone()).map_err(|_| {
        native_build_error(
            NativeBuildErrorKind::NativeResourcePathInvalid {
                target: runtime_target.clone(),
                path: request.path.clone(),
            },
            None,
        )
    })?;

    if !run_grants
        .iter()
        .any(|grant| grant_covers_resource(grant, &planned_grant))
    {
        return Err(native_build_error(
            NativeBuildErrorKind::NativeRunGrantMissing {
                target: runtime_target,
                operation: request.operation.clone(),
                path: request.path.clone(),
            },
            None,
        ));
    }
    Ok(planned_grant)
}

#[cfg(test)]
pub(crate) fn validate_resource_requirement(
    request: &ExecutionResourceRequest,
    owner: &ResolvedResourceOwner<'_, '_>,
    run_grants: &[RunResourceGrantConfig],
) -> MResult<PlannedResourceGrantKey> {
    let grant = validate_resource_authorization(request, owner, run_grants)?;
    let key = RuntimeResourceKey::new(&request.base_uri, &request.path)
        .expect("resource authorization validated the canonical resource key");

    let provider_result = match request.intent {
        mech_core::ResourceIntent::Read => owner
            .provider
            .plan_read(RuntimeResourceReadRequest {
                base_uri: key.base_uri.clone(),
                path: key.path.clone(),
                context_name: request.context_name.clone(),
            })
            .map(|_| ()),
        mech_core::ResourceIntent::Assign | mech_core::ResourceIntent::Send => {
            let intent = match request.intent {
                mech_core::ResourceIntent::Assign => RuntimeResourceWriteIntent::Assign,
                mech_core::ResourceIntent::Send => RuntimeResourceWriteIntent::Send,
                mech_core::ResourceIntent::Read => unreachable!(),
            };
            owner
                .provider
                .preflight_write(RuntimeResourceWritePreflightRequest {
                    base_uri: key.base_uri.clone(),
                    path: key.path.clone(),
                    context_name: request.context_name.clone(),
                    operation: RuntimeCapabilityOperation::from_name(request.operation.clone())
                        .expect("resource authorization validated the capability operation"),
                    intent,
                })
        }
    };
    provider_result.map_err(|_| {
        native_build_error(
            NativeBuildErrorKind::NativeResourcePathInvalid {
                target: runtime_resource_grant_target(&grant),
                path: request.path.clone(),
            },
            None,
        )
    })?;
    Ok(grant)
}

fn resource_path_scope_matches(scope: &str, path: &str) -> bool {
    if scope == "*" {
        return true;
    }
    let Some(prefix) = scope.strip_suffix("/*") else {
        return scope == path;
    };
    path == prefix
        || path
            .strip_prefix(prefix)
            .is_some_and(|remainder| remainder.starts_with('/'))
}
