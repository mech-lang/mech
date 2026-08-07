use crate::runtime::MechRuntime;
use crate::{EventId, RuntimeContext, RuntimeEvent, RuntimeEventKind};
use mech_core::MResult;

impl MechRuntime {
    #[cfg(feature = "runtime_bench_probes")]
    #[doc(hidden)]
    pub fn gate_a_emit_representative_event(
        &mut self,
        context: &mut RuntimeContext,
    ) -> MResult<EventId> {
        self.emit_event_to_context(context, RuntimeEventKind::RuntimeTickStarted)
    }

    #[cfg(feature = "runtime_bench_probes")]
    #[doc(hidden)]
    pub fn gate_a_seed_context_event_history(
        &mut self,
        context: &mut RuntimeContext,
        count: usize,
    ) -> MResult<()> {
        self.validate_context_for_runtime(context)?;
        for _ in 0..count {
            let event = self.make_event(RuntimeEventKind::RuntimeTickStarted);
            context.push_event(event);
        }
        Ok(())
    }

    pub fn next_event_sequence(&mut self) -> u64 {
        let sequence = self.event_sequence;
        self.event_sequence = self.event_sequence.saturating_add(1);
        sequence
    }

    pub(in crate::runtime) fn make_event(&mut self, kind: RuntimeEventKind) -> RuntimeEvent {
        RuntimeEvent::new(self.next_event_id(), self.next_event_sequence(), kind)
    }

    pub(in crate::runtime) fn emit_event_to_context(
        &mut self,
        context: &mut RuntimeContext,
        kind: RuntimeEventKind,
    ) -> MResult<EventId> {
        self.validate_context_for_runtime(context)?;

        #[cfg(any(test, feature = "runtime_bench_probes"))]
        crate::runtime::gate_a_probe::record_context_event_snapshot(context.events.len());
        let context_events_before = context.events.clone();
        let event = self.make_event(kind);
        let id = event.id;

        context.push_event(event.clone());
        self.trim_events_to_retention(&mut context.events);
        if let Some(transaction_id) = context.transaction {
            if let Some(transaction) = self.active_transactions.get_mut(&transaction_id) {
                if let Err(error) = transaction.store.stage_event(event) {
                    context.events = context_events_before;
                    return Err(error);
                }
                return Ok(id);
            }
        }

        if let Err(error) = self.store.append_event(event) {
            context.events = context_events_before;
            return Err(error);
        }

        Ok(id)
    }

    pub(in crate::runtime) fn emit_event_immediate_to_context(
        &mut self,
        context: &mut RuntimeContext,
        kind: RuntimeEventKind,
    ) -> MResult<EventId> {
        self.validate_context_for_runtime(context)?;

        #[cfg(any(test, feature = "runtime_bench_probes"))]
        crate::runtime::gate_a_probe::record_context_event_snapshot(context.events.len());
        let context_events_before = context.events.clone();
        let event = self.make_event(kind);
        let id = event.id;

        context.push_event(event.clone());
        self.trim_events_to_retention(&mut context.events);
        if let Err(error) = self.store.append_event(event) {
            context.events = context_events_before;
            return Err(error);
        }

        Ok(id)
    }

    pub(in crate::runtime) fn push_persisted_event_to_context(
        &self,
        context: &mut RuntimeContext,
        event: RuntimeEvent,
    ) -> EventId {
        let id = event.id;
        context.push_event(event);
        self.trim_events_to_retention(&mut context.events);
        id
    }
}
