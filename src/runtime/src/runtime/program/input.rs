use mech_core::{MechError, MechErrorKind};
use std::collections::BTreeMap;
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
use std::time::Instant;
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use web_time::Instant;

#[derive(Debug)]
pub struct ResidentHostDrainOutcome {
    pub dequeued_packets: usize,
    pub matched_packets: usize,
    pub coalesced_packets: usize,
    pub ignored_packets: usize,
    pub turn: Option<crate::ResidentExternalTurnOutcome>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResidentHostTurnFailed {
    pub turn: crate::TurnId,
    pub status: &'static str,
    pub failure_count: usize,
    pub phase: Option<crate::TurnFailurePhase>,
    pub failures: Vec<String>,
}

impl ResidentHostTurnFailed {
    /// A rejected candidate never published state and may be followed by a
    /// later turn. Outcomes that advanced or indeterminately published state
    /// require host intervention instead.
    pub fn is_recoverable(&self) -> bool {
        self.status == "rejected"
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResidentHostTurnCause {
    pub phase: crate::TurnFailurePhase,
    pub kind: String,
    pub detail: String,
}

impl MechErrorKind for ResidentHostTurnCause {
    fn name(&self) -> &str {
        &self.kind
    }

    fn message(&self) -> String {
        format!(
            "resident turn failed during {:?}: {}",
            self.phase, self.detail
        )
    }
}

impl MechErrorKind for ResidentHostTurnFailed {
    fn name(&self) -> &str {
        "ResidentHostTurnFailed"
    }

    fn message(&self) -> String {
        let details = if self.failures.is_empty() {
            String::new()
        } else {
            format!(": {}", self.failures.join("; "))
        };
        format!(
            "resident host turn {} completed as {}{} with {} reported failures{}",
            self.turn.get(),
            self.status,
            self.phase
                .map(|phase| format!(" during {phase:?}"))
                .unwrap_or_default(),
            self.failure_count,
            details,
        )
    }
}

pub(crate) fn resident_host_turn_error(
    outcome: &crate::ResidentExternalTurnOutcome,
) -> Option<MechError> {
    let (failure, source) = match outcome {
        crate::ResidentExternalTurnOutcome::Accepted {
            delivery_failures, ..
        } if delivery_failures.is_empty() => return None,
        crate::ResidentExternalTurnOutcome::Accepted {
            turn,
            delivery_failures,
            ..
        } => (
            ResidentHostTurnFailed {
                turn: *turn,
                status: "accepted-with-delivery-failures",
                failure_count: delivery_failures.len(),
                phase: None,
                failures: delivery_failures
                    .iter()
                    .map(|failure| failure.message.clone())
                    .collect(),
            },
            None,
        ),
        crate::ResidentExternalTurnOutcome::Rejected {
            turn,
            phase,
            failure,
            ..
        } => (
            ResidentHostTurnFailed {
                turn: *turn,
                status: "rejected",
                failure_count: 1,
                phase: Some(*phase),
                failures: vec![format!("{}: {}", failure.kind, failure.message)],
            },
            Some(MechError::new(
                ResidentHostTurnCause {
                    phase: failure.phase,
                    kind: failure.kind.clone(),
                    detail: failure.message.clone(),
                },
                None,
            )),
        ),
        crate::ResidentExternalTurnOutcome::PublishedIndeterminate { turn, failures, .. } => (
            ResidentHostTurnFailed {
                turn: *turn,
                status: "published-indeterminate",
                failure_count: failures.len(),
                phase: None,
                failures: failures
                    .iter()
                    .map(|failure| failure.message.clone())
                    .collect(),
            },
            None,
        ),
    };
    let error = MechError::new(failure, None);
    Some(match source {
        Some(source) => error.with_source(source),
        None => error,
    })
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ResidentProductionProbe {
    pub resident_turns: u64,
    pub resident_rejections: u64,
    pub scene_effects_prepared: u64,
    pub scene_effects_delivered: u64,
    pub scene_effects_before_publication: u64,
    pub scene_effects_for_rejected_turns: u64,
}

impl crate::runtime::MechRuntime {
    pub fn resident_production_probe(&self) -> ResidentProductionProbe {
        self.resident_production_probe
    }

    pub(crate) fn drain_resident_host_inputs(
        &mut self,
        max_inputs: usize,
    ) -> mech_core::MResult<ResidentHostDrainOutcome> {
        self.revalidate_active_resident_grants()?;
        let trigger_sources = if let super::ActiveProgramExecution::ResidentExternal(execution) =
            &self.active_program
        {
            execution.trigger_sources.clone()
        } else {
            return Err(super::invalid_active_program(
                "resident host draining requires an active external resident program",
            ));
        };
        let mut packets = Vec::new();
        for _ in 0..max_inputs {
            let packet = {
                let mut guard = self.host_input_queue.lock().map_err(|_| {
                    crate::input::input_error(
                        "RuntimeIngressUnavailable",
                        "host input queue lock is poisoned",
                    )
                })?;
                guard.queue.pop_front()
            };
            let Some(packet) = packet else { break };
            packets.push(packet);
        }

        let mut matched_packets = 0;
        let mut latest_updates = BTreeMap::new();
        for packet in &packets {
            let mut matched = false;
            for update in &packet.updates {
                if trigger_sources
                    .iter()
                    .any(|source| source == &update.source)
                {
                    matched = true;
                    latest_updates.insert(update.source.clone(), update.value.clone());
                }
            }
            matched_packets += usize::from(matched);
        }
        let latest_updates = latest_updates
            .into_iter()
            .map(|(source, value)| crate::RuntimeHostInputUpdate { source, value })
            .collect::<Vec<_>>();
        let ignored_packets = packets.len().saturating_sub(matched_packets);
        let coalesced_packets = matched_packets.saturating_sub(1);
        let turn = if matched_packets == 0 {
            None
        } else {
            let max_turn_duration_ms = self.config.limits.max_turn_duration_ms;
            let turn_started = Instant::now();
            let admission = {
                let super::ActiveProgramExecution::ResidentExternal(execution) =
                    &mut self.active_program
                else {
                    unreachable!("resident route checked before ingress dequeue")
                };
                match execution.coordinator.admit_host_turn(&latest_updates) {
                    Ok(admission) => admission,
                    Err(error) => {
                        self.restore_resident_host_packets(packets)?;
                        return Err(error);
                    }
                }
            };
            let super::ActiveProgramExecution::ResidentExternal(execution) =
                &mut self.active_program
            else {
                unreachable!("resident route checked before ingress dequeue")
            };
            let before = execution.coordinator.structural_probe();
            let turn = execution.coordinator.execute_admitted_host_turn(
                &latest_updates,
                admission,
                move || {
                    super::super::limits::enforce_turn_duration_limit(
                        max_turn_duration_ms,
                        turn_started,
                    )
                },
            )?;
            let after = execution.coordinator.structural_probe();
            self.resident_production_probe
                .scene_effects_before_publication = self
                .resident_production_probe
                .scene_effects_before_publication
                .saturating_add(
                    after
                        .effects_delivered_before_publication
                        .saturating_sub(before.effects_delivered_before_publication)
                        as u64,
                );
            self.resident_production_probe
                .scene_effects_for_rejected_turns = self
                .resident_production_probe
                .scene_effects_for_rejected_turns
                .saturating_add(
                    after
                        .effects_delivered_for_rejected_turns
                        .saturating_sub(before.effects_delivered_for_rejected_turns)
                        as u64,
                );
            self.resident_production_probe.scene_effects_prepared = self
                .resident_production_probe
                .scene_effects_prepared
                .saturating_add(
                    after
                        .scene_effects_prepared
                        .saturating_sub(before.scene_effects_prepared) as u64,
                );
            self.resident_production_probe.scene_effects_delivered = self
                .resident_production_probe
                .scene_effects_delivered
                .saturating_add(
                    after
                        .scene_effects_delivered
                        .saturating_sub(before.scene_effects_delivered) as u64,
                );
            match &turn {
                crate::ResidentExternalTurnOutcome::Accepted { .. } => {
                    self.program_execution_info.resident_accepted_turns = self
                        .program_execution_info
                        .resident_accepted_turns
                        .saturating_add(1);
                    self.resident_production_probe.resident_turns = self
                        .resident_production_probe
                        .resident_turns
                        .saturating_add(1);
                }
                crate::ResidentExternalTurnOutcome::PublishedIndeterminate { .. } => {
                    self.program_execution_info.resident_accepted_turns = self
                        .program_execution_info
                        .resident_accepted_turns
                        .saturating_add(1);
                    self.resident_production_probe.resident_turns = self
                        .resident_production_probe
                        .resident_turns
                        .saturating_add(1);
                }
                crate::ResidentExternalTurnOutcome::Rejected { .. } => {
                    self.program_execution_info.resident_rejected_turns = self
                        .program_execution_info
                        .resident_rejected_turns
                        .saturating_add(1);
                    self.resident_production_probe.resident_rejections = self
                        .resident_production_probe
                        .resident_rejections
                        .saturating_add(1);
                }
            }
            Some(turn)
        };

        self.program_execution_info.ignored_host_packets = self
            .program_execution_info
            .ignored_host_packets
            .saturating_add(ignored_packets as u64);
        self.program_execution_info.coalesced_host_packets = self
            .program_execution_info
            .coalesced_host_packets
            .saturating_add(coalesced_packets as u64);

        Ok(ResidentHostDrainOutcome {
            dequeued_packets: packets.len(),
            matched_packets,
            coalesced_packets,
            ignored_packets,
            turn,
        })
    }

    fn restore_resident_host_packets(
        &self,
        packets: Vec<crate::RuntimeHostInput>,
    ) -> mech_core::MResult<()> {
        let mut guard = self.host_input_queue.lock().map_err(|_| {
            crate::input::input_error(
                "RuntimeIngressUnavailable",
                "host input queue lock is poisoned while restoring resident admission",
            )
        })?;
        for packet in packets.into_iter().rev() {
            guard.queue.push_front(packet);
        }
        Ok(())
    }
}
