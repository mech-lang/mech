#[derive(Debug)]
pub struct ResidentHostDrainOutcome {
    pub dequeued_packets: usize,
    pub matched_packets: usize,
    pub coalesced_packets: usize,
    pub ignored_packets: usize,
    pub turn: Option<crate::ResidentExternalTurnOutcome>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ResidentProductionProbe {
    pub resident_turns: u64,
    pub resident_rejections: u64,
    pub legacy_turns: u64,
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

        let super::ActiveProgramExecution::ResidentExternal(execution) = &mut self.active_program
        else {
            return Err(super::invalid_active_program(
                "resident host draining requires an active external resident program",
            ));
        };
        let matched_packets = packets
            .iter()
            .filter(|packet| {
                packet.updates.iter().any(|update| {
                    execution
                        .trigger_sources
                        .iter()
                        .any(|source| source == &update.source)
                })
            })
            .count();
        let ignored_packets = packets.len().saturating_sub(matched_packets);
        let coalesced_packets = matched_packets.saturating_sub(1);
        self.program_execution_info.ignored_host_packets = self
            .program_execution_info
            .ignored_host_packets
            .saturating_add(ignored_packets as u64);
        self.program_execution_info.coalesced_host_packets = self
            .program_execution_info
            .coalesced_host_packets
            .saturating_add(coalesced_packets as u64);

        let turn = if matched_packets == 0 {
            None
        } else {
            let before = execution.coordinator.structural_probe();
            let turn = execution.coordinator.execute_turn(&self.resources)?;
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
            match &turn {
                crate::ResidentExternalTurnOutcome::Accepted {
                    delivery_failures, ..
                } => {
                    self.program_execution_info.resident_accepted_turns = self
                        .program_execution_info
                        .resident_accepted_turns
                        .saturating_add(1);
                    self.resident_production_probe.resident_turns = self
                        .resident_production_probe
                        .resident_turns
                        .saturating_add(1);
                    self.resident_production_probe.scene_effects_prepared = self
                        .resident_production_probe
                        .scene_effects_prepared
                        .saturating_add(self.program_execution_info.effect_count as u64);
                    self.resident_production_probe.scene_effects_delivered = self
                        .resident_production_probe
                        .scene_effects_delivered
                        .saturating_add(
                            self.program_execution_info
                                .effect_count
                                .saturating_sub(delivery_failures.len())
                                as u64,
                        );
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

        Ok(ResidentHostDrainOutcome {
            dequeued_packets: packets.len(),
            matched_packets,
            coalesced_packets,
            ignored_packets,
            turn,
        })
    }
}
