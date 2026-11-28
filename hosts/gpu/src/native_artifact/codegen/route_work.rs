//! Per-attempt host telemetry for the compact route sequence.
use serde::Serialize;
use std::{
    cell::Cell,
    sync::{Arc, Mutex, mpsc},
};

const DETAIL_LIMIT: usize = 4096;
const SCOPE: &str = "Explicit CFG and dominance builds owned by route searches. Counts exclude cleanup, verifier and other pass builds.";

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
pub(in crate::native_artifact) struct RouteWork {
    pub loop_searches: usize,
    pub identity_searches: usize,
    pub cfg_requests: usize,
    pub cfg_builds: usize,
    pub dom_requests: usize,
    pub dom_builds: usize,
    pub graph_pair_reuses: usize,
    pub loop_edges_rewritten: usize,
    pub identity_edges_rewritten: usize,
}

impl RouteWork {
    fn add(&mut self, other: Self) {
        self.loop_searches += other.loop_searches;
        self.identity_searches += other.identity_searches;
        self.cfg_requests += other.cfg_requests;
        self.cfg_builds += other.cfg_builds;
        self.dom_requests += other.dom_requests;
        self.dom_builds += other.dom_builds;
        self.graph_pair_reuses += other.graph_pair_reuses;
        self.loop_edges_rewritten += other.loop_edges_rewritten;
        self.identity_edges_rewritten += other.identity_edges_rewritten;
    }
}

#[derive(Serialize)]
struct Row {
    ordinal: usize,
    symbol: String,
    #[serde(flatten)]
    work: RouteWork,
}

#[derive(Default)]
struct OverflowRouteWork {
    units: usize,
    work: RouteWork,
}

enum Destination {
    Detail {
        ordinal: usize,
        symbol: String,
        sender: mpsc::Sender<Row>,
    },
    Overflow(Arc<Mutex<OverflowRouteWork>>),
}

pub(in crate::native_artifact) struct RouteUnitRecorder {
    work: Cell<RouteWork>,
    destination: Destination,
}

impl RouteUnitRecorder {
    pub fn record(&self, update: impl FnOnce(&mut RouteWork)) {
        let mut work = self.work.get();
        update(&mut work);
        self.work.set(work);
    }
}

impl Drop for RouteUnitRecorder {
    fn drop(&mut self) {
        let work = self.work.get();
        match &mut self.destination {
            Destination::Detail {
                ordinal,
                symbol,
                sender,
            } => {
                let _ = sender.send(Row {
                    ordinal: *ordinal,
                    symbol: std::mem::take(symbol),
                    work,
                });
            }
            Destination::Overflow(overflow) => {
                let mut overflow = overflow
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                overflow.units += 1;
                overflow.work.add(work);
            }
        }
    }
}

pub(in crate::native_artifact) struct RouteWorkCollector {
    next: Cell<usize>,
    sender: mpsc::Sender<Row>,
    receiver: mpsc::Receiver<Row>,
    overflow: Arc<Mutex<OverflowRouteWork>>,
}

impl RouteWorkCollector {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            next: Cell::new(0),
            sender,
            receiver,
            overflow: Arc::default(),
        }
    }

    pub fn register(&self, symbol: &str) -> RouteUnitRecorder {
        let ordinal = self.next.get();
        self.next.set(ordinal + 1);
        let destination = if ordinal < DETAIL_LIMIT {
            Destination::Detail {
                ordinal,
                symbol: symbol.to_owned(),
                sender: self.sender.clone(),
            }
        } else {
            Destination::Overflow(self.overflow.clone())
        };
        RouteUnitRecorder {
            work: Cell::default(),
            destination,
        }
    }

    /// Drain only after the compact call returns and its workers have joined.
    pub fn json(&self) -> serde_json::Value {
        let mut rows: Vec<_> = self.receiver.try_iter().collect();
        rows.sort_unstable_by_key(|row| row.ordinal);
        let overflow = self
            .overflow
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut totals = overflow.work;
        for row in &rows {
            totals.add(row.work);
        }
        serde_json::json!({
            "scope": SCOPE,
            "detail_unit_limit": DETAIL_LIMIT,
            "units_truncated": overflow.units,
            "units": rows,
            "overflow": overflow.work,
            "totals": totals,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detail_cap_and_overflow_keep_complete_work_in_registration_order() {
        let collector = RouteWorkCollector::new();
        let first = collector.register("prepared first");
        first.record(|work| work.loop_searches += 2);
        for _ in 1..DETAIL_LIMIT {
            let recorder = collector.register("discarded preparation");
            recorder.record(|work| work.cfg_builds += 1);
        }
        for _ in 0..3 {
            let recorder = collector.register("overflow");
            recorder.record(|work| work.cfg_builds += 2);
        }
        drop(first);
        let json = collector.json();
        let rows = json["units"].as_array().unwrap();
        assert_eq!(rows.len(), DETAIL_LIMIT);
        assert_eq!(rows[0]["symbol"], "prepared first");
        assert!(
            rows.iter()
                .enumerate()
                .all(|(ordinal, row)| row["ordinal"] == ordinal)
        );
        assert_eq!(json["units_truncated"], 3);
        assert_eq!(json["overflow"]["cfg_builds"], 6);
        assert_eq!(json["totals"]["cfg_builds"], DETAIL_LIMIT + 5);
        assert_eq!(json["totals"]["loop_searches"], 2);
    }

    #[test]
    fn attempt_recorders_cross_workers_and_keep_independent_totals() {
        let first = RouteWorkCollector::new();
        let second = RouteWorkCollector::new();
        let rejected = first.register("unused candidate");
        let selected = second.register("retained candidate");
        std::thread::scope(|scope| {
            scope.spawn(move || rejected.record(|work| work.identity_searches += 3));
            scope.spawn(move || selected.record(|work| work.loop_searches += 2));
        });
        assert_eq!(first.json()["totals"]["identity_searches"], 3);
        let row = second.json();
        assert_eq!(row["totals"]["identity_searches"], 0);
        assert_eq!(row["totals"]["loop_searches"], 2);
    }
}
