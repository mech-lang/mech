use crate::RuntimeContext;
use crate::event::RuntimeEventKind;
use crate::runtime::MechRuntime;
use mech_core::MechError;
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
use std::time::Instant;
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use web_time::Instant;

impl MechRuntime {
    pub(super) fn emit_program_failure_audit(
        &mut self,
        context: &mut RuntimeContext,
        error: &MechError,
        profile_started: Option<Instant>,
    ) {
        let _ = self.emit_event_to_context(
            context,
            RuntimeEventKind::ProgramFailed {
                task_id: context.task,
                message: format!("{:?}", error),
            },
        );
        if let Some(started) = profile_started {
            let _ = self.emit_event_to_context(
                context,
                RuntimeEventKind::ProgramProfiled {
                    task_id: context.task,
                    duration_ns: started.elapsed().as_nanos(),
                },
            );
        }
    }
}
