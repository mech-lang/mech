use std::collections::VecDeque;

use mech_core::MResult;
use mech_runtime::{RuntimeHostInput, RuntimeIngress};

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
    submit_pending_with(instance, snapshot, pending, |packet| match ingress {
        Some(ingress) => ingress.submit(packet),
        None => Ok(()),
    })
}

pub(crate) fn submit_pending_with<F>(
    instance: &str,
    snapshot: &SharedTimerSnapshot,
    pending: &mut VecDeque<TimerSnapshot>,
    mut submit: F,
) -> MResult<(usize, TimerSubmitState)>
where
    F: FnMut(RuntimeHostInput) -> MResult<()>,
{
    let mut submitted = 0;
    while let Some(next) = pending.front().copied() {
        match submit(next.into_host_input(instance)?) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use mech_runtime::RuntimeHostInputValue;
    use crate::{new_shared_snapshot, timer_error};

    fn pending_ticks(ticks: &[u64]) -> VecDeque<TimerSnapshot> {
        ticks.iter().map(|tick| TimerSnapshot::new(*tick, 100, 0)).collect()
    }

    fn packet_tick(packet: &RuntimeHostInput) -> u64 {
        packet
            .updates
            .iter()
            .find(|update| update.source.path() == "tick")
            .and_then(|update| match update.value {
                RuntimeHostInputValue::F64(value) => Some(value as u64),
                _ => None,
            })
            .unwrap()
    }

    #[test]
    fn submit_pending_with_accepts_packets_in_order() {
        let snapshot = new_shared_snapshot(TimerSnapshot::new(0, 100, 0));
        let mut pending = pending_ticks(&[1, 2, 3]);
        let mut observed = Vec::new();
        let (submitted, state) = submit_pending_with("physics", &snapshot, &mut pending, |packet| {
            observed.push(packet_tick(&packet));
            Ok(())
        }).unwrap();
        assert_eq!(submitted, 3);
        assert_eq!(state, TimerSubmitState::Drained);
        assert_eq!(observed, vec![1, 2, 3]);
        assert!(pending.is_empty());
        assert_eq!(snapshot.lock().unwrap().tick, 3);
    }

    #[test]
    fn submit_pending_with_retains_front_on_full() {
        let snapshot = new_shared_snapshot(TimerSnapshot::new(0, 100, 0));
        let mut pending = pending_ticks(&[1, 2]);
        let mut calls = 0;
        let (submitted, state) = submit_pending_with("physics", &snapshot, &mut pending, |_| {
            calls += 1;
            if calls == 1 { Ok(()) } else { Err(timer_error("RuntimeIngressFull", "full")) }
        }).unwrap();
        assert_eq!(submitted, 1);
        assert_eq!(state, TimerSubmitState::Full);
        assert_eq!(pending.front().unwrap().tick, 2);
        assert_eq!(snapshot.lock().unwrap().tick, 1);
    }

    #[test]
    fn submit_pending_with_reports_closed() {
        let snapshot = new_shared_snapshot(TimerSnapshot::new(0, 100, 0));
        let mut pending = pending_ticks(&[1]);
        let (submitted, state) = submit_pending_with("physics", &snapshot, &mut pending, |_| {
            Err(timer_error("RuntimeIngressClosed", "closed"))
        }).unwrap();
        assert_eq!(submitted, 0);
        assert_eq!(state, TimerSubmitState::Closed);
        assert_eq!(pending.front().unwrap().tick, 1);
        assert_eq!(snapshot.lock().unwrap().tick, 0);
    }

    #[test]
    fn submit_pending_with_propagates_unexpected_error() {
        let snapshot = new_shared_snapshot(TimerSnapshot::new(0, 100, 0));
        let mut pending = pending_ticks(&[1]);
        let error = submit_pending_with("physics", &snapshot, &mut pending, |_| {
            Err(timer_error("InjectedTimerFailure", "boom"))
        }).unwrap_err();
        assert_eq!(error.kind_name(), "InjectedTimerFailure");
        assert_eq!(pending.front().unwrap().tick, 1);
        assert_eq!(snapshot.lock().unwrap().tick, 0);
    }

    #[test]
    fn snapshot_advances_only_after_success() {
        let snapshot = new_shared_snapshot(TimerSnapshot::new(0, 100, 0));
        let mut pending = pending_ticks(&[1, 2]);
        let mut calls = 0;
        let _ = submit_pending_with("physics", &snapshot, &mut pending, |_| {
            calls += 1;
            if calls == 1 { Ok(()) } else { Err(timer_error("InjectedTimerFailure", "boom")) }
        });
        assert_eq!(snapshot.lock().unwrap().tick, 1);
        assert_eq!(pending.front().unwrap().tick, 2);
    }
}
