//! Application-owned delta mirror. Full comparison is a correctness oracle;
//! only suffix application contributes to consumer-work measurements.
use mech_syntax::document::parser::event::Event;
use mech_syntax::document::{
    Diagnostic, DiagnosticAnchor, Revision, StreamChange, StreamUpdate, StreamView,
};

/// The application retains suffix ranges backed by immutable parser views.
/// Applying a wide replacement adopts one shared range; it does not clone each
/// event. Ranges before `from` stay attached to their historical views.
#[derive(Default, Debug)]
pub struct Mirror {
    events: Ranges,
    diagnostics: Ranges,
    pub records_read: u64,
    pub records_removed: u64,
    pub ranges_adopted: u64,
    pub ranges_removed: u64,
    pub range_visits: u64,
    pub updates: u64,
    pub rewinds: u64,
    pub diagnostic_updates: u64,
}
#[derive(Debug)]
struct SharedRange {
    start: usize,
    end: usize,
    view: StreamView,
}
#[derive(Default, Debug)]
struct Ranges {
    items: Vec<SharedRange>,
    len: usize,
}
impl Ranges {
    fn replace(&mut self, change: StreamChange, view: &StreamView) -> (u64, u64, u64) {
        let mut removed = 0;
        let mut visited = 0;
        while let Some(last) = self.items.last_mut() {
            visited += 1;
            if last.start >= change.from {
                self.items.pop();
                removed += 1;
            } else {
                last.end = last.end.min(change.from);
                break;
            }
        }
        let adopted = u64::from(change.from < change.new_len);
        if adopted != 0 {
            self.items.push(SharedRange {
                start: change.from,
                end: change.new_len,
                view: view.clone(),
            });
        }
        self.len = change.new_len;
        (adopted, removed, visited)
    }
}
fn bind(diagnostic: &mut Diagnostic, revision: Revision) {
    for anchor in core::iter::once(&mut diagnostic.primary)
        .chain(diagnostic.labels.iter_mut().map(|label| &mut label.anchor))
    {
        if let DiagnosticAnchor::Absolute { revision: old, .. } = anchor {
            *old = revision;
        }
    }
}
fn check(change: StreamChange, old: usize, new: usize) {
    assert_eq!(change.old_len, old, "consumer lost a publication baseline");
    assert_eq!(change.new_len, new);
    assert!(change.from <= old.min(new));
}
fn same_event(left: &Event, right: &Event) {
    assert_eq!(format!("{left:?}"), format!("{right:?}"));
    match (left, right) {
        (
            Event::Start {
                identity: a,
                cached: ac,
                ..
            },
            Event::Start {
                identity: b,
                cached: bc,
                ..
            },
        ) => {
            assert_eq!(a, b);
            assert_eq!(ac.is_some(), bc.is_some());
            if let (Some(a), Some(b)) = (ac, bc) {
                assert!(std::sync::Arc::ptr_eq(&a.node, &b.node));
                assert_eq!((a.range, a.end_event), (b.range, b.end_event));
            }
        }
        (Event::Reuse { node: a }, Event::Reuse { node: b }) => {
            assert!(std::sync::Arc::ptr_eq(a, b))
        }
        _ => {}
    }
}
impl Mirror {
    /// Actual application operations on shared range descriptors. Logical record
    /// coverage is reported separately; adopting a view clones retained roots.
    pub fn work(&self) -> u64 {
        self.range_visits + self.ranges_adopted + self.ranges_removed + self.updates
    }
    pub fn apply(&mut self, update: &StreamUpdate) {
        check(update.syntax, self.events.len, update.view.event_count());
        check(
            update.diagnostics,
            self.diagnostics.len,
            update.view.diagnostic_count(),
        );
        self.rewinds += u64::from(update.syntax.from < update.syntax.old_len);
        self.diagnostic_updates += u64::from(update.diagnostics.from < update.diagnostics.new_len);
        // These are logical records replaced, not enumerated by range adoption.
        self.records_removed += (update.syntax.old_len - update.syntax.from
            + update.diagnostics.old_len
            - update.diagnostics.from) as u64;
        for (adopted, removed, visited) in [
            self.events.replace(update.syntax, &update.view),
            self.diagnostics.replace(update.diagnostics, &update.view),
        ] {
            self.ranges_adopted += adopted;
            self.ranges_removed += removed;
            self.range_visits += visited;
        }
        self.updates += 1;
    }
    #[allow(
        dead_code,
        reason = "limited-export resynchronization is exercised by the publication integration target"
    )]
    pub fn resynchronize(&mut self, view: &StreamView) {
        self.apply(&StreamUpdate {
            progress: mech_syntax::document::StreamProgress::Limited,
            accepted_bytes: 0,
            syntax: StreamChange {
                from: 0,
                old_len: self.events.len,
                new_len: view.event_count(),
            },
            diagnostics: StreamChange {
                from: 0,
                old_len: self.diagnostics.len,
                new_len: view.diagnostic_count(),
            },
            view: view.clone(),
            work: Default::default(),
        });
    }
    pub fn assert_matches(&self, view: &StreamView) {
        assert_eq!(self.events.len, view.event_count());
        let mut index = 0;
        for range in &self.events.items {
            assert_eq!(range.start, index, "ranges must cover the complete journal");
            for event in range
                .view
                .events_from(range.start)
                .take(range.end - range.start)
            {
                same_event(event, view.event(index).unwrap());
                index += 1;
            }
        }
        assert_eq!(index, view.event_count());
        assert_eq!(self.diagnostics.len, view.diagnostic_count());
        index = 0;
        for range in &self.diagnostics.items {
            assert_eq!(range.start, index);
            for at in range.start..range.end {
                let mut diagnostic = range.view.diagnostic(at).unwrap();
                // Absolute anchors bind lazily to the current source revision.
                bind(&mut diagnostic, view.identity.revision);
                assert_eq!(Some(diagnostic), view.diagnostic(index));
                index += 1;
            }
        }
        assert_eq!(index, view.diagnostic_count());
    }
}
