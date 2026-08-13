use crate::RuntimeContext;
use crate::runtime::{MechRuntime, transaction::PreparedRuntimeHostInput};
use mech_core::MResult;
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
use std::time::Instant;
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use web_time::Instant;

impl MechRuntime {
    pub fn apply_host_input(
        &mut self,
        input: crate::RuntimeHostInput,
    ) -> MResult<crate::RuntimeHostInputOutcome> {
        self.ensure_runtime_mutation_allowed("apply_host_input")?;
        self.reject_program_operation_reentrancy("apply_host_input")?;
        let prepared = self.prepare_runtime_host_input(&input)?;
        if prepared.binding_count == 0 {
            return Ok(crate::RuntimeHostInputOutcome {
                update_count: prepared.update_count,
                ignored_update_count: prepared.ignored_update_count,
                binding_count: 0,
                turn: None,
                #[cfg(feature = "resident-routing")]
                resident_turn: None,
            });
        }

        let mut context = self.live_turn_context()?;
        self.validate_context_for_runtime(&context)?;
        self.validate_live_turn_context(&context)?;
        self.apply_prepared_host_input_with_context(&mut context, prepared)
    }

    pub fn apply_host_input_with_context(
        &mut self,
        context: &mut RuntimeContext,
        input: crate::RuntimeHostInput,
    ) -> MResult<crate::RuntimeHostInputOutcome> {
        self.ensure_runtime_mutation_allowed("apply_host_input_with_context")?;
        self.validate_context_for_runtime(context)?;
        self.reject_program_operation_reentrancy("apply_host_input_with_context")?;
        self.validate_live_turn_context(context)?;
        let prepared = self.prepare_runtime_host_input(&input)?;

        if prepared.binding_count == 0 {
            return Ok(crate::RuntimeHostInputOutcome {
                update_count: prepared.update_count,
                ignored_update_count: prepared.ignored_update_count,
                binding_count: 0,
                turn: None,
                #[cfg(feature = "resident-routing")]
                resident_turn: None,
            });
        }

        self.apply_prepared_host_input_with_context(context, prepared)
    }

    fn apply_prepared_host_input_with_context(
        &mut self,
        context: &mut RuntimeContext,
        prepared: PreparedRuntimeHostInput,
    ) -> MResult<crate::RuntimeHostInputOutcome> {
        let turn_started = Instant::now();
        let turn = self.with_atomic_reactive_turn(
            context,
            "apply_host_input_with_context",
            |program, services, finalize| {
                program.update_cells_and_advance_turn_coordinated(
                    &prepared.updates,
                    services,
                    |turn| finalize(turn),
                )
            },
            move |runtime, _context, _turn| runtime.enforce_turn_duration(turn_started),
        )?;
        Ok(crate::RuntimeHostInputOutcome {
            update_count: prepared.update_count,
            ignored_update_count: prepared.ignored_update_count,
            binding_count: prepared.binding_count,
            turn: Some(turn),
            #[cfg(feature = "resident-routing")]
            resident_turn: None,
        })
    }
}
