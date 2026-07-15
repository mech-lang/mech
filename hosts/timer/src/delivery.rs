use std::collections::VecDeque;

use mech_core::MResult;
use mech_runtime::RuntimeIngress;

use crate::{SharedTimerSnapshot, TimerSnapshot};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TimerSubmitState {
    Drained,
    Full,
    Closed,
}

pub(crate) fn submit_pending_timer_snapshots(
    instance: &str,
    ingress: Option<&RuntimeIngress>,
    snapshot: &SharedTimerSnapshot,
    pending: &mut VecDeque<TimerSnapshot>,
) -> MResult<(usize, TimerSubmitState)> {
    let mut submitted = 0;
    while let Some(next) = pending.front().copied() {
        let Some(ingress) = ingress else {
            *snapshot.lock().map_err(|_| {
                crate::timer_error("TimerDelivery", "timer snapshot lock is poisoned")
            })? = next;
            pending.pop_front();
            submitted += 1;
            continue;
        };
        match ingress.submit(next.into_host_input(instance)?) {
            Ok(()) => {
                *snapshot.lock().map_err(|_| {
                    crate::timer_error("TimerDelivery", "timer snapshot lock is poisoned")
                })? = next;
                pending.pop_front();
                submitted += 1;
            }
            Err(err) if err.kind_name() == "RuntimeIngressFull" => {
                return Ok((submitted, TimerSubmitState::Full));
            }
            Err(err) if err.kind_name() == "RuntimeIngressClosed" => {
                return Ok((submitted, TimerSubmitState::Closed));
            }
            Err(err) => return Err(err),
        }
    }
    Ok((submitted, TimerSubmitState::Drained))
}
