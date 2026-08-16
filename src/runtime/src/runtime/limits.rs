//! Runtime budget, source, duration, and event-retention limits.

use super::MechRuntime;
use crate::{ResourceBudget, ResourceBudgetExceededError, RuntimeContext};
use mech_core::{MResult, MechError, MechSourceCode};
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
use std::time::Instant;
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use web_time::Instant;

impl MechRuntime {
    pub fn default_budget(&self) -> ResourceBudget {
        let mut budget = ResourceBudget::default();

        if let Some(max_steps) = self.config.limits.max_steps_per_turn {
            budget = budget.with_max_steps(max_steps);
        }

        if let Some(max_bytes) = self.config.limits.max_memory_bytes {
            budget = budget.with_max_bytes(max_bytes);
        }

        budget
    }

    fn known_source_bytes(source: &MechSourceCode) -> MResult<Option<u64>> {
        match source {
            MechSourceCode::String(source) | MechSourceCode::Html(source) => Ok(Some(
                u64::try_from(source.as_bytes().len()).map_err(|_| {
                    MechError::new(
                        ResourceBudgetExceededError {
                            resource: "source_bytes",
                            used: u64::MAX,
                            requested: 1,
                            max: None,
                        },
                        None,
                    )
                })?,
            )),
            MechSourceCode::ByteCode(bytes) => {
                Ok(Some(u64::try_from(bytes.len()).map_err(|_| {
                    MechError::new(
                        ResourceBudgetExceededError {
                            resource: "source_bytes",
                            used: u64::MAX,
                            requested: 1,
                            max: None,
                        },
                        None,
                    )
                })?))
            }
            MechSourceCode::Image(_, bytes) => {
                Ok(Some(u64::try_from(bytes.len()).map_err(|_| {
                    MechError::new(
                        ResourceBudgetExceededError {
                            resource: "source_bytes",
                            used: u64::MAX,
                            requested: 1,
                            max: None,
                        },
                        None,
                    )
                })?))
            }
            MechSourceCode::Program(sources) => {
                let mut total = 0u64;
                for source in sources {
                    let Some(bytes) = Self::known_source_bytes(source)? else {
                        return Ok(None);
                    };
                    total = total.checked_add(bytes).ok_or_else(|| {
                        MechError::new(
                            ResourceBudgetExceededError {
                                resource: "source_bytes",
                                used: total,
                                requested: bytes,
                                max: None,
                            },
                            None,
                        )
                    })?;
                }
                Ok(Some(total))
            }
            MechSourceCode::Tree(_) => Ok(None),
        }
    }

    pub(in crate::runtime) fn enforce_source_limits(
        &self,
        context: &mut RuntimeContext,
        source: &MechSourceCode,
    ) -> MResult<()> {
        let Some(source_bytes) = Self::known_source_bytes(source)? else {
            return Ok(());
        };

        self.enforce_source_byte_count(context, source_bytes)
    }

    pub(in crate::runtime) fn enforce_source_byte_count(
        &self,
        context: &mut RuntimeContext,
        source_bytes: u64,
    ) -> MResult<()> {
        self.enforce_source_byte_limit(source_bytes)?;
        context.charge_bytes(source_bytes)
    }

    /// Checks the configured per-source ceiling without charging a context.
    /// Resident planning runs before an execution context exists; the selected
    /// executor remains responsible for its own resource accounting.
    pub(in crate::runtime) fn enforce_source_byte_limit(&self, source_bytes: u64) -> MResult<()> {
        if let Some(max) = self.config.limits.max_source_bytes {
            if source_bytes > max {
                return Err(MechError::new(
                    ResourceBudgetExceededError {
                        resource: "source_bytes",
                        used: 0,
                        requested: source_bytes,
                        max: Some(max),
                    },
                    None,
                ));
            }
        }
        Ok(())
    }

    pub(in crate::runtime) fn apply_context_event_retention(&self, context: &mut RuntimeContext) {
        let Some(max_events) = self.config.limits.max_in_memory_events else {
            return;
        };
        let max_events = usize::try_from(max_events).unwrap_or(usize::MAX);
        context.events.retain_last(max_events);
    }
}

pub(in crate::runtime) fn enforce_turn_duration_limit(
    max: Option<u64>,
    started: Instant,
) -> MResult<()> {
    let Some(max) = max else {
        return Ok(());
    };
    let requested = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    if requested > max {
        return Err(MechError::new(
            ResourceBudgetExceededError {
                resource: "turn_duration_ms",
                used: 0,
                requested,
                max: Some(max),
            },
            None,
        ));
    }
    Ok(())
}
