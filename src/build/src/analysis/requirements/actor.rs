use mech_core::{LegacyValue, MResult, MechError};
use mech_runtime::ActorHostPlanningState;

use crate::{
    NativeActorBootstrap, NativeRuntimeConfig,
    error::{NativeBuildErrorKind, native_build_error},
};

pub(super) struct NativeActorPlanning {
    bootstrap: Option<NativeActorBootstrap>,
    state: Option<ActorHostPlanningState>,
    required: bool,
}

impl NativeActorPlanning {
    pub(super) fn new(config: Option<&NativeRuntimeConfig>) -> Self {
        let bootstrap = config.and_then(|config| config.actor_bootstrap.clone());
        let state = bootstrap.as_ref().map(|bootstrap| {
            ActorHostPlanningState::new(
                &bootstrap.subject,
                bootstrap.message_kind.clone(),
                bootstrap.message_payload.clone(),
                bootstrap.initial_state.clone(),
            )
        });
        Self {
            bootstrap,
            state,
            required: false,
        }
    }

    pub(super) fn plan(
        &mut self,
        instruction: u32,
        name: &str,
        arguments: &[LegacyValue],
    ) -> MResult<LegacyValue> {
        self.required = true;
        self.state
            .as_mut()
            .ok_or_else(|| {
                native_build_error(NativeBuildErrorKind::NativeActorBootstrapMissing, None)
            })?
            .plan(name, arguments)
            .map_err(|error| actor_planning_error(instruction, name, error))
    }

    pub(super) fn finish(&self, live: bool) -> MResult<Option<NativeActorBootstrap>> {
        match (self.required, self.bootstrap.is_some()) {
            (true, false) => {
                return Err(native_build_error(
                    NativeBuildErrorKind::NativeActorBootstrapMissing,
                    None,
                ));
            }
            (false, true) => {
                return Err(native_build_error(
                    NativeBuildErrorKind::NativeActorBootstrapUnused,
                    None,
                ));
            }
            _ => {}
        }
        if self.required && live {
            return Err(native_build_error(
                NativeBuildErrorKind::NativeActorLiveApplicationUnsupported,
                None,
            ));
        }
        Ok(self.bootstrap.clone())
    }
}

fn actor_planning_error(instruction: u32, name: &str, error: MechError) -> MechError {
    super::external::application_instruction_error(
        instruction,
        format!(
            "host function `{name}` rejected its arguments: {}",
            error.display_message(),
        ),
    )
    .with_source(error)
}
