use mech_core::MResult;
use mech_runtime::validate_run_resource_grant;

use crate::{
    NativeActorBootstrap, NativeRuntimeConfig,
    error::{NativeBuildErrorKind, native_build_error},
};

/// Clones and deterministically normalizes the runtime configuration fields
/// that are permitted to influence a native build plan.
pub fn normalize_native_runtime_config(
    config: &NativeRuntimeConfig,
) -> MResult<NativeRuntimeConfig> {
    config.runtime.validate()?;

    let mut hosts = config.hosts.clone();
    hosts.sort_by(|lhs, rhs| (&lhs.name, &lhs.provider).cmp(&(&rhs.name, &rhs.provider)));
    if let Some(duplicate) = hosts.windows(2).find(|pair| pair[0].name == pair[1].name) {
        return Err(native_build_error(
            NativeBuildErrorKind::NativeRuntimeConfigDuplicateHostInstance {
                instance: duplicate[0].name.clone(),
            },
            None,
        ));
    }

    let mut run_grants = config.run_grants.clone();
    for grant in &mut run_grants {
        grant.operations.sort();
        grant.operations.dedup();
        grant.paths.sort();
        grant.paths.dedup();
        validate_run_resource_grant(grant)?;
    }
    run_grants.sort_by(|lhs, rhs| {
        (&lhs.target, &lhs.operations, &lhs.paths).cmp(&(&rhs.target, &rhs.operations, &rhs.paths))
    });
    run_grants.dedup();

    let actor_bootstrap = config
        .actor_bootstrap
        .as_ref()
        .map(|bootstrap| {
            let subject = bootstrap.subject.trim().to_owned();
            if subject.is_empty() {
                return Err(native_build_error(
                    NativeBuildErrorKind::NativeRuntimeConfigUnsupported {
                        reason: "actor bootstrap subject must be non-empty".to_owned(),
                    },
                    None,
                ));
            }
            let message_kind = bootstrap.message_kind.trim().to_owned();
            if message_kind.is_empty() {
                return Err(native_build_error(
                    NativeBuildErrorKind::NativeRuntimeConfigUnsupported {
                        reason: "actor bootstrap message kind must be non-empty".to_owned(),
                    },
                    None,
                ));
            }
            Ok(NativeActorBootstrap {
                subject,
                message_kind,
                message_payload: bootstrap.message_payload.clone(),
                initial_state: bootstrap.initial_state.clone(),
            })
        })
        .transpose()?;

    Ok(NativeRuntimeConfig {
        runtime: config.runtime.clone(),
        hosts,
        run_grants,
        actor_bootstrap,
    })
}
